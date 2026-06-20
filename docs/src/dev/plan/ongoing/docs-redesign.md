---
title: "文档站页面重设计"
status: "讨论中"
created: "2026-06-17"
---

# 文档站页面重设计

## 目标

重新设计 6 个自定义页面的布局，统一视觉语言，美化所有页面。

## 范围

### 自定义布局页面（6 个）

| 页面 | 文件 | 当前组件 | 说明 |
|------|------|---------|------|
| 首页 | `index.md` | `Home.vue` | 保持现有设计作为基准 |
| 下载 | `download.md` | `Download.vue` | 重新设计 |
| 社区 | `community.md` | `Community.vue` | 重新设计 |
| 码场 | `playground.md` | `Playground.vue` | 重新设计 |
| 工具 | `tools.md` | 无（裸 HTML） | 需新建 Tools.vue |
| 博客 | `blog/index.md` | 无（裸 HTML） | 需新建 Blog.vue |

### 全局美化

所有默认 VitePress 页面（教程、指南、参考、开发文档等）调整全局 CSS 配色、字体、间距，不改布局结构。

## 架构决策

## 组件化策略

daisyUI 组件（btn/badge/card/dropdown 等）的功能逻辑保留，视觉由 `yaoxiang` 主题 token 统一覆盖，不需要额外包装。

仅提取真正重复的页面结构为 Vue 组件：

| 组件 | 复用 | Props |
|------|------|-------|
| `HeroSection.vue` | 下载/社区/码场/工具/博客——5 个页面 Hero 一致 | `badge`, `title`, `description`, 默认 slot（badge 下方额外内容） |
| `TerminalWindow.vue` | 首页代码展示 / 下载安装命令 / 社区 git log | `title?`, `shadow`（primary/secondary）, 默认 slot |
| `TimelineTrack.vue` | 首页 tracks / 博客文章列表 | `dotColor?`, `tag?`, `date?`, 默认 slot |
| `FeatureCard.vue` | 下载平台 / 工具卡片 / 社区入口 | `icon?`, `title`, `description`, `status?`, 默认 slot（底部操作） |

不提取：PageShell（VitePress 已够用）、UnderConstruction（inline 即可）。

### i18n 兼容

需要翻译的内容全部留在 markdown frontmatter，Vue 组件只负责渲染。博客文章列表通过 `virtual:blog-index` 虚拟模块提供。

### 设计语言基准

以 `Home.vue` 为基准：
- 字体：`Space Mono`（全局）+ `Microsoft YaHei`（中文标题）+ `JetBrains Mono`（代码）
- 偏移阴影：`shadow-[Xpx_Xpx_0px_color]`（无模糊硬阴影）
- 卡片：`card bg-base-200 rounded-none shadow-[6px_6px_0px_var(--p)]`（锐角 + 粗偏移阴影 + 页面底色抬高）
- 时间轴：`w-1 bg-base-300` backbone + `rounded-full` 圆形节点
- 倾斜标签：`rotate-2` + `shadow-lg` + `rounded-none`
- 页脚：`bg-neutral text-neutral-content rounded-none`
- 圆角策略：仅代码窗口 `rounded-lg`，圆形节点 `rounded-full`，其余全部 `rounded-none`

## 色系

### 墨朱色系

以传统书法/篆刻/拓本为隐喻，亮色如宣纸朱印（暖调），暗色如碑石拓本（冷调）。

daisyUI v5 自定义主题，使用 `@plugin "daisyui/theme"` OKLCH 格式。

### 亮色主题：`yaoxiang`（默认）

| Token | 值 | 用途 |
|-------|-----|------|
| `base-100` | `oklch(98% 0.008 85)` | 宣纸页面底色 |
| `base-200` | `oklch(93% 0.016 85)` | 卡片/Hero 渐变 |
| `base-300` | `oklch(88% 0.03 85)` | 边框/时间轴 |
| `base-content` | `oklch(18% 0.02 85)` | 墨色正文 |
| `primary` | `oklch(55% 0.24 28)` | 朱印·一级强调 |
| `primary-content` | `oklch(98% 0.005 85)` | 朱印上文字 |
| `secondary` | `oklch(45% 0.05 85)` | 印泥·二级强调 |
| `secondary-content` | `oklch(98% 0.005 85)` | |
| `accent` | `oklch(60% 0.12 180)` | 青绿·代码高亮 |
| `accent-content` | `oklch(15% 0.02 180)` | |
| `neutral` | `oklch(28% 0.02 85)` | 墨底·页脚 |
| `neutral-content` | `oklch(90% 0.01 85)` | |
| `--radius-box` | `0rem` | 卡片/弹窗圆角 |
| `--radius-selector` | `0rem` | 选择器圆角 |
| `--radius-field` | `0rem` | 按钮/输入框圆角 |

### 暗色主题：`yaoxiang-dark`

