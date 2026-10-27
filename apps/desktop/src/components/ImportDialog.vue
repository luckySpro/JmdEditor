<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import RichEditor from '../editor/RichEditor.vue'
import { errorText, formatBytes, t, type Key } from '../i18n'
import { analyze } from '../lib/analyze'
import { platform } from '../platform'
import { applyImport, closeDialog, state, toast, updateSettings } from '../store'
import type { ImportResult, Resolved } from '../types'
import BaseDialog from './BaseDialog.vue'
import Icon from './Icon.vue'

/** The resolver's notes are English; show them in the interface language. */
const NOTE_KEYS: Record<string, Key> = {
  'Will fetch the raw file from raw.githubusercontent.com': 'import.note.githubRaw',
  'Will fetch the repository README': 'import.note.readme',
  'Raw GitHub file': 'import.note.rawGitHub',
  "Will fetch the gist's first file": 'import.note.gistFirst',
  'Raw gist file': 'import.note.rawGist',
  'Will fetch the raw file from Gitee': 'import.note.giteeRaw',
  'Raw Gitee file': 'import.note.rawGitee',
  'Will fetch the raw file from GitLab': 'import.note.gitlabRaw',
  'Raw GitLab file': 'import.note.rawGitLab',
  'Markdown file': 'import.note.markdown',
  'The main article will be converted to Markdown': 'import.note.article',
}
const noteText = (note: string) => (NOTE_KEYS[note] ? t(NOTE_KEYS[note]) : note)

const url = ref(state.importPrefill)
state.importPrefill = ''
const resolved = ref<Resolved | null>(null)
const resolveError = ref('')
const preview = ref<ImportResult | null>(null)
const previewKey = ref('')
const busy = ref(false)
const error = ref('')
const where = ref<'new' | 'cursor'>('new')
const saveImages = ref(state.settings.importSaveImages)
const recordSource = ref(state.settings.importRecordSource)

const canSaveImages = computed(() => platform.kind === 'tauri' && !!state.settings.workspace)
const summary = computed(() => {
  if (!preview.value) return ''
  const images = (preview.value.markdown.match(/!\[[^\]]*\]\(/g) ?? []).length
  return t('import.summary', {
    size: formatBytes(preview.value.bytes),
    images,
    headings: analyze(preview.value.markdown).outline.length,
  })
})

let timer: ReturnType<typeof setTimeout> | undefined
watch(
  url,
  (value) => {
    clearTimeout(timer)
    resolved.value = null
    resolveError.value = ''
    error.value = ''
    if (!value.trim()) return
    timer = setTimeout(async () => {
      try {
        resolved.value = await platform.resolveUrl(value.trim())
      } catch (e) {
        resolveError.value = errorText(e)
      }
    }, 250)
  },
  { immediate: true },
)
onBeforeUnmount(() => clearTimeout(timer))

function request(withImages: boolean) {
  return {
    url: url.value.trim(),
    saveImages: withImages,
    recordSource: recordSource.value,
    folder: state.settings.workspace,
  }
}

