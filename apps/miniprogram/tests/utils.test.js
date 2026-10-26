// Run with `pnpm --filter jmd-miniprogram test` (no WeChat runtime needed).
const test = require('node:test')
const assert = require('node:assert')

// api.js only touches `wx` inside functions, so it loads under Node.
const api = require('../utils/api')
const i18n = require('../utils/i18n')

test('reads UTF-8 file names from Content-Disposition', () => {
  const h = 'attachment; filename="____.docx"; filename*=UTF-8\'\'%E5%9B%A2%E9%98%9F%E6%89%8B%E5%86%8C.docx'
  assert.strictEqual(api.fileNameFromDisposition(h, 'x'), '团队手册.docx')
  assert.strictEqual(api.fileNameFromDisposition('attachment; filename="a.pdf"', 'x'), 'a.pdf')
  assert.strictEqual(api.fileNameFromDisposition('', 'x.pdf'), 'x.pdf')
})

test('finds a title for the file name', () => {
  assert.strictEqual(api.titleOf('---\ntitle: "Plan"\n---\n# Other'), 'Plan')
  assert.strictEqual(api.titleOf('```\n# not\n```\n# **Team** handbook\n'), 'Team handbook')
})

test('decodes UTF-8 error bodies', () => {
  const bytes = Buffer.from('{"code":"busy","message":"忙"}')
  const ab = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.length)
  assert.strictEqual(JSON.parse(api.decodeUtf8(ab)).message, '忙')
})

test('maps WeChat languages and has every text in every language', () => {
  assert.strictEqual(i18n.detectLang('zh_CN'), 'zh-CN')
  assert.strictEqual(i18n.detectLang('zh_HK'), 'zh-TW')
  assert.strictEqual(i18n.detectLang('ja'), 'ja')
  assert.strictEqual(i18n.detectLang('fr'), 'zh-CN')
  const keys = Object.keys(i18n.MESSAGES.en).sort()
  for (const lang of Object.keys(i18n.MESSAGES)) assert.deepStrictEqual(Object.keys(i18n.MESSAGES[lang]).sort(), keys, lang)
  assert.strictEqual(i18n.format('{name} ok', { name: 'a.pdf' }), 'a.pdf ok')
})

