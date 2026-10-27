//! Heading outline, anchor ids and section numbers used by the sidebar and every exporter.

use serde::{Deserialize, Serialize};

use crate::model::{plain_text, Block, Document};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineEntry {
    pub level: u8,
    pub text: String,
    pub id: String,
    /// Section number such as `2.1`, counted from the shallowest heading level present.
    pub number: String,
}

/// GitHub-style anchor: lower case, letters and digits of any script kept, spaces to `-`.
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.trim().chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            out.extend(ch.to_lowercase());
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }
    out
}

/// Top-level headings of the document with their section numbers.
///
/// Numbering starts at the shallowest level used, so a document whose top
/// heading is `##` still numbers `1, 1.1, 2`. A level-1 heading that is the
/// document's only `#` heading is treated as the title and left unnumbered.
pub fn outline(doc: &Document) -> Vec<OutlineEntry> {
    let headings: Vec<(u8, String, String)> = doc
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Heading { level, content, id } => {
                Some((*level, plain_text(content), id.clone()))
            }
            _ => None,
        })
        .collect();

    let h1_count = headings.iter().filter(|(l, ..)| *l == 1).count();
    let title_is_h1 = h1_count == 1 && headings.first().map(|h| h.0) == Some(1);
    let base = headings
        .iter()
        .enumerate()
        .filter(|(i, _)| !(title_is_h1 && *i == 0))
        .map(|(_, h)| h.0)
        .min()
        .unwrap_or(1);

    let mut counters = [0usize; 6];
    headings
        .into_iter()
        .enumerate()
        .map(|(i, (level, text, id))| {
            let number = if title_is_h1 && i == 0 {
                String::new()
            } else {
                let depth = usize::from(level.saturating_sub(base));
                counters[depth] += 1;
                for c in counters.iter_mut().skip(depth + 1) {
                    *c = 0;
                }
                counters[..=depth]
                    .iter()
                    .map(|n| n.max(&1).to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            };
            OutlineEntry {
                level,
                text,
                id,
                number,
            }
        })
        .collect()
}
