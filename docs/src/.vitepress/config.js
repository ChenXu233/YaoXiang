import { defineConfig } from 'vitepress'
import { generateSidebar as _generateSidebar } from 'vitepress-sidebar'
import yaoxiangGrammar from './syntaxes/yaoxiang.tmLanguage.json'
import enI18n from './i18n/en.json'
import jaI18n from './i18n/ja.json'
import ruI18n from './i18n/ru.json'

// VitePress 源文件在 src/ 目录下，vitepress-sidebar 从 process.cwd() 解析路径
// 需要 documentRootPath: '/src' 让插件从 docs/src/ 开始扫描
const generateSidebar = (opts) => _generateSidebar({ documentRootPath: '/src', ...opts })

// 将 i18n JSON 的 sidebar key 加上 locale 前缀（/tutorial/ → /en/tutorial/）
function prefixSidebarKeys(sidebar, prefix) {
  const result = {}
  for (const [key, value] of Object.entries(sidebar)) {
    result[prefix + key.slice(1)] = value
  }
  return result
}

function makeLocale(i18n, prefix, lang, label) {
  return {
    lang,
    label,
    link: `/${prefix}/`,
    themeConfig: {
      nav: i18n.nav,
      sidebar: prefixSidebarKeys(i18n.sidebar, `/${prefix}/`),
    },
  }
}

