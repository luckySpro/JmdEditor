<script setup lang="ts">
import { computed, ref } from 'vue'

import { t } from '../i18n'
import { platform } from '../platform'
import { dirty, docName, isDark, newNote, openFolder, openPicker, save, setMode, state, updateSettings } from '../store'
import type { EditorMode } from '../types'
import Icon from './Icon.vue'

const menuOpen = ref(false)
const modes: EditorMode[] = platform.mobile ? ['rich', 'source', 'read'] : ['rich', 'split', 'source', 'read']

const subtitle = computed(() => {
  const where = state.doc.dir ?? (platform.kind === 'web' ? t('app.inBrowser') : '')
  const status = state.saving ? t('app.saving') : dirty.value ? (state.doc.path ? t('app.unsaved') : t('app.notSaved')) : t('app.saved')
  return [where, status].filter(Boolean).join(' · ')
})

function toggleTheme() {
  updateSettings({ theme: isDark.value ? 'light' : 'dark' })
}

function pick(fn: () => unknown) {
  menuOpen.value = false
  void fn()
}
</script>

<template>
  <header class="app-header">
    <div class="doc">
      <button
        v-if="platform.folders"
        type="button"
        class="icon-btn ghost"
        :aria-label="t('action.sidebar')"
        :aria-pressed="state.sidebarOpen"
        @click="state.sidebarOpen = !state.sidebarOpen"
      >
        <Icon name="sidebar" />
      </button>
      <div class="menu-wrap">
        <button type="button" class="logo" :aria-label="t('action.menu')" :aria-expanded="menuOpen" @click="menuOpen = !menuOpen">J</button>
        <div v-if="menuOpen" class="popover" role="menu" @mouseleave="menuOpen = false">
          <button type="button" role="menuitem" @click="pick(newNote)"><Icon name="plus" />{{ t('action.new') }}<kbd>Ctrl N</kbd></button>
          <button type="button" role="menuitem" @click="pick(openPicker)"><Icon name="file" />{{ t('action.open') }}<kbd>Ctrl O</kbd></button>
          <button v-if="platform.folders" type="button" role="menuitem" @click="pick(() => openFolder())"><Icon name="folder" />{{ t('action.openFolder') }}</button>
          <hr />
          <button type="button" role="menuitem" @click="pick(() => save())"><Icon name="save" />{{ t('action.save') }}<kbd>Ctrl S</kbd></button>
          <button type="button" role="menuitem" @click="pick(() => save(true))"><Icon name="save" />{{ t('action.saveAs') }}<kbd>Ctrl ⇧ S</kbd></button>
        </div>
      </div>
      <div class="names">
        <span class="name" :title="state.doc.path ?? docName">{{ docName }}<span v-if="dirty" class="dot" aria-hidden="true">•</span></span>
        <span class="sub">{{ subtitle }}</span>
      </div>
    </div>

    <div class="segmented" role="group" :aria-label="t('mode.label')">
      <button v-for="m in modes" :key="m" type="button" :aria-pressed="state.mode === m" @click="setMode(m)">
        {{ t(`mode.${m}`) }}
      </button>
    </div>

    <div class="actions">
      <button type="button" class="btn secondary hide-sm" @click="state.dialog = 'import'">
        <Icon name="link" /><span>{{ t('action.import') }}</span>
      </button>
      <button type="button" class="btn primary hide-sm" @click="state.dialog = 'export'">
        <Icon name="download" /><span>{{ t('action.export') }}</span>
      </button>
      <button type="button" class="icon-btn show-sm" :aria-label="t('mobile.share')" @click="state.dialog = 'sheet'">
        <Icon name="share" />
      </button>
      <button type="button" class="icon-btn" :aria-label="t('action.theme')" @click="toggleTheme">
        <Icon :name="isDark ? 'sun' : 'moon'" />
      </button>
      <button type="button" class="icon-btn" :aria-label="t('action.settings')" @click="state.dialog = 'settings'">
        <Icon name="settings" />
      </button>
    </div>
  </header>
</template>

<style scoped>
.app-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  padding-top: max(10px, env(safe-area-inset-top));
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
.doc {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  flex: 1 1 220px;
}
.logo {
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
  font-weight: 700;
  font-size: 13px;
  flex: none;
}
.names {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.name {
  font-weight: 600;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dot {
  color: var(--accent);
  margin-left: 4px;
}
.sub {
  font-size: 12px;
  color: var(--text-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1 1 220px;
  justify-content: flex-end;
}
.menu-wrap {
  position: relative;
}
.popover {
  position: absolute;
  top: 38px;
  left: 0;
  z-index: 30;
  min-width: 240px;
  padding: 6px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
}
.popover button {
  display: flex;
  align-items: center;
  gap: 10px;
  border: 0;
  background: transparent;
  padding: 9px 10px;
  border-radius: 6px;
  color: var(--text);
  font-size: 13px;
  text-align: left;
}
.popover button:hover {
  background: var(--hover);
}
.popover kbd {
  margin-left: auto;
  font-family: inherit;
  font-size: 11px;
  color: var(--text-3);
}
.popover hr {
  border: 0;
  border-top: 1px solid var(--border);
  margin: 4px 0;
}
.show-sm {
  display: none;
}
@media (max-width: 720px) {
  .hide-sm {
    display: none;
  }
  .show-sm {
    display: inline-flex;
  }
  .segmented {
    order: 3;
    width: 100%;
  }
  .segmented button {
    flex: 1;
  }
  .doc,
  .actions {
    flex-basis: auto;
  }
}
</style>
