// Milkdown's image block keeps its size ratio in the alt text and the caption
// in the title (`![1.00](a.png "caption")`). Notes keep the usual
// `![alt](a.png)`, so standalone images are translated on the way in and out
// of the rich editor.

const FENCE = /^\s{0,3}(`{3,}|~{3,})/

function mapOutsideFences(markdown: string, fn: (line: string) => string): string {
  let fence: string | null = null
  return markdown
    .split('\n')
    .map((line) => {
      const f = line.match(FENCE)
      if (f) {
        if (fence === null) fence = f[1][0]
        else if (f[1][0] === fence) fence = null
        return line
      }
      return fence === null ? fn(line) : line
    })
    .join('\n')
}

const PLAIN_IMAGE = /^(\s*)!\[([^\]]*)\]\((<[^>]*>|[^\s)]+)\)\s*$/
const BLOCK_IMAGE = /^(\s*)!\[(\d+(?:\.\d+)?)\]\((<[^>]*>|[^\s)]+)(?:\s+"((?:[^"\\]|\\.)*)")?\)\s*$/

export function toRichImages(markdown: string): string {
  return mapOutsideFences(markdown, (line) => {
    const m = line.match(PLAIN_IMAGE)
    if (!m || !m[2]) return line
    const caption = m[2].replace(/\\/g, '\\\\').replace(/"/g, '\\"')
    return `${m[1]}![1.00](${m[3]} "${caption}")`
  })
}

export function fromRichImages(markdown: string): string {
  return mapOutsideFences(markdown, (line) => {
    const m = line.match(BLOCK_IMAGE)
    if (!m) return line
    const caption = (m[4] ?? '').replace(/\\(.)/g, '$1')
    return `${m[1]}![${caption.replace(/([[\]])/g, '\\$1')}](${m[3]})`
  })
}
