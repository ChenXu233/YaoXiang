```markdown
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

| Page | File | Current Component | Description |
|------|------|---------|------|
| Home | `index.md` | `Home.vue` | Keep existing design as the baseline |
| Download | `download.md` | `Download.vue` | Redesign |
| Community | `community.md` | `Community.vue` | Redesign |
| Playground | `playground.md` | `Playground.vue` | Redesign |
| Tools | `tools.md` | None (raw HTML) | Need to create Tools.vue |
| Blog | `blog/index.md` | None (raw HTML) | Need to create Blog.vue |

### Global Beautification

All default VitePress pages (tutorials, guides, references, development docs, etc.) adjust global CSS colors, fonts, and spacing without changing the layout structure.

## Architecture Decisions

## Componentization Strategy

The functional logic of daisyUI components (btn/badge/card/dropdown etc.) is retained, with visuals uniformly overridden by `yaoxiang` theme tokens, no extra wrapping needed.

Only extract truly repeated page structures into Vue components:

| Component | Reuse | Props |
|------|------|-------|
| `HeroSection.vue` | Download/Community/Playground/Tools/Blog — consistent Hero across 5 pages | `badge`, `title`, `description`, default slot (extra content below the badge) |
| `TerminalWindow.vue` | Home page code display / Download install commands / Community git log | `title?`, `shadow` (primary/secondary), default slot |
| `TimelineTrack.vue` | Home page tracks / Blog article list | `dotColor?`, `tag?`, `date?`, default slot |
| `FeatureCard.vue` | Download platforms / Tool cards / Community entries | `icon?`, `title`, `description`, `status?`, default slot (bottom action) |

Not extracted: PageShell (VitePress is sufficient), UnderConstruction (inline is enough).

### i18n Compatibility

All content that needs translation remains in markdown frontmatter; Vue components only handle rendering. The blog article list is provided through the `virtual:blog-index` virtual module.

### Design Language Baseline

Use `Home.vue` as the baseline:

- Fonts: `Space Mono` (global) + `Microsoft YaHei` (Chinese titles) + `JetBrains Mono` (code)
- Offset shadow: `shadow-[Xpx_Xpx_0px_color]` (no blur, hard shadow)
- Cards: `card bg-base-200 rounded-none shadow-[6px_6px_0px_var(--p)]` (sharp corners + thick offset shadow + elevated page base color)
- Timeline: `w-1 bg-base-300` backbone + `rounded-full` circular nodes
- Tilted labels: `rotate-2` + `shadow-lg` + `rounded-none`
- Footer: `bg-neutral text-neutral-content rounded-none`
- Rounded corner strategy: only code windows `rounded-lg`, circular nodes `rounded-full`, everything else `rounded-none`

## Color System

### Ink-Vermilion Color System

Using traditional calligraphy/seal carving/rubbing as metaphor, light colors are like vermilion seals on rice paper (warm tone), dark colors are like stone tablet rubbings (cool tone).

daisyUI v5 custom theme, using `@plugin "daisyui/theme"` OKLCH format.

### Light Theme: `yaoxiang` (default)

| Token | Value | Purpose |
|-------|-----|------|
| `base-100` | `oklch(98% 0.008 85)` | Rice paper page background |
| `base-200` | `oklch(93% 0.016 85)` | Card/Hero gradient |
| `base-300` | `oklch(88% 0.03 85)` | Border/Timeline |
| `base-content` | `oklch(18% 0.02 85)` | Ink body text |
| `primary` | `oklch(55% 0.24 28)` | Vermilion seal · Primary emphasis |
| `primary-content` | `oklch(98% 0.005 85)` | Text on vermilion seal |
| `secondary` | `oklch(45% 0.05 85)` | Seal paste · Secondary emphasis |
| `secondary-content` | `oklch(98% 0.005 85)` | |
| `accent` | `oklch(60% 0.12 180)` | Cyan-green · Code highlighting |
| `accent-content` | `oklch(15% 0.02 180)` | |
| `neutral` | `oklch(28% 0.02 85)` | Ink base · Footer |
| `neutral-content` | `oklch(90% 0.01 85)` | |
| `--radius-box` | `0rem` | Card/modal rounded corners |
| `--radius-selector` | `0rem` | Selector rounded corners |
| `--radius-field` | `0rem` | Button/input rounded corners |

### Dark Theme: `yaoxiang-dark`

| Token | Value | Purpose |
|-------|-----|------|
| `base-100` | `oklch(14% 0.008 250)` | Stone tablet page background |
| `base-200` | `oklch(20% 0.01 250)` | Card/Hero gradient |
| `base-300` | `oklch(26% 0.015 250)` | Border/Timeline |
| `base-content` | `oklch(82% 0.01 250)` | Rubbing white body text |
| `primary` | `oklch(58% 0.20 18)` | Cool vermilion · Primary emphasis |
| `primary-content` | `oklch(98% 0.005 85)` | Text on cool vermilion |
| `secondary` | `oklch(48% 0.04 245)` | Tablet gray · Secondary emphasis |
| `secondary-content` | `oklch(98% 0.01 250)` | |
| `accent` | `oklch(60% 0.10 185)` | Cool cyan · Code highlighting |
| `accent-content` | `oklch(15% 0.02 185)` | |
| `neutral` | `oklch(10% 0.008 250)` | Deep tablet · Footer |
| `neutral-content` | `oklch(80% 0.01 250)` | |
| `--radius-box` | `0rem` | |
| `--radius-selector` | `0rem` | |
| `--radius-field` | `0rem` | |

### Primary/Secondary Color Division

- **Primary (Vermilion seal)**: Hero Badge, CTA buttons, primary card shadows, section title badges, key emphasis
- **Secondary (Seal paste/Tablet gray)**: Secondary card shadows, minor label chips, secondary timeline nodes
- **Card hierarchy**: Page `base-100` → Card `base-200` (brightness difference ~5-6%), primary card `shadow-primary`, secondary card `shadow-secondary`
- **Accent (Cyan-green)**: Code syntax highlighting reserved for keywords

### Font System (consistent with index.md)

| Purpose | Font Stack |
|------|--------|
| Global body text | `Space Mono`, monospace |
| Chinese titles | `Microsoft YaHei`, `SimHei`, `PingFang SC`, `Heiti SC`, sans-serif |
| Code | `JetBrains Mono`, monospace |

Google Fonts load: `JetBrains Mono:wght@400;700` + `Space Mono:ital,wght@0,400;0,700;1,400`

## Blog Index

### virtual:blog-index Plugin

Custom VitePress plugin (Vite virtual module), no CI dependency.

- Path: `.vitepress/plugins/blog-index.ts`
- At build time, scan `blog/*.md` and extract frontmatter (title, date, description)
- Sort by date, output JSON
- In dev mode, watch file changes, HMR auto-updates
- The blog page gets data via `import { blogPosts } from 'virtual:blog-index'`

## New Plugins

| Plugin | Purpose | Extra config |
|------|------|---------|
| `vitepress-plugin-group-icons` | Code block file type icons | None |
| `vitepress-plugin-back-to-top` | Back to top button | None (style overridden by theme) |
| `vitepress-plugin-nprogress` | Page load progress bar | `color: oklch(55% 0.24 28)` |
| `@vuepress/plugin-reading-time` | Reading time + word count | None (only used on blog page) |
| `vitepress-plugin-tabs` | Markdown tab syntax | None |
| `@nolebase/vitepress-plugin-git-changelog` | Contributors + changelog at bottom of each page | `maxGitLogCount: 5` |

Note: Plugins need to be installed one by one to confirm VitePress 1.6.4 compatibility, replace with alternatives if necessary.

## Page Layout Plans

### Tools Page (Tools.vue) · New

Changed from raw HTML to Vue component.

| Element | Plan |
|------|------|
| Hero | TOOLS badge + bold title + description |
| Card area | Three feature cards (compiler/formatter/LSP), icon + title + description + status label |
| Status | Compiler/Formatter → `Available` (primary badge), LSP → `In Development` (cool semi-transparent badge) |

### Blog List Page (Blog.vue) · New

Timeline layout, consistent with Home page Tracks:

| Element | Plan |
|------|------|
| Hero | BLOG badge + bold title + description |
| Timeline | backbone (w-1 bg-base-300) + `rounded-full` nodes |
| Article cards | bg-base-200 + shadow-secondary hard shadow + date + title + excerpt + tilted NEW label |
| Data source | `virtual:blog-index` virtual module |
| Empty state | Dashed placeholder "More articles coming soon..." |

### Community Page (Community.vue)

Complete refactoring, clean up old structure. Principles: large area of restrained whitespace, cards only for terminals and bulletin boards, Badges throughout the page, data scattered into semantic areas.

**Layout (top to bottom):**

| Area | Plan |
|------|------|
| Hero | Grid background + bold title + tagline + a row of fun Badges (each with slight rotation + unique animation, hover to straighten and zoom) |
| Maintainers | Left border `border-l-3 border-primary` + avatar (with shadow, hover to zoom) + CORE/SOLO Badge next to name + contributor count naturally embedded in bio |
| Contributors | PR/Issue two columns, dashed box empty state to guide participation |
| Recent activity | Two columns: terminal git log (commit count integrated at bottom) + bulletin board (Meetup/Conf/Contributor Day, status Badge embedded in title) |
| Get involved | Three underlined links (Stars Badge next to GitHub, Open Issues Badge next to contribution guide), hover arrow slides right + underline color changes |

**Badge System (reused class):**

6 variants — hot (vermilion + shadow + tilt), cool (secondary color + wobble), teal (cyan-green + tilt), glow (border pulse breathing), muted (gray + tilt), soft (small gray). Mixed use throughout the page, not as separate blocks.

**No longer retained:**

- mockup-window terminal (changed to mini-console)
- Independent stats card area (data scattered and integrated)
- border-l-4 left border description blocks
- avatar online status

### Playground Page (Playground.vue)

Keep CodeMirror editor core, replace all shells.

| Element | Plan |
|------|------|
| Hero | PLAYGROUND badge + bold title + description |
| Editor | Terminal three dots + filename `main.yx` + bottom status bar (line number/column number/indent/encoding) |
| Top right | Compiler version label `v0.1.0 · WASM` |
| Run button | primary background + offset shadow + press displacement |
| Output panel | secondary color shadow + title bar showing compilation time |
| Shortcuts | Three groups below run button (Ctrl+Enter run / Ctrl+S share / Ctrl+K format) |
| Path | Playground.vue path fix (theme/index.js import path vs actual file location) |

### Download Page (Download.vue)

Keep existing functionality, align with color system, layout structure unchanged.

**Change List:**

| Element | Change |
|------|------|
| Main title | Add "Download YaoXiang" (bold `SimHei`) |
| Subtitle | "TYPE THE UNIVERSE" Space Mono font-black |
| Description | "Choose your platform and start building the world." Space Mono, remove `>>_` |
| Hero badge | `rounded-full` → `rounded-none` + offset shadow |
| Terminal window | Remove `border-l-2 border-success`, switch shadow to primary color. Label "Coming Soon", indicating quick install is not yet available |
| Version selector | `rounded-lg` → `rounded-none`, label "Built from GitHub Release" |
| Platform cards | `bg-base-100` → `bg-base-200`, switch shadow to secondary color, arch badge retained |
| New WASM card | accent cyan-green border, copy "No installation, use in browser" "Try in browser ↗", link to Playground |
| macOS copy | "Universal binary (.tar.gz) + supports both M-series and Intel chips" |
| Linux copy | "Static binary (.tar.gz) + musl compiled, no runtime dependencies" |
| Fonts | Space Mono global (original Download.vue has no Microsoft YaHei) |
| 3D mouse tracking | Retained (global pending items handled uniformly) |
| Bottom two columns | `align-items: stretch` height alignment |

## Other Implementation Items

### Global CSS Rewrite (tailwind.css)

1. `@import "tailwindcss"`
2. `@plugin "daisyui/theme"` light `yaoxiang` (default, prefersdark: false)
3. `@plugin "daisyui/theme"` dark `yaoxiang-dark` (prefersdark: true)
4. VitePress `--vp-c-brand` etc. variables mapped to primary token
5. Scrollbar thumb color `oklch(55% 0.24 28)`
6. Navigation bar VPNavBarMenu fix retained

### Mouse Tracking Optimization

Home.vue + Download.vue: Wrap with `useIntersectionObserver` or `v-if`, only activate `useMouse` when the component is visible, pause 3D tilt calculation when invisible.

### Playground.vue Path Fix

The import path in `theme/index.js` L9 is inconsistent with the actual file location, unified to one place.

## Pending Items

- [x] Color system redesign (Ink-Vermilion color system: light rice paper vermilion seal + dark stone tablet rubbing)
- [x] 6 page layout designs (Download/Community/Playground/Tools/Blog, Home page unchanged)
- [x] Component extraction and reuse (4 components: HeroSection / TerminalWindow / TimelineTrack / FeatureCard, rest use daisyUI)
- [x] 6 plugins installation and configuration (6 plugins + nprogress color + git-changelog maxGitLogCount)
- [ ] blog-index virtual module implementation (design ready, awaiting coding)
- [ ] Global CSS rewrite (tailwind.css → two @plugin "daisyui/theme")
- [ ] Remove global mouse tracking (only activate useMouse when component is visible)
- [ ] Playground.vue path fix (theme/index.js import path)

## Related

- RFC-006: Documentation Site Optimization
```