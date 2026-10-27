<script setup lang="ts">
import { LANGUAGES, lang, t } from '../i18n'
import { platform } from '../platform'
import { state, updateSettings } from '../store'
import type { Lang } from '../types'
</script>

<template>
  <footer class="status">
    <span class="hide-sm">{{ t('status.encoding') }}</span>
    <span>{{ t('status.words', { words: state.analysis.stats.words, minutes: state.analysis.stats.readingMinutes }) }}</span>
    <span v-if="state.mode === 'source' || state.mode === 'split'" class="hide-sm">{{ t('status.position', state.cursor) }}</span>
    <span class="right">
      <span class="dot" :class="{ web: platform.kind === 'web' }" aria-hidden="true"></span>
      {{ platform.kind === 'tauri' ? t('status.offline') : t('status.web') }}
    </span>
    <span class="langs hide-sm">
      <button
        v-for="l in LANGUAGES"
        :key="l.code"
        type="button"
        :aria-pressed="lang === l.code"
        @click="updateSettings({ language: l.code as Lang })"
      >
        {{ l.label }}
      </button>
    </span>
  </footer>
</template>

<style scoped>
.status {
  display: flex;
  flex-wrap: wrap;
  gap: 18px;
  align-items: center;
  padding: 6px 16px;
  padding-bottom: max(6px, env(safe-area-inset-bottom));
  background: var(--surface);
  border-top: 1px solid var(--border);
  font-size: 12px;
  color: var(--text-2);
}
.right {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 6px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--success);
}
.dot.web {
  background: var(--accent);
}
.langs {
  display: flex;
  gap: 2px;
}
.langs button {
  border: 0;
  background: transparent;
  color: var(--text-2);
  font-size: 12px;
  padding: 2px 4px;
  border-radius: 4px;
}
.langs button + button::before {
  content: '· ';
  color: var(--text-3);
}
.langs button[aria-pressed='true'] {
  color: var(--text);
  font-weight: 600;
}
@media (max-width: 720px) {
  .hide-sm {
    display: none;
  }
}
</style>
