use std::fmt::Write;

use crate::law::Law;
use crate::read_as::{self, Warning};

/// The code points of a text as a Lean list of numerals.
fn points(text: &str) -> String {
    let numbers: Vec<String> = text.chars().map(|c| (c as u32).to_string()).collect();
    format!("[{}]", numbers.join(", "))
}

fn excerpt(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= 120 {
        text.to_string()
    } else {
        format!("{}…", chars[..120].iter().collect::<String>())
    }
}

fn provision(article: &str, paragraph: u64) -> String {
    let article = if article.is_empty() {
        "the main provisions".to_string()
    } else {
        format!("article {}", article.replace('_', "-"))
    };
    if paragraph > 0 {
        format!("{article} paragraph {paragraph}")
    } else {
        article
    }
}

/// The Lean theorems that state, for each high-confidence `read-as-quote` warning of a law
/// whose quoted words are absent from a provision the law has, that the clause is vacuous on
/// the text of that provision.
pub fn theorems(law: &Law, warnings: &[Warning]) -> String {
    let mut out = String::new();
    for w in warnings {
        if w.rule != read_as::RULE
            || w.confidence != "high"
            || !matches!(w.status.as_str(), "absent" | "other-paragraph")
        {
            continue;
        }
        let quoted = w.quoted.replace('／', "");
        for (k, target) in w.targets.iter().enumerate() {
            let Some(article) = law.article(&target.article) else {
                continue;
            };
            let text = if target.paragraph > 0 {
                match article.paragraph(target.paragraph) {
                    Some(p) => &p.text,
                    None => continue,
                }
            } else {
                &article.text
            };
            if text.contains(&quoted) {
                continue;
            }
            let suffix = if w.targets.len() > 1 {
                format!("_{}", k + 1)
            } else {
                String::new()
            };
            let _ = write!(
                out,
                "\n/-- {} ({}), revision {}: the read-as clause in {} quotes words that {} does not\nhave.\n\n- quoted: {}\n- read as: {}\n- provision: {} -/\ntheorem finding_{}{} :\n    Clause.Vacuous\n      ⟨{},\n       {}⟩\n      {} :=\n  vacuous_of_absent _ _ (by decide +kernel)\n",
                w.law_title,
                w.law_id,
                w.law_revision_id,
                provision(&w.source.article, w.source.paragraph),
                provision(&target.article, target.paragraph),
                quoted,
                w.replacement,
                excerpt(text),
                w.id,
                suffix,
                points(&quoted),
                points(&w.replacement),
                points(text),
            );
        }
    }
    out
}

pub const HEADER: &str = "import Ritsulint.ReadAs\n\n/-!\n# Read-as clauses of the laws in force that change nothing\n\n`ritsulint corpus write-findings` writes this file from the e-Gov law XML: one theorem for each\nhigh-confidence `read-as-quote` warning. Each states that the clause, applied to the text of the\nprovision it names, leaves the text as it is, and the kernel checks it. A text is the list of the\ncode points of its characters; the comment of a theorem gives the words. The name of a theorem\nends in the ID of the warning.\n-/\n\nset_option maxRecDepth 100000\n\nnamespace Ritsulint.Findings\n\nopen Ritsulint.ReadAs\n";

pub const FOOTER: &str = "\nend Ritsulint.Findings\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_with_absent_words_becomes_a_theorem_over_code_points() {
        let xml = "<Law><LawBody><LawTitle>テスト法</LawTitle><MainProvision><Article Num=\"1\"><Paragraph Num=\"1\"><ParagraphSentence>甲は、乙とする。ただし、丙を除く。</ParagraphSentence></Paragraph></Article><Article Num=\"2\"><Paragraph Num=\"1\"><ParagraphSentence>前条の規定は、丁について準用する。この場合において、第一条中「乙とする。但し」とあるのは「戊とする。ただし」と読み替えるものとする。</ParagraphSentence></Paragraph></Article></MainProvision></LawBody></Law>";
        let law = Law::from_xml(xml, "999AC0000000001_20260101_000000000000000")
            .expect("the fixture parses");
        let (_, warnings, _) = read_as::check(&law);
        let text = theorems(&law, &warnings);
        assert_eq!(text.matches("theorem finding_").count(), 1);
        assert!(text.contains("⟨[20057, 12392, 12377, 12427, 12290, 20294, 12375],"));
        assert!(text.contains("vacuous_of_absent _ _ (by decide +kernel)"));
    }
}
