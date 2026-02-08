import { defineConfig } from 'vitepress'
import yaoxiangGrammar from './syntaxes/yaoxiang.tmLanguage.json'

export default defineConfig({
  base: '/YaoXiang/',
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

  // 代码高亮配置
  markdown: {
    theme: {
      light: 'github-light',
      dark: 'github-dark',
    },
    languages: [
      {
        id: 'yaoxiang',
        scopeName: 'source.yaoxiang',
        grammar: yaoxiangGrammar,
        name: 'yaoxiang',
        aliases: ['yx']
      }
    ]
  },

  themeConfig: {
    logo: '/logo.svg',

    // 全局nav配置，用于作为fallback
    nav: [
      { text: '开始', link: '/getting-started' },
      { text: '教程', link: '/tutorial/' },
      { text: '参考', link: '/reference/' },
      { text: 'GitHub', link: 'https://github.com/ChenXu233/yaoxiang' },
    ],

    sidebar: {
      '/zh/': [
        {
          text: '中文文档',
          items: [
            { text: '快速开始', link: '/getting-started' },
            { text: '教程', link: '/tutorial/' },
            { text: '贡献指南', link: '/contributing' },
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
    root: {
      lang: 'zh-CN',
      label: '中文',
      nav: [
        { text: '开始', link: '/getting-started' },
        { text: '教程', link: '/tutorial/' },
        { text: '参考', link: '/reference/' },
        { text: 'GitHub', link: 'https://github.com/ChenXu233/yaoxiang' },
      ],
    },
    en: {
      lang: 'en-US',
      label: 'English',
      nav: [
        { text: 'Getting Started', link: '/en/getting-started' },
        { text: 'Tutorial', link: '/en/tutorial/' },
        { text: 'Reference', link: '/en/reference/' },
        { text: 'GitHub', link: 'https://github.com/ChenXu233/yaoxiang' },
      ],
    },
  },
})