async function fetchPreview() {
  if (!url.value.trim() || busy.value) return
  busy.value = true
  error.value = ''
  try {
    preview.value = await platform.importUrl(request(false))
    previewKey.value = `${url.value.trim()}|${recordSource.value}`
  } catch (e) {
    preview.value = null
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}

async function submit() {
  if (!url.value.trim() || busy.value) return
  const withImages = saveImages.value && canSaveImages.value
  const reusable = preview.value && !withImages && previewKey.value === `${url.value.trim()}|${recordSource.value}`
  busy.value = true
  error.value = ''
  try {
    const result = reusable ? preview.value! : await platform.importUrl(request(withImages))
    updateSettings({ importSaveImages: saveImages.value, importRecordSource: recordSource.value })
    closeDialog()
    if (result.images.failed.length) {
      toast(t('import.imagesFailed', { n: result.images.failed.length }), 'error')
    }
    await applyImport(result, where.value)
  } catch (e) {
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}

async function paste() {
  try {
    url.value = (await navigator.clipboard.readText()).trim()
  } catch {
    /* clipboard access refused; the user can paste with the keyboard */
  }
}
</script>

<template>
  <BaseDialog :title="t('import.title')" icon="link" width="920px" @close="closeDialog">
    <template #lead><p>{{ t('import.lead') }}</p></template>

    <div class="cols">
      <form class="form" @submit.prevent="fetchPreview">
        <div class="field">
          <label for="imp-url" class="field-label">{{ t('import.link') }}</label>
          <div class="row">
            <input id="imp-url" v-model="url" class="input mono" type="url" inputmode="url" placeholder="https://" autofocus spellcheck="false" />
            <button type="button" class="btn secondary" @click="paste">{{ t('import.paste') }}</button>
          </div>
          <div v-if="resolved" class="detected">
            <span class="pill"><Icon name="check" :size="13" />{{ t('import.detected', { kind: t(`import.kind.${resolved.kind}`) }) }}</span>
            <span class="hint">{{ noteText(resolved.note) }}</span>
          </div>
          <p v-else-if="resolveError" class="err">{{ resolveError }}</p>
        </div>

        <fieldset class="box">
          <legend>{{ t('import.sources') }}</legend>
          <div class="sources">
            <div><strong>{{ t('import.src.repos') }}</strong><span>{{ t('import.src.reposHint') }}</span></div>
            <div><strong>{{ t('import.src.raw') }}</strong><span>{{ t('import.src.rawHint') }}</span></div>
            <div><strong>{{ t('import.src.gist') }}</strong><span>{{ t('import.src.gistHint') }}</span></div>
            <div><strong>{{ t('import.src.web') }}</strong><span>{{ t('import.src.webHint') }}</span></div>
          </div>
        </fieldset>

        <fieldset class="box">
          <legend>{{ t('import.options') }}</legend>
          <label v-if="platform.kind === 'tauri'" class="check">
            <input v-model="saveImages" type="checkbox" :disabled="!canSaveImages" />
            <span>
              <span>{{ t('import.saveImages') }}</span>
              <span class="hint">{{ canSaveImages ? t('import.saveImagesHint') : t('import.saveImagesNoFolder') }}</span>
            </span>
          </label>
          <label class="check">
            <input v-model="recordSource" type="checkbox" />
            <span>
              <span>{{ t('import.recordSource') }}</span>
              <span class="hint">{{ t('import.recordSourceHint') }}</span>
            </span>
          </label>
          <div class="radios" role="radiogroup" :aria-label="t('import.dest')">
            <label><input v-model="where" type="radio" value="new" />{{ t('import.asNew') }}</label>
            <label><input v-model="where" type="radio" value="cursor" :disabled="state.mode === 'read'" />{{ t('import.atCursor') }}</label>
          </div>
        </fieldset>

        <div v-if="state.settings.recentUrls.length" class="recent">
          <span class="field-label">{{ t('import.recent') }}</span>
          <button v-for="u in state.settings.recentUrls.slice(0, 4)" :key="u" type="button" class="recent-link" @click="url = u">
            {{ u.replace(/^https?:\/\//, '') }}
          </button>
        </div>
      </form>

      <div class="preview">
        <div class="preview-head">
          <span class="field-label">{{ t('import.preview') }}</span>
          <span v-if="preview" class="hint">{{ summary }}</span>
          <button v-else type="button" class="btn secondary small" :disabled="!url.trim() || busy" @click="fetchPreview">
            {{ busy ? t('import.fetching') : t('import.fetch') }}
          </button>
        </div>
        <div class="paper">
          <RichEditor v-if="preview" :key="previewKey" :model-value="preview.markdown" readonly :register="false" />
          <p v-else class="placeholder">{{ busy ? t('import.fetching') : t('import.previewEmpty') }}</p>
        </div>
        <p v-if="error" class="err" role="alert">{{ error }}</p>
        <p class="safety"><Icon name="shield" :size="14" />{{ t('import.safety') }}</p>
      </div>
    </div>

    <template #footer>
      <span class="tip">{{ t('import.tip') }}</span>
      <button type="button" class="btn secondary" @click="closeDialog">{{ t('action.cancel') }}</button>
      <button type="button" class="btn primary" :disabled="!url.trim() || busy || !!resolveError" @click="submit">
        {{ busy ? t('import.fetching') : t('import.submit') }}
      </button>
    </template>
  </BaseDialog>
</template>

<style scoped>
.cols {
  display: flex;
  flex-wrap: wrap;
}
.form {
  flex: 1 1 380px;
  min-width: 0;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.preview {
  flex: 1 1 340px;
  min-width: 0;
  padding: 20px 24px;
  background: var(--surface-2);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.row {
  display: flex;
  gap: 8px;
}
.row .input {
  flex: 1;
  min-width: 0;
}
.detected {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
.pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 9px;
  border-radius: 999px;
  background: var(--success-soft);
  color: var(--success);
  font-size: 12px;
  font-weight: 600;
}
.sources {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 10px;
}
.sources div {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 13px;
}
.sources span {
  font-size: 12px;
  color: var(--text-2);
}
.radios {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  font-size: 13px;
}
.radios label {
  display: flex;
  align-items: center;
  gap: 6px;
}
.recent {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: flex-start;
}
.recent-link {
  border: 0;
  background: transparent;
  padding: 2px 0;
  color: var(--accent);
  font-size: 12px;
  font-family: var(--font-mono);
  text-align: left;
  word-break: break-all;
}
.preview-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.paper {
  flex: 1;
  min-height: 260px;
  max-height: 420px;
  overflow: hidden;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--paper);
  display: flex;
  flex-direction: column;
}
.paper :deep(.rich-editor) {
  --editor-font-size: 13px;
}
.placeholder {
  margin: auto;
  padding: 24px;
  text-align: center;
  color: var(--text-3);
  font-size: 13px;
}
.safety {
  display: flex;
  gap: 8px;
  margin: 0;
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.5;
}
.safety svg {
  flex: none;
  margin-top: 2px;
}
.tip {
  margin-right: auto;
  font-size: 12px;
  color: var(--text-2);
}
@media (max-width: 760px) {
  .preview {
    border-left: 0;
    border-top: 1px solid var(--border);
  }
}
</style>
