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

## D7 · Native WeChat mini-program instead of uni-app (2026-10-26)

Only WeChat is needed (D4), so the mini-program is written natively (WXML/WXSS/JS) with no build step and no uni-app toolchain. It is a reader: open from a link or a chat file, read, export PDF/Word through `jmd-server`, open or send the file. Pages render with `rich-text` from `/api/render` with `mini: true`, which inlines every style because `rich-text` ignores `<style>`.

## D8 · Fonts (2026-10-26)

The UI bundles Inter, Source Serif 4 and JetBrains Mono (`@fontsource`) so it looks the same offline. PDF export uses Typst's embedded fonts plus system fonts plus anything in `resources/fonts/` (desktop) or `JMD_FONT_DIRS` (server). `scripts/fetch-fonts.sh` downloads Noto Sans/Serif CJK into `resources/fonts/` before a release build, so Chinese and Japanese PDFs do not depend on the user's system fonts.

## D9 · Images in exports (2026-10-26)

Local images are read from the note's folder (no `..` escapes). Web images are downloaded at export time in Rust, in parallel, within the import limits (D6) and 40 MB in total; offline they are skipped and the alt text remains. The Read view and the server's `/api/render` keep web image links for the viewer to load.

## D10 · Required updates (2026-10-26)

User asked for forced upgrades. Every release carries `policy.json` (`version`, `minimum`, `notes`) and `policy.json.sig`, signed with the same minisign key as the Tauri updater packages; the public key is in `tauri.conf.json`. The desktop app checks 3 s after start and every 6 hours (official GitHub link, then a gh-proxy mirror; the signature makes the mirror safe).

- Only a verified policy can lock the editor; a failed check never does. The highest verified minimum is cached, so offline starts and stale mirrors cannot lower it.
- `current < minimum` → a dialog that cannot be closed (no close button, Escape or backdrop), shortcuts and file drops are ignored. It saves the note first (asking where if it is new), then downloads, verifies, installs and restarts. "Open download page" is always there for installs the updater cannot replace.
- `current < version` → a toast with "View", and a dialog that can be closed.
- The minimum comes from `apps/desktop/src-tauri/update-policy.json`, or from the Release workflow's `force` input, which sets it to the new version.
- Phones have no in-app updater; they show the download page. Versions released before D10 have no checker and need one manual install.
