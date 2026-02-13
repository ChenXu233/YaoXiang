<template>
  <div ref="container" class="mermaid-container">
    <div v-if="error" class="mermaid-error">{{ error }}</div>
  </div>
</template>

<script setup>
import { ref, onMounted, watch } from 'vue'
import mermaid from 'mermaid'

const props = defineProps({
  code: {
    type: String,
    required: true,
  },
  config: {
    type: String,
    default: '',
  },
})

const container = ref(null)
const error = ref(null)
const id = `mermaid-${Math.random().toString(36).slice(2, 9)}`

// 初始化 mermaid
mermaid.initialize({
  startOnLoad: false,
  theme: 'default',
  securityLevel: 'loose',
  fontFamily: 'inherit',
})

const render = async () => {
  if (!container.value) return

  error.value = null
  const decodedCode = decodeURIComponent(props.code)
  container.value.innerHTML = decodedCode

  try {
    const { svg } = await mermaid.render(`${id}-graph`, decodedCode)
    container.value.innerHTML = svg
  } catch (e) {
    error.value = e.message
    console.error('Mermaid render error:', e)
  }
}

onMounted(() => {
  render()
})

watch(() => props.code, () => {
  render()
})
</script>

<style scoped>
.mermaid-container {
  text-align: center;
  margin: 1rem 0;
  overflow-x: auto;
}

.mermaid-container :deep(svg) {
  max-width: 100%;
  height: auto;
}

.mermaid-error {
  color: #ef4444;
  padding: 1rem;
  background: #fef2f2;
  border-radius: 0.5rem;
  font-size: 0.875rem;
}
</style>
