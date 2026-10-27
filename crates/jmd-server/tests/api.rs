//! The HTTP API, driven in-process with `tower::ServiceExt::oneshot`.

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use jmd_export::PdfEngine;
use jmd_server::{router, AppState, Config};
use tower::ServiceExt;

fn engine() -> PdfEngine {
    PdfEngine::new(&[])
}

fn app(config: Config) -> axum::Router {
    router(AppState::new(config, engine()))
}

fn post(path: &str, body: serde_json::Value) -> Request<Body> {
    let mut req = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([203, 0, 113, 7], 5000))));
    req
}

async fn bytes(res: axum::response::Response) -> Vec<u8> {
    res.into_body().collect().await.unwrap().to_bytes().to_vec()
}

#[tokio::test]
async fn exports_word_and_pdf() {
    let app = app(Config::default());
    let md = "# 团队手册\n\n## Weekly rhythm\n\nPlanning on Monday.\n";

    let res = app
        .clone()
        .oneshot(post(
            "/api/export",
            serde_json::json!({ "markdown": md, "options": { "format": "docx" } }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cd = res.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        cd.contains("filename*=UTF-8''%E5%9B%A2%E9%98%9F%E6%89%8B%E5%86%8C.docx"),
        "{cd}"
    );
    assert!(bytes(res).await.starts_with(b"PK"));

    let res = app
        .oneshot(post("/api/export", serde_json::json!({ "markdown": md, "options": { "format": "pdf" }, "fileName": "handbook.md" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "application/pdf");
    assert!(res.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .contains("handbook.pdf"));
    assert!(bytes(res).await.starts_with(b"%PDF"));
}

#[tokio::test]
async fn renders_html_and_resolves_links() {
    let app = app(Config::default());
    let res = app
        .clone()
        .oneshot(post("/api/render", serde_json::json!({ "markdown": "# T\n\n## A\n", "options": { "numberHeadings": true } })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let html = String::from_utf8(bytes(res).await).unwrap();
    assert!(html.contains("<h2 id=\"a\">"), "{html}");

    let res = app
        .oneshot(post(
            "/api/resolve",
            serde_json::json!({ "url": "https://github.com/o/r/blob/main/README.md" }),
        ))
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes(res).await).unwrap();
    assert_eq!(v["kind"], "gitHub");
    assert_eq!(
        v["fetchUrl"],
        "https://raw.githubusercontent.com/o/r/main/README.md"
    );
}

#[tokio::test]
async fn refuses_private_addresses_and_bad_input() {
    let app = app(Config::default());
    let res = app
        .clone()
        .oneshot(post(
            "/api/import",
            serde_json::json!({ "url": "http://127.0.0.1:9/a.md" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let v: serde_json::Value = serde_json::from_slice(&bytes(res).await).unwrap();
    assert_eq!(v["code"], "forbidden");

    let res = app
        .clone()
        .oneshot(post(
            "/api/import",
            serde_json::json!({ "url": "not a link" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let huge = "a".repeat(jmd_server::MAX_MARKDOWN_BYTES + 10);
    let res = app
        .oneshot(post("/api/export", serde_json::json!({ "markdown": huge })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn limits_calls_per_client() {
    let app = app(Config {
        rate_per_minute: 2,
        ..Default::default()
    });
    for _ in 0..2 {
        let res = app
            .clone()
            .oneshot(post(
                "/api/resolve",
                serde_json::json!({ "url": "https://x.test/a.md" }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = app
        .oneshot(post(
            "/api/resolve",
            serde_json::json!({ "url": "https://x.test/a.md" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}
