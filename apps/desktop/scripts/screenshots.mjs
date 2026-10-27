// Captures the browser and phone layouts for docs/screenshots.
//   cargo run -p jmd-server   (with JMD_STATIC=apps/desktop/dist)
//   pnpm --filter jmdeditor-desktop screenshots [http://127.0.0.1:8787]
// Desktop-window screenshots come from the real Tauri app; see docs/screenshots/README.md.
import { mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const { chromium } = require('playwright')

const base = process.argv[2] ?? 'http://127.0.0.1:8787'
const out = resolve(dirname(fileURLToPath(import.meta.url)), '../../../docs/screenshots')
mkdirSync(out, { recursive: true })

const browser = await chromium.launch()
const phone = await browser.newPage({
  viewport: { width: 390, height: 844 },
  deviceScaleFactor: 2,
  userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148',
})
await phone.goto(base)
await phone.waitForTimeout(1500)
await phone.screenshot({ path: `${out}/mobile-rich.png` })
await phone.getByRole('button', { name: 'Export & share' }).click()
await phone.waitForTimeout(500)
await phone.screenshot({ path: `${out}/mobile-share.png` })
await phone.keyboard.press('Escape')
await phone.mouse.click(195, 100)
await phone.getByRole('button', { name: 'Read' }).click()
await phone.waitForTimeout(1500)
await phone.screenshot({ path: `${out}/mobile-read.png` })
await browser.close()
console.log(`screenshots written to ${out}`)
