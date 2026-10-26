use jmd_core::{
    outline, parse, plain_text, set_front_matter, split_front_matter, stats, Block, Inline,
};

const SAMPLE: &str = "---
title: \"Offline sync design\"
author: M&J Group
---

# Offline sync design

Notes stay **on the device** and sync when a [network](https://example.com) is available.

## Goals

- [x] Open any .md file
- [ ] Import a page from a URL

## Platforms

| Target | Shell |
|:--|--:|
| Desktop | Tauri 2 |

```rust
let doc = jmd_core::parse(&src)?;
```

### Goals

> [!NOTE]
> Export uses the same model.

Math $E=mc^2$ and a note[^1].

[^1]: The footnote.

---

离线优先
";

#[test]
fn parses_front_matter_and_title() {
    let doc = parse(SAMPLE);
    assert_eq!(doc.meta("title"), Some("Offline sync design"));
    assert_eq!(doc.meta("author"), Some("M&J Group"));
    assert_eq!(doc.title().as_deref(), Some("Offline sync design"));
}

#[test]
fn headings_get_unique_ids() {
    let doc = parse(SAMPLE);
    let ids: Vec<_> = doc
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Heading { id, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        ids,
        ["offline-sync-design", "goals", "platforms", "goals-1"]
    );
}

#[test]
fn outline_numbers_sections_below_the_title() {
    let doc = parse(SAMPLE);
    let numbers: Vec<_> = outline(&doc)
        .into_iter()
        .map(|e| (e.text, e.number))
        .collect();
    assert_eq!(
        numbers,
        [
            ("Offline sync design".to_string(), "".to_string()),
            ("Goals".to_string(), "1".to_string()),
            ("Platforms".to_string(), "2".to_string()),
            ("Goals".to_string(), "2.1".to_string()),
        ]
    );
}

#[test]
fn parses_gfm_blocks() {
    let doc = parse(SAMPLE);
    let list = doc.blocks.iter().find_map(|b| match b {
        Block::List { items, .. } => Some(items),
        _ => None,
    });
    let tasks: Vec<_> = list.unwrap().iter().map(|i| i.task).collect();
    assert_eq!(tasks, [Some(true), Some(false)]);

    assert!(doc.blocks.iter().any(
        |b| matches!(b, Block::Table { header, rows, .. } if header.len() == 2 && rows.len() == 1)
    ));
    assert!(doc
        .blocks
        .iter()
        .any(|b| matches!(b, Block::CodeBlock { lang, .. } if lang == "rust")));
    assert!(doc
        .blocks
        .iter()
        .any(|b| matches!(b, Block::Alert { kind, .. } if kind == "note")));
    assert!(doc
        .blocks
        .iter()
        .any(|b| matches!(b, Block::FootnoteDefinition { .. })));
    assert!(doc.blocks.iter().any(|b| matches!(b, Block::Rule)));
}

#[test]
fn inline_text_runs_are_merged() {
    let doc = parse("a *b* c & d");
    match &doc.blocks[0] {
        Block::Paragraph { content } => {
            assert_eq!(plain_text(content), "a b c & d");
            assert!(matches!(content.last(), Some(Inline::Text { text }) if text == " c & d"));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn front_matter_round_trip() {
    let src = "---\ntitle: A\n---\n\nBody\n";
    let out = set_front_matter(src, &[("source", "https://x.test/a.md"), ("title", "B")]);
    assert_eq!(
        out,
        "---\ntitle: B\nsource: https://x.test/a.md\n---\n\nBody\n"
    );
    assert_eq!(
        split_front_matter("no front matter"),
        (None, "no front matter")
    );
    let added = set_front_matter("# Hi\n", &[("imported", "2026-10-26")]);
    assert_eq!(added, "---\nimported: 2026-10-26\n---\n\n# Hi\n");
}

#[test]
fn counts_cjk_and_latin_words() {
    let s = stats("Hello world 离线优先");
    assert_eq!(s.words, 6);
    assert_eq!(s.reading_minutes, 1);
}

#[test]
fn collects_image_urls() {
    let doc = parse("![a](img/a.png)\n\n> ![b](https://x.test/b.jpg)");
    assert_eq!(doc.image_urls(), ["img/a.png", "https://x.test/b.jpg"]);
}
