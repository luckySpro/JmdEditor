//! HTTP API behind the browser build and the WeChat mini-program.
//!
//! The same crates as the desktop app do the work: `jmd-import` for links and
//! `jmd-export` for Word, PDF and HTML. Requests are size-limited, exports run
//! a few at a time, and each client address gets a fixed number of calls a minute.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use jmd_export::{ExportOptions, Format, ImageSource, PdfEngine};
use jmd_import::{FetchLimits, ImportOptions, Resolved};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

/// Markdown sent for export or rendering may not exceed this.
pub const MAX_MARKDOWN_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Config {
    pub addr: SocketAddr,
    /// Built web UI (`apps/desktop/dist`) served at `/` when set.
    pub static_dir: Option<PathBuf>,
    /// Extra folders with fonts for PDF export, for example Noto CJK.
    pub font_dirs: Vec<PathBuf>,
    /// Browser origins allowed to call the API; empty allows any origin.
    pub cors_origins: Vec<String>,
    pub max_concurrent_exports: usize,
    pub export_timeout: Duration,
    /// Calls per client address per minute; 0 turns the limit off.
    pub rate_per_minute: u32,
    /// Take the client address from `X-Forwarded-For` (only behind your own proxy).
    pub trust_proxy: bool,
    pub fetch: FetchLimits,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], 8787)),
            static_dir: None,
            font_dirs: Vec::new(),
            cors_origins: Vec::new(),
            max_concurrent_exports: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(2)
                .clamp(1, 8),
            export_timeout: Duration::from_secs(60),
            rate_per_minute: 60,
            trust_proxy: false,
            fetch: FetchLimits::default(),
        }
    }
}

impl Config {
    /// Reads `JMD_*` environment variables; see the README for the list.
    pub fn from_env() -> Result<Self, String> {
        let mut c = Config::default();
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        if let Some(v) = var("JMD_ADDR") {
            c.addr = v.parse().map_err(|e| format!("JMD_ADDR: {e}"))?;
        }
        c.static_dir = var("JMD_STATIC").map(PathBuf::from);
        if let Some(v) = var("JMD_FONT_DIRS") {
            c.font_dirs = std::env::split_paths(&v).collect();
        }
        if let Some(v) = var("JMD_CORS_ORIGINS") {
            c.cors_origins = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Some(v) = var("JMD_MAX_CONCURRENT") {
            c.max_concurrent_exports = v
                .parse::<usize>()
                .map_err(|e| format!("JMD_MAX_CONCURRENT: {e}"))?
                .max(1);
        }
        if let Some(v) = var("JMD_EXPORT_TIMEOUT_SECS") {
            c.export_timeout = Duration::from_secs(
                v.parse()
                    .map_err(|e| format!("JMD_EXPORT_TIMEOUT_SECS: {e}"))?,
            );
        }
        if let Some(v) = var("JMD_RATE_PER_MIN") {
            c.rate_per_minute = v.parse().map_err(|e| format!("JMD_RATE_PER_MIN: {e}"))?;
        }
        c.trust_proxy = matches!(
            var("JMD_TRUST_PROXY").as_deref(),
            Some("1" | "true" | "yes")
        );
        Ok(c)
    }
}

#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    engine: Arc<PdfEngine>,
    exports: Arc<Semaphore>,
    limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(config: Config, engine: PdfEngine) -> Self {
        Self {
            exports: Arc::new(Semaphore::new(config.max_concurrent_exports)),
            limiter: Arc::new(RateLimiter::new(config.rate_per_minute)),
            engine: Arc::new(engine),
            config: Arc::new(config),
        }
    }
}

/// Fixed one-minute windows per client address.
pub struct RateLimiter {
    per_minute: u32,
    windows: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(per_minute: u32) -> Self {
        Self {
            per_minute,
            windows: Mutex::new(HashMap::new()),
        }
    }

