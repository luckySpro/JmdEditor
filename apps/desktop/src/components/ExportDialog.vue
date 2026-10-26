<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'

import ReadView from '../editor/ReadView.vue'
import { errorText, formatBytes, lang, t } from '../i18n'
import { withExtension } from '../lib/paths'
import { platform } from '../platform'
import {
  closeDialog,
  defaultExportPath,
  docName,
  noteExportName,
  rememberExport,
  state,
  toast,
  updateSettings,
} from '../store'
import type { ExportOptions, Format, Margins, Paper, Template } from '../types'
import BaseDialog from './BaseDialog.vue'
import Icon from './Icon.vue'

const props = withDefaults(defineProps<{ format?: Format | null }>(), { format: null })

const opts = reactive<ExportOptions>({
  ...state.settings.export,
  format: props.format ?? state.settings.export.format,
  title: state.analysis.title,
  date: state.info?.today ?? new Date().toISOString().slice(0, 10),
})
const outPath = ref('')
const openAfter = ref(state.settings.openAfterExport)
const busy = ref(false)
const error = ref('')

const FORMATS: { id: Format; label: string }[] = [
  { id: 'pdf', label: 'PDF' },
  { id: 'docx', label: 'Word' },
  { id: 'html', label: 'HTML' },
]
const TEMPLATES: Template[] = ['clean', 'academic', 'business', 'github']
const PAPERS: Paper[] = ['a4', 'letter']
const MARGINS: Margins[] = ['narrow', 'normal', 'wide']
const fmtLabel = computed(() => FORMATS.find((f) => f.id === opts.format)!.label)

async function suggestPath() {
  const current = outPath.value
  outPath.value = current ? withExtension(current, opts.format) : ((await defaultExportPath(opts.format)) ?? '')
}
onMounted(suggestPath)
watch(() => opts.format, suggestPath)

async function choose() {
  const picked = await platform.pickExportPath(outPath.value || noteExportName(opts.format), opts.format)
  if (picked) outPath.value = picked
}

async function submit() {
  if (busy.value) return
  busy.value = true
  error.value = ''
  const options: ExportOptions = { ...opts, lang: lang.value, title: opts.title?.trim() || null, author: opts.author?.trim() || null }
  try {
    const result = await platform.exportDocument({
      markdown: state.doc.content,
      baseDir: state.doc.dir,
      options,
      fileName: noteExportName(opts.format),
      outPath: platform.kind === 'tauri' ? outPath.value : null,
    })
    // Remember the choices for next time, except the per-document title and date.
    updateSettings({
      export: { ...options, title: null, date: null },
      openAfterExport: openAfter.value,
    })
    closeDialog()
    const name = result.path.split(/[\\/]/).pop() ?? result.path
    if (platform.kind === 'tauri') {
      rememberExport(result.path)
      if (openAfter.value) void platform.openPath(result.path).catch(() => {})
    }
    toast(
      t('export.done', { name, size: formatBytes(result.bytes), time: `${(result.millis / 1000).toFixed(1)} s` }),
      'success',
      platform.kind === 'tauri' ? { label: t('action.reveal'), run: () => void platform.revealPath(result.path) } : undefined,
    )
  } catch (e) {
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <BaseDialog :title="t('export.title', { name: docName })" width="1040px" labelled-by="exp-title" @close="closeDialog">
    <template #title>
      <div class="title-row">
        <h1 id="exp-title">{{ t('export.title', { name: docName }) }}</h1>
        <div class="segmented" role="tablist" :aria-label="t('export.format')">
          <button v-for="f in FORMATS" :key="f.id" type="button" role="tab" :aria-selected="opts.format === f.id" :aria-pressed="opts.format === f.id" @click="opts.format = f.id">
            {{ f.label }}
          </button>
        </div>
      </div>
    </template>

    <div class="cols">
      <form class="form" @submit.prevent="submit">
        <div class="field">
          <span class="field-label">{{ t('export.template') }}</span>
          <div class="templates">
            <button v-for="tp in TEMPLATES" :key="tp" type="button" :class="['tpl', tp]" :aria-pressed="opts.template === tp" @click="opts.template = tp">
              <span class="thumb" aria-hidden="true"><span></span><span></span><span></span><span></span></span>
              {{ t(`export.tpl.${tp}`) }}
            </button>
          </div>
        </div>

        <div class="grid2" v-if="opts.format !== 'html'">
          <label class="field">
            <span class="field-label">{{ t('export.paper') }}</span>
            <select v-model="opts.paper" class="input">
              <option v-for="p in PAPERS" :key="p" :value="p">{{ t(`export.paper.${p}`) }}</option>
            </select>
          </label>
          <label class="field">
            <span class="field-label">{{ t('export.margins') }}</span>
            <select v-model="opts.margins" class="input">
              <option v-for="m in MARGINS" :key="m" :value="m">{{ t(`export.margins.${m}`) }}</option>
            </select>
          </label>
        </div>

        <fieldset class="box">
          <legend>{{ t('export.details') }}</legend>
          <div class="grid3">
            <label class="field"><span class="field-label">{{ t('export.docTitle') }}</span><input v-model="opts.title" class="input" type="text" /></label>
            <label class="field"><span class="field-label">{{ t('export.author') }}</span><input v-model="opts.author" class="input" type="text" placeholder="M&J Group" /></label>
            <label class="field"><span class="field-label">{{ t('export.date') }}</span><input v-model="opts.date" class="input" type="date" /></label>
          </div>
        </fieldset>

        <fieldset class="box">
          <legend>{{ t('export.structure') }}</legend>
          <label class="switch-row">
            {{ t('export.toc') }}
            <span class="inline">
              <select v-model.number="opts.tocDepth" class="input small" :aria-label="t('export.toc')" :disabled="!opts.toc">
                <option v-for="n in [1, 2, 3, 4, 5, 6]" :key="n" :value="n">{{ t('export.tocDepth', { n }) }}</option>
              </select>
              <input v-model="opts.toc" type="checkbox" />
            </span>
          </label>
          <label class="switch-row">{{ t('export.number') }}<input v-model="opts.numberHeadings" type="checkbox" /></label>
          <label v-if="opts.format !== 'html'" class="switch-row">{{ t('export.pageNumbers') }}<input v-model="opts.pageNumbers" type="checkbox" /></label>
          <label class="switch-row">{{ t('export.cover') }}<input v-model="opts.cover" type="checkbox" /></label>
        </fieldset>

        <fieldset v-if="opts.format === 'pdf'" class="box">
          <legend>{{ t('export.pdfOnly') }}</legend>
          <label class="switch-row">{{ t('export.bookmarks') }}<input v-model="opts.bookmarks" type="checkbox" /></label>
          <label class="switch-row">{{ t('export.pdfA') }}<input v-model="opts.pdfA" type="checkbox" /></label>
        </fieldset>
        <fieldset v-else-if="opts.format === 'docx'" class="box">
          <legend>{{ t('export.wordOnly') }}</legend>
          <label class="switch-row">{{ t('export.codeTables') }}<input v-model="opts.codeAsTable" type="checkbox" /></label>
          <span class="hint">{{ t('export.wordTocNote') }}</span>
        </fieldset>
        <fieldset v-else class="box">
          <legend>{{ t('export.htmlOnly') }}</legend>
          <label class="switch-row">{{ t('export.inline') }}<input v-model="opts.inlineImages" type="checkbox" /></label>
        </fieldset>
      </form>

      <div class="preview">
        <div class="preview-head">
          <span class="field-label">{{ t('export.preview', { format: fmtLabel }) }}</span>
          <span class="hint">{{ t('export.previewNote') }}</span>
        </div>
        <div class="sheet" :class="[opts.paper, opts.margins]">
          <ReadView :markdown="state.doc.content" :base-dir="state.doc.dir" :options="opts" compact />
        </div>
      </div>
    </div>

    <template #footer>
      <label v-if="platform.kind === 'tauri'" class="save-to">
        <span class="field-label">{{ t('export.saveTo') }}</span>
        <input v-model="outPath" class="input mono" type="text" spellcheck="false" />
        <button type="button" class="btn secondary" @click="choose">{{ t('export.choose') }}</button>
      </label>
      <span v-else class="hint save-to">{{ t('export.download') }}</span>
      <label v-if="platform.kind === 'tauri'" class="check-inline"><input v-model="openAfter" type="checkbox" />{{ t('export.openWhenDone') }}</label>
      <p v-if="error" class="err" role="alert">{{ error }}</p>
      <button type="button" class="btn secondary" @click="closeDialog">{{ t('action.cancel') }}</button>
      <button type="button" class="btn primary" :disabled="busy || (platform.kind === 'tauri' && !outPath)" @click="submit">
        <Icon name="download" />{{ busy ? t('export.working') : t('export.submit', { format: fmtLabel }) }}
      </button>
    </template>
  </BaseDialog>
</template>

<style scoped>
.title-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px 20px;
}
.cols {
  display: flex;
  flex-wrap: wrap;
}
.form {
  flex: 1 1 400px;
  min-width: 0;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.preview {
  flex: 1 1 380px;
  min-width: 0;
  padding: 20px 24px;
  background: var(--surface-3);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.preview-head {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.sheet {
  flex: 1;
  min-height: 380px;
  max-height: 560px;
  border-radius: 4px;
  overflow: hidden;
  background: #fff;
  box-shadow: 0 1px 3px rgba(16, 24, 40, 0.18), 0 8px 24px rgba(16, 24, 40, 0.12);
  display: flex;
  flex-direction: column;
}
.templates {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}
.tpl {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 10px 6px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface);
  color: var(--text);
  font-size: 12px;
}
.tpl[aria-pressed='true'] {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
  color: var(--accent-ink);
  font-weight: 600;
}
.thumb {
  width: 44px;
  height: 56px;
  border-radius: 3px;
  background: #fff;
  border: 1px solid #e3e6eb;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px 5px;
  box-sizing: border-box;
}
.thumb span {
  height: 3px;
  border-radius: 2px;
  background: #d5dae1;
}
.thumb span:first-child {
  height: 5px;
  width: 70%;
  background: #1b1e24;
}
.tpl.academic .thumb span:first-child {
  margin: 0 auto;
}
.tpl.business .thumb span:first-child {
  background: var(--accent);
  width: 100%;
}
.tpl.github .thumb span:first-child {
  background: #24292f;
}
.tpl.github .thumb span:nth-child(2) {
  background: #d0d7de;
  height: 1px;
}
.grid2 {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 12px;
}
.grid3 {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 12px;
}
.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 13px;
  min-height: 32px;
}
.inline {
  display: flex;
  align-items: center;
  gap: 10px;
}
.save-to {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1 1 320px;
  min-width: 0;
}
.save-to .input {
  flex: 1;
  min-width: 0;
}
.check-inline {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}
@media (max-width: 760px) {
  .preview {
    border-left: 0;
    border-top: 1px solid var(--border);
  }
  .templates {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
