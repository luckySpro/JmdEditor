//! PDF export through Typst: the document model is turned into Typst markup
//! and compiled in-process, which gives real pagination, a table of contents
//! with page numbers, bookmarks and embedded fonts without a browser.

use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

use jmd_core::{outline, Align, Block, Document, Inline};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards};

use crate::docx::strip_tags;
use crate::html::title_case;
use crate::images::{ImageKind, ImageSource};
use crate::options::{ExportOptions, Paper, Template};
use crate::ExportError;

/// Holds the fonts and Typst library; build once and reuse for every export.
pub struct PdfEngine {
    library: LazyHash<Library>,
    fonts: FontStore,
}

impl PdfEngine {
    /// Fonts bundled with Typst, the system's fonts and any extra folders
    /// (for example the CJK fonts shipped in the app's resources).
    pub fn new(extra_font_dirs: &[PathBuf]) -> Self {
        let mut fonts = FontStore::new();
        for dir in extra_font_dirs {
            fonts.extend(typst_kit::fonts::scan(dir));
        }
        fonts.extend(typst_kit::fonts::system());
        fonts.extend(typst_kit::fonts::embedded());
        Self {
            library: LazyHash::new(Library::default()),
            fonts,
        }
    }

    /// A process-wide engine with system and embedded fonts only.
    pub fn shared() -> &'static PdfEngine {
        static ENGINE: OnceLock<PdfEngine> = OnceLock::new();
        ENGINE.get_or_init(|| PdfEngine::new(&[]))
    }

    pub fn family_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .fonts
            .book()
            .families()
            .map(|(name, _)| name.to_string())
            .collect();
        names.dedup();
        names
    }

    pub fn render(
        &self,
        doc: &Document,
        opts: &ExportOptions,
        images: &ImageSource,
    ) -> Result<Vec<u8>, ExportError> {
        let (markup, files) = typst_source(doc, opts, images, true);
        match self.compile(markup, files, opts) {
            Ok(pdf) => Ok(pdf),
            // A formula the translator got wrong must not cost the whole
            // export: retry with formulas shown as TeX source.
            Err(_) => {
                let (markup, files) = typst_source(doc, opts, images, false);
                self.compile(markup, files, opts)
            }
        }
    }

    fn compile(
        &self,
        markup: String,
        files: HashMap<String, Vec<u8>>,
        opts: &ExportOptions,
    ) -> Result<Vec<u8>, ExportError> {
        let main = FileId::new(RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new("/main.typ").map_err(|e| ExportError::Pdf(format!("{e:?}")))?,
        ));
        let today = opts.date.as_deref().and_then(parse_date);
        let world = JmdWorld {
            engine: self,
            main,
            source: Source::new(main, markup),
            files: files.into_iter().map(|(k, v)| (k, Bytes::new(v))).collect(),
            today,
        };
        let warned = typst::compile::<PagedDocument>(&world);
        let document = warned.output.map_err(|errors| {
            ExportError::Pdf(
                errors
                    .iter()
                    .map(|d| d.message.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        let mut pdf_opts = PdfOptions::default();
        if opts.pdf_a {
            pdf_opts.standards = PdfStandards::new(&[PdfStandard::A_2b])
                .map_err(|e| ExportError::Pdf(format!("{e:?}")))?;
        }
        typst_pdf::pdf(&document, &pdf_opts).map_err(|errors| {
            ExportError::Pdf(
                errors
                    .iter()
                    .map(|d| d.message.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })
    }
}

fn parse_date(s: &str) -> Option<Datetime> {
    let mut parts = s.split('-');
    let y = parts.next()?.parse().ok()?;
    let m = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    Datetime::from_ymd(y, m, d)
}

struct JmdWorld<'a> {
    engine: &'a PdfEngine,
    main: FileId,
    source: Source,
    files: HashMap<String, Bytes>,
    today: Option<Datetime>,
}

impl World for JmdWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.engine.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.engine.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(
                id.get().vpath().get_with_slash().into(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.get().vpath().get_with_slash();
        match (id.get().root(), self.files.get(path)) {
            (VirtualRoot::Project, Some(bytes)) => Ok(bytes.clone()),
            _ => Err(FileError::NotFound(path.into())),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.engine.fonts.font(index)
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        self.today
    }
}

/// A Typst string literal.
fn lit(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => {}
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

const CJK_SERIF: &str = r#""Noto Serif CJK SC", "Noto Serif SC", "Source Han Serif SC", "Songti SC", "STSong", "SimSun", "Noto Serif CJK JP", "Noto Sans CJK SC", "Noto Sans SC", "PingFang SC", "Microsoft YaHei", "WenQuanYi Zen Hei""#;
const CJK_SANS: &str = r#""Noto Sans CJK SC", "Noto Sans SC", "Source Han Sans SC", "PingFang SC", "Microsoft YaHei", "Hiragino Sans GB", "Noto Sans CJK JP", "Noto Serif CJK SC", "WenQuanYi Zen Hei""#;

struct Look {
    body: String,
    accent: &'static str,
    justify: bool,
    rule_under_h1: bool,
}

fn look(t: Template) -> Look {
    match t {
        Template::Clean => Look {
            body: format!(r#""Source Serif 4", "Libertinus Serif", {CJK_SERIF}"#),
            accent: "#2F5BD3",
            justify: false,
            rule_under_h1: false,
        },
        Template::Academic => Look {
            body: format!(r#""New Computer Modern", "Libertinus Serif", {CJK_SERIF}"#),
            accent: "#1B1E24",
            justify: true,
            rule_under_h1: false,
        },
        Template::Business => Look {
            body: format!(r#""Inter", "Noto Sans", "DejaVu Sans", "Libertinus Serif", {CJK_SANS}"#),
            accent: "#1E3A8A",
            justify: false,
            rule_under_h1: true,
        },
        Template::Github => Look {
            body: format!(r#""Noto Sans", "DejaVu Sans", "Inter", "Libertinus Serif", {CJK_SANS}"#),
            accent: "#0969DA",
            justify: false,
            rule_under_h1: true,
        },
    }
}

fn text_lang(lang: &str) -> &'static str {
    match lang {
        "zh-CN" | "zh" => r#"lang: "zh", region: "cn""#,
        "zh-TW" => r#"lang: "zh", region: "tw""#,
        "ja" => r#"lang: "ja""#,
        _ => r#"lang: "en""#,
    }
}

struct Gen<'a> {
    typeset_math: bool,
    images: &'a ImageSource,
    files: HashMap<String, Vec<u8>>,
    image_paths: HashMap<String, Option<String>>,
    /// Heading id → depth (0 = title).
    depths: HashMap<String, usize>,
    footnotes: HashMap<String, Vec<Block>>,
}

/// Builds the Typst source and the files it refers to.
pub fn typst_source(
    doc: &Document,
    opts: &ExportOptions,
    images: &ImageSource,
    typeset_math: bool,
) -> (String, HashMap<String, Vec<u8>>) {
    let entries = outline(doc);
    let title = crate::display_title(doc, opts);
    let has_sections = entries.iter().any(|e| !e.number.is_empty());
    let look = look(opts.template);
    let paper = match opts.paper {
        Paper::A4 => "a4",
        Paper::Letter => "us-letter",
    };

    let mut s = String::new();
    let author = opts
        .author
        .as_deref()
        .map(|a| format!(", author: {}", lit(a)))
        .unwrap_or_default();
    let doc_title = title
        .as_deref()
        .map(|t| format!("title: {}", lit(t)))
        .unwrap_or_else(|| "title: none".into());
    let _ = writeln!(s, "#set document({doc_title}{author})");
    let numbering = if opts.page_numbers { "\"1\"" } else { "none" };
    let _ = writeln!(
        s,
        "#set page(paper: \"{paper}\", margin: {}mm, numbering: {numbering}, number-align: center)",
        opts.margins.mm()
    );
    let _ = writeln!(
        s,
        "#set text(font: ({}), size: 11pt, {})",
        look.body,
        text_lang(&opts.lang)
    );
    let _ = writeln!(
        s,
        "#set par(leading: 0.75em, spacing: 1.1em, justify: {})",
        look.justify
    );
    let _ = writeln!(
        s,
        "#set heading(numbering: {}, bookmarked: {})",
        if opts.number_headings {
            "\"1.1\""
        } else {
            "none"
        },
        opts.bookmarks
    );
    let _ = writeln!(s, "#let accent = rgb(\"{}\")", look.accent);
    s.push_str(PREAMBLE);
    if look.rule_under_h1 {
        s.push_str("#show heading.where(level: 1): it => block(width: 100%, stroke: (bottom: 0.6pt + luma(205)), inset: (bottom: 5pt), it)\n");
    }
    let _ = writeln!(s, "#show raw: set text(font: (\"JetBrains Mono\", \"DejaVu Sans Mono\", {CJK_SANS}), size: 9pt)");
    s.push('\n');

    // Title, cover and table of contents.
    let meta: Vec<String> = [opts.author.as_deref(), opts.date.as_deref()]
        .into_iter()
        .flatten()
        .map(lit)
        .collect();
    let meta_line = if meta.is_empty() {
        String::new()
    } else {
        format!("#{}", meta.join("#\" · \"#"))
    };
    let title_lit = lit(title.as_deref().unwrap_or(""));
    if opts.cover {
        let _ = writeln!(
            s,
            "#page(numbering: none)[#v(1fr)#align(center)[#text(size: 28pt, weight: \"bold\")[#{title_lit}]\n\n#text(fill: luma(90))[{meta_line}]]#v(1.4fr)]"
        );
    } else if title.is_some() {
        let _ = writeln!(
            s,
            "#align(center)[#text(size: 22pt, weight: \"bold\")[#{title_lit}]]"
        );
        if !meta.is_empty() {
            let _ = writeln!(s, "#align(center)[#text(fill: luma(90))[{meta_line}]]");
        }
        s.push_str("#v(0.8em)\n");
    }
    if opts.toc && has_sections {
        let _ = writeln!(
            s,
            "#outline(title: [#{}], depth: {}, indent: auto)",
            lit(opts.contents_label()),
            opts.toc_depth()
        );
        if opts.cover {
            s.push_str("#pagebreak()\n");
        } else {
            s.push_str("#v(1.2em)\n");
        }
    }
    s.push('\n');

    let mut gen = Gen {
        typeset_math,
        images,
        files: HashMap::new(),
        image_paths: HashMap::new(),
        depths: entries
            .iter()
            .map(|e| {
                (
                    e.id.clone(),
                    if e.number.is_empty() {
                        0
                    } else {
                        e.number.matches('.').count() + 1
                    },
                )
            })
            .collect(),
        footnotes: doc
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::FootnoteDefinition { name, blocks } => Some((name.clone(), blocks.clone())),
                _ => None,
            })
            .collect(),
    };
    for block in &doc.blocks {
        let out = gen.block(block);
        if !out.is_empty() {
            s.push_str(&out);
            s.push_str("\n\n");
        }
    }
    (s, gen.files)
}

const PREAMBLE: &str = r##"#show heading: set block(above: 1.5em, below: 0.8em)
#show heading.where(level: 1): set text(size: 17pt, fill: accent)
#show heading.where(level: 2): set text(size: 14pt, fill: accent)
#show heading.where(level: 3): set text(size: 12pt)
#show heading: set text(weight: "bold")
#show link: set text(fill: accent)
#show outline.entry.where(level: 1): set text(weight: "bold")
#show raw.where(block: true): it => block(fill: luma(246), inset: 10pt, radius: 4pt, width: 100%, it)
#show raw.where(block: false): it => box(fill: luma(240), inset: (x: 3pt), outset: (y: 3pt), radius: 2pt, it)
#show quote.where(block: true): it => block(fill: rgb("#F4F7FE"), inset: (x: 12pt, y: 8pt), radius: 4pt, width: 100%, it.body)
#set table(stroke: 0.5pt + luma(205), inset: 6pt, fill: (x, y) => if y == 0 { luma(244) })
#show table.cell.where(y: 0): set text(weight: "bold")
#let jmdimg(path) = layout(size => {
  let img = image(path)
  if measure(img).width > size.width { image(path, width: 100%) } else { img }
})
#let jmdtask(done) = box(width: 0.8em, height: 0.8em, radius: 1.5pt, baseline: 0.05em, stroke: 0.7pt + if done { accent } else { luma(120) }, fill: if done { accent } else { none })
#let jmdalert(title, color, body) = block(fill: color, inset: (x: 12pt, y: 9pt), radius: 4pt, width: 100%)[#strong(title) \ #body]
#let jmdmath(tex) = text(style: "italic", font: ("New Computer Modern", "Libertinus Serif"), tex)
"##;

impl Gen<'_> {
    fn block(&mut self, block: &Block) -> String {
        match block {
            Block::Heading { content, id, .. } => {
                let depth = self.depths.get(id).copied().unwrap_or(1);
                let body = self.inlines(content);
                if depth == 0 {
                    // The document title is already printed above.
                    String::new()
                } else {
                    format!("#heading(level: {})[{body}]", depth.min(6))
                }
            }
            Block::Paragraph { content } => {
                let only_images = !content.is_empty()
                    && content.iter().all(|i| {
                        matches!(
                            i,
                            Inline::Image { .. } | Inline::SoftBreak | Inline::LineBreak
                        )
                    });
                if only_images {
                    let imgs: Vec<String> = content
                        .iter()
                        .filter_map(|i| match i {
                            Inline::Image { url, alt, .. } => Some(self.image(url, alt, true)),
                            _ => None,
                        })
                        .collect();
                    format!("#align(center)[{}]", imgs.join("#v(4pt)"))
                } else {
                    self.inlines(content)
                }
            }
            Block::List {
                ordered,
                start,
                items,
            } => {
                let all_tasks = !items.is_empty() && items.iter().all(|i| i.task.is_some());
                let mut parts = Vec::new();
                for item in items {
                    let mut body = String::new();
                    if let Some(done) = item.task {
                        let _ = write!(body, "#jmdtask({done})#h(0.45em)");
                    }
                    let inner: Vec<String> = item
                        .blocks
                        .iter()
                        .map(|b| self.block(b))
                        .filter(|s| !s.is_empty())
                        .collect();
                    body.push_str(&inner.join("\n\n"));
                    parts.push(format!("[{body}]"));
                }
                if *ordered {
                    format!("#enum(start: {start}, tight: true, {})", parts.join(", "))
                } else if all_tasks {
                    format!("#list(marker: [], tight: true, {})", parts.join(", "))
                } else {
                    format!("#list(tight: true, {})", parts.join(", "))
                }
            }
            Block::CodeBlock { lang, code } => {
                if lang.is_empty() {
                    format!("#raw(block: true, {})", lit(code))
                } else {
                    format!("#raw(block: true, lang: {}, {})", lit(lang), lit(code))
                }
            }
            Block::Quote { blocks } => {
                let inner: Vec<String> = blocks
                    .iter()
                    .map(|b| self.block(b))
                    .filter(|s| !s.is_empty())
                    .collect();
                format!("#quote(block: true)[{}]", inner.join("\n\n"))
            }
            Block::Alert { kind, blocks } => {
                let color = match kind.as_str() {
                    "warning" | "caution" => "#FFF6E5",
                    "tip" => "#E7F4EC",
                    _ => "#F4F7FE",
                };
                let inner: Vec<String> = blocks
                    .iter()
                    .map(|b| self.block(b))
                    .filter(|s| !s.is_empty())
                    .collect();
                format!(
                    "#jmdalert({}, rgb(\"{color}\"))[{}]",
                    lit(&title_case(kind)),
                    inner.join("\n\n")
                )
            }
            Block::Table {
                align,
                header,
                rows,
            } => {
                let cols = header
                    .len()
                    .max(rows.iter().map(Vec::len).max().unwrap_or(0))
                    .max(1);
                let aligns: Vec<&str> = (0..cols)
                    .map(|i| match align.get(i) {
                        Some(Align::Center) => "center",
                        Some(Align::Right) => "right",
                        _ => "left",
                    })
                    .collect();
                let mut out = format!("#table(columns: {cols}, align: ({},), ", aligns.join(", "));
                let head: Vec<String> = (0..cols)
                    .map(|i| {
                        format!(
                            "[{}]",
                            header.get(i).map(|c| self.inlines(c)).unwrap_or_default()
                        )
                    })
                    .collect();
                let _ = write!(out, "table.header({}), ", head.join(", "));
                for row in rows {
                    for i in 0..cols {
                        let _ = write!(
                            out,
                            "[{}], ",
                            row.get(i).map(|c| self.inlines(c)).unwrap_or_default()
                        );
                    }
                }
                out.push(')');
                out
            }
            Block::Math { tex } => self.math(tex, true),
            Block::Html { html } => {
                let text = strip_tags(html);
                if text.trim().is_empty() {
                    String::new()
                } else {
                    format!("#{}", lit(&text))
                }
            }
            Block::Rule => "#line(length: 100%, stroke: 0.5pt + luma(200))".into(),
            Block::FootnoteDefinition { .. } => String::new(),
        }
    }

    fn inlines(&mut self, inlines: &[Inline]) -> String {
        let mut out = String::new();
        for i in inlines {
            match i {
                Inline::Text { text } => {
                    out.push('#');
                    out.push_str(&lit(text));
                }
                Inline::Strong { content } => {
                    let _ = write!(out, "#strong[{}]", self.inlines(content));
                }
                Inline::Emph { content } => {
                    let _ = write!(out, "#emph[{}]", self.inlines(content));
                }
                Inline::Strike { content } => {
                    let _ = write!(out, "#strike[{}]", self.inlines(content));
                }
                Inline::Code { code } => {
                    let _ = write!(out, "#raw({})", lit(code));
                }
                Inline::Link { url, content, .. } => {
                    let body = self.inlines(content);
                    let lower = url.to_ascii_lowercase();
                    if lower.starts_with("http://")
                        || lower.starts_with("https://")
                        || lower.starts_with("mailto:")
                    {
                        let _ = write!(out, "#link({})[{body}]", lit(url));
                    } else {
                        out.push_str(&body);
                    }
                }
                Inline::Image { url, alt, .. } => out.push_str(&self.image(url, alt, false)),
                Inline::Math { tex, display } => out.push_str(&self.math(tex, *display)),
                Inline::FootnoteRef { name, .. } => {
                    let blocks = self.footnotes.get(name).cloned().unwrap_or_default();
                    let inner: Vec<String> = blocks
                        .iter()
                        .map(|b| self.block(b))
                        .filter(|s| !s.is_empty())
                        .collect();
                    let _ = write!(out, "#footnote[{}]", inner.join(" "));
                }
                Inline::Html { html } => {
                    let text = strip_tags(html);
                    if !text.is_empty() {
                        out.push('#');
                        out.push_str(&lit(&text));
                    }
                }
                Inline::SoftBreak => out.push_str("#\" \""),
                Inline::LineBreak => out.push_str("#linebreak()"),
            }
        }
        out
    }

    fn math(&self, tex: &str, display: bool) -> String {
        match crate::math::tex_to_typst(tex).filter(|_| self.typeset_math) {
            Some(m) if display => format!("$ {m} $"),
            Some(m) => format!("${m}$"),
            None if display => format!("#align(center, jmdmath({}))", lit(tex)),
            None => format!("#jmdmath({})", lit(tex)),
        }
    }

    /// Typst markup for an image, or its alt text in brackets when it cannot be loaded.
    fn image(&mut self, url: &str, alt: &str, block: bool) -> String {
        let path = match self.image_paths.get(url) {
            Some(p) => p.clone(),
            None => {
                let loaded = self
                    .images
                    .load(url)
                    .filter(|img| img.kind != ImageKind::Bmp);
                let path = loaded.map(|img| {
                    let p = format!("/img/{}.{}", self.files.len(), img.kind.extension());
                    self.files.insert(p.clone(), img.bytes);
                    p
                });
                self.image_paths.insert(url.to_string(), path.clone());
                path
            }
        };
        match path {
            Some(p) if block => format!("#jmdimg({})", lit(&p)),
            Some(p) => format!("#box(height: 1.2em, image({}))", lit(&p)),
            None => {
                let label = if alt.is_empty() { url } else { alt };
                format!("#text(fill: luma(110))[#{}]", lit(&format!("[{label}]")))
            }
        }
    }
}
