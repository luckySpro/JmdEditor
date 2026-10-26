import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { ask, open, save } from '@tauri-apps/plugin-dialog'
import { openPath, openUrl, revealItemInDir } from '@tauri-apps/plugin-opener'

import { isRemote, joinPath, resolveRelative } from '../lib/paths'
import type { Analysis, ExportResult, FolderEntry, ImportResult, LoadedSettings, OpenedFile, Resolved, Settings } from '../types'
import type { AppInfo, Platform, SavedFile } from './types'

const MARKDOWN = [{ name: 'Markdown', extensions: ['md', 'markdown', 'mdown', 'mkd', 'txt'] }]
const FILTERS = {
  pdf: [{ name: 'PDF', extensions: ['pdf'] }],
  docx: [{ name: 'Word', extensions: ['docx'] }],
  html: [{ name: 'HTML', extensions: ['html'] }],
}

export function createTauriPlatform(): Platform {
  const mobile = /Android|iPhone|iPad|iPod/i.test(navigator.userAgent)
  return {
    kind: 'tauri',
    mobile,
    folders: !mobile,
    appInfo: () => invoke<AppInfo>('app_info'),
    initialFile: () => invoke<string | null>('initial_file'),
    async pickFile() {
      const path = await open({ multiple: false, directory: false, filters: MARKDOWN })
      return typeof path === 'string' ? invoke<OpenedFile>('open_file', { path }) : null
    },
    async pickFolder() {
      const path = await open({ multiple: false, directory: true })
      return typeof path === 'string' ? path : null
    },
    openFile: (path) => invoke<OpenedFile>('open_file', { path }),
    listFolder: (path) => invoke<FolderEntry[]>('list_folder', { path }),
    async saveAsset(noteDir, file) {
      if (!noteDir) throw { code: 'needsSave', message: '' }
      const data = Array.from(new Uint8Array(await file.arrayBuffer()))
      return invoke<string>('save_asset', { noteDir, fileName: file.name || 'image.png', data })
    },
    async saveFile(path, content, suggestedName, dir) {
      let target = path
      if (!target) {
        target = await save({ defaultPath: dir ? joinPath(dir, suggestedName) : suggestedName, filters: MARKDOWN })
        if (!target) return null
      }
      return invoke<SavedFile>('save_file', { path: target, content })
    },
    async pickExportPath(suggested, format) {
      return save({ defaultPath: suggested, filters: FILTERS[format] })
    },
    analyze: (markdown) => invoke<Analysis>('analyze', { markdown }),
    resolveUrl: (url) => invoke<Resolved>('resolve_url', { url }),
    importUrl: (request) => invoke<ImportResult>('import_url', { request }),
    async exportDocument(request) {
      const outPath = request.outPath
      if (!outPath) throw { code: 'invalid', message: 'Choose where to save the file.' }
      return invoke<ExportResult>('export_document', {
        request: { markdown: request.markdown, baseDir: request.baseDir, options: request.options, outPath },
      })
    },
    renderHtml: (markdown, baseDir, options) => invoke<string>('render_html', { markdown, baseDir, options }),
    loadSettings: () => invoke<LoadedSettings>('load_settings'),
    saveSettings: (settings: Settings) => invoke<Settings>('save_settings', { settings }),
    resetSettings: () => invoke<Settings>('reset_settings'),
    defaultExportDir: () => invoke<string>('default_export_dir'),
    fontFamilies: () => invoke<string[]>('font_families'),
    openPath: (path) => openPath(path),
    revealPath: (path) => revealItemInDir(path),
    openUrl: (url) => openUrl(url),
    imageUrl(src, baseDir) {
      if (isRemote(src) || !baseDir) return src
      return convertFileSrc(resolveRelative(baseDir, src))
    },
    confirm: (message, title) => ask(message, { title, kind: 'warning' }),
  }
}
