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
          { text: '指南', link: '/src/guide/' },
          { text: '参考', link: '/src/reference/' },
          {
            text: '更多',
            items: [
              { text: '设计', link: '/src/design/' },
              { text: '开发', link: '/src/dev/' },
              { text: '码场', link: '/src/playground/' },
              { text: '工具', link: '/src/tools/' },
              { text: '社区', link: '/src/community/' },
              { text: '博客', link: '/src/blog/' },
            ]
          },
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
              ],
            },
          ],

          '/src/design/': [
            {
              text: '设计文档',
              items: [
                { text: '设计目录', link: '/src/design' },
                { text: '语言规范', link: '/src/reference/language-spec/' },
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
                { text: '语言规范', link: '/src/reference/language-spec/' },
                { text: '语法规范', link: '/src/reference/language-spec/syntax' },
                { text: '类型系统', link: '/src/reference/language-spec/type-system' },
                { text: '模块系统', link: '/src/reference/language-spec/modules' },
                { text: '并发模型', link: '/src/reference/language-spec/concurrency' },
                { text: '标准库', link: '/src/reference/language-spec/stdlib' },
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
              text: '警告码',
              items: generateSidebar({
                scanStartPath: '/src/reference/warning-code',
                useTitleFromFrontmatter: true,
                collapsed: true,
                hyphenToSpace: true,
              }),
            },
            {
              text: '包管理系统',
              collapsed: true,
              items: [
                generateSidebar({
                scanStartPath: '/src/reference/package',
                useTitleFromFrontmatter: true,
                collapsed: true,
                hyphenToSpace: true,
                })
              ]
            },
          ],

          '/src/dev/': [
            {
              text: '开发文档',
              items: [
                { text: '开发目录', link: '/src/dev/' },
                { text: '贡献指南', link: '/src/dev/contributing' },
                { text: '提交指南', link: '/src/dev/commit-convention' },
                { text: '分支指南', link: '/src/dev/branch-maintenance-guide' },
              ],
            },
            {
              text: '计划',
              items: [
                { text: '计划目录', link: '/src/dev/plan' },
                {
                  text: '处理中',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/dev/plan/ongoing',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '已完成',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/src/dev/plan/completed',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                 },
              ],
            },
          ],

          '/src/guide/': [
            {
              text: '指南文档',
              items: [
                { text: '指南目录', link: '/src/guide/' },
                { text: '包管理系统', link: '/src/guide/packaging' },
              ],
            },
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
  },
})