| Token | 值 | 用途 |
|-------|-----|------|
| `base-100` | `oklch(14% 0.008 250)` | 碑石页面底色 |
| `base-200` | `oklch(20% 0.01 250)` | 卡片/Hero 渐变 |
| `base-300` | `oklch(26% 0.015 250)` | 边框/时间轴 |
| `base-content` | `oklch(82% 0.01 250)` | 拓白正文 |
| `primary` | `oklch(58% 0.20 18)` | 冷朱·一级强调 |
| `primary-content` | `oklch(98% 0.005 85)` | 冷朱上文字 |
| `secondary` | `oklch(48% 0.04 245)` | 碑灰·二级强调 |
| `secondary-content` | `oklch(98% 0.01 250)` | |
| `accent` | `oklch(60% 0.10 185)` | 冷青·代码高亮 |
| `accent-content` | `oklch(15% 0.02 185)` | |
| `neutral` | `oklch(10% 0.008 250)` | 深碑·页脚 |
| `neutral-content` | `oklch(80% 0.01 250)` | |
| `--radius-box` | `0rem` | |
| `--radius-selector` | `0rem` | |
| `--radius-field` | `0rem` | |

### 一二级色分工

- **Primary（朱印）**：Hero Badge、CTA 按钮、一级卡片阴影、分区标题 badge、关键强调
- **Secondary（印泥/碑灰）**：二级卡片阴影、次要标签 chip、时间轴次要节点
- **卡片层次**：页面 `base-100` → 卡片 `base-200`（明度差约 5-6%），一级卡 `shadow-primary`，二级卡 `shadow-secondary`
- **Accent（青绿）**：代码语法高亮保留关键字

### 字体系统（与 index.md 一致）

| 用途 | 字体栈 |
|------|--------|
| 全局正文 | `Space Mono`, monospace |
| 中文标题 | `Microsoft YaHei`, `SimHei`, `PingFang SC`, `Heiti SC`, sans-serif |
| 代码 | `JetBrains Mono`, monospace |

Google Fonts 加载：`JetBrains Mono:wght@400;700` + `Space Mono:ital,wght@0,400;0,700;1,400`

## 博客索引

### virtual:blog-index 插件

自定义 VitePress 插件（Vite 虚拟模块），不依赖 CI。

- 路径：`.vitepress/plugins/blog-index.ts`
- 构建时扫描 `blog/*.md`，提取 frontmatter（title, date, description）
- 按日期排序，输出 JSON
- 开发模式 watch 文件变化，HMR 自动更新
- 博客页通过 `import { blogPosts } from 'virtual:blog-index'` 获取数据

## 新增插件

| 插件 | 用途 | 额外配置 |
|------|------|---------|
| `vitepress-plugin-group-icons` | 代码块文件类型图标 | 无 |
| `vitepress-plugin-back-to-top` | 回到顶部按钮 | 无（样式由主题覆盖） |
| `vitepress-plugin-nprogress` | 页面加载进度条 | `color: oklch(55% 0.24 28)` |
| `@vuepress/plugin-reading-time` | 阅读时间 + 字数统计 | 无（仅博客页使用） |
| `vitepress-plugin-tabs` | Markdown tab 语法 | 无 |
| `@nolebase/vitepress-plugin-git-changelog` | 每页底部贡献者 + changelog | `maxGitLogCount: 5` |

注意：插件需逐一安装确认 VitePress 1.6.4 兼容性，必要时替换为替代方案。

## 页面布局方案

### 工具页（Tools.vue）· 新建

由裸 HTML 改为 Vue 组件。

| 元素 | 方案 |
|------|------|
| Hero | TOOLS badge + 黑体标题 + 描述 |
| 卡片区 | 三张特性卡片（编译器/格式化器/LSP），图标 + 标题 + 描述 + 状态标签 |
| 状态 | 编译器/格式化器 → `可用`（primary badge），LSP → `开发中`（cool 半透明 badge） |

### 博客列表页（Blog.vue）· 新建

时间线布局，与首页 Tracks 一致：

| 元素 | 方案 |
|------|------|
| Hero | BLOG badge + 黑体标题 + 描述 |
| 时间轴 | backbone（w-1 bg-base-300）+ `rounded-full` 节点 |
| 文章卡片 | bg-base-200 + shadow-secondary 硬阴影 + 日期 + 标题 + 摘要 + 倾斜 NEW 标签 |
| 数据来源 | `virtual:blog-index` 虚拟模块 |
| 空状态 | 虚线占位「更多文章即将发布...」

### 社区页（Community.vue）

完全重构，清理旧结构。原则：大面积留白克制、卡片仅用于终端和公告栏、Badge 贯穿全页、数据分散融入语义区域。

**布局（自上而下）：**

