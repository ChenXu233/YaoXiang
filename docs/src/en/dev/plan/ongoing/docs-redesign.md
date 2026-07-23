---
title: "Documentation Site Page Redesign"
status: "In Discussion"
created: "2026-06-17"
---

# Documentation Site Page Redesign

## Goals

Redesign the layout of 6 custom pages, unify the visual language, and beautify all pages.

## Scope

### Custom Layout Pages (6)

| Page | File | Current Component | Notes |
|------|------|------------------|-------|
| Home | `index.md` | `Home.vue` | Keep existing design as baseline |
| Download | `download.md` | `Download.vue` | Redesign |
| Community | `community.md` | `Community.vue` | Redesign |
| Playground | `playground.md` | `Playground.vue` | Redesign |
| Tools | `tools.md` | None (bare HTML) | Need to create Tools.vue |
| Blog | `blog/index.md` | None (bare HTML) | Need to create Blog.vue |

### Global Beautification

Adjust global CSS colors, fonts, and spacing for all default VitePress pages (tutorials, guides, reference, development docs), without changing layout structure.

## Architectural Decisions

## Component Strategy

Retain the functional logic of daisyUI components (btn/badge/card/dropdown etc.), with visuals overridden by `yaoxiang` theme tokens, no additional wrappers needed.

Only extract truly repetitive page structures into Vue components:

| Component | Reuse | Props |
|-----------|-------|-------|
| `HeroSection.vue` | Download/Community/Playground/Tools/Blog — 5 pages share same Hero | `badge`, `title`, `description`, default slot (extra content below badge) |
| `TerminalWindow.vue` | Home code display / Download install commands / Community git log | `title?`, `shadow` (primary/secondary), default slot |
| `TimelineTrack.vue` | Home tracks / Blog post list | `dotColor?`, `tag?`, `date?`, default slot |
| `FeatureCard.vue` | Download platforms / Tools cards / Community entrances | `icon?`, `title`, `description`, `status?`, default slot (bottom actions) |

Not extracted: PageShell (VitePress is sufficient), UnderConstruction (inline is fine).

### i18n Compatibility

All content requiring translation stays in markdown frontmatter, Vue components only handle rendering. Blog post list provided via `virtual:blog-index` virtual module.

### Design Language Baseline

Using `Home.vue` as baseline:

- Fonts: `Space Mono` (global) + `Microsoft YaHei` (Chinese titles) + `JetBrains Mono` (code)
- Offset shadows: `shadow-[Xpx_Xpx_0px_color]` (no blur, hard shadows)
- Cards: `card bg-base-200 rounded-none shadow-[6px_6px_0px_var(--p)]` (sharp corners + thick offset shadow + elevated from page background)
- Timeline: `w-1 bg-base-300` backbone + `rounded-full` circular nodes
- Tilted tags: `rotate-2` + `shadow-lg` + `rounded-none`
- Footer: `bg-neutral text-neutral-content rounded-none`
- Border-radius strategy: Only code windows `rounded-lg`, circular nodes `rounded-full`, everything else `rounded-none`

## Color System

### Ink and Vermillion System

Using traditional calligraphy/seal carving/rubbings as metaphor, light colors like rice paper with vermillion seals (warm tones), dark colors like stone inscriptions (cool tones).

daisyUI v5 custom theme, using `@plugin "daisyui/theme"` OKLCH format.

### Light Theme: `yaoxiang` (default)

| Token | Value | Usage |
|-------|-------|-------|
| `base-100` | `oklch(98% 0.008 85)` | Rice paper page background |
| `base-200` | `oklch(93% 0.016 85)` | Cards/Hero gradient |
| `base-300` | `oklch(88% 0.03 85)` | Borders/Timeline |
| `base-content` | `oklch(18% 0.02 85)` | Ink body text |
| `primary` | `oklch(55% 0.24 28)` | Vermillion seal · Primary emphasis |
| `primary-content` | `oklch(98% 0.005 85)` | Text on vermillion seal |
| `secondary` | `oklch(45% 0.05 85)` | Seal paste · Secondary emphasis |
| `secondary-content` | `oklch(98% 0.005 85)` | |
| `accent` | `oklch(60% 0.12 180)` | Cyan-green · Code highlighting |
| `accent-content` | `oklch(15% 0.02 180)` | |
| `neutral` | `oklch(28% 0.02 85)` | Ink background · Footer |
| `neutral-content` | `oklch(90% 0.01 85)` | |
| `--radius-box` | `0rem` | Card/modal border-radius |
| `--radius-selector` | `0rem` | Selector border-radius |
| `--radius-field` | `0rem` | Button/input border-radius |

### Dark Theme: `yaoxiang-dark`