export default defineConfig({
  base: '/YaoXiang/',
  title: 'YaoXiang',
  description: '一门面向未来的编程语言',

  // 排除有问题文件的目录
  srcExclude: [
    'archive/**',
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
      pattern: 'https://github.com/ChenXu233/yaoxiang/edit/main/docs/src/:path',
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
          { text: '指南', link: '/guide/' },
          { text: '参考', link: '/reference/' },
          {
            text: '更多',
            items: [
              { text: '设计', link: '/design/' },
              { text: '开发', link: '/dev/' },
              { text: '码场', link: '/playground/' },
              { text: '工具', link: '/tools/' },
              { text: '社区', link: '/community/' },
              { text: '博客', link: '/blog/' },
            ]
          },
          { component: 'VersionSwitcher' },
        ],
        sidebar: {
          '/tutorial/': [
            {
              text: '教程',
              items: [
                { text: '教程首页', link: '/tutorial/' },
                { text: '快速开始', link: '/tutorial/getting-started' },
                { text: '爻象手册', link: '/tutorial/YaoXiang-book' },
              ],
            },
            {
              text: '零基础入门',
              collapsed: true,
              items: generateSidebar({
                scanStartPath: '/tutorial/basics',
                useTitleFromFrontmatter: true,
                collapsed: true,
              }),
            },
            {
              text: '示例项目',
              collapsed: true,
              items: generateSidebar({
                scanStartPath: '/tutorial/examples',
                useTitleFromFrontmatter: true,
                collapsed: true,
                hyphenToSpace: true,
              }),
            },
          ],

          '/design/': [
            {
              text: '设计文档',
              items: [
                { text: '设计目录', link: '/design' },
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
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '进行中',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/design/rfc/review',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '草案',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/design/rfc/draft',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '已拒绝',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/design/rfc/rejected',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '已废弃',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/design/rfc/deprecated',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
              ],
            },
            {
              text: '工具设计',
              collapsed: true,
              items: [
                { text: 'check 命令', link: '/design/check/' },
                { text: '诊断系统', link: '/design/check/diagnostic-system' },
                { text: '跨文件分析', link: '/design/check/cross-file-analysis' },
                { text: '增量检查', link: '/design/check/incremental-checking' },
              ],
            },
            {
              text: '格式化规范',
              collapsed: true,
              items: [
                { text: '规范总览', link: '/design/formatter/' },
                {
                  text: '格式化规则',
                  collapsed: true,
                  items: [
                    { text: '规则总览', link: '/design/formatter/formatting-rules/' },
                    { text: '基础格式', link: '/design/formatter/formatting-rules/basic' },
                    { text: '函数和调用', link: '/design/formatter/formatting-rules/functions' },
                    { text: '类型系统', link: '/design/formatter/formatting-rules/types' },
                    { text: '数据结构', link: '/design/formatter/formatting-rules/data-structures' },
                    { text: '控制流', link: '/design/formatter/formatting-rules/control-flow' },
                    { text: '特殊语法', link: '/design/formatter/formatting-rules/special-syntax' },
                  ],
                },
                { text: '配置规范', link: '/design/formatter/configuration' },
                { text: '注释规范', link: '/design/formatter/comments' },
                { text: '错误处理', link: '/design/formatter/error-handling' },
                { text: 'CLI 规范', link: '/design/formatter/cli' },
              ],
            },
          ],

          '/reference/': [
            {
              text: '语言规范',
              items: [
                { text: '参考目录', link: '/reference' },
                { text: '语言规范总览', link: '/reference/language-spec/' },
                { text: '语法规范', link: '/reference/language-spec/syntax' },
                { text: '类型系统', link: '/reference/language-spec/type-system' },
                { text: '模块系统', link: '/reference/language-spec/modules' },
                { text: 'FFI', link: '/reference/language-spec/ffi' },
                { text: '并发模型', link: '/reference/language-spec/concurrency' },
                { text: '标准库', link: '/reference/language-spec/stdlib' },
              ],
            },
            {
              text: '标准库',
              items: [
                { text: '标准库总览', link: '/reference/stdlib/' },
              ],
            },
            {
              text: '错误与警告',
              items: [
                { text: '错误码总览', link: '/reference/error-code/' },
                {
                  text: '错误码分类',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/reference/error-code',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                { text: '警告码', link: '/reference/warning-code/warning-codes' },
              ],
            },
            {
              text: '包管理',
              collapsed: true,
              items: generateSidebar({
                scanStartPath: '/reference/package',
                useTitleFromFrontmatter: true,
                collapsed: true,
                hyphenToSpace: true,
              }),
            },
            {
              text: '工具命令',
              items: [
                { text: 'check 命令', link: '/reference/check-command' },
              ],
            },
          ],

          '/dev/': [
            {
              text: '开发文档',
              items: [
                { text: '开发目录', link: '/dev/' },
                { text: '贡献指南', link: '/dev/contributing' },
                { text: '提交指南', link: '/dev/commit-convention' },
                { text: '分支指南', link: '/dev/branch-maintenance-guide' },
                { text: '发布流程', link: '/dev/release' },
                { text: '测试规范', link: '/dev/test-specification' },
              ],
            },
            {
              text: '计划',
              items: [
                { text: '计划目录', link: '/dev/plan' },
                {
                  text: '处理中',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/dev/plan/ongoing',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: '任务',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/dev/plan/task',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
              ],
            },
            {
              text: '路线图',
              items: [
                { text: '路线图首页', link: '/dev/plan/roadmap' },
                {
                  text: '模块状态',
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: '/dev/plan/roadmap',
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
              ],
            },
          ],

          '/guide/': [
            {
              text: '指南',
              items: [
                { text: '指南目录', link: '/guide/' },
                { text: '包管理系统', link: '/guide/packaging' },
                { text: 'CI 集成', link: '/guide/ci-integration' },
                { text: 'REPL 交互环境', link: '/guide/repl' },
              ],
            },
          ],

          '/': [
            {
              text: '中文文档',
              items: [
                { text: '快速开始', link: '/getting-started' },
                { text: '教程', link: '/tutorial/' },
                { text: '指南', link: '/guide/' },
                { text: '参考', link: '/reference/' },
              ],
            },
          ],
        },
      },
    },

    en: makeLocale(enI18n, 'en', 'en', 'English'),
    ja: makeLocale(jaI18n, 'ja', 'ja', '日本語'),
    ru: makeLocale(ruI18n, 'ru', 'ru', 'Русский'),
  },
})
