// Browser build: notes come from file pickers and downloads, import and
// export go through jmd-server (the same Rust crates behind an HTTP API).

import { analyze } from '../lib/analyze'
import { withExtension } from '../lib/paths'
import { DEFAULT_SETTINGS, type AppError, type LoadedSettings, type Settings } from '../types'
import type { Platform } from './types'

const API = (import.meta.env.VITE_API_BASE ?? '').replace(/\/$/, '')
const SETTINGS_KEY = 'jmdeditor.settings'

async function api<T>(path: string, body: unknown, as: 'json' | 'text' | 'blob' = 'json'): Promise<T> {
  let res: Response
  try {
    res = await fetch(`${API}${path}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    })
  } catch (e) {
    throw { code: 'offline', message: `The export service could not be reached (${String(e)}).` } satisfies AppError
  }
  if (!res.ok) {
    if ([502, 503, 504].includes(res.status)) throw { code: 'offline', message: res.statusText } satisfies AppError
    let err: AppError = { code: 'server', message: `${res.status} ${res.statusText}` }
    try {
      err = await res.json()
    } catch {
      /* keep the status text */
    }
    throw err
  }
  return (as === 'json' ? res.json() : as === 'text' ? res.text() : res.blob()) as Promise<T>
}

function download(blob: Blob, name: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = name
  document.body.append(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 30_000)
}

function readStored(): LoadedSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY)
    if (!raw) return { settings: structuredClone(DEFAULT_SETTINGS), problem: null, path: 'browser storage' }
    const stored = JSON.parse(raw) as Partial<Settings>
    return {
      settings: { ...DEFAULT_SETTINGS, ...stored, export: { ...DEFAULT_SETTINGS.export, ...stored.export } },
      problem: null,
      path: 'browser storage',
    }
  } catch (e) {
    return { settings: structuredClone(DEFAULT_SETTINGS), problem: String(e), path: 'browser storage' }
  }
}

function pickLocalFile(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.md,.markdown,.mdown,.mkd,.txt,text/markdown,text/plain'
    input.onchange = () => resolve(input.files?.[0] ?? null)
    input.oncancel = () => resolve(null)
    input.click()
  })
}

export function createWebPlatform(): Platform {
  const unsupported = (what: string) => Promise.reject({ code: 'unsupported', message: `${what} needs the desktop app.` })
  return {
    kind: 'web',
    mobile: /Android|iPhone|iPad|iPod/i.test(navigator.userAgent),
    folders: false,
    appInfo: async () => ({
      name: 'JmdEditor',
      version: '0.1.0',
      author: 'M&J Group',
      email: '473276@qq.com',
      repository: 'https://github.com/luckySpro/JmdEditor',
      today: new Date().toISOString().slice(0, 10),
      platform: 'web',
    }),
    initialFile: async () => null,
    async pickFile() {
      const file = await pickLocalFile()
      if (!file) return null
      const content = (await file.text()).replace(/^﻿/, '')
      return { path: '', name: file.name, dir: '', content }
    },
    pickFolder: async () => null,
    openFile: () => unsupported('Opening files by path'),
    listFolder: async () => [],
    // No file system in the browser: the image is kept inside the note.
    saveAsset: (_dir, file) =>
      new Promise((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(String(reader.result))
        reader.onerror = () => reject({ code: 'io', message: String(reader.error) })
        reader.readAsDataURL(file)
      }),
    async saveFile(_path, content, suggestedName) {
      download(new Blob([content], { type: 'text/markdown;charset=utf-8' }), suggestedName)
      return { path: '', name: suggestedName, dir: '' }
    },
    pickExportPath: async () => null,
    analyze: async (markdown) => analyze(markdown),
    resolveUrl: (url) => api('/api/resolve', { url }),
    importUrl: (request) => api('/api/import', { url: request.url, recordSource: request.recordSource }),
    async exportDocument(request) {
      const started = performance.now()
      const blob = await api<Blob>('/api/export', { markdown: request.markdown, options: request.options }, 'blob')
      download(blob, withExtension(request.fileName || 'document', request.options.format))
      return { path: request.fileName, bytes: blob.size, millis: Math.round(performance.now() - started) }
    },
    renderHtml: (markdown, _baseDir, options) => api('/api/render', { markdown, options }, 'text'),
    loadSettings: async () => readStored(),
    async saveSettings(settings) {
      try {
        localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings))
      } catch {
        /* private mode: settings last for this tab only */
      }
      return settings
    },
    async resetSettings() {
      try {
        localStorage.removeItem(SETTINGS_KEY)
      } catch {
        /* ignore */
      }
      return structuredClone(DEFAULT_SETTINGS)
    },
    defaultExportDir: async () => null,
    fontFamilies: async () => [],
    openPath: async () => {},
    revealPath: async () => {},
    async openUrl(url) {
      window.open(url, '_blank', 'noopener')
    },
    imageUrl: (src) => src,
    confirm: async (message) => window.confirm(message),
  }
}

