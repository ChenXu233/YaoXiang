import { defineConfig } from 'vitepress'
import { generateSidebar } from 'vitepress-sidebar'
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

  // 忽略死链接
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
          { text: '参考', link: '/reference/' },
          { text: '社区', link: '/community/' },
          { text: '博客', link: '/blog/' },
        ],
        sidebar: {
          '/tutorial/': [
            {
              text: '教程文档',
              items: [
                { text: '快速开始', link: '/tutorial/getting-started' },
                { text: '贡献指南', link: '/tutorial/contributing' },
              ],
            },
          ],

          '/design/': [
            {
              text: '设计文档', 
              items: [
                { text: '设计目录', link: '/design' },
                { text: '语言规范', link: '/design/language-spec' },
                { text: '爻象宣言', link: '/design/manifesto' },
                { text: '爻象宣言 WTF 版', link: '/design/manifesto-wtf' },
                { text: '一个 2006 年出生者的语言设计观', link: '/design/2006-born-language-design' },
              ],
             },
            {
              text: 'RFC 文档',
              items: [
                { text: 'RFC 目录', link: '/design/rfc' },
                { text: 'RFC 模板', link: '/design/rfc/RFC_TEMPLATE' },
                { text: 'RFC 完整模板（示例）', link: '/design/rfc/EXAMPLE_full_feature_proposal' },
                {
                  text: '已接受',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/design/rfc/accepted',
                    useTitleFromFile: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '进行中',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/design/rfc/review',
                    useTitleFromFile: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '提案',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/design/rfc/draft',
                    useTitleFromFile: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '已拒绝',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/design/rfc/rejected',
                    useTitleFromFile: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
              ]
            },
          ],

          '/reference/': [
            {
              text: '参考文档',
              items: [
                { text: '参考目录', link: '/reference' },
              ],
            },
            {
              text: '错误码',
              items: generateSidebar({
                scanStartPath: '/reference/error-code',
                useTitleFromFile: true,
                collapsed: true,
                hyphenToSpace: true,
              })
            }
          ],

          '/': [
            {
              text: '中文文档',
              items: [
                { text: '快速开始', link: '/getting-started' },
                { text: '教程', link: '/tutorial/' },
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
          { text: 'Design', link: '/en/design/' },
          { text: 'Reference', link: '/en/reference/' },
          { text: 'Community', link: '/en/community/' },
          { text: 'Blog', link: '/en/blog/' },
        ],
        sidebar: {
          '/en/tutorial/': [
            {
              text: 'Tutorial Documentation',
              items: [
                { text: 'Quick Start', link: '/en/tutorial/getting-started' },
                { text: 'Contributing', link: '/en/tutorial/contributing' },
              ],
            },
          ],
          '/en/design/': [
            {
              text: 'Design Index',
              items: [
                { text: 'Design Index', link: '/en/design' },
                { text: 'Language Spec', link: '/en/design/language-spec' },
                { text: 'YaoXiang Manifesto', link: '/en/design/manifesto' },
                { text: 'YaoXiang Manifesto WTF', link: '/en/design/manifesto-wtf' },
                { text: 'A Language Design Observation from a 2006 Born', link: '/en/design/2006-born-language-design' },
              ],
             },
            {
              text: 'RFC Documentation',
              items: [
                { text: 'RFC Index', link: '/en/design/rfc' },
                { text: 'RFC Template', link: '/en/design/rfc/RFC_TEMPLATE' },
                { text: 'RFC Full Template (Example)', link: '/en/design/rfc/EXAMPLE_full_feature_proposal' },
                {
                  text: 'Accepted',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/en/design/rfc/accepted',
                    useTitleFromFile: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: 'Review',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/en/design/rfc/review',
                    useTitleFromFile: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: 'Draft',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/en/design/rfc/draft',
                    useTitleFromFile: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: 'Rejected',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/en/design/rfc/rejected',
                    useTitleFromFile: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
              ]
            },
          ],
          '/en/reference/': [
            {
              text: 'Reference Documentation',
              items: [
                { text: 'Reference Index', link: '/en/reference' },
              ],
            },
            {
              text: 'Error Codes',
              items: generateSidebar({
                scanStartPath: '/en/reference/error-code',
                useTitleFromFile: true,
                collapsed: true,
                hyphenToSpace: true,
              }),
            },
          ],
          '/en/': [
            {
              text: 'English Documentation',
              items: [
                { text: 'Quick Start', link: '/en/getting-started' },
                { text: 'Tutorial', link: '/en/tutorial/' },
              ],
            },
          ],
        },
      },
    },
  },
})
