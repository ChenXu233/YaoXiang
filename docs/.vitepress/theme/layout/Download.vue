<script setup lang="ts">
import { useData } from 'vitepress'
import { computed, ref } from 'vue'
import { useMouse, useWindowSize } from '@vueuse/core'

const { frontmatter, isDark } = useData<any>()

// 3D Tilt Logic for the main terminal
const { x: mouseX, y: mouseY } = useMouse()
const { width: windowWidth, height: windowHeight } = useWindowSize()

const cardTransform = computed(() => {
  const factorX = (mouseX.value / windowWidth.value) - 0.5
  const factorY = (mouseY.value / windowHeight.value) - 0.5
  const intensity = 20
  const rotateY = factorX * intensity
  const rotateX = factorY * -intensity
  return {
    transform: `perspective(1000px) rotateX(${rotateX}deg) rotateY(${rotateY}deg)`,
    transition: 'transform 0.1s ease-out'
  }
})

// Shadow colors for light/dark mode
const shadowColor = computed(() => isDark.value ? 'rgba(0,0,0,0.3)' : 'rgba(0,0,0,0.15)')
const shadowColorHover = computed(() => isDark.value ? 'rgba(0,0,0,0.2)' : 'rgba(0,0,0,0.08)')

const copySuccess = ref(false)
const copyInstall = async () => {
  const cmd = frontmatter.value.install_command || 'curl -fsSL https://yaoxiang.org/install.sh | sh'
  await navigator.clipboard.writeText(cmd)
  copySuccess.value = true
  setTimeout(() => copySuccess.value = false, 2000)
}

const downloads = computed(() => frontmatter.value.downloads || [])
const d = computed(() => frontmatter.value.download || {})
</script>

