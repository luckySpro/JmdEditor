//! Markdown → [`Document`] using comrak with GitHub Flavored Markdown extensions.

use std::collections::HashMap;

use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment};
use comrak::{parse_document, Arena, Options};

use crate::model::{plain_text, Align, Block, Document, Inline, ListItem};
use crate::outline::slugify;

/// comrak options used everywhere JmdEditor reads Markdown.
pub fn options() -> Options<'static> {
    let mut o = Options::default();
    o.extension.strikethrough = true;
    o.extension.table = true;
    o.extension.autolink = true;
    o.extension.tasklist = true;
    o.extension.footnotes = true;
    o.extension.front_matter_delimiter = Some("---".to_string());
    o.extension.alerts = true;
    o.extension.math_dollars = true;
    o.extension.math_code = true;
    o
}

/// Parses Markdown source into the shared document model.
pub fn parse(source: &str) -> Document {
    let arena = Arena::new();
    let root = parse_document(&arena, source, &options());
    let mut ctx = Ctx::default();
    let mut doc = Document::default();
    for child in root.children() {
        if let NodeValue::FrontMatter(raw) = &child.data().value {
            doc.front_matter = parse_front_matter(raw);
            continue;
        }
        if let Some(block) = ctx.block(child) {
            doc.blocks.push(block);
        }
    }
    doc
}

/// Reads simple `key: value` lines from a front matter block (nested YAML is kept as raw text).
pub fn parse_front_matter(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .map(str::trim_end)
        .filter(|l| {
            *l != "---" && !l.trim().is_empty() && !l.starts_with(' ') && !l.starts_with('#')
        })
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            let v = v.trim();
            let v = v
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                .unwrap_or(v);
            Some((k.trim().to_string(), v.to_string()))
        })
        .collect()
}

#[derive(Default)]
struct Ctx {
    used_ids: HashMap<String, usize>,
}

impl Ctx {
    fn unique_id(&mut self, text: &str) -> String {
        let base = slugify(text);
        let base = if base.is_empty() {
            "section".to_string()
        } else {
            base
        };
        let n = self.used_ids.entry(base.clone()).or_insert(0);
        let id = if *n == 0 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        *n += 1;
        id
    }

    fn blocks<'a>(&mut self, node: &'a AstNode<'a>) -> Vec<Block> {
        node.children().filter_map(|c| self.block(c)).collect()
    }

    fn block<'a>(&mut self, node: &'a AstNode<'a>) -> Option<Block> {
        let value = node.data().value.clone();
        Some(match value {
            NodeValue::Heading(h) => {
                let content = inlines(node);
                let id = self.unique_id(&plain_text(&content));
                Block::Heading {
                    level: h.level,
                    content,
                    id,
                }
            }
            NodeValue::Paragraph => Block::Paragraph {
                content: inlines(node),
            },
            NodeValue::List(list) => {
                let items = node
                    .children()
                    .map(|item| {
                        let task = match &item.data().value {
                            NodeValue::TaskItem(t) => Some(t.symbol.is_some()),
                            _ => None,
                        };
                        ListItem {
                            task,
                            blocks: self.blocks(item),
                        }
                    })
                    .collect();
                Block::List {
                    ordered: list.list_type == ListType::Ordered,
                    start: list.start.max(1),
                    items,
                }
            }
            NodeValue::CodeBlock(cb) => {
                let lang = cb.info.split_whitespace().next().unwrap_or("").to_string();
                if lang == "math" {
                    Block::Math {
                        tex: cb.literal.trim_end().to_string(),
                    }
                } else {
                    Block::CodeBlock {
                        lang,
                        code: cb.literal.trim_end_matches('\n').to_string(),
                    }
                }
            }
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => Block::Quote {
                blocks: self.blocks(node),
            },
            NodeValue::Alert(alert) => Block::Alert {
                kind: alert.alert_type.default_title().to_lowercase(),
                blocks: self.blocks(node),
            },
            NodeValue::Table(table) => {
                let align = table
                    .alignments
                    .iter()
                    .map(|a| match a {
                        TableAlignment::None => Align::None,
                        TableAlignment::Left => Align::Left,
                        TableAlignment::Center => Align::Center,
                        TableAlignment::Right => Align::Right,
                    })
                    .collect();
                let mut header = Vec::new();
                let mut rows = Vec::new();
                for row in node.children() {
                    let is_header = matches!(row.data().value, NodeValue::TableRow(true));
                    let cells: Vec<Vec<Inline>> = row.children().map(inlines).collect();
                    if is_header {
                        header = cells;
                    } else {
                        rows.push(cells);
                    }
                }
                Block::Table {
                    align,
                    header,
                    rows,
                }
            }
            NodeValue::ThematicBreak => Block::Rule,
            NodeValue::HtmlBlock(h) => Block::Html { html: h.literal },
            NodeValue::FootnoteDefinition(f) => Block::FootnoteDefinition {
                name: f.name,
                blocks: self.blocks(node),
            },
            NodeValue::Math(m) if m.display_math => Block::Math { tex: m.literal },
            _ => return None,
        })
    }
}

fn inlines<'a>(node: &'a AstNode<'a>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for child in node.children() {
        let value = child.data().value.clone();
        let inline = match value {
            NodeValue::Text(t) => Inline::Text {
                text: t.to_string(),
            },
            NodeValue::Code(c) => Inline::Code { code: c.literal },
            NodeValue::Strong => Inline::Strong {
                content: inlines(child),
            },
            NodeValue::Emph => Inline::Emph {
                content: inlines(child),
            },
            NodeValue::Strikethrough => Inline::Strike {
                content: inlines(child),
            },
            NodeValue::Link(l) => Inline::Link {
                url: l.url,
                title: l.title,
                content: inlines(child),
            },
            NodeValue::Image(l) => Inline::Image {
                url: l.url,
                title: l.title,
                alt: plain_text(&inlines(child)),
            },
            NodeValue::Math(m) => Inline::Math {
                tex: m.literal,
                display: m.display_math,
            },
            NodeValue::FootnoteReference(f) => Inline::FootnoteRef {
                name: f.name,
                number: f.ix,
            },
            NodeValue::HtmlInline(h) => Inline::Html { html: h },
            NodeValue::SoftBreak => Inline::SoftBreak,
            NodeValue::LineBreak => Inline::LineBreak,
            // Formatting JmdEditor does not model keeps its text.
            _ => {
                out.extend(inlines(child));
                continue;
            }
        };
        // Merge adjacent text runs so exporters see whole words.
        if let (Inline::Text { text }, Some(Inline::Text { text: prev })) =
            (&inline, out.last_mut())
        {
            prev.push_str(text);
            continue;
        }
        out.push(inline);
    }
    out
}
