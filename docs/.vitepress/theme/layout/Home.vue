<script setup lang="ts">
import { useData, withBase } from 'vitepress'
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useMouse, useWindowSize } from '@vueuse/core'

const { site, frontmatter } = useData<any>()

// Typewriter Effect for Description
const displayedText = ref('')
const cursorVisible = ref(true)
let typeTimeout: ReturnType<typeof setTimeout> | null = null
let cursorInterval: ReturnType<typeof setInterval> | null = null
let isDeleting = false
let isFirstRun = true
let hasStarted = false

// Code Showcase Copy
const copySuccess = ref(false)
const codeContent = `yaoxiang new universe
Creating project...
type Universe = {
    matter: Amount,
    energy: Amount,
    expand: (self) -> Void = ...
}
Done in 0.04s ✨`

const copyCode = async () => {
  await navigator.clipboard.writeText(codeContent)
  copySuccess.value = true
  setTimeout(() => copySuccess.value = false, 2000)
}

const targetText = computed(() => frontmatter.value?.hero?.text || '')

const typeWriter = () => {
  const text = targetText.value
  if (!text) {
    typeTimeout = setTimeout(typeWriter, 100)
    return
  }

  if (isFirstRun) {
    if (displayedText.value.length > 0) {
      displayedText.value = displayedText.value.slice(0, -1)
      typeTimeout = setTimeout(typeWriter, 20)
      return
    }
    isFirstRun = false
  }

  if (!isDeleting) {
    if (displayedText.value.length < text.length) {
      displayedText.value = text.slice(0, displayedText.value.length + 1)
      typeTimeout = setTimeout(typeWriter, 140 + Math.random() * 120)
    } else {
      isDeleting = true
      typeTimeout = setTimeout(typeWriter, 5000)
    }
  } else {
    if (displayedText.value.length > 0) {
      displayedText.value = displayedText.value.slice(0, -1)
      typeTimeout = setTimeout(typeWriter, 100)
    } else {
      isDeleting = false
      typeTimeout = setTimeout(typeWriter, 500)
    }
  }
}

const startTypewriter = () => {
  if (hasStarted) return
  hasStarted = true
  cursorInterval = setInterval(() => {
    cursorVisible.value = !cursorVisible.value
  }, 500)
  typeWriter()
}

// 监听 frontmatter 变化
watch(() => frontmatter.value?.hero?.text, (newText) => {
  if (newText && !hasStarted) {
    startTypewriter()
  }
}, { immediate: true })

onUnmounted(() => {
  if (typeTimeout) clearTimeout(typeTimeout)
  if (cursorInterval) clearInterval(cursorInterval)
})

// 3D Code Showcase Logic (Global Mouse Tracking)
const { x: mouseX, y: mouseY } = useMouse()
const { width: windowWidth, height: windowHeight } = useWindowSize()

const codeTransform = computed(() => {
  // 计算鼠标在屏幕中的相对位置 (-0.5 ~ 0.5)
  const factorX = (mouseX.value / windowWidth.value) - 0.5 - 0.2
  const factorY = (mouseY.value / windowHeight.value) - 0.5
  
  // 旋转强度
  const intensity = 60

  // 计算旋转角度
  // Y轴旋转（左右）：鼠标在右(>0) -> 元素面朝右 -> 左边前移，右边后退 -> rotateY (+)
  // X轴旋转（上下）：鼠标在下(>0) -> 元素面朝下 -> 顶部后退，底部前移 -> rotateX (-)
  const rotateY = factorX * intensity
  const rotateX = factorY * -intensity
  
  return {
    transform: `perspective(1000px) rotateX(${rotateX}deg) rotateY(${rotateY}deg)`,
    transition: 'transform 0.1s ease-out'
  }
})
</script>

