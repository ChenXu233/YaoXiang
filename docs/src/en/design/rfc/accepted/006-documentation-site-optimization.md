---
title: 'RFC-006: Documentation Site Construction'
---

# RFC-006: Documentation Site Construction

> **Status**: Accepted
> **Author**: ChenXu
> **Created Date**: 2025-01-05
> **Last Updated**: 2026-02-12 (No type syntax changes)

> **Reference**: See [RFC Template](RFC_TEMPLATE.md) for RFC specifications.

## Summary

Build YaoXiang documentation site, integrate scattered documents, provide search, navigation, multilingual, and version switching support.

## Motivation

### Why is this feature needed?

Current documents are scattered across multiple directories, only displayed via GitHub Readme, new users cannot find needed information, no search, Chinese and English documents out of sync.

### Current Problems

```
docs/
├── README.md              # Main index (limited content)
├── tutorial/              # Tutorials
├── guides/               # Guides
├── architecture/          # Architecture documents
├── design/               # Design documents
├── examples/             # Examples
├── plans/                # Implementation plans
├── implementation/       # Implementation documents
├── maintenance/          # Maintenance documents
└── archived/             # Archived
```

Problems:
1. No unified entry, only via GitHub Readme
2. No search capability
3. No version switching, users may read outdated documents
4. .obsidian mixed into version control

## Proposal

### Core Design

```
┌─────────────────────────────────────────────────────────┐
│                    Documentation Site Frontend            │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ Navbar    │ │ Sidebar   │ │ Version Switch Menu │   │
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
│              GitHub Pages (Hosted)                     │
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
│   └── favicon.ico
│
├── index.md                    # Home page
├── getting-started.md          # Getting started guide
├── installation.md            # Installation guide
│
├── tutorial/                   # Tutorials (Chinese)
│   ├── index.md
│   ├── hello-world.md
│   └── ...
│
├── guides/                    # Guides (Chinese)
│   ├── index.md
│   └── ...
│
├── reference/                 # Reference docs (Chinese)
│   ├── index.md
│   ├── standard-library.md
│   └── ...
│
├── design/                    # Design docs (Chinese)
│   ├── index.md
│   ├── accepted/
│   └── rfc/
│
├── en/                        # English docs
│   ├── index.md
│   ├── tutorial/
│   ├── guides/
│   ├── reference/
│   └── design/
│
└── README.md                  # GitHub entrance
```

### Features

| Feature | Description |
|---------|-------------|
| **Search** | Full-text search powered by MiniSearch |
| **Navigation** | Navbar + sidebar hierarchical navigation |
| **i18n** | Chinese/English language switching |
| **Versioning** | Version switching via dropdown |
| **Dark Mode** | Dark/light theme toggle |
| **Code Copy** | One-click code copying |
| **Mobile Support** | Responsive design for mobile |

### Technical Stack

| Component | Technology |
|-----------|------------|
| **Framework** | VitePress |
| **Plugin** | Starlight |
| **Hosting** | GitHub Pages |
| **Search** | MiniSearch |
| **Deployment** | GitHub Actions |

## Implementation

### Phase 1: Basic Site (Completed)

| Task | Status |
|------|--------|
| VitePress setup | ✅ |
| Starlight integration | ✅ |
| Chinese docs migration | ✅ |
| Navigation configuration | ✅ |

### Phase 2: Search & i18n (In Progress)

| Task | Status |
|------|--------|
| Search integration | 🔄 |
| English docs | 🔄 |
| Version switching | ⏳ |

### Phase 3: Advanced Features (Future)

| Feature | Status |
|---------|--------|
| Version switching | ⏳ |
| API documentation | ⏳ |
| Interactive examples | ⏳ |

## Migration Plan

### Document Migration Checklist

- [x] Integrate scattered docs into site structure
- [x] Fix broken links
- [x] Add frontmatter metadata
- [ ] Add English translations
- [ ] Verify code examples
- [ ] Add search keywords

### CI/CD Pipeline

```yaml
# .github/workflows/docs.yml
name: Docs

on:
  push:
    branches: [main]
    paths: [docs/**]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v4
      - run: pnpm install
      - run: pnpm docs:build
      - uses: amondnet/vercel-action@v25
        with:
          vercel-token: ${{ secrets.VERCEL_TOKEN }}
          vercel-org-id: ${{ secrets.VERCEL_ORG_ID }}
          vercel-project-id: ${{ secrets.VERCEL_PROJECT_ID }}
          vercel-args: '--prod'
```

---

## Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Framework | VitePress + Starlight | 2025-01-05 | ChenXu |
| Hosting | GitHub Pages | 2025-01-05 | ChenXu |
| Search | MiniSearch | 2025-02-07 | ChenXu |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| VitePress | Vue-powered static site generator |
| Starlight | Documentation framework built on VitePress |
| GitHub Pages | Static site hosting service |
| i18n | Internationalization |
