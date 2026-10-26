import { describe, expect, it } from 'vitest'
import { applyFormat, type FormatCommand } from './format'
import { joinFrontMatter, splitFrontMatter } from './frontmatter'

function run(text: string, sel: [number, number], cmd: FormatCommand) {
  const e = applyFormat(text, sel[0], sel[1], cmd)
  const out = text.slice(0, e.from) + e.insert + text.slice(e.to)
  return { out, sel: out.slice(e.anchor, e.head) }
}

describe('applyFormat', () => {
  it('wraps and unwraps inline marks', () => {
    const a = run('say hello', [4, 9], 'bold')
    expect(a.out).toBe('say **hello**')
    expect(a.sel).toBe('hello')
    expect(run('say **hello**', [6, 11], 'bold').out).toBe('say hello')
  })

  it('inserts a placeholder when nothing is selected', () => {
    const a = run('x ', [2, 2], 'italic')
    expect(a.out).toBe('x *italic text*')
    expect(a.sel).toBe('italic text')
  })

  it('turns lines into lists and back', () => {
    expect(run('one\ntwo', [0, 7], 'ordered').out).toBe('1. one\n2. two')
    expect(run('- one\n- two', [0, 11], 'bullet').out).toBe('one\ntwo')
    expect(run('- one', [2, 2], 'task').out).toBe('- [ ] one')
  })

  it('replaces heading levels', () => {
    expect(run('## Title', [3, 3], 'h3').out).toBe('### Title')
    expect(run('## Title', [3, 3], 'paragraph').out).toBe('Title')
  })

  it('inserts blocks on their own lines', () => {
    expect(run('text', [4, 4], 'rule').out).toBe('text\n\n---\n')
    expect(run('a\n\n', [3, 3], 'codeBlock').out).toBe('a\n\n```\n\n```\n')
  })

  it('selects the URL of a new link', () => {
    const a = run('see docs', [4, 8], 'link')
    expect(a.out).toBe('see [docs](https://)')
    expect(a.sel).toBe('https://')
  })
})

describe('front matter', () => {
  it('splits and joins', () => {
    const md = '---\ntitle: A\n---\n\n# A\n'
    const { frontMatter, body } = splitFrontMatter(md)
    expect(frontMatter).toBe('---\ntitle: A\n---\n')
    expect(body).toBe('# A\n')
    expect(joinFrontMatter(frontMatter, body)).toBe(md)
    expect(splitFrontMatter('# none').frontMatter).toBe('')
  })
})
