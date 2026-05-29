<script setup>
import { ref, computed, onMounted } from 'vue'
import { useData } from 'vitepress'

const { site, page } = useData()

// 配置你的版本列表
// value: 对应URL中的版本路径段
// latest 的 value 设置为 'src'
// v0.1 的 value 设置为 'archive/v0.1' (根据用户要求添加 archive 前缀)
const versions = [
  { label: 'latest', value: 'src', link: '/', texts: { 'zh-CN': '🚀 latest', 'en': '🚀 latest' } },
]

const isOpen = ref(false)
const currentPath = ref('')

onMounted(() => {
  currentPath.value = window.location.pathname
})

function isEnglish() {
  if (typeof window === 'undefined') return false
  return window.location.pathname.includes('/en/') || page.value.relativePath.startsWith('en/')
}

const base = site.value.base

// 获取当前激活的版本
const currentVersion = computed(() => {
  if (typeof window === 'undefined') return versions[0]
  
  const path = currentPath.value
  
  // 检查路径中是否包含显式的版本目录
  for (const v of versions) {
    if (v.value === 'src') continue // latest (src) 作为默认 fallback
    const versionPrefix = (base + v.value).replace(/\/\/+/g, '/')
    if (path.startsWith(versionPrefix)) {
      return v
    }
  }
  
  // 如果没有匹配到 v0.1 等显式版本，则认为是 latest (src)
  return versions.find(v => v.value === 'src')
})

function getText(v) {
  return isEnglish() ? v.texts.en : v.texts['zh-CN']
}

function switchVersion(targetVersion) {
  if (typeof window === 'undefined') return
  
  let path = window.location.pathname
  const activeVer = currentVersion.value

  // 如果源版本和目标版本一样，直接返回
  if (activeVer.value === targetVersion.value) {
    isOpen.value = false
    return
  }
  
  // 替换逻辑：
  // 1. 如果当前是 latest (src)，路径可能是 /YaoXiang/src/... 或 /YaoXiang/... (生产环境可能没有src)
  //    我们需要把 'src' 替换为 targetVersion.value (v0.1)
  // 2. 如果当前是 v0.1，路径是 /YaoXiang/v0.1/...
  //    我们需要把 'v0.1' 替换为 targetVersion.value (src)
  
  // 为了准确替换，我们构建当前版本的完整前缀
  // 但是 latest 有点特殊：开发环境是 /YaoXiang/src/，生产环境可能是 /YaoXiang/
  // 这里我们假设用户提到的情况：latest 对应 'src' 目录
  
  let activePrefix = ''
  if (activeVer.value === 'src') {
    // 检查路径里是否有 src，如果有就把它作为前缀
    const possibleSrcPrefix = (base + activeVer.value).replace(/\/\/+/g, '/')
    if (path.startsWith(possibleSrcPrefix)) {
       activePrefix = possibleSrcPrefix
    } else {
       // 如果路径里没有 src (比如生产环境)，则 activePrefix 就是 base
       activePrefix = base
    }
  } else {
    activePrefix = (base + activeVer.value).replace(/\/\/+/g, '/')
  }
  
  // 构建目标前缀
  let targetPrefix = ''
  if (targetVersion.value === 'src') {
    // 目标是src，如果生产环境可能不需要src，但在本地开发需要
    // 我们假设目标就是带src的
    targetPrefix = (base + targetVersion.value).replace(/\/\/+/g, '/')
  } else {
    targetPrefix = (base + targetVersion.value).replace(/\/\/+/g, '/')
  }

  // 特殊修正：
  // 按照用户需求：/YaoXiang/src/en/ -> /YaoXiang/v0.1/en/
  // 这意味着，虽然 latest 是 src，但是 v0.1 是不包含 src 的！
  // 所以：
  // latest -> value: 'src', prefix: /YaoXiang/src/
  // v0.1   -> value: 'v0.1', prefix: /YaoXiang/v0.1/
  
  // 如果是 /YaoXiang/src/en/ (activePrefix=/YaoXiang/src/) -> targetPrefix=/YaoXiang/v0.1/
  // result: /YaoXiang/v0.1/en/ -> 正确！

  // 如果是 /YaoXiang/v0.1/en/ (activePrefix=/YaoXiang/v0.1/) -> targetPrefix=/YaoXiang/src/
  // result: /YaoXiang/src/en/ -> 正确！
  
  let newPath = path
  if (path.startsWith(activePrefix)) {
    newPath = path.replace(activePrefix, targetPrefix)
  } else if (path.startsWith(base)) {
      // 如果 activePrefix 没匹配到（比如 active=src, 但 path=/YaoXiang/en/）
      // 我们需要从 base 开始替换
      // /YaoXiang/en/ -> targetPrefix + en/
      // targetPrefix 是 /YaoXiang/v0.1/
      // result: /YaoXiang/v0.1/en/
      
      // 我们把 activePrefix 视为 base
      newPath = path.replace(base, targetPrefix)
  }

  // 3. 规范化斜杠
  newPath = newPath.replace(/\/\/+/g, '/')
  
  window.location.href = newPath
  isOpen.value = false
}
</script>

