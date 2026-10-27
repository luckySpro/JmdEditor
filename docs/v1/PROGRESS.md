# Progress

| Time (UTC) | Phase | Done | Next |
| --- | --- | --- | --- |
| 2026-10-26 11:52 | 0 Plan | plan, decisions, progress log and checkpoint script | Rust workspace and jmd-core |
| 2026-10-26 11:55 | 1 Core | jmd-core parses GFM into the document model; 8 tests pass | jmd-import |
| 2026-10-26 12:00 | 2 Import | jmd-import: link resolving, safe fetch, HTML to Markdown, local images; 16 tests | jmd-export (DOCX, PDF, HTML) |
| 2026-10-26 12:22 | 3 Export | jmd-export: Word (styles, TOC field, footnotes, images), PDF via Typst (TOC, bookmarks, CJK, math), HTML; 12 tests | Tauri 2 + Vue 3 + Milkdown app |
| 2026-10-26 12:50 | 4 App | Tauri 2 shell with file, import, export and settings commands; Vue 3 UI with Rich (Milkdown), Split, Source (CodeMirror) and Read modes, import/export/settings dialogs, mobile sheet, 4 languages | jmd-server for the browser build and mini-program |
| 2026-10-26 13:11 | 5 Server | jmd-server (Axum): /api/resolve, /api/import, /api/render, /api/export with size, concurrency and per-client rate limits, optional static web UI; exports now include web images; README HTML images absolutized; title stays above TOC in HTML | WeChat mini-program |
| 2026-10-26 13:14 | 6 Mini-program | Native WeChat reader: open from link or chat file, rich-text reading via /api/render mini mode, PDF/Word export saved to user data, open and send to chat, 4 languages, node tests | CI, fonts script, screenshots, README, PR |
| 2026-10-26 13:26 | 7 Delivery | CI and release workflows, pinned Noto CJK download, jmd-server Dockerfile, README, LICENSE, screenshots from the real Tauri app and phone layout; rich editor keeps image alt text and '-' bullets | Pull request |
| 2026-10-26 14:10 | 8 Updates | Required updates: signed policy.json (minisign, same key as updater), cached highest minimum, non-dismissible dialog that saves then installs and restarts; Release workflow signs updater packages and policy, `force` input; mini-program About page with desktop downloads and restart-into-new-version. Tested 0.1.0 → 0.2.0 AppImage locally | Release v0.1.0, intro article |
| 2026-10-26 14:40 | 8 Updates | Opening .md files from the system: macOS open-file events, single instance (a second launch opens its file in the running window, tested on Linux), settings explain how to set the default app per OS; link-import hints translated | Next release |
| 2026-10-27 00:20 | 8 Updates | Raw HTML in notes (README badges, centred blocks, sponsor tables) renders sanitised in the rich editor and split preview instead of showing tags; releases attach jmd-server-linux-x64.tar.gz for BaoTa/systemd with start script and Chinese deploy guide | Next release |
| 2026-10-27 00:58 | 8 Updates | Version 0.2.0, released as a required update to test forced upgrades from 0.1.0 | Release v0.2.0 |
