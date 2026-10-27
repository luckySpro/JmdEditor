//! Word (.docx) export with real heading styles, an updatable TOC field,
//! numbered lists, tables, images and footnotes.

use std::collections::HashMap;
use std::io::Cursor;

use docx_rs::{
    AbstractNumbering, AlignmentType, BreakType, Docx, Footer, Footnote, Hyperlink, HyperlinkType,
    IndentLevel, Level, LevelJc, LevelText, LineSpacing, LineSpacingType, NumberFormat, Numbering,
    NumberingId, PageMargin, PageNum, Paragraph, Pic, Run, RunFonts, Shading, ShdType,
    SpecialIndentType, Start, Style, StyleType, Table, TableCell, TableOfContents, TableRow,
    WidthType,
};
use jmd_core::{outline, Align, Block, Document, Inline};

use crate::html::title_case;
use crate::images::{ImageKind, ImageSource};
use crate::options::{ExportOptions, Template};
use crate::ExportError;

const EMU_PER_MM: f64 = 36_000.0;
const TWIPS_PER_MM: f64 = 56.692_913;
// docx-rs reserves numbering id 1 for its default list.
const BULLET_NUMBERING: usize = 2;

struct Fonts {
    body: &'static str,
    east_asia: &'static str,
    heading: &'static str,
    accent: &'static str,
}

fn fonts(t: Template) -> Fonts {
    match t {
        Template::Clean => Fonts {
            body: "Georgia",
            east_asia: "SimSun",
            heading: "Georgia",
            accent: "2F5BD3",
        },
        Template::Academic => Fonts {
            body: "Times New Roman",
            east_asia: "SimSun",
            heading: "Times New Roman",
            accent: "1B1E24",
        },
        Template::Business => Fonts {
            body: "Calibri",
            east_asia: "Microsoft YaHei",
            heading: "Calibri",
            accent: "1E3A8A",
        },
        Template::Github => Fonts {
            body: "Segoe UI",
            east_asia: "Microsoft YaHei",
            heading: "Segoe UI",
            accent: "1F2328",
        },
    }
}

fn run_fonts(latin: &str, east_asia: &str) -> RunFonts {
    RunFonts::new()
        .ascii(latin)
        .hi_ansi(latin)
        .cs(latin)
        .east_asia(east_asia)
}

/// Inline formatting carried down while walking nested inlines.
#[derive(Clone, Copy, Default)]
struct Fmt {
    bold: bool,
    italic: bool,
    strike: bool,
}

impl Fmt {
    fn apply(self, mut run: Run) -> Run {
        if self.bold {
            run = run.bold();
        }
        if self.italic {
            run = run.italic();
        }
        if self.strike {
            run = run.strike();
        }
        run
    }
}

enum Piece {
    Run(Box<Run>),
    Link(Hyperlink),
}

fn piece(run: Run) -> Piece {
    Piece::Run(Box::new(run))
}

struct Ctx<'a> {
    opts: &'a ExportOptions,
    images: &'a ImageSource,
    fonts: Fonts,
    /// Heading id → (Word heading level, number, bookmark name).
    headings: HashMap<String, (usize, String, String)>,
    footnotes: HashMap<String, Vec<Block>>,
    content_width_emu: f64,
    docx: Docx,
    next_numbering: usize,
    next_bookmark: usize,
}

