---
title: "RFC-006: Documentation Site Construction"
status: "Accepted"
author: "晨煦"
created: "2025-01-05"
updated: "2026-02-12"
---

# RFC-006: Documentation Site Construction

> **Reference**: See [RFC Template](RFC_TEMPLATE.md) for RFC standards.

## Summary

Establish a YaoXiang documentation site, consolidate scattered documentation, and provide search, navigation, multilingual, and version switching support.

## Motivation

### Why is this feature needed?

Currently, documentation is scattered across multiple directories and only displayed via GitHub README. New users have difficulty finding the information they need, there is no search functionality, and Chinese and English documentation are not synchronized.

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
1. No unified entry point, relying only on GitHub README
2. No search capability
3. No version switching, users may read outdated documentation
4. `.obsidian` mixed into version control

## Proposal

### Core Design

```
┌─────────────────────────────────────────────────────────┐
│                    Documentation Site Frontend          │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ Navbar    │ │ Sidebar   │ │ Version Switch       │   │
│  │           │ │           │ │ Dropdown Menu        │   │
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
|----------|-----------|-------------|
| Latest (Chinese) | `/zh/getting-started/` | Redirects to latest version |
| Latest (English) | `/en/getting-started/` | Redirects to latest version |
| Specific version | `/v0.5/zh/getting-started/` | Version number prefix |
| Homepage | `/zh/` or `/en/` | Language homepage |

**Version Switching Design**:
```
Version Switch Dropdown:
├── v0.6 (latest)
├── v0.5
├── v0.4
└── v0.3
```

**Version Path Convention** (Key decision, difficult to change later):
- Latest version: `/zh/xxx/` → Redirects to latest version
- Specific version: `/v0.5/zh/xxx/` → Fixed version
- Navbar version switching: Switch combinations of `/v0.5/` and `/zh/`

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
        { text: 'Builtins', link: '/zh/reference/builtins' },
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
  { text: 'Getting Started', link: '/zh/getting-started' },
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

## Trade-offs

### Pros

- Professional documentation site enhances project image
- Users can quickly find the information they need
- Local search is free and sufficient
- Multilingual support serves international community
- Version switching prevents reading outdated documentation

### Cons

- Maintenance cost: Need to maintain site configuration
- Tech stack introduction: Node.js

## Alternatives

| Approach | Why Not Chosen |
|----------|---------------|
| GitHub Wiki | Poor search, low customizability |
| README only | No search, no navigation |
| Docusaurus | Heavier, slower startup |

## Implementation Strategy

### Phases

| Phase | Content | Status |
|-------|---------|--------|
| P0 | Initialize VitePress + Starlight configuration | Todo |
| P0 | Configure directory structure, navbar, sidebar | Todo |
| P0 | Migrate README + Quick Start | Todo |
| P0 | CI/CD auto-deploy to GitHub Pages | Todo |
| P1 | Migrate tutorial, reference documentation | Todo |
| P1 | Configure version switching menu | Todo |
| P2 | Supplement English documentation | Todo |

### Dependencies

No external RFC dependencies

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Content loss | Complete backup before migration |

## Open Questions

**None** - All decisions have been made

---

## Appendices

### Appendix A: Design Decision Record

| Decision | Decision Made | Date | Recorder |
|----------|--------------|------|----------|
| SSG Selection | VitePress + Starlight | 2025-02-07 | 晨煦 |
| Hosting Platform | GitHub Pages | 2025-02-07 | 晨煦 |
| Search Solution | Local search | 2025-02-07 | 晨煦 |
| Multilingual Structure | `/zh/` and `/en/` prefixes | 2025-02-07 | 晨煦 |
| Version Path | `/v0.5/zh/` format | 2025-02-07 | 晨煦 |

---

## References

- [VitePress Documentation](https://vitepress.dev/)
- [Starlight Documentation](https://starlight.astro.build/)