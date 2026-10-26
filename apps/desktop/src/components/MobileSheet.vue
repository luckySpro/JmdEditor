<script setup lang="ts">
// Bottom sheet on phones: quick PDF/Word export, import and saving.
import { onMounted, ref } from 'vue'

import { errorText, formatBytes, lang, t } from '../i18n'
import { platform } from '../platform'
import { closeDialog, defaultExportPath, noteExportName, rememberExport, save, state, toast } from '../store'
import type { Format } from '../types'
import Icon from './Icon.vue'

const emit = defineEmits<{ customize: [format: Format] }>()
const busy = ref<Format | null>(null)
const sheet = ref<HTMLElement>()

onMounted(() => sheet.value?.querySelector('button')?.focus())

async function quick(format: Format) {
  busy.value = format
  try {
    const outPath = platform.kind === 'tauri' ? await defaultExportPath(format) : null
    const result = await platform.exportDocument({
      markdown: state.doc.content,
      baseDir: state.doc.dir,
      options: { ...state.settings.export, format, lang: lang.value, title: state.analysis.title },
      fileName: noteExportName(format),
      outPath,
    })
    closeDialog()
    if (platform.kind === 'tauri') {
      rememberExport(result.path)
      void platform.openPath(result.path).catch(() => {})
    }
    toast(t('export.done', { name: noteExportName(format), size: formatBytes(result.bytes), time: `${(result.millis / 1000).toFixed(1)} s` }), 'success')
  } catch (e) {
    toast(errorText(e), 'error')
  } finally {
    busy.value = null
  }
}
</script>

<template>
  <div class="scrim" @click.self="closeDialog" @keydown.esc="closeDialog">
    <section ref="sheet" class="sheet" role="dialog" aria-modal="true" :aria-label="t('mobile.share')">
      <span class="grip" aria-hidden="true"></span>
      <span class="title">{{ t('mobile.share') }}</span>
      <div class="big">
        <button type="button" :disabled="!!busy" @click="quick('pdf')">
          <span class="badge pdf">PDF</span>
          <span class="txt"><strong>PDF</strong><span>{{ busy === 'pdf' ? t('export.working') : t('mobile.pdfHint') }}</span></span>
        </button>
        <button type="button" :disabled="!!busy" @click="quick('docx')">
          <span class="badge docx">DOCX</span>
          <span class="txt"><strong>Word</strong><span>{{ busy === 'docx' ? t('export.working') : t('mobile.wordHint') }}</span></span>
        </button>
      </div>
      <div class="list">
        <button type="button" @click="state.dialog = 'import'">
          <Icon name="link" /><span class="txt"><span>{{ t('action.import') }}</span><span class="hint">{{ t('mobile.importHint') }}</span></span>
        </button>
        <button type="button" @click="closeDialog(); save()"><Icon name="save" /><span class="txt">{{ t('mobile.saveMd') }}</span></button>
        <button type="button" @click="emit('customize', state.settings.export.format)">
          <Icon name="settings" />
          <span class="txt">
            <span>{{ t('mobile.exportSettings') }}</span>
            <span class="hint">{{ state.settings.export.paper.toUpperCase() }} · {{ t(`export.tpl.${state.settings.export.template}`) }}</span>
          </span>
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: var(--backdrop);
  display: flex;
  align-items: flex-end;
  justify-content: center;
}
.sheet {
  width: 100%;
  max-width: 560px;
  background: var(--surface);
  border-radius: 18px 18px 0 0;
  padding: 10px 16px max(20px, env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 14px;
  animation: up 0.18s ease-out;
}
.grip {
  align-self: center;
  width: 40px;
  height: 4px;
  border-radius: 2px;
  background: var(--border-strong);
}
.title {
  font-weight: 600;
  font-size: 16px;
}
.big {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.big button {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 12px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-2);
  color: var(--text);
  text-align: left;
}
.badge {
  width: 40px;
  height: 48px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 700;
  color: #fff;
  flex: none;
}
.badge.pdf {
  background: #c2410c;
}
.badge.docx {
  background: #1d4ed8;
}
.txt {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 14px;
  min-width: 0;
}
.txt span {
  font-size: 12px;
  color: var(--text-2);
}
.list {
  display: flex;
  flex-direction: column;
}
.list button {
  display: flex;
  align-items: center;
  gap: 14px;
  min-height: 52px;
  border: 0;
  border-top: 1px solid var(--border);
  background: transparent;
  color: var(--text);
  text-align: left;
  font-size: 15px;
}
.list .txt > span:first-child {
  font-size: 15px;
  color: var(--text);
}
@keyframes up {
  from {
    transform: translateY(40px);
    opacity: 0;
  }
}
</style>