pub fn render(
    doc: &Document,
    opts: &ExportOptions,
    images: &ImageSource,
) -> Result<Vec<u8>, ExportError> {
    let entries = outline(doc);
    let title = crate::display_title(doc, opts);
    let headings = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let depth = if e.number.is_empty() {
                0
            } else {
                e.number.matches('.').count() + 1
            };
            (
                e.id.clone(),
                (depth, e.number.clone(), format!("_jmd_h{i}")),
            )
        })
        .collect();
    let footnotes = doc
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::FootnoteDefinition { name, blocks } => Some((name.clone(), blocks.clone())),
            _ => None,
        })
        .collect();

    let (pw, ph) = opts.paper.size_mm();
    let margin = opts.margins.mm();
    let fonts = fonts(opts.template);
    let docx = base_docx(opts, &fonts, title.as_deref().unwrap_or(""))
        .page_size((pw * TWIPS_PER_MM) as u32, (ph * TWIPS_PER_MM) as u32)
        .page_margin(PageMargin {
            top: (margin * TWIPS_PER_MM) as i32,
            bottom: (margin * TWIPS_PER_MM) as i32,
            left: (margin * TWIPS_PER_MM) as i32,
            right: (margin * TWIPS_PER_MM) as i32,
            header: 708,
            footer: 708,
            gutter: 0,
        });

    let mut ctx = Ctx {
        opts,
        images,
        fonts,
        headings,
        footnotes,
        content_width_emu: (pw - 2.0 * margin) * EMU_PER_MM,
        docx,
        next_numbering: 10,
        next_bookmark: 1,
    };

    if let Some(title) = &title {
        ctx.push(
            Paragraph::new()
                .style("Title")
                .add_run(Run::new().add_text(title.clone())),
        );
        let meta: Vec<&str> = [opts.author.as_deref(), opts.date.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        if !meta.is_empty() {
            ctx.push(
                Paragraph::new()
                    .style("Subtitle")
                    .add_run(Run::new().add_text(meta.join(" · "))),
            );
        }
        if opts.cover {
            ctx.push(Paragraph::new().add_run(Run::new().add_break(BreakType::Page)));
        }
    }
    if opts.toc && entries.iter().any(|e| !e.number.is_empty()) {
        let toc = TableOfContents::new()
            .heading_styles_range(1, usize::from(opts.toc_depth()))
            .alias(opts.contents_label())
            .hyperlink()
            .auto()
            .dirty()
            .add_before_paragraph(
                Paragraph::new()
                    .style("TOCHeading")
                    .add_run(Run::new().add_text(opts.contents_label())),
            );
        ctx.docx = std::mem::take(&mut ctx.docx).add_table_of_contents(toc);
        if opts.cover {
            ctx.push(Paragraph::new().add_run(Run::new().add_break(BreakType::Page)));
        }
    }

    for block in &doc.blocks {
        ctx.block(block, 0);
    }

    let mut buf = Cursor::new(Vec::new());
    ctx.docx
        .build()
        .pack(&mut buf)
        .map_err(|e| ExportError::Docx(e.to_string()))?;
    Ok(buf.into_inner())
}

fn base_docx(opts: &ExportOptions, f: &Fonts, title: &str) -> Docx {
    let heading_sizes = [32, 28, 26, 24, 22, 22];
    let mut docx = Docx::new()
        .default_fonts(run_fonts(f.body, f.east_asia))
        .default_size(22)
        .default_line_spacing(
            LineSpacing::new()
                .after(140)
                .line(276)
                .line_rule(LineSpacingType::Auto),
        )
        .custom_property("Title", title)
        .add_style(
            Style::new("Title", StyleType::Paragraph)
                .name("Title")
                .size(52)
                .bold()
                .fonts(run_fonts(f.heading, f.east_asia))
                .align(AlignmentType::Center),
        )
        .add_style(
            Style::new("Subtitle", StyleType::Paragraph)
                .name("Subtitle")
                .size(24)
                .color("5B6270")
                .align(AlignmentType::Center),
        )
        .add_style(
            Style::new("TOCHeading", StyleType::Paragraph)
                .name("TOC Heading")
                .size(28)
                .bold()
                .fonts(run_fonts(f.heading, f.east_asia)),
        )
        .add_style(
            Style::new("Code", StyleType::Paragraph)
                .name("Code")
                .size(18)
                .line_spacing(
                    LineSpacing::new()
                        .after(0)
                        .line(240)
                        .line_rule(LineSpacingType::Auto),
                )
                .fonts(run_fonts("Consolas", f.east_asia)),
        )
        .add_style(
            Style::new("CodeChar", StyleType::Character)
                .name("Code Char")
                .size(19)
                .fonts(run_fonts("Consolas", f.east_asia)),
        )
        .add_style(
            Style::new("Quote", StyleType::Paragraph)
                .name("Quote")
                .italic()
                .color("2A3550")
                .indent(Some(567), None, Some(567), None),
        )
        .add_style(
            Style::new("Hyperlink", StyleType::Character)
                .name("Hyperlink")
                .color(f.accent)
                .underline("single"),
        )
        .add_abstract_numbering(bullet_numbering())
        .add_numbering(Numbering::new(BULLET_NUMBERING, BULLET_NUMBERING));
    for (i, size) in heading_sizes.into_iter().enumerate() {
        let level = i + 1;
        let style = Style::new(format!("Heading{level}"), StyleType::Paragraph)
            .name(format!("Heading {level}"))
            .size(size)
            .bold()
            .color(if level <= 2 { f.accent } else { "1B1E24" })
            .fonts(run_fonts(f.heading, f.east_asia))
            .line_spacing(
                LineSpacing::new()
                    .before(if level == 1 { 360 } else { 240 })
                    .after(120),
            )
            .outline_lvl(i);
        docx = docx.add_style(style);
    }
    if opts.page_numbers {
        docx = docx.footer(
            Footer::new().add_paragraph(
                Paragraph::new()
                    .align(AlignmentType::Center)
                    .add_page_num(PageNum::new()),
            ),
        );
    }
    docx
}

