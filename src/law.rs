use std::collections::HashMap;
use std::fs;
use std::path::Path;

use roxmltree::{Document, Node};

#[derive(Debug)]
pub struct Item {
    pub num: String,
    pub sentence: String,
}

#[derive(Debug)]
pub struct Paragraph {
    pub num: u64,
    pub text: String,
    /// The text without numbers, titles, captions, tables, figures and forms.
    pub prose: String,
    pub items: Vec<Item>,
}

/// An entry of the table of contents that gives a range of articles, with the first and last
/// article of the part of the main provisions it stands for.
#[derive(Debug)]
pub struct TocEntry {
    pub label: String,
    pub range: String,
    pub body: Option<(String, String)>,
}

/// An article. `num` is empty for the one article that stands for the main provisions of a law
/// that is not divided into articles.
#[derive(Debug)]
pub struct Article {
    pub num: String,
    pub text: String,
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug)]
pub struct Law {
    pub law_id: String,
    pub revision_id: String,
    pub title: String,
    pub law_num: String,
    pub toc: Vec<TocEntry>,
    /// The whole text of the law: main and supplementary provisions, tables and forms.
    pub full_text: String,
    pub articles: Vec<Article>,
    index: HashMap<String, usize>,
}

#[derive(Debug)]
pub struct LawError {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for LawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

fn collect_text(node: Node, out: &mut String) {
    for child in node.children() {
        if child.is_text() {
            if let Some(t) = child.text() {
                out.extend(t.chars().filter(|c| !c.is_whitespace()));
            }
        } else if child.is_element() && child.tag_name().name() != "Rt" {
            collect_text(child, out);
        }
    }
}

fn text_of(node: Node) -> String {
    let mut out = String::new();
    collect_text(node, &mut out);
    out
}

const NOT_PROSE: [&str; 12] = [
    "Rt",
    "ParagraphNum",
    "ParagraphCaption",
    "ItemTitle",
    "Subitem1Title",
    "Subitem2Title",
    "Subitem3Title",
    "TableStruct",
    "Table",
    "FigStruct",
    "StyleStruct",
    "FormatStruct",
];

fn collect_prose(node: Node, out: &mut String) {
    for child in node.children() {
        if child.is_text() {
            if let Some(t) = child.text() {
                out.extend(t.chars().filter(|c| !c.is_whitespace()));
            }
        } else if child.is_element() && !NOT_PROSE.contains(&child.tag_name().name()) {
            collect_prose(child, out);
        }
    }
}

fn prose_of(node: Node) -> String {
    let mut out = String::new();
    collect_prose(node, &mut out);
    out
}

/// The paragraphs that are children of an article, or of the main provisions of a law that
/// is not divided into articles.
fn paragraphs_of(node: Node) -> Vec<Paragraph> {
    node.children()
        .filter(|n| n.has_tag_name("Paragraph"))
        .map(|p| Paragraph {
            num: p.attribute("Num").and_then(|n| n.parse().ok()).unwrap_or(0),
            text: text_of(p),
            prose: prose_of(p),
            items: p
                .children()
                .filter(|n| n.has_tag_name("Item"))
                .map(|i| Item {
                    num: i.attribute("Num").unwrap_or_default().to_string(),
                    sentence: i
                        .children()
                        .find(|n| n.has_tag_name("ItemSentence"))
                        .map(prose_of)
                        .unwrap_or_default(),
                })
                .collect(),
        })
        .collect()
}

fn body_tag(toc_tag: &str) -> Option<&'static str> {
    match toc_tag {
        "TOCPart" => Some("Part"),
        "TOCChapter" => Some("Chapter"),
        "TOCSection" => Some("Section"),
        "TOCSubsection" => Some("Subsection"),
        "TOCDivision" => Some("Division"),
        _ => None,
    }
}