<template>
  <div class="retro-download min-h-screen bg-base-100 font-monoselection:bg-secondary selection:text-secondary-content pb-20 overflow-x-hidden">
    
    <!-- Header/Hero -->
    <div class="relative py-20 px-4 border-b-4 border-primary/20 bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-base-200 via-base-100 to-base-100">
      <div class="absolute top-10 left-10 opacity-10 font-black text-9xl select-none -z-0 pointer-events-none hidden lg:block">DOWNLOAD</div>
      
      <div class="hero-content flex-col gap-8 relative z-10 w-full max-w-5xl mx-auto">
        <div class="text-center">
            <div class="inline-flex items-center gap-2 px-3 py-1 border border-primary/50 text-primary text-xs font-bold tracking-widest uppercase mb-6 rounded-full bg-primary/5">
               <span class="w-2 h-2 bg-primary rounded-full animate-pulse"></span>
               {{ d.latest_stable?.replace('{version}', frontmatter.version || '0.1.0') || 'LATEST STABLE v' + (frontmatter.version || '0.1.0') }}
            </div>
            <h1 class="text-5xl md:text-7xl font-black mb-6 glitch-text tracking-tighter" :data-text="frontmatter.title">{{ frontmatter.title }}</h1>
            <p class="text-xl opacity-60 max-w-2xl mx-auto font-mono leading-relaxed">{{ frontmatter.description }}</p>
        </div>
      </div>
    </div>

    <!-- Main Content -->
    <div class="container mx-auto px-4 -mt-12 relative z-20">
      
      <!-- Terminal Install Block with 3D Tilt -->
      <div :style="cardTransform" class="max-w-4xl mx-auto mb-24 [transform-style:preserve-3d]">
        <div class="mockup-code bg-white dark:bg-[#1a1a1a] text-gray-800 dark:text-gray-300 shadow-[20px_20px_0px_rgba(0,0,0,0.1)] dark:shadow-[20px_20px_0px_rgba(0,0,0,0.3)] border-2 border-base-content/20 dark:border-primary/30 relative overflow-visible group transition-all duration-300">
          
          <!-- Decorative bits -->
          <div class="absolute -right-2 -top-2 w-4 h-4 bg-primary/20 dark:bg-primary/20 border border-primary z-50"></div>
          <div class="absolute -left-2 -bottom-2 w-4 h-4 bg-primary/20 dark:bg-primary/20 border border-primary z-50"></div>

          <div class="px-8 py-8 flex flex-col md:flex-row items-center gap-6">
             <div class="flex-1 w-full">
               <div class="text-xs text-primary/70 dark:text-primary/70 font-bold mb-2 tracking-widest uppercase">{{ d.quick_install || 'QUICK INSTALL' }}</div>
               <code class="block bg-gray-100 dark:bg-black/50 p-4 rounded text-success font-mono text-sm md:text-base border-l-2 border-success/50 w-full overflow-x-auto">
                 <span class="text-gray-500 dark:text-gray-500">$</span> {{ frontmatter.install_command || 'curl -fsSL https://yaoxiang.org/install.sh | sh' }}
               </code>
             </div>
             <button
                @click="copyInstall"
                class="btn btn-primary btn-outline btn-lg font-mono rounded-none border-2 shadow-[4px_4px_0px_currentColor] hover:translate-y-[2px] hover:shadow-[2px_2px_0px_currentColor] active:translate-y-[4px] active:shadow-none min-w-[140px]"
             >
                {{ copySuccess ? (d.copied || 'COPIED!') : (d.copy || 'COPY') }}
             </button>
          </div>
          <div class="px-8 pb-4 text-[10px] text-gray-500 dark:text-gray-500 font-mono text-center md:text-left">
            {{ d.supported || 'Supported: Windows (PowerShell), macOS, Linux (x64/ARM64)' }}
          </div>
        </div>
      </div>

      <!-- OS Selection Grid -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-6xl mx-auto">
        <div v-for="(os, index) in downloads" :key="index" 
            class="card bg-base-100 border-2 border-base-content/10 hover:border-primary transition-all duration-300 hover:-translate-y-2 hover:shadow-[12px_12px_0px_rgba(0,0,0,0.1)] group rounded-none"
        >
          <div class="card-body">
            <div class="flex justify-between items-start mb-4">
               <h2 class="card-title text-3xl font-black">{{ os.os }}</h2>
               <div class="badge badge-lg badge-neutral rounded-none font-mono">{{ os.arch }}</div>
            </div>
            
            <ul class="my-6 space-y-2 opacity-70 font-mono text-sm">
                <li v-for="feat in os.features" :key="feat" class="flex items-center gap-2">
                    <span class="w-1.5 h-1.5 bg-primary rounded-full"></span>{{ feat }}
                </li>
            </ul>

            <div class="card-actions justify-end mt-auto">
              <div v-if="os.links && os.links.length > 1" class="dropdown dropdown-top dropdown-end w-full">
                <div tabindex="0" role="button" class="btn btn-block btn-primary rounded-none font-bold tracking-widest">{{ d.download_btn || 'DOWNLOAD' }} ▼</div>
                <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52 border border-base-300">
                  <li v-for="link in os.links" :key="link.name">
                    <a :href="link.url" class="font-mono">{{ link.name }}</a>
                  </li>
                </ul>
              </div>
              <a v-else-if="os.links && os.links.length === 1"
                 :href="os.links[0].url"
                 class="btn btn-block btn-primary rounded-none font-bold tracking-widest shadow-[4px_4px_0px_currentColor] hover:translate-y-[1px] hover:shadow-[3px_3px_0px_currentColor]">
                 {{ d.download_btn || 'DOWNLOAD' }}
              </a>
              <button v-else class="btn btn-block btn-disabled rounded-none">{{ d.coming_soon || 'COMING SOON' }}</button>
            </div>
            <div class="mt-4 text-center">
                <a href="#" class="text-xs font-mono opacity-40 hover:opacity-100 hover:text-primary transition-colors underline decoration-dotted">{{ d.checksum || 'checksum / signatures' }}</a>
            </div>
          </div>
        </div>
      </div>

      <!-- Other Options -->
      <div class="mt-24 border-t-2 border-base-300 pt-16 max-w-4xl mx-auto">
         <div class="flex flex-col md:flex-row gap-12">
            <div class="flex-1">
                <h3 class="text-2xl font-bold mb-4">{{ d.build_from_source?.title || 'Build from Source' }}</h3>
                <p class="opacity-70 mb-4">{{ d.build_from_source?.description || 'You can build YaoXiang from source using Cargo. Make sure you have Rust installed.' }}</p>
                <div class="mockup-code bg-base-200 text-xs">
                    <pre data-prefix="$"><code>git clone https://github.com/ChenXu233/YaoXiang</code></pre>
                    <pre data-prefix="$"><code>cd YaoXiang</code></pre>
                    <pre data-prefix="$"><code>cargo build --release</code></pre>
                </div>
            </div>
            <div class="flex-1">
                <h3 class="text-2xl font-bold mb-4">{{ d.nightly_builds?.title || 'Nightly Builds' }}</h3>
                <p class="opacity-70 mb-4">{{ d.nightly_builds?.description || 'Bleeding edge builds are available for testing. Not recommended for production.' }}</p>
                <a href="https://github.com/ChenXu233/YaoXiang/actions" target="_blank" class="btn btn-outline gap-2 rounded-none">
                    <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path fill-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clip-rule="evenodd"></path></svg>
                    {{ d.github_actions || 'Go to GitHub Actions' }}
                </a>
            </div>
         </div>
      </div>

    </div>
  </div>
</template>

<style scoped>
@import url('https://fonts.googleapis.com/css2?family=Space+Mono:ital,wght@0,400;0,700;1,400&display=swap');

.retro-download {
  font-family: 'Space Mono', monospace;
}

</style>