| Token | Value | Usage |
|-------|-------|-------|
| `base-100` | `oklch(14% 0.008 250)` | Stone page background |
| `base-200` | `oklch(20% 0.01 250)` | Cards/Hero gradient |
| `base-300` | `oklch(26% 0.015 250)` | Borders/Timeline |
| `base-content` | `oklch(82% 0.01 250)` | Rubbing white body text |
| `primary` | `oklch(58% 0.20 18)` | Cool vermillion · Primary emphasis |
| `primary-content` | `oklch(98% 0.005 85)` | Text on cool vermillion |
| `secondary` | `oklch(48% 0.04 245)` | Stone gray · Secondary emphasis |
| `secondary-content` | `oklch(98% 0.01 250)` | |
| `accent` | `oklch(60% 0.10 185)` | Cool cyan · Code highlighting |
| `accent-content` | `oklch(15% 0.02 185)` | |
| `neutral` | `oklch(10% 0.008 250)` | Deep stone · Footer |
| `neutral-content` | `oklch(80% 0.01 250)` | |
| `--radius-box` | `0rem` | |
| `--radius-selector` | `0rem` | |
| `--radius-field` | `0rem` | |

### Primary/Secondary Color Division

- **Primary (Vermillion Seal)**: Hero Badge, CTA buttons, primary card shadows, section title badges, key emphasis
- **Secondary (Seal Paste/Stone Gray)**: Secondary card shadows, secondary tag chips, timeline secondary nodes
- **Card Hierarchy**: Page `base-100` → Cards `base-200` (brightness difference ~5-6%), primary cards `shadow-primary`, secondary cards `shadow-secondary`
- **Accent (Cyan-Green)**: Code syntax highlighting reserved keywords

### Font System (consistent with index.md)

| Usage | Font Stack |
|-------|-----------|
| Global body | `Space Mono`, monospace |
| Chinese titles | `Microsoft YaHei`, `SimHei`, `PingFang SC`, `Heiti SC`, sans-serif |
| Code | `JetBrains Mono`, monospace |

Google Fonts loading: `JetBrains Mono:wght@400;700` + `Space Mono:ital,wght@0,400;0,700;1,400`

## Blog Index

### virtual:blog-index Plugin

Custom VitePress plugin (Vite virtual module), no CI dependency.

- Path: `.vitepress/plugins/blog-index.ts`
- Scan `blog/*.md` at build time, extract frontmatter (title, date, description)
- Sort by date, output JSON
- Dev mode watches file changes, HMR auto-updates
- Blog pages get data via `import { blogPosts } from 'virtual:blog-index'`

## New Plugins

| Plugin | Purpose | Extra Config |
|--------|---------|--------------|
| `vitepress-plugin-group-icons` | Code block file type icons | None |
| `vitepress-plugin-back-to-top` | Back to top button | None (styled by theme override) |
| `vitepress-plugin-nprogress` | Page loading progress bar | `color: oklch(55% 0.24 28)` |
| `@vuepress/plugin-reading-time` | Reading time + word count | None (blog pages only) |
| `vitepress-plugin-tabs` | Markdown tab syntax | None |
| `@nolebase/vitepress-plugin-git-changelog` | Per-page contributors + changelog | `maxGitLogCount: 5` |

Note: Plugins need to be installed one by one to confirm VitePress 1.6.4 compatibility, replace with alternatives if necessary.

## Page Layout Solutions

### Tools Page (Tools.vue) · New

Change from bare HTML to Vue component.

| Element | Solution |
|---------|----------|
| Hero | TOOLS badge + Chinese title + description |
| Card area | Three feature cards (compiler/formatter/LSP), icon + title + description + status tag |
| Status | Compiler/Formatter → `Available` (primary badge), LSP → `In Development` (cool semi-transparent badge) |

### Blog List Page (Blog.vue) · New

Timeline layout, consistent with Home Tracks:

| Element | Solution |
|---------|----------|
| Hero | BLOG badge + Chinese title + description |
| Timeline | backbone (w-1 bg-base-300) + `rounded-full` nodes |
| Post cards | bg-base-200 + shadow-secondary hard shadow + date + title + summary + tilted NEW tag |
| Data source | `virtual:blog-index` virtual module |
| Empty state | Dashed placeholder "More articles coming soon..." |

### Community Page (Community.vue)

Complete restructure, clean up old structure. Principles: generous whitespace restraint, cards only for terminal and bulletin board, badges throughout the page, data scattered into semantic areas.

**Layout (top to bottom):**

