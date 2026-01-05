# RFC-006: 文档站点建设与优化方案

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2025-01-05

## 摘要

本 RFC 提出建立现代化的 YaoXiang 文档站点，整合当前分散的文档资源，提供更好的导航、搜索、国际化支持和用户体验，同时优化现有 `docs/` 目录的组织结构。

## 动机

### 为什么需要这个特性？

当前文档体系存在以下问题：

1. **文档分散**：文档散落在多个目录，缺乏统一入口
2. **导航困难**：新用户难以找到所需信息
3. **搜索缺失**：无法快速定位内容
4. **国际化不完整**：中英文文档不同步
5. **静态展示**：缺少交互式示例和代码运行

### 当前的问题

```
当前 docs/ 目录结构：
├── docs/
│   ├── README.md              # 主索引（内容有限）
│   ├── tutorial/              # 教程
│   │   ├── zh/               # 中文教程
│   │   └── en/               # 英文教程
│   ├── guides/               # 指南
│   ├── architecture/         # 架构文档
│   ├── design/               # 设计文档
│   ├── examples/             # 示例
│   ├── plans/                # 实施计划
│   ├── implementation/       # 实现文档
│   ├── maintenance/          # 维护文档
│   ├── archived/             # 归档
│   └── .obsidian/            # Obsidian 配置（不应该在版本控制中！）

问题：
1. 没有文档站点，仅靠 GitHub Readme 展示
2. .obsidian 目录混入版本控制
3. 文档质量参差不齐
4. 缺少版本切换功能
5. 搜索功能缺失
6. 没有代码高亮和运行示例
```

## 提案

### 核心架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang 文档站点                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────────────┐   │
│  │   用户访问   │   │   搜索      │   │   版本切换          │   │
│  └─────────────┘   └─────────────┘   └─────────────────────┘   │
│         │                │                    │                 │
│         └────────────────┼────────────────────┘                 │
│                          ▼                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    文档站点前端                          │   │
│  │  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │   │
│  │  │ 导航栏    │ │ 侧边栏    │ │ 内容区域            │   │   │
│  │  └───────────┘ └───────────┘ └─────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                          │                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                 静态站点生成器                           │   │
│  │    (VitePress / Docusaurus / Starlight / mdBook)        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                          │                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              CI/CD 流水线                                │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │   │
│  │  │ 文档构建 │ │ 链接检查 │ │ 翻译同步 │ │ 版本发布 │   │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                          │                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    内容源（Git）                         │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────┘
```

### 技术选型

| 组件 | 推荐方案 | 说明 |
|------|---------|------|
| 静态站点生成器 | **VitePress** 或 **Starlight** | 现代、Vue 生态、搜索内置 |
| 托管平台 | GitHub Pages / Vercel / Cloudflare Pages | 免费、易于部署 |
| 国际化 | VitePress i18n | 原生支持多语言 |
| 搜索 | Algolia DocSearch / 本地搜索 | 免费方案：local search |
| 代码高亮 | Shiki | 语法高亮质量高 |
| 代码运行 | YaoXiang Playground (WASM) | 交互式示例 |

### 推荐的静态站点生成器

#### 方案 A：VitePress + Starlight（推荐）

```bash
# 优点：
# - 开箱即用的搜索、导航、国际化
# - Starlight 提供完整文档模板
# - Vue 生态系统，性能优秀
# - 社区活跃，文档完善

# 缺点：
# - 需要 Node.js 环境
# - 学习曲线略陡
```

#### 方案 B：mdBook

```bash
# 优点：
# - Rust 生态，与项目技术栈一致
# - 简单轻量，构建速度快
# - 易于定制

# 缺点：
# - 搜索功能需要额外配置
# - 国际化支持较弱
# - 主题定制灵活性较低
```

#### 方案 C：Docusaurus

```bash
# 优点：
# - React 生态，功能丰富
# - 文档化程度高
# - 插件系统强大

