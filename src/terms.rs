use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::law::Law;
use crate::read_as::{Provision, Warning, plain};

pub const NEAR_RULE: &str = "near-term";
pub const SCOPE_RULE: &str = "definition-scope";
pub const UNUSED_RULE: &str = "unused-definition";

static DEFINITION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "以下([^「」。（）]{0,40}?において)?(?:単に)?「([^「」]{2,30})」(?:という|と総称する)",
    )
    .expect("the pattern is valid")
});

struct Definition {
    term: String,
    scope: Option<String>,
    article: usize,
    source: Provision,
}

fn definitions(law: &Law) -> Vec<Definition> {
    let mut out = Vec::new();
    for (index, article) in law.articles.iter().enumerate() {
        for paragraph in &article.paragraphs {
            for m in DEFINITION.captures_iter(&paragraph.text) {
                out.push(Definition {
                    term: m[2].to_string(),
                    scope: m.get(1).map(|s| s.as_str().to_string()),
                    article: index,
                    source: Provision {
                        article: article.num.clone(),
                        paragraph: paragraph.num,
                    },
                });
            }
        }
    }
    out
}

fn around(chars: &[char], at: usize, length: usize) -> String {
    let from = at.saturating_sub(20);
    let to = (at + length + 20).min(chars.len());
    chars[from..to].iter().collect()
}

fn count(chars: &[char], word: &[char]) -> usize {
    if word.is_empty() || chars.len() < word.len() {
        return 0;
    }
    (0..=chars.len() - word.len())
        .filter(|&i| chars[i] == word[0] && chars[i..i + word.len()] == *word)
        .count()
}

/// Characters that tell sibling terms apart (第一種… and 第二種…) or belong to an ordinary
/// phrase around a term (研修の修了者 beside 研修修了者).
fn is_numeral(c: char) -> bool {
    "〇一二三四五六七八九十百千甲乙丙丁前後新旧上下各当該等".contains(c)
        || c.is_ascii_digit()
        || ('０'..='９').contains(&c)
        || ('ぁ'..='ん').contains(&c)
        || !c.is_alphanumeric()
}

/// A word one character away from a defined term: one character replaced, left out or added,
/// not at either end. The word must be rare where the term is common.
fn near(term: &[char], chars: &[char], defined: &HashSet<String>) -> Option<(usize, String)> {
    let len = term.len();
    let mut found: HashMap<String, usize> = HashMap::new();
    for p in 0..chars.len().saturating_sub(len) {
        if chars[p] != term[0] || chars[p + 1] != term[1] || chars[p..].starts_with(term) {
            continue;
        }
        let window = &chars[p..(p + len + 1).min(chars.len())];
        let replaced = window.len() >= len && {
            let differs: Vec<usize> = (0..len).filter(|&k| window[k] != term[k]).collect();
            differs.len() == 1
                && differs[0] < len - 1
                && !is_numeral(window[differs[0]])
                && !is_numeral(term[differs[0]])
        };
        let left_out = (2..len - 1).any(|k| {
            window.len() >= len - 1
                && window[..k] == term[..k]
                && window[k..len - 1] == term[k + 1..]
                && !is_numeral(term[k])
        });
        let added = window.len() > len
            && (2..len).any(|k| {
                window[..k] == term[..k] && window[k + 1..] == term[k..] && !is_numeral(window[k])
            });
        let word: Option<String> = if replaced {
            Some(window[..len].iter().collect())
        } else if left_out {
            Some(window[..len - 1].iter().collect())
        } else if added {
            Some(window.iter().collect())
        } else {
            None
        };
        let length = word.as_ref().map_or(0, |w| w.chars().count());
        let defining = chars[(p + length).min(chars.len())..].starts_with(&['（', '以', '下'])
            || chars[..p].ends_with(&['規', '定', 'す', 'る']);
        if let Some(word) = word
            && !defined.contains(&word)
            && !defining
        {
            found.entry(word).or_insert(p);
        }
    }
    let uses = count(chars, term);
    found
        .into_iter()
        .filter(|(word, _)| {
            let w: Vec<char> = word.chars().collect();
            let n = count(chars, &w);
            n <= 2 && uses >= 3 * n
        })
        .map(|(word, at)| (at, word))
        .min()
}

/// How often a term is written in the articles other than those that define it.
fn outside_uses(law: &Law, homes: &HashSet<usize>, term: &str) -> usize {
    law.articles
        .iter()
        .enumerate()
        .filter(|(i, _)| !homes.contains(i))
        .map(|(_, a)| a.text.matches(term).count())
        .sum()
}

