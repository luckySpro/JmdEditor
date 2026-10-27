//! Turns what the user pasted into the URL we actually fetch.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::ImportError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    GitHub,
    Gitee,
    GitLab,
    Gist,
    /// A link that already points at a Markdown file.
    RawMarkdown,
    /// Anything else; HTML is converted to Markdown.
    WebPage,
}

impl SourceKind {
    pub fn label(self) -> &'static str {
        match self {
            SourceKind::GitHub => "GitHub file",
            SourceKind::Gitee => "Gitee file",
            SourceKind::GitLab => "GitLab file",
            SourceKind::Gist => "Gist",
            SourceKind::RawMarkdown => "Markdown file",
            SourceKind::WebPage => "Web page",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolved {
    /// What the user gave us, normalised.
    pub source_url: String,
    /// What we will download.
    pub fetch_url: String,
    pub kind: SourceKind,
    /// A short explanation for the import dialog.
    pub note: String,
}

const MARKDOWN_EXTENSIONS: &[&str] = &[".md", ".markdown", ".mdx", ".mdown", ".mkd", ".txt"];

pub fn looks_like_markdown_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    MARKDOWN_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Parses user input, adding `https://` when the scheme is missing.
pub fn parse_input(input: &str) -> Result<Url, ImportError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ImportError::InvalidUrl("the link is empty".into()));
    }
    let with_scheme = if input.contains("://") {
        input.to_string()
    } else {
        format!("https://{input}")
    };
    let url = Url::parse(&with_scheme).map_err(|e| ImportError::InvalidUrl(e.to_string()))?;
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(ImportError::InvalidUrl(format!(
                "{other}: links are not supported"
            )))
        }
    }
    if url.host_str().is_none() {
        return Err(ImportError::InvalidUrl("the link has no host".into()));
    }
    Ok(url)
}

/// Maps repository and gist links to their raw files.
pub fn resolve(input: &str) -> Result<Resolved, ImportError> {
    let url = parse_input(input)?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let segments: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|p| !p.is_empty()).collect())
        .unwrap_or_default();
    let source_url = url.to_string();

    let done = |fetch: String, kind: SourceKind, note: &str| {
        Ok(Resolved {
            source_url: source_url.clone(),
            fetch_url: fetch,
            kind,
            note: note.to_string(),
        })
    };

    match host.as_str() {
        "github.com" | "www.github.com" => {
            // /owner/repo/blob/ref/path...
            if segments.len() >= 5 && (segments[2] == "blob" || segments[2] == "raw") {
                let fetch = format!(
                    "https://raw.githubusercontent.com/{}/{}/{}",
                    segments[0],
                    segments[1],
                    segments[3..].join("/")
                );
                return done(
                    fetch,
                    SourceKind::GitHub,
                    "Will fetch the raw file from raw.githubusercontent.com",
                );
            }
            // /owner/repo → README on the default branch
            if segments.len() == 2 {
                let fetch = format!(
                    "https://raw.githubusercontent.com/{}/{}/HEAD/README.md",
                    segments[0], segments[1]
                );
                return done(
                    fetch,
                    SourceKind::GitHub,
                    "Will fetch the repository README",
                );
            }
        }
        "raw.githubusercontent.com" => {
            return done(source_url.clone(), SourceKind::GitHub, "Raw GitHub file");
        }
        "gist.github.com" => {
            if segments.len() >= 2 {
                let fetch = format!(
                    "https://gist.githubusercontent.com/{}/{}/raw",
                    segments[0], segments[1]
                );
                return done(fetch, SourceKind::Gist, "Will fetch the gist's first file");
            }
        }
        "gist.githubusercontent.com" => {
            return done(source_url.clone(), SourceKind::Gist, "Raw gist file");
        }
        "gitee.com" | "www.gitee.com" => {
            if segments.len() >= 5 && segments[2] == "blob" {
                let fetch = format!(
                    "https://gitee.com/{}/{}/raw/{}",
                    segments[0],
                    segments[1],
                    segments[3..].join("/")
                );
                return done(
                    fetch,
                    SourceKind::Gitee,
                    "Will fetch the raw file from Gitee",
                );
            }
            if segments.len() >= 5 && segments[2] == "raw" {
                return done(source_url.clone(), SourceKind::Gitee, "Raw Gitee file");
            }
        }
        "gitlab.com" | "www.gitlab.com" => {
            if let Some(dash) = segments.iter().position(|s| *s == "-") {
                if segments.get(dash + 1) == Some(&"blob") {
                    let mut parts: Vec<&str> = segments.clone();
                    parts[dash + 1] = "raw";
                    let fetch = format!("https://gitlab.com/{}", parts.join("/"));
                    return done(
                        fetch,
                        SourceKind::GitLab,
                        "Will fetch the raw file from GitLab",
                    );
                }
                if segments.get(dash + 1) == Some(&"raw") {
                    return done(source_url.clone(), SourceKind::GitLab, "Raw GitLab file");
                }
            }
        }
        _ => {}
    }

    if looks_like_markdown_path(url.path()) {
        return done(source_url.clone(), SourceKind::RawMarkdown, "Markdown file");
    }
    done(
        source_url.clone(),
        SourceKind::WebPage,
        "The main article will be converted to Markdown",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetch(input: &str) -> (String, SourceKind) {
        let r = resolve(input).unwrap();
        (r.fetch_url, r.kind)
    }

    #[test]
    fn github_blob_becomes_raw() {
        assert_eq!(
            fetch("https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/updater.mdx"),
            (
                "https://raw.githubusercontent.com/tauri-apps/tauri-docs/v2/src/content/docs/plugin/updater.mdx".into(),
                SourceKind::GitHub
            )
        );
    }

    #[test]
    fn github_repo_root_fetches_readme() {
        assert_eq!(
            fetch("github.com/luckySpro/JmdEditor").0,
            "https://raw.githubusercontent.com/luckySpro/JmdEditor/HEAD/README.md"
        );
    }

    #[test]
    fn gist_and_gitee_and_gitlab() {
        assert_eq!(
            fetch("https://gist.github.com/mj/abc123"),
            (
                "https://gist.githubusercontent.com/mj/abc123/raw".into(),
                SourceKind::Gist
            )
        );
        assert_eq!(
            fetch("https://gitee.com/mj-group/handbook/blob/master/README.md"),
            (
                "https://gitee.com/mj-group/handbook/raw/master/README.md".into(),
                SourceKind::Gitee
            )
        );
        assert_eq!(
            fetch("https://gitlab.com/group/sub/proj/-/blob/main/docs/a.md"),
            (
                "https://gitlab.com/group/sub/proj/-/raw/main/docs/a.md".into(),
                SourceKind::GitLab
            )
        );
    }

    #[test]
    fn markdown_links_and_pages() {
        assert_eq!(
            fetch("https://example.com/notes/a.md").1,
            SourceKind::RawMarkdown
        );
        assert_eq!(
            fetch("https://example.com/blog/post").1,
            SourceKind::WebPage
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(resolve("").is_err());
        assert!(resolve("file:///etc/passwd").is_err());
        assert!(resolve("javascript:alert(1)").is_err());
    }
}
