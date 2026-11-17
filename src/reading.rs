use serde::Serialize;

use crate::generated::ritsulint_api as core;
use crate::law::Law;
use crate::read_as::{Clause, Provision, Warning, warning_id};

pub const RULE: &str = "read-as-overlap";

/// The provisions read under the read-as clauses written for them. A group is the clauses of one
/// paragraph for one provision of the same law whose quoted words stand in it.
#[derive(Debug, Default, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Counts {
    pub groups: u64,
    /// Groups of two or more clauses.
    pub several: u64,
    /// Groups left out: the same words are read as two things, which the clauses do for parts
    /// of the provision the linter does not tell apart (items, the proviso).
    pub finer_scope: u64,
    /// Groups of several clauses where every clause is independent of the clauses before it.
    pub independent: u64,
    /// Groups where replacing clause by clause gives another text than reading at once.
    pub order_changes: u64,
    /// Groups where reading from the right gives another text than reading from the left.
    pub direction_changes: u64,
    /// Independent groups where the two readings differ. The theorem
    /// `Ritsulint.ReadAs.applySeq_eq_applyAll` says there are none.
    pub independent_but_differs: u64,
}

impl Counts {
    pub fn add(&mut self, other: &Counts) {
        self.groups += other.groups;
        self.several += other.several;
        self.finer_scope += other.finer_scope;
        self.independent += other.independent;
        self.order_changes += other.order_changes;
        self.direction_changes += other.direction_changes;
        self.independent_but_differs += other.independent_but_differs;
    }
}

/// A group whose text depends on how the clauses are read.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Group {
    pub law_id: String,
    pub source: Provision,
    pub target: Provision,
    pub clauses: Vec<(String, String)>,
    pub order_changes: bool,
    pub direction_changes: bool,
}

struct Pending<'a> {
    source: &'a Provision,
    target: &'a Provision,
    clauses: Vec<(String, String)>,
}

fn text_of<'a>(law: &'a Law, target: &Provision) -> Option<&'a str> {
    let article = law.article(&target.article)?;
    if target.paragraph == 0 {
        Some(&article.text)
    } else {
        article.paragraph(target.paragraph).map(|p| p.text.as_str())
    }
}

fn around(text: &str, other: &str) -> (String, String) {
    let a: Vec<char> = text.chars().collect();
    let b: Vec<char> = other.chars().collect();
    let at = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let cut = |s: &[char]| -> String {
        s[at.saturating_sub(25)..(at + 40).min(s.len())]
            .iter()
            .collect()
    };
    (cut(&a), cut(&b))
}

/// The first stretch of the text where the quoted words of one clause run into the quoted words
/// of another.
fn crossing(text: &str, clauses: &[(String, String)]) -> Option<(String, usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let words: Vec<Vec<char>> = clauses.iter().map(|c| c.0.chars().collect()).collect();
    let opens = |w: &[char], at: usize| chars[at..].starts_with(w);
    for at in 0..chars.len() {
        for (i, a) in words.iter().enumerate() {
            if !opens(a, at) {
                continue;
            }
            for inner in at + 1..at + a.len() {
                for (j, b) in words.iter().enumerate() {
                    if i != j && inner + b.len() > at + a.len() && opens(b, inner) {
                        let span = chars[at..inner + b.len()].iter().collect();
                        return Some((span, i, j));
                    }
                }
            }
        }
    }
    None
}

