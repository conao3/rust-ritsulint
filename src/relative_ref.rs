use crate::generated::ritsulint_api as core;
use crate::law::{Article, Law, Paragraph};
use crate::read_as::{Provision, Warning, warning_id};

pub const RULE: &str = "relative-ref";
pub const UNIT_RULE: &str = "missing-unit";

const APPENDIX_WORDS: [&str; 9] = [
    "別表", "様式", "附則", "別記", "付録", "別図", "別紙", "付表", "告示",
];

fn deleted(article: &Article) -> bool {
    article.num.contains(':') || article.text.ends_with("削除")
}

fn item_base(num: &str) -> Option<(u64, u64)> {
    let part = |s: &str| s.split('_').next()?.parse::<u64>().ok();
    let mut ends = num.split(':');
    let lo = part(ends.next()?)?;
    let hi = ends.next().and_then(part).unwrap_or(lo);
    Some((lo, hi))
}

fn has_item(paragraph: &Paragraph, n: u64) -> bool {
    paragraph
        .items
        .iter()
        .filter_map(|i| item_base(&i.num))
        .any(|(lo, hi)| lo <= n && n <= hi)
}

fn shape(article: &Article) -> String {
    format!("{}: {} paragraphs", article.num, article.paragraphs.len())
}

fn snippet(chars: &[char], offset: usize, length: usize) -> String {
    let from = offset.saturating_sub(25);
    let to = (offset + length + 25).min(chars.len());
    chars[from..to].iter().collect()
}

fn in_appendix(prefix: &[char]) -> bool {
    let from = prefix.len().saturating_sub(14);
    let tail: String = prefix[from..].iter().collect();
    let tail = tail.rsplit(['、', '。']).next().unwrap_or("");
    APPENDIX_WORDS.iter().any(|w| tail.contains(w))
}

struct Found {
    rule: &'static str,
    status: &'static str,
    confidence: &'static str,
    target: Option<Provision>,
    quoted: String,
    context: String,
    actual: String,
}

fn nat(n: &leanfran::Nat) -> usize {
    usize::try_from(n.clone()).unwrap_or(usize::MAX)
}

fn paragraph_refs(
    law: &Law,
    index: usize,
    article: &Article,
    position: usize,
    paragraph: &Paragraph,
    out: &mut Vec<Found>,
) {
    let text = core::unquote(&paragraph.prose);
    let chars: Vec<char> = text.chars().collect();
    for r in core::relative_refs(&text) {
        let (offset, length, count) = (nat(&r.offset), nat(&r.length), nat(&r.count) as u64);
        let prefix = &chars[..offset.min(chars.len())];
        let quoted: String = chars[offset..(offset + length).min(chars.len())]
            .iter()
            .collect();
        let context = snippet(&chars, offset, length);
        let mut push = |rule, status, confidence, target, actual| {
            out.push(Found {
                rule,
                status,
                confidence,
                target,
                quoted: quoted.clone(),
                context: context.clone(),
                actual,
            });
        };
        if r.kind == "missing-unit" {
            if !in_appendix(prefix) {
                push(UNIT_RULE, "missing-unit", "high", None, String::new());
            }
            continue;
        }
        if prefix.ends_with(&['準', '用', 'す', 'る']) {
            continue;
        }
        match r.kind.as_str() {
            "prev-article-paragraph" | "next-article-paragraph" => {
                let other = if r.kind == "prev-article-paragraph" {
                    index.checked_sub(1).and_then(|i| law.articles.get(i))
                } else {
                    law.articles.get(index + 1)
                };
                if let Some(other) = other
                    && !deleted(other)
                    && other.paragraph(count).is_none()
                {
                    let target = Provision {
                        article: other.num.clone(),
                        paragraph: count,
                    };
                    push(RULE, "no-paragraph", "high", Some(target), shape(other));
                }
            }
            "prev-paragraph-item" => {
                if let Some(before) = position
                    .checked_sub(1)
                    .and_then(|i| article.paragraphs.get(i))
                    && !before.items.is_empty()
                    && !has_item(before, count)
                {
                    let target = Provision {
                        article: article.num.clone(),
                        paragraph: before.num,
                    };
                    let actual = format!(
                        "{}/{}: {} items",
                        article.num,
                        before.num,
                        before.items.len()
                    );
                    push(RULE, "no-item", "high", Some(target), actual);
                }
            }
            "prev-paragraphs" => {
                if (position as u64) < count {
                    let confidence = if position == 0 { "low" } else { "high" };
                    push(RULE, "no-paragraph", confidence, None, shape(article));
                }
            }
            "next-paragraph" if position + 1 >= article.paragraphs.len() => {
                push(RULE, "no-paragraph", "high", None, shape(article));
            }
            _ => {}
        }
    }
}