    pub fn allow(&self, ip: IpAddr) -> bool {
        if self.per_minute == 0 {
            return true;
        }
        let now = Instant::now();
        let mut map = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() > 10_000 {
            map.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
        }
        let entry = map.entry(ip).or_insert((now, 0));
        if now.duration_since(entry.0) >= Duration::from_secs(60) {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.per_minute
    }
}

/// Errors as `{ code, message }`, the same shape the desktop commands return.
#[derive(Debug, Serialize)]
pub struct ApiError {
    #[serde(skip)]
    status: StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.to_string(),
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(&self)).into_response()
    }
}

impl From<jmd_import::ImportError> for ApiError {
    fn from(e: jmd_import::ImportError) -> Self {
        let status = match e.code() {
            "invalid_url" => StatusCode::BAD_REQUEST,
            "forbidden" => StatusCode::FORBIDDEN,
            "too_large" => StatusCode::PAYLOAD_TOO_LARGE,
            "unsupported" => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        };
        ApiError::new(status, e.code(), e.to_string())
    }
}

fn client_ip(req: &Request, trust_proxy: bool) -> Option<IpAddr> {
    if trust_proxy {
        let forwarded = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .and_then(|v| v.trim().parse().ok());
        if forwarded.is_some() {
            return forwarded;
        }
    }
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
}

async fn rate_limit(State(state): State<AppState>, req: Request, next: Next) -> Response {
    if let Some(ip) = client_ip(&req, state.config.trust_proxy) {
        if !state.limiter.allow(ip) {
            return ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "busy",
                "too many requests; try again in a minute",
            )
            .into_response();
        }
    }
    next.run(req).await
}

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/info", get(info))
        .route("/api/resolve", post(resolve))
        .route("/api/import", post(import))
        .route("/api/render", post(render))
        .route("/api/export", post(export))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .layer(DefaultBodyLimit::max(MAX_MARKDOWN_BYTES + 64 * 1024))
        .layer(cors(&state.config));

    let app = match &state.config.static_dir {
        Some(dir) => api
            .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")))),
        None => api,
    };
    app.with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}

fn cors(config: &Config) -> CorsLayer {
    let origin = if config.cors_origins.is_empty() {
        AllowOrigin::any()
    } else {
        AllowOrigin::list(
            config
                .cors_origins
                .iter()
                .filter_map(|o| HeaderValue::from_str(o).ok()),
        )
    };
    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .expose_headers([header::CONTENT_DISPOSITION])
        .max_age(Duration::from_secs(3600))
}

async fn info(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "jmd-server",
        "version": env!("CARGO_PKG_VERSION"),
        "author": "M&J Group",
        "fonts": state.engine.family_names().len(),
        "maxMarkdownBytes": MAX_MARKDOWN_BYTES,
    }))
}

#[derive(Deserialize)]
struct UrlRequest {
    url: String,
}

