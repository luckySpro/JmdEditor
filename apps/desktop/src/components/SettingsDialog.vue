<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

import { LANGUAGES, t } from '../i18n'
import { platform } from '../platform'
import { checkForUpdates, closeDialog, resetSettings, state, updateSettings } from '../store'
import type { EditorMode, Settings } from '../types'
import BaseDialog from './BaseDialog.vue'

const ACCENTS = ['#2F5BD3', '#0F766E', '#7C3AED', '#B45309', '#BE123C']
const MODES: EditorMode[] = ['rich', 'split', 'source', 'read']
const fonts = ref<string[] | null>(null)
/** Apps cannot make themselves the default, so the settings say how. */
const defaultAppHint = computed(() => {
  const os = state.info?.platform
  return os === 'windows' || os === 'macos' || os === 'linux' ? t(`settings.defaultApp.${os}`) : null
})

onMounted(async () => {
  if (platform.kind === 'tauri') fonts.value = await platform.fontFamilies().catch(() => [])
})

function set<K extends keyof Settings>(key: K, value: Settings[K]) {
  updateSettings({ [key]: value } as Partial<Settings>)
}

async function chooseExportDir() {
  const dir = await platform.pickFolder()
  if (dir) set('exportDir', dir)
}

async function reset() {
  if (await platform.confirm(t('settings.resetConfirm'), t('settings.reset'))) await resetSettings()
}
</script>

