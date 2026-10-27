<script setup lang="ts">
// Markdown source editor (CodeMirror 6), used by the Split and Source modes.
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { languages } from '@codemirror/language-data'
import { EditorSelection, EditorState } from '@codemirror/state'
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  placeholder,
} from '@codemirror/view'
import { tags } from '@lezer/highlight'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { applyFormat, type FormatCommand } from '../lib/format'
import { headingLines } from '../lib/analyze'
import { platform } from '../platform'
import { reportError, state, type EditorHandle } from '../store'

const props = defineProps<{ modelValue: string; revision: number }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const host = ref<HTMLDivElement>()
let view: EditorView | null = null

const highlight = HighlightStyle.define([
  { tag: tags.heading, color: 'var(--md-heading)', fontWeight: '600' },
  { tag: [tags.strong], fontWeight: '700' },
  { tag: [tags.emphasis], fontStyle: 'italic' },
  { tag: [tags.strikethrough], textDecoration: 'line-through' },
  { tag: [tags.link, tags.url], color: 'var(--accent)' },
  { tag: [tags.monospace], color: 'var(--md-code)' },
  { tag: [tags.quote], color: 'var(--text-2)', fontStyle: 'italic' },
  { tag: [tags.list, tags.processingInstruction, tags.meta], color: 'var(--md-mark)' },
  { tag: [tags.keyword, tags.controlKeyword], color: 'var(--md-keyword)' },
  { tag: [tags.string], color: 'var(--md-string)' },
  { tag: [tags.comment], color: 'var(--text-3)' },
  { tag: [tags.number, tags.bool], color: 'var(--md-number)' },
])

function run(cmd: FormatCommand) {
  if (!view) return
  const { from, to } = view.state.selection.main
  const e = applyFormat(view.state.doc.toString(), from, to, cmd)
  view.dispatch({
    changes: { from: e.from, to: e.to, insert: e.insert },
    selection: EditorSelection.single(e.anchor, e.head),
    scrollIntoView: true,
  })
  view.focus()
}

function insertMarkdown(md: string) {
  if (!view) return
  const { from, to } = view.state.selection.main
  view.dispatch({
    changes: { from, to, insert: md },
    selection: EditorSelection.cursor(from + md.length),
    scrollIntoView: true,
  })
  view.focus()
}

const handle: EditorHandle = {
  command: run,
  insertMarkdown,
  focus: () => view?.focus(),
  revealHeading(index) {
    if (!view) return
    const line = headingLines(view.state.doc.toString())[index]
    if (line === undefined) return
    const pos = view.state.doc.line(line + 1).from
    view.dispatch({ selection: EditorSelection.cursor(pos), effects: EditorView.scrollIntoView(pos, { y: 'start', yMargin: 24 }) })
    view.focus()
  },
}

function reportCursor(s: EditorState) {
  const pos = s.selection.main.head
  const line = s.doc.lineAt(pos)
  state.cursor = { line: line.number, col: pos - line.from + 1 }
}

async function insertImages(files: File[], at: number) {
  for (const file of files) {
    try {
      const link = await platform.saveAsset(state.doc.dir, file)
      const alt = file.name.replace(/\.[^.]+$/, '')
      view?.dispatch({ changes: { from: at, insert: `![${alt}](${link})\n` } })
    } catch (e) {
      reportError(e)
    }
  }
}

function create(doc: string) {
  view?.destroy()
  view = new EditorView({
    parent: host.value!,
    state: EditorState.create({
      doc,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightActiveLine(),
        drawSelection(),
        history(),
        EditorView.lineWrapping,
        markdown({ base: markdownLanguage, codeLanguages: languages }),
        syntaxHighlighting(highlight),
        placeholder('# …'),
        keymap.of([
          { key: 'Mod-b', run: () => (run('bold'), true) },
          { key: 'Mod-i', run: () => (run('italic'), true) },
          { key: 'Mod-k', run: () => (run('link'), true) },
          { key: 'Mod-e', run: () => (run('code'), true) },
          indentWithTab,
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) emit('update:modelValue', u.state.doc.toString())
          if (u.docChanged || u.selectionSet) reportCursor(u.state)
        }),
        EditorView.domEventHandlers({
          paste(e, v) {
            const files = [...(e.clipboardData?.files ?? [])].filter((f) => f.type.startsWith('image/'))
            if (!files.length) return false
            e.preventDefault()
            void insertImages(files, v.state.selection.main.head)
            return true
          },
          drop(e, v) {
            const files = [...(e.dataTransfer?.files ?? [])].filter((f) => f.type.startsWith('image/'))
            if (!files.length) return false
            e.preventDefault()
            const pos = v.posAtCoords({ x: e.clientX, y: e.clientY }) ?? v.state.selection.main.head
            void insertImages(files, pos)
            return true
          },
        }),
      ],
    }),
  })
  reportCursor(view.state)
}

onMounted(() => {
  create(props.modelValue)
  state.editor = handle
})

// A new document (open, import, new note) replaces the editor state so undo
// history does not leak between notes.
watch(
  () => props.revision,
  () => create(props.modelValue),
)

// Content changed elsewhere (for example the rich editor in another mode).
watch(
  () => props.modelValue,
  (value) => {
    if (view && value !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } })
    }
  },
)

onBeforeUnmount(() => {
  if (state.editor === handle) state.editor = null
  view?.destroy()
  view = null
})
</script>

<template>
  <div ref="host" class="source-editor" aria-label="Markdown source"></div>
</template>

<style scoped>
.source-editor {
  height: 100%;
  min-height: 0;
  background: var(--surface-2);
}
.source-editor :deep(.cm-editor) {
  height: 100%;
  font-size: calc(var(--editor-font-size) - 2px);
  color: var(--text-2);
  background: transparent;
}
.source-editor :deep(.cm-editor.cm-focused) {
  outline: none;
}
.source-editor :deep(.cm-scroller) {
  font-family: var(--font-mono);
  line-height: 1.75;
  padding: 16px 0;
}
.source-editor :deep(.cm-content) {
  caret-color: var(--accent);
  padding-right: 24px;
}
.source-editor :deep(.cm-cursor) {
  border-left: 2px solid var(--accent);
}
.source-editor :deep(.cm-gutters) {
  background: transparent;
  border: 0;
  color: var(--text-3);
  min-width: 48px;
}
.source-editor :deep(.cm-lineNumbers .cm-gutterElement) {
  padding: 0 14px 0 8px;
}
.source-editor :deep(.cm-activeLine),
.source-editor :deep(.cm-activeLineGutter) {
  background: var(--active-line);
}
.source-editor :deep(.cm-selectionBackground),
.source-editor :deep(.cm-focused .cm-selectionBackground) {
  background: var(--selection) !important;
}
</style>
