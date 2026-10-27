<script setup lang="ts">
import { t } from '../i18n'
import { dismissToast, state } from '../store'
import Icon from './Icon.vue'
</script>

<template>
  <div class="toasts" aria-live="polite">
    <div v-for="x in state.toasts" :key="x.id" class="toast" :class="x.kind" :role="x.kind === 'error' ? 'alert' : 'status'">
      <Icon :name="x.kind === 'error' ? 'alert' : 'check'" />
      <span class="text">{{ x.text }}</span>
      <button v-if="x.action" type="button" class="act" @click="x.action.run(); dismissToast(x.id)">{{ x.action.label }}</button>
      <button type="button" class="x" :aria-label="t('action.close')" @click="dismissToast(x.id)"><Icon name="close" :size="14" /></button>
    </div>
  </div>
</template>

<style scoped>
.toasts {
  position: fixed;
  right: 16px;
  bottom: 44px;
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: min(440px, calc(100vw - 32px));
}
.toast {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--inverse);
  color: var(--on-inverse);
  box-shadow: var(--shadow-lg);
  font-size: 13px;
  line-height: 1.4;
  animation: rise 0.15s ease-out;
}
.toast.error svg:first-child {
  color: #ff9b9b;
}
.toast.success svg:first-child {
  color: #7fd8a8;
}
.text {
  flex: 1;
}
.act {
  border: 0;
  background: transparent;
  color: #9db7ff;
  font-weight: 600;
  font-size: 13px;
}
.x {
  border: 0;
  background: transparent;
  color: inherit;
  opacity: 0.7;
  display: flex;
}
@keyframes rise {
  from {
    transform: translateY(8px);
    opacity: 0;
  }
}
</style>
