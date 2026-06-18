<script setup>
import { ref, onMounted, onUnmounted, shallowRef } from 'vue'
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import { defaultKeymap, indentWithTab, history, historyKeymap } from '@codemirror/commands'
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching, indentOnInput } from '@codemirror/language'
import { closeBrackets, closeBracketsKeymap, autocompletion, completionKeymap } from '@codemirror/autocomplete'
import { oneDark } from '@codemirror/theme-one-dark'

// --- State ---
const editorRef = ref(null)
const outputRef = ref(null)
const isRunning = ref(false)
const statusText = ref('')
const statusType = ref('info') // 'info' | 'success' | 'error'
const output = ref('')
const selectedExample = ref('hello')
const isDark = ref(false)

let editorView = null
let wasmModule = null

// --- Examples ---
const EXAMPLES = {
  hello: {
    label: 'Hello World',
    code: `main() -> () = () => {\n  print("Hello, YaoXiang!")\n}\n`,
  },
  fibonacci: {
    label: 'Fibonacci',
    code: `fib(n: Int) -> Int = () => {\n  if n <= 1 {\n    return n\n  }\n  return fib(n - 1) + fib(n - 2)\n}\n\nmain() -> () = () => {\n  let result = fib(10)\n  print("fib(10) = ", result)\n}\n`,
  },
  variables: {
    label: 'Variables',
    code: `main() -> () = () => {\n  let name = "YaoXiang"\n  let version = 1\n  let pi = 3.14\n  \n  print("Language: ", name)\n  print("Version: ", version)\n  print("Pi: ", pi)\n}\n`,
  },
  list_ops: {
    label: 'List Operations',
    code: `main() -> () = () => {\n  let numbers = [1, 2, 3, 4, 5]\n  print("List: ", numbers)\n  print("Length: ", len(numbers))\n}\n`,
  },
}

// --- Dark mode detection ---
function checkDarkMode() {
  isDark.value = document.documentElement.classList.contains('dark')
}

// --- Init CodeMirror ---
function initEditor() {
  if (!editorRef.value) return

  const extensions = [
    lineNumbers(),
    highlightActiveLine(),
    highlightActiveLineGutter(),
    history(),
    bracketMatching(),
    closeBrackets(),
    indentOnInput(),
    autocompletion(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    keymap.of([
      ...defaultKeymap,
      ...historyKeymap,
      ...closeBracketsKeymap,
      ...completionKeymap,
      indentWithTab,
      { key: 'Ctrl-Enter', run: () => { runCode(); return true } },
      { key: 'Cmd-Enter', run: () => { runCode(); return true } },
    ]),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        // Save to localStorage
        try {
          localStorage.setItem('yx-playground-code', update.state.doc.toString())
        } catch {}
      }
    }),
  ]

  if (isDark.value) {
    extensions.push(oneDark)
  }

  // Restore saved code or use default example
  let initialCode = EXAMPLES.hello.code
  try {
    const saved = localStorage.getItem('yx-playground-code')
    if (saved) initialCode = saved
  } catch {}

  const state = EditorState.create({
    doc: initialCode,
    extensions,
  })

  editorView = new EditorView({
    state,
    parent: editorRef.value,
  })
}

// --- Load Wasm ---
async function loadWasm() {
  try {
    statusText.value = 'Loading YaoXiang runtime...'
    statusType.value = 'info'

    // Use fetch + blob to load wasm-pack JS glue at runtime.
    // This avoids Rollup trying to resolve the import at build time.
    const baseUrl = import.meta.env.BASE_URL || '/'
    const wasmJsUrl = `${baseUrl}wasm/yaoxiang.js`
    const response = await fetch(wasmJsUrl)
    let jsCode = await response.text()

    // wasm-pack glue uses `new URL('yaoxiang_bg.wasm', import.meta.url)` to find .wasm.
    // In a blob URL context, import.meta.url points to the blob, so we patch it to an absolute URL.
    const wasmBgUrl = `${baseUrl}wasm/yaoxiang_bg.wasm`
    jsCode = jsCode.replace(
      /new URL\(['"]yaoxiang_bg\.wasm['"],\s*import\.meta\.url\)/g,
      `new URL('${wasmBgUrl}', window.location.origin)`
    )

    const blob = new Blob([jsCode], { type: 'application/javascript' })
    const blobUrl = URL.createObjectURL(blob)
    const wasm = await import(/* @vite-ignore */ blobUrl)
    URL.revokeObjectURL(blobUrl)

    await wasm.default()
    wasm.init_panic_hook()
    wasmModule = wasm

    statusText.value = 'Ready'
    statusType.value = 'success'
  } catch (e) {
    statusText.value = 'Failed to load Wasm — run scripts/build-wasm.sh first'
    statusType.value = 'error'
    console.error('Wasm load error:', e)
  }
}

// --- Run Code ---
async function runCode() {
  if (!wasmModule || isRunning.value) return

  isRunning.value = true
  output.value = ''
  statusText.value = 'Running...'
  statusType.value = 'info'

  const code = editorView.state.doc.toString()

  try {
    const start = performance.now()
    const result = wasmModule.run_code(code)
    const elapsed = (performance.now() - start).toFixed(1)

    output.value = result || '(no output)'
    statusText.value = `Completed in ${elapsed}ms`
    statusType.value = result.includes('Error') ? 'error' : 'success'
  } catch (e) {
    output.value = `Wasm Error: ${e.message || e}`
    statusText.value = 'Error'
    statusType.value = 'error'
  } finally {
    isRunning.value = false
  }
}

// --- Load example ---
function loadExample(key) {
  if (!editorView) return
  const example = EXAMPLES[key]
  if (!example) return

  selectedExample.value = key
  editorView.dispatch({
    changes: { from: 0, to: editorView.state.doc.length, insert: example.code },
  })
  output.value = ''
  statusText.value = ''
}

// --- Lifecycle ---
onMounted(() => {
  checkDarkMode()
  // Watch for dark mode changes
  const observer = new MutationObserver(checkDarkMode)
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })

  initEditor()
  loadWasm()
})

