// Shapes shared with the Rust side (serde camelCase).

export type Format = 'pdf' | 'docx' | 'html'
export type Template = 'clean' | 'academic' | 'business' | 'github'
export type Paper = 'a4' | 'letter'
export type Margins = 'narrow' | 'normal' | 'wide'
export type EditorMode = 'rich' | 'split' | 'source' | 'read'
export type Lang = 'en' | 'zh-CN' | 'zh-TW' | 'ja'

export interface ExportOptions {
  format: Format
  template: Template
  paper: Paper
  margins: Margins
  toc: boolean
  tocDepth: number
  numberHeadings: boolean
  pageNumbers: boolean
  cover: boolean
  title: string | null
  author: string | null
  date: string | null
  bookmarks: boolean
  pdfA: boolean
  codeAsTable: boolean
  inlineImages: boolean
  lang: string
}

export interface Settings {
  language: 'system' | Lang
  theme: 'system' | 'light' | 'dark'
  accent: string
  editorMode: EditorMode
  editorFontSize: number
  workspace: string | null
  recentFiles: string[]
  recentUrls: string[]
  export: ExportOptions
  exportDir: string | null
  openAfterExport: boolean
  importSaveImages: boolean
  importRecordSource: boolean
  autosave: boolean
}

export interface LoadedSettings {
  settings: Settings
  problem: string | null
  path: string
}

export interface OpenedFile {
  path: string
  name: string
  dir: string
  content: string
}

export interface FolderEntry {
  path: string
  name: string
  isDir: boolean
}

export interface OutlineEntry {
  level: number
  text: string
  id: string
  number: string
}

export interface Stats {
  words: number
  characters: number
  readingMinutes: number
}

export interface Analysis {
  title: string | null
  outline: OutlineEntry[]
  stats: Stats
}

export type SourceKind = 'gitHub' | 'gitee' | 'gitLab' | 'gist' | 'rawMarkdown' | 'webPage'

export interface Resolved {
  sourceUrl: string
  fetchUrl: string
  kind: SourceKind
  note: string
}

export interface ImportResult {
  title: string
  markdown: string
  suggestedPath: string | null
  suggestedName: string
  resolved: Resolved
  images: { saved: string[]; failed: string[] }
  bytes: number
}

export interface ExportResult {
  path: string
  bytes: number
  millis: number
}

export interface AppError {
  code: string
  message: string
}

export const DEFAULT_EXPORT: ExportOptions = {
  format: 'pdf',
  template: 'clean',
  paper: 'a4',
  margins: 'normal',
  toc: true,
  tocDepth: 3,
  numberHeadings: true,
  pageNumbers: true,
  cover: false,
  title: null,
  author: null,
  date: null,
  bookmarks: true,
  pdfA: false,
  codeAsTable: true,
  inlineImages: true,
  lang: 'en',
}

export const DEFAULT_SETTINGS: Settings = {
  language: 'system',
  theme: 'system',
  accent: '#2F5BD3',
  editorMode: 'split',
  editorFontSize: 15,
  workspace: null,
  recentFiles: [],
  recentUrls: [],
  export: { ...DEFAULT_EXPORT },
  exportDir: null,
  openAfterExport: true,
  importSaveImages: true,
  importRecordSource: true,
  autosave: true,
}

export function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e
}
