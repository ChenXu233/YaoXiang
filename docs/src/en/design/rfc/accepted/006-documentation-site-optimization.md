---
title: 'RFC-006: Documentation Site Construction'
status: 'Accepted'
author: 'Chenxu'
created: '2025-01-05'
updated: '2026-07-05'

issue: '#130'
---

# RFC-006: Documentation Site Construction

> **Reference**: See [RFC Template](../RFC_TEMPLATE.md) to understand the RFC specification.

## Summary

Build the YaoXiang documentation site, consolidate scattered documentation, and provide search,
navigation, multi-language, and version switching support.

## Motivation

### Why is this feature needed?

Currently, documentation is scattered across multiple directories and is only displayed via the
GitHub Readme. New users have difficulty finding the information they need, there is no search, and
the Chinese and English documentation are not kept in sync.

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
└── archived/             # Archives
```

Problems:

1. No unified entry point, only the GitHub Readme
2. No search capability
3. No version switching, users may read outdated documentation
4. .obsidian mixed into version control

## Proposal

### Core Design

```
┌─────────────────────────────────────────────────────────┐
│                 Documentation Site Frontend             │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ Navbar    │ │ Sidebar   │ │ Version Switch Dropdown│  │
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

### URL Path Specification (Core Design)

| Scenario         | URL Format                  | Description                 |
| ---------------- | --------------------------- | --------------------------- |
| Latest Chinese   | `/zh/getting-started/`      | Redirects to latest version |
| Latest English   | `/en/getting-started/`      | Redirects to latest version |
| Specific version | `/v0.5/zh/getting-started/` | Version number prefix       |
| Homepage         | `/zh/` or `/en/`            | Language homepage           |

**Version Switching Design**:

```
Version Switch Dropdown:
├── v0.6 (latest)
├── v0.5
├── v0.4
└── v0.3
```

**Version Path Specification** (Key decision, hard to change later):

- Latest version: `/zh/xxx/` → Redirects to the latest version
- Specific version: `/v0.5/zh/xxx/` → Fixed version
- Navbar version switching: Toggle the combination of `/v0.5/` and `/zh/`

### Sidebar Specification

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
      items: [{ text: '内置函数', link: '/zh/reference/builtins' }],
    },
  ],
};
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
];
```

### Site Configuration

```typescript
// docs/.vitepress/config.mts
import { defineConfig } from 'vitepress';
import starlight from '@astrojs/starlight';

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
});
```

## Trade-offs

### Advantages

- A professional documentation site enhances the project's image
- Users can quickly find the information they need
- Local search is free and sufficient
- Multi-language support serves the international community
- Version switching avoids reading outdated documentation

### Disadvantages

- Maintenance cost: Site configuration needs to be maintained
- Technology stack introduction: Node.js

## Alternatives

| Option      | Why Not Chosen                  |
| ----------- | ------------------------------- |
| GitHub Wiki | Poor search, low customization  |
| README only | No search, no navigation        |
| Docusaurus  | Relatively heavy, slow to start |

## Implementation Strategy

### Phases

| Phase | Content                                        | Status |
| ----- | ---------------------------------------------- | ------ |
| P0    | Initialize VitePress + Starlight config        | Todo   |
| P0    | Configure directory structure, navbar, sidebar | Todo   |
| P0    | Migrate README + Quick Start                   | Todo   |
| P0    | CI/CD auto-deploy to GitHub Pages              | Todo   |
| P1    | Migrate tutorials and reference documentation  | Todo   |
| P1    | Configure version switch menu                  | Todo   |
| P2    | Add English documentation                      | Todo   |

### Dependencies

No external RFC dependencies

### Risks

| Risk         | Impact | Mitigation                   |
| ------------ | ------ | ---------------------------- |
| Content loss | Major  | Full backup before migration |

## Open Questions

**None** - All decisions have been made

---

## Appendix

### Appendix A: Design Decision Records

| Decision                 | Choice                   | Date       | Recorder |
| ------------------------ | ------------------------ | ---------- | -------- |
| SSG selection            | VitePress + Starlight    | 2025-02-07 | Chenxu   |
| Hosting platform         | GitHub Pages             | 2025-02-07 | Chenxu   |
| Search solution          | Local search             | 2025-02-07 | Chenxu   |
| Multi-language structure | `/zh/` and `/en/` prefix | 2025-02-07 | Chenxu   |
| Version path             | `/v0.5/zh/` format       | 2025-02-07 | Chenxu   |

---

## References

- [VitePress Documentation](https://vitepress.dev/)
- [Starlight Documentation](https://starlight.astro.build/)
