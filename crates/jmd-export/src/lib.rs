//! JmdEditor exporters. Every format reads the same [`jmd_core::Document`], so
//! numbering, order and the table of contents agree across Word, PDF and HTML.

pub mod docx;
pub mod html;
pub mod images;
pub mod math;
pub mod options;
pub mod pdf;

pub use images::ImageSource;
pub use options::{ExportOptions, Format, Margins, Paper, Template};
pub use pdf::PdfEngine;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Word export failed: {0}")]
    Docx(String),
    #[error("PDF export failed: {0}")]
    Pdf(String),
}

/// The title printed at the top of an export: the explicit option, the front
/// matter `title`, or a level-1 heading that is the document's only one.
pub fn display_title(doc: &jmd_core::Document, opts: &ExportOptions) -> Option<String> {
    opts.title
        .clone()
        .or_else(|| doc.meta("title").map(str::to_string))
        .or_else(|| {
            jmd_core::outline(doc)
                .into_iter()
                .find(|e| e.number.is_empty())
                .map(|e| e.text)
        })
}

/// Exports Markdown source in the format chosen in `opts`.
pub fn export_markdown(
    source: &str,
    opts: &ExportOptions,
    images: &ImageSource,
    engine: &PdfEngine,
) -> Result<Vec<u8>, ExportError> {
    let doc = jmd_core::parse(source);
    export(&doc, opts, images, engine)
}

pub fn export(
    doc: &jmd_core::Document,
    opts: &ExportOptions,
    images: &ImageSource,
    engine: &PdfEngine,
) -> Result<Vec<u8>, ExportError> {
    match opts.format {
        Format::Pdf => engine.render(doc, opts, images),
        Format::Docx => docx::render(doc, opts, images),
        Format::Html => Ok(html::render(doc, opts, images).into_bytes()),
    }
}
