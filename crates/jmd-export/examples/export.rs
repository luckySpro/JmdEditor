//! `cargo run -p jmd-export --example export -- <file.md> <out.pdf|out.docx|out.html> [--cover]`

use std::path::Path;

use jmd_export::{ExportOptions, Format, ImageSource, PdfEngine};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (input, output) = match (args.first(), args.get(1)) {
        (Some(i), Some(o)) => (Path::new(i), Path::new(o)),
        _ => {
            eprintln!("usage: export <file.md> <out.pdf|out.docx|out.html> [--cover]");
            std::process::exit(2);
        }
    };
    let format = match output.extension().and_then(|e| e.to_str()) {
        Some("pdf") => Format::Pdf,
        Some("docx") => Format::Docx,
        _ => Format::Html,
    };
    let source = std::fs::read_to_string(input).expect("read input");
    let doc = jmd_core::parse(&source);
    let opts = ExportOptions {
        format,
        cover: args.iter().any(|a| a == "--cover"),
        author: doc.meta("author").map(str::to_string),
        date: Some("2026-10-26".into()),
        ..Default::default()
    };
    let images = ImageSource {
        base_dir: input.parent().map(Path::to_path_buf),
        ..Default::default()
    };
    let started = std::time::Instant::now();
    let engine = PdfEngine::new(&[]);
    let bytes = jmd_export::export(&doc, &opts, &images, &engine).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    std::fs::write(output, &bytes).expect("write output");
    eprintln!("{} bytes in {:?}", bytes.len(), started.elapsed());
}
