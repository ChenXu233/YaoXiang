---
title: "RFC-006: Documentation Site Construction"
status: "Accepted"
author: "Chenxu"
created: "2025-01-05"
updated: "2026-07-05"

issue: "#130"
---

# RFC-006: Documentation Site Construction

> **Reference**: See [RFC Template](RFC_TEMPLATE.md) for RFC specifications.

## Summary

Build a YaoXiang documentation site, consolidate scattered documentation, and provide search, navigation, multi-language, and version switching support.

## Motivation

### Why is this feature needed?

Currently documentation is scattered across multiple directories, displayed only through GitHub Readme, making it difficult for new users to find the information they need, with no search capability, and Chinese/English docs are out of sync.

### Current Problems

```
docs/
├── README.md              # Main index (limited content)
├── tutorial/              # Tutorials
├── guides/               # Guides
├── architecture/          # Architecture docs
├── design/               # Design docs
├── examples/             # Examples
├── plans/                # Implementation plans
├── implementation/       # Implementation docs
├── maintenance/          # Maintenance docs
└── archived/             # Archive
```

Problems:
1. No unified entry point, relies only on GitHub Readme
2. No search capability
3. No version switching, users may read outdated documentation
4. `.obsidian` files mixed into version control

## Proposal

### Core Design

```
┌─────────────────────────────────────────────────────────┐
│               Documentation Site Frontend                │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ Navbar    │ │ Sidebar   │ │ Version Switcher     │   │
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
│              GitHub Pages (Hosting)                     │
└─────────────────────────────────────────────────────────┘
```

### Directory Structure (Core Design)

```
docs/
├── .vitepress/
│   ├── config.mts              # Site configuration
│   ├── navbar.ts              # Navbar configuration
│   └── sidebar/               # Sidebar configuration
│       ├── zh.ts
│       └── en.ts
│
├── public/
│   ├── favicon.ico
│   └── logo.svg
│
├── zh/                        # Chinese documentation
│   ├── index.md               # Chinese homepage
│   ├── getting-started.md
│   ├── tutorial/
│   │   └── README.md
│   ├── reference/
│   │   └── README.md
│   ├── guide/
│   └── contributing.md
│
└── en/                        # English documentation
    ├── index.md
    └── getting-started.md
```

### URL Path Convention (Core Design)

| Scenario | URL Format | Description |
|------|---------|------|
| Latest Chinese | `/zh/getting-started/` | Redirects to latest version |
| Latest English | `/en/getting-started/` | Redirects to latest version |
| Specific version | `/v0.5/zh/getting-started/` | Version prefix |
| Homepage | `/zh/` or `/en/` | Language homepage |

**Version Switching Design**:
```
Version Switcher Dropdown:
├── v0.6 (latest)
├── v0.5
├── v0.4
└── v0.3
```

**Version Path Convention** (Key decision, hard to change later):
- Latest version: `/zh/xxx/` → Redirects to latest version
- Specific version: `/v0.5/zh/xxx/` → Fixed version
- Navbar version switch: Toggle between `/v0.5/` and `/zh/` combinations

### Sidebar Convention

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

### CI/CD Integration

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

## Detailed Design

### Navbar Configuration

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

### Site Configuration

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

  // Local search
  plugins: [
    starlight({
      title: 'YaoXiang',
      localSearch: {},
    }),
  ],

  // Edit link
  editLink: {
    pattern: 'https://github.com/yaoxiang-lang/yaoxiang/edit/main/docs/:path',
  },
})
```

## Trade-offs

### Advantages

- Professional documentation site enhances project image
- Users quickly find the information they need
- Local search is free and sufficient
- Multi-language support serves the international community
- Version switching prevents reading outdated documentation

### Disadvantages

- Maintenance cost: site configuration needs to be maintained
- Tech stack introduction: Node.js

## Alternatives

| Option | Why Not Chosen |
|------|-----------|
| GitHub Wiki | Poor search, low customization |
| README only | No search, no navigation |
| Docusaurus | Heavier, slower startup |

## Implementation Strategy

### Phasing

| Phase | Content | Status |
|------|------|------|
| P0 | Initialize VitePress + Starlight configuration | Todo |
| P0 | Configure directory structure, navbar, sidebar | Todo |
| P0 | Migrate README + Getting Started | Todo |
| P0 | CI/CD auto-deploy to GitHub Pages | Todo |
| P1 | Migrate tutorials, reference docs | Todo |
| P1 | Configure version switcher menu | Todo |
| P2 | Supplement English documentation | Todo |

### Dependencies

No external RFC dependencies

### Risks

| Risk | Impact | Mitigation |
|------|------|---------|
| Content loss | Complete backup before migration |

## Open Questions

**None** - All decisions have been made

---

## Appendix

### Appendix A: Design Decision Record

| Decision | Resolution | Date | Recorder |
|------|------|------|--------|
| SSG Selection | VitePress + Starlight | 2025-02-07 | Chenxu |
| Hosting Platform | GitHub Pages | 2025-02-07 | Chenxu |
| Search Solution | Local search | 2025-02-07 | Chenxu |
| Multi-language Structure | `/zh/` and `/en/` prefix | 2025-02-07 | Chenxu |
| Version Path | `/v0.5/zh/` format | 2025-02-07 | Chenxu |

---

## References

- [VitePress Documentation](https://vitepress.dev/)
- [Starlight Documentation](https://starlight.astro.build/)