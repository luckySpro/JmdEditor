// App state shared by every screen. Components read `state` and call the
// actions below; nothing else talks to the platform layer for documents.

import { computed, reactive, watch } from 'vue'

import { errorText, setLanguage, t } from './i18n'
import { baseName, joinPath } from './lib/paths'
import { platform } from './platform'
import type { AppInfo } from './platform/types'
import type { FormatCommand } from './lib/format'
import {
  DEFAULT_SETTINGS,
  type Analysis,
  type EditorMode,
  type FolderEntry,
  type OpenedFile,
  type Settings,
} from './types'

export interface EditorHandle {
  command(cmd: FormatCommand): void
  insertMarkdown(markdown: string): void
  focus(): void
  /** Scrolls to the nth heading of the outline. */
  revealHeading(index: number, id: string): void
}

export interface Toast {
  id: number
  text: string
  kind: 'info' | 'error' | 'success'
  action?: { label: string; run: () => void }
}

export interface RecentExport {
  name: string
  path: string
  at: number
}

export type Dialog = 'import' | 'export' | 'settings' | 'sheet' | null

export const state = reactive({
  ready: false,
  info: null as AppInfo | null,
  settings: structuredClone(DEFAULT_SETTINGS) as Settings,
  settingsProblem: null as string | null,
  settingsPath: '',
  doc: {
    path: null as string | null,
    name: '',
    dir: null as string | null,
    content: '',
    savedContent: '',
    /** Bumped when the content is replaced from outside the editors (open, import). */
    revision: 0,
  },
  analysis: { title: null, outline: [], stats: { words: 0, characters: 0, readingMinutes: 1 } } as Analysis,
  cursor: { line: 1, col: 1 },
  mode: 'split' as EditorMode,
  folder: [] as FolderEntry[],
  imported: [] as { name: string; path: string | null; url: string }[],
  recentExports: [] as RecentExport[],
  saving: false,
  dialog: null as Dialog,
  /** Link dropped on the editor, handed to the import dialog. */
  importPrefill: '',
  sidebarTab: 'files' as 'files' | 'outline',
  sidebarOpen: true,
  toasts: [] as Toast[],
  editor: null as EditorHandle | null,
  systemDark: window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false,
})

export const dirty = computed(() => state.doc.content !== state.doc.savedContent)
export const isDark = computed(() =>
  state.settings.theme === 'system' ? state.systemDark : state.settings.theme === 'dark',
)
export const docName = computed(() => state.doc.name || t('app.untitled'))

let toastId = 0
export function toast(text: string, kind: Toast['kind'] = 'info', action?: Toast['action']) {
  const id = ++toastId
  state.toasts.push({ id, text, kind, action })
  setTimeout(() => dismissToast(id), kind === 'error' ? 9000 : 5000)
}

export function dismissToast(id: number) {
  const i = state.toasts.findIndex((x) => x.id === id)
  if (i >= 0) state.toasts.splice(i, 1)
}

export function reportError(e: unknown) {
  console.error(e)
  toast(errorText(e), 'error')
}

// ---- settings -------------------------------------------------------------

let saveSettingsTimer: ReturnType<typeof setTimeout> | undefined

export function updateSettings(patch: Partial<Settings>) {
  Object.assign(state.settings, patch)
  clearTimeout(saveSettingsTimer)
  saveSettingsTimer = setTimeout(async () => {
    try {
      const saved = await platform.saveSettings(JSON.parse(JSON.stringify(state.settings)))
      // Keep normalised values (for example a clamped font size).
      Object.assign(state.settings, saved)
    } catch (e) {
      reportError(e)
    }
  }, 300)
}

export async function resetSettings() {
  try {
    state.settings = await platform.resetSettings()
    state.settingsProblem = null
    applyAppearance()
  } catch (e) {
    reportError(e)
  }
}

export function applyAppearance() {
  setLanguage(state.settings.language)
  const root = document.documentElement
  root.dataset.theme = isDark.value ? 'dark' : 'light'
  root.style.setProperty('--accent', state.settings.accent)
  root.style.setProperty('--editor-font-size', `${state.settings.editorFontSize}px`)
}

