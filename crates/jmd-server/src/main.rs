use std::net::SocketAddr;

use jmd_server::{router, AppState, Config};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jmd_server=info,tower_http=info".into()),
        )
        .init();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("jmd-server: {e}");
            std::process::exit(2);
        }
    };
    let font_dirs = config.font_dirs.clone();
    let engine = tokio::task::spawn_blocking(move || jmd_export::PdfEngine::new(&font_dirs))
        .await
        .expect("font scan");
    tracing::info!(fonts = engine.family_names().len(), "PDF engine ready");

    let addr = config.addr;
    let app = router(AppState::new(config, engine));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("jmd-server: cannot listen on {addr}: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!(%addr, "listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .expect("server error");
}
