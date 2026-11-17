use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::law::Law;
use crate::read_as::{self, Clause, Warning};
use crate::{relative_ref, structure};

/// Since which revision a warning of the latest revision has been given, and what the revision
/// before that had in its place.
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Origin {
    pub id: String,
    pub rule: String,
    pub law_id: String,
    pub first_revision_id: String,
    /// `oldest`: given since the oldest revision read. `enacted`: the oldest revision is the law
    /// as enacted. `target-changed`: the revision before had the same read-as clause, resolved
    /// to the same provisions, with the quoted words there. `renumbered`: the same clause
    /// matched other provisions. `clause-new`: the revision before lacked the clause.
    /// `was-unchecked`: the revision before had the clause outside what the rule resolves.
    /// `introduced`: a warning of another rule that the revision before lacked.
    pub kind: String,
    pub revisions: usize,
}

struct Revision {
    id: String,
    warnings: Vec<Warning>,
    clauses: Vec<Clause>,
}

fn read(file: &Path) -> Option<Revision> {
    let law = Law::from_file(file).ok()?;
    if law.articles.is_empty() {
        return None;
    }
    let (_, mut warnings, clauses) = read_as::check(&law);
    warnings.extend(relative_ref::check(&law));
    warnings.extend(structure::check(&law));
    Some(Revision {
        id: law.revision_id,
        warnings,
        clauses,
    })
}

fn same_clause(w: &Warning, quoted: &str, replacement: &str) -> bool {
    w.quoted == quoted && w.replacement == replacement
}

fn before<'a>(previous: &'a Revision, current: &Warning) -> Option<&'a Warning> {
    if let Some(same) = previous.warnings.iter().find(|w| w.id == current.id) {
        return Some(same);
    }
    if current.rule != read_as::RULE {
        return None;
    }
    let candidates: Vec<&Warning> = previous
        .warnings
        .iter()
        .filter(|w| w.rule == current.rule && same_clause(w, &current.quoted, &current.replacement))
        .collect();
    candidates
        .iter()
        .find(|w| w.source == current.source && w.scope == current.scope)
        .or_else(|| candidates.iter().find(|w| w.source == current.source))
        .or_else(|| candidates.first())
        .copied()
}

fn kind(previous: &Revision, first: &Warning) -> &'static str {
    if first.rule != read_as::RULE {
        return "introduced";
    }
    let candidates: Vec<&Clause> = previous
        .clauses
        .iter()
        .filter(|c| c.quoted == first.quoted && c.replacement == first.replacement)
        .collect();
    let clause = candidates
        .iter()
        .find(|c| c.source == first.source && c.scope == first.scope)
        .or_else(|| candidates.iter().find(|c| c.source == first.source))
        .or_else(|| candidates.first());
    match clause {
        None => "clause-new",
        Some(c) if c.result == "present" => {
            if c.targets == first.targets && c.source == first.source {
                "target-changed"
            } else {
                "renumbered"
            }
        }
        Some(_) => "was-unchecked",
    }
}

/// For every law in `files` (revision files grouped by law, oldest first), the origin of each
/// warning of its latest revision. The rules that need the whole corpus are not run.
pub fn origins(files: &BTreeMap<String, Vec<PathBuf>>) -> Vec<Origin> {
    let mut out = Vec::new();
    for (law_id, paths) in files {
        let revisions: Vec<Revision> = paths.iter().filter_map(|p| read(p)).collect();
        let Some(latest) = revisions.last() else {
            continue;
        };
        for warning in &latest.warnings {
            let mut index = revisions.len() - 1;
            let mut current = warning;
            while index > 0 {
                match before(&revisions[index - 1], current) {
                    Some(w) => {
                        current = w;
                        index -= 1;
                    }
                    None => break,
                }
            }
            let first = &revisions[index];
            out.push(Origin {
                id: warning.id.clone(),
                rule: warning.rule.clone(),
                law_id: law_id.clone(),
                first_revision_id: first.id.clone(),
                kind: if index > 0 {
                    kind(&revisions[index - 1], current)
                } else if first.id.ends_with("_000000000000000") {
                    "enacted"
                } else {
                    "oldest"
                }
                .to_string(),
                revisions: revisions.len(),
            });
        }
    }
    out
}