function remember(list: string[], item: string) {
  const next = [item, ...list.filter((x) => x !== item)]
  return next.slice(0, 12)
}

// ---- documents -------------------------------------------------------------

let analyzeTimer: ReturnType<typeof setTimeout> | undefined
let autosaveTimer: ReturnType<typeof setTimeout> | undefined

function scheduleAnalysis() {
  clearTimeout(analyzeTimer)
  analyzeTimer = setTimeout(async () => {
    try {
      state.analysis = await platform.analyze(state.doc.content)
    } catch (e) {
      console.warn('analysis failed', e)
    }
  }, 150)
}

/** Called by the editors on every change. */
export function setContent(markdown: string) {
  if (markdown === state.doc.content) return
  state.doc.content = markdown
  scheduleAnalysis()
  if (state.settings.autosave && state.doc.path && platform.kind === 'tauri') {
    clearTimeout(autosaveTimer)
    autosaveTimer = setTimeout(() => {
      if (dirty.value && state.doc.path) void save()
    }, 1500)
  }
}

function loadDocument(file: { path: string | null; name: string; dir: string | null; content: string }, saved = true) {
  clearTimeout(autosaveTimer)
  state.doc.path = file.path || null
  state.doc.name = file.name
  state.doc.dir = file.dir || null
  state.doc.content = file.content
  state.doc.savedContent = saved ? file.content : ''
  state.doc.revision++
  scheduleAnalysis()
}

async function confirmDiscard(): Promise<boolean> {
  if (!dirty.value) return true
  // A saved note with autosave on is written before switching.
  if (state.doc.path && state.settings.autosave && platform.kind === 'tauri') {
    return save()
  }
  return platform.confirm(t('confirm.discard', { name: docName.value }), t('confirm.title'))
}

export async function newNote() {
  if (!(await confirmDiscard())) return
  loadDocument({ path: null, name: '', dir: state.settings.workspace, content: '' })
  state.editor?.focus()
}

function opened(file: OpenedFile) {
  loadDocument(file)
  if (file.path) {
    updateSettings({ recentFiles: remember(state.settings.recentFiles, file.path) })
  }
}

export async function openPath(path: string) {
  if (path === state.doc.path) return
  if (!(await confirmDiscard())) return
  try {
    opened(await platform.openFile(path))
  } catch (e) {
    reportError(e)
    // Forget files that no longer exist.
    updateSettings({ recentFiles: state.settings.recentFiles.filter((p) => p !== path) })
  }
}

export async function openPicker() {
  if (!(await confirmDiscard())) return
  try {
    const file = await platform.pickFile()
    if (file) opened(file)
  } catch (e) {
    reportError(e)
  }
}

export async function openFolder(path?: string) {
  try {
    const dir = path ?? (await platform.pickFolder())
    if (!dir) return
    state.folder = await platform.listFolder(dir)
    if (dir !== state.settings.workspace) updateSettings({ workspace: dir })
  } catch (e) {
    reportError(e)
    if (path) updateSettings({ workspace: null })
  }
}

export async function refreshFolder() {
  if (state.settings.workspace && platform.folders) {
    try {
      state.folder = await platform.listFolder(state.settings.workspace)
    } catch {
      /* the folder may have been removed; keep the old list */
    }
  }
}

