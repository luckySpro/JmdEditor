<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import AppHeader from './components/AppHeader.vue'
import ExportDialog from './components/ExportDialog.vue'
import FormatToolbar from './components/FormatToolbar.vue'
import ImportDialog from './components/ImportDialog.vue'
import MobileSheet from './components/MobileSheet.vue'
import SettingsDialog from './components/SettingsDialog.vue'
import Sidebar from './components/Sidebar.vue'
import StatusBar from './components/StatusBar.vue'
import Toasts from './components/Toasts.vue'
import ReadView from './editor/ReadView.vue'
import RichEditor from './editor/RichEditor.vue'
import SourceEditor from './editor/SourceEditor.vue'
import { t } from './i18n'
import { platform } from './platform'
import { dirty, docName, init, newNote, openPath, openPicker, save, setContent, state } from './store'
import type { Format } from './types'

const exportFormat = ref<Format | null>(null)
const showSidebar = computed(() => platform.folders && state.sidebarOpen)

function onKey(e: KeyboardEvent) {
  const mod = e.ctrlKey || e.metaKey
  if (!mod || state.dialog) return
  const key = e.key.toLowerCase()
  const actions: Record<string, () => void> = {
    s: () => void save(e.shiftKey),
    o: () => void openPicker(),
    n: () => void newNote(),
    p: () => (state.dialog = 'export'),
    '\\': () => (state.sidebarOpen = !state.sidebarOpen),
  }
  if (e.shiftKey && key === 'i') {
    e.preventDefault()
    state.dialog = 'import'
    return
  }
  const run = actions[key]
  if (run && !(key === 's' && e.altKey)) {
    e.preventDefault()
    run()
  }
}

/** A link dropped on the window opens the import dialog; a Markdown file opens. */
function onDrop(e: DragEvent) {
  const dt = e.dataTransfer
  if (!dt) return
  const files = [...dt.files]
  if (files.some((f) => f.type.startsWith('image/'))) return // editors store images themselves
  const uri = dt.getData('text/uri-list') || dt.getData('text/plain')
  if (!files.length && uri && /^https?:\/\//i.test(uri.trim())) {
    e.preventDefault()
    state.importPrefill = uri.split(/\r?\n/)[0].trim()
    state.dialog = 'import'
    return
  }
  const md = files.find((f) => /\.(md|markdown|mdown|mkd|txt)$/i.test(f.name))
  if (md && platform.kind === 'web') {
    e.preventDefault()
    void md.text().then((content) => {
      state.doc.path = null
      state.doc.name = md.name
      state.doc.dir = null
      state.doc.savedContent = content
      state.doc.content = content
      state.doc.revision++
      setContent(content)
    })
  }
}

function onBeforeUnload(e: BeforeUnloadEvent) {
  if (dirty.value && platform.kind === 'web') e.preventDefault()
}

let unlisten: (() => void)[] = []

onMounted(async () => {
  window.addEventListener('keydown', onKey)
  window.addEventListener('dragover', (e) => e.preventDefault())
  window.addEventListener('drop', onDrop)
  window.addEventListener('beforeunload', onBeforeUnload)
  await init()
  if (platform.kind === 'tauri') {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const { getCurrentWebview } = await import('@tauri-apps/api/webview')
    const win = getCurrentWindow()
    unlisten.push(
      await win.onCloseRequested(async (event) => {
        if (!dirty.value) return
        if (state.doc.path && state.settings.autosave) {
          if (!(await save())) event.preventDefault()
          return
        }
        const ok = await platform.confirm(t('confirm.discard', { name: docName.value }), t('confirm.title'))
        if (!ok) event.preventDefault()
      }),
      await getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type !== 'drop') return
        const md = event.payload.paths.find((p) => /\.(md|markdown|mdown|mkd|txt)$/i.test(p))
        if (md) void openPath(md)
      }),
    )
    watch(
      () => `${dirty.value ? '• ' : ''}${docName.value} — JmdEditor`,
      (title) => void win.setTitle(title).catch(() => {}),
      { immediate: true },
    )
  } else {
    watch(() => `${dirty.value ? '• ' : ''}${docName.value} — JmdEditor`, (title) => (document.title = title), { immediate: true })
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('drop', onDrop)
  window.removeEventListener('beforeunload', onBeforeUnload)
  unlisten.forEach((u) => u())
  unlisten = []
})

function customizeExport(format: Format) {
  exportFormat.value = format
  state.dialog = 'export'
}

watch(
  () => state.dialog,
  (d) => {
    if (d !== 'export') exportFormat.value = null
  },
)
</script>

<template>
  <div v-if="state.ready" class="app" :class="[`mode-${state.mode}`, { mobile: platform.mobile }]">
    <AppHeader />
    <div class="body">
      <Sidebar v-if="showSidebar" />
      <main class="main">
        <FormatToolbar />
        <div class="panes">
          <template v-if="state.mode === 'split'">
            <SourceEditor
              class="pane source"
              :model-value="state.doc.content"
              :revision="state.doc.revision"
              @update:model-value="setContent"
            />
            <RichEditor class="pane preview" :model-value="state.doc.content" :base-dir="state.doc.dir" readonly :register="false" />
          </template>
          <SourceEditor
            v-else-if="state.mode === 'source'"
            class="pane"
            :model-value="state.doc.content"
            :revision="state.doc.revision"
            @update:model-value="setContent"
          />
          <RichEditor
            v-else-if="state.mode === 'rich'"
            :key="state.doc.revision"
            class="pane"
            :model-value="state.doc.content"
            :base-dir="state.doc.dir"
            @update:model-value="setContent"
          />
          <ReadView v-else class="pane" :markdown="state.doc.content" :base-dir="state.doc.dir" />
        </div>
      </main>
    </div>
    <StatusBar />

    <ImportDialog v-if="state.dialog === 'import'" />
    <ExportDialog v-else-if="state.dialog === 'export'" :format="exportFormat" />
    <SettingsDialog v-else-if="state.dialog === 'settings'" />
    <MobileSheet v-else-if="state.dialog === 'sheet'" @customize="customizeExport" />
    <Toasts />
  </div>
</template>

<style scoped>
.app {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.body {
  flex: 1;
  min-height: 0;
  display: flex;
}
.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.panes {
  flex: 1;
  min-height: 0;
  display: flex;
}
.pane {
  flex: 1 1 0;
  min-width: 0;
}
.pane.source {
  border-right: 1px solid var(--border);
}
@media (max-width: 900px) {
  .mode-split .panes {
    flex-direction: column;
  }
  .pane.source {
    border-right: 0;
    border-bottom: 1px solid var(--border);
  }
}
</style>
