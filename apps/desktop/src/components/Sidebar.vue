<script setup lang="ts">
import { computed, ref } from 'vue'

import { t } from '../i18n'
import { baseName } from '../lib/paths'
import { platform } from '../platform'
import { openFolder, openPath, state } from '../store'
import Icon from './Icon.vue'

const query = ref('')

const files = computed(() => {
  const q = query.value.trim().toLowerCase()
  return state.folder.filter((e) => !e.isDir && (!q || e.name.toLowerCase().includes(q)))
})
const folders = computed(() => state.folder.filter((e) => e.isDir))
const recent = computed(() => {
  const q = query.value.trim().toLowerCase()
  return state.settings.recentFiles.filter((p) => !q || baseName(p).toLowerCase().includes(q)).slice(0, 8)
})
const minDepth = computed(() => Math.min(...state.analysis.outline.map((o) => o.level), 6))

function ago(at: number) {
  const m = Math.round((Date.now() - at) / 60000)
  return m < 1 ? t('sidebar.justNow') : t('sidebar.minutesAgo', { n: m })
}
</script>

<template>
  <aside class="sidebar">
    <div class="tabs" role="tablist">
      <button type="button" role="tab" :aria-selected="state.sidebarTab === 'files'" @click="state.sidebarTab = 'files'">{{ t('sidebar.files') }}</button>
      <button type="button" role="tab" :aria-selected="state.sidebarTab === 'outline'" @click="state.sidebarTab = 'outline'">{{ t('sidebar.outline') }}</button>
    </div>

    <template v-if="state.sidebarTab === 'files'">
      <label class="search">
        <Icon name="search" :size="15" />
        <input v-model="query" type="search" :placeholder="t('sidebar.search')" />
      </label>

      <div class="group">
        <span class="label">
          {{ state.settings.workspace ? baseName(state.settings.workspace) : t('sidebar.folder') }}
          <button type="button" class="link-btn" :title="t('action.openFolder')" @click="openFolder()"><Icon name="folder" :size="14" /></button>
        </span>
        <p v-if="!state.settings.workspace" class="empty">
          {{ t('sidebar.noFolder') }}
          <button type="button" class="btn secondary small" @click="openFolder()">{{ t('action.openFolder') }}</button>
        </p>
        <button v-for="f in folders" :key="f.path" type="button" class="item dir" @click="openFolder(f.path)">
          <Icon name="folder" :size="15" /><span>{{ f.name }}</span>
        </button>
        <button
          v-for="f in files"
          :key="f.path"
          type="button"
          class="item"
          :aria-current="f.path === state.doc.path ? 'page' : undefined"
          @click="openPath(f.path)"
        >
          <Icon name="file" :size="15" /><span>{{ f.name }}</span>
        </button>
        <p v-if="state.settings.workspace && query && !files.length" class="empty">{{ t('sidebar.noMatch') }}</p>
      </div>

      <div v-if="state.imported.length" class="group">
        <span class="label">{{ t('sidebar.imported') }}</span>
        <button
          v-for="n in state.imported"
          :key="n.url + n.name"
          type="button"
          class="item"
          :title="n.url"
          :disabled="!n.path"
          @click="n.path && openPath(n.path)"
        >
          <Icon name="link" :size="15" /><span>{{ n.name }}</span>
        </button>
      </div>

      <div v-if="!state.settings.workspace && recent.length && platform.kind === 'tauri'" class="group">
        <span class="label">{{ t('sidebar.recentFiles') }}</span>
        <button v-for="p in recent" :key="p" type="button" class="item" :title="p" :aria-current="p === state.doc.path ? 'page' : undefined" @click="openPath(p)">
          <Icon name="file" :size="15" /><span>{{ baseName(p) }}</span>
        </button>
      </div>

      <div v-if="state.recentExports.length" class="card">
        <span class="card-title">{{ t('sidebar.recentExports') }}</span>
        <button v-for="e in state.recentExports" :key="e.path + e.at" type="button" class="export" :title="e.path" @click="platform.openPath(e.path)">
          {{ e.name }} · {{ ago(e.at) }}
        </button>
      </div>
    </template>

    <nav v-else class="outline" :aria-label="t('sidebar.outline')">
      <p v-if="!state.analysis.outline.length" class="empty">{{ t('sidebar.noHeadings') }}</p>
      <button
        v-for="(h, i) in state.analysis.outline"
        :key="h.id"
        type="button"
        class="item"
        :style="{ paddingLeft: `${8 + (h.level - minDepth) * 14}px` }"
        @click="state.editor?.revealHeading(i, h.id)"
      >
        <span v-if="h.number && state.settings.export.numberHeadings" class="num">{{ h.number }}</span>
        <span>{{ h.text }}</span>
      </button>
    </nav>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 260px;
  flex: none;
  min-width: 0;
  background: var(--sidebar);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 16px 12px;
  overflow-y: auto;
  box-sizing: border-box;
}
.tabs {
  display: flex;
  gap: 6px;
}
.tabs button {
  flex: 1;
  height: 34px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--text-2);
  font-size: 13px;
}
.tabs button[aria-selected='true'] {
  background: var(--accent-soft);
  color: var(--accent-ink);
  font-weight: 600;
}
.search {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  color: var(--text-2);
}
.search input {
  border: 0;
  outline: 0;
  background: transparent;
  font: inherit;
  font-size: 13px;
  min-width: 0;
  flex: 1;
  color: var(--text);
}
.group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-2);
  padding: 0 8px 6px;
}
.link-btn {
  border: 0;
  background: transparent;
  color: var(--text-3);
  padding: 2px;
  border-radius: 4px;
}
.item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  text-align: left;
  min-width: 0;
}
.item span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item svg {
  flex: none;
  color: var(--text-2);
}
.item:hover:not(:disabled) {
  background: var(--hover);
}
.item[aria-current='page'] {
  background: var(--surface);
  box-shadow: 0 0 0 1px var(--border);
  font-weight: 600;
}
.item[aria-current='page'] svg {
  color: var(--accent);
}
.item:disabled {
  color: var(--text-2);
}
.empty {
  margin: 0;
  padding: 4px 8px;
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.5;
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: flex-start;
}
.card {
  margin-top: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px;
  border-radius: 10px;
  background: var(--surface);
  border: 1px solid var(--border);
}
.card-title {
  font-size: 12px;
  font-weight: 600;
}
.export {
  border: 0;
  background: transparent;
  padding: 0;
  text-align: left;
  font-size: 12px;
  color: var(--text-2);
}
.export:hover {
  color: var(--accent);
}
.outline {
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.num {
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
  flex: none;
}
</style>
