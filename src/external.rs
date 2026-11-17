use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;

use crate::generated::ritsulint_api as core;
use crate::law::Law;
use crate::read_as::{Provision, Status, Warning, nearest, similarity, status_of, warning_id};

pub const RULE: &str = "read-as-quote-external";

const N: &str = "[一二三四五六七八九十百千]+";

static LAW_NUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "(?:明治|大正|昭和|平成|令和)[元一二三四五六七八九十]+年[^（）。「」、]{{1,25}}?第{N}号"
    ))
    .expect("the pattern is valid")
});

static ABBREVIATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "（((?:明治|大正|昭和|平成|令和)[元一二三四五六七八九十]+年[^（）。「」、]{{1,25}}?第{N}号)。[^（）「」]{{0,40}}?以下(この[^「」、。]{{1,12}}において)?(?:単に)?「([^「」]{{1,30}})」という。[^（）]*）"
    ))
    .expect("the pattern is valid")
});

static CLAUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "(同条|[^「」。、（）]{{1,60}}?)(（[^（）]*(?:（[^（）]*）[^（）]*)*）)?(?:第({N}(?:の{N})*)条)?(?:第({N})項)?中「([^「」]{{2,300}})」とあるのは"
    ))
    .expect("the pattern is valid")
});

const SAME_LAW: [&str; 4] = ["同法", "同令", "同規則", "同省令"];
const GLUE: [&str; 6] = ["する", "及び", "又は", "並びに", "おいて", "かかわらず"];
const NOT_CURRENT: [&str; 8] = [
    "旧",
    "改正前",
    "改正後",
    "廃止前",
    "新法",
    "新令",
    "新規則",
    "読み替え",
];
const TRANSITIONAL: [&str; 9] = [
    "復帰に伴う",
    "に対処するため",
    "施行法",
    "暫定措置",
    "の一部を改正する",
    "年度における",
    "整備",
    "経過措置",
    "解散",
];

/// A read-as clause that names a provision of another law, before the law is looked up.
pub struct Pending {
    law_id: String,
    law_revision_id: String,
    law_title: String,
    source: Provision,
    name: String,
    law_num: Option<String>,
    same: bool,
    article: Option<String>,
    paragraph: u64,
    scope: String,
    quoted: String,
    abbreviations: HashMap<String, String>,
    group: usize,
}

fn article_key(text: &str) -> Option<String> {
    let parts: Vec<String> = text
        .split('の')
        .map(|p| u64::try_from(core::kansuji(p)).unwrap_or(0))
        .map(|n| n.to_string())
        .collect();
    (!parts.iter().any(|p| p == "0")).then(|| parts.join("_"))
}

/// The clauses of a law that write the name of a law, a law number or an abbreviation right
/// before the article they quote from.
pub fn collect(law: &Law) -> Vec<Pending> {
    let mut abbreviations: HashMap<String, HashSet<String>> = HashMap::new();
    for article in &law.articles {
        for m in ABBREVIATION.captures_iter(&article.text) {
            if m.get(2).is_none() {
                abbreviations
                    .entry(m[3].to_string())
                    .or_default()
                    .insert(m[1].to_string());
            }
        }
    }
    let abbreviations: HashMap<String, String> = abbreviations
        .into_iter()
        .filter(|(_, nums)| nums.len() == 1)
        .filter_map(|(name, nums)| Some((name, nums.into_iter().next()?)))
        .collect();
    let mut out = Vec::new();
    let mut group = 0;
    for article in &law.articles {
        for paragraph in &article.paragraphs {
            if !paragraph.text.contains("とあるのは") {
                continue;
            }
            group += 1;
            for m in CLAUSE.captures_iter(&paragraph.text) {
                let name = m[1].to_string();
                let whole = m.get(0).map_or("", |x| x.as_str());
                out.push(Pending {
                    law_id: law.law_id.clone(),
                    law_revision_id: law.revision_id.clone(),
                    law_title: law.title.clone(),
                    source: Provision {
                        article: article.num.clone(),
                        paragraph: paragraph.num,
                    },
                    same: name == "同条" || SAME_LAW.iter().any(|s| name.ends_with(s)),
                    law_num: m
                        .get(2)
                        .and_then(|p| LAW_NUM.find(p.as_str()))
                        .map(|x| x.as_str().to_string()),
                    article: m.get(3).and_then(|a| article_key(a.as_str())),
                    paragraph: m
                        .get(4)
                        .map_or(0, |p| u64::try_from(core::kansuji(p.as_str())).unwrap_or(0)),
                    scope: whole.split("中「").next().unwrap_or("").to_string(),
                    quoted: m[5].to_string(),
                    abbreviations: abbreviations.clone(),
                    name,
                    group,
                });
            }
        }
    }
    out
}

/// The titles and law numbers of the corpus, each to the law that alone has it.
#[derive(Default)]
pub struct Index {
    titles: HashMap<String, Vec<String>>,
    numbers: HashMap<String, Vec<String>>,
    pub files: HashMap<String, PathBuf>,
}