| 区域 | 方案 |
|------|------|
| Hero | 网格背景 + 黑体标题 + tagline + 一排趣味 Badge（各自微旋转+个性动画，hover 弹正放大） |
| 维护者 | 左边框 `border-l-3 border-primary` + 头像（带阴影 hover 放大）+ 名字旁 CORE/SOLO Badge + 简介中自然嵌入贡献者数 |
| 贡献者 | PR / Issue 两列，空状态虚线框引导参与 |
| 最近动态 | 双栏：终端 git log（提交数融在底部）+ 公告栏（Meetup/Conf/贡献者日，状态 Badge 嵌在标题内） |
| 参与进来 | 三个下划线链接（GitHub 旁挂 Stars Badge、贡献指南旁挂 Open Issues Badge），hover 箭头右滑 + 底线变色 |

**Badge 体系（复用 class）：**

6 种变体——hot（朱色+阴影+tilt）、cool（二级色+wobble）、teal（青绿+tilt）、glow（border pulse 呼吸）、muted（灰+tilt）、soft（小号灰）。全页混用，不作独立区块。

**不再保留：**
- mockup-window 终端（改为 mini-console）
- 独立统计卡片区（数据分散融入）
- border-l-4 左边框描述块
- avatar online 状态

### 码场页（Playground.vue）

保留 CodeMirror 编辑器核心，外壳全部替换。

| 元素 | 方案 |
|------|------|
| Hero | PLAYGROUND badge + 黑体标题 + 描述 |
| 编辑器 | 终端三点 + 文件名 `main.yx` + 底部状态栏（行号/列号/缩进/编码） |
| 右上角 | 编译器版本标注 `v0.1.0 · WASM` |
| 运行按钮 | primary 底色 + 偏移阴影 + 按下位移 |
| 输出面板 | secondary 色阴影 + 标题栏显示编译耗时 |
| 快捷键 | 运行按钮下方三组（Ctrl+Enter 运行 / Ctrl+S 分享 / Ctrl+K 格式化） |
| 路径 | Playground.vue 路径修正（theme/index.js import 路径 vs 实际文件位置） |

### 下载页（Download.vue）

保留现有功能，对齐色系，布局结构不变。

**改动清单：**

| 元素 | 变更 |
|------|------|
| 主标题 | 新增「下载 YaoXiang」（黑体 `SimHei`） |
| 副标题 | 「TYPE THE UNIVERSE」Space Mono font-black |
| 描述 | 「选择你的平台，开始构建世界。」Space Mono，去掉 `>>_` |
| Hero badge | `rounded-full` → `rounded-none` + 偏移阴影 |
| 终端窗口 | 去掉 `border-l-2 border-success`，阴影切 primary 色。标注「即将推出」，说明快速安装尚未开放 |
| 版本选择器 | `rounded-lg` → `rounded-none`，标注「构建自 GitHub Release」 |
| 平台卡片 | `bg-base-100` → `bg-base-200`，阴影切 secondary 色，arch badge 保留 |
| 新增 WASM 卡片 | accent 青绿色边框，文案「无需安装，浏览器即用」「在浏览器中试用 ↗」，链接到码场 |
| macOS 文案 | 「通用二进制 (.tar.gz) + 同时支持 M 系列和 Intel 芯片」 |
| Linux 文案 | 「静态二进制 (.tar.gz) + musl 编译，无运行时依赖」 |
| 字体 | Space Mono 全局（原版 Download.vue 无微软雅黑） |
| 3D 鼠标跟踪 | 保留（全局悬置项统一处理） |
| 底部两列 | `align-items: stretch` 高度对齐 |

## 其它实现项

### 全局 CSS 重写（tailwind.css）

1. `@import "tailwindcss"`
2. `@plugin "daisyui/theme"` 亮色 `yaoxiang`（default, prefersdark: false）
3. `@plugin "daisyui/theme"` 暗色 `yaoxiang-dark`（prefersdark: true）
4. VitePress `--vp-c-brand` 等变量映射到 primary token
5. 滚动条 thumb 色 `oklch(55% 0.24 28)`
6. 导航栏 VPNavBarMenu 修复保留

### 鼠标跟踪优化

Home.vue + Download.vue：用 `useIntersectionObserver` 或 `v-if` 包裹，仅在组件可见时激活 `useMouse`，不可见时暂停 3D tilt 计算。

### Playground.vue 路径修正

`theme/index.js` L9 的 import 路径与实际文件位置不一致，统一到一处。

## 悬置项

- [x] 色系重设计（墨朱色系：亮色宣纸朱印 + 暗色碑石拓本）
- [x] 6 个页面布局设计（下载/社区/码场/工具/博客，首页不改）
- [x] 组件提取与复用（4 个组件：HeroSection / TerminalWindow / TimelineTrack / FeatureCard，其余用 daisyUI）
- [x] 6 个插件安装与配置（6 个插件 + nprogress 颜色 + git-changelog maxGitLogCount）
- [ ] blog-index 虚拟模块实现（设计已有，待编码）
- [ ] 全局 CSS 重写（tailwind.css → 两个 @plugin "daisyui/theme"）
- [ ] 移除全局鼠标跟踪（组件可见时才激活 useMouse）
- [ ] Playground.vue 路径修正（theme/index.js import 路径）

## 相关

- RFC-006: 文档站优化
