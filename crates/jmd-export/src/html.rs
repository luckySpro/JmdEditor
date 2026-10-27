//! Standalone HTML export.

use std::collections::HashMap;
use std::fmt::Write;

use base64::Engine;
use jmd_core::{outline, Align, Block, Document, Inline, OutlineEntry};

use crate::images::ImageSource;
use crate::options::{ExportOptions, Template};

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Links that may appear in `href`/`src`: web, mail, in-page and relative ones.
pub fn safe_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    let scheme_end = lower.find(':');
    let first_sep = lower.find(['/', '?', '#']);
    match (scheme_end, first_sep) {
        (Some(c), Some(s)) if s < c => true,
        (Some(_), _) => ["http:", "https:", "mailto:", "data:image/"]
            .iter()
            .any(|p| lower.starts_with(p)),
        (None, _) => true,
    }
}

/// Raw HTML from the note is kept unless it could run code.
pub fn html_is_safe(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    if [
        "<script",
        "<iframe",
        "<object",
        "<embed",
        "<style",
        "javascript:",
        "vbscript:",
    ]
    .iter()
    .any(|t| lower.contains(t))
    {
        return false;
    }
    // Inline event handlers such as onclick= or onerror =
    let bytes = lower.as_bytes();
    for (i, w) in bytes.windows(3).enumerate() {
        if w[0].is_ascii_whitespace() && w[1] == b'o' && w[2] == b'n' {
            let rest = &lower[i + 3..];
            let name_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).count();
            if name_len > 0 && rest[name_len..].trim_start().starts_with('=') {
                return false;
            }
        }
    }
    true
}

fn theme_css(template: Template) -> &'static str {
    match template {
        Template::Clean => ":root{--accent:#2F5BD3;--font:'Source Serif 4',Georgia,'Noto Serif SC','Songti SC',serif;--head:var(--font)}",
        Template::Academic => ":root{--accent:#1B1E24;--font:'Libertinus Serif','Times New Roman','Noto Serif SC','SimSun',serif;--head:var(--font)}body{text-align:justify}h1{text-align:center}",
        Template::Business => ":root{--accent:#1E3A8A;--font:Inter,'Segoe UI',Arial,'Noto Sans SC','Microsoft YaHei',sans-serif;--head:var(--font)}h1{border-bottom:4px solid var(--accent);padding-bottom:.3em}",
        Template::Github => ":root{--accent:#0969DA;--font:-apple-system,'Segoe UI','Noto Sans',Helvetica,Arial,'Noto Sans SC',sans-serif;--head:var(--font)}h1,h2{border-bottom:1px solid #d0d7de;padding-bottom:.3em}",
    }
}

