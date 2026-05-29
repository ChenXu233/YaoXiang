---
title: "RFC-006: Documentation Site Construction"
---

# RFC-006: Documentation Site Construction

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2026-02-12

> **Reference**: See [RFC Template](RFC_TEMPLATE.md) for RFC conventions.

## Summary

Establish a YaoXiang documentation site, consolidating scattered documentation and providing search, navigation, multi-language, and version switching support.

## Motivation

### Why is this feature needed?

Current documentation is scattered across multiple directories, only displayed through GitHub Readme, making it difficult for new users to find the information they need, with no search capability and unsynchronized Chinese/English documentation.

### Current Problems

```
docs/
├── README.md              # Main index (limited content)
├── tutorial/              # Tutorials
├── guides/               # Guides
├── architecture/          # Architecture documentation
├── design/               # Design documentation
├── examples/             # Examples
├── plans/                # Implementation plans
├── implementation/       # Implementation documentation
├── maintenance/          # Maintenance documentation
└── archived/             # Archived
```

Problems:
1. No unified entry point, only relying on GitHub Readme
2. No search capability
3. No version switching, users may read outdated documentation
4. .obsidian mixed into version control

## Proposal

### Core Design

```
┌─────────────────────────────────────────────────────────┐
│                  Documentation Site Frontend             │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ Navbar    │ │ Sidebar   │ │ Version Switch      │   │
│  │           │ │           │ │ Dropdown Menu       │   │
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
|----------|------------|-------------|
| Latest Chinese | `/zh/getting-started/` | Redirect to latest version |
| Latest English | `/en/getting-started/` | Redirect to latest version |
| Specific version | `/v0.5/zh/getting-started/` | Version number prefix |
| Homepage | `/zh/` or `/en/` | Language homepage |

**Version Switching Design**:
```
Version dropdown menu:
├── v0.6 (latest)
├── v0.5
├── v0.4
└── v0.3
```

**Version Path Convention** (key decision, difficult to change later):
- Latest version: `/zh/xxx/` → redirect to latest version
- Specific version: `/v0.5/zh/xxx/` → fixed version
- Navbar version switching: switch combination of `/v0.5/` and `/zh/`

### Sidebar Convention

```typescript
// docs/.vitepress/sidebar/zh.ts
export default {
  '/zh/tutorial/': [
    {
      text: 'Tutorial',
      items: [
        { text: 'Quick Start', link: '/zh/getting-started' },
        { text: 'Basics', link: '/zh/tutorial/basics' },
      ],
    },
  ],
  '/zh/reference/': [
    {
      text: 'Reference',
      items: [
        { text: 'Built-in Functions', link: '/zh/reference/builtins' },
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
  { text: 'Get Started', link: '/zh/getting-started' },
  { text: 'Tutorial', link: '/zh/tutorial/' },
  { text: 'Reference', link: '/zh/reference/' },
  { text: 'Design', link: '/zh/design/' },
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
  description: 'A programming language for the future',

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

## Tradeoffs

### Pros

- Professional documentation site enhances project image
- Users can quickly find the information they need
- Local search is free and sufficient
- Multi-language support serves international community
- Version switching prevents reading outdated documentation

### Cons

- Maintenance cost: need to maintain site configuration
- Technology stack introduction: Node.js

## Alternative Solutions

| Solution | Why Not Chosen |
|----------|---------------|
| GitHub Wiki | Poor search, low customization |
| README Only | No search, no navigation |
| Docusaurus | Heavier, slower startup |

## Implementation Strategy

### Phase Breakdown

| Phase | Content | Status |
|-------|---------|--------|
| P0 | Initialize VitePress + Starlight configuration | Pending |
| P0 | Configure directory structure, navbar, sidebar | Pending |
| P0 | Migrate README + Quick Start | Pending |
| P0 | CI/CD auto-deploy to GitHub Pages | Pending |
| P1 | Migrate tutorials, reference documentation | Pending |
| P1 | Configure version switching menu | Pending |
| P2 | Complete English documentation | Pending |

### Dependencies

No external RFC dependencies

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Content loss | Complete backup before migration |

## Open Questions

**None** - All decisions have been made

---

## Appendix

### Appendix A: Design Decision Record

| Decision | Resolution | Date | Recorder |
|----------|-----------|------|----------|
| SSG Selection | VitePress + Starlight | 2025-02-07 | Chen Xu |
| Hosting Platform | GitHub Pages | 2025-02-07 | Chen Xu |
| Search Solution | Local Search | 2025-02-07 | Chen Xu |
| Multi-language Structure | `/zh/` and `/en/` prefixes | 2025-02-07 | Chen Xu |
| Version Path | `/v0.5/zh/` format | 2025-02-07 | Chen Xu |

---

## References

- [VitePress Documentation](https://vitepress.dev/)
- [Starlight Documentation](https://starlight.astro.build/)