| Area | Solution |
|------|----------|
| Hero | Grid background + Chinese title + tagline + a row of fun Badges (each with micro-rotation + individual animation, hover to pop and enlarge) |
| Maintainers | Left border `border-l-3 border-primary` + avatar (shadow on hover, enlarge) + CORE/SOLO Badge next to name + contribution count naturally embedded in bio |
| Contributors | PR / Issue two columns, dashed empty state frames guiding participation |
| Recent Activity | Two columns: terminal git log (commit count merged at bottom) + bulletin board (Meetup/Conf/Contributor Day, status Badge embedded in title) |
| Get Involved | Three underline links (GitHub with Stars Badge beside it, contribution guide with Open Issues Badge beside it), hover arrow slides right + underline color change |

**Badge System (reusable classes):**

6 variants — hot (vermillion + shadow + tilt), cool (secondary color + wobble), teal (cyan-green + tilt), glow (border pulse breathing), muted (gray + tilt), soft (small gray). Mixed throughout the page, not as separate blocks.

**No longer kept:**
- mockup-window terminal (changed to mini-console)
- Independent stats card area (data scattered)
- border-l-4 left border description blocks
- avatar online status

### Playground Page (Playground.vue)

Keep CodeMirror editor core, replace entire shell.

| Element | Solution |
|---------|----------|
| Hero | PLAYGROUND badge + Chinese title + description |
| Editor | Terminal three dots + filename `main.yx` + bottom status bar (line number/column/indent/encoding) |
| Top right | Compiler version annotation `v0.1.0 · WASM` |
| Run button | Primary background + offset shadow + press displacement |
| Output panel | Secondary color shadow + title bar showing compile time |
| Shortcuts | Three groups below run button (Ctrl+Enter Run / Ctrl+S Share / Ctrl+K Format) |
| Path | Playground.vue path correction (theme/index.js import path vs actual file location) |

### Download Page (Download.vue)

Keep existing functionality, align with color system, layout structure unchanged.

**Change list:**

| Element | Change |
|---------|--------|
| Main title | New "Download YaoXiang" (Chinese `SimHei`) |
| Subtitle | "TYPE THE UNIVERSE" Space Mono font-black |
| Description | "Choose your platform, start building the world." Space Mono, removed `>>_` |
| Hero badge | `rounded-full` → `rounded-none` + offset shadow |
| Terminal window | Removed `border-l-2 border-success`, shadow changed to primary color. Annotation "Coming Soon", explaining quick install not yet available |
| Version selector | `rounded-lg` → `rounded-none`, annotation "Built from GitHub Release" |
| Platform cards | `bg-base-100` → `bg-base-200`, shadow changed to secondary color, arch badge kept |
| New WASM card | accent cyan-green border, copy "No install needed, use right in browser" "Try in browser ↗", links to playground |
| macOS copy | "Universal binary (.tar.gz) + supports both M-series and Intel chips" |
| Linux copy | "Static binary (.tar.gz) + musl compiled, no runtime dependencies" |
| Fonts | Space Mono globally (original Download.vue had no Microsoft YaHei) |
| 3D mouse tracking | Keep (global hover item handled uniformly) |
| Bottom two columns | `align-items: stretch` height alignment |

## Other Implementation Items

### Global CSS Rewrite (tailwind.css)

1. `@import "tailwindcss"`
2. `@plugin "daisyui/theme"` light `yaoxiang` (default, prefersdark: false)
3. `@plugin "daisyui/theme"` dark `yaoxiang-dark` (prefersdark: true)
4. VitePress `--vp-c-brand` and other variables mapped to primary token
5. Scrollbar thumb color `oklch(55% 0.24 28)`
6. Navbar VPNavBarMenu fix preserved

### Mouse Tracking Optimization

Home.vue + Download.vue: Wrap with `useIntersectionObserver` or `v-if`, activate `useMouse` only when component is visible, pause 3D tilt calculation when not visible.

### Playground.vue Path Correction

Import path in `theme/index.js` L9 inconsistent with actual file location, unify to one place.

## Pending Items

- [x] Color system redesign (Ink and Vermillion: light rice paper vermillion seal + dark stone rubbing)
- [x] 6 page layout designs (Download/Community/Playground/Tools/Blog, Home unchanged)
- [x] Component extraction and reuse (4 components: HeroSection / TerminalWindow / TimelineTrack / FeatureCard, rest use daisyUI)
- [x] 6 plugin installations and configurations (6 plugins + nprogress color + git-changelog maxGitLogCount)
- [ ] blog-index virtual module implementation (design ready, pending coding)
- [ ] Global CSS rewrite (tailwind.css → two @plugin "daisyui/theme")
- [ ] Remove global mouse tracking (activate useMouse only when component visible)
- [ ] Playground.vue path correction (theme/index.js import path)

## Related

- RFC-006: Documentation Site Optimization