fn bullet_numbering() -> AbstractNumbering {
    let glyphs = ["•", "◦", "▪"];
    let mut n = AbstractNumbering::new(BULLET_NUMBERING);
    for level in 0..9 {
        n = n.add_level(
            Level::new(
                level,
                Start::new(1),
                NumberFormat::new("bullet"),
                LevelText::new(glyphs[level % 3]),
                LevelJc::new("left"),
            )
            .indent(
                Some(360 * (level as i32 + 1)),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        );
    }
    n
}

fn ordered_numbering(id: usize, start: usize) -> AbstractNumbering {
    let mut n = AbstractNumbering::new(id);
    for level in 0..9 {
        let (format, text) = match level % 3 {
            0 => ("decimal", format!("%{}.", level + 1)),
            1 => ("lowerLetter", format!("%{}.", level + 1)),
            _ => ("lowerRoman", format!("%{}.", level + 1)),
        };
        n = n.add_level(
            Level::new(
                level,
                Start::new(if level == 0 { start } else { 1 }),
                NumberFormat::new(format),
                LevelText::new(text),
                LevelJc::new("left"),
            )
            .indent(
                Some(360 * (level as i32 + 1)),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        );
    }
    n
}

impl Ctx<'_> {
    fn push(&mut self, p: Paragraph) {
        self.docx = std::mem::take(&mut self.docx).add_paragraph(p);
    }

    fn push_table(&mut self, t: Table) {
        self.docx = std::mem::take(&mut self.docx).add_table(t);
    }

    fn block(&mut self, block: &Block, list_level: usize) {
        match block {
            Block::Heading { content, id, .. } => {
                let (depth, number, bookmark) = self.headings.get(id).cloned().unwrap_or_default();
                if depth == 0 {
                    // The title heading is printed once at the top.
                    return;
                }
                let mut p = Paragraph::new()
                    .style(&format!("Heading{}", depth.min(6)))
                    .keep_next(true);
                let bid = self.next_bookmark;
                self.next_bookmark += 1;
                p = p.add_bookmark_start(bid, bookmark);
                if self.opts.number_headings && !number.is_empty() {
                    p = p.add_run(Run::new().add_text(format!("{number}  ")));
                }
                p = self.add_inlines(p, content, Fmt::default());
                self.push(p.add_bookmark_end(bid));
            }
            Block::Paragraph { content } => {
                // A paragraph holding only images is centred.
                let only_images = !content.is_empty()
                    && content.iter().all(|i| {
                        matches!(
                            i,
                            Inline::Image { .. } | Inline::SoftBreak | Inline::LineBreak
                        )
                    });
                let mut p = Paragraph::new();
                if only_images {
                    p = p.align(AlignmentType::Center);
                }
                let p = self.add_inlines(p, content, Fmt::default());
                self.push(p);
            }
            Block::List {
                ordered,
                start,
                items,
            } => {
                let numbering = if *ordered {
                    let id = self.next_numbering;
                    self.next_numbering += 1;
                    self.docx = std::mem::take(&mut self.docx)
                        .add_abstract_numbering(ordered_numbering(id, *start))
                        .add_numbering(Numbering::new(id, id));
                    id
                } else {
                    BULLET_NUMBERING
                };
                for item in items {
                    let mut first = true;
                    for b in &item.blocks {
                        match b {
                            Block::Paragraph { content } if first => {
                                let mut p = Paragraph::new();
                                if let Some(done) = item.task {
                                    p =
                                        p.indent(
                                            Some(360 * (list_level as i32 + 1)),
                                            None,
                                            None,
                                            None,
                                        )
                                        .add_run(
                                            Run::new().add_text(if done { "☒ " } else { "☐ " }),
                                        );
                                } else {
                                    p = p.numbering(
                                        NumberingId::new(numbering),
                                        IndentLevel::new(list_level.min(8)),
                                    );
                                }
                                let p = self.add_inlines(p, content, Fmt::default());
                                self.push(p);
                            }
                            Block::List { .. } => self.block(b, list_level + 1),
                            other => self.block(other, list_level + 1),
                        }
                        first = false;
                    }
                }
            }
            Block::CodeBlock { code, .. } => {
                let lines: Vec<Paragraph> = code
                    .lines()
                    .map(|l| {
                        Paragraph::new()
                            .style("Code")
                            .add_run(Run::new().add_text(l.to_string()))
                    })
                    .collect();
                if self.opts.code_as_table {
                    let mut cell = TableCell::new()
                        .shading(Shading::new().shd_type(ShdType::Clear).fill("F6F8FA"));
                    for p in lines {
                        cell = cell.add_paragraph(p);
                    }
                    self.push_table(
                        Table::new(vec![TableRow::new(vec![cell])]).width(5000, WidthType::Pct),
                    );
                    self.push(Paragraph::new());
                } else {
                    for p in lines {
                        self.push(p);
                    }
                }
            }
            Block::Quote { blocks } => {
                for b in blocks {
                    match b {
                        Block::Paragraph { content } => {
                            let p = self.add_inlines(
                                Paragraph::new().style("Quote"),
                                content,
                                Fmt::default(),
                            );
                            self.push(p);
                        }
                        other => self.block(other, list_level),
                    }
                }
            }
            Block::Alert { kind, blocks } => {
                let mut cell = TableCell::new()
                    .shading(
                        Shading::new()
                            .shd_type(ShdType::Clear)
                            .fill(alert_fill(kind)),
                    )
                    .add_paragraph(
                        Paragraph::new().add_run(Run::new().add_text(title_case(kind)).bold()),
                    );
                for b in blocks {
                    if let Block::Paragraph { content } = b {
                        cell = cell.add_paragraph(self.add_inlines(
                            Paragraph::new(),
                            content,
                            Fmt::default(),
                        ));
                    }
                }
                self.push_table(
                    Table::new(vec![TableRow::new(vec![cell])]).width(5000, WidthType::Pct),
                );
                self.push(Paragraph::new());
            }
            Block::Table {
                align,
                header,
                rows,
            } => {
                let mut table_rows = Vec::new();
                let mk_cell = |ctx: &Ctx, cell: &[Inline], i: usize, head: bool| {
                    let mut p = Paragraph::new();
                    match align.get(i) {
                        Some(Align::Center) => p = p.align(AlignmentType::Center),
                        Some(Align::Right) => p = p.align(AlignmentType::Right),
                        _ => {}
                    }
                    let p = ctx.add_inlines(
                        p,
                        cell,
                        Fmt {
                            bold: head,
                            ..Fmt::default()
                        },
                    );
                    let mut c = TableCell::new().add_paragraph(p);
                    if head {
                        c = c.shading(Shading::new().shd_type(ShdType::Clear).fill("F4F5F7"));
                    }
                    c
                };
                table_rows.push(TableRow::new(
                    header
                        .iter()
                        .enumerate()
                        .map(|(i, c)| mk_cell(self, c, i, true))
                        .collect(),
                ));
                for row in rows {
                    let mut cells: Vec<TableCell> = row
                        .iter()
                        .enumerate()
                        .map(|(i, c)| mk_cell(self, c, i, false))
                        .collect();
                    while cells.len() < header.len() {
                        cells.push(TableCell::new().add_paragraph(Paragraph::new()));
                    }
                    table_rows.push(TableRow::new(cells));
                }
                self.push_table(Table::new(table_rows).width(5000, WidthType::Pct));
                self.push(Paragraph::new());
            }
            Block::Math { tex } => {
                self.push(
                    Paragraph::new().align(AlignmentType::Center).add_run(
                        Run::new()
                            .add_text(tex.clone())
                            .italic()
                            .fonts(run_fonts("Cambria Math", self.fonts.east_asia)),
                    ),
                );
            }
            Block::Html { html } => {
                let text = strip_tags(html);
                if !text.trim().is_empty() {
                    self.push(Paragraph::new().add_run(Run::new().add_text(text)));
                }
            }
            Block::Rule => {
                self.push(
                    Paragraph::new()
                        .align(AlignmentType::Center)
                        .add_run(Run::new().add_text("*  *  *").color("9AA1AD")),
                );
            }
            // Rendered at their references.
            Block::FootnoteDefinition { .. } => {}
        }
    }

    fn add_inlines(&self, mut p: Paragraph, inlines: &[Inline], fmt: Fmt) -> Paragraph {
        for piece in self.pieces(inlines, fmt) {
            p = match piece {
                Piece::Run(r) => p.add_run(*r),
                Piece::Link(h) => p.add_hyperlink(h),
            };
        }
        p
    }

    fn pieces(&self, inlines: &[Inline], fmt: Fmt) -> Vec<Piece> {
        let mut out = Vec::new();
        for inline in inlines {
            match inline {
                Inline::Text { text } => {
                    out.push(piece(fmt.apply(Run::new().add_text(text.clone()))))
                }
                Inline::Strong { content } => {
                    out.extend(self.pieces(content, Fmt { bold: true, ..fmt }))
                }
                Inline::Emph { content } => out.extend(self.pieces(
                    content,
                    Fmt {
                        italic: true,
                        ..fmt
                    },
                )),
                Inline::Strike { content } => out.extend(self.pieces(
                    content,
                    Fmt {
                        strike: true,
                        ..fmt
                    },
                )),
                Inline::Code { code } => out.push(piece(
                    fmt.apply(Run::new().add_text(code.clone()).style("CodeChar"))
                        .shading(Shading::new().shd_type(ShdType::Clear).fill("F0F2F5")),
                )),
                Inline::Math { tex, .. } => out.push(piece(
                    fmt.apply(
                        Run::new()
                            .add_text(tex.clone())
                            .italic()
                            .fonts(run_fonts("Cambria Math", self.fonts.east_asia)),
                    ),
                )),
                Inline::Link { url, content, .. } => {
                    let (target, kind) = match url.strip_prefix('#') {
                        Some(anchor) => (
                            self.headings
                                .get(anchor)
                                .map(|h| h.2.clone())
                                .unwrap_or_else(|| anchor.to_string()),
                            HyperlinkType::Anchor,
                        ),
                        None => (url.clone(), HyperlinkType::External),
                    };
                    let mut link = Hyperlink::new(target, kind);
                    for piece in self.pieces(content, fmt) {
                        if let Piece::Run(r) = piece {
                            link = link.add_run((*r).style("Hyperlink"));
                        }
                    }
                    out.push(Piece::Link(link));
                }
                Inline::Image { url, alt, .. } => out.push(piece(self.image_run(url, alt))),
                Inline::FootnoteRef { name, .. } => {
                    let mut note = Footnote::new();
                    for b in self
                        .footnotes
                        .get(name)
                        .map(Vec::as_slice)
                        .unwrap_or_default()
                    {
                        if let Block::Paragraph { content } = b {
                            note = note.add_content(self.add_inlines(
                                Paragraph::new(),
                                content,
                                Fmt::default(),
                            ));
                        }
                    }
                    out.push(piece(Run::new().add_footnote_reference(note)));
                }
                Inline::Html { html } => {
                    let text = strip_tags(html);
                    if !text.is_empty() {
                        out.push(piece(fmt.apply(Run::new().add_text(text))));
                    }
                }
                Inline::SoftBreak => out.push(piece(Run::new().add_text(" "))),
                Inline::LineBreak => out.push(piece(Run::new().add_break(BreakType::TextWrapping))),
            }
        }
        out
    }

    /// An embedded picture scaled to the text width, or `[alt]` when it cannot be loaded.
    fn image_run(&self, url: &str, alt: &str) -> Run {
        let placeholder = || {
            Run::new()
                .add_text(format!("[{}]", if alt.is_empty() { url } else { alt }))
                .color("5B6270")
        };
        let Some(img) = self.images.load(url) else {
            return placeholder();
        };
        if img.kind == ImageKind::Svg {
            return placeholder();
        }
        let Ok(decoded) = image::load_from_memory(&img.bytes) else {
            return placeholder();
        };
        let (w, h) = (decoded.width(), decoded.height());
        if w == 0 || h == 0 {
            return placeholder();
        }
        let png = if img.kind == ImageKind::Png {
            img.bytes
        } else {
            let mut out = Cursor::new(Vec::new());
            if decoded.write_to(&mut out, image::ImageFormat::Png).is_err() {
                return placeholder();
            }
            out.into_inner()
        };
        // 96 dpi pixels → EMU, capped at the text width.
        let mut w_emu = f64::from(w) * 9525.0;
        let mut h_emu = f64::from(h) * 9525.0;
        if w_emu > self.content_width_emu {
            let scale = self.content_width_emu / w_emu;
            w_emu *= scale;
            h_emu *= scale;
        }
        Run::new().add_image(Pic::new_with_dimensions(png, w, h).size(w_emu as u32, h_emu as u32))
    }
}

fn alert_fill(kind: &str) -> &'static str {
    match kind {
        "warning" | "caution" => "FFF6E5",
        "tip" => "E7F4EC",
        _ => "F4F7FE",
    }
}

/// Visible text of an HTML fragment.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}
