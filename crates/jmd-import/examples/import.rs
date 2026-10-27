//! `cargo run -p jmd-import --example import -- <url>` prints the imported Markdown.

#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).expect("usage: import <url>");
    let opts = jmd_import::ImportOptions {
        record_source: true,
        ..Default::default()
    };
    match jmd_import::import_url(&url, &opts).await {
        Ok(note) => {
            eprintln!(
                "{} · {} · {} bytes",
                note.resolved.kind.label(),
                note.title,
                note.bytes
            );
            println!("{}", note.markdown);
        }
        Err(e) => {
            eprintln!("error ({}): {e}", e.code());
            std::process::exit(1);
        }
    }
}