# 缺点：
# - 较重，启动慢
# - 对非 React 项目集成度较低
```

**推荐方案**：VitePress + Starlight

### 目标文档站点结构

```
yaoxiang-docs/
├── docs/                           # 文档源文件
│   ├── .vitepress/
│   │   ├── config.mts              # 站点配置
│   │   ├── theme/
│   │   │   └── index.ts            # 主题配置
│   │   └── navbar.ts               # 导航栏配置
│   │
│   ├── index.md                    # 首页
│   ├── getting-started.md          # 快速开始
│   │
│   ├── guide/
│   │   ├── README.md
│   │   ├── installation.md
│   │   ├── hello-world.md
│   │   └── best-practices.md
│   │
│   ├── tutorial/
│   │   ├── README.md
│   │   ├── basics.md
│   │   ├── types.md
│   │   └── functions.md
│   │
│   ├── reference/
│   │   ├── README.md
│   │   ├── builtins.md
│   │   ├── standard-lib.md
│   │   └── syntax.md
│   │
│   ├── design/
│   │   ├── README.md
│   │   ├── language-spec.md
│   │   └── manifestos/
│   │       ├── manifesto.md
│   │       └── design-philosophy.md
│   │
│   ├── architecture/
│   │   ├── README.md
│   │   ├── compiler.md
│   │   ├── runtime.md
│   │   └── project-structure.md
│   │
│   ├── internals/
│   │   ├── README.md
│   │   ├── type-system.md
│   │   └── code-generation.md
│   │
│   ├── contributing/
│   │   ├── README.md
│   │   ├── how-to-contribute.md
│   │   └── commit-convention.md
│   │
│   └── public/
│       ├── favicon.ico
│       └── logo.svg
│
├── package.json
├── vite.config.ts
└── starlight.config.ts
```

### 核心功能

#### 1. 导航结构

```typescript
// .vitepress/config.ts
export default defineConfig({
  title: 'YaoXiang',
  description: '一门面向未来的编程语言',

  // 国际化配置
  locales: {
    root: {
      label: '中文',
      lang: 'zh-CN',
      link: '/zh/',
    },
    en: {
      label: 'English',
      lang: 'en-US',
      link: '/en/',
    },
  },

  // 社交链接
  social: {
    github: 'https://github.com/yaoxiang-lang/yaoxiang',
    discord: 'https://discord.gg/yaoxiang',
  },

  // 导航栏
  navbar: {
    left: [
      { text: '开始', link: '/zh/getting-started' },
      { text: '教程', link: '/zh/tutorial/' },
      { text: '参考', link: '/zh/reference/' },
      { text: '设计', link: '/zh/design/' },
    ],
    right: [
      { text: 'GitHub', link: 'https://github.com/yaoxiang-lang/yaoxiang' },
    ],
  },

  // 侧边栏
  sidebar: {
    '/zh/tutorial/': [
      {
        text: '教程',
        items: [
          { text: '快速开始', link: '/zh/tutorial/basics' },
          { text: '类型系统', link: '/zh/tutorial/types' },
          { text: '函数', link: '/zh/tutorial/functions' },
        ],
      },
    ],
  },

  // 搜索
  plugins: [
    starlight({
      title: 'YaoXiang',
      social: {
        github: 'https://github.com/yaoxiang-lang/yaoxiang',
      },
      // 本地搜索（免费方案）
      localSearch: {
        indexBy: 'title',
      },
      // 或 Algolia 搜索
      // algolia: {
      //   appId: '...',
      //   apiKey: '...',
      //   indexName: 'yaoxiang',
      // },
    }),
  ],
})
```

#### 2. 交互式代码示例

```markdown
# 示例：Hello World

```yaoxiang
// YaoXiang 代码示例
main() -> Int = {
    println("Hello, YaoXiang!")
    0
}
```

::: tip 运行效果
点击右上角 "Run" 按钮运行代码！
:::
```

#### 3. 版本切换

```typescript
// .vitepress/config.ts
export default defineConfig({
  // 版本管理
  lastUpdated: true,
  editLink: {
    pattern: 'https://github.com/yaoxiang-lang/yaoxiang/edit/main/docs/:path',
    text: '在 GitHub 上编辑此页',
  },
})
```

#### 4. 国际化同步

```bash
# 脚本：同步中英文文档
#!/bin/bash

