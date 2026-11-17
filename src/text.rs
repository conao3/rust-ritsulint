use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::generated::ritsulint_api as core;
use crate::law::Law;
use crate::read_as::{Provision, Warning, plain};

pub const BRACKET_RULE: &str = "brackets";
pub const SPELLING_RULE: &str = "spelling";
pub const TERM_RULE: &str = "referenced-term";

static ARTICLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(^|[^法令則程範約款）号])第([一二三四五六七八九十百千]+)条((?:の[一二三四五六七八九十百千]+)*)")
        .expect("the pattern is valid")
});

/// The article `同条` stands for: the last article the text before it names without the name of
/// a law, as the `Num` attribute.
fn same_article(before: &str) -> Option<String> {
    let last = ARTICLE.captures_iter(before).last()?;
    let end = last.get(0)?.end();
    if before[end..].contains('条') {
        return None;
    }
    let mut key = u64::try_from(core::kansuji(&last[2])).ok()?.to_string();
    for branch in last[3].split('の').filter(|b| !b.is_empty()) {
        key.push('_');
        key.push_str(&u64::try_from(core::kansuji(branch)).ok()?.to_string());
    }
    Some(key)
}

static KATAKANA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("[ァ-ヶー]{4,}").expect("the pattern is valid"));

static REFERENCED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(前条|次条|同条)?(前項|第([一二三四五六七八九十百]+)項)に規定する([\\p{Han}ァ-ヶー]{2,20})",
    )
    .expect("the pattern is valid")
});

fn large(c: char) -> char {
    match c {
        'ッ' => 'ツ',
        'ャ' => 'ヤ',
        'ュ' => 'ユ',
        'ョ' => 'ヨ',
        'ァ' => 'ア',
        'ィ' => 'イ',
        'ゥ' => 'ウ',
        'ェ' => 'エ',
        'ォ' => 'オ',
        c => c,
    }
}

/// `Some(description)` when the quotation marks of a text do not pair, or its parentheses
/// outside quotations do not. A quotation may hold half of a pair of parentheses.
fn unpaired(text: &str) -> Option<String> {
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '「' => depth += 1,
            '」' if depth == 0 => return Some("」 without 「".to_string()),
            '」' => depth -= 1,
            _ => {}
        }
    }
    if depth > 0 {
        return Some("「 not closed".to_string());
    }
    let mut depth = 0usize;
    for c in core::unquote(text).chars() {
        match c {
            '（' => depth += 1,
            '）' if depth == 0 => return Some("） without （".to_string()),
            '）' => depth -= 1,
            _ => {}
        }
    }
    (depth > 0).then(|| "（ not closed".to_string())
}

const SUFFIXES: [&str; 12] = [
    "以外", "以内", "以上", "以下", "内", "外", "等", "中", "後", "前", "時", "分",
];
const GENERIC: [&str; 24] = [
    "手続", "場合", "事項", "措置", "規定", "方法", "基準", "要件", "期間", "期限", "金額", "書類",
    "行為", "事由", "区分", "割合", "事務", "業務", "権限", "範囲", "条件", "事業", "職員", "機関",
];

fn snippet(text: &str, part: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let at = text.find(part).map_or(0, |b| text[..b].chars().count());
    let from = at.saturating_sub(20);
    let to = (at + part.chars().count() + 20).min(chars.len());
    chars[from..to].iter().collect()
}

/// Brackets that do not pair in a paragraph, katakana words written with both a small and a
/// large kana in one law, and `第N項に規定するX` where the paragraph named has no `X`.
pub fn check(law: &Law) -> Vec<Warning> {
    let mut out = Vec::new();
    let mut words: BTreeMap<String, BTreeMap<String, (usize, Provision, String)>> = BTreeMap::new();
    for (index, article) in law.articles.iter().enumerate() {
        for (position, paragraph) in article.paragraphs.iter().enumerate() {
            let source = Provision {
                article: article.num.clone(),
                paragraph: paragraph.num,
            };
            if !paragraph.text.contains("とあるのは")
                && let Some(what) = unpaired(&paragraph.prose)
            {
                out.push(plain(
                    law,
                    BRACKET_RULE,
                    "unpaired",
                    "medium",
                    source.clone(),
                    what.chars().take(1).collect(),
                    paragraph.prose.chars().take(80).collect(),
                    what,
                    0,
                ));
            }
            for m in KATAKANA.find_iter(&paragraph.prose) {
                let word = m.as_str();
                let key: String = word.chars().map(large).collect();
                let entry = words
                    .entry(key)
                    .or_default()
                    .entry(word.to_string())
                    .or_insert_with(|| (0, source.clone(), snippet(&paragraph.prose, word)));
                entry.0 += 1;
            }
            if paragraph.text.contains("とあるのは") {
                continue;
            }
            let prose = core::unquote(&paragraph.prose);
            let mut occurrence = 0;
            for m in REFERENCED.captures_iter(&prose) {
                let whole = m.get(0).expect("the match exists");
                let before = &prose[..whole.start()];
                let number = match m.get(3) {
                    Some(n) => u64::try_from(core::kansuji(n.as_str())).unwrap_or(0),
                    None => 0,
                };
                let target = match (m.get(1).map(|x| x.as_str()), number) {
                    (Some("前条"), n) if n > 0 => index
                        .checked_sub(1)
                        .and_then(|i| law.articles.get(i))
                        .and_then(|a| a.paragraph(n)),
                    (Some("次条"), n) if n > 0 => {
                        law.articles.get(index + 1).and_then(|a| a.paragraph(n))
                    }
                    (Some("同条"), n) if n > 0 => same_article(before)
                        .and_then(|key| law.article(&key))
                        .and_then(|a| a.paragraph(n)),
                    (None, 0) => position
                        .checked_sub(1)
                        .and_then(|i| article.paragraphs.get(i)),
                    (None, n) => {
                        let outside = ["条", "同項", "附則", "別表", "法", "令", "則"]
                            .iter()
                            .any(|w| before.contains(w));
                        if outside || before.ends_with('び') || before.ends_with('、') {
                            None
                        } else {
                            article.paragraph(n)
                        }
                    }
                    _ => None,
                };
                let Some(target) = target else {
                    continue;
                };
                let mut term = &m[4];
                if let Some(at) = term.find(['第', '又', '及', '若', '並']) {
                    term = &term[..at];
                }
                for lead in ["各", "当該"] {
                    term = term.strip_prefix(lead).unwrap_or(term);
                }
                if let Some(cut) = SUFFIXES.iter().find_map(|x| term.strip_suffix(x)) {
                    term = cut;
                }
                let length = term.chars().count();
                if length < 5
                    || GENERIC.iter().any(|g| term.ends_with(g))
                    || before.ends_with("及び")
                    || before.ends_with("又は")
                {
                    continue;
                }
                let shorter: String = term.chars().take(length - 1).collect();
                let stem: String = term.chars().take(2).collect();
                if target.text.contains(term) || target.text.contains(&shorter) {
                    continue;
                }
                out.push(plain(
                    law,
                    TERM_RULE,
                    "term-absent",
                    if target.text.contains(&stem) {
                        "low"
                    } else {
                        "medium"
                    },
                    source.clone(),
                    whole.as_str().to_string(),
                    snippet(&prose, whole.as_str()),
                    target.num.to_string(),
                    occurrence,
                ));
                occurrence += 1;
            }
        }
    }
    for forms in words.values().filter(|f| f.len() > 1) {
        let common = forms
            .iter()
            .max_by_key(|(_, (n, _, _))| *n)
            .map(|(w, _)| w.clone())
            .unwrap_or_default();
        for (word, (n, source, context)) in forms {
            if *word != common {
                out.push(plain(
                    law,
                    SPELLING_RULE,
                    "mixed-kana",
                    "medium",
                    source.clone(),
                    word.clone(),
                    context.clone(),
                    format!("{common} ({} times), {word} ({n} times)", forms[&common].0),
                    0,
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law(articles: &[&[&str]]) -> Law {
        let body: String = articles
            .iter()
            .enumerate()
            .map(|(i, paragraphs)| {
                let inner: String = paragraphs
                    .iter()
                    .enumerate()
                    .map(|(j, text)| {
                        format!(
                            "<Paragraph Num=\"{}\"><ParagraphSentence>{text}</ParagraphSentence></Paragraph>",
                            j + 1
                        )
                    })
                    .collect();
                format!("<Article Num=\"{}\">{inner}</Article>", i + 1)
            })
            .collect();
        let xml = format!(
            "<Law><LawBody><LawTitle>テスト規則</LawTitle><MainProvision>{body}</MainProvision></LawBody></Law>"
        );
        Law::from_xml(&xml, "999M60000000001_20260101_000000000000000").expect("the fixture parses")
    }

    fn found(articles: &[&[&str]]) -> Vec<(String, String)> {
        check(&law(articles))
            .into_iter()
            .map(|w| (w.rule, w.quoted))
            .collect()
    }

    #[test]
    fn parenthesis_that_is_not_closed_is_reported() {
        let articles: [&[&str]; 1] =
            [&["事業者（外国法人を除く。以下同じ。は、届け出なければならない。"]];
        assert_eq!(
            found(&articles),
            [("brackets".to_string(), "（".to_string())]
        );
    }

    #[test]
    fn half_a_pair_inside_a_quotation_gives_no_warning() {
        let articles: [&[&str]; 1] = [&["帳簿には「氏名（法人にあっては」の次に名称を記載する。"]];
        assert!(found(&articles).is_empty());
    }

    #[test]
    fn word_written_with_a_small_and_a_large_kana_is_reported() {
        let articles: [&[&str]; 2] = [
            &[
                "インターネットを利用して公表する。",
                "インターネットによる公表は、毎年行う。",
            ],
            &["インターネツトを利用する方法とする。"],
        ];
        assert_eq!(
            found(&articles),
            [("spelling".to_string(), "インターネツト".to_string())]
        );
    }

    #[test]
    fn term_the_named_paragraph_lacks_is_reported() {
        let articles: [&[&str]; 1] = [&[
            "市町村は、特別徴収対象年金給付の支払をする者に通知する。",
            "前項に規定する特別徴収対象被保険者は、届け出なければならない。",
        ]];
        assert_eq!(
            found(&articles),
            [(
                "referenced-term".to_string(),
                "前項に規定する特別徴収対象被保険者".to_string()
            )]
        );
    }
}
