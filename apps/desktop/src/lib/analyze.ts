// A browser-side version of jmd-core's outline and stats, used by the web
// build so the outline works without a server round trip.

import type { Analysis, OutlineEntry } from '../types'

export function slugify(text: string): string {
  return text
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, '')
    .replace(/\s+/g, '-')
}

const CJK = /[぀-ヿ㐀-䶿一-鿿豈-﫿가-힯]/

function stripFrontMatter(md: string): string {
  const m = md.match(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/)
  return m ? md.slice(m[0].length) : md
}

function inlineText(s: string): string {
  return s
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[*_`~]/g, '')
    .replace(/\s+#+\s*$/, '')
    .trim()
}

function frontMatterTitle(md: string): string | null {
  const m = md.match(/^---\r?\n([\s\S]*?)\r?\n---/)
  const t = m?.[1].match(/^title:\s*(.+)$/m)?.[1].trim()
  return t ? t.replace(/^(['"])(.*)\1$/, '$2') : null
}

export function analyze(markdown: string): Analysis {
  const body = stripFrontMatter(markdown)
  const lines = body.split(/\r?\n/)
  const heads: { level: number; text: string }[] = []
  let fence: string | null = null
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    const f = line.match(/^\s{0,3}(`{3,}|~{3,})/)
    if (f) {
      if (fence === null) fence = f[1][0]
      else if (f[1][0] === fence) fence = null
      continue
    }
    if (fence !== null) continue
    const h = line.match(/^\s{0,3}(#{1,6})\s+(.*)$/)
    if (h) {
      heads.push({ level: h[1].length, text: inlineText(h[2]) })
      continue
    }
    const next = lines[i + 1]
    if (line.trim() && next !== undefined && /^\s{0,3}(=+|-+)\s*$/.test(next) && !/^\s*[-*+>|]/.test(line)) {
      heads.push({ level: next.trim().startsWith('=') ? 1 : 2, text: inlineText(line) })
      i++
    }
  }

  // Same numbering rule as jmd-core: a lone H1 is the title and gets no number.
  const h1s = heads.filter((h) => h.level === 1).length
  const titleIsH1 = h1s === 1 && heads[0]?.level === 1
  const numbered = titleIsH1 ? heads.slice(1) : heads
  const top = numbered.length ? Math.min(...numbered.map((h) => h.level)) : 1
  const counters = [0, 0, 0, 0, 0, 0]
  const seen = new Map<string, number>()
  const outline: OutlineEntry[] = heads.map((h, idx) => {
    let id = slugify(h.text) || 'section'
    const n = seen.get(id) ?? 0
    seen.set(id, n + 1)
    if (n > 0) id = `${id}-${n}`
    let number = ''
    if (!(titleIsH1 && idx === 0)) {
      const depth = Math.max(0, h.level - top)
      counters[depth]++
      for (let d = depth + 1; d < 6; d++) counters[d] = 0
      number = counters.slice(0, depth + 1).map((c) => Math.max(c, 1)).join('.')
    }
    return { level: h.level, text: h.text, id, number }
  })

  // Same counting as jmd-core::stats: each CJK character is a word.
  let cjk = 0
  let latin = 0
  let characters = 0
  let inWord = false
  for (const ch of body) {
    if (!/\s/.test(ch)) characters++
    if (CJK.test(ch)) {
      cjk++
      inWord = false
    } else if (/[\p{L}\p{N}]/u.test(ch)) {
      if (!inWord) latin++
      inWord = true
    } else inWord = false
  }
  const minutes = Math.max(1, Math.ceil(latin / 220 + cjk / 400))
  return {
    title: frontMatterTitle(markdown) ?? heads.find((h) => h.level === 1)?.text ?? null,
    outline,
    stats: { words: latin + cjk, characters, readingMinutes: minutes },
  }
}

/** Zero-based line numbers of the headings, in outline order. */
export function headingLines(markdown: string): number[] {
  const lines = markdown.split(/\r?\n/)
  let start = 0
  const fm = markdown.match(/^---[ \t]*\r?\n[\s\S]*?\r?\n---[ \t]*(\r?\n|$)/)
  if (fm) start = fm[0].split(/\r?\n/).length - 1
  const out: number[] = []
  let fence: string | null = null
  for (let i = start; i < lines.length; i++) {
    const line = lines[i]
    const f = line.match(/^\s{0,3}(`{3,}|~{3,})/)
    if (f) {
      if (fence === null) fence = f[1][0]
      else if (f[1][0] === fence) fence = null
      continue
    }
    if (fence !== null) continue
    if (/^\s{0,3}#{1,6}\s+/.test(line)) {
      out.push(i)
      continue
    }
    const next = lines[i + 1]
    if (line.trim() && next !== undefined && /^\s{0,3}(=+|-+)\s*$/.test(next) && !/^\s*[-*+>|]/.test(line)) {
      out.push(i)
      i++
    }
  }
  return out
}
