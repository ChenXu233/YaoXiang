# RFC-006: 文档站点建设

> **状态**: 审核中
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2025-02-07

> **参考**: 查看 [RFC 模板](RFC_TEMPLATE.md) 了解 RFC 规范。

## 摘要

建立 YaoXiang 文档站点，整合分散文档，提供搜索、导航、多语言和版本切换支持。

## 动机

### 为什么需要这个特性？

当前文档散落在多个目录，仅靠 GitHub Readme 展示，新用户难以找到所需信息，无法搜索，中英文文档不同步。

### 当前的问题

```
docs/
├── README.md              # 主索引（内容有限）
├── tutorial/              # 教程
├── guides/               # 指南
├── architecture/          # 架构文档
├── design/               # 设计文档
├── examples/             # 示例
├── plans/                # 实施计划
├── implementation/       # 实现文档
├── maintenance/          # 维护文档
└── archived/             # 归档
```

问题：
1. 无统一入口，仅靠 GitHub Readme
2. 无法搜索
3. 无版本切换，用户可能阅读过时文档
4. .obsidian 混入版本控制

## 提案

### 核心设计

```
┌─────────────────────────────────────────────────────────┐
│                    文档站点前端                          │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ 导航栏    │ │ 侧边栏    │ │ 版本切换下拉菜单     │   │
│  └───────────┘ └───────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              VitePress + Starlight                      │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              GitHub Pages（托管）                        │
└─────────────────────────────────────────────────────────┘
```

### 目录结构（核心设计）

```
docs/
├── .vitepress/
│   ├── config.mts              # 站点配置
│   ├── navbar.ts              # 导航栏配置
│   └── sidebar/               # 侧边栏配置
│       ├── zh.ts
│       └── en.ts
│
├── public/
│   ├── favicon.ico
│   └── logo.svg
│
├── zh/                        # 中文文档
│   ├── index.md               # 中文首页
│   ├── getting-started.md
│   ├── tutorial/
│   │   └── README.md
│   ├── reference/
│   │   └── README.md
│   ├── guide/
│   └── contributing.md
│
└── en/                        # 英文文档
    ├── index.md
    └── getting-started.md
```

### URL 路径规范（核心设计）

| 场景 | URL 格式 | 说明 |
|------|---------|------|
| 最新版中文 | `/zh/getting-started/` | 跳转到最新版本 |
| 最新版英文 | `/en/getting-started/` | 跳转到最新版本 |
| 指定版本 | `/v0.5/zh/getting-started/` | 版本号前缀 |
| 首页 | `/zh/` 或 `/en/` | 语言首页 |

**版本切换设计**：
```
版本切换下拉菜单：
├── v0.6 (latest)
├── v0.5
├── v0.4
└── v0.3
```

**版本路径规范**（关键决策，后期难改）：
- 最新版：`/zh/xxx/` → 重定向到最新版本
- 指定版本：`/v0.5/zh/xxx/` → 固定版本
- 导航栏版本切换：切换 `/v0.5/` 和 `/zh/` 的组合

### 侧边栏规范

```typescript
// docs/.vitepress/sidebar/zh.ts
export default {
  '/zh/tutorial/': [
    {
      text: '教程',
      items: [
        { text: '快速开始', link: '/zh/getting-started' },
        { text: '基础', link: '/zh/tutorial/basics' },
      ],
    },
  ],
  '/zh/reference/': [
    {
      text: '参考',
      items: [
        { text: '内置函数', link: '/zh/reference/builtins' },
      ],
    },
  ],
}
```

### CI/CD 集成

```yaml
# .github/workflows/docs-deploy.yml
name: Deploy Docs

on:
  push:
    branches: [main]
    paths: ['docs/**', '!.obsidian/**']

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
        working-directory: docs
      - run: npm run build
      - uses: actions/deploy-pages@v4
        with:
          build_dir: docs/.vitepress/dist
```

## 详细设计

### 导航栏配置

```typescript
// docs/.vitepress/navbar.ts
export default [
  { text: '开始', link: '/zh/getting-started' },
  { text: '教程', link: '/zh/tutorial/' },
  { text: '参考', link: '/zh/reference/' },
  { text: '设计', link: '/zh/design/' },
  { text: 'GitHub', link: 'https://github.com/yaoxiang-lang/yaoxiang' },
]
```

### 站点配置

```typescript
// docs/.vitepress/config.mts
import { defineConfig } from 'vitepress'
import starlight from '@astrojs/starlight'

export default defineConfig({
  title: 'YaoXiang',
  description: '一门面向未来的编程语言',

  locales: {
    root: { label: '中文', lang: 'zh-CN', link: '/zh/' },
    en: { label: 'English', lang: 'en-US', link: '/en/' },
  },

  // 本地搜索
  plugins: [
    starlight({
      title: 'YaoXiang',
      localSearch: {},
    }),
  ],

  // 编辑链接
  editLink: {
    pattern: 'https://github.com/yaoxiang-lang/yaoxiang/edit/main/docs/:path',
  },
})
```

## 权衡

### 优点

- 专业文档站提升项目形象
- 用户快速找到所需信息
- 本地搜索免费够用
- 多语言支持服务国际社区
- 版本切换避免阅读过时文档

### 缺点

- 维护成本：需要维护站点配置
- 技术栈引入：Node.js

## 替代方案

| 方案 | 为什么不选 |
|------|-----------|
| GitHub Wiki | 搜索差，定制性低 |
| 仅 README | 无搜索、无导航 |
| Docusaurus | 较重，启动慢 |

## 实现策略

### 阶段划分

| 阶段 | 内容 | 状态 |
|------|------|------|
| P0 | 初始化 VitePress + Starlight 配置 | 待办 |
| P0 | 配置目录结构、导航栏、侧边栏 | 待办 |
| P0 | 迁移 README + 快速开始 | 待办 |
| P0 | CI/CD 自动部署到 GitHub Pages | 待办 |
| P1 | 迁移教程、参考文档 | 待办 |
| P1 | 配置版本切换菜单 | 待办 |
| P2 | 补充英文文档 | 待办 |

### 依赖关系

无外部 RFC 依赖

### 风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 内容丢失 | 迁移前完整备份 |

## 开放问题

**无** - 所有决策已做出

---

## 附录

### 附录A：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| SSG 选型 | VitePress + Starlight | 2025-02-07 | 晨煦 |
| 托管平台 | GitHub Pages | 2025-02-07 | 晨煦 |
| 搜索方案 | 本地搜索 | 2025-02-07 | 晨煦 |
| 多语言结构 | `/zh/` 和 `/en/` 前缀 | 2025-02-07 | 晨煦 |
| 版本路径 | `/v0.5/zh/` 格式 | 2025-02-07 | 晨煦 |

---

## 参考文献

- [VitePress 文档](https://vitepress.dev/)
- [Starlight 文档](https://starlight.astro.build/)
