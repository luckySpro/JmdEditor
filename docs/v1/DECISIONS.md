# Decisions

Append only. To change a decision, add a new entry that supersedes it.

## D1 · Build our own app from proven parts (2026-10-26)

No open-source app combines a polished editor, Word + PDF export and mobile / mini-program reach. MarkText and Zettlr are Electron; Zettlr's export needs Pandoc. Markra is AGPL. We compose MIT / Apache parts instead.

## D2 · Tauri 2 + Vue 3 + Milkdown (2026-10-26)

Milkdown (MIT, ProseMirror + remark) gives WYSIWYG with lossless Markdown and a plugin model. Vditor was the fallback; rejected because it is harder to restyle to the approved mockup.

## D3 · Exports in Rust from one document model (2026-10-26)

comrak parses; docx-rs writes Word; Typst (as a library) writes PDF. No Pandoc, LibreOffice or WebView printing, so exports work offline on every platform and on the server.

## D4 · WeChat first, hosted export API (2026-10-26)

User chose WeChat as the first mini-program and approved a small hosted export API (`jmd-server`, Axum) reusing the same crates.

## D5 · Name and author (2026-10-26)

Product name JmdEditor; author M&J Group, 473276@qq.com. Bundle id `com.mjgroup.jmdeditor`.

## D6 · URL fetching happens in Rust (2026-10-26)

The WebView never fetches imported URLs. HTTPS/HTTP only, private and loopback addresses refused, 10 MB limit, 15 s timeout, 5 redirects.
