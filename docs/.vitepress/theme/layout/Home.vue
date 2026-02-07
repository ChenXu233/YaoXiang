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
          <div class="grid grid-cols-2 gap-4 justify-items-stretch" style="margin-bottom: 1rem; max-width: 400px;">
            <a
              v-for="(action, index) in frontmatter.hero.actions"
              :key="action.text"
              :href="withBase(action.link)"
              class="btn btn-lg h-14 text-lg rounded-none border-2 shadow-[4px_4px_0px_currentColor] hover:translate-y-[2px] hover:shadow-[2px_2px_0px_currentColor] transition-all font-black uppercase w-full"
              :class="action.theme === 'brand' ? 'btn-primary border-primary' : 'btn-outline border-base-content hover:bg-base-content hover:text-base-100'"
            >
              {{ action.text }}
            </a>
          </div>
        </div>
      </div>
    </div>

    <!-- Main Content Container -->
    <div class="container mx-auto px-4 py-16 max-w-5xl relative">
      
      <!-- Timeline Connector Backbone -->
      <div class="absolute left-8 md:left-[2.25rem] top-16 bottom-16 w-1 bg-base-300 dark:bg-base-content/10 hidden md:block"></div>

      <!-- SIDE A: Architecture -->
      <div class="mb-32">
        <!-- Cassette Header A -->
        <div class="sticky top-4 z-20 flex items-center gap-6 mb-16 backdrop-blur-sm py-4">
           <div class="w-20 h-20 shrink-0 bg-primary text-primary-content rounded-full flex items-center justify-center font-black text-4xl shadow-[4px_4px_0px_rgba(0,0,0,0.3)] border-4 border-base-100 z-10 transition-transform hover:scale-110 duration-200">
            A
          </div>
          <div class="bg-base-100/90 border-l-8 border-primary p-4 pr-12 shadow-lg backdrop-blur flex-grow max-w-2xl">
              <h2 class="text-3xl font-black tracking-widest text-base-content">THE ARCHITECTURE</h2>
              <div class="text-xs font-mono opacity-60 uppercase tracking-widest mt-1">RFC-009 / RFC-010 / RFC-011</div>
          </div>
        </div>

        <div class="space-y-24">
          
          <!-- Track 01 -->
          <div class="relative pl-0 md:pl-24 group">
            <!-- Timeline Dot -->
            <div class="absolute left-9 top-8 w-4 h-4 rounded-full bg-base-100 border-4 border-primary z-10 hidden md:block group-hover:scale-150 transition-transform duration-300"></div>
            
            <div class="card bg-base-100 shadow-xl border-l-4 border-primary hover:shadow-2xl hover:-translate-y-1 transition-all duration-300 overflow-visible">
               <div class="absolute -top-4 -right-4 bg-primary text-primary-content px-4 py-1 font-mono text-sm font-bold shadow-lg rotate-2 group-hover:rotate-0 transition-transform">
                 {{ frontmatter.tracks.track01.trackLabel }}
               </div>
               <div class="card-body md:flex-row gap-8 items-start">
                  <div class="flex-1">
                      <div class="badge badge-neutral mb-2">{{ frontmatter.tracks.track01.rfc }}</div>
                      <h3 class="text-3xl font-bold mb-4">{{ frontmatter.tracks.track01.title }}</h3>
                      <p class="text-lg opacity-80 mb-6 font-light leading-relaxed">
                        {{ frontmatter.tracks.track01.description }}
                      </p>
                      <ul class="space-y-2 text-sm opacity-70 font-mono">
                        <li v-for="feature in frontmatter.tracks.track01.features" :key="feature" class="flex items-center gap-2">
                          <span class="text-success">✓</span> {{ feature }}
                        </li>
                      </ul>
                  </div>
                  <div class="flex-1 w-full relative">
                      <div class="mockup-code bg-[#282c34] text-gray-300 shadow-2xl transform md:rotate-2 group-hover:rotate-0 transition-all duration-500 border border-white/10 text-xs">
                        <pre data-prefix="1"><code><span class="text-[#c678dd]">let</span> <span class="text-[#e06c75]">x</span>: Int = <span class="text-[#d19a66]">42</span></code></pre>
                        <pre data-prefix="2"><code><span class="text-[#c678dd]">let</span> <span class="text-[#61afef]">add</span>: (Int)->Int = ...</code></pre>
                        <pre data-prefix="3"><code><span class="text-[#c678dd]">type</span> <span class="text-[#e5c07b]">Point</span> = { x: Int, y: Int }</code></pre>
                      </div>
                  </div>
               </div>
            </div>
          </div>

          <!-- Track 02 -->
          <div class="relative pl-0 md:pl-24 group">
             <!-- Timeline Dot -->
             <div class="absolute left-9 top-8 w-4 h-4 rounded-full bg-base-100 border-4 border-primary z-10 hidden md:block group-hover:bg-primary transition-colors duration-300"></div>

             <div class="card bg-base-100 shadow-xl border-l-4 border-primary hover:shadow-2xl hover:-translate-y-1 transition-all duration-300">
                <div class="card-body">
                   <div class="flex flex-col md:flex-row gap-8 items-center">
                      <div class="flex-1">
                         <div class="badge badge-neutral mb-2">{{ frontmatter.tracks.track02.rfc }}</div>
                         <h3 class="text-3xl font-bold mb-4">{{ frontmatter.tracks.track02.title }}</h3>
                         <p class="text-lg opacity-80 mb-4">
                           {{ frontmatter.tracks.track02.description }}
                         </p>
                      </div>
                      <div class="flex-1 flex justify-center items-center py-8 bg-base-200/50 rounded-lg w-full">
                         <div class="flex items-center gap-4 font-mono text-xl sm:text-2xl font-bold opacity-80">
                            <span class="text-primary">Box&lt;T></span>
                            <span class="text-base-content/30">→</span>
                            <span class="text-secondary">Box_Int</span>
                         </div>
                      </div>
                   </div>
                </div>
             </div>
          </div>

           <!-- Track 03 -->
           <div class="relative pl-0 md:pl-24 group">
             <!-- Timeline Dot -->
             <div class="absolute left-9 top-8 w-4 h-4 rounded-full bg-base-100 border-4 border-primary z-10 hidden md:block group-hover:scale-150 transition-transform duration-300"></div>

             <div class="card bg-base-100 shadow-xl border-l-4 border-primary hover:shadow-2xl hover:-translate-y-1 transition-all duration-300">
                <div class="card-body md:flex-row gap-12 items-center">
                    <div class="flex-1">
                       <div class="badge badge-neutral mb-2">{{ frontmatter.tracks.track03.rfc }}</div>
                       <h3 class="text-3xl font-bold mb-4">{{ frontmatter.tracks.track03.title }}</h3>
                       <p class="text-lg opacity-80">
                         {{ frontmatter.tracks.track03.description }}
                       </p>
                    </div>
                    <div class="flex-1 grid grid-cols-2 gap-4 w-full">
                        <div v-for="(feature, index) in frontmatter.tracks.track03.features.slice(0, 2)" :key="feature" class="p-4 bg-success/10 border border-success/20 rounded-lg text-center">
                           <div class="text-2xl mb-1">✓</div>
                           <div class="font-bold text-success text-xs sm:text-sm">{{ feature }}</div>
                        </div>
                        <div v-for="(feature, index) in frontmatter.tracks.track03.features.slice(2)" :key="feature" class="p-4 bg-error/10 border border-error/20 rounded-lg text-center opacity-60">
                           <div class="text-2xl mb-1">✕</div>
                           <div class="font-bold text-error text-xs sm:text-sm">{{ feature }}</div>
                        </div>
                    </div>
                </div>
             </div>
          </div>

        </div>
      </div>

      <!-- SIDE B: Runtime -->
      <div class="mb-24">
         <!-- Cassette Header B -->
         <div class="sticky top-4 z-20 flex items-center gap-6 mb-16 backdrop-blur-sm py-4">
           <div class="w-20 h-20 shrink-0 bg-secondary text-secondary-content rounded-full flex items-center justify-center font-black text-4xl shadow-[4px_4px_0px_rgba(0,0,0,0.3)] border-4 border-base-100 z-10 transition-transform hover:scale-110 duration-200">
            B
          </div>
          <div class="bg-base-100/90 border-l-8 border-secondary p-4 pr-12 shadow-lg backdrop-blur flex-grow max-w-2xl">
              <h2 class="text-3xl font-black tracking-widest text-base-content">THE RUNTIME</h2>
              <div class="text-xs font-mono opacity-60 uppercase tracking-widest mt-1">RFC-008 / STD-LIB</div>
          </div>
        </div>

        <div class="space-y-24">

           <!-- Track 04 -->
           <div class="relative pl-0 md:pl-24 group">
              <!-- Timeline Dot -->
              <div class="absolute left-9 top-8 w-4 h-4 rounded-full bg-base-100 border-4 border-secondary z-10 hidden md:block group-hover:scale-150 transition-transform duration-300"></div>

              <div class="card bg-base-100 shadow-xl border-l-4 border-secondary hover:shadow-2xl hover:-translate-y-1 transition-all duration-300">
                  <div class="absolute -top-4 -right-4 bg-secondary text-secondary-content px-4 py-1 font-mono text-sm font-bold shadow-lg -rotate-2 group-hover:rotate-0 transition-transform">
                     {{ frontmatter.tracks.track04.trackLabel }}
                  </div>
                  <div class="card-body">
                      <div class="flex flex-col gap-6">
                        <div>
                           <h3 class="text-3xl font-bold mb-2">{{ frontmatter.tracks.track04.title }}</h3>
                           <p class="opacity-80 max-w-2xl">{{ frontmatter.tracks.track04.description }}</p>
                        </div>
                        <div class="w-full py-8">
                           <ul class="steps steps-vertical md:steps-horizontal w-full">
                            <li v-for="step in frontmatter.tracks.track04.steps" :key="step.label" class="step step-secondary font-mono text-sm" :data-content="step.label === 'Full' ? '★' : '●'">
                              {{ step.label }}<br/><span class="opacity-60 text-xs">({{ step.sub }})</span>
                            </li>
                          </ul>
                        </div>
                      </div>
                  </div>
              </div>
           </div>

           <!-- Track 05 -->
           <div class="relative pl-0 md:pl-24 group">
               <!-- Timeline Dot -->
               <div class="absolute left-9 top-8 w-4 h-4 rounded-full bg-base-100 border-4 border-secondary z-10 hidden md:block group-hover:bg-secondary transition-colors duration-300"></div>

               <div class="card bg-base-100 shadow-xl border-l-4 border-secondary hover:shadow-2xl hover:-translate-y-1 transition-all duration-300">
                  <div class="card-body md:flex-row items-center gap-8">
                      <div class="flex-1">
                          <h3 class="text-3xl font-bold mb-4">{{ frontmatter.tracks.track05.title }}</h3>
                          <p class="text-lg opacity-80 mb-6">
                            {{ frontmatter.tracks.track05.description }}
                          </p>
                          <a :href="frontmatter.hero.actions[0].link" class="link link-secondary no-underline font-bold hover:underline">Get Started →</a>
                      </div>
                      <div class="flex-1 flex flex-wrap gap-2 justify-end content-start">
                         <div class="badge badge-lg badge-outline p-4 hover:bg-secondary hover:text-secondary-content hover:scale-105 transition-all cursor-default">18 Keywords</div>
                         <div class="badge badge-lg badge-outline p-4 hover:bg-secondary hover:text-secondary-content hover:scale-105 transition-all cursor-default">Type Inference</div>
                         <div class="badge badge-lg badge-outline p-4 hover:bg-secondary hover:text-secondary-content hover:scale-105 transition-all cursor-default">Pattern Match</div>
                         <div class="badge badge-lg badge-outline p-4 hover:bg-secondary hover:text-secondary-content hover:scale-105 transition-all cursor-default">Traits</div>
                         <div class="badge badge-lg badge-outline p-4 hover:bg-secondary hover:text-secondary-content hover:scale-105 transition-all cursor-default">Modules</div>
                         <div class="badge badge-lg badge-outline p-4 hover:bg-secondary hover:text-secondary-content hover:scale-105 transition-all cursor-default">FFI</div>
                      </div>
                  </div>
               </div>
           </div>

        </div>

      </div>

    </div>

    <!-- Retro Footer -->
    <footer class="footer items-center p-10 bg-neutral text-neutral-content rounded-none">
      <div class="items-center grid-flow-col md:place-self-center">
        <p class="font-bold text-xl">YX</p> 
        <p>Copyright © 2026 - All right reserved by YaoXiang Foundation</p>
      </div> 
      <div class="grid-flow-col gap-4 md:place-self-center">
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