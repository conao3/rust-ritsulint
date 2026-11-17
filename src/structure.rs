use std::collections::HashSet;

use crate::generated::ritsulint_api as core;
use crate::law::Law;
use crate::read_as::{Provision, Warning, warning_id};

pub const NUMBER_RULE: &str = "numbering";
pub const TOC_RULE: &str = "toc-range";

type Key = Vec<u64>;

fn key(num: &str) -> Option<(Key, Key)> {
    let part = |s: &str| {
        s.split('_')
            .map(|x| x.parse().ok())
            .collect::<Option<Key>>()
    };
    let mut ends = num.split(':');
    let lo = part(ends.next()?)?;
    let hi = ends.next().and_then(part).unwrap_or_else(|| lo.clone());
    Some((lo, hi))
}

fn kansuji(text: &str) -> Option<u64> {
    let n = u64::try_from(core::kansuji(text)).ok()?;
    (n > 0).then_some(n)
}

fn is_kansuji(c: char) -> bool {
    "一二三四五六七八九十百千".contains(c)
}

/// The articles a range of the table of contents writes (`第三条―第七条の二`), in order.
fn articles_in(range: &str) -> Vec<Key> {
    let chars: Vec<char> = range.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '第' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && is_kansuji(chars[j]) {
            j += 1;
        }
        let head: String = chars[i + 1..j].iter().collect();
        match (kansuji(&head), chars.get(j)) {
            (Some(n), Some('条')) => {
                let mut found = vec![n];
                j += 1;
                while matches!(chars.get(j), Some('の' | 'ノ')) {
                    let mut k = j + 1;
                    while k < chars.len() && is_kansuji(chars[k]) {
                        k += 1;
                    }
                    let branch: String = chars[j + 1..k].iter().collect();
                    match kansuji(&branch) {
                        Some(b) => {
                            found.push(b);
                            j = k;
                        }
                        None => break,
                    }
                }
                out.push(found);
                i = j;
            }
            _ => i += 1,
        }
    }
    out
}

fn show(key: &Key) -> String {
    key.iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join("_")
}

struct Found {
    rule: &'static str,
    status: &'static str,
    source: Provision,
    quoted: String,
    context: String,
    actual: String,
}

