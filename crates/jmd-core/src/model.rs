//! The document model shared by the preview, the exporters and the server.

use serde::{Deserialize, Serialize};

/// A parsed Markdown document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// `key: value` pairs from a leading `---` front matter block, in order.
    pub front_matter: Vec<(String, String)>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
        /// Stable anchor id, unique within the document.
        id: String,
    },
    Paragraph {
        content: Vec<Inline>,
    },
    List {
        ordered: bool,
        start: usize,
        items: Vec<ListItem>,
    },
    CodeBlock {
        lang: String,
        code: String,
    },
    Quote {
        blocks: Vec<Block>,
    },
    /// A GitHub-style alert (`> [!NOTE]`).
    Alert {
        kind: String,
        blocks: Vec<Block>,
    },
    Table {
        align: Vec<Align>,
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Math {
        tex: String,
    },
    FootnoteDefinition {
        name: String,
        blocks: Vec<Block>,
    },
    Html {
        html: String,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    /// `Some(checked)` for task list items.
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    Text {
        text: String,
    },
    Strong {
        content: Vec<Inline>,
    },
    Emph {
        content: Vec<Inline>,
    },
    Strike {
        content: Vec<Inline>,
    },
    Code {
        code: String,
    },
    Link {
        url: String,
        title: String,
        content: Vec<Inline>,
    },
    Image {
        url: String,
        title: String,
        alt: String,
    },
    Math {
        tex: String,
        /// True for `$$ … $$`.
        display: bool,
    },
    FootnoteRef {
        name: String,
        number: u32,
    },
    Html {
        html: String,
    },
    SoftBreak,
    LineBreak,
}

impl Inline {
    pub fn text(s: impl Into<String>) -> Self {
        Inline::Text { text: s.into() }
    }
}

/// Concatenated visible text of a run of inlines (used for headings, TOC and alt text).
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    push_plain(inlines, &mut out);
    out
}

fn push_plain(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Code { code } => out.push_str(code),
            Inline::Math { tex, .. } => out.push_str(tex),
            Inline::Strong { content }
            | Inline::Emph { content }
            | Inline::Strike { content }
            | Inline::Link { content, .. } => push_plain(content, out),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::SoftBreak | Inline::LineBreak => out.push(' '),
            Inline::FootnoteRef { .. } | Inline::Html { .. } => {}
        }
    }
}

impl Document {
    /// The value of a front matter key, if present.
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.front_matter
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The document title: front matter `title`, else the first level-1 heading.
    pub fn title(&self) -> Option<String> {
        if let Some(t) = self.meta("title") {
            return Some(t.to_string());
        }
        self.blocks.iter().find_map(|b| match b {
            Block::Heading {
                level: 1, content, ..
            } => Some(plain_text(content)),
            _ => None,
        })
    }

    /// Every image URL in the document, in order.
    pub fn image_urls(&self) -> Vec<String> {
        let mut out = Vec::new();
        visit_inlines(&self.blocks, &mut |i| {
            if let Inline::Image { url, .. } = i {
                out.push(url.clone());
            }
        });
        out
    }
}

/// Calls `f` for every inline in `blocks`, depth first.
pub fn visit_inlines(blocks: &[Block], f: &mut dyn FnMut(&Inline)) {
    fn walk_inlines(inlines: &[Inline], f: &mut dyn FnMut(&Inline)) {
        for i in inlines {
            f(i);
            match i {
                Inline::Strong { content }
                | Inline::Emph { content }
                | Inline::Strike { content }
                | Inline::Link { content, .. } => walk_inlines(content, f),
                _ => {}
            }
        }
    }
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph { content } => {
                walk_inlines(content, f)
            }
            Block::List { items, .. } => {
                for item in items {
                    visit_inlines(&item.blocks, f);
                }
            }
            Block::Quote { blocks }
            | Block::Alert { blocks, .. }
            | Block::FootnoteDefinition { blocks, .. } => visit_inlines(blocks, f),
            Block::Table { header, rows, .. } => {
                for cell in header {
                    walk_inlines(cell, f);
                }
                for row in rows {
                    for cell in row {
                        walk_inlines(cell, f);
                    }
                }
            }
            Block::CodeBlock { .. } | Block::Math { .. } | Block::Html { .. } | Block::Rule => {}
        }
    }
}
