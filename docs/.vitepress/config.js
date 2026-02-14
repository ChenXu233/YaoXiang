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
    ],
  },

  themeConfig: {
    logo: '/logo.png',

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
      link: '/src/',
      themeConfig: {
        nav: [
          { text: '首页', link: '/' },
          { text: '下载', link: '/src/download' },
          { text: '教程', link: '/src/tutorial/' },
          { text: '码场', link: '/src/playground/' },
          { text: '工具', link: '/src/tools/' },
          { text: '设计', link: '/src/design/' },
          { text: '参考', link: '/src/reference/' },
          { text: '社区', link: '/src/community/' },
          { text: '博客', link: '/src/blog/' },
          { component: 'VersionSwitcher' },
        ],
        sidebar: {
          '/src/tutorial/': [
            {
              text: '教程文档',
              items: [
                { text: '教程首页', link: '/src/tutorial/' },
                { text: '快速开始', link: '/src/tutorial/getting-started' },
                { text: '爻象手册', link: '/src/tutorial/YaoXiang-book' },
                {
                  text: '零基础入门', 
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/tutorial/basics',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                  }),
                 },
                {
                  text: '贡献指南',
                  collapsed: true,
                  items: [
                    { text: '贡献指南', link: '/src/tutorial/dev/contributing' },
                    { text: '提交指南', link: '/src/tutorial/dev/commit-convention' },
                    { text: '分支指南', link: '/src/tutorial/dev/branch-maintenance-guide' },
                  ]
                },
              ],
            },
          ],

          '/src/design/': [
            {
              text: '设计文档', 
              items: [
                { text: '设计目录', link: '/src/design' },
                { text: '语言规范', link: '/src/design/language-spec' },
                { text: '爻象宣言', link: '/src/design/manifesto' },
                { text: '爻象宣言 WTF 版', link: '/src/design/manifesto-wtf' },
                { text: '一个 2006 年出生者的语言设计观', link: '/src/design/2006-born-language-design' },
              ],
             },
            {
              text: 'RFC 文档',
              items: [
                { text: 'RFC 目录', link: '/src/design/rfc' },
                { text: 'RFC 模板', link: '/src/design/rfc/RFC_TEMPLATE' },
                { text: 'RFC 完整模板（示例）', link: '/src/design/rfc/EXAMPLE_full_feature_proposal' },
                {
                  text: '已接受',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/design/rfc/accepted',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '进行中',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/design/rfc/review',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '草案',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/design/rfc/draft',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '已拒绝',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/design/rfc/rejected',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
              ]
            },
          ],

          '/src/reference/': [
            {
              text: '参考文档',
              items: [
                { text: '参考目录', link: '/src/reference' },
              ],
            },
            {
              text: '错误码',
              items: generateSidebar({
                scanStartPath: '/src/reference/error-code',
                useTitleFromFrontmatter: true,
                collapsed: true,
                hyphenToSpace: true,
              })
            },
            {
              text: '计划',
              items: [
                { text: '计划目录', link: '/src/reference/plan' },
                {
                  text: '处理中', 
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/reference/plan/ongoing',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '已完成', 
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/reference/plan/completed',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                 },
              ],
            }
          ],

          '/': [
            {
              text: '中文文档',
              items: [
                { text: '快速开始', link: '/src/getting-started' },
                { text: '教程', link: '/src/tutorial/' },
              ],
            },
          ],
        },
      },
    },
    en: {
      lang: 'en-US',
      label: 'English',
      link: '/src/en/',
      themeConfig: {
        nav: [
          { text: 'Home', link: '/src/en/' },
          { text: 'Download', link: '/src/en/download' },
          { text: 'Tutorial', link: '/src/en/tutorial/' },
          { text: 'Playground', link: '/src/en/playground/' },
          { text: 'Tools', link: '/src/en/tools/' },
          { text: 'Design', link: '/src/en/design/' },
          { text: 'Reference', link: '/en/reference/' },
          { text: 'Community', link: '/en/community/' },
          { text: 'Blog', link: '/en/blog/' },
          { component: 'VersionSwitcher' },
        ],
        sidebar: {
          '/src/en/tutorial/': [
            {
              text: 'Tutorial Documentation',
              items: [
                { text: 'Tutorial Index', link: '/src/en/tutorial/' },
                { text: 'Quick Start', link: '/src/en/tutorial/getting-started' },
                { text: 'YaoXiang Handbook', link: '/src/en/tutorial/YaoXiang-book' },
                { text: 'Zero-to-Hero', 
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/en/tutorial/basics',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: 'Contribution Guide',
                  collapsed: true,
                  items: [
                    { text: 'Contribution Guide', link: '/src/en/tutorial/dev/contributing' },
                    { text: 'Commit Guide', link: '/src/en/tutorial/dev/commit-convention' },
                    { text: 'Branch Maintenance Guide', link: '/src/en/tutorial/dev/branch-maintenance-guide' },
                  ]
                },
              ],
            },
          ],
          '/src/en/design/': [
            {
              text: 'Design Index',
              items: [
                { text: 'Design Index', link: '/src/en/design' },
                { text: 'Language Spec', link: '/src/en/design/language-spec' },
                { text: 'YaoXiang Manifesto', link: '/src/en/design/manifesto' },
                { text: 'YaoXiang Manifesto WTF', link: '/src/en/design/manifesto-wtf' },
                { text: 'A Language Design Observation from a 2006 Born', link: '/src/en/design/2006-born-language-design' },
              ],
             },
            {
              text: 'RFC Documentation',
              items: [
                { text: 'RFC Index', link: '/src/en/design/rfc' },
                { text: 'RFC Template', link: '/src/en/design/rfc/RFC_TEMPLATE' },
                { text: 'RFC Full Template (Example)', link: '/src/en/design/rfc/EXAMPLE_full_feature_proposal' },
                {
                  text: 'Accepted',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/en/design/rfc/accepted',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: 'Review',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/en/design/rfc/review',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: 'Draft',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/en/design/rfc/draft',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: 'Rejected',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/en/design/rfc/rejected',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
              ]
            },
          ],
          '/src/en/reference/': [
            {
              text: 'Reference Documentation',
              items: [
                { text: 'Reference Index', link: '/src/en/reference' },
              ],
            },
            {
              text: 'Error Codes',
              items: generateSidebar({
                scanStartPath: '/src/en/reference/error-code',
                useTitleFromFrontmatter: true,
                collapsed: true,
                hyphenToSpace: true,
              }),
            },
          ],
          '/en/': [
            {
              text: 'English Documentation',
              items: [
                { text: 'Quick Start', link: '/src/en/getting-started' },
                { text: 'Tutorial', link: '/src/en/tutorial/' },
              ],
            },
          ],
        },
      },
    },
  },
})
