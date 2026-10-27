import { describe, expect, it } from 'vitest'
import { analyze, slugify } from './analyze'
import { resolveRelative, withExtension } from './paths'

describe('analyze', () => {
  it('numbers headings like the exporter', () => {
    const a = analyze('# Title\n\n## Goals\n\n### Offline\n\n## Platforms\n\n```\n# not a heading\n```\n')
    expect(a.title).toBe('Title')
    expect(a.outline.map((o) => o.number)).toEqual(['', '1', '1.1', '2'])
  })

  it('counts CJK characters as words', () => {
    const a = analyze('离线优先 offline first')
    expect(a.stats.words).toBe(6)
  })

  it('prefers the front matter title', () => {
    expect(analyze('---\ntitle: "Plan"\n---\n# Other\n').title).toBe('Plan')
  })

  it('makes unique anchors', () => {
    const a = analyze('## A\n## A\n')
    expect(a.outline.map((o) => o.id)).toEqual(['a', 'a-1'])
    expect(slugify('Hello, World 你好')).toBe('hello-world-你好')
  })
})

describe('paths', () => {
  it('resolves relative image links', () => {
    expect(resolveRelative('/notes/a', './img/../pic.png')).toBe('/notes/a/pic.png')
    expect(resolveRelative('C:\\notes', 'assets/x.png')).toBe('C:\\notes\\assets\\x.png')
  })
  it('swaps extensions', () => {
    expect(withExtension('Design-Spec.md', 'pdf')).toBe('Design-Spec.pdf')
  })
})

import { en } from '../i18n/en'
import { ja } from '../i18n/ja'
import { systemLang } from '../i18n'
import { zhCN } from '../i18n/zh-CN'
import { zhTW } from '../i18n/zh-TW'

describe('i18n', () => {
  it('has every key in every language', () => {
    const keys = Object.keys(en).sort()
    for (const cat of [zhCN, zhTW, ja]) expect(Object.keys(cat).sort()).toEqual(keys)
  })
  it('keeps placeholders', () => {
    for (const cat of [zhCN, zhTW, ja] as Record<string, string>[]) {
      for (const [k, v] of Object.entries(en)) {
        const want = (v.match(/\{\w+\}/g) ?? []).sort()
        expect((cat[k].match(/\{\w+\}/g) ?? []).sort(), k).toEqual(want)
      }
    }
  })
  it('maps system languages', () => {
    expect(systemLang(['zh-Hant-TW'])).toBe('zh-TW')
    expect(systemLang(['zh-CN'])).toBe('zh-CN')
    expect(systemLang(['ja-JP'])).toBe('ja')
    expect(systemLang(['fr-FR'])).toBe('en')
  })
})

import { headingLines } from './analyze'
describe('headingLines', () => {
  it('skips front matter and code', () => {
    expect(headingLines('---\na: 1\n---\n# T\n```\n# no\n```\nSub\n---\n')).toEqual([3, 7])
  })
})
