//! Saves remote images next to a note so it works offline and exports with its pictures.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::fetch::{fetch, FetchLimits};

/// At most this many images are downloaded per import.
pub const MAX_IMAGES: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageReport {
    pub saved: Vec<String>,
    pub failed: Vec<String>,
}

fn extension_for(content_type: &str, url: &Url) -> Option<&'static str> {
    let ct = content_type.split(';').next().unwrap_or("").trim();
    let by_type = match ct {
        "image/png" => Some("png"),
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "image/avif" => Some("avif"),
        "image/bmp" => Some("bmp"),
        _ => None,
    };
    by_type.or_else(|| {
        let path = url.path().to_ascii_lowercase();
        ["png", "jpg", "jpeg", "gif", "webp", "svg", "avif"]
            .into_iter()
            .find(|ext| path.ends_with(&format!(".{ext}")))
            .map(|ext| if ext == "jpeg" { "jpg" } else { ext })
    })
}

/// Stable file name for an image URL: the first 16 hex digits of its SHA-256.
pub fn file_name_for(url: &str, ext: &str) -> String {
    let digest = Sha256::digest(url.as_bytes());
    format!("{}.{ext}", &hex::encode(digest)[..16])
}

/// Downloads every absolute http(s) image in `markdown` into `dir` and points the
/// Markdown at `rel_dir/<file>`. Failures leave the remote link in place.
pub async fn localize_images(
    markdown: &str,
    client: &reqwest::Client,
    limits: &FetchLimits,
    dir: &Path,
    rel_dir: &str,
) -> (String, ImageReport) {
    let mut out = markdown.to_string();
    let mut report = ImageReport {
        saved: Vec::new(),
        failed: Vec::new(),
    };
    let mut urls = jmd_core::parse(markdown).image_urls();
    urls.sort();
    urls.dedup();

    for raw in urls.into_iter().take(MAX_IMAGES) {
        let Ok(url) = Url::parse(&raw) else { continue };
        if !matches!(url.scheme(), "http" | "https") {
            continue;
        }
        let result = async {
            let fetched = fetch(client, &url, limits)
                .await
                .map_err(|e| e.to_string())?;
            let ext = extension_for(&fetched.content_type, &url).ok_or("not an image")?;
            if fetched.content_type.starts_with("text/html") {
                return Err("not an image".to_string());
            }
            tokio::fs::create_dir_all(dir)
                .await
                .map_err(|e| e.to_string())?;
            let name = file_name_for(&raw, ext);
            tokio::fs::write(dir.join(&name), &fetched.body)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<String, String>(name)
        }
        .await;
        match result {
            Ok(name) => {
                let local = format!("{}/{}", rel_dir.trim_end_matches('/'), name);
                out = out.replace(&format!("]({raw}"), &format!("]({local}"));
                report.saved.push(local);
            }
            Err(_) => report.failed.push(raw),
        }
    }
    (out, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable_and_short() {
        let a = file_name_for("https://x.test/a.png", "png");
        assert_eq!(a, file_name_for("https://x.test/a.png", "png"));
        assert_eq!(a.len(), 16 + 4);
        assert_ne!(a, file_name_for("https://x.test/b.png", "png"));
    }

    #[test]
    fn picks_extension_from_type_then_path() {
        let u = Url::parse("https://x.test/pic.jpeg?w=1").unwrap();
        assert_eq!(extension_for("image/webp", &u), Some("webp"));
        assert_eq!(extension_for("application/octet-stream", &u), Some("jpg"));
        assert_eq!(
            extension_for("text/html", &Url::parse("https://x.test/").unwrap()),
            None
        );
    }
}