const BASE_CSS: &str = "body{margin:0 auto;max-width:46em;padding:3em 1.5em;font-family:var(--font);font-size:17px;line-height:1.65;color:#1B1E24;background:#fff}
h1,h2,h3,h4,h5,h6{font-family:var(--head);line-height:1.25;margin:1.6em 0 .6em}
a{color:var(--accent)}
img{max-width:100%;height:auto}
code,pre{font-family:'JetBrains Mono',ui-monospace,Menlo,Consolas,monospace;font-size:.88em}
code{background:#F0F2F5;padding:.1em .35em;border-radius:4px}
pre{background:#F6F8FA;padding:1em 1.2em;border-radius:8px;overflow:auto;line-height:1.5}
pre code{background:none;padding:0}
blockquote{margin:1em 0;padding:.4em 1em;background:#F4F7FE;border-radius:8px;color:#2A3550}
table{border-collapse:collapse;width:100%;margin:1em 0;font-size:.95em}
th,td{border:1px solid #E3E6EB;padding:.45em .75em}
th{background:#F4F5F7}
hr{border:0;border-top:1px solid #E3E6EB;margin:2em 0}
.task{list-style:none;margin-left:-1.4em}
.task input{margin-right:.5em}
.alert{padding:.6em 1em;border-radius:8px;margin:1em 0;background:#F4F7FE}
.alert-title{font-weight:600;margin:0 0 .3em}
.alert-warning,.alert-caution{background:#FFF6E5}
.math{font-family:'Latin Modern Math','STIX Two Math',serif;font-style:italic}
.toc{background:#FAFBFC;border:1px solid #E3E6EB;border-radius:10px;padding:1em 1.5em;margin:0 0 2em}
.toc ol{list-style:none;padding-left:1.2em;margin:.2em 0}
.toc>ol{padding-left:0}
.toc a{text-decoration:none;color:inherit}
.num{color:#5B6270;margin-right:.5em}
.cover{text-align:center;margin:4em 0}
.cover .meta{color:#5B6270}
.footnotes{font-size:.9em;color:#3D4350;border-top:1px solid #E3E6EB;margin-top:3em}
@media print{body{max-width:none;padding:0}.toc{break-after:page}}";

struct Ctx<'a> {
    opts: &'a ExportOptions,
    images: &'a ImageSource,
    numbers: HashMap<String, String>,
    footnotes: Vec<(String, String)>,
}

/// Renders the document as one self-contained HTML file.
pub fn render(doc: &Document, opts: &ExportOptions, images: &ImageSource) -> String {
    let entries = outline(doc);
    let title = crate::display_title(doc, opts).unwrap_or_else(|| "Untitled".into());
    let numbers = entries
        .iter()
        .filter(|e| opts.number_headings && !e.number.is_empty())
        .map(|e| (e.id.clone(), e.number.clone()))
        .collect();
    let mut ctx = Ctx {
        opts,
        images,
        numbers,
        footnotes: Vec::new(),
    };

    let mut body = String::new();
    if opts.cover {
        let _ = write!(body, "<header class=\"cover\"><h1>{}</h1>", escape(&title));
        let meta: Vec<String> = [opts.author.as_deref(), opts.date.as_deref()]
            .into_iter()
            .flatten()
            .map(escape)
            .collect();
        if !meta.is_empty() {
            let _ = write!(body, "<p class=\"meta\">{}</p>", meta.join(" · "));
        }
        body.push_str("</header>\n");
    }
    // Without a cover, a leading title heading stays above the contents.
    let title_first =
        !opts.cover && matches!(doc.blocks.first(), Some(Block::Heading { level: 1, .. }));
    if title_first {
        ctx.block(&doc.blocks[0], &mut body);
    }
    if opts.toc {
        body.push_str(&toc_html(&entries, opts));
    }
    let skip_title = opts.cover;
    for (i, block) in doc.blocks.iter().enumerate() {
        if (skip_title && i == first_h1_index(doc)) || (title_first && i == 0) {
            continue;
        }
        ctx.block(block, &mut body);
    }
    if !ctx.footnotes.is_empty() {
        body.push_str("<section class=\"footnotes\"><ol>");
        for (name, html) in std::mem::take(&mut ctx.footnotes) {
            let _ = write!(body, "<li id=\"fn-{}\">{}</li>", escape(&name), html);
        }
        body.push_str("</ol></section>");
    }

    let lang = escape(&opts.lang);
    format!(
        "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"JmdEditor\">\n<title>{}</title>\n<style>{}\n{}</style>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        escape(&title),
        theme_css(opts.template),
        BASE_CSS,
        body
    )
}

fn first_h1_index(doc: &Document) -> usize {
    doc.blocks
        .iter()
        .position(|b| matches!(b, Block::Heading { level: 1, .. }))
        .unwrap_or(usize::MAX)
}

fn toc_html(entries: &[OutlineEntry], opts: &ExportOptions) -> String {
    let listed: Vec<&OutlineEntry> = entries
        .iter()
        // The title heading has no number and is not listed.
        .filter(|e| !e.number.is_empty())
        .filter(|e| e.number.matches('.').count() < usize::from(opts.toc_depth()))
        .collect();
    if listed.is_empty() {
        return String::new();
    }
    let mut out = format!(
        "<nav class=\"toc\"><strong>{}</strong><ol>",
        opts.contents_label()
    );
    let mut depth = 1;
    for e in listed {
        let d = e.number.matches('.').count() + 1;
        while depth < d {
            out.push_str("<ol>");
            depth += 1;
        }
        while depth > d {
            out.push_str("</ol>");
            depth -= 1;
        }
        let num = if opts.number_headings {
            format!("<span class=\"num\">{}</span>", e.number)
        } else {
            String::new()
        };
        let _ = write!(
            out,
            "<li><a href=\"#{}\">{}{}</a></li>",
            escape(&e.id),
            num,
            escape(&e.text)
        );
    }
    while depth > 1 {
        out.push_str("</ol>");
        depth -= 1;
    }
    out.push_str("</ol></nav>\n");
    out
}

impl Ctx<'_> {
    fn block(&mut self, block: &Block, out: &mut String) {
        match block {
            Block::Heading { level, content, id } => {
                let num = self
                    .numbers
                    .get(id)
                    .map(|n| format!("<span class=\"num\">{n}</span>"))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "<h{level} id=\"{}\">{num}{}</h{level}>",
                    escape(id),
                    self.inlines(content)
                );
            }
            Block::Paragraph { content } => {
                let _ = writeln!(out, "<p>{}</p>", self.inlines(content));
            }
            Block::List {
                ordered,
                start,
                items,
            } => {
                let tag = if *ordered { "ol" } else { "ul" };
                let start_attr = if *ordered && *start != 1 {
                    format!(" start=\"{start}\"")
                } else {
                    String::new()
                };
                let _ = write!(out, "<{tag}{start_attr}>");
                for item in items {
                    match item.task {
                        Some(done) => {
                            let checked = if done { " checked" } else { "" };
                            let _ = write!(
                                out,
                                "<li class=\"task\"><input type=\"checkbox\" disabled{checked}>"
                            );
                        }
                        None => out.push_str("<li>"),
                    }
                    // Tight list items hold a single paragraph: drop the <p>.
                    if let [Block::Paragraph { content }] = item.blocks.as_slice() {
                        out.push_str(&self.inlines(content));
                    } else {
                        for b in &item.blocks {
                            self.block(b, out);
                        }
                    }
                    out.push_str("</li>");
                }
                let _ = writeln!(out, "</{tag}>");
            }
            Block::CodeBlock { lang, code } => {
                let class = if lang.is_empty() {
                    String::new()
                } else {
                    format!(" class=\"language-{}\"", escape(lang))
                };
                let _ = writeln!(out, "<pre><code{class}>{}</code></pre>", escape(code));
            }
            Block::Quote { blocks } => {
                out.push_str("<blockquote>");
                for b in blocks {
                    self.block(b, out);
                }
                out.push_str("</blockquote>\n");
            }
            Block::Alert { kind, blocks } => {
                let _ = write!(
                    out,
                    "<div class=\"alert alert-{}\"><p class=\"alert-title\">{}</p>",
                    escape(kind),
                    escape(&title_case(kind))
                );
                for b in blocks {
                    self.block(b, out);
                }
                out.push_str("</div>\n");
            }
            Block::Table {
                align,
                header,
                rows,
            } => {
                out.push_str("<table><thead><tr>");
                for (i, cell) in header.iter().enumerate() {
                    let _ = write!(
                        out,
                        "<th{}>{}</th>",
                        align_attr(align.get(i)),
                        self.inlines(cell)
                    );
                }
                out.push_str("</tr></thead><tbody>");
                for row in rows {
                    out.push_str("<tr>");
                    for (i, cell) in row.iter().enumerate() {
                        let _ = write!(
                            out,
                            "<td{}>{}</td>",
                            align_attr(align.get(i)),
                            self.inlines(cell)
                        );
                    }
                    out.push_str("</tr>");
                }
                out.push_str("</tbody></table>\n");
            }
            Block::Math { tex } => {
                let _ = writeln!(out, "<p class=\"math\">{}</p>", escape(tex));
            }
            Block::FootnoteDefinition { name, blocks } => {
                let mut html = String::new();
                for b in blocks {
                    self.block(b, &mut html);
                }
                self.footnotes.push((name.clone(), html));
            }
            Block::Html { html } => {
                if html_is_safe(html) {
                    out.push_str(html);
                } else {
                    let _ = writeln!(out, "<pre>{}</pre>", escape(html));
                }
            }
            Block::Rule => out.push_str("<hr>\n"),
        }
    }

    fn inlines(&self, inlines: &[Inline]) -> String {
        let mut out = String::new();
        for i in inlines {
            match i {
                Inline::Text { text } => out.push_str(&escape(text)),
                Inline::Strong { content } => {
                    let _ = write!(out, "<strong>{}</strong>", self.inlines(content));
                }
                Inline::Emph { content } => {
                    let _ = write!(out, "<em>{}</em>", self.inlines(content));
                }
                Inline::Strike { content } => {
                    let _ = write!(out, "<del>{}</del>", self.inlines(content));
                }
                Inline::Code { code } => {
                    let _ = write!(out, "<code>{}</code>", escape(code));
                }
                Inline::Link {
                    url,
                    title,
                    content,
                } => {
                    let href = if safe_url(url) {
                        escape(url)
                    } else {
                        "#".into()
                    };
                    let title_attr = if title.is_empty() {
                        String::new()
                    } else {
                        format!(" title=\"{}\"", escape(title))
                    };
                    let _ = write!(
                        out,
                        "<a href=\"{href}\"{title_attr}>{}</a>",
                        self.inlines(content)
                    );
                }
                Inline::Image { url, title, alt } => {
                    let src = if self.opts.inline_images {
                        self.images
                            .load(url)
                            .map(|img| {
                                format!(
                                    "data:{};base64,{}",
                                    img.kind.mime(),
                                    base64::engine::general_purpose::STANDARD.encode(&img.bytes)
                                )
                            })
                            .unwrap_or_else(|| url.clone())
                    } else {
                        url.clone()
                    };
                    let src = if safe_url(&src) { src } else { String::new() };
                    let title_attr = if title.is_empty() {
                        String::new()
                    } else {
                        format!(" title=\"{}\"", escape(title))
                    };
                    let _ = write!(
                        out,
                        "<img src=\"{}\" alt=\"{}\"{title_attr}>",
                        escape(&src),
                        escape(alt)
                    );
                }
                Inline::Math { tex, display } => {
                    let tag = if *display { "div" } else { "span" };
                    let _ = write!(out, "<{tag} class=\"math\">{}</{tag}>", escape(tex));
                }
                Inline::FootnoteRef { name, number } => {
                    let _ = write!(
                        out,
                        "<sup><a href=\"#fn-{}\">{number}</a></sup>",
                        escape(name)
                    );
                }
                Inline::Html { html } => {
                    if html_is_safe(html) {
                        out.push_str(html);
                    }
                }
                Inline::SoftBreak => out.push('\n'),
                Inline::LineBreak => out.push_str("<br>"),
            }
        }
        out
    }
}

fn align_attr(a: Option<&Align>) -> &'static str {
    match a {
        Some(Align::Left) => " style=\"text-align:left\"",
        Some(Align::Center) => " style=\"text-align:center\"",
        Some(Align::Right) => " style=\"text-align:right\"",
        _ => "",
    }
}

pub fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