<template>
  <div class="retro-home min-h-screen bg-base-100 font-monoselection:bg-primary selection:text-primary-content">
    
    <!-- Hero Section -->
    <div class="hero min-h-[80vh] border-b-4 border-primary/20 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-base-200 via-base-100 to-base-100">
      <div class="hero-content flex-col lg:flex-row-reverse gap-12 w-full max-w-7xl px-4">
        
        <!-- Retro Code Showcase (Right Side) -->
        <div
          :style="codeTransform"
          class="code-showcase border-2 border-primary shadow-[8px_8px_0px_rgba(255,62,0,0.4)] w-full max-w-lg rounded-lg overflow-hidden [transform-style:preserve-3d]"
        >
          <!-- Window Title Bar -->
          <div class="window-header flex items-center justify-between px-4 py-2 bg-base-200 dark:bg-[#2a2a2a] border-b border-base-300 dark:border-base-content/20">
            <div class="flex gap-2">
              <span class="window-btn w-3 h-3 rounded-full bg-[#ff5f56]"></span>
              <span class="window-btn w-3 h-3 rounded-full bg-[#ffbd2e]"></span>
              <span class="window-btn w-3 h-3 rounded-full bg-[#27ca40]"></span>
            </div>
            <button
              @click="copyCode"
              class="copy-btn text-xs font-mono px-3 py-1 rounded transition-all"
              :class="copySuccess ? 'bg-success text-success-content' : 'bg-base-300 hover:bg-primary hover:text-primary-content'"
            >
              {{ copySuccess ? 'COPIED!' : 'COPY' }}
            </button>
          </div>
          <!-- Code Content -->
          <div class="mockup-code bg-transparent text-sm p-4 overflow-x-auto">
            <pre data-prefix="$"><code>yaoxiang new universe</code></pre>
            <pre data-prefix=">" class="text-success"><code>Creating project...</code></pre>
            <pre data-prefix=">"><code><span class="text-primary">type</span> <span class="text-warning">Universe</span> = {</code></pre>
            <pre data-prefix=">"><code>    matter: <span class="text-warning">Amount</span>,</code></pre>
            <pre data-prefix=">"><code>    energy: <span class="text-warning">Amount</span>,</code></pre>
            <pre data-prefix=">"><code>    <span class="text-info">expand</span>: (<span class="text-secondary">self</span>) -> <span class="text-primary">Void</span> = ...</code></pre>
            <pre data-prefix=">"><code>}</code></pre>
            <pre data-prefix=">" class="text-success"><code>Done in 0.04s ✨</code></pre>
          </div>
        </div>

        <!-- Text Content (Left Side) -->
        <div class="text-center lg:text-left">
          
          <!-- Slogan -->
          <div class="mb-8 flex justify-center lg:justify-start animate-fade-in-up">
            <div class="badge badge-primary badge-outline badge-lg gap-3 rounded-none p-4 font-bold tracking-[0.2em] shadow-[4px_4px_0px_currentColor]">
              <div class="w-2 h-2 bg-primary animate-pulse"></div>
              TYPE THE UNIVERSE
            </div>
          </div>

          <!-- Main Title -->
          <h1 class="hero-title glitch-text" :data-text="frontmatter.hero.name" style="font-size: 5rem; font-size: max(3rem, 10vw); margin-bottom: 0.5rem; line-height: 1.1;">
            {{ frontmatter.hero.name }}
          </h1>
          
          <!-- Description -->
          <p class="hero-description text-2xl lg:text-3xl font-bold tracking-tight opacity-90" style=" min-height: 3rem;">
            <span>{{ displayedText }}</span><span class="typewriter-cursor" :class="{ hidden: !cursorVisible }">_</span>
          </p>

          <!-- Tagline -->
          <p class="text-base font-mono opacity-60 tracking-wide text-xl" style="margin-bottom: 1.5rem;">
            <span class="text-primary font-bold">>>_</span> {{ frontmatter.hero.tagline }}
          </p>
          
          <!-- Actions -->
          <div class="flex flex-col sm:flex-row justify-center lg:justify-start" style="margin-bottom: 1rem; gap: 1.5rem;">
            <a
              v-for="action in frontmatter.hero.actions"
              :key="action.text"
              :href="withBase(action.link)"
              :class="[
                'btn btn-lg h-14 px-8 text-lg rounded-none border-2 shadow-[6px_6px_0px_currentColor] hover:translate-y-[2px] hover:shadow-[2px_2px_0px_currentColor] transition-all font-black uppercase',
                action.theme === 'brand' ? 'btn-primary border-primary' : 'btn-outline border-base-content hover:bg-base-content hover:text-base-100'
              ]"
            >
              {{ action.text }}
            </a>
          </div>
        </div>
      </div>
    </div>

    <!-- Main Content Container -->
    <div class="container mx-auto px-4 py-16 max-w-6xl">
      
      <!-- SIDE A: Architecture -->
      <div class="mb-24 relative">
        <!-- Cassette Header -->
        <div class="flex flex-col md:flex-row justify-between items-center bg-primary text-primary-content p-4 mb-8 border-b-4 border-base-content border-r-4 shadow-[8px_8px_0px_rgba(0,0,0,0.2)] dark:shadow-[8px_8px_0px_rgba(255,255,255,0.1)]">
          <div class="text-2xl font-black tracking-widest flex items-center gap-2">
            <span class="badge badge-lg bg-base-content text-base-100 rounded-full h-8 w-8 flex items-center justify-center">A</span>
            THE ARCHITECTURE
          </div>
          <div class="font-mono text-sm opacity-80 mt-2 md:mt-0">RFC-009 / RFC-010 / RFC-011</div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
          
          <!-- Track 01 -->
          <div class="card bg-base-200 border-l-4 border-primary rounded-none hover:bg-base-300 transition-colors group">
            <div class="card-body p-6">
              <h2 class="card-title font-mono text-sm opacity-50 mb-0 group-hover:text-primary transition-colors">TRACK [01]</h2>
              <h3 class="text-xl font-bold mb-2">Unified Syntax</h3>
              <div class="badge badge-neutral mb-4">RFC-010</div>
              <p class="text-sm opacity-80 mb-4">极简统一模型。一切皆 <code>name: type = value</code>。</p>
              <div class="mockup-code bg-base-100 text-[10px] w-full transform scale-95 origin-top-left border border-base-content/10">
                <pre><code><span class="text-primary">x</span>: Int = 42</code></pre>
                <pre><code><span class="text-secondary">add</span>: (Int)->Int = ...</code></pre>
              </div>
            </div>
          </div>

          <!-- Track 02 -->
          <div class="card bg-base-200 border-l-4 border-primary rounded-none hover:bg-base-300 transition-colors group">
            <div class="card-body p-6">
              <h2 class="card-title font-mono text-sm opacity-50 mb-0 group-hover:text-primary transition-colors">TRACK [02]</h2>
              <h3 class="text-xl font-bold mb-2">Zero-Cost Generics</h3>
              <div class="badge badge-neutral mb-4">RFC-011</div>
              <p class="text-sm opacity-80">编译期单态化。死代码消除。类型系统即宏。</p>
            </div>
          </div>

          <!-- Track 03 -->
          <div class="card bg-base-200 border-l-4 border-primary rounded-none hover:bg-base-300 transition-colors group">
            <div class="card-body p-6">
              <h2 class="card-title font-mono text-sm opacity-50 mb-0 group-hover:text-primary transition-colors">TRACK [03]</h2>
              <h3 class="text-xl font-bold mb-2">Ownership</h3>
              <div class="badge badge-neutral mb-4">RFC-009</div>
              <p class="text-sm opacity-80">
                <span class="text-success font-bold">✓</span> Ref Sharing <br>
                <span class="text-error font-bold">✕</span> GC <br>
                <span class="text-error font-bold">✕</span> Lifetimes
              </p>
            </div>
          </div>

        </div>
      </div>

      <!-- SIDE B: Runtime -->
      <div class="mb-24 relative">
        <!-- Cassette Header B-Side -->
        <div class="flex flex-col md:flex-row justify-between items-center bg-secondary text-secondary-content p-4 mb-8 border-b-4 border-base-content border-r-4 shadow-[8px_8px_0px_rgba(0,0,0,0.2)] dark:shadow-[8px_8px_0px_rgba(255,255,255,0.1)]">
          <div class="text-2xl font-black tracking-widest flex items-center gap-2">
            <span class="badge badge-lg bg-base-content text-base-100 rounded-full h-8 w-8 flex items-center justify-center">B</span>
            THE RUNTIME
          </div>
          <div class="font-mono text-sm opacity-80 mt-2 md:mt-0">RFC-008 / STD-LIB</div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
          
           <!-- Track 04 -->
           <div class="card bg-base-200 border-l-4 border-secondary rounded-none hover:bg-base-300 transition-colors">
            <div class="card-body">
              <h2 class="card-title font-mono text-sm opacity-50 mb-0">TRACK [04]</h2>
              <h3 class="text-xl font-bold">Decoupled Scheduler</h3>
              <div class="py-4">
                 <ul class="steps steps-vertical lg:steps-horizontal w-full text-sm">
                  <li class="step step-secondary opacity-60">Embedded <br/>(Sync)</li>
                  <li class="step step-secondary">Standard <br/>(DAG)</li>
                  <li class="step step-secondary font-bold">Full <br/>(WorkSteal)</li>
                </ul>
              </div>
              <p class="text-sm opacity-80">适应从微控制器到高性能服务器的所有场景。</p>
            </div>
          </div>

          <!-- Track 05 -->
          <div class="card bg-base-200 border-l-4 border-secondary rounded-none hover:bg-base-300 transition-colors">
            <div class="card-body">
              <h2 class="card-title font-mono text-sm opacity-50 mb-0">TRACK [05]</h2>
              <h3 class="text-xl font-bold">Language Spec v1.6</h3>
              <div class="flex flex-wrap gap-2 mt-2">
                <span class="badge badge-outline">18 Keywords</span>
                <span class="badge badge-outline">Type Inference</span>
                <span class="badge badge-outline">Pattern Match</span>
              </div>
              <p class="mt-4 text-sm opacity-80">没有复杂的语法糖，只有纯粹的表达力。</p>
            </div>
          </div>

        </div>
      </div>

    </div>

    <!-- Retro Footer -->
    <footer class="footer items-center p-10 bg-neutral text-neutral-content rounded-none">
      <div class="items-center grid-flow-col">
        <p class="font-bold text-xl">YX</p> 
        <p>Copyright © 2026 - All right reserved by YaoXiang Foundation</p>
      </div> 
      <div class="grid-flow-col gap-4 md:place-self-center md:justify-self-end">
        <a class="link link-hover" :href="frontmatter.hero.actions[2].link">GitHub</a>
        <a class="link link-hover" :href="withBase('/zh/contributing')">Contributing</a>
      </div>
    </footer>

  </div>
