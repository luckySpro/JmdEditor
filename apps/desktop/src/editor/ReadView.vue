<script setup lang="ts">
// Read mode: the note rendered by the Rust HTML exporter, so numbering, TOC
// and alerts look the way they will in the exported file.
import { onBeforeUnmount, ref, watch } from 'vue'

import { lang, t } from '../i18n'
import { platform } from '../platform'
import { state, type EditorHandle } from '../store'
import type { ExportOptions } from '../types'
import RichEditor from './RichEditor.vue'

const props = withDefaults(
  defineProps<{ markdown: string; baseDir: string | null; compact?: boolean; options?: ExportOptions | null }>(),
  { compact: false, options: null },
)

const frame = ref<HTMLIFrameElement>()
const html = ref('')
const failed = ref(false)
let timer: ReturnType<typeof setTimeout> | undefined
let generation = 0

async function render() {
  const mine = ++generation
  try {
    const out = await platform.renderHtml(props.markdown, props.baseDir, {
      ...(props.options ?? state.settings.export),
      format: 'html',
      inlineImages: true,
      lang: lang.value,
    })
    if (mine !== generation) return
    html.value = out
    failed.value = false
  } catch (e) {
    console.warn('read view render failed', e)
    if (mine === generation) failed.value = true
  }
}

watch(
  () => [props.markdown, props.baseDir, lang.value, JSON.stringify(props.options ?? state.settings.export)],
  () => {
    clearTimeout(timer)
    timer = setTimeout(render, html.value ? 250 : 0)
  },
  { immediate: true },
)

/** Links open in the system browser; in-page anchors scroll. */
function onLoad() {
  const doc = frame.value?.contentDocument
  if (!doc) return
  doc.addEventListener('click', (e) => {
    const a = (e.target as Element | null)?.closest?.('a')
    const href = a?.getAttribute('href')
    if (!href) return
    e.preventDefault()
    if (href.startsWith('#')) doc.getElementById(decodeURIComponent(href.slice(1)))?.scrollIntoView({ behavior: 'smooth' })
    else if (/^(https?:|mailto:)/i.test(href)) void platform.openUrl(href)
  })
}

const handle: EditorHandle = {
  command: () => {},
  insertMarkdown: () => {},
  focus: () => frame.value?.focus(),
  revealHeading(_index, id) {
    frame.value?.contentDocument?.getElementById(id)?.scrollIntoView({ block: 'start', behavior: 'smooth' })
  },
}
if (!props.compact) state.editor = handle

onBeforeUnmount(() => {
  clearTimeout(timer)
  if (state.editor === handle) state.editor = null
})
</script>

<template>
  <div class="read-view">
    <template v-if="failed">
      <p class="fallback" role="status">{{ t('read.fallback') }}</p>
      <RichEditor :model-value="markdown" :base-dir="baseDir" readonly :register="false" />
    </template>
    <iframe
      v-else
      ref="frame"
      :srcdoc="html"
      sandbox="allow-same-origin"
      :title="t('mode.read')"
      @load="onLoad"
    ></iframe>
  </div>
</template>

<style scoped>
.read-view {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: #fff;
}
iframe {
  flex: 1;
  width: 100%;
  border: 0;
  background: #fff;
}
.fallback {
  margin: 0;
  padding: 8px 16px;
  font-size: 12px;
  color: var(--text-2);
  background: var(--surface-2);
  border-bottom: 1px solid var(--border);
}
</style>
