//! JmdEditor core: Markdown parsing and the document model every exporter reads.

pub mod model;
pub mod outline;
pub mod parse;
pub mod stats;

pub use model::{plain_text, Align, Block, Document, Inline, ListItem};
pub use outline::{outline, slugify, OutlineEntry};
pub use parse::parse;
pub use stats::{stats, Stats};

/// Replaces (or adds) front matter keys in Markdown source, keeping the body untouched.
pub fn set_front_matter(source: &str, entries: &[(&str, &str)]) -> String {
    let (existing, body) = split_front_matter(source);
    let mut pairs = parse::parse_front_matter(existing.unwrap_or(""));
    for (k, v) in entries {
        match pairs.iter_mut().find(|(key, _)| key == k) {
            Some(pair) => pair.1 = v.to_string(),
            None => pairs.push((k.to_string(), v.to_string())),
        }
    }
    let mut out = String::from("---\n");
    for (k, v) in &pairs {
        out.push_str(&format!("{k}: {v}\n"));
    }
    out.push_str("---\n\n");
    out.push_str(body.trim_start_matches('\n'));
    out
}

/// Splits `---` front matter from the body. Returns `(Some(front matter), body)` when present.
pub fn split_front_matter(source: &str) -> (Option<&str>, &str) {
    let rest = match source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))
    {
        Some(r) => r,
        None => return (None, source),
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return (Some(&rest[..offset]), &rest[offset + line.len()..]);
        }
        offset += line.len();
    }
    (None, source)
}
