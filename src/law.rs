use std::collections::HashMap;
use std::fs;
use std::path::Path;

use roxmltree::{Document, Node};

#[derive(Debug)]
pub struct Paragraph {
    pub num: u64,
    pub text: String,
}

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
        let mut articles = Vec::new();
        if let Some(main) = doc.descendants().find(|n| n.has_tag_name("MainProvision")) {
            for node in main.descendants().filter(|n| n.has_tag_name("Article")) {
                let num = node.attribute("Num").unwrap_or_default().to_string();
                let paragraphs = node
                    .children()
                    .filter(|n| n.has_tag_name("Paragraph"))
                    .map(|p| Paragraph {
                        num: p.attribute("Num").and_then(|n| n.parse().ok()).unwrap_or(0),
                        text: text_of(p),
                    })
                    .collect();
                articles.push(Article {
                    num,
                    text: text_of(node),
                    paragraphs,
                });
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
