# Progress

| Time (UTC) | Phase | Done | Next |
| --- | --- | --- | --- |
| 2026-10-26 11:52 | 0 Plan | plan, decisions, progress log and checkpoint script | Rust workspace and jmd-core |
| 2026-10-26 11:55 | 1 Core | jmd-core parses GFM into the document model; 8 tests pass | jmd-import |
| 2026-10-26 12:00 | 2 Import | jmd-import: link resolving, safe fetch, HTML to Markdown, local images; 16 tests | jmd-export (DOCX, PDF, HTML) |
| 2026-10-26 12:22 | 3 Export | jmd-export: Word (styles, TOC field, footnotes, images), PDF via Typst (TOC, bookmarks, CJK, math), HTML; 12 tests | Tauri 2 + Vue 3 + Milkdown app |
| 2026-10-26 12:50 | 4 App | Tauri 2 shell with file, import, export and settings commands; Vue 3 UI with Rich (Milkdown), Split, Source (CodeMirror) and Read modes, import/export/settings dialogs, mobile sheet, 4 languages | jmd-server for the browser build and mini-program |
| 2026-10-26 13:11 | 5 Server | jmd-server (Axum): /api/resolve, /api/import, /api/render, /api/export with size, concurrency and per-client rate limits, optional static web UI; exports now include web images; README HTML images absolutized; title stays above TOC in HTML | WeChat mini-program |
