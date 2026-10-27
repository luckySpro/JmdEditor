// Small path helpers that work for both `/` and `\` separators.

export function joinPath(dir: string, name: string): string {
  if (!dir) return name
  const sep = dir.includes('\\') && !dir.includes('/') ? '\\' : '/'
  return dir.endsWith('/') || dir.endsWith('\\') ? dir + name : dir + sep + name
}

export function baseName(path: string): string {
  const parts = path.split(/[\\/]/)
  return parts[parts.length - 1] ?? path
}

export function dirName(path: string): string {
  const i = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return i > 0 ? path.slice(0, i) : ''
}

export function stem(name: string): string {
  const i = name.lastIndexOf('.')
  return i > 0 ? name.slice(0, i) : name
}

export function withExtension(name: string, ext: string): string {
  return `${stem(name)}.${ext}`
}

/** True for links that point outside the note: web, data and blob URLs. */
export function isRemote(src: string): boolean {
  return /^(https?:|data:|blob:|asset:|mailto:)/i.test(src) || src.startsWith('//')
}

/** Resolves `./a/../b.png` against a folder; keeps absolute paths. */
export function resolveRelative(baseDir: string, src: string): string {
  const clean = decodeURI(src.split('#')[0].split('?')[0])
  if (/^([a-zA-Z]:[\\/]|\/)/.test(clean)) return clean
  const sep = baseDir.includes('\\') && !baseDir.includes('/') ? '\\' : '/'
  const out = baseDir.split(/[\\/]/)
  for (const part of clean.split(/[\\/]/)) {
    if (part === '' || part === '.') continue
    if (part === '..') out.pop()
    else out.push(part)
  }
  return out.join(sep)
}
