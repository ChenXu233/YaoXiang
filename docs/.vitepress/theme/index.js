import DefaultTheme from 'vitepress/theme'
import './tailwind.css'
import mediumZoom from 'medium-zoom'
import { useDark, useToggle } from '@vueuse/core'
import { h } from 'vue'
import { useData } from 'vitepress'
import Mermaid from './component/Mermaid.vue'
import Home from './layout/Home.vue'

// medium-zoom 指令
const zoomDirective = {
  mounted(el, binding) {
    mediumZoom(el, {
      background: '#000000',
      margin: 24,
      ...(typeof binding.value === 'object' ? binding.value : {}),
    })
  },
}

export default {
  extends: DefaultTheme,

  enhanceApp({ app }) {
    // 注册 medium-zoom 指令
    app.directive('zoom', zoomDirective)

    // 注册 Mermaid 组件
    app.component('Mermaid', Mermaid)
  },

  Layout() {
    const { frontmatter } = useData()
    if (frontmatter.value.is_home) {
      return h(DefaultTheme.Layout, null, {
        'page-top': () => h(Home)
      })
    }
    return h(DefaultTheme.Layout, null, {
      'page-top': () => h(Home)
    })
  },

  mounted() {
    // 暗色模式切换
    const isDark = useDark()
    const toggleDark = useToggle(isDark)

    // 将 toggleDark 添加到全局
    window.toggleDark = toggleDark
  },
}
