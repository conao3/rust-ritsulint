use leanfran::Nat;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::generated::ritsulint_api as core;
use crate::law::{Article, Law};

pub const RULE: &str = "read-as-quote";

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Provision {
    pub article: String,
    pub paragraph: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Warning {
    pub id: String,
    pub rule: String,
    pub status: String,
    pub law_id: String,
    pub law_revision_id: String,
    pub law_title: String,
    pub source: Provision,
    pub scope: String,
    pub targets: Vec<Provision>,
    pub quoted: String,
    pub replacement: String,
    pub nearest: String,
    pub similarity: f64,
    pub confidence: String,
}

#[derive(Debug, Default, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Counts {
    pub clauses: u64,
    pub checked: u64,
    pub matched: u64,
    pub matched_without_parentheses: u64,
    pub external: u64,
    pub unsupported: u64,
    pub table_paragraphs: u64,
    pub warnings: u64,
}

impl Counts {
    pub fn add(&mut self, other: &Counts) {
        self.clauses += other.clauses;
        self.checked += other.checked;
        self.matched += other.matched;
        self.matched_without_parentheses += other.matched_without_parentheses;
        self.external += other.external;
        self.unsupported += other.unsupported;
        self.table_paragraphs += other.table_paragraphs;
        self.warnings += other.warnings;
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Status {
    Present,
    PresentWithoutParentheses,
    OtherParagraph,
    Absent,
    NoParagraph,
    NoArticle,
}

impl Status {
    fn name(self) -> &'static str {
        match self {
            Status::Present | Status::PresentWithoutParentheses => "present",
            Status::OtherParagraph => "other-paragraph",
            Status::Absent => "absent",
            Status::NoParagraph => "no-paragraph",
            Status::NoArticle => "no-article",
        }
    }
}

fn status_of(law: &Law, target: &Provision, quoted: &str) -> Status {
    let Some(article) = law.article(&target.article) else {
        return Status::NoArticle;
    };
    let text = if target.paragraph == 0 {
        &article.text
    } else {
        match article.paragraph(target.paragraph) {
            Some(p) => &p.text,
            None => return Status::NoParagraph,
        }
    };
    let quoted = &quoted.replace('／', "");
    if text.contains(quoted.as_str()) {
        Status::Present
    } else if core::strip_parens(text).contains(quoted.as_str()) {
        Status::PresentWithoutParentheses
    } else if article.text.contains(quoted.as_str()) {
        Status::OtherParagraph
    } else {
        Status::Absent
    }
}

fn nearest(text: &str, quoted: &str) -> String {
    let t: Vec<char> = text.chars().collect();
    let q: Vec<char> = quoted.chars().collect();
    if t.is_empty() || q.is_empty() || t.len() > 40_000 {
        return String::new();
    }
    let mut best = (0usize, 0usize, 0usize);
    let mut prev = vec![0usize; q.len() + 1];
    for i in 1..=t.len() {
        let mut cur = vec![0usize; q.len() + 1];
        for j in 1..=q.len() {
            if t[i - 1] == q[j - 1] {
                cur[j] = prev[j - 1] + 1;
                if cur[j] > best.0 {
                    best = (cur[j], i, j);
                }
            }
        }
        prev = cur;
    }
    if best.0 == 0 {
        return String::new();
    }
    let start = (best.1 - best.0).saturating_sub(best.2 - best.0);
    let end = (start + q.len() + 4).min(t.len());
    t[start..end].iter().collect()
}

fn similarity(quoted: &str, near: &str) -> f64 {
    let q: Vec<char> = quoted.chars().collect();
    let n: Vec<char> = near.chars().take(q.len()).collect();
    if q.is_empty() || n.is_empty() {
        return 0.0;
    }
    let mut prev: Vec<usize> = (0..=n.len()).collect();
    for i in 1..=q.len() {
        let mut cur = vec![i; n.len() + 1];
        for j in 1..=n.len() {
            let cost = usize::from(q[i - 1] != n[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    let ratio = 1.0 - prev[n.len()] as f64 / q.len().max(n.len()) as f64;
    (ratio * 100.0).round() / 100.0
}

fn names_article(scope: &str) -> bool {
    let chars: Vec<char> = scope.chars().collect();
    chars
        .windows(2)
        .any(|w| w[1] == '条' && "一二三四五六七八九十百千".contains(w[0]))
}

fn warning_id(
    law_id: &str,
    source: &Provision,
    scope: &str,
    targets: &[Provision],
    quoted: &str,
    occurrence: usize,
) -> String {
    let targets: String = targets
        .iter()
        .map(|t| format!("{}/{};", t.article, t.paragraph))
        .collect();
    let mut h = Sha256::new();
    for part in [
        RULE,
        law_id,
        &source.article,
        &source.paragraph.to_string(),
        scope,
        &targets,
        quoted,
        &occurrence.to_string(),
    ] {
        h.update(part.as_bytes());
        h.update([0x1f]);
    }
    h.finalize()
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn neighbors(law: &Law, index: usize) -> (&str, &str) {
    let prev = if index == 0 {
        ""
    } else {
        &law.articles[index - 1].num
    };
    let next = law.articles.get(index + 1).map_or("", |a| a.num.as_str());
    (prev, next)
}

fn check_article(
    law: &Law,
    index: usize,
    article: &Article,
    counts: &mut Counts,
    out: &mut Vec<Warning>,
) {
    let (prev, next) = neighbors(law, index);
    for paragraph in &article.paragraphs {
        if !paragraph.text.contains("とあるのは") {
            continue;
        }
        if paragraph.text.contains("読み替えられる字句") {
            counts.table_paragraphs += 1;
            continue;
        }
        let source = Provision {
            article: article.num.clone(),
            paragraph: paragraph.num,
        };
        let first = out.len();
        let findings = core::analyze(
            &paragraph.text,
            &article.num,
            prev,
            next,
            Nat::from(paragraph.num),
        );
        for f in findings {
            counts.clauses += 1;
            match f.kind.as_str() {
                "external" => {
                    counts.external += 1;
                    continue;
                }
                "refs" => {}
                _ => {
                    counts.unsupported += 1;
                    continue;
                }
            }
            let targets: Vec<Provision> = f
                .targets
                .into_iter()
                .map(|t| Provision {
                    article: t.article,
                    paragraph: u64::try_from(t.paragraph).unwrap_or(0),
                })
                .collect();
            counts.checked += 1;
            let statuses: Vec<Status> = targets
                .iter()
                .map(|t| status_of(law, t, &f.quoted))
                .collect();
            let best = statuses.iter().copied().min().unwrap_or(Status::Absent);
            match best {
                Status::Present => {
                    counts.matched += 1;
                    continue;
                }
                Status::PresentWithoutParentheses => {
                    counts.matched_without_parentheses += 1;
                    continue;
                }
                _ => {}
            }
            let near = targets
                .iter()
                .find_map(|t| law.article(&t.article))
                .map(|a| nearest(&a.text, &f.quoted))
                .unwrap_or_default();
            let sim = similarity(&f.quoted, &near);
            let confidence = if f.after_law_name && sim < 0.75 {
                "low"
            } else if names_article(&f.scope) && best != Status::NoArticle {
                "high"
            } else {
                "medium"
            };
            counts.warnings += 1;
            out.push(Warning {
                id: {
                    let occurrence = out[first..]
                        .iter()
                        .filter(|w| {
                            w.scope == f.scope && w.targets == targets && w.quoted == f.quoted
                        })
                        .count();
                    warning_id(
                        &law.law_id,
                        &source,
                        &f.scope,
                        &targets,
                        &f.quoted,
                        occurrence,
                    )
                },
                rule: RULE.to_string(),
                status: best.name().to_string(),
                law_id: law.law_id.clone(),
                law_revision_id: law.revision_id.clone(),
                law_title: law.title.clone(),
                source: source.clone(),
                scope: f.scope,
                targets,
                quoted: f.quoted,
                replacement: f.replacement,
                nearest: near,
                similarity: sim,
                confidence: confidence.to_string(),
            });
        }
        let missing = out[first..]
            .iter()
            .filter(|w| w.status == "no-article")
            .count();
        if missing >= 2 {
            for w in &mut out[first..] {
                w.confidence = "low".to_string();
            }
        }
    }
}

pub fn check(law: &Law) -> (Counts, Vec<Warning>) {
    let mut counts = Counts::default();
    let mut out = Vec::new();
    for (index, article) in law.articles.iter().enumerate() {
        check_article(law, index, article, &mut counts, &mut out);
    }
    (counts, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law(body: &str) -> Law {
        let xml = format!(
            "<Law><LawBody><LawTitle>テスト法</LawTitle><MainProvision>{body}</MainProvision></LawBody></Law>"
        );
        Law::from_xml(&xml, "999AC0000000001_20260101_000000000000000").expect("the fixture parses")
    }

    fn article(num: &str, paragraphs: &[&str]) -> String {
        let body: String = paragraphs
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "<Paragraph Num=\"{}\"><ParagraphSentence>{p}</ParagraphSentence></Paragraph>",
                    i + 1
                )
            })
            .collect();
        format!("<Article Num=\"{num}\">{body}</Article>")
    }

    fn statuses(body: &str) -> Vec<(String, String, String)> {
        check(&law(body))
            .1
            .into_iter()
            .map(|w| (w.status, w.confidence, w.quoted))
            .collect()
    }

    #[test]
    fn quotation_split_over_two_sentences_is_found() {
        let body = article(
            "1",
            &[
                "<Sentence>期間は、三十日以内とする。</Sentence><Sentence>ただし、延長することができる。</Sentence>",
            ],
        ) + &article(
            "2",
            &[
                "<Sentence>前条の規定は、収容に準用する。この場合において、第一条第一項中「三十日以内とする。但し、延長することができる。」とあるのは「必要と認める期間とする。」と読み替えるものとする。</Sentence>",
            ],
        );
        let found = statuses(&body);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "absent");
        assert_eq!(found[0].1, "high");
    }

    #[test]
    fn matching_quotation_gives_no_warning() {
        let body = article("1", &["<Sentence>期間は、三十日以内とする。</Sentence>"])
            + &article(
                "2",
                &[
                    "<Sentence>この場合において、前条中「三十日以内」とあるのは「十日以内」と、「期間」とあるのは「日数」と読み替える。</Sentence>",
                ],
            );
        let (counts, warnings) = check(&law(&body));
        assert!(warnings.is_empty());
        assert_eq!(counts.matched, 2);
    }

    #[test]
    fn missing_paragraph_is_reported() {
        let body = article(
            "1",
            &["<Sentence>申請書には、写し二通を添える。</Sentence>"],
        ) + &article(
            "2",
            &[
                "<Sentence>この場合において、届出については、第一条第五項中「二通」とあるのは、「一通」と読み替えるものとする。</Sentence>",
            ],
        );
        let found = statuses(&body);
        assert_eq!(
            found,
            vec![(
                "no-paragraph".to_string(),
                "high".to_string(),
                "二通".to_string()
            )]
        );
    }

    #[test]
    fn quotation_in_another_paragraph_is_reported() {
        let body = article(
            "1",
            &[
                "<Sentence>届書を提出する。</Sentence>",
                "<Sentence>第四十条の二第一項の届書とみなす。</Sentence>",
            ],
        ) + &article(
            "2",
            &[
                "<Sentence>この場合において、第一条第一項中「第四十条の二第一項」とあるのは「第五十六条の二第一項」と読み替える。</Sentence>",
            ],
        );
        assert_eq!(statuses(&body)[0].0, "other-paragraph");
    }

    #[test]
    fn reference_to_another_rule_is_skipped() {
        let body = article("1", &["<Sentence>所属長は、報告する。</Sentence>"])
            + &article(
                "2",
                &[
                    "<Sentence>この場合において、拳銃規範第一条第一項中「警察官は、」とあるのは「指揮官は、」と、同条第三項中「所属長」とあるのは「指定所属長」とする。</Sentence>",
                ],
            );
        let (counts, warnings) = check(&law(&body));
        assert!(warnings.is_empty());
        assert_eq!(counts.external, 2);
    }

    #[test]
    fn abbreviation_defined_inside_the_quoted_words_matches() {
        let body = article(
            "1",
            &[
                "<Sentence>社会福祉士法（昭和六十二年法律第三十号。以下「法」という。）第十条第二項の指定を受ける。</Sentence>",
            ],
        ) + &article(
            "2",
            &[
                "<Sentence>この場合において、第一条第一項中「法第十条第二項」とあるのは「法第三十五条」と読み替える。</Sentence>",
            ],
        );
        let (counts, warnings) = check(&law(&body));
        assert!(warnings.is_empty());
        assert_eq!(counts.matched_without_parentheses, 1);
    }

    #[test]
    fn difference_inside_parentheses_is_reported() {
        let body = article(
            "1",
            &[
                "<Sentence>都道府県知事（特定市町村の区域にある場合にあっては、特定市町村の長）に提出する。</Sentence>",
            ],
        ) + &article(
            "2",
            &[
                "<Sentence>この場合において、第一条中「都道府県知事（特定市町村の区域にある場所にあっては、特定市町村の長）」とあるのは「委任都道府県知事」と読み替える。</Sentence>",
            ],
        );
        let found = statuses(&body);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "absent");
    }

    #[test]
    fn warning_id_does_not_depend_on_the_revision() {
        let body = article("1", &["<Sentence>期間は、三十日とする。</Sentence>"])
            + &article(
                "2",
                &[
                    "<Sentence>この場合において、前条中「二十日」とあるのは「十日」と読み替える。</Sentence>",
                ],
            );
        let xml = format!(
            "<Law><LawBody><LawTitle>テスト法</LawTitle><MainProvision>{body}</MainProvision></LawBody></Law>"
        );
        let a = check(&Law::from_xml(&xml, "999AC0000000001_20260101_000000000000000").unwrap()).1;
        let b = check(&Law::from_xml(&xml, "999AC0000000001_20270401_509AC0000000001").unwrap()).1;
        assert_eq!(a[0].id, b[0].id);
        assert_ne!(a[0].law_revision_id, b[0].law_revision_id);
    }
}
