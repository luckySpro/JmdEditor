// The rich editor does not understand YAML front matter, so it is split off
// before the note reaches Milkdown and put back on every change.

const FRONT_MATTER = /^---[ \t]*\r?\n[\s\S]*?\r?\n---[ \t]*(\r?\n|$)/

export function splitFrontMatter(markdown: string): { frontMatter: string; body: string } {
  const m = markdown.match(FRONT_MATTER)
  if (!m) return { frontMatter: '', body: markdown }
  return { frontMatter: m[0], body: markdown.slice(m[0].length).replace(/^\r?\n/, '') }
}

export function joinFrontMatter(frontMatter: string, body: string): string {
  if (!frontMatter) return body
  const fm = frontMatter.endsWith('\n') ? frontMatter : `${frontMatter}\n`
  return `${fm}\n${body}`
}