/// Paragraph and item numbers that occur twice or skip a number, and ranges of the table of
/// contents that leave out an article of the chapter or section they stand for.
pub fn check(law: &Law) -> Vec<Warning> {
    let mut found = Vec::new();
    for article in &law.articles {
        let mut seen = HashSet::new();
        for paragraph in &article.paragraphs {
            let source = Provision {
                article: article.num.clone(),
                paragraph: paragraph.num,
            };
            if !seen.insert(paragraph.num) {
                found.push(Found {
                    rule: NUMBER_RULE,
                    status: "duplicate-paragraph",
                    source: source.clone(),
                    quoted: paragraph.num.to_string(),
                    context: paragraph.prose.chars().take(60).collect(),
                    actual: String::new(),
                });
            }
            let mut items = HashSet::new();
            let mut before: Option<Key> = None;
            for item in &paragraph.items {
                if !items.insert(item.num.as_str()) {
                    found.push(Found {
                        rule: NUMBER_RULE,
                        status: "duplicate-item",
                        source: source.clone(),
                        quoted: item.num.clone(),
                        context: item.sentence.chars().take(60).collect(),
                        actual: String::new(),
                    });
                }
                let Some((lo, hi)) = key(&item.num) else {
                    continue;
                };
                if let Some(prev) = &before
                    && lo[0] > prev[0] + 1
                {
                    found.push(Found {
                        rule: NUMBER_RULE,
                        status: "item-gap",
                        source: source.clone(),
                        quoted: item.num.clone(),
                        context: item.sentence.chars().take(60).collect(),
                        actual: format!("after item {}", show(prev)),
                    });
                }
                before = Some(match before {
                    Some(prev) if prev > hi => prev,
                    _ => hi,
                });
            }
        }
    }
    for entry in &law.toc {
        let Some((first, last)) = &entry.body else {
            continue;
        };
        let (Some((first, _)), Some((_, last))) = (key(first), key(last)) else {
            continue;
        };
        let written = articles_in(&entry.range);
        let (Some(from), Some(to)) = (written.first(), written.last()) else {
            continue;
        };
        if &first < from || &last > to {
            let malformed = written.len() == 1 && entry.range.contains('―');
            found.push(Found {
                rule: TOC_RULE,
                status: if malformed {
                    "toc-malformed"
                } else {
                    "toc-range"
                },
                source: Provision {
                    article: show(&first),
                    paragraph: 0,
                },
                quoted: entry.range.clone(),
                context: entry.label.clone(),
                actual: format!("{}..{}", show(&first), show(&last)),
            });
        }
    }
    let mut out: Vec<Warning> = Vec::new();
    for f in found {
        let occurrence = out
            .iter()
            .filter(|w| w.rule == f.rule && w.source == f.source && w.quoted == f.quoted)
            .count();
        out.push(Warning {
            id: warning_id(
                f.rule,
                &law.law_id,
                &f.source,
                f.status,
                &[],
                &f.quoted,
                occurrence,
            ),
            rule: f.rule.to_string(),
            status: f.status.to_string(),
            law_id: law.law_id.clone(),
            law_revision_id: law.revision_id.clone(),
            law_title: law.title.clone(),
            source: f.source,
            scope: String::new(),
            targets: Vec::new(),
            quoted: f.quoted,
            replacement: String::new(),
            nearest: String::new(),
            similarity: 0.0,
            confidence: if f.status == "item-gap" {
                "medium"
            } else {
                "high"
            }
            .to_string(),
            context: f.context,
            actual: f.actual,
            target_law_id: String::new(),
            target_law_title: String::new(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law(toc: &str, body: &str) -> Law {
        let xml = format!(
            "<Law><LawNum>令和八年法律第一号</LawNum><LawBody><LawTitle>テスト法</LawTitle>{toc}<MainProvision>{body}</MainProvision></LawBody></Law>"
        );
        Law::from_xml(&xml, "999AC0000000001_20260101_000000000000000").expect("the fixture parses")
    }

    fn article(num: &str) -> String {
        format!(
            "<Article Num=\"{num}\"><Paragraph Num=\"1\"><ParagraphSentence>本文</ParagraphSentence></Paragraph></Article>"
        )
    }

    #[test]
    fn range_of_the_table_of_contents_that_leaves_out_the_last_article_is_reported() {
        let toc = "<TOC><TOCChapter Num=\"1\"><ChapterTitle>第一章　総則</ChapterTitle><ArticleRange>（第一条―第二条の二）</ArticleRange></TOCChapter></TOC>";
        let body = format!(
            "<Chapter Num=\"1\">{}{}{}{}</Chapter>",
            article("1"),
            article("2"),
            article("2_2"),
            article("2_3")
        );
        let found = check(&law(toc, &body));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].status, "toc-range");
        assert_eq!(found[0].actual, "1..2_3");
    }

    #[test]
    fn range_that_covers_the_chapter_gives_no_warning() {
        let toc = "<TOC><TOCChapter Num=\"1\"><ChapterTitle>第一章　総則</ChapterTitle><ArticleRange>（第一条・第二条ノ二）</ArticleRange></TOCChapter></TOC>";
        let body = format!(
            "<Chapter Num=\"1\">{}{}</Chapter>",
            article("1"),
            article("2_2")
        );
        assert!(check(&law(toc, &body)).is_empty());
    }

    #[test]
    fn item_number_written_twice_is_reported() {
        let body = "<Article Num=\"1\"><Paragraph Num=\"1\"><ParagraphSentence>次のとおりとする。</ParagraphSentence><Item Num=\"1\"><ItemSentence>甲</ItemSentence></Item><Item Num=\"2\"><ItemSentence>乙</ItemSentence></Item><Item Num=\"2\"><ItemSentence>丙</ItemSentence></Item><Item Num=\"4\"><ItemSentence>丁</ItemSentence></Item></Paragraph></Article>";
        let statuses: Vec<String> = check(&law("", body))
            .into_iter()
            .map(|w| w.status)
            .collect();
        assert_eq!(statuses, ["duplicate-item", "item-gap"]);
    }
}
