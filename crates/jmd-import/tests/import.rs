//! End-to-end imports against a local HTTP server (private addresses allowed for the test only).

use std::time::Duration;

use jmd_import::{import_url, FetchLimits, ImportError, ImportOptions, SourceKind};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0];

async fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (status, ct, body): (&str, &str, Vec<u8>) = match path.as_str() {
                    "/notes/a.md" => (
                        "200 OK",
                        "text/plain; charset=utf-8",
                        b"# Notes\n\n![chart](img/chart.png)\n\n<script>bad()</script>Done.\n"
                            .to_vec(),
                    ),
                    "/notes/img/chart.png" => ("200 OK", "image/png", PNG.to_vec()),
                    "/big.md" => ("200 OK", "text/markdown", vec![b'a'; 4096]),
                    "/bin" => ("200 OK", "application/zip", vec![0; 10]),
                    _ => ("404 Not Found", "text/plain", b"no".to_vec()),
                };
                let head = format!("HTTP/1.1 {status}\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(&body).await;
            });
        }
    });
    format!("http://{addr}")
}

fn opts(dir: Option<std::path::PathBuf>) -> ImportOptions {
    ImportOptions {
        limits: FetchLimits {
            allow_private: true,
            timeout: Duration::from_secs(5),
            ..Default::default()
        },
        image_dir: dir,
        image_rel_dir: "assets/notes".into(),
        record_source: true,
        today: "2026-10-26".into(),
    }
}

#[tokio::test]
async fn imports_markdown_and_saves_images() {
    let base = serve().await;
    let dir = std::env::temp_dir().join(format!("jmd-import-test-{}", std::process::id()));
    let note = import_url(&format!("{base}/notes/a.md"), &opts(Some(dir.clone())))
        .await
        .unwrap();
    assert_eq!(note.resolved.kind, SourceKind::RawMarkdown);
    assert_eq!(note.title, "Notes");
    assert!(
        note.markdown.starts_with("---\nsource: "),
        "{}",
        note.markdown
    );
    assert!(note.markdown.contains("imported: 2026-10-26"));
    assert!(!note.markdown.contains("bad()"));
    assert_eq!(note.images.saved.len(), 1, "{:?}", note.images);
    assert!(note
        .markdown
        .contains(&format!("]({})", note.images.saved[0])));
    let file = dir.join(note.images.saved[0].trim_start_matches("assets/notes/"));
    assert_eq!(std::fs::read(file).unwrap(), PNG);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn enforces_limits_and_types() {
    let base = serve().await;
    let mut small = opts(None);
    small.limits.max_bytes = 1024;
    assert!(matches!(
        import_url(&format!("{base}/big.md"), &small).await,
        Err(ImportError::TooLarge(_))
    ));
    assert!(matches!(
        import_url(&format!("{base}/bin"), &opts(None)).await,
        Err(ImportError::Unsupported(_))
    ));
    assert!(matches!(
        import_url(&format!("{base}/missing.md"), &opts(None)).await,
        Err(ImportError::Status(404))
    ));
}

#[tokio::test]
async fn refuses_private_addresses_by_default() {
    let base = serve().await;
    let err = import_url(&format!("{base}/notes/a.md"), &ImportOptions::default())
        .await
        .unwrap_err();
    assert!(matches!(err, ImportError::Forbidden(_)), "{err:?}");
}

#[tokio::test]
async fn prefetches_remote_images_for_export() {
    let base = serve().await;
    let md = format!("![a]({base}/notes/img/chart.png) ![b]({base}/missing.png) ![c](local.png)");
    let limits = FetchLimits {
        allow_private: true,
        ..Default::default()
    };
    let got = jmd_import::prefetch_images(&md, &limits).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[&format!("{base}/notes/img/chart.png")], PNG);
    // Private addresses stay blocked with default limits.
    assert!(jmd_import::prefetch_images(&md, &FetchLimits::default())
        .await
        .is_empty());
}
