// Raw HTML inside Markdown (README badges, `<div align="center">`, `<img width>`)
// is shown rendered in the rich editor instead of as tag text. It is sanitised
// first: no scripts, frames, forms, styles sheets or event handlers.
import DOMPurify from 'dompurify'

const FORBID_TAGS = ['style', 'form', 'input', 'button', 'textarea', 'select', 'iframe', 'object', 'embed', 'meta', 'link', 'base']

/** Sanitises an HTML fragment; `resolveImage` maps image sources (relative paths, local files). */
export function sanitizeHtml(html: string, resolveImage: (src: string) => string = (s) => s): string {
  const hook = (node: Element) => {
    if (node.tagName === 'IMG') {
      const src = node.getAttribute('src')
      if (src) node.setAttribute('src', resolveImage(src))
      node.setAttribute('loading', 'lazy')
    }
    if (node.tagName === 'A' && node.getAttribute('href')) node.setAttribute('rel', 'noopener noreferrer')
  }
  DOMPurify.addHook('afterSanitizeAttributes', hook)
  try {
    return DOMPurify.sanitize(html, { FORBID_TAGS, FORBID_ATTR: ['style'] })
  } finally {
    DOMPurify.removeHook('afterSanitizeAttributes')
  }
}

/** True when sanitised HTML shows nothing (a lone `<div align="center">` or `</a>`). */
export function rendersEmpty(html: string): boolean {
  return !/<(img|picture|video|audio|svg|hr|br)\b/i.test(html) && !html.replace(/<[^>]*>/g, '').trim()
}

interface MdNode {
  type: string
  value?: string
  children?: MdNode[]
}

/**
 * remark plugin: a paragraph made only of HTML tags (plus spaces between them),
 * such as `<a href><img></a> <a href><picture>…</picture></a>`, becomes one
 * html node, so the browser sees whole elements rather than one tag at a time.
 * The Markdown it serialises back to is unchanged.
 */
export function remarkMergeHtml() {
  const visit = (node: MdNode) => {
    const kids = node.children
    if (!kids) return
    if (
      node.type === 'paragraph' &&
      kids.length > 1 &&
      kids.some((k) => k.type === 'html') &&
      kids.every((k) => k.type === 'html' || (k.type === 'text' && !k.value?.trim()))
    ) {
      node.children = [{ type: 'html', value: kids.map((k) => k.value ?? '').join('') }]
      return
    }
    kids.forEach(visit)
  }
  return (tree: MdNode) => visit(tree)
}
