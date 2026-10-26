# JmdEditor v1 plan

JmdEditor is an offline-first Markdown editor by M&J Group for desktop (Windows, macOS, Linux), mobile (iOS, Android) and the WeChat mini-program.

## Acceptance path

Open or import a Markdown note → edit it → export it to Word and PDF with a real table of contents.

| Feature | Accepted when |
| --- | --- |
| Open / save `.md` | A file opened from disk saves back byte-for-byte except for the user's edits; saves are atomic |
| Import from URL | GitHub / Gitee / GitLab blob links, gist links, raw `.md` links and article pages become a new note; images can be saved locally |
| Export Word | `.docx` opens in Word / LibreOffice with Heading 1–6 styles, numbered headings, tables, code blocks, images and an updatable TOC field |
| Export PDF | PDF has a TOC with real page numbers, bookmarks from headings, embedded fonts (Latin + CJK) and clickable links |
| Export HTML | One self-contained file |
| Mini-program | Reads a note from a link or chat file, exports Word / PDF through the export API and sends the file to a chat |

## Architecture

```
apps/desktop      Vue 3 + Milkdown UI (shared by desktop and mobile)
  src-tauri/      Tauri 2 shell: file access, IPC commands, dialogs
crates/jmd-core   Markdown → document model (comrak), outline, front matter
crates/jmd-import URL normalisation, fetch, HTML → Markdown, image download
crates/jmd-export DOCX (docx-rs), PDF (Typst), HTML renderers over the model
crates/jmd-server Axum export / fetch API for the mini-program
apps/miniprogram  native WeChat mini-program: reader + import + export (D7)
```

All exporters read the same `jmd_core::Document`, so preview, Word and PDF keep the same order, numbering and TOC.

## Phases

| Phase | Checkable result |
| --- | --- |
| 0 Plan | This file, DECISIONS, PROGRESS, checkpoint script on the remote branch |
| 1 Core | `jmd-core` parses GFM into the model; unit tests green |
| 2 Import | `jmd-import` normalises links and converts HTML; tests with fixtures, no network in tests |
| 3 Export | `jmd-export` writes DOCX, PDF and HTML; tests check the files' structure |
| 4 App | Tauri app builds; UI matches the approved mockup; open / save / import / export work through IPC |
| 5 Server | `jmd-server` serves `/api/import` and `/api/export` with limits; tests via `tower::ServiceExt` |
| 6 Mini-program | WeChat mini-program (native, D7): read, import, export, send to chat |
| 7 Delivery | CI for crates, frontend and Tauri builds; screenshots; README |

## Risks

- Mini-program editing is reader-only in v1; WeChat cannot host Milkdown.
- Bundled CJK fonts add about 50 MB; release builds run `scripts/fetch-fonts.sh`, development builds use system fonts (D8).
- iOS / Android release signing and Apple notarisation are not configured; CI produces unsigned artifacts.
