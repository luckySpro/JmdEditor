//! HTML → Markdown and link fixing.

use jmd_core::{model::visit_inlines, Inline};
use url::Url;

/// Decodes a response body using the charset from its content type, falling back to UTF-8.
pub fn decode(body: &[u8], content_type: &str) -> String {
    let charset = content_type
        .split(';')
        .filter_map(|p| p.trim().strip_prefix("charset="))
        .next()
        .map(|c| c.trim_matches('"'));
    if let Some(enc) = charset.and_then(|c| encoding_rs::Encoding::for_label(c.as_bytes())) {
        if enc != encoding_rs::UTF_8 {
            return enc.decode(body).0.into_owned();
        }
    }
    let text = String::from_utf8_lossy(body);
    // A page may declare its charset only in a <meta> tag.
    if text.contains('\u{FFFD}') {
        let head = text
            .chars()
            .take(2048)
            .collect::<String>()
            .to_ascii_lowercase();
        if let Some(pos) = head.find("charset=") {
            let label: String = head[pos + 8..]
                .trim_start_matches(['"', '\''])
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes()) {
                return enc.decode(body).0.into_owned();
            }
        }
    }
    text.into_owned()
}

/// Extracts the main article of a page and converts it to Markdown.
/// Returns the page title and the Markdown body.
pub fn html_to_markdown(html: &str, page_url: &Url) -> (String, String) {
    let (title, content) = match dom_smoothie::Readability::new(html, Some(page_url.as_str()), None)
        .and_then(|mut r| r.parse())
    {
        Ok(article) => (
            article.title.trim().to_string(),
            article.content.to_string(),
        ),
        // Not an article (for example a short page): convert the whole body.
        Err(_) => (String::new(), html.to_string()),
    };
    let converter = htmd::HtmlToMarkdown::builder()
        .skip_tags(vec![
            "script", "style", "noscript", "iframe", "object", "embed", "form", "button", "svg",
            "canvas",
        ])
        .build();
    let markdown = converter.convert(&content).unwrap_or_default();
    let markdown = absolutize_links(&markdown, page_url);
    let markdown = tidy(&markdown);
    let markdown = if !title.is_empty() && !markdown.trim_start().starts_with("# ") {
        format!("# {title}\n\n{markdown}")
    } else {
        markdown
    };
    (title, markdown)
}

