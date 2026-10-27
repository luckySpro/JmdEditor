<script setup lang="ts">
import { ref } from 'vue'

import { t } from '../i18n'
import type { FormatCommand } from '../lib/format'
import { platform } from '../platform'
import { reportError, state } from '../store'
import Icon from './Icon.vue'

const picker = ref<HTMLInputElement>()
const headingOpen = ref(false)

function run(cmd: FormatCommand) {
  headingOpen.value = false
  state.editor?.command(cmd)
}

async function onImages(e: Event) {
  const input = e.target as HTMLInputElement
  for (const file of [...(input.files ?? [])]) {
    try {
      const link = await platform.saveAsset(state.doc.dir, file)
      state.editor?.insertMarkdown(`![${file.name.replace(/\.[^.]+$/, '')}](${link})\n`)
    } catch (err) {
      reportError(err)
    }
  }
  input.value = ''
}

const headings: { cmd: FormatCommand; label: () => string }[] = [
  { cmd: 'paragraph', label: () => t('toolbar.paragraph') },
  { cmd: 'h1', label: () => t('toolbar.heading', { n: 1 }) },
  { cmd: 'h2', label: () => t('toolbar.heading', { n: 2 }) },
  { cmd: 'h3', label: () => t('toolbar.heading', { n: 3 }) },
  { cmd: 'h4', label: () => t('toolbar.heading', { n: 4 }) },
]
</script>

<template>
  <div v-if="state.mode === 'read'" class="toolbar note">{{ t('toolbar.readOnly') }}</div>
  <div v-else role="toolbar" :aria-label="t('toolbar.label')" class="toolbar">
    <div class="menu-wrap">
      <button type="button" class="tb text" :aria-expanded="headingOpen" @click="headingOpen = !headingOpen">
        {{ t('toolbar.heading', { n: '' }).trim() }} <Icon name="chevron" :size="12" />
      </button>
      <div v-if="headingOpen" class="popover" role="menu" @mouseleave="headingOpen = false">
        <button v-for="h in headings" :key="h.cmd" type="button" role="menuitem" :class="['pop-item', h.cmd]" @click="run(h.cmd)">
          {{ h.label() }}
        </button>
      </div>
    </div>
    <span class="sep" aria-hidden="true"></span>
    <button type="button" class="tb b" :aria-label="t('toolbar.bold')" :title="t('toolbar.bold') + ' (Ctrl+B)'" @click="run('bold')">B</button>
    <button type="button" class="tb i" :aria-label="t('toolbar.italic')" :title="t('toolbar.italic') + ' (Ctrl+I)'" @click="run('italic')">I</button>
    <button type="button" class="tb s" :aria-label="t('toolbar.strike')" :title="t('toolbar.strike')" @click="run('strike')">S</button>
    <button type="button" class="tb mono" :aria-label="t('toolbar.code')" :title="t('toolbar.code')" @click="run('code')">&lt;/&gt;</button>
    <span class="sep" aria-hidden="true"></span>
    <button type="button" class="tb" :aria-label="t('toolbar.bullet')" :title="t('toolbar.bullet')" @click="run('bullet')"><Icon name="list" /></button>
    <button type="button" class="tb" :aria-label="t('toolbar.ordered')" :title="t('toolbar.ordered')" @click="run('ordered')"><Icon name="ordered" /></button>
    <button type="button" class="tb" :aria-label="t('toolbar.task')" :title="t('toolbar.task')" @click="run('task')"><Icon name="task" /></button>
    <button type="button" class="tb quote" :aria-label="t('toolbar.quote')" :title="t('toolbar.quote')" @click="run('quote')">“</button>
    <span class="sep" aria-hidden="true"></span>
    <button type="button" class="tb" :aria-label="t('toolbar.link')" :title="t('toolbar.link') + ' (Ctrl+K)'" @click="run('link')"><Icon name="link" /></button>
    <button type="button" class="tb" :aria-label="t('toolbar.image')" :title="t('toolbar.image')" @click="picker?.click()"><Icon name="image" /></button>
    <button type="button" class="tb" :aria-label="t('toolbar.table')" :title="t('toolbar.table')" @click="run('table')"><Icon name="table" /></button>
    <button type="button" class="tb" :aria-label="t('toolbar.codeBlock')" :title="t('toolbar.codeBlock')" @click="run('codeBlock')"><Icon name="code" /></button>
    <button type="button" class="tb text" :title="t('toolbar.math')" @click="run('math')">Σ {{ t('toolbar.math') }}</button>
    <span class="plugins">{{ t('toolbar.plugins') }}</span>
    <input ref="picker" type="file" accept="image/*" multiple hidden @change="onImages" />
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px;
  padding: 6px 12px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  color: var(--text-2);
  font-size: 13px;
  min-height: 44px;
  box-sizing: border-box;
}
.toolbar.note {
  color: var(--text-3);
  font-size: 12px;
}
.tb {
  min-width: 32px;
  height: 32px;
  padding: 0 6px;
  border: 0;
  background: transparent;
  border-radius: 6px;
  color: var(--text-2);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  font-size: 13px;
}
.tb:hover,
.pop-item:hover {
  background: var(--hover);
  color: var(--text);
}
.tb.text {
  padding: 0 10px;
}
.tb.b {
  font-weight: 700;
}
.tb.i {
  font-style: italic;
  font-family: Georgia, serif;
}
.tb.s {
  text-decoration: line-through;
}
.tb.mono {
  font-family: var(--font-mono);
  font-size: 12px;
}
.tb.quote {
  font-family: Georgia, serif;
  font-size: 20px;
}
.sep {
  width: 1px;
  height: 20px;
  background: var(--border);
  margin: 0 6px;
}
.plugins {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-3);
}
.menu-wrap {
  position: relative;
}
.popover {
  position: absolute;
  top: 36px;
  left: 0;
  z-index: 20;
  min-width: 160px;
  padding: 6px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
}
.pop-item {
  text-align: left;
  border: 0;
  background: transparent;
  padding: 8px 10px;
  border-radius: 6px;
  color: var(--text);
  font-size: 14px;
}
.pop-item.h1 {
  font-size: 20px;
  font-weight: 700;
}
.pop-item.h2 {
  font-size: 17px;
  font-weight: 600;
}
.pop-item.h3 {
  font-size: 15px;
  font-weight: 600;
}
@media (max-width: 1100px) {
  .plugins {
    display: none;
  }
}
@media (max-width: 720px) {
  .toolbar {
    flex-wrap: nowrap;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tb {
    flex: none;
  }
  .menu-wrap {
    position: static;
  }
}
</style>
