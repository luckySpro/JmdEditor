//! Export settings shared by every format; mirrors the export dialog.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    #[default]
    Pdf,
    Docx,
    Html,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Pdf => "pdf",
            Format::Docx => "docx",
            Format::Html => "html",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Format::Pdf => "application/pdf",
            Format::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            Format::Html => "text/html; charset=utf-8",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Template {
    #[default]
    Clean,
    Academic,
    Business,
    Github,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Paper {
    #[default]
    A4,
    Letter,
}

impl Paper {
    /// Width and height in millimetres.
    pub fn size_mm(self) -> (f64, f64) {
        match self {
            Paper::A4 => (210.0, 297.0),
            Paper::Letter => (215.9, 279.4),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Margins {
    Narrow,
    #[default]
    Normal,
    Wide,
}

impl Margins {
    pub fn mm(self) -> f64 {
        match self {
            Margins::Narrow => 15.0,
            Margins::Normal => 25.0,
            Margins::Wide => 35.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportOptions {
    pub format: Format,
    pub template: Template,
    pub paper: Paper,
    pub margins: Margins,
    pub toc: bool,
    /// Deepest heading level listed in the table of contents (1–6).
    pub toc_depth: u8,
    pub number_headings: bool,
    pub page_numbers: bool,
    pub cover: bool,
    /// Overrides the document title.
    pub title: Option<String>,
    pub author: Option<String>,
    /// Shown on the cover, `YYYY-MM-DD`.
    pub date: Option<String>,
    /// PDF: bookmarks from headings.
    pub bookmarks: bool,
    /// PDF: archival PDF/A-2b.
    pub pdf_a: bool,
    /// Word: render code blocks as shaded single-cell tables.
    pub code_as_table: bool,
    /// HTML: embed images as data URIs.
    pub inline_images: bool,
    /// UI language for generated labels such as "Contents" (`en`, `zh-CN`, `zh-TW`, `ja`).
    pub lang: String,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: Format::Pdf,
            template: Template::Clean,
            paper: Paper::A4,
            margins: Margins::Normal,
            toc: true,
            toc_depth: 3,
            number_headings: true,
            page_numbers: true,
            cover: false,
            title: None,
            author: None,
            date: None,
            bookmarks: true,
            pdf_a: false,
            code_as_table: true,
            inline_images: true,
            lang: "en".into(),
        }
    }
}

impl ExportOptions {
    pub fn contents_label(&self) -> &'static str {
        match self.lang.as_str() {
            "zh-CN" | "zh" => "目录",
            "zh-TW" => "目錄",
            "ja" => "目次",
            _ => "Contents",
        }
    }

    pub fn toc_depth(&self) -> u8 {
        self.toc_depth.clamp(1, 6)
    }
}
