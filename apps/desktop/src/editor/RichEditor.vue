<script setup lang="ts">
// WYSIWYG editor (Milkdown Crepe). Editable in Rich mode, read-only as the
// Split preview and in the import dialog.
import { Crepe } from '@milkdown/crepe'
import '@milkdown/crepe/theme/common/style.css'
import '@milkdown/crepe/theme/classic.css'
import { editorViewCtx } from '@milkdown/kit/core'
import {
  createCodeBlockCommand,
  insertHrCommand,
  toggleEmphasisCommand,
  toggleInlineCodeCommand,
  toggleStrongCommand,
  turnIntoTextCommand,
  wrapInBlockquoteCommand,
  wrapInBulletListCommand,
  wrapInHeadingCommand,
  wrapInOrderedListCommand,
} from '@milkdown/kit/preset/commonmark'
import { insertTableCommand, toggleStrikethroughCommand } from '@milkdown/kit/preset/gfm'
import { callCommand, insert, replaceAll } from '@milkdown/kit/utils'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { t } from '../i18n'
import { snippet, type FormatCommand } from '../lib/format'
import { joinFrontMatter, splitFrontMatter } from '../lib/frontmatter'
import { platform } from '../platform'
import { reportError, state, type EditorHandle } from '../store'

const props = withDefaults(
  defineProps<{ modelValue: string; readonly?: boolean; baseDir?: string | null; register?: boolean }>(),
  { readonly: false, baseDir: null, register: true },
)
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const host = ref<HTMLDivElement>()
let crepe: Crepe | null = null
let ready = false
let frontMatter = ''
/** Milkdown's own serialisation of the loaded note; equal output is not a user edit. */
let baseline = ''
let pending: ReturnType<typeof setTimeout> | undefined

function act(fn: Parameters<Crepe['editor']['action']>[0]) {
  if (!crepe || !ready) return
  try {
    crepe.editor.action(fn)
  } catch (e) {
    console.warn('editor command failed', e)
  }
}

function command(cmd: FormatCommand) {
  const map: Partial<Record<FormatCommand, () => ReturnType<typeof callCommand>>> = {
    bold: () => callCommand(toggleStrongCommand.key),
    italic: () => callCommand(toggleEmphasisCommand.key),
    strike: () => callCommand(toggleStrikethroughCommand.key),
    code: () => callCommand(toggleInlineCodeCommand.key),
    bullet: () => callCommand(wrapInBulletListCommand.key),
    ordered: () => callCommand(wrapInOrderedListCommand.key),
    quote: () => callCommand(wrapInBlockquoteCommand.key),
    table: () => callCommand(insertTableCommand.key, { row: 3, col: 3 }),
    rule: () => callCommand(insertHrCommand.key),
    codeBlock: () => callCommand(createCodeBlockCommand.key),
    paragraph: () => callCommand(turnIntoTextCommand.key),
    h1: () => callCommand(wrapInHeadingCommand.key, 1),
    h2: () => callCommand(wrapInHeadingCommand.key, 2),
    h3: () => callCommand(wrapInHeadingCommand.key, 3),
    h4: () => callCommand(wrapInHeadingCommand.key, 4),
  }
  const fn = map[cmd]
  if (fn) act(fn())
  else {
    const md = snippet(cmd)
    if (md) act(insert(md, cmd === 'link'))
  }
  handle.focus()
}

const handle: EditorHandle = {
  command,
  insertMarkdown: (md) => act(insert(md)),
  focus() {
    act((ctx) => ctx.get(editorViewCtx).focus())
  },
  revealHeading(index) {
    const heads = host.value?.querySelectorAll('.ProseMirror > h1, .ProseMirror > h2, .ProseMirror > h3, .ProseMirror > h4, .ProseMirror > h5, .ProseMirror > h6')
    heads?.[index]?.scrollIntoView({ block: 'start', behavior: 'smooth' })
  },
}

async function upload(file: File) {
  try {
    return await platform.saveAsset(state.doc.dir, file)
  } catch (e) {
    reportError(e)
    throw e
  }
}

async function create() {
  const split = splitFrontMatter(props.modelValue)
  frontMatter = split.frontMatter
  crepe = new Crepe({
    root: host.value!,
    defaultValue: split.body,
    featureConfigs: {
      [Crepe.Feature.ImageBlock]: {
        proxyDomURL: (url: string) => platform.imageUrl(url, props.baseDir),
        onUpload: upload,
        inlineOnUpload: upload,
        blockOnUpload: upload,
      },
      [Crepe.Feature.Placeholder]: { text: t('editor.placeholder'), mode: 'doc' },
      [Crepe.Feature.LinkTooltip]: {},
    },
  })
  crepe.on((api) => {
    api.markdownUpdated((_ctx, markdown) => {
      if (props.readonly || markdown === baseline) return
      baseline = ''
      emit('update:modelValue', joinFrontMatter(frontMatter, markdown))
    })
  })
  await crepe.create()
  ready = true
  baseline = crepe.getMarkdown()
  crepe.setReadonly(props.readonly)
  if (props.register && !props.readonly) state.editor = handle
}

onMounted(() => {
  create().catch(reportError)
})

// The read-only preview follows the source editor, a little behind the typing.
watch(
  () => props.modelValue,
  (value) => {
    if (!props.readonly) return
    clearTimeout(pending)
    pending = setTimeout(() => {
      const split = splitFrontMatter(value)
      frontMatter = split.frontMatter
      act(replaceAll(split.body, true))
    }, 180)
  },
)

onBeforeUnmount(() => {
  clearTimeout(pending)
  if (state.editor === handle) state.editor = null
  ready = false
  void crepe?.destroy()
  crepe = null
})

defineExpose({ handle })
</script>

<template>
  <div ref="host" class="rich-editor" :class="{ readonly: props.readonly }"></div>
</template>

<style scoped>
.rich-editor {
  height: 100%;
  min-height: 0;
  overflow: auto;
  background: var(--paper);
}
.rich-editor :deep(.milkdown) {
  --crepe-color-background: var(--paper);
  --crepe-color-on-background: var(--text);
  --crepe-color-surface: var(--surface);
  --crepe-color-surface-low: var(--surface-2);
  --crepe-color-on-surface: var(--text);
  --crepe-color-on-surface-variant: var(--text-2);
  --crepe-color-outline: var(--border-strong);
  --crepe-color-primary: var(--accent);
  --crepe-color-secondary: var(--accent-soft);
  --crepe-color-on-secondary: var(--accent-ink);
  --crepe-color-inverse: var(--text);
  --crepe-color-on-inverse: var(--paper);
  --crepe-color-inline-code: var(--md-code);
  --crepe-color-error: var(--danger);
  --crepe-color-hover: var(--hover);
  --crepe-color-selected: var(--selection);
  --crepe-color-inline-area: var(--surface-2);
  --crepe-base-font-size: var(--editor-font-size);
  --crepe-font-title: var(--font-serif);
  --crepe-font-default: var(--font-serif);
  --crepe-font-code: var(--font-mono);
  background: transparent;
}
.rich-editor :deep(.milkdown .ProseMirror) {
  padding: 32px clamp(20px, 5vw, 56px) 96px;
  max-width: 820px;
  margin: 0 auto;
}
.rich-editor.readonly :deep(.milkdown .ProseMirror) {
  padding-top: 24px;
}
.rich-editor :deep(.milkdown h2) {
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border);
}
.rich-editor :deep(.milkdown blockquote) {
  background: var(--quote-bg);
  border-radius: 8px;
}
</style>
