// Formatting commands for the Markdown source editor, as plain text edits so
// they can be tested without a DOM.

export type FormatCommand =
  | 'bold'
  | 'italic'
  | 'strike'
  | 'code'
  | 'bullet'
  | 'ordered'
  | 'task'
  | 'quote'
  | 'link'
  | 'table'
  | 'math'
  | 'codeBlock'
  | 'rule'
  | 'paragraph'
  | 'h1'
  | 'h2'
  | 'h3'
  | 'h4'

export interface Edit {
  from: number
  to: number
  insert: string
  /** Selection after the edit, in the new text. */
  anchor: number
  head: number
}

const WRAP: Partial<Record<FormatCommand, [string, string]>> = {
  bold: ['**', 'bold text'],
  italic: ['*', 'italic text'],
  strike: ['~~', 'struck text'],
  code: ['`', 'code'],
}

const PREFIX: Partial<Record<FormatCommand, (i: number) => string>> = {
  bullet: () => '- ',
  ordered: (i) => `${i + 1}. `,
  task: () => '- [ ] ',
  quote: () => '> ',
  paragraph: () => '',
  h1: () => '# ',
  h2: () => '## ',
  h3: () => '### ',
  h4: () => '#### ',
}

const BLOCKS: Partial<Record<FormatCommand, [string, number, number]>> = {
  // [text, selection start, selection end] relative to the inserted text
  table: ['| Column | Column |\n| --- | --- |\n| Cell | Cell |\n', 2, 8],
  math: ['$$\nE = mc^2\n$$\n', 3, 11],
  codeBlock: ['```\n\n```\n', 4, 4],
  rule: ['---\n', 4, 4],
}

/** Removes list, quote and heading markers from the start of a line. */
function stripPrefix(line: string): string {
  return line.replace(/^(\s*)(#{1,6}\s+|>\s?|[-*+]\s+\[[ xX]\]\s+|[-*+]\s+|\d+[.)]\s+)/, '$1')
}

export function applyFormat(text: string, from: number, to: number, cmd: FormatCommand): Edit {
  const wrap = WRAP[cmd]
  if (wrap) {
    const [mark, placeholder] = wrap
    const selected = text.slice(from, to)
    // Toggle off when the selection is already wrapped.
    if (
      text.slice(from - mark.length, from) === mark &&
      text.slice(to, to + mark.length) === mark &&
      !(mark === '*' && text.slice(from - 2, from) === '**')
    ) {
      return { from: from - mark.length, to: to + mark.length, insert: selected, anchor: from - mark.length, head: to - mark.length }
    }
    const inner = selected || placeholder
    return { from, to, insert: mark + inner + mark, anchor: from + mark.length, head: from + mark.length + inner.length }
  }

  const prefix = PREFIX[cmd]
  if (prefix) {
    const lineStart = text.lastIndexOf('\n', from - 1) + 1
    let lineEnd = text.indexOf('\n', Math.max(to - (to > from && text[to - 1] === '\n' ? 1 : 0), from))
    if (lineEnd === -1) lineEnd = text.length
    const lines = text.slice(lineStart, lineEnd).split('\n')
    const already = cmd !== 'paragraph' && lines.every((l) => !l.trim() || l.trimStart().startsWith(prefix(0).trimEnd()) && (cmd !== 'bullet' || !/\[[ xX]\]/.test(l)))
    let n = 0
    const out = lines.map((l) => {
      const bare = stripPrefix(l)
      if (!l.trim()) return l
      if (already) return bare
      return prefix(n++) + bare.trimStart()
    })
    const insert = out.join('\n')
    return { from: lineStart, to: lineEnd, insert, anchor: lineStart + insert.length, head: lineStart + insert.length }
  }

  if (cmd === 'link') {
    const label = text.slice(from, to) || 'link text'
    const insert = `[${label}](https://)`
    const urlStart = from + label.length + 3
    return { from, to, insert, anchor: urlStart, head: urlStart + 8 }
  }

  const block = BLOCKS[cmd]
  if (block) {
    const [body, a, h] = block
    const lineStart = text.lastIndexOf('\n', from - 1) + 1
    const atLineStart = text.slice(lineStart, from).trim() === ''
    const before = atLineStart ? (lineStart > 0 && text[lineStart - 2] !== '\n' ? '\n' : '') : '\n\n'
    const insert = before + body
    const base = (atLineStart ? lineStart : to) + before.length
    return { from: atLineStart ? lineStart : to, to: atLineStart ? Math.max(to, from) : to, insert, anchor: base + a, head: base + h }
  }

  return { from, to, insert: text.slice(from, to), anchor: from, head: to }
}

/** Markdown snippet the rich editor inserts for commands it has no node command for. */
export function snippet(cmd: FormatCommand): string | null {
  switch (cmd) {
    case 'task':
      return '- [ ] '
    case 'math':
      return '$$\nE = mc^2\n$$'
    case 'link':
      return '[link text](https://)'
    default:
      return null
  }
}
