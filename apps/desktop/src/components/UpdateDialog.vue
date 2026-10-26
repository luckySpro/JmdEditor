<script setup lang="ts">
// Optional or required update. A required one cannot be closed: the only ways
// out are updating here or installing from the download page.
import { computed } from 'vue'

import { formatBytes, t } from '../i18n'
import { platform } from '../platform'
import { closeUpdate, dirty, installUpdate, RELEASES_URL, state, updateRequired } from '../store'
import BaseDialog from './BaseDialog.vue'

const status = computed(() => state.update.status)
const version = computed(() => (status.value && 'version' in status.value ? status.value.version : ''))
const notes = computed(() => (status.value && 'notes' in status.value ? status.value.notes.trim() : ''))
const current = computed(() => status.value?.current ?? state.info?.version ?? '')

const progressText = computed(() => {
  const p = state.update.progress
  if (!p) return ''
  if (p.installing) return t('update.installing')
  const percent = p.total ? `${Math.min(100, Math.round(((p.received ?? 0) / p.total) * 100))}%` : formatBytes(p.received ?? 0)
  return t('update.downloading', { percent })
})
const progressRatio = computed(() => {
  const p = state.update.progress
  if (!p) return 0
  if (p.installing) return 1
  return p.total ? Math.min(1, (p.received ?? 0) / p.total) : 0
})
</script>

<template>
  <BaseDialog
    :title="updateRequired ? t('update.requiredTitle', { version }) : t('update.optionalTitle', { version })"
    icon="download"
    width="520px"
    :dismissible="!updateRequired && !state.update.installing"
    @close="closeUpdate"
  >
    <template #lead>
      <p>{{ updateRequired ? t('update.requiredLead', { current }) : t('update.optionalLead', { current }) }}</p>
    </template>
    <div class="update">
      <template v-if="notes">
        <h2>{{ t('update.notes') }}</h2>
        <p class="notes">{{ notes }}</p>
      </template>
      <div v-if="state.update.installing || state.update.progress" class="progress" role="status">
        <div class="bar"><span :style="{ width: `${progressRatio * 100}%` }" /></div>
        <p>{{ progressText }}</p>
      </div>
      <p v-if="state.update.error" class="error" role="alert">{{ state.update.error }}</p>
    </div>
    <template #footer>
      <button v-if="!updateRequired" type="button" class="btn secondary" :disabled="state.update.installing" @click="closeUpdate">
        {{ t('update.later') }}
      </button>
      <span class="spacer" />
      <button type="button" class="btn secondary" @click="platform.openUrl(RELEASES_URL)">{{ t('update.download') }}</button>
      <button
        v-if="!platform.mobile"
        type="button"
        class="btn primary"
        autofocus
        :disabled="state.update.installing"
        @click="installUpdate"
      >
        {{ dirty ? t('update.saveAndInstall') : t('update.install') }}
      </button>
    </template>
  </BaseDialog>
</template>

<style scoped>
.update {
  padding: 16px 24px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-2);
}
.notes {
  margin: 0;
  white-space: pre-wrap;
  font-size: 14px;
  line-height: 1.6;
  max-height: 240px;
  overflow: auto;
}
.progress p {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--text-2);
}
.bar {
  height: 6px;
  border-radius: 3px;
  background: var(--surface-3);
  overflow: hidden;
}
.bar span {
  display: block;
  height: 100%;
  background: var(--accent);
  transition: width 0.2s ease-out;
}
.error {
  margin: 0;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--danger-soft);
  color: var(--danger);
  font-size: 13px;
  line-height: 1.5;
}
.spacer {
  flex: 1;
}
</style>
