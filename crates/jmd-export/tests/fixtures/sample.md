---
title: Offline sync design
author: M&J Group
---

# Offline sync design

Notes stay **on the device** and sync when a [network](https://example.com) is available. 离线优先：所有字体随应用打包，中文与日文导出无需联网。

## Goals

- [x] Open any .md file without a network
- [x] Export to **Word** and **PDF** with a TOC
- [ ] Import a page from a URL

### Offline first

1. Save locally
2. Sync later
   - retry with *backoff*
   - ~~drop~~ keep conflicts

## Platforms

| Target | Shell | Export |
|:--|:-:|--:|
| Desktop | Tauri 2 | Rust core |
| iOS / Android | Tauri 2 | Rust core |
| Mini-program | uni-app | Export API |

![Sync chart](chart.png)

## Export pipeline

```rust
let doc = jmd_core::parse(&src)?;
jmd_export::docx(&doc, &opts)?;
```

> Export uses the same document model as the preview.

> [!WARNING]
> Word asks to update the table of contents on first open.

Inline math $E = mc^2$, `inline code`, and a footnote.[^1]

$$
a^2 + b^2 = c^2
$$

---

日本語の段落も正しく表示されます。

[^1]: Footnotes become real footnotes in Word and PDF.
