//! Loading the pictures a note refers to. Exports never touch the network:
//! local files, `data:` URIs and anything the caller pre-fetched are used.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use base64::Engine;

/// Image bytes with a detected format.
#[derive(Debug, Clone)]
pub struct LoadedImage {
    pub bytes: Vec<u8>,
    pub kind: ImageKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
    Svg,
    Bmp,
}

impl ImageKind {
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            Some(ImageKind::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(ImageKind::Jpeg)
        } else if bytes.starts_with(b"GIF8") {
            Some(ImageKind::Gif)
        } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(ImageKind::Webp)
        } else if bytes.starts_with(b"BM") {
            Some(ImageKind::Bmp)
        } else {
            let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).to_ascii_lowercase();
            (head.contains("<svg")).then_some(ImageKind::Svg)
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpg",
            ImageKind::Gif => "gif",
            ImageKind::Webp => "webp",
            ImageKind::Svg => "svg",
            ImageKind::Bmp => "bmp",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            ImageKind::Png => "image/png",
            ImageKind::Jpeg => "image/jpeg",
            ImageKind::Gif => "image/gif",
            ImageKind::Webp => "image/webp",
            ImageKind::Svg => "image/svg+xml",
            ImageKind::Bmp => "image/bmp",
        }
    }
}

/// Where image links are looked up.
#[derive(Debug, Clone, Default)]
pub struct ImageSource {
    /// Folder of the note; relative links resolve against it and may not leave it.
    pub base_dir: Option<PathBuf>,
    /// Bytes supplied by the caller, keyed by the exact link in the Markdown.
    pub provided: HashMap<String, Vec<u8>>,
}

/// Images above this size are skipped.
pub const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

impl ImageSource {
    pub fn load(&self, url: &str) -> Option<LoadedImage> {
        let bytes = if let Some(b) = self.provided.get(url) {
            b.clone()
        } else if let Some(rest) = url.strip_prefix("data:") {
            let (meta, data) = rest.split_once(',')?;
            if !meta.ends_with(";base64") {
                return None;
            }
            base64::engine::general_purpose::STANDARD
                .decode(data.trim())
                .ok()?
        } else if url.contains("://") {
            return None;
        } else {
            let path = self.local_path(url)?;
            let meta = std::fs::metadata(&path).ok()?;
            if meta.len() as usize > MAX_IMAGE_BYTES {
                return None;
            }
            std::fs::read(path).ok()?
        };
        if bytes.len() > MAX_IMAGE_BYTES {
            return None;
        }
        let kind = ImageKind::sniff(&bytes)?;
        Some(LoadedImage { bytes, kind })
    }

    /// Resolves a relative link inside `base_dir`, refusing `..` escapes and absolute paths.
    fn local_path(&self, url: &str) -> Option<PathBuf> {
        let base = self.base_dir.as_ref()?;
        let clean = url.split(['?', '#']).next()?;
        let decoded = percent_decode(clean);
        let rel = Path::new(&decoded);
        let mut out = base.clone();
        for c in rel.components() {
            match c {
                Component::Normal(p) => out.push(p),
                Component::CurDir => {}
                _ => return None,
            }
        }
        Some(out)
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(v) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_paths_outside_the_note_folder() {
        let src = ImageSource {
            base_dir: Some(PathBuf::from("/notes")),
            ..Default::default()
        };
        assert_eq!(
            src.local_path("img/a.png"),
            Some(PathBuf::from("/notes/img/a.png"))
        );
        assert_eq!(
            src.local_path("./img/my%20pic.png"),
            Some(PathBuf::from("/notes/img/my pic.png"))
        );
        assert_eq!(src.local_path("../secret.png"), None);
        assert_eq!(src.local_path("/etc/passwd"), None);
    }

    #[test]
    fn decodes_data_uris_and_sniffs() {
        let png = [0x89, b'P', b'N', b'G', 1, 2, 3];
        let uri = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        );
        let img = ImageSource::default().load(&uri).unwrap();
        assert_eq!(img.kind, ImageKind::Png);
        assert!(ImageSource::default()
            .load("https://x.test/a.png")
            .is_none());
    }
}
