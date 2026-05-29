<script setup lang="ts">
import { useData } from 'vitepress'
import { ref, onMounted, computed } from 'vue'

const { frontmatter, lang } = useData<any>()

// Animated counter for stats
const memberCount = ref(0)
onMounted(() => {
    // Dramatic pause then jump to 1
    setTimeout(() => {
        memberCount.value = 1
    }, 500)
})

const isZh = computed(() => lang.value === 'zh' || lang.value === 'zh-CN' || lang.value === 'zh-Hans')

const t = (en: string, zh: string) => isZh.value ? zh : en

const items = computed(() => [
    { label: t('Contributors', '贡献者'), value: memberCount, desc: t('Just me (ChenXu233) for now!', '目前只有我 (ChenXu233)！'), color: 'text-primary' },
    { label: t('Commits', '提交数'), value: '100+', desc: t('Building the foundation', '正在通过搬砖构建世界'), color: 'text-secondary' },
    { label: t('Stars', '✨ Stars'), value: '?', desc: t('Waiting for you', '期待你的点亮'), color: 'text-accent' },
])

</script>

<template>
<div class="retro-community min-h-screen bg-base-100 font-mono pb-20 overflow-x-hidden selection:bg-primary selection:text-primary-content">
    
    <!-- Hero Section with Grid Background -->
    <div class="relative py-24 px-4 border-b-4 border-primary/20">
        <!-- Grid Background -->
        <div class="absolute inset-0 z-0 opacity-10" 
             style="background-image: linear-gradient(currentColor 1px, transparent 1px), linear-gradient(90deg, currentColor 1px, transparent 1px); background-size: 20px 20px;">
        </div>

        <div class="hero-content text-center flex-col z-10 relative mx-auto max-w-5xl">

            <div class="text-3xl md:text-5xl font-black mb-6 uppercase font-serif" :data-text="frontmatter.title || t('Community', '社区')">
                {{ frontmatter.title || t('Community', '社区') }}
            </div>
            
            <div class="max-w-3xl mx-auto border-l-4 border-secondary pl-6 text-left my-8 bg-base-200/50 p-6">
                <p class="text-xl md:text-2xl font-bold opacity-80 leading-relaxed font-mono">
                    <span class="text-secondary mr-2">></span>
                    {{ frontmatter.description || t('Welcome to the YaoXiang community. Currently, it\'s a cozy room of one, but the door is wide open!', '欢迎来到爻象社区。目前这里还是一个人的温馨小屋，但大门永远为你敞开！') }}
                    <span class="animate-pulse">_</span>
                </p>
            </div>
        </div>
    </div>

    <!-- The "Review of the Situation" - Retro Terminal Style -->
    <div class="container mx-auto px-4 max-w-6xl mb-24 -mt-12 relative z-20">
        <div class="grid md:grid-cols-2 gap-12 items-start">
            
            <!-- Terminal Window -->
            <div class="mockup-window border-2 border-base-content bg-base-200/80 backdrop-blur-md shadow-[12px_12px_0px_rgba(0,0,0,1)] dark:shadow-[12px_12px_0px_rgba(255,255,255,0.2)] rounded-none">
                <div class="flex justify-center px-4 py-16 bg-base-100/80 border-t-2 border-base-content">
                    <div class="w-full max-w-md">
                        <h3 class="text-xl font-bold mb-6 border-b-2 border-dashed border-base-content/30 pb-2 uppercase tracking-widest flex justify-between">
                            <span>{{ t('Current_Maintainer', '当前_维护者') }}</span>
                            <span>[ID: 001]</span>
                        </h3>
                        
                        <div class="flex items-center gap-6 mb-8">
                            <div class="avatar online">
                                <div class="w-24 border-4 border-primary rounded-none shadow-[4px_4px_0px_currentColor]">
                                    <img src="https://github.com/ChenXu233.png" />
                                </div>
                            </div>
                            <div>
                                <a href="https://github.com/ChenXu233" target="_blank" class="text-2xl font-black hover:bg-primary hover:text-primary-content px-1 transition-colors">@ChenXu233</a>
                                <div class="text-xs opacity-60 mt-2 font-mono uppercase">{{ t('Role: Creator / Janitor', '角色: 创造者 / 清洁工') }}</div>
                            </div>
                        </div>

                        <!-- Chat / Terminal Logs -->
                        <div class="font-mono text-sm space-y-4 bg-black text-green-500 p-4 border border-green-800 shadow-inner">
                            <div>
                                <span class="text-primary">root@yaoxiang:~$</span> check_status
                            </div>
                            <div class="opacity-80">
                                > {{ t("Hello! I'm building this language with ❤️.", '你好！我正在用 ❤️ 构建这门语言。') }}
                            </div>
                            <div>
                                <span class="text-primary">root@yaoxiang:~$</span> check_members
                            </div>
                             <div class="opacity-80">
                                > Result: 1 member found.
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Stats Block -->
            <div class="flex flex-col justify-center h-full space-y-8">
                <div v-for="(item, idx) in items" :key="idx" 
                     class="relative border-2 border-base-content p-4 bg-base-100 shadow-[8px_8px_0px_currentColor] hover:translate-x-1 hover:translate-y-1 hover:shadow-[4px_4px_0px_currentColor] transition-all group cursor-default">
                    
                    <div class="absolute -top-3 -right-3 px-2 py-0.5 bg-base-content text-base-100 text-xs font-bold">{{ '0' + (idx + 1) }}</div>
                    
                    <div class="flex items-baseline justify-between mb-2">
                        <div class="text-sm font-bold uppercase tracking-widest opacity-60">{{ item.label }}</div>
                        <div class="text-4xl font-black font-mono group-hover:scale-110 transition-transform origin-right" :class="item.color">
                            {{ typeof item.value === 'object' ? item.value.value : item.value }}
                        </div>
                    </div>
                    <div class="h-1 w-full bg-base-content/10 mb-2 overflow-hidden">
                        <div class="h-full bg-current w-1/3 animate-progress-loading opacity-40"></div>
                    </div>
                    <div class="text-xs opacity-70 font-mono text-right">{{ item.desc }}</div>
                </div>
            </div>
        </div>
    </div>

    <!-- Cards Section -->
    <div class="container mx-auto px-4 max-w-6xl">
        <div class="bg-base-200 p-2 md:p-8 border-2 border-base-content/20 shadow-inner">
             <div class="flex flex-col md:flex-row gap-8">
                <!-- Github -->
                <a href="https://github.com/ChenXu233/YaoXiang" target="_blank" 
                   class="flex-1 card rounded-none bg-base-100 border-2 border-base-content hover:border-primary transition-all group hover:-translate-y-2 hover:shadow-[8px_8px_0px_var(--color-primary)]">
                    <div class="card-body items-center text-center">
                        <div class="w-16 h-16 bg-base-content text-base-100 flex items-center justify-center text-3xl mb-4 rounded-none shadow-[4px_4px_0px_rgba(0,0,0,0.2)]">
                            <i class="opacity-80 not-italic">Git</i>
                        </div>
                        <h2 class="card-title text-2xl font-black uppercase">GitHub</h2>
                        <p class="font-mono text-xs mt-2 opacity-70 border-t border-dashed border-base-content/20 pt-4 w-full">
                            {{ t('Star local repository. Initialize PR sequence.', 'Star 本地仓库。初始化 PR 序列。') }}
                        </p>
                    </div>
                </a>

                <!-- Discussions -->
                <a href="https://github.com/ChenXu233/YaoXiang/discussions" target="_blank" 
                   class="flex-1 card rounded-none bg-base-100 border-2 border-base-content hover:border-primary transition-all group hover:-translate-y-2 hover:shadow-[8px_8px_0px_var(--color-primary)]">
                    <div class="card-body items-center text-center">
                        <div class="w-16 h-16 bg-secondary text-secondary-content flex items-center justify-center text-3xl mb-4 rounded-none shadow-[4px_4px_0px_rgba(0,0,0,0.2)]">
                            <i class="not-italic">Talk</i>
                        </div>
                        <h2 class="card-title text-2xl font-black uppercase">{{ t('Discussions', '讨论区') }}</h2>
                        <p class="font-mono text-xs mt-2 opacity-70 border-t border-dashed border-base-content/20 pt-4 w-full">
                            {{ t('Establish communication channel. Open frequencies.', '建立通讯频道。开放频率。') }}
                        </p>
                    </div>
                </a>

                <!-- Contribute -->
                <div class="flex-1 card rounded-none bg-base-100 border-2 border-base-content hover:border-secondary transition-all group hover:-translate-y-2 hover:shadow-[8px_8px_0px_var(--color-secondary)] cursor-pointer">
                    <div class="card-body items-center text-center">
                        <div class="w-16 h-16 bg-secondary text-secondary-content flex items-center justify-center text-3xl mb-4 rounded-none shadow-[4px_4px_0px_rgba(0,0,0,0.2)]">
                            <i class="not-italic">Code</i>
                        </div>
                        <h2 class="card-title text-2xl font-black uppercase">{{ t('Contribute', '贡献') }}</h2>
                         <p class="font-mono text-xs mt-2 opacity-70 border-t border-dashed border-base-content/20 pt-4 w-full">
                            {{ t('Submit patches. Upgrade codebase.', '提交补丁。升级代码库。') }}
                        </p>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <!-- Standard Content Slot -->
    <div class="container mx-auto px-4 mt-20 max-w-4xl border-l-2 border-base-content/20 pl-8 ml-4 md:ml-auto md:mr-auto" v-if="frontmatter.content">
         <div class="prose dark:prose-invert font-mono">
            <Content />
         </div>
    </div>

</div>
</template>
