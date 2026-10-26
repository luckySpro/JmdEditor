import { ref } from 'vue'

import type { Lang } from '../types'
import { en } from './en'
import { ja } from './ja'
import { zhCN } from './zh-CN'
import { zhTW } from './zh-TW'

export type Key = keyof typeof en
export type Messages = Record<Key, string>

const catalogs: Record<Lang, Messages> = { en, 'zh-CN': zhCN, 'zh-TW': zhTW, ja }

export const LANGUAGES: { code: Lang; label: string }[] = [
  { code: 'en', label: 'English' },
  { code: 'zh-CN', label: '简体中文' },
  { code: 'zh-TW', label: '繁體中文' },
  { code: 'ja', label: '日本語' },
]

export const lang = ref<Lang>('en')

export function systemLang(languages: readonly string[] = navigator.languages ?? [navigator.language]): Lang {
  for (const l of languages) {
    const lower = l.toLowerCase()
    if (lower.startsWith('zh')) {
      return /tw|hk|mo|hant/.test(lower) ? 'zh-TW' : 'zh-CN'
    }
    if (lower.startsWith('ja')) return 'ja'
    if (lower.startsWith('en')) return 'en'
  }
  return 'en'
}

export function setLanguage(pref: 'system' | Lang) {
  lang.value = pref === 'system' ? systemLang() : pref
  document.documentElement.lang = lang.value
}

export function t(key: Key, vars?: Record<string, string | number>): string {
  let text = catalogs[lang.value][key] ?? en[key] ?? key
  if (vars) {
    for (const [k, v] of Object.entries(vars)) text = text.replaceAll(`{${k}}`, String(v))
  }
  return text
}

/** Message for an error coming back from Rust or the server. */
export function errorText(e: unknown): string {
  if (typeof e === 'object' && e !== null && 'code' in e) {
    const { code, message } = e as { code: string; message: string }
    const key = `error.${code}` as Key
    if (key in en) return t(key, { message })
    return t('error.generic', { message })
  }
  return t('error.generic', { message: String(e) })
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(1)} MB`
}