fn item_refs(article: &Article, paragraph: &Paragraph, out: &mut Vec<Found>) {
    for (position, item) in paragraph.items.iter().enumerate() {
        let text = core::unquote(&item.sentence);
        let chars: Vec<char> = text.chars().collect();
        for r in core::relative_refs(&text) {
            let (offset, length, count) = (nat(&r.offset), nat(&r.length), nat(&r.count));
            let missing = match r.kind.as_str() {
                "prev-items" => position < count,
                "next-item" => position + 1 >= paragraph.items.len(),
                _ => false,
            };
            if missing {
                out.push(Found {
                    rule: RULE,
                    status: "no-item",
                    confidence: if position == 0 { "low" } else { "high" },
                    target: None,
                    quoted: chars[offset..(offset + length).min(chars.len())]
                        .iter()
                        .collect(),
                    context: snippet(&chars, offset, length),
                    actual: format!(
                        "{}/{}: {} items, item {} of them",
                        article.num,
                        paragraph.num,
                        paragraph.items.len(),
                        position + 1
                    ),
                });
            }
        }
    }
}

/// References by position (`前条第二項`, `次項`, `前三号`) that point at a paragraph or an item
/// the law does not have, and references that lack their unit (`第九十八第二項`). A paragraph
/// with a read-as clause is not read.
pub fn check(law: &Law) -> Vec<Warning> {
    let mut out = Vec::new();
    for (index, article) in law.articles.iter().enumerate() {
        for (position, paragraph) in article.paragraphs.iter().enumerate() {
            if paragraph.text.contains("とあるのは") {
                continue;
            }
            let mut found = Vec::new();
            paragraph_refs(law, index, article, position, paragraph, &mut found);
            item_refs(article, paragraph, &mut found);
            let source = Provision {
                article: article.num.clone(),
                paragraph: paragraph.num,
            };
            let first = out.len();
            for f in found {
                let targets: Vec<Provision> = f.target.into_iter().collect();
                let occurrence = out[first..]
                    .iter()
                    .filter(|w: &&Warning| w.rule == f.rule && w.quoted == f.quoted)
                    .count();
                out.push(Warning {
                    id: warning_id(
                        f.rule,
                        &law.law_id,
                        &source,
                        f.status,
                        &targets,
                        &f.quoted,
                        occurrence,
                    ),
                    rule: f.rule.to_string(),
                    status: f.status.to_string(),
                    law_id: law.law_id.clone(),
                    law_revision_id: law.revision_id.clone(),
                    law_title: law.title.clone(),
                    source: source.clone(),
                    scope: String::new(),
                    targets,
                    quoted: f.quoted,
                    replacement: String::new(),
                    nearest: String::new(),
                    similarity: 0.0,
                    confidence: f.confidence.to_string(),
                    context: f.context,
                    actual: f.actual,
                    target_law_id: String::new(),
                    target_law_title: String::new(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law(body: &str) -> Law {
        let xml = format!(
            "<Law><LawBody><LawTitle>テスト規則</LawTitle><MainProvision>{body}</MainProvision></LawBody></Law>"
        );
        Law::from_xml(&xml, "999M60000000001_20260101_000000000000000").expect("the fixture parses")
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

    fn found(body: &str) -> Vec<(String, String, String, String)> {
        check(&law(body))
            .into_iter()
            .map(|w| (w.rule, w.status, w.confidence, w.quoted))
            .collect()
    }

    #[test]
    fn reference_to_a_paragraph_the_article_before_lacks_is_reported() {
        let body = [
            article(
                "45",
                &[
                    "申請書を提出しなければならない。",
                    "図面を添付しなければならない。",
                ],
            ),
            article("45_2", &["国土交通省令で定める者は、次のとおりとする。"]),
            article("46", &["前条第二項第一号の図面に明示しなければならない。"]),
        ]
        .concat();
        assert_eq!(
            found(&body),
            [(
                "relative-ref".to_string(),
                "no-paragraph".to_string(),
                "high".to_string(),
                "前条第二項".to_string()
            )]
        );
    }

    #[test]
    fn reference_that_the_law_has_gives_no_warning() {
        let body = [
            article("1", &["次条第二項の規定による。", "前項の場合に準用する。"]),
            article(
                "2",
                &["甲とする。", "前項において準用する前条第九項の規定による。"],
            ),
        ]
        .concat();
        assert!(found(&body).is_empty());
    }

    #[test]
    fn next_paragraph_in_the_last_paragraph_is_reported() {
        let body = article(
            "1",
            &["職員（委員を含む。次項において同じ。）は、秘密を漏らしてはならない。"],
        );
        assert_eq!(found(&body)[0].3, "次項");
    }

    #[test]
    fn law_without_articles_is_read_by_its_paragraphs() {
        let body = "<Paragraph Num=\"1\"><ParagraphSentence>法第三条の政令で定める額は、百万円とする。</ParagraphSentence></Paragraph><Paragraph Num=\"2\"><ParagraphSentence>前二項の額は、毎年度改定する。</ParagraphSentence></Paragraph>";
        let law = law(body);
        assert_eq!(law.articles.len(), 1);
        assert_eq!(law.articles[0].num, "");
        let warnings = check(&law);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].quoted, "前二項");
        assert_eq!(warnings[0].source.article, "");
        assert_eq!(warnings[0].source.paragraph, 2);
    }

    #[test]
    fn reference_without_its_unit_is_reported() {
        let body = article(
            "1",
            &["法第九十八第二項第二号の方法は、別表第一第二号に掲げるものとする。"],
        );
        assert_eq!(
            found(&body),
            [(
                "missing-unit".to_string(),
                "missing-unit".to_string(),
                "high".to_string(),
                "第九十八第二項".to_string()
            )]
        );
    }
}