impl Index {
    pub fn add(&mut self, law: &Law, file: PathBuf) {
        self.titles
            .entry(law.title.clone())
            .or_default()
            .push(law.law_id.clone());
        self.numbers
            .entry(law.law_num.clone())
            .or_default()
            .push(law.law_id.clone());
        self.files.insert(law.law_id.clone(), file);
    }

    fn by_number(&self, number: &str) -> Option<&String> {
        self.numbers.get(number).filter(|l| l.len() == 1)?.first()
    }

    fn by_title(&self, name: &str) -> Option<&String> {
        let chars: Vec<char> = name.chars().collect();
        (0..chars.len().saturating_sub(1)).find_map(|i| {
            let tail: String = chars[i..].iter().collect();
            let head: String = chars[..i].iter().collect();
            let whole = i == 0 || GLUE.iter().any(|g| head.ends_with(g));
            self.titles
                .get(&tail)
                .filter(|l| l.len() == 1 && whole)?
                .first()
        })
    }

    fn resolve(&self, p: &Pending) -> Option<&String> {
        if NOT_CURRENT.iter().any(|w| p.name.contains(w)) {
            return None;
        }
        if let Some(found) = p.law_num.as_deref().and_then(|n| self.by_number(n)) {
            return Some(found);
        }
        if let Some(found) = self.by_title(&p.name) {
            return Some(found);
        }
        p.abbreviations
            .iter()
            .find(|(a, _)| {
                p.name == **a || GLUE.iter().any(|g| p.name.ends_with(&format!("{g}{a}")))
            })
            .and_then(|(_, number)| self.by_number(number))
    }
}

#[derive(Default)]
pub struct Counts {
    pub checked: u64,
    pub matched: u64,
}

/// Looks up the provision each clause names in the law it names and reports the clauses whose
/// quoted words are not there. `load` reads a law of the corpus by its ID.
pub fn check(
    pending: &[Pending],
    index: &Index,
    mut load: impl FnMut(&str) -> Option<Law>,
) -> (Counts, Vec<Warning>) {
    let mut laws: HashMap<String, Option<Law>> = HashMap::new();
    let mut counts = Counts::default();
    let mut out: Vec<Warning> = Vec::new();
    let mut last: Option<(usize, String, String, Option<String>)> = None;
    for p in pending {
        let here = (p.group, p.law_id.clone());
        let previous = last
            .as_ref()
            .filter(|(g, l, _, _)| (*g, l.clone()) == here)
            .cloned();
        let (target_law, article) = if p.name == "同条" {
            match previous {
                Some((_, _, law, Some(article))) => (law, article),
                _ => {
                    last = None;
                    continue;
                }
            }
        } else if p.same {
            match (previous, &p.article) {
                (Some((_, _, law, _)), Some(article)) => (law, article.clone()),
                _ => {
                    last = None;
                    continue;
                }
            }
        } else {
            match (index.resolve(p), &p.article) {
                (Some(law), Some(article)) => (law.clone(), article.clone()),
                _ => {
                    last = None;
                    continue;
                }
            }
        };
        last = Some((
            p.group,
            p.law_id.clone(),
            target_law.clone(),
            Some(article.clone()),
        ));
        if target_law == p.law_id {
            continue;
        }
        let Some(law) = laws
            .entry(target_law.clone())
            .or_insert_with(|| load(&target_law))
            .as_ref()
        else {
            continue;
        };
        let target = Provision {
            article,
            paragraph: p.paragraph,
        };
        counts.checked += 1;
        let status = status_of(law, &target, &p.quoted);
        if matches!(status, Status::Present | Status::PresentWithoutParentheses) {
            counts.matched += 1;
            continue;
        }
        let near = law
            .article(&target.article)
            .map(|a| nearest(&a.text, &p.quoted))
            .unwrap_or_default();
        let targets = vec![target];
        let occurrence = out
            .iter()
            .filter(|w| {
                w.law_id == p.law_id
                    && w.source == p.source
                    && w.scope == p.scope
                    && w.quoted == p.quoted
            })
            .count();
        out.push(Warning {
            id: warning_id(
                RULE,
                &p.law_id,
                &p.source,
                &format!("{}\u{1f}{}", p.scope, target_law),
                &targets,
                &p.quoted,
                occurrence,
            ),
            rule: RULE.to_string(),
            status: status.name().to_string(),
            law_id: p.law_id.clone(),
            law_revision_id: p.law_revision_id.clone(),
            law_title: p.law_title.clone(),
            source: p.source.clone(),
            scope: p.scope.clone(),
            targets,
            quoted: p.quoted.clone(),
            replacement: String::new(),
            similarity: similarity(&p.quoted, &near),
            nearest: near,
            confidence: if TRANSITIONAL.iter().any(|w| p.law_title.contains(w)) {
                "low"
            } else {
                "medium"
            }
            .to_string(),
            context: String::new(),
            actual: String::new(),
            target_law_id: law.law_id.clone(),
            target_law_title: law.title.clone(),
        });
    }
    (counts, out)
}
