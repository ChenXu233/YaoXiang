import DefaultTheme from 'vitepress/theme'
import './tailwind.css'
import 'vitepress-plugin-nprogress/lib/css/index.css'
import mediumZoom from 'medium-zoom'
import { useDark, useToggle } from '@vueuse/core'
import { h, nextTick, watch } from 'vue'
import { useData } from 'vitepress'
import { createMermaidRenderer } from 'vitepress-mermaid-renderer'
import nprogress from 'vitepress-plugin-nprogress'
import BackToTop from 'vitepress-plugin-back-to-top'
import VersionSwitcher from './components/VersionSwitcher.vue'
import Playground from "./components/Playground.vue";
import Home from "./layout/Home.vue";
import Download from "./layout/Download.vue";
import Community from "./layout/Community.vue";

// medium-zoom directive
const zoomDirective = {
  mounted(el, binding) {
    mediumZoom(el, {
      background: "#000000",
      margin: 24,
      ...(typeof binding.value === "object" ? binding.value : {}),
    });
  },
};

export default {
  extends: DefaultTheme,

  enhanceApp({ app, router }) {
    // 注册 medium-zoom 指令
    app.directive("zoom", zoomDirective);

    // 注册 VersionSwitcher 组件
    app.component("VersionSwitcher", VersionSwitcher);
    app.component("Playground", Playground);
    app.component("BackToTop", BackToTop);

    // nprogress 页面加载进度条
    nprogress({ router });
  },

  Layout() {
    const { isDark } = useData();
    const { frontmatter } = useData();

    // 初始化 mermaid 渲染器
    const initMermaid = () => {
      createMermaidRenderer({
        theme: isDark.value ? "dark" : "default",
      });
    };

    // 初始化
    nextTick(() => initMermaid());

    // 主题切换时重新渲染
    watch(
      () => isDark.value,
      () => {
        initMermaid();
      },
    );

    if (frontmatter.value.is_download) {
      return h(DefaultTheme.Layout, null, {
        "page-top": () => h(Download),
        "layout-bottom": () => h(BackToTop),
      });
    }

    if (frontmatter.value.is_home) {
      return h(DefaultTheme.Layout, null, {
        "page-top": () => h(Home),
        "layout-bottom": () => h(BackToTop),
      });
    }

    if (frontmatter.value.is_community) {
      return h(DefaultTheme.Layout, null, {
        "page-top": () => h(Community),
        "layout-bottom": () => h(BackToTop),
      });
    }

    if (frontmatter.value.is_playground) {
      return h(DefaultTheme.Layout, null, {
        "page-top": () => h(Playground),
        "layout-bottom": () => h(BackToTop),
      });
    }

    return h(DefaultTheme.Layout, null, {
      "layout-bottom": () => h(BackToTop),
    });
  },

  mounted() {
    // 暗色模式切换
    const isDark = useDark();
    const toggleDark = useToggle(isDark);

    // 将 toggleDark 添加到全局
    window.toggleDark = toggleDark;
  },
};
