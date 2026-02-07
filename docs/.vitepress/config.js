import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'YaoXiang',
  description: '一门面向未来的编程语言',

  // 排除有问题文件的目录
  srcExclude: [
    'archived/**',
    'plan/old/**',
    '**/*.backup.md',
  ],

  // 忽略死链接（现有文档有 146 个死链接）
  ignoreDeadLinks: true,

  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: '开始', link: '/zh/getting-started' },
      { text: '教程', link: '/zh/tutorial/' },
      { text: '参考', link: '/zh/reference/' },
      { text: 'GitHub', link: 'https://github.com/yaoxiang-lang/yaoxiang' },
    ],

    sidebar: {
      '/zh/': [
        {
          text: '中文文档',
          items: [
            { text: '快速开始', link: '/zh/getting-started' },
            { text: '教程', link: '/zh/tutorial/' },
            { text: '贡献指南', link: '/zh/contributing' },
          ],
        },
      ],
      '/en/': [
        {
          text: 'English',
          items: [
            { text: 'Quick Start', link: '/en/getting-started' },
            { text: 'Tutorial', link: '/en/tutorial/' },
            { text: 'Contributing', link: '/en/contributing' },
          ],
        },
      ],
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/ChenXu233/yaoxiang' },
    ],

    editLink: {
      pattern: 'https://github.com/ChenXu233/yaoxiang/edit/main/docs/:path',
      text: '在 GitHub 上编辑此页',
    },

    search: {
      provider: 'local',
    },

    outline: 'deep',
  },

  locales: {
    root: { lang: 'zh-CN', label: '中文' },
    en: { lang: 'en-US', label: 'English' },
  },
})
