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
      link: '/',
      themeConfig: {
        nav: [
          { text: '下载', link: '/download' },
          { text: '教程', link: '/tutorial/' },
          { text: '实验', link: '/playground/' },
          { text: '工具', link: '/tools/' },
          { text: '社区', link: '/community/' },
          { text: '归档', link: '/archived/' },
          { text: '博客', link: '/blog/' },
        ],
        sidebar: [
          {
            text: '中文文档',
            items: [
              { text: '快速开始', link: '/getting-started' },
              { text: '教程', link: '/tutorial/' },
              { text: '贡献指南', link: '/contributing' },
            ],
          },
        ],
      },
    },
    en: {
      lang: 'en-US',
      label: 'English',
      link: '/en/',
      themeConfig: {
        nav: [
          { text: 'Download', link: '/en/download' },
          { text: 'Tutorial', link: '/en/tutorial/' },
          { text: 'Playground', link: '/en/playground/' },
          { text: 'Tools', link: '/en/tools/' },
          { text: 'Community', link: '/en/community/' },
          { text: 'Archived', link: '/en/archived/' },
          { text: 'Blog', link: '/en/blog/' },
        ],
        sidebar: [
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
    },
  },
})
