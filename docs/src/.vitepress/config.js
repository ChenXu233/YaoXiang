import { defineConfig } from 'vitepress'
import { generateSidebar as _generateSidebar } from 'vitepress-sidebar'
import yaoxiangGrammar from './syntaxes/yaoxiang.tmLanguage.json'
import enI18n from './i18n/en.json'
import { tabsMarkdownPlugin } from 'vitepress-plugin-tabs'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { GitChangelog, GitChangelogMarkdownSection } from '@nolebase/vitepress-plugin-git-changelog'
import { readFileSync } from 'node:fs'

// 编译器版本单一来源：仓库根 Cargo.toml。文档里的 `<!-- yx-version -->`
// 占位在渲染期替换为该值（见 markdown.config），版本 bump 后文档零漂移。
// 读不到直接炸构建——版本注入失败不得静默渲染成占位符。
const cargoVersion = (() => {
  const cargo = readFileSync(new URL('../../../Cargo.toml', import.meta.url), 'utf-8')
  const m = cargo.match(/^version\s*=\s*"([^"]+)"/m)
  if (!m) {
    throw new Error('无法从仓库根 Cargo.toml 解析 version 字段，文档版本注入中止')
  }
  return m[1]
})()

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
  base: "/YaoXiang/",
  title: "YaoXiang",
  description: "一门面向未来的编程语言",

  // 排除有问题文件的目录
  // 注意：srcExclude 的 glob 相对于 src/ 解析，"archive/**" 只能匹配中文侧。
  // 英文侧位于 src/en/archive/，必须显式列出，否则归档文档会绕过排除规则被
  // 公开发布（2026-10-02 修复：dist/en/archive/*.html 长期对外可达，其中包含
  // 已归档的 v1.8 语言规范，而站点首页正在宣传「语言规范 v1.8」）。
  srcExclude: [
    "archive/**",
    "en/archive/**",
    "old/**",
    "en/old/**",
    "**/*.backup.md",
  ],

  // 最后更新时间
  lastUpdated: true,

  // 死链检查：不忽略任何链接。相对链接按文件位置解析，站点内与跨根链接
  // 均在构建时校验。原为 `true`（全局关闭，2026-05-29 引入），
  // 导致 287 个站内链接失效仍一直构绿。
  ignoreDeadLinks: false,

  // 代码高亮配置
  markdown: {
    theme: {
      light: "github-light",
      dark: "github-dark",
    },
    languages: [
      {
        id: "yaoxiang",
        scopeName: "source.yaoxiang",
        grammar: yaoxiangGrammar,
        name: "yaoxiang",
        aliases: ["yx"],
      },
    ],
    config(md) {
      md.use(tabsMarkdownPlugin)
      md.use(groupIconMdPlugin)
      md.use(GitChangelogMarkdownSection)

      // YaoXiang 的 f-string 用 `{{` / `}}` 做字面花括号转义（RFC-012），
      // 而 VitePress 把每个 .md 当 Vue 模板编译：行内代码 <code>里的 `{{`
      // 会被当成插值起始，构建期直接报 "Interpolation end sign was not found"。
      // 凡是讲到 f-string 转义的文档都会踩，因此在这里统一兜底：
      // 内容含 `{{` 的行内代码加 v-pre，跳过 Vue 编译。
      // （`}}` 单独出现不构成插值，无需处理。）
      const codeInline =
        md.renderer.rules.code_inline ||
        function (tokens, idx, options, env, slf) {
          const token = tokens[idx]
          return `<code>${md.utils.escapeHtml(token.content)}</code>`
        }
      md.renderer.rules.code_inline = (tokens, idx, options, env, slf) => {
        const token = tokens[idx]
        if (token.content.includes("{{")) {
          return `<code v-pre>${md.utils.escapeHtml(token.content)}</code>`
        }
        return codeInline(tokens, idx, options, env, slf)
      }

      // `<!-- yx-version -->` 渲染为 Cargo.toml 的 version（见文件顶部
      // cargoVersion）。占位选 HTML 注释是刻意的：翻译 bot 的 prompt 承诺
      // 逐字保留注释（vpi18n.config.json），zh→en 重译不会碰坏它。
      const htmlInline =
        md.renderer.rules.html_inline ||
        function (tokens, idx) {
          return tokens[idx].content
        }
      md.renderer.rules.html_inline = (tokens, idx) => {
        const content = tokens[idx].content
        if (content.includes("<!-- yx-version -->")) {
          return `<strong>${cargoVersion}</strong>`
        }
        return htmlInline(tokens, idx)
      }
    },
  },

  vite: {
    plugins: [
      groupIconVitePlugin(),
      GitChangelog({ maxGitLogCount: 5 }),
    ],
  },

  themeConfig: {
    logo: "/logo.png",

    socialLinks: [
      { icon: "github", link: "https://github.com/ChenXu233/yaoxiang" },
    ],

    editLink: {
      pattern: "https://github.com/ChenXu233/yaoxiang/edit/main/docs/src/:path",
      text: "在 GitHub 上编辑此页",
    },

    search: {
      provider: "local",
    },

    outline: "deep",
  },

  locales: {
    root: {
      lang: "zh-CN",
      label: "中文",
      link: "/",
      themeConfig: {
        nav: [
          { text: "首页", link: "/" },
          { text: "下载", link: "/download" },
          { text: "教程", link: "/tutorial/" },
          { text: "指南", link: "/guide/" },
          { text: "参考", link: "/reference/" },
          {
            text: "更多",
            items: [
              { text: "阐释", link: "/explanation/" },
              { text: "RFC", link: "/rfc/" },
              { text: "开发", link: "/dev/" },
              { text: "码场", link: "/playground/" },
              { text: "博客", link: "/blog/" },
            ],
          },
          { component: "VersionSwitcher" },
        ],
        sidebar: {
          "/tutorial/": [
            {
              text: "教程",
              items: [
                { text: "教程首页", link: "/tutorial/" },
                { text: "快速开始", link: "/tutorial/getting-started" },
              ],
            },
            {
              text: "零基础入门",
              collapsed: true,
              items: generateSidebar({
                scanStartPath: "/tutorial/basics",
                useTitleFromFrontmatter: true,
                collapsed: true,
              }),
            },
            {
              text: "进阶",
              collapsed: true,
              items: generateSidebar({
                scanStartPath: "/tutorial/advanced",
                useTitleFromFrontmatter: true,
                collapsed: true,
                hyphenToSpace: true,
              }),
            },
          ],

          "/explanation/": [
            {
              text: "阐释",
              items: [
                { text: "阐释目录", link: "/explanation/" },
                { text: "爻象宣言", link: "/explanation/manifesto" },
                { text: "爻象宣言 WTF 版", link: "/explanation/manifesto-wtf" },
                {
                  text: "一个 2006 年出生者的语言设计观",
                  link: "/explanation/2006-born-language-design",
                },
              ],
            },
          ],

          "/rfc/": [
            {
              text: "RFC 文档",
              items: [
                { text: "RFC 目录", link: "/rfc/" },
                { text: "RFC 模板", link: "/rfc/RFC_TEMPLATE" },
                {
                  text: "RFC 完整模板（示例）",
                  link: "/rfc/EXAMPLE_full_feature_proposal",
                },
                { text: "RFC 追踪表", link: "/rfc/TRACKING" },
                {
                  text: "已接受",
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: "/rfc/accepted",
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: "审核中",
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: "/rfc/review",
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: "草案",
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: "/rfc/draft",
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: "已拒绝",
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: "/rfc/rejected",
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: "已废弃",
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: "/rfc/deprecated",
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
              ],
            },
          ],

          "/reference/": [
            {
              text: "语言规范",
              items: [
                { text: "参考目录", link: "/reference" },
                { text: "语言规范总览", link: "/reference/language-spec/" },
                { text: "语法规范", link: "/reference/language-spec/syntax" },
                {
                  text: "类型系统",
                  link: "/reference/language-spec/type-system",
                },
                { text: "模块系统", link: "/reference/language-spec/modules" },
                { text: "FFI", link: "/reference/language-spec/ffi" },
                {
                  text: "并发模型",
                  link: "/reference/language-spec/concurrency",
                },
                { text: "标准库", link: "/reference/language-spec/stdlib" },
              ],
            },
            {
              text: "标准库",
              items: [
                { text: "标准库总览", link: "/reference/stdlib/" },
                { text: "std.string", link: "/reference/stdlib/string" },
                { text: "std.list", link: "/reference/stdlib/list" },
                { text: "std.dict", link: "/reference/stdlib/dict" },
                { text: "std.math", link: "/reference/stdlib/math" },
                { text: "std.io", link: "/reference/stdlib/io" },
                { text: "std.os", link: "/reference/stdlib/os" },
                { text: "std.time", link: "/reference/stdlib/time" },
                { text: "std.net", link: "/reference/stdlib/net" },
                {
                  text: "std.concurrent",
                  link: "/reference/stdlib/concurrent",
                },
                { text: "std.convert", link: "/reference/stdlib/convert" },
                { text: "std.result", link: "/reference/stdlib/result" },
                { text: "std.range", link: "/reference/stdlib/range" },
                { text: "std.assert", link: "/reference/stdlib/assert" },
                { text: "std.weak", link: "/reference/stdlib/weak" },
              ],
            },
            {
              text: "错误与警告",
              items: [
                { text: "错误码总览", link: "/reference/error-code/" },
                {
                  text: "错误码分类",
                  collapsed: true,
                  items: generateSidebar({
                    scanStartPath: "/reference/error-code",
                    useTitleFromFrontmatter: true,
                    collapsed: true,
                    hyphenToSpace: true,
                  }),
                },
                {
                  text: "警告码",
                  link: "/reference/warning-code/warning-codes",
                },
              ],
            },
            {
              text: "包管理",
              collapsed: true,
              items: generateSidebar({
                scanStartPath: "/reference/package",
                useTitleFromFrontmatter: true,
                collapsed: true,
                hyphenToSpace: true,
              }),
            },
            {
              text: "工具命令",
              items: [
                { text: "check 命令", link: "/reference/check-command" },
                { text: "format 命令", link: "/reference/format-command" },
                { text: "test 命令", link: "/reference/test-command" },
              ],
            },
          ],

          "/dev/": [
            {
              text: "实现者手册",
              items: [
                { text: "开发目录", link: "/dev/" },
                { text: "动工前自检（HOWTO）", link: "/dev/HOWTO" },
                { text: "代码规则（coding-rules）", link: "/dev/coding-rules" },
                { text: "文档规则（docs-rules）", link: "/dev/docs-rules" },
              ],
            },
            {
              text: "编译器架构（RFC-039 附属）",
              collapsed: true,
              items: [
                { text: "架构文档目录", link: "/dev/architecture/" },
                { text: "01 功能路由与依赖规范", link: "/dev/architecture/01-routing" },
                { text: "02 编译阶段契约与义务账本", link: "/dev/architecture/02-stage-contract" },
                { text: "03 类型表示单一化", link: "/dev/architecture/03-type-unification" },
                { text: "04 中间表示 SSA 化", link: "/dev/architecture/04-ssa" },
                { text: "05 前端范式：词法与语法", link: "/dev/architecture/05-frontend-paradigm" },
                { text: "06 死代码与空头设计清理", link: "/dev/architecture/06-cleanup-inventory" },
                { text: "07 重构等价性判据", link: "/dev/architecture/07-equivalence-oracle" },
                { text: "08 仓库维护机制与决策规程", link: "/dev/architecture/08-maintenance-mechanism" },
                { text: "09 多级施工任务表（WBS）", link: "/dev/architecture/09-execution-wbs" },
              ],
            },
            {
              text: "工具设计",
              collapsed: true,
              items: [
                { text: "check 命令", link: "/dev/design/check/" },
                { text: "诊断系统", link: "/dev/design/check/diagnostic-system" },
                { text: "跨文件分析", link: "/dev/design/check/cross-file-analysis" },
                { text: "增量检查", link: "/dev/design/check/incremental-checking" },
              ],
            },
            {
              text: "格式化规范",
              collapsed: true,
              items: [
                { text: "规范总览", link: "/dev/design/formatter/" },
                {
                  text: "格式化规则",
                  collapsed: true,
                  items: [
                    { text: "规则总览", link: "/dev/design/formatter/formatting-rules/" },
                    { text: "基础格式", link: "/dev/design/formatter/formatting-rules/basic" },
                    { text: "函数和调用", link: "/dev/design/formatter/formatting-rules/functions" },
                    { text: "类型系统", link: "/dev/design/formatter/formatting-rules/types" },
                    { text: "数据结构", link: "/dev/design/formatter/formatting-rules/data-structures" },
                    { text: "控制流", link: "/dev/design/formatter/formatting-rules/control-flow" },
                    { text: "特殊语法", link: "/dev/design/formatter/formatting-rules/special-syntax" },
                  ],
                },
                { text: "配置规范", link: "/dev/design/formatter/configuration" },
                { text: "注释规范", link: "/dev/design/formatter/comments" },
                { text: "错误处理", link: "/dev/design/formatter/error-handling" },
                { text: "CLI 规范", link: "/dev/design/formatter/cli" },
              ],
            },
            {
              text: "流程与规范",
              collapsed: true,
              items: [
                { text: "贡献指南", link: "/dev/contributing" },
                { text: "提交指南", link: "/dev/commit-convention" },
                { text: "分支指南", link: "/dev/branch-maintenance-guide" },
                { text: "发布流程", link: "/dev/release" },
                { text: "测试规范", link: "/dev/test-specification" },
              ],
            },
          ],

          "/guide/": [
            {
              text: "指南",
              items: [
                { text: "指南目录", link: "/guide/" },
                { text: "安装 YaoXiang", link: "/guide/installation" },
                { text: "语法速查", link: "/guide/language-overview" },
                { text: "模块系统", link: "/guide/modules" },
                { text: "包管理系统", link: "/guide/packaging" },
                { text: "CI 集成", link: "/guide/ci-integration" },
                { text: "REPL 交互环境", link: "/guide/repl" },
              ],
            },
          ],

          "/": [
            {
              text: "中文文档",
              items: [
                { text: "快速开始", link: "/tutorial/getting-started" },
                { text: "教程", link: "/tutorial/" },
                { text: "指南", link: "/guide/" },
                { text: "参考", link: "/reference/" },
              ],
            },
          ],
        },
      },
    },

    en: makeLocale(enI18n, "en", "en", "English"),
  },
});
