//! Import Markdown into JmdEditor from a link: repository files, gists, raw
//! Markdown and ordinary web pages (converted to Markdown).
//!
//! Fetching happens here, never in the WebView: HTTP(S) only, private networks
//! refused, size, time and redirect limits enforced.

pub mod convert;
pub mod fetch;
pub mod images;
pub mod resolve;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use fetch::FetchLimits;
pub use images::ImageReport;
pub use resolve::{resolve, Resolved, SourceKind};

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("invalid link: {0}")]
    InvalidUrl(String),
    #[error("{0} is a private or local address and cannot be imported")]
    Forbidden(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("the server answered with status {0}")]
    Status(u16),
    #[error("the file is larger than {} MB", .0 / (1024 * 1024))]
    TooLarge(usize),
    #[error("the link does not point to text or a web page ({0})")]
    Unsupported(String),
}

impl ImportError {
    /// Stable machine-readable code for the UI and the HTTP API.
    pub fn code(&self) -> &'static str {
        match self {
            ImportError::InvalidUrl(_) => "invalid_url",
            ImportError::Forbidden(_) => "forbidden",
            ImportError::Network(_) => "network",
            ImportError::Status(_) => "status",
            ImportError::TooLarge(_) => "too_large",
            ImportError::Unsupported(_) => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    pub limits: FetchLimits,
    /// Where to save images; `None` keeps remote image links.
    pub image_dir: Option<PathBuf>,
    /// How the note refers to `image_dir`, for example `assets/my-note`.
    pub image_rel_dir: String,
    /// Adds `source` and `imported` to the front matter.
    pub record_source: bool,
    /// Today's date (`YYYY-MM-DD`) for the `imported` key, supplied by the caller.
    pub today: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub title: String,
    pub markdown: String,
    pub resolved: Resolved,
    pub images: ImageReport,
    pub bytes: usize,
}

/// Suggested file name for an imported note.
pub fn suggested_file_name(title: &str, resolved: &Resolved) -> String {
    let from_title = jmd_core::slugify(title);
    let stem = if !from_title.is_empty() {
        from_title
    } else {
        url::Url::parse(&resolved.fetch_url)
            .ok()
            .and_then(|u| {
                u.path_segments()?
                    .filter(|s| !s.is_empty())
                    .next_back()
                    .map(str::to_string)
            })
            .map(|s| {
                s.trim_end_matches(".md")
                    .trim_end_matches(".mdx")
                    .trim_end_matches(".markdown")
                    .to_string()
            })
            .filter(|s| !s.is_empty() && s != "raw")
            .unwrap_or_else(|| "imported-note".to_string())
    };
    let stem: String = stem.chars().take(80).collect();
    format!("{stem}.md")
}

/// Fetches `input` and returns it as Markdown.
pub async fn import_url(input: &str, opts: &ImportOptions) -> Result<Imported, ImportError> {
    let resolved = resolve(input)?;
    let client = fetch::client(&opts.limits)?;
    let fetch_url =
        url::Url::parse(&resolved.fetch_url).map_err(|e| ImportError::InvalidUrl(e.to_string()))?;
    let fetched = fetch::fetch(&client, &fetch_url, &opts.limits).await?;
    let bytes = fetched.body.len();
    let ct = fetched.content_type.clone();
    let text = convert::decode(&fetched.body, &ct);

    let is_html = ct.starts_with("text/html")
        || ct.starts_with("application/xhtml")
        || (ct.is_empty()
            && text
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("<!doctype html"));
    let is_text =
        ct.starts_with("text/") || ct.is_empty() || ct.contains("markdown") || ct.contains("json");
    if !is_html && !is_text {
        return Err(ImportError::Unsupported(ct));
    }

    let (mut title, mut markdown) = if is_html && resolved.kind == SourceKind::WebPage {
        convert::html_to_markdown(&text, &fetched.final_url)
    } else {
        let md = convert::strip_dangerous_html(&text);
        let md = convert::absolutize_links(&md, &fetched.final_url);
        (String::new(), md)
    };
    if title.is_empty() {
        title = jmd_core::parse(&markdown).title().unwrap_or_default();
    }

    let mut images = ImageReport {
        saved: Vec::new(),
        failed: Vec::new(),
    };
    if let Some(dir) = &opts.image_dir {
        let (md, report) =
            images::localize_images(&markdown, &client, &opts.limits, dir, &opts.image_rel_dir)
                .await;
        markdown = md;
        images = report;
    }

    if opts.record_source {
        let mut entries = vec![("source", resolved.source_url.as_str())];
        if !opts.today.is_empty() {
            entries.push(("imported", opts.today.as_str()));
        }
        markdown = jmd_core::set_front_matter(&markdown, &entries);
    }

    Ok(Imported {
        title,
        markdown,
        resolved,
        images,
        bytes,
    })
}