/// Words one character away from a defined term, terms defined for one article and used in
/// another, and definitions never used.
pub fn check(law: &Law) -> Vec<Warning> {
    let defs = definitions(law);
    if defs.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let defined: HashSet<String> = defs.iter().map(|d| d.term.clone()).collect();
    let mut starts = Vec::new();
    let mut main: Vec<char> = Vec::new();
    for article in &law.articles {
        for paragraph in &article.paragraphs {
            starts.push((main.len(), article.num.clone(), paragraph.num));
            main.extend(paragraph.prose.chars());
            main.push('\n');
        }
    }
    let locate = |at: usize| {
        let i = starts
            .partition_point(|(s, _, _)| *s <= at)
            .saturating_sub(1);
        Provision {
            article: starts[i].1.clone(),
            paragraph: starts[i].2,
        }
    };
    let mut seen = HashSet::new();
    for d in &defs {
        if !seen.insert(d.term.clone()) {
            continue;
        }
        let term: Vec<char> = d.term.chars().collect();
        if term.len() >= 5
            && let Some((at, word)) = near(&term, &main, &defined)
        {
            let length = word.chars().count();
            let source = locate(at);
            let occurrence = out
                .iter()
                .filter(|w: &&Warning| {
                    w.rule == NEAR_RULE && w.source == source && w.quoted == word
                })
                .count();
            out.push(plain(
                law,
                NEAR_RULE,
                "near-term",
                "low",
                source,
                word,
                around(&main, at, length),
                d.term.clone(),
                occurrence,
            ));
        }
        let uses = law.full_text.matches(d.term.as_str()).count();
        let quoted = law.full_text.matches(&format!("「{}」", d.term)).count();
        if uses == quoted {
            out.push(plain(
                law,
                UNUSED_RULE,
                "unused-definition",
                "medium",
                d.source.clone(),
                d.term.clone(),
                String::new(),
                String::new(),
                0,
            ));
        }
    }
    let wider: HashSet<&str> = defs
        .iter()
        .filter(|d| d.scope.as_deref() != Some("この条において"))
        .map(|d| d.term.as_str())
        .collect();
    let mut scoped = HashSet::new();
    for d in &defs {
        if d.scope.as_deref() != Some("この条において")
            || d.term.chars().count() < 4
            || wider.contains(d.term.as_str())
            || !scoped.insert(d.term.clone())
        {
            continue;
        }
        let homes: HashSet<usize> = defs
            .iter()
            .filter(|x| x.term == d.term)
            .map(|x| x.article)
            .collect();
        let mark = format!("「{}」", d.term);
        for (index, article) in law.articles.iter().enumerate() {
            if homes.contains(&index) || article.text.contains(&mark) {
                continue;
            }
            let term: Vec<char> = d.term.chars().collect();
            let bare = |chars: &[char]| {
                (0..chars.len()).find(|&i| {
                    chars[i..].starts_with(&term)
                        && !chars[..i].last().is_some_and(|c| {
                            ('一'..='龥').contains(c) || ('ァ'..='ヶ').contains(c) || *c == 'ー'
                        })
                        && !chars[..i].ends_with(&['規', '定', 'す', 'る'])
                        && !chars[..i].ends_with(&['定', 'め', 'る'])
                })
            };
            let Some((paragraph, chars, at)) = article.paragraphs.iter().find_map(|p| {
                let chars: Vec<char> = p.prose.chars().collect();
                let at = bare(&chars)?;
                Some((p, chars, at))
            }) else {
                continue;
            };
            if outside_uses(law, &homes, &d.term) > 3 {
                break;
            }
            out.push(plain(
                law,
                SCOPE_RULE,
                "definition-scope",
                "low",
                Provision {
                    article: article.num.clone(),
                    paragraph: paragraph.num,
                },
                d.term.clone(),
                around(&chars, at, term.len()),
                d.source.article.clone(),
                0,
            ));
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law(articles: &[&str]) -> Law {
        let body: String = articles
            .iter()
            .enumerate()
            .map(|(i, text)| {
                format!(
                    "<Article Num=\"{}\"><Paragraph Num=\"1\"><ParagraphSentence>{text}</ParagraphSentence></Paragraph></Article>",
                    i + 1
                )
            })
            .collect();
        let xml = format!(
            "<Law><LawBody><LawTitle>テスト規則</LawTitle><MainProvision>{body}</MainProvision></LawBody></Law>"
        );
        Law::from_xml(&xml, "999M60000000001_20260101_000000000000000").expect("the fixture parses")
    }

    fn found(articles: &[&str]) -> Vec<(String, String)> {
        check(&law(articles))
            .into_iter()
            .map(|w| (w.rule, w.quoted))
            .collect()
    }

    #[test]
    fn word_that_leaves_out_a_character_of_a_defined_term_is_reported() {
        let articles = [
            "認定（以下「限度額適用認定」という。）を受けた者は、限度額適用認定証の交付を受ける。",
            "限度額適用認定を受けた者が限度額適用認定証を提出したときは、限度適用認定の効力を確認する。",
        ];
        assert_eq!(
            found(&articles),
            [("near-term".to_string(), "限度適用認定".to_string())]
        );
    }

    #[test]
    fn definition_that_is_never_used_is_reported() {
        let articles = [
            "大臣が指定する者（以下「指定検査機関」という。）は、検査を行う。",
            "検査は、毎年行う。",
        ];
        assert_eq!(
            found(&articles),
            [("unused-definition".to_string(), "指定検査機関".to_string())]
        );
    }

    #[test]
    fn term_defined_for_one_article_and_used_in_another_is_reported() {
        let articles = [
            "届出をした者（以下この条において「特定届出者」という。）は、特定届出者である旨を表示する。",
            "特定届出者は、帳簿を備えなければならない。",
        ];
        assert_eq!(
            found(&articles),
            [("definition-scope".to_string(), "特定届出者".to_string())]
        );
    }
}
