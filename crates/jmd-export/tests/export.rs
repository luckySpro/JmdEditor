use std::io::Read;
use std::path::PathBuf;

use jmd_export::{export_markdown, ExportOptions, Format, ImageSource, PdfEngine, Template};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn sample() -> String {
    std::fs::read_to_string(fixtures().join("sample.md")).unwrap()
}

fn images() -> ImageSource {
    ImageSource {
        base_dir: Some(fixtures()),
        ..Default::default()
    }
}

fn opts(format: Format) -> ExportOptions {
    ExportOptions {
        format,
        author: Some("M&J Group".into()),
        date: Some("2026-10-26".into()),
        ..Default::default()
    }
}

fn engine() -> &'static PdfEngine {
    PdfEngine::shared()
}

fn unzip(bytes: &[u8], name: &str) -> String {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut out = String::new();
    zip.by_name(name).unwrap().read_to_string(&mut out).unwrap();
    out
}

#[test]
fn docx_has_styles_toc_numbering_footnotes_and_images() {
    let bytes = export_markdown(&sample(), &opts(Format::Docx), &images(), engine()).unwrap();
    assert!(bytes.starts_with(b"PK"));
    let doc = unzip(&bytes, "word/document.xml");
    assert!(doc.contains("w:val=\"Heading1\""), "heading styles");
    assert!(doc.contains("w:val=\"Heading2\""));
    assert!(doc.contains("TOC \\o &quot;1-3&quot; \\h"), "TOC field");
    assert!(
        doc.contains(">1  <") || doc.contains(">1  </w:t>"),
        "section numbers"
    );
    assert!(doc.contains("Offline sync design"));
    assert!(doc.contains("离线优先"));
    assert!(doc.contains("w:footnoteReference"));
    assert!(
        doc.contains("<wp:extent cx=\"3048000\" cy=\"1143000\""),
        "image keeps its size"
    );
    let footnotes = unzip(&bytes, "word/footnotes.xml");
    assert!(footnotes.contains("Footnotes become real footnotes"));
    let styles = unzip(&bytes, "word/styles.xml");
    assert!(styles.contains("w:outlineLvl w:val=\"0\""));
}

#[test]
fn pdf_has_title_bookmarks_and_pages() {
    let bytes = export_markdown(&sample(), &opts(Format::Pdf), &images(), engine()).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/Outlines"), "bookmarks from headings");
    assert!(
        text.contains("Offline sync design"),
        "document title metadata"
    );
    assert!(bytes.len() > 10_000);
}

#[test]
fn pdf_without_toc_numbers_or_bookmarks_and_with_cover() {
    let o = ExportOptions {
        toc: false,
        number_headings: false,
        bookmarks: false,
        cover: true,
        template: Template::Business,
        ..opts(Format::Pdf)
    };
    let bytes = export_markdown(&sample(), &o, &images(), engine()).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}

#[test]
fn every_template_and_format_exports() {
    for template in [
        Template::Clean,
        Template::Academic,
        Template::Business,
        Template::Github,
    ] {
        for format in [Format::Pdf, Format::Docx, Format::Html] {
            let o = ExportOptions {
                template,
                ..opts(format)
            };
            let out = export_markdown(&sample(), &o, &images(), engine());
            assert!(out.is_ok(), "{template:?} {format:?}: {:?}", out.err());
        }
    }
}

#[test]
fn bad_formulas_and_missing_images_do_not_fail_the_export() {
    let md = "# T\n\n$\\unknown{x} & y$ and $$\\frac{1}{$$\n\n![gone](missing.png) ![remote](https://x.test/a.png)\n";
    for format in [Format::Pdf, Format::Docx, Format::Html] {
        assert!(
            export_markdown(md, &opts(format), &images(), engine()).is_ok(),
            "{format:?}"
        );
    }
}

#[test]
fn hostile_text_is_escaped() {
    let md = "# Hi \"#panic()\n\nText with #set and $ and \\ and ] [ } { <script>alert(1)</script> <img src=x onerror=alert(1)>\n\n[x](javascript:alert(1))\n";
    let html =
        String::from_utf8(export_markdown(md, &opts(Format::Html), &images(), engine()).unwrap())
            .unwrap();
    assert!(!html.contains("<script>alert"));
    assert!(!html.contains("onerror=alert"));
    assert!(!html.contains("href=\"javascript:"));
    assert!(export_markdown(md, &opts(Format::Pdf), &images(), engine()).is_ok());
    assert!(export_markdown(md, &opts(Format::Docx), &images(), engine()).is_ok());
}

#[test]
fn html_is_standalone_with_toc_and_inline_images() {
    let html = String::from_utf8(
        export_markdown(&sample(), &opts(Format::Html), &images(), engine()).unwrap(),
    )
    .unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<nav class=\"toc\">"));
    assert!(html.contains("href=\"#goals\""));
    assert!(html.contains("<h2 id=\"goals\"><span class=\"num\">1</span>Goals</h2>"));
    assert!(html.contains("src=\"data:image/png;base64,"));
    assert!(html.contains("<input type=\"checkbox\" disabled checked>"));
}

#[test]
fn chinese_labels() {
    let o = ExportOptions {
        lang: "zh-CN".into(),
        ..opts(Format::Html)
    };
    let html =
        String::from_utf8(export_markdown(&sample(), &o, &images(), engine()).unwrap()).unwrap();
    assert!(html.contains("<strong>目录</strong>"));
}