pub fn check(law: &Law, clauses: &[Clause]) -> (Counts, Vec<Warning>, Vec<Group>) {
    let mut pending: Vec<Pending> = Vec::new();
    for c in clauses.iter().filter(|c| c.result == "present") {
        let quoted = c.quoted.replace('／', "");
        for target in &c.targets {
            let Some(text) = text_of(law, target) else {
                continue;
            };
            if quoted.is_empty() || !text.contains(quoted.as_str()) {
                continue;
            }
            let pair = (quoted.clone(), c.replacement.clone());
            match pending
                .iter_mut()
                .find(|p| p.source == &c.source && p.target == target)
            {
                Some(p) => {
                    if !p.clauses.contains(&pair) {
                        p.clauses.push(pair);
                    }
                }
                None => pending.push(Pending {
                    source: &c.source,
                    target,
                    clauses: vec![pair],
                }),
            }
        }
    }
    let mut counts = Counts::default();
    let mut warnings = Vec::new();
    let mut groups = Vec::new();
    for p in pending {
        counts.groups += 1;
        if p.clauses.len() < 2 {
            continue;
        }
        counts.several += 1;
        let twice = p
            .clauses
            .iter()
            .enumerate()
            .any(|(i, a)| p.clauses[..i].iter().any(|b| a.0 == b.0));
        if twice {
            counts.finer_scope += 1;
            continue;
        }
        let Some(text) = text_of(law, p.target) else {
            continue;
        };
        let reading = core::read_as(
            p.clauses
                .iter()
                .map(|(quoted, replacement)| core::ReadClause {
                    quoted: quoted.clone(),
                    replacement: replacement.clone(),
                })
                .collect(),
            text,
        );
        let order_changes = reading.in_order != reading.at_once;
        let direction_changes = reading.from_right != reading.at_once;
        counts.independent += u64::from(reading.independent);
        counts.order_changes += u64::from(order_changes);
        counts.direction_changes += u64::from(direction_changes);
        counts.independent_but_differs += u64::from(reading.independent && order_changes);
        if direction_changes {
            let (context, actual) = around(&reading.at_once, &reading.from_right);
            let (quoted, replacement) = match crossing(text, &p.clauses) {
                Some((span, i, j)) => (
                    span,
                    format!(
                        "「{}」→「{}」／「{}」→「{}」",
                        p.clauses[i].0, p.clauses[i].1, p.clauses[j].0, p.clauses[j].1
                    ),
                ),
                None => (String::new(), String::new()),
            };
            let targets = vec![p.target.clone()];
            warnings.push(Warning {
                id: warning_id(RULE, &law.law_id, p.source, "", &targets, &quoted, 0),
                rule: RULE.to_string(),
                status: "direction".to_string(),
                law_id: law.law_id.clone(),
                law_revision_id: law.revision_id.clone(),
                law_title: law.title.clone(),
                source: p.source.clone(),
                scope: String::new(),
                targets,
                quoted,
                replacement,
                nearest: String::new(),
                similarity: 0.0,
                confidence: "medium".to_string(),
                context,
                actual,
                target_law_id: String::new(),
                target_law_title: String::new(),
            });
        }
        if order_changes || direction_changes {
            groups.push(Group {
                law_id: law.law_id.clone(),
                source: p.source.clone(),
                target: p.target.clone(),
                clauses: p.clauses,
                order_changes,
                direction_changes,
            });
        }
    }
    (counts, warnings, groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_as;

    fn law(body: &str) -> Law {
        let xml = format!(
            "<Law><LawBody><LawTitle>テスト法</LawTitle><MainProvision>{body}</MainProvision></LawBody></Law>"
        );
        Law::from_xml(&xml, "999AC0000000001_20260101_000000000000000").expect("the fixture parses")
    }

    fn article(num: &str, text: &str) -> String {
        format!(
            "<Article Num=\"{num}\"><Paragraph Num=\"1\"><ParagraphSentence><Sentence>{text}</Sentence></ParagraphSentence></Paragraph></Article>"
        )
    }

    fn run(first: &str, second: &str) -> (Counts, Vec<Warning>, Vec<Group>) {
        let law = law(&format!("{}{}", article("1", first), article("2", second)));
        let (_, _, clauses) = read_as::check(&law);
        check(&law, &clauses)
    }

    #[test]
    fn exchanged_words_depend_on_the_order() {
        let (counts, warnings, groups) = run(
            "長の選挙は議員の選挙の後に行う。",
            "この場合において、前条中「長の選挙」とあるのは「議員の選挙」と、「議員の選挙」とあるのは「長の選挙」と読み替えるものとする。",
        );
        assert_eq!(counts.several, 1);
        assert_eq!(counts.independent, 0);
        assert_eq!(counts.order_changes, 1);
        assert_eq!(counts.direction_changes, 0);
        assert!(warnings.is_empty());
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn separate_words_are_independent() {
        let (counts, warnings, groups) = run(
            "市長は、条例で定める。",
            "この場合において、前条中「市長」とあるのは「町長」と、「条例」とあるのは「規則」と読み替えるものとする。",
        );
        assert_eq!(counts.several, 1);
        assert_eq!(counts.independent, 1);
        assert_eq!(counts.order_changes, 0);
        assert_eq!(counts.independent_but_differs, 0);
        assert!(warnings.is_empty());
        assert!(groups.is_empty());
    }

    #[test]
    fn words_that_run_into_each_other_are_reported() {
        let (counts, warnings, _) = run(
            "当該変更の登録の日から起算する。",
            "この場合において、前条中「変更の登録」とあるのは「指定の変更」と、「登録の日」とあるのは「指定の日」と読み替えるものとする。",
        );
        assert_eq!(counts.direction_changes, 1);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, RULE);
        assert_eq!(warnings[0].quoted, "変更の登録の日");
        assert!(warnings[0].context.contains("指定の変更の日"));
        assert!(warnings[0].actual.contains("変更の指定の日"));
    }

    #[test]
    fn the_same_words_read_as_two_things_are_left_out() {
        let (counts, _, _) = run(
            "申請人は、申請人の住所を届け出る。",
            "この場合において、前条中「申請人」とあるのは「申立人」と、「申請人」とあるのは「審査申立人」と読み替えるものとする。",
        );
        assert_eq!(counts.finer_scope, 1);
    }
}