<template>
  <div class="version-switcher" @mouseleave="isOpen = false">
    <button 
      class="switcher-btn" 
      @click="isOpen = !isOpen"
      :aria-expanded="isOpen"
    >
      <span class="text">{{ getText(currentVersion) }}</span>
      <span class="caret" :class="{ open: isOpen }">
        <svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" focusable="false" viewBox="0 0 24 24" class="vt-link-icon box-icon"><path d="M12,16c-0.3,0-0.5-0.1-0.7-0.3l-6-6c-0.4-0.4-0.4-1,0-1.4s1-0.4,1.4,0l5.3,5.3l5.3-5.3c0.4-0.4,1-0.4,1.4,0s0.4,1,0,1.4l-6,6C12.5,15.9,12.3,16,12,16z"/></svg>
      </span>
    </button>
    
    <div v-if="isOpen" class="dropdown-menu">
      <div
        v-for="v in versions"
        :key="v.label"
        class="dropdown-item"
        :class="{ active: v.value === currentVersion.value }"
        @click="switchVersion(v)"
      >
        {{ getText(v) }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.version-switcher {
  position: relative;
  display: flex;
  align-items: center;
  height: var(--vp-nav-height); 
  padding: 0 12px;
  cursor: pointer;
  z-index: 100;
}

.switcher-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-text-1);
  transition: color 0.25s;
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 0;
  height: 100%;
}

.switcher-btn:hover {
  color: var(--vp-c-brand);
}

.switcher-btn .text {
  font-family: var(--vp-font-family-base);
}

.caret {
  display: flex;
  align-items: center;
  width: 14px;
  height: 14px;
  fill: currentColor;
  transition: transform 0.25s;
}

.caret.open {
  transform: rotate(180deg);
}

.dropdown-menu {
  position: absolute;
  top: calc(100% - 10px);
  right: 0;
  min-width: 128px;
  max-height: calc(100vh - var(--vp-nav-height));
  overflow-y: auto;
  
  background-color: var(--vp-c-bg-elevated);
  border: 1px solid var(--vp-c-divider);
  border-radius: 12px;
  box-shadow: var(--vp-shadow-3);
  padding: 12px;
  
  opacity: 1;
  visibility: visible;
  transform: translateY(0);
}

.dropdown-item {
  display: block;
  border-radius: 6px;
  padding: 0 12px;
  line-height: 32px;
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-text-1);
  white-space: nowrap;
  transition: background-color 0.25s, color 0.25s;
  cursor: pointer;
}

.dropdown-item:hover {
  background-color: var(--vp-c-bg-soft);
  color: var(--vp-c-brand);
}

.dropdown-item.active {
  color: var(--vp-c-brand);
}
</style>