<template>
  <BaseDialog :title="t('settings.title')" icon="settings" width="640px" @close="closeDialog">
    <div class="settings">
      <p v-if="state.settingsProblem" class="banner" role="alert">{{ t('settings.problem', { problem: state.settingsProblem }) }}</p>

      <section>
        <h2>{{ t('settings.general') }}</h2>
        <label class="row">
          <span>{{ t('settings.language') }}</span>
          <select class="input" :value="state.settings.language" @change="set('language', ($event.target as HTMLSelectElement).value as Settings['language'])">
            <option value="system">{{ t('settings.system') }}</option>
            <option v-for="l in LANGUAGES" :key="l.code" :value="l.code">{{ l.label }}</option>
          </select>
        </label>
        <div class="row">
          <span>{{ t('settings.theme') }}</span>
          <div class="segmented" role="group" :aria-label="t('settings.theme')">
            <button v-for="th in ['system', 'light', 'dark'] as const" :key="th" type="button" :aria-pressed="state.settings.theme === th" @click="set('theme', th)">
              {{ th === 'system' ? t('settings.system') : t(`settings.${th}`) }}
            </button>
          </div>
        </div>
        <div class="row">
          <span>{{ t('settings.accent') }}</span>
          <div class="swatches" role="group" :aria-label="t('settings.accent')">
            <button
              v-for="c in ACCENTS"
              :key="c"
              type="button"
              class="swatch"
              :style="{ background: c }"
              :aria-label="c"
              :aria-pressed="state.settings.accent.toLowerCase() === c.toLowerCase()"
              @click="set('accent', c)"
            ></button>
            <input type="color" :value="state.settings.accent" :aria-label="t('settings.accent')" @change="set('accent', ($event.target as HTMLInputElement).value)" />
          </div>
        </div>
      </section>

      <section>
        <h2>{{ t('settings.editor') }}</h2>
        <label class="row">
          <span>{{ t('settings.defaultMode') }}</span>
          <select class="input" :value="state.settings.editorMode" @change="set('editorMode', ($event.target as HTMLSelectElement).value as EditorMode)">
            <option v-for="m in MODES" :key="m" :value="m">{{ t(`mode.${m}`) }}</option>
          </select>
        </label>
        <label class="row">
          <span>{{ t('settings.fontSize') }} · {{ state.settings.editorFontSize }}px</span>
          <input type="range" min="11" max="24" :value="state.settings.editorFontSize" @input="set('editorFontSize', Number(($event.target as HTMLInputElement).value))" />
        </label>
        <label v-if="platform.kind === 'tauri'" class="row">
          <span>{{ t('settings.autosave') }}</span>
          <input type="checkbox" :checked="state.settings.autosave" @change="set('autosave', ($event.target as HTMLInputElement).checked)" />
        </label>
      </section>

      <section>
        <h2>{{ t('settings.files') }}</h2>
        <div v-if="platform.kind === 'tauri'" class="row">
          <span>{{ t('settings.exportDir') }}</span>
          <span class="inline">
            <code class="path">{{ state.settings.exportDir ?? '—' }}</code>
            <button type="button" class="btn secondary small" @click="chooseExportDir">{{ t('export.choose') }}</button>
          </span>
        </div>
        <p class="hint">{{ t('settings.exportDirHint') }}</p>
        <label v-if="platform.kind === 'tauri'" class="row">
          <span>{{ t('settings.openAfter') }}</span>
          <input type="checkbox" :checked="state.settings.openAfterExport" @change="set('openAfterExport', ($event.target as HTMLInputElement).checked)" />
        </label>
        <label v-if="platform.kind === 'tauri'" class="row">
          <span>{{ t('settings.importImages') }}</span>
          <input type="checkbox" :checked="state.settings.importSaveImages" @change="set('importSaveImages', ($event.target as HTMLInputElement).checked)" />
        </label>
        <label class="row">
          <span>{{ t('settings.importSource') }}</span>
          <input type="checkbox" :checked="state.settings.importRecordSource" @change="set('importRecordSource', ($event.target as HTMLInputElement).checked)" />
        </label>
        <template v-if="platform.kind === 'tauri' && defaultAppHint">
          <div class="row"><span>{{ t('settings.defaultApp') }}</span></div>
          <p class="hint">{{ defaultAppHint }}</p>
        </template>
        <details v-if="platform.kind === 'tauri'">
          <summary>{{ t('settings.fonts') }} ({{ fonts?.length ?? '…' }})</summary>
          <p class="hint fonts">{{ fonts?.length ? fonts.join(' · ') : t('settings.fontsNone') }}</p>
        </details>
      </section>

      <section>
        <h2>{{ t('settings.about') }}</h2>
        <p class="about">
          <strong>JmdEditor</strong> ·
          {{ t('settings.version', { version: state.info?.version ?? '0.1.0', author: state.info?.author ?? 'M&J Group' }) }}
          · <a href="#" @click.prevent="platform.openUrl('https://github.com/luckySpro/JmdEditor')">GitHub</a>
        </p>
        <p v-if="platform.kind === 'tauri'">
          <button type="button" class="btn secondary small" :disabled="state.update.checking" @click="checkForUpdates(true)">
            {{ state.update.checking ? t('update.checking') : t('update.check') }}
          </button>
        </p>
        <p class="hint">{{ t('settings.storedAt', { path: state.settingsPath }) }}</p>
        <button type="button" class="btn danger small" @click="reset">{{ t('settings.reset') }}</button>
      </section>
    </div>
  </BaseDialog>
</template>

<style scoped>
.settings {
  padding: 8px 24px 24px;
}
section {
  padding: 14px 0;
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 10px;
}
section:last-child {
  border-bottom: 0;
}
h2 {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-2);
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  font-size: 14px;
  min-height: 36px;
}
.row .input {
  max-width: 220px;
}
.swatches {
  display: flex;
  align-items: center;
  gap: 8px;
}
.swatch {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid var(--surface);
  box-shadow: 0 0 0 1px var(--border);
}
.swatch[aria-pressed='true'] {
  box-shadow: 0 0 0 2px var(--text);
}
input[type='color'] {
  width: 28px;
  height: 28px;
  border: 0;
  padding: 0;
  background: none;
}
.inline {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.path {
  font-size: 12px;
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 260px;
}
.banner {
  margin: 12px 0 0;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--danger-soft);
  color: var(--danger);
  font-size: 13px;
}
.about {
  margin: 0;
  font-size: 14px;
}
.fonts {
  max-height: 140px;
  overflow: auto;
}
details summary {
  font-size: 14px;
  cursor: pointer;
}
.btn.danger {
  align-self: flex-start;
}
</style>