fn collect_toc(toc: Node, body: Option<Node>, out: &mut Vec<TocEntry>) {
    for entry in toc.children().filter(|n| n.is_element()) {
        let Some(tag) = body_tag(entry.tag_name().name()) else {
            continue;
        };
        let found = body.and_then(|b| {
            b.children()
                .find(|n| n.has_tag_name(tag) && n.attribute("Num") == entry.attribute("Num"))
        });
        let nested = entry
            .children()
            .any(|n| n.is_element() && body_tag(n.tag_name().name()).is_some());
        if let Some(range) = entry.children().find(|n| n.has_tag_name("ArticleRange")) {
            let nums: Vec<&str> = found
                .map(|f| {
                    if nested {
                        f.children()
                            .filter(|n| n.has_tag_name("Article"))
                            .filter_map(|n| n.attribute("Num"))
                            .collect()
                    } else {
                        f.descendants()
                            .filter(|n| n.has_tag_name("Article"))
                            .filter_map(|n| n.attribute("Num"))
                            .collect()
                    }
                })
                .unwrap_or_default();
            out.push(TocEntry {
                label: entry
                    .children()
                    .find(|n| n.tag_name().name().ends_with("Title"))
                    .map(text_of)
                    .unwrap_or_default(),
                range: text_of(range),
                body: match (nums.first(), nums.last()) {
                    (Some(a), Some(b)) => Some((a.to_string(), b.to_string())),
                    _ => None,
                },
            });
        }
        collect_toc(entry, found, out);
    }
}

impl Law {
    pub fn from_file(path: &Path) -> Result<Law, LawError> {
        let err = |message: String| LawError {
            path: path.display().to_string(),
            message,
        };
        let revision_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| err("the file name is not valid UTF-8".to_string()))?
            .to_string();
        let source = fs::read_to_string(path).map_err(|e| err(e.to_string()))?;
        Law::from_xml(&source, &revision_id).map_err(err)
    }

    pub fn from_xml(source: &str, revision_id: &str) -> Result<Law, String> {
        let doc = Document::parse(source).map_err(|e| e.to_string())?;
        let law_id = revision_id
            .split('_')
            .next()
            .unwrap_or(revision_id)
            .to_string();
        let title = doc
            .descendants()
            .find(|n| n.has_tag_name("LawTitle"))
            .map(text_of)
            .unwrap_or_default();
        let law_num = doc
            .descendants()
            .find(|n| n.has_tag_name("LawNum"))
            .map(text_of)
            .unwrap_or_default();
        let mut toc = Vec::new();
        if let Some(node) = doc.descendants().find(|n| n.has_tag_name("TOC")) {
            collect_toc(
                node,
                doc.descendants().find(|n| n.has_tag_name("MainProvision")),
                &mut toc,
            );
        }
        let full_text = doc
            .descendants()
            .find(|n| n.has_tag_name("LawBody"))
            .map(text_of)
            .unwrap_or_default();
        let mut articles = Vec::new();
        if let Some(main) = doc.descendants().find(|n| n.has_tag_name("MainProvision")) {
            for node in main.descendants().filter(|n| n.has_tag_name("Article")) {
                articles.push(Article {
                    num: node.attribute("Num").unwrap_or_default().to_string(),
                    text: text_of(node),
                    paragraphs: paragraphs_of(node),
                });
            }
            if articles.is_empty() {
                let paragraphs = paragraphs_of(main);
                if !paragraphs.is_empty() {
                    articles.push(Article {
                        num: String::new(),
                        text: text_of(main),
                        paragraphs,
                    });
                }
            }
        }
        let mut index = HashMap::new();
        for (i, a) in articles.iter().enumerate() {
            index.entry(a.num.clone()).or_insert(i);
        }
        Ok(Law {
            law_id,
            revision_id: revision_id.to_string(),
            title,
            law_num,
            toc,
            full_text,
            articles,
            index,
        })
    }

    pub fn article(&self, num: &str) -> Option<&Article> {
        self.index.get(num).map(|&i| &self.articles[i])
    }
}

impl Article {
    pub fn paragraph(&self, num: u64) -> Option<&Paragraph> {
        self.paragraphs.iter().find(|p| p.num == num)
    }
}
