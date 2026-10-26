import type {
  Analysis,
  ExportOptions,
  ExportResult,
  FolderEntry,
  ImportResult,
  LoadedSettings,
  OpenedFile,
  Resolved,
  Settings,
} from '../types'

export interface AppInfo {
  name: string
  version: string
  author: string
  email: string
  repository: string
  today: string
  platform: string
}

/** Result of checking the signed release policy (see src-tauri/src/update.rs). */
export type UpdateStatus =
  | { state: 'up_to_date'; current: string }
  | { state: 'optional'; current: string; version: string; notes: string }
  | { state: 'required'; current: string; version: string; minimum: string; notes: string }
  | { state: 'unknown'; current: string; reason: string }

export interface UpdateProgress {
  received?: number
  total?: number | null
  installing?: boolean
}

export interface SavedFile {
  path: string
  name: string
  dir: string
}

export interface ImportRequest {
  url: string
  saveImages: boolean
  recordSource: boolean
  /** Folder the note will live in; images are saved under it. */
  folder: string | null
}

export interface ExportRequest {
  markdown: string
  baseDir: string | null
  options: ExportOptions
  /** File name without folder, e.g. `Design-Spec.pdf`. */
  fileName: string
  /** Full target path chosen in the dialog (desktop only). */
  outPath: string | null
}

/**
 * Everything the UI needs from the host. The desktop and mobile apps use
 * Tauri commands; the browser build talks to jmd-server and downloads files.
 */
export interface Platform {
  kind: 'tauri' | 'web'
  /** Phones and tablets: bottom sheet instead of dialogs, no sidebar folders. */
  mobile: boolean
  /** Whether a local folder can be opened in the sidebar. */
  folders: boolean
  appInfo(): Promise<AppInfo>
  initialFile(): Promise<string | null>
  /** Files the system hands over later (double-click, "Open with"). Returns a stop function. */
  onOpenFile(handler: (path: string) => void): Promise<() => void>
  pickFile(): Promise<OpenedFile | null>
  pickFolder(): Promise<string | null>
  openFile(path: string): Promise<OpenedFile>
  listFolder(path: string): Promise<FolderEntry[]>
  /** Stores a pasted or dropped image; returns the link to put in the note. */
  saveAsset(noteDir: string | null, file: File): Promise<string>
  /** Saves to `path`, or asks where when `path` is null. Returns null when cancelled. */
  saveFile(path: string | null, content: string, suggestedName: string, dir: string | null): Promise<SavedFile | null>
  /** Asks where to save an export; desktop only. */
  pickExportPath(suggested: string, format: ExportOptions['format']): Promise<string | null>
  analyze(markdown: string): Promise<Analysis>
  resolveUrl(url: string): Promise<Resolved>
  importUrl(request: ImportRequest): Promise<ImportResult>
  exportDocument(request: ExportRequest): Promise<ExportResult>
  renderHtml(markdown: string, baseDir: string | null, options: ExportOptions): Promise<string>
  loadSettings(): Promise<LoadedSettings>
  saveSettings(settings: Settings): Promise<Settings>
  resetSettings(): Promise<Settings>
  defaultExportDir(): Promise<string | null>
  fontFamilies(): Promise<string[]>
  openPath(path: string): Promise<void>
  revealPath(path: string): Promise<void>
  openUrl(url: string): Promise<void>
  /** URL the webview can load for an image link found in a note. */
  imageUrl(src: string, baseDir: string | null): string
  confirm(message: string, title: string): Promise<boolean>
  /** Null where the host updates itself (the browser build). */
  checkUpdate(): Promise<UpdateStatus | null>
  /** Downloads, verifies and installs the newest release, then restarts. */
  installUpdate(onProgress: (p: UpdateProgress) => void): Promise<void>
}
