import { isTauri } from '@tauri-apps/api/core'

import { createTauriPlatform } from './tauri'
import type { Platform } from './types'
import { createWebPlatform } from './web'

export type { Platform } from './types'

export const platform: Platform = isTauri() ? createTauriPlatform() : createWebPlatform()