async fn resolve(Json(req): Json<UrlRequest>) -> Result<Json<Resolved>, ApiError> {
    Ok(Json(jmd_import::resolve(&req.url)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    url: String,
    #[serde(default = "yes")]
    record_source: bool,
}

fn yes() -> bool {
    true
}

/// Same shape as the desktop `import_url` result; images keep their web links.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResponse {
    title: String,
    markdown: String,
    suggested_path: Option<String>,
    suggested_name: String,
    resolved: Resolved,
    images: jmd_import::ImageReport,
    bytes: usize,
}

fn today() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

async fn import(
    State(state): State<AppState>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<ImportResponse>, ApiError> {
    let opts = ImportOptions {
        limits: state.config.fetch.clone(),
        record_source: req.record_source,
        today: today(),
        ..Default::default()
    };
    let note = jmd_import::import_url(&req.url, &opts).await?;
    let suggested_name = jmd_import::suggested_file_name(&note.title, &note.resolved);
    Ok(Json(ImportResponse {
        title: note.title,
        markdown: note.markdown,
        suggested_path: None,
        suggested_name,
        resolved: note.resolved,
        images: note.images,
        bytes: note.bytes,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportRequest {
    markdown: String,
    #[serde(default)]
    options: ExportOptions,
    /// Download name; derived from the title when missing.
    file_name: Option<String>,
}

fn check_size(markdown: &str) -> Result<(), ApiError> {
    if markdown.len() > MAX_MARKDOWN_BYTES {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "too_large",
            "the note is larger than 5 MB",
        ));
    }
    Ok(())
}

async fn render(Json(req): Json<ExportRequest>) -> Result<Response, ApiError> {
    check_size(&req.markdown)?;
    // Web images keep their links here; the viewer's browser loads them.
    let html = tokio::task::spawn_blocking(move || {
        let doc = jmd_core::parse(&req.markdown);
        jmd_export::html::render(&doc, &req.options, &ImageSource::default())
    })
    .await
    .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "export", e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response())
}

/// `attachment; filename="ascii"; filename*=UTF-8''percent-encoded`
fn content_disposition(name: &str) -> String {
    let ascii: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut encoded = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

fn download_name(req_name: Option<&str>, doc: &jmd_core::Document, opts: &ExportOptions) -> String {
    let ext = opts.format.extension();
    let stem = req_name
        .map(|n| n.rsplit_once('.').map(|(s, _)| s).unwrap_or(n).to_string())
        .filter(|s| !s.trim().is_empty())
        .or_else(|| jmd_export::display_title(doc, opts))
        .unwrap_or_else(|| "document".to_string());
    let stem: String = stem
        .chars()
        .filter(|c| {
            !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') && !c.is_control()
        })
        .take(80)
        .collect();
    format!("{}.{ext}", stem.trim())
}

async fn export(
    State(state): State<AppState>,
    Json(req): Json<ExportRequest>,
) -> Result<Response, ApiError> {
    check_size(&req.markdown)?;
    let permit = tokio::time::timeout(
        Duration::from_secs(20),
        state.exports.clone().acquire_owned(),
    )
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            "the export service is busy; try again shortly",
        )
    })?
    .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "export", e.to_string()))?;

    let provided = jmd_import::prefetch_images(&req.markdown, &state.config.fetch).await;
    let engine = state.engine.clone();
    let format = req.options.format;
    let job = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let doc = jmd_core::parse(&req.markdown);
        let name = download_name(req.file_name.as_deref(), &doc, &req.options);
        let images = ImageSource {
            base_dir: None,
            provided,
        };
        jmd_export::export(&doc, &req.options, &images, &engine).map(|bytes| (name, bytes))
    });
    let (name, bytes) = tokio::time::timeout(state.config.export_timeout, job)
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "export",
                "the export took too long",
            )
        })?
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "export", e.to_string()))?
        .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "export", e.to_string()))?;

    let mime = match format {
        Format::Html => "text/html; charset=utf-8",
        f => f.mime(),
    };
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, content_disposition(&name))
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(bytes))
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "export", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_counts_per_address() {
        let l = RateLimiter::new(2);
        let a: IpAddr = "1.2.3.4".parse().unwrap();
        let b: IpAddr = "5.6.7.8".parse().unwrap();
        assert!(l.allow(a) && l.allow(a));
        assert!(!l.allow(a));
        assert!(l.allow(b));
        assert!(RateLimiter::new(0).allow(a));
    }

    #[test]
    fn download_names_are_safe() {
        let doc = jmd_core::parse("# Plan: Q4/2026\n");
        let opts = ExportOptions {
            format: Format::Docx,
            ..Default::default()
        };
        assert_eq!(download_name(None, &doc, &opts), "Plan Q42026.docx");
        assert_eq!(
            download_name(Some("团队手册.md"), &doc, &opts),
            "团队手册.docx"
        );
        let cd = content_disposition("团队手册.docx");
        assert!(
            cd.starts_with("attachment; filename=\"____.docx\"; filename*=UTF-8''%E5%9B%A2"),
            "{cd}"
        );
    }
}