</template>

<style scoped>
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;700&family=Space+Mono:ital,wght@0,400;0,700;1,400&display=swap');

.retro-home {
  font-family: 'Space Mono', monospace;
}

.hero-title {
  font-family: 'Microsoft YaHei', 'SimHei', 'PingFang SC', 'Heiti SC', sans-serif;
  font-weight: 900;
  font-size: 4rem !important;
}

.hero-description {
  font-family: 'Microsoft YaHei', 'SimHei', 'PingFang SC', 'Heiti SC', sans-serif;
  font-weight: 700;
}

.typewriter-cursor {
  display: inline-block;
  width: 4em;
  height: 1.2em;
  background: hsl(var(--bc));
  animation: cursor-blink 1s step-end infinite;
  margin-left: 2px;
  vertical-align: middle;
}

.typewriter-cursor.hidden {
  opacity: 0;
}

.install-box {
  border-left: 3px solid hsl(var(--p));
  transition: all 0.2s ease;
}

.install-box:hover {
  border-left-color: hsl(var(--su));
  transform: translateX(4px);
  box-shadow: 6px 6px 0px hsl(var(--su) / 0.3) !important;
}

@keyframes cursor-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.glitch-text {
  position: relative;
  display: inline-block;
  color: hsl(var(--bc));
}

.glitch-text::before,
.glitch-text::after {
  content: attr(data-text);
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
}

</style>