/// Collapses runs of blank lines left over from conversion.
fn tidy(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut blank = 0;
    for line in md.lines() {
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out.trim().to_string() + "\n"
}

/// Rewrites relative link and image targets so they still work once the note lives elsewhere.
pub fn absolutize_links(md: &str, base: &Url) -> String {
    let doc = jmd_core::parse(md);
    let mut targets: Vec<String> = Vec::new();
    visit_inlines(&doc.blocks, &mut |i| match i {
        Inline::Image { url, .. } | Inline::Link { url, .. } => targets.push(url.clone()),
        _ => {}
    });
    let mut out = md.to_string();
    targets.sort();
    targets.dedup();
    for target in targets {
        if target.is_empty() || target.starts_with('#') || Url::parse(&target).is_ok() {
            continue;
        }
        if let Ok(abs) = base.join(&target) {
            out = out.replace(&format!("]({target}"), &format!("]({abs}"));
        }
    }
    // Raw HTML such as `<img src="docs/logo.png">` in README files.
    let mut raw: Vec<String> = Vec::new();
    collect_html(&doc.blocks, &mut raw);
    visit_inlines(&doc.blocks, &mut |i| {
        if let Inline::Html { html } = i {
            raw.push(html.clone());
        }
    });
    raw.sort();
    raw.dedup();
    for html in raw {
        let fixed = absolutize_html_attrs(&html, base);
        if fixed != html {
            out = out.replace(&html, &fixed);
        }
    }
    out
}

fn collect_html(blocks: &[jmd_core::Block], out: &mut Vec<String>) {
    use jmd_core::Block;
    for b in blocks {
        match b {
            Block::Html { html } => out.push(html.trim_end().to_string()),
            Block::Quote { blocks }
            | Block::Alert { blocks, .. }
            | Block::FootnoteDefinition { blocks, .. } => collect_html(blocks, out),
            Block::List { items, .. } => items.iter().for_each(|i| collect_html(&i.blocks, out)),
            _ => {}
        }
    }
}

/// Makes `src="…"` and `href="…"` values absolute against `base`.
pub fn absolutize_html_attrs(html: &str, base: &Url) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(pos) = find_attr(rest) {
        let (head, tail) = rest.split_at(pos.value_start);
        out.push_str(head);
        let end = tail.find(pos.quote).unwrap_or(tail.len());
        let value = &tail[..end];
        let keep = value.is_empty()
            || value.starts_with('#')
            || Url::parse(value).is_ok()
            || value.starts_with("//");
        match (keep, base.join(value)) {
            (false, Ok(abs)) => out.push_str(abs.as_str()),
            _ => out.push_str(value),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

struct AttrPos {
    value_start: usize,
    quote: char,
}

fn find_attr(s: &str) -> Option<AttrPos> {
    let lower = s.to_ascii_lowercase();
    let mut best: Option<AttrPos> = None;
    for name in ["src=", "href="] {
        let mut from = 0;
        while let Some(i) = lower[from..].find(name) {
            let at = from + i;
            from = at + name.len();
            let before_ok = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
            let quote = s[from..].chars().next();
            if before_ok && matches!(quote, Some('"') | Some('\'')) {
                let cand = AttrPos {
                    value_start: from + 1,
                    quote: quote.unwrap(),
                };
                if best
                    .as_ref()
                    .is_none_or(|b| cand.value_start < b.value_start)
                {
                    best = Some(cand);
                }
                break;
            }
        }
    }
    best
}

/// Strips HTML tags that must never reach the editor from raw Markdown (scripts, frames, handlers).
pub fn strip_dangerous_html(md: &str) -> String {
    let mut out = md.to_string();
    for tag in ["script", "iframe", "object", "embed", "style"] {
        loop {
            let lower = out.to_ascii_lowercase();
            let Some(start) = lower.find(&format!("<{tag}")) else {
                break;
            };
            let close = format!("</{tag}>");
            let end = lower[start..]
                .find(&close)
                .map(|e| start + e + close.len())
                .or_else(|| lower[start..].find('>').map(|e| start + e + 1))
                .unwrap_or(out.len());
            out.replace_range(start..end, "");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolutizes_raw_html_attributes() {
        let base = Url::parse("https://raw.example.com/o/r/main/README.md").unwrap();
        let md = "<img src=\".github/splash.png\" alt=\"x\" />\n\nText <a href='docs/a.md'>a</a> and <img src=\"https://x.test/a.png\">\n\n```html\n<img src=\"keep.png\">\n```\n";
        let out = absolutize_links(md, &base);
        assert!(
            out.contains("src=\"https://raw.example.com/o/r/main/.github/splash.png\""),
            "{out}"
        );
        assert!(
            out.contains("href='https://raw.example.com/o/r/main/docs/a.md'"),
            "{out}"
        );
        assert!(out.contains("src=\"https://x.test/a.png\""));
        assert!(
            out.contains("<img src=\"keep.png\">"),
            "code is left alone: {out}"
        );
    }

    #[test]
    fn converts_article_html() {
        let html = r#"<html><head><title>Updater | Tauri</title><script>alert(1)</script></head>
        <body><nav>menu menu menu</nav><article><h1>Updater</h1>
        <p>Automatically update your Tauri app with an update server or a static JSON. This paragraph is long enough
        to look like real content for the readability scorer, with several sentences, commas, and words.</p>
        <p>Install the plugin to get started. Another paragraph, with commas, so that the article wins the score.</p>
        <img src="/img/platforms.png" alt="Platforms">
        <p>See <a href="guide/next">the next guide</a>.</p></article><footer>footer</footer></body></html>"#;
        let url = Url::parse("https://v2.tauri.app/plugin/updater/").unwrap();
        let (title, md) = html_to_markdown(html, &url);
        assert!(title.contains("Updater"), "{title}");
        assert!(md.contains("Automatically update your Tauri app"), "{md}");
        assert!(
            md.contains("https://v2.tauri.app/img/platforms.png"),
            "{md}"
        );
        assert!(
            md.contains("https://v2.tauri.app/plugin/updater/guide/next"),
            "{md}"
        );
        assert!(!md.contains("alert(1)"));
    }

    #[test]
    fn absolutizes_relative_targets_only() {
        let base = Url::parse("https://raw.githubusercontent.com/o/r/main/docs/a.md").unwrap();
        let md = "![x](img/x.png) [y](../y.md) [z](https://z.test/) [top](#top)";
        assert_eq!(
            absolutize_links(md, &base),
            "![x](https://raw.githubusercontent.com/o/r/main/docs/img/x.png) [y](https://raw.githubusercontent.com/o/r/main/y.md) [z](https://z.test/) [top](#top)"
        );
    }

    #[test]
    fn strips_scripts_from_markdown() {
        let md = "Hello <script>steal()</script> world <iframe src=x></iframe>!";
        assert_eq!(strip_dangerous_html(md), "Hello  world !");
    }

    #[test]
    fn decodes_gbk() {
        let (bytes, _, _) = encoding_rs::GBK.encode("离线优先");
        assert_eq!(decode(&bytes, "text/plain; charset=gbk"), "离线优先");
        assert_eq!(decode("ok".as_bytes(), "text/plain"), "ok");
    }
}