# docs-sync.sh

for file in docs/guide/*.md; do
    en_file="docs/guide/en/$(basename $file)"
    zh_file="docs/guide/zh/$(basename $file)"

    # 检查文件是否存在
    if [ ! -f "$en_file" ]; then
        echo "Missing: $en_file"
    fi
    if [ ! -f "$zh_file" ]; then
        echo "Missing: $zh_file"
    fi
done
```

### CI/CD 集成

```yaml
# .github/workflows/docs-deploy.yml
name: Deploy Documentation

on:
  push:
    branches: [main]
    paths:
      - 'docs/**'
      - '!.obsidian/**'
  pull_request:
    paths:
      - 'docs/**'

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'

      - name: Install Dependencies
        run: |
          cd docs
          npm ci

      - name: Build Docs
        run: npm run build

      - name: Check Links
        run: npm run check-links

      - name: Deploy to Pages
        if: github.ref == 'refs/heads/main'
        uses: actions/deploy-pages@v4
        with:
          build_dir: docs/.vitepress/dist
```

## 详细设计

### 目录结构优化

```
当前问题：
1. .obsidian/ 混入版本控制
2. 文档分类不够清晰
3. 部分目录内容稀少

优化后的目录结构：
docs/
├── README.md                    # 站点首页（重定向到文档站）
├── docs/                        # 文档源文件（VitePress/Starlight）
│   ├── .vitepress/
│   │   ├── config.mts
│   │   └── theme/
│   ├── guide/                   # 使用指南
│   ├── tutorial/                # 教程
│   ├── reference/               # 参考文档
│   ├── design/                  # 设计文档
│   ├── architecture/            # 架构文档
│   ├── internals/               # 内部实现
│   ├── contributing/            # 贡献指南
│   └── public/                  # 静态资源
│
├── src/                         # 项目源码（保持不变）
│
├── examples/                    # 示例代码（独立）
│
└── scripts/                     # 工具脚本
    └── sync-docs.sh             # 文档同步脚本
```

### 迁移计划

| 阶段 | 操作 | 影响 |
|------|------|------|
| Phase 1 | 删除 `.obsidian/` | 减少仓库噪音 |
| Phase 2 | 创建文档站点基础配置 | 建立新架构 |
| Phase 3 | 迁移核心文档 | 保证内容完整性 |
| Phase 4 | 配置 CI/CD | 自动化部署 |
| Phase 5 | 优化搜索和导航 | 提升体验 |
| Phase 6 | 添加多语言支持 | 国际化 |

### 搜索功能

#### 本地搜索（推荐初始方案）

```typescript
import { localSearch } from 'vitepress-plugin-local-search'

export default {
  plugins: [
    localSearch({
      options: {
        // 排除特定路径
        exclude: ['/archived/', '/plans/'],
        // 索引选项
        tokenize: 'full',
      },
    }),
  ],
}
```

#### Algolia DocSearch（进阶方案）

1. 申请 [DocSearch](https://docsearch.algolia.com/)
2. 配置爬虫
3. 更新配置：

```typescript
export default {
  plugins: [
    starlight({
      algolia: {
        appId: 'YOUR_APP_ID',
        apiKey: 'YOUR_API_KEY',
        indexName: 'yaoxiang',
      },
    }),
  ],
}
```

### 代码示例系统

```typescript
// .vitepress/theme/playground.ts
import { definePlayground } from 'vitepress-plugin-playground'

export default definePlayground({
  // 配置 YaoXiang Playground
  compilerUrl: '/yaoxiang-compiler.wasm',
  defaultCode: `main() -> Int = {
    println("Hello, YaoXiang!")
    0
}`,
})
```

## 权衡

### 优点

- **专业文档站**：提升项目形象和可信度
- **良好导航**：用户快速找到所需信息
- **搜索功能**：快速定位内容
- **国际化**：支持多语言社区
- **交互式示例**：提升学习体验
- **自动化部署**：降低维护成本

### 缺点

- **维护成本**：需要维护站点配置和构建流程
- **技术栈引入**：引入 Node.js 技术栈
- **内容同步**：多语言内容需要保持同步
- **部署依赖**：依赖外部托管服务

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| 仅用 GitHub Wiki | 简单但功能有限 | 搜索差，定制性低 |
| Docusaurus | 功能丰富但较重 | 学习曲线陡，性能一般 |
| mkdocs-material | Python 生态 | 非项目技术栈 |
| 仅 README | 最简单 | 功能最弱 |
| GitBook | 商业服务限制 | 付费，定制受限 |

## 实现策略

### 阶段划分

1. **Phase 1: 基础搭建**（v0.3）
   - 删除 `.obsidian/` 目录
   - 初始化 VitePress + Starlight 配置
   - 配置基础导航和侧边栏
   - 部署到 GitHub Pages

2. **Phase 2: 内容迁移**（v0.4）
   - 迁移核心文档（getting-started, tutorial）
   - 优化文档结构
   - 添加代码高亮

3. **Phase 3: 搜索与导航**（v0.5）
   - 配置本地搜索
   - 优化导航结构
   - 添加面包屑导航

4. **Phase 4: 国际化**（v0.6）
   - 配置中英文双语
   - 建立翻译同步机制
   - 国际化SEO优化

5. **Phase 5: 高级功能**（v0.7）
   - 集成 YaoXiang Playground
   - 添加交互式示例
   - 实现版本切换

### 依赖关系

- Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5（顺序依赖）
- 无外部 RFC 依赖

### 风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 内容丢失 | 文档不可用 | 迁移前完整备份 |
| 维护成本 | 团队负担 | 自动化流程，简化配置 |
| 构建失败 | 站点不可用 | CI 检查，回滚机制 |

## 开放问题

- [ ] 是否使用 Starlight 主题还是纯 VitePress？
- [ ] 选择哪个托管平台？（GitHub Pages / Vercel / Cloudflare Pages）
- [ ] 是否需要实现 YaoXiang Playground？
- [ ] 翻译流程如何管理？（人工 vs 机器翻译）

---

## 附录

### 附录A：当前文档清单

| 文档 | 路径 | 状态 | 优先级 |
|------|------|------|--------|
| README | docs/README.md | 需更新 | P0 |
| 快速开始 | docs/guides/getting-started.md | 需迁移 | P0 |
| 教程 | docs/tutorial/ | 需迁移 | P0 |
| 语言规范 | docs/design/language-spec.md | 需迁移 | P1 |
| 架构设计 | docs/architecture/ | 需迁移 | P1 |
| 贡献指南 | docs/guides/dev/commit-convention.md | 需迁移 | P2 |

### 附录B：推荐的文档元数据

```yaml
---
title: 快速开始
description: 学习如何安装和运行第一个 YaoXiang 程序
lastUpdated: 2025-01-05
editLink: true
sidebar:
  label: 快速开始
  order: 1
---
```

### 附录C：文档审核清单

- [ ] 标题清晰准确
- [ ] 有适当的摘要或引言
- [ ] 包含可运行的代码示例
- [ ] 有相关链接和参考文献
- [ ] 语法正确，无错别字
- [ ] 中英文术语一致
- [ ] 符合项目代码风格

### 附录D：术语表

| 术语 | 定义 |
|------|------|
| SSG | Static Site Generator，静态站点生成器 |
| i18n | Internationalization，国际化 |
| l10n | Localization，本地化 |
| CI/CD | Continuous Integration/Continuous Deployment，持续集成/持续部署 |

---

## 参考文献

- [VitePress 文档](https://vitepress.dev/)
- [Starlight 文档](https://starlight.astro.build/)
- [VuePress 对比](https://vitepress.dev/guide/comparisons)
- [Rust 文档实践](https://rust-lang.github.io/rfcs/1574-more-api-documentation-conventions.html)
