<script setup lang="ts">
// Modal shell: backdrop, Escape to close, focus kept inside while open.
// `dismissible: false` hides the close button and ignores Escape and the backdrop.
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

import { t } from '../i18n'
import Icon from './Icon.vue'

const props = withDefaults(
  defineProps<{ title: string; icon?: string; width?: string; labelledBy?: string; dismissible?: boolean }>(),
  {
    icon: '',
    width: '880px',
    labelledBy: '',
    dismissible: true,
  },
)
const emit = defineEmits<{ close: [] }>()
const panel = ref<HTMLElement>()
const titleId = `dlg-${Math.random().toString(36).slice(2, 8)}`
let previous: Element | null = null

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.stopPropagation()
    e.preventDefault()
    if (props.dismissible) emit('close')
  } else if (e.key === 'Tab' && panel.value) {
    const items = [...panel.value.querySelectorAll<HTMLElement>('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')].filter(
      (el) => !el.hasAttribute('disabled') && el.offsetParent !== null,
    )
    if (!items.length) return
    const first = items[0]
    const last = items[items.length - 1]
    if (e.shiftKey && document.activeElement === first) {
      last.focus()
      e.preventDefault()
    } else if (!e.shiftKey && document.activeElement === last) {
      first.focus()
      e.preventDefault()
    }
  }
}

onMounted(async () => {
  previous = document.activeElement
  await nextTick()
  const auto = panel.value?.querySelector<HTMLElement>('[autofocus], input, select, button')
  auto?.focus()
})
onBeforeUnmount(() => (previous as HTMLElement | null)?.focus?.())
</script>

<template>
  <div class="backdrop" @mousedown.self="props.dismissible && emit('close')">
    <section
      ref="panel"
      class="dialog"
      role="dialog"
      aria-modal="true"
      :aria-labelledby="props.labelledBy || titleId"
      :style="{ maxWidth: props.width }"
      @keydown="onKey"
    >
      <header class="dialog-head">
        <div v-if="props.icon" class="dialog-icon"><Icon :name="props.icon" :size="20" /></div>
        <div class="dialog-heading">
          <slot name="title"><h1 :id="titleId">{{ props.title }}</h1></slot>
          <slot name="lead" />
        </div>
        <slot name="head-extra" />
        <button v-if="props.dismissible" type="button" class="icon-btn ghost" :aria-label="t('action.close')" @click="emit('close')">
          <Icon name="close" :size="18" />
        </button>
      </header>
      <div class="dialog-body"><slot /></div>
      <footer v-if="$slots.footer" class="dialog-foot"><slot name="footer" /></footer>
    </section>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px 16px;
  background: var(--backdrop);
  animation: fade 0.12s ease-out;
}
.dialog {
  width: 100%;
  max-height: calc(100vh - 48px);
  display: flex;
  flex-direction: column;
  background: var(--surface);
  color: var(--text);
  border-radius: 14px;
  box-shadow: var(--shadow-lg);
  overflow: hidden;
}
.dialog-head {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 20px 20px 16px 24px;
  border-bottom: 1px solid var(--border);
}
.dialog-icon {
  width: 40px;
  height: 40px;
  flex: none;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--accent-soft);
  color: var(--accent);
}
.dialog-heading {
  flex: 1;
  min-width: 0;
}
.dialog-heading :deep(h1) {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  line-height: 1.3;
}
.dialog-heading :deep(p) {
  margin: 4px 0 0;
  color: var(--text-2);
  font-size: 13px;
  line-height: 1.5;
}
.dialog-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
.dialog-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  padding: 14px 24px;
  border-top: 1px solid var(--border);
  background: var(--surface-2);
}
@keyframes fade {
  from {
    opacity: 0;
  }
}
@media (max-width: 640px) {
  .backdrop {
    padding: 0;
    align-items: flex-end;
  }
  .dialog {
    max-height: 92vh;
    border-radius: 16px 16px 0 0;
  }
}
</style>
