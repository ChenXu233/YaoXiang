import { defineConfig } from 'vitepress'
import yaoxiangGrammar from './syntaxes/yaoxiang.tmLanguage.json'

export default defineConfig({
  base: '/YaoXiang/',
  title: 'YaoXiang',
  description: '一门面向未来的编程语言',

  // 排除有问题文件的目录
  srcExclude: [
    'archived/**',
    'old/**',
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
          { text: '首页', link: '/' },
          { text: '下载', link: '/download' },
          { text: '教程', link: '/tutorial/' },
          { text: '码场', link: '/playground/' },
          { text: '工具', link: '/tools/' },
          { text: '设计', link: '/design/' },
          { text: '社区', link: '/community/' },
          { text: '博客', link: '/blog/' },
        ],
        sidebar: {
          // 教程目录的 sidebar
          '/tutorial/': [
            {
              text: '教程文档',
              items: [
                { text: '快速开始', link: '/tutorial/getting-started' },
                { text: '贡献指南', link: '/tutorial/contributing' },
              ],
            },
          ],
          // 设计文档的 sidebar
          '/design/': [
            {
              text: '设计文档',
              items: [
                { text: 'RFC', link: '/design/rfc/' },
                { text: '语言设计', link: '/design/language-spec' },
                { text: '错误码', link: '/design/error-code' },
              ],
            },
          ],
          // 默认 sidebar
          '/': [
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
    },
    en: {
      lang: 'en-US',
      label: 'English',
      link: '/en/',
      themeConfig: {
        nav: [
          { text: 'Home', link: '/en/' },
          { text: 'Download', link: '/en/download' },
          { text: 'Tutorial', link: '/en/tutorial/' },
          { text: 'Playground', link: '/en/playground/' },
          { text: 'Tools', link: '/en/tools/' },
          { text: 'Community', link: '/en/community/' },
          { text: 'Design', link: '/en/design/' },
          { text: 'Blog', link: '/en/blog/' },
        ],
        sidebar: {
          // 教程目录的 sidebar
          '/en/tutorial/': [
            {
              text: 'Tutorial Documentation',
              items: [
                { text: 'Quick Start', link: '/en/tutorial/getting-started' },
                { text: 'Contributing', link: '/en/tutorial/contributing' },
              ],
            },
          ],
          // 设计文档的 sidebar
          '/en/design/': [
            {
              text: 'Design Documentation',
              items: [
                { text: 'RFC', link: '/en/design/rfc/' },
                { text: 'Language Design', link: '/en/design/language-spec' },
                { text: 'Error Code', link: '/en/design/error-code' },
              ],
            },
          ],
          // 默认 sidebar
          '/en/': [
            {
              text: 'English Documentation',
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
  },
})