function suggestedName() {
  if (state.doc.name) return state.doc.name
  const title = state.analysis.title?.replace(/[\\/:*?"<>|#]/g, '').trim().slice(0, 60)
  return `${title || 'Untitled'}.md`
}

/** Saves the note; asks for a place when it has none. Returns false when cancelled or failed. */
export async function save(asNew = false): Promise<boolean> {
  if (state.saving) return false
  state.saving = true
  const content = state.doc.content
  try {
    const target = asNew ? null : state.doc.path
    const saved = await platform.saveFile(target, content, suggestedName(), state.doc.dir ?? state.settings.workspace)
    if (!saved) return false
    state.doc.savedContent = content
    if (saved.path) {
      const isNew = saved.path !== state.doc.path
      state.doc.path = saved.path
      state.doc.name = saved.name
      state.doc.dir = saved.dir
      if (isNew) {
        updateSettings({ recentFiles: remember(state.settings.recentFiles, saved.path) })
        void refreshFolder()
      }
    } else {
      state.doc.name = saved.name
    }
    return true
  } catch (e) {
    reportError(e)
    return false
  } finally {
    state.saving = false
  }
}

// ---- import -----------------------------------------------------------------

export async function applyImport(
  result: { title: string; markdown: string; suggestedPath: string | null; suggestedName: string; resolved: { sourceUrl: string } },
  where: 'new' | 'cursor',
) {
  updateSettings({ recentUrls: remember(state.settings.recentUrls, result.resolved.sourceUrl) })
  if (where === 'cursor' && state.editor) {
    state.editor.insertMarkdown(result.markdown)
    toast(t('import.done', { title: result.title || result.suggestedName }), 'success')
    return
  }
  if (!(await confirmDiscard())) return
  // Imported notes start unsaved; when a folder is open they are saved there right away
  // so the downloaded images and the note stay together.
  loadDocument(
    {
      path: null,
      name: result.suggestedName,
      dir: state.settings.workspace,
      content: result.markdown,
    },
    false,
  )
  if (result.suggestedPath && platform.kind === 'tauri') {
    try {
      const saved = await platform.saveFile(result.suggestedPath, result.markdown, result.suggestedName, null)
      if (saved) {
        state.doc.path = saved.path
        state.doc.name = saved.name
        state.doc.dir = saved.dir
        state.doc.savedContent = result.markdown
        updateSettings({ recentFiles: remember(state.settings.recentFiles, saved.path) })
        void refreshFolder()
      }
    } catch (e) {
      reportError(e)
    }
  }
  state.imported.unshift({ name: state.doc.name, path: state.doc.path, url: result.resolved.sourceUrl })
  state.imported.splice(6)
  toast(t('import.done', { title: result.title || state.doc.name }), 'success')
}

// ---- export -----------------------------------------------------------------

export function noteExportName(ext: string) {
  const base = suggestedName().replace(/\.(md|markdown|mdown|mkd|txt)$/i, '')
  return `${base}.${ext}`
}

export async function defaultExportPath(ext: string): Promise<string | null> {
  const dir = state.settings.exportDir ?? state.doc.dir ?? (await platform.defaultExportDir())
  return dir ? joinPath(dir, noteExportName(ext)) : null
}

export function rememberExport(path: string) {
  state.recentExports.unshift({ name: baseName(path), path, at: Date.now() })
  state.recentExports.splice(5)
}

// ---- start-up ---------------------------------------------------------------

export async function init() {
  try {
    const loaded = await platform.loadSettings()
    state.settings = { ...structuredClone(DEFAULT_SETTINGS), ...loaded.settings }
    state.settingsProblem = loaded.problem
    state.settingsPath = loaded.path
  } catch (e) {
    reportError(e)
  }
  state.mode = platform.mobile && state.settings.editorMode === 'split' ? 'rich' : state.settings.editorMode
  state.sidebarOpen = !platform.mobile && window.innerWidth > 900
  applyAppearance()
  platform.appInfo().then((i) => (state.info = i), console.warn)

  window.matchMedia?.('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    state.systemDark = e.matches
  })
  watch(
    () => [state.settings.theme, state.settings.accent, state.settings.language, state.settings.editorFontSize, state.systemDark],
    applyAppearance,
  )

  if (state.settings.workspace && platform.folders) await openFolder(state.settings.workspace)

  const initial = await platform.initialFile().catch(() => null)
  if (initial) {
    try {
      opened(await platform.openFile(initial))
    } catch (e) {
      reportError(e)
    }
  } else {
    loadDocument({ path: null, name: t('welcome.name'), dir: null, content: t('welcome.body') })
  }
  if (state.settingsProblem) toast(t('settings.problem', { problem: state.settingsProblem }), 'error')
  state.ready = true
}

export function setMode(mode: EditorMode) {
  state.mode = mode
  if (mode !== state.settings.editorMode && !platform.mobile) updateSettings({ editorMode: mode })
}

export function closeDialog() {
  state.dialog = null
  state.editor?.focus()
}