onUnmounted(() => {
  if (editorView) editorView.destroy()
})
</script>

<template>
  <div class="playground-container">
    <!-- Header -->
    <div class="playground-header">
      <h1 class="playground-title">YaoXiang Playground</h1>
      <p class="playground-subtitle">
        在浏览器中运行爻象代码 · 按
        <kbd>Ctrl+Enter</kbd> 运行
      </p>
    </div>

    <!-- Toolbar -->
    <div class="playground-toolbar">
      <div class="toolbar-left">
        <select
          class="example-select"
          :value="selectedExample"
          @change="loadExample($event.target.value)"
        >
          <option v-for="(ex, key) in EXAMPLES" :key="key" :value="key">
            {{ ex.label }}
          </option>
        </select>
      </div>
      <div class="toolbar-right">
        <button
          class="run-button"
          :disabled="isRunning || !wasmModule"
          @click="runCode"
        >
          <span v-if="isRunning" class="spinner"></span>
          <span v-else>▶</span>
          {{ isRunning ? 'Running...' : 'Run' }}
        </button>
      </div>
    </div>

    <!-- Editor + Output -->
    <div class="playground-panels">
      <div class="panel editor-panel">
        <div class="panel-header">
          <span class="panel-label">Editor</span>
        </div>
        <div ref="editorRef" class="editor-container"></div>
      </div>

      <div class="panel output-panel">
        <div class="panel-header">
          <span class="panel-label">Output</span>
        </div>
        <pre class="output-content">{{ output || 'Click "Run" or press Ctrl+Enter to execute...' }}</pre>
      </div>
    </div>

    <!-- Status bar -->
    <div class="status-bar" :class="`status-${statusType}`">
      {{ statusText }}
    </div>
  </div>
</template>

<style scoped>
.playground-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 1.5rem;
}

.playground-header {
  text-align: center;
  margin-bottom: 1.5rem;
}

.playground-title {
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 0.5rem 0;
}

.playground-subtitle {
  color: var(--vp-c-text-2, #666);
  margin: 0;
  font-size: 0.95rem;
}

.playground-subtitle kbd {
  background: var(--vp-c-bg-soft, #f0f0f0);
  border: 1px solid var(--vp-c-border, #ddd);
  border-radius: 4px;
  padding: 0.1em 0.4em;
  font-size: 0.85em;
  font-family: monospace;
}

/* Toolbar */
.playground-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
  gap: 1rem;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.example-select {
  background: var(--vp-c-bg-soft, #f5f5f5);
  color: var(--vp-c-text-1, #333);
  border: 1px solid var(--vp-c-border, #ddd);
  border-radius: 6px;
  padding: 0.4rem 0.75rem;
  font-size: 0.9rem;
  cursor: pointer;
  outline: none;
}

.example-select:focus {
  border-color: var(--vp-c-brand, #6366f1);
}

.run-button {
  background: #22c55e;
  color: white;
  border: none;
  border-radius: 6px;
  padding: 0.5rem 1.25rem;
  font-size: 0.95rem;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.4rem;
  transition: background 0.2s;
}

.run-button:hover:not(:disabled) {
  background: #16a34a;
}

.run-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Panels */
.playground-panels {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
  min-height: 400px;
}

@media (max-width: 768px) {
  .playground-panels {
    grid-template-columns: 1fr;
  }
}

.panel {
  border: 1px solid var(--vp-c-border, #ddd);
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.panel-header {
  background: var(--vp-c-bg-soft, #f5f5f5);
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--vp-c-border, #ddd);
}

.panel-label {
  font-size: 0.8rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--vp-c-text-2, #666);
}

/* Editor */
.editor-container {
  flex: 1;
  overflow: auto;
}

.editor-container :deep(.cm-editor) {
  height: 100%;
  font-size: 14px;
}

.editor-container :deep(.cm-scroller) {
  overflow: auto;
}

/* Output */
.output-content {
  flex: 1;
  margin: 0;
  padding: 1rem;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  font-size: 14px;
  line-height: 1.6;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--vp-c-text-1, #333);
  min-height: 300px;
}

/* Status bar */
.status-bar {
  margin-top: 0.75rem;
  padding: 0.4rem 0.75rem;
  border-radius: 6px;
  font-size: 0.85rem;
  text-align: center;
}

.status-info {
  background: var(--vp-c-bg-soft, #f0f0f0);
  color: var(--vp-c-text-2, #666);
}

.status-success {
  background: #dcfce7;
  color: #166534;
}

.status-error {
  background: #fef2f2;
  color: #991b1b;
}

:root.dark .status-success {
  background: #052e16;
  color: #86efac;
}

:root.dark .status-error {
  background: #450a0a;
  color: #fca5a5;
}
</style>
