---
title: "ドキュメントサイトページ再設計"
status: "議論中"
created: "2026-06-17"
---

# ドキュメントサイトページ再設計

## 目標

6 つのカスタムページのレイアウトを再設計し、ビジュアル言語を統一し、すべてのページをより美しくする。

## 範囲

### カスタムレイアウトページ（6 つ）

| ページ | ファイル | 現在のコンポーネント | 説明 |
|------|------|---------|------|
| ホーム | `index.md` | `Home.vue` | 既存デザインを基準として維持 |
| ダウンロード | `download.md` | `Download.vue` | 再設計 |
| コミュニティ | `community.md` | `Community.vue` | 再設計 |
| コードラボ | `playground.md` | `Playground.vue` | 再設計 |
| ツール | `tools.md` | なし（素の HTML） | 新規に `Tools.vue` を作成 |
| ブログ | `blog/index.md` | なし（素の HTML） | 新規に `Blog.vue` を作成 |

### 全体的な美化

すべてのデフォルト VitePress ページ（チュートリアル、ガイド、リファレンス、開発ドキュメントなど）のグローバル CSS の配色、フォント、間隔を調整し、レイアウト構造は変更しない。

## アーキテクチャの決定

## コンポーネント化戦略

daisyUI コンポーネント（btn/badge/card/dropdown など）の機能ロジックは維持し、ビジュアルは `yaoxiang` テーマのトークンで統一的に上書きするため、追加のラッパーは不要。

実際に重複するページ構造のみ Vue コンポーネントとして抽出する：

| コンポーネント | 再利用 | Props |
|------|------|-------|
| `HeroSection.vue` | ダウンロード/コミュニティ/コードラボ/ツール/ブログ——5 ページの Hero が統一 | `badge`, `title`, `description`, デフォルトスロット（badge 下に追加コンテンツ） |
| `TerminalWindow.vue` | ホームのコード表示 / ダウンロードのインストールコマンド / コミュニティの git log | `title?`, `shadow`（primary/secondary）、デフォルトスロット |
| `TimelineTrack.vue` | ホームの tracks / ブログ記事一覧 | `dotColor?`, `tag?`, `date?`、デフォルトスロット |
| `FeatureCard.vue` | ダウンロードプラットフォーム / ツールカード / コミュニティエントリ | `icon?`, `title`, `description`, `status?`、デフォルトスロット（底部アクション） |

抽出しないもの：PageShell（VitePress で十分）、UnderConstruction（インラインで十分）。

### i18n 互換性

翻訳が必要なコンテンツはすべて markdown の frontmatter に残し、Vue コンポーネントは描画のみを担当する。ブログ記事一覧は `virtual:blog-index` 仮想モジュールで提供する。

### デザイン言語の基準

`Home.vue` を基準とする：
- フォント：`Space Mono`（グローバル）+ `Microsoft YaHei`（中文タイトル）+ `JetBrains Mono`（コード）
- オフセットシャドウ：`shadow-[Xpx_Xpx_0px_color]`（ぼかしなしの硬いシャドウ）
- カード：`card bg-base-200 rounded-none shadow-[6px_6px_0px_var(--p)]`（鋭角 + 太いオフセットシャドウ + ページ背景より一段上げる）
- タイムライン：`w-1 bg-base-300` バックボーン + `rounded-full` 円形ノード
- 傾斜ラベル：`rotate-2` + `shadow-lg` + `rounded-none`
- フッター：`bg-neutral text-neutral-content rounded-none`
- 角丸戦略：コードウィンドウの `rounded-lg` と円形ノードの `rounded-full` のみ、それ以外はすべて `rounded-none`

## カラーシステム

### 墨朱カラーシステム

伝統的な書道・篆刻・拓本をメタファーとし、ライトモードは宣紙に押された朱印（暖色調）、ダークモードは石碑の拓本（寒色調）とする。

daisyUI v5 のカスタムテーマは `@plugin "daisyui/theme"` の OKLCH 形式を使用。

### ライトテーマ：`yaoxiang`（デフォルト）

| トークン | 値 | 用途 |
|-------|-----|------|
| `base-100` | `oklch(98% 0.008 85)` | 宣紙のページ背景 |
| `base-200` | `oklch(93% 0.016 85)` | カード/Hero のグラデーション |
| `base-300` | `oklch(88% 0.03 85)` | ボーダー/タイムライン |
| `base-content` | `oklch(18% 0.02 85)` | 墨色の本文 |
| `primary` | `oklch(55% 0.24 28)` | 朱印・第一階層の強調 |
| `primary-content` | `oklch(98% 0.005 85)` | 朱印上の文字 |
| `secondary` | `oklch(45% 0.05 85)` | 印肉・第二階層の強調 |
| `secondary-content` | `oklch(98% 0.005 85)` | |
| `accent` | `oklch(60% 0.12 180)` | 青緑・コードハイライト |
| `accent-content` | `oklch(15% 0.02 180)` | |
| `neutral` | `oklch(28% 0.02 85)` | 墨底・フッター |
| `neutral-content` | `oklch(90% 0.01 85)` | |
| `--radius-box` | `0rem` | カード/ポップアップの角丸 |
| `--radius-selector` | `0rem` | セレクターの角丸 |
| `--radius-field` | `0rem` | ボタン/インプットの角丸 |

### ダークテーマ：`yaoxiang-dark`

| トークン | 値 | 用途 |
|-------|-----|------|
| `base-100` | `oklch(14% 0.008 250)` | 石碑のページ背景 |
| `base-200` | `oklch(20% 0.01 250)` | カード/Hero のグラデーション |
| `base-300` | `oklch(26% 0.015 250)` | ボーダー/タイムライン |
| `base-content` | `oklch(82% 0.01 250)` | 拓白の本文 |
| `primary` | `oklch(58% 0.20 18)` | 寒朱・第一階層の強調 |
| `primary-content` | `oklch(98% 0.005 85)` | 寒朱上の文字 |
| `secondary` | `oklch(48% 0.04 245)` | 碑灰・第二階層の強調 |
| `secondary-content` | `oklch(98% 0.01 250)` | |
| `accent` | `oklch(60% 0.10 185)` | 寒青・コードハイライト |
| `accent-content` | `oklch(15% 0.02 185)` | |
| `neutral` | `oklch(10% 0.008 250)` | 深碑・フッター |
| `neutral-content` | `oklch(80% 0.01 250)` | |
| `--radius-box` | `0rem` | |
| `--radius-selector` | `0rem` | |
| `--radius-field` | `0rem` | |

### 第一/第二階層の色の役割分担

- **Primary（朱印）**：Hero Badge、CTA ボタン、第一階層カードのシャドウ、セクションタイトル badge、重要な強調
- **Secondary（印肉/碑灰）**：第二階層カードのシャドウ、補助的なラベル chip、タイムラインの補助ノード
- **カードの階層**：ページ `base-100` → カード `base-200`（明度差 約 5-6%）、第一階層カード `shadow-primary`、第二階層カード `shadow-secondary`
- **Accent（青緑）**：コードのシンタックスハイライトでキーワードを保持

### フォントシステム（index.md と統一）

| 用途 | フォントスタック |
|------|--------|
| グローバル本文 | `Space Mono`, monospace |
| 中文タイトル | `Microsoft YaHei`, `SimHei`, `PingFang SC`, `Heiti SC`, sans-serif |
| コード | `JetBrains Mono`, monospace |

Google Fonts 読み込み：`JetBrains Mono:wght@400;700` + `Space Mono:ital,wght@0,400;0,700;1,400`

## ブログインデックス

### virtual:blog-index プラグイン

カスタム VitePress プラグイン（Vite 仮想モジュール）、CI に依存しない。

- パス：`.vitepress/plugins/blog-index.ts`
- ビルド時に `blog/*.md` をスキャンし、frontmatter（title, date, description）を抽出
- 日付順にソートし、JSON を出力
- 開発モードではファイル変更を watch し、HMR で自動更新
- ブログページは `import { blogPosts } from 'virtual:blog-index'` でデータを取得

## 追加プラグイン

| プラグイン | 用途 | 追加設定 |
|------|------|---------|
| `vitepress-plugin-group-icons` | コードブロックのファイルタイプアイコン | なし |
| `vitepress-plugin-back-to-top` | トップへ戻るボタン | なし（スタイルはテーマで上書き） |
| `vitepress-plugin-nprogress` | ページ読み込みプログレスバー | `color: oklch(55% 0.24 28)` |
| `@vuepress/plugin-reading-time` | 読了時間 + 文字数統計 | なし（ブログページのみ使用） |
| `vitepress-plugin-tabs` | Markdown のタブ構文 | なし |
| `@nolebase/vitepress-plugin-git-changelog` | 各ページ下部のコントリビューター + changelog | `maxGitLogCount: 5` |

注意：プラグインは VitePress 1.6.4 との互換性を一つずつ確認してインストールする必要があり、必要に応じて代替案に置き換える。

## ページレイアウト案

### ツールページ（Tools.vue）・新規作成

素の HTML から Vue コンポーネントに変更。

| 要素 | 案 |
|------|------|
| Hero | TOOLS badge + ゴシック体のタイトル + 説明 |
| カードエリア | 3 枚のフィーチャーカード（コンパイラ/フォーマッター/LSP）、アイコン + タイトル + 説明 + ステータスラベル |
| ステータス | コンパイラ/フォーマッター → `利用可能`（primary badge）、LSP → `開発中`（cool 半透明 badge） |

### ブログ一覧ページ（Blog.vue）・新規作成

ホームページの Tracks と統一感のあるタイムラインレイアウト：

| 要素 | 案 |
|------|------|
| Hero | BLOG badge + ゴシック体のタイトル + 説明 |
| タイムライン | バックボーン（`w-1 bg-base-300`）+ `rounded-full` ノード |
| 記事カード | `bg-base-200` + `shadow-secondary` ハードシャドウ + 日付 + タイトル + 概要 + 傾斜 NEW ラベル |
| データソース | `virtual:blog-index` 仮想モジュール |
| 空状態 | 破線プレースホルダー「更多文章即将发布...」 |

### コミュニティページ（Community.vue）

完全に再構築し、旧構造を整理。原則：大面積の余白は控えめに、カードターミナルと告知欄のみに使用、Badge をページ全体に通し、データは意味的領域に分散して溶け込ませる。

**レイアウト（上から下へ）：**

| エリア | 案 |
|------|------|
| Hero | グリッド背景 + ゴシック体のタイトル + tagline + 個性豊かな Badge 群（それぞれ微回転+個別アニメーション、hover で正規化&拡大） |
| メンテナー | 左ボーダー `border-l-3 border-primary` + アバター（シャドウ付き hover 拡大）+ 名前横に CORE/SOLO Badge + コントリビューター数を自然に説明文に埋め込む |
| コントリビューター | PR / Issue の 2 列、空状態は破線枠で参加を促す |
| 最近の活動 | 2 列：ターミナル git log（コミット数を底部に埋め込み）+ 告知欄（Meetup/Conf/コントリビューターデー、ステータス Badge をタイトル内に埋め込み） |
| 参加する | 3 つの下線付きリンク（GitHub 横に Stars Badge、コントリビュートガイド横に Open Issues Badge）、hover で矢印が右にスライド + 下線色変化 |

**Badge 体系（class を再利用）：**

6 種類のバリアント——hot（朱色+シャドウ+tilt）、cool（第二階層色+wobble）、teal（青緑+tilt）、glow（border のパルス呼吸）、muted（グレー+tilt）、soft（小サイズグレー）。ページ全体で混在して使用し、独立したブロックとはしない。

**もう保持しないもの：**
- mockup-window ターミナル（mini-console に変更）
- 独立した統計カードエリア（データは分散して溶け込ませる）
- `border-l-4` 左ボーダーの説明ブロック
- avatar のオンライン状態

### コードラボページ（Playground.vue）

CodeMirror エディタのコアを維持し、外殻をすべて置き換え。

| 要素 | 案 |
|------|------|
| Hero | PLAYGROUND badge + ゴシック体のタイトル + 説明 |
| エディタ | ターミナル三点 + ファイル名 `main.yx` + 底部ステータスバー（行番号/列番号/インデント/エンコーディング） |
| 右上 | コンパイラバージョン表記 `v0.1.0 · WASM` |
| 実行ボタン | primary 背景 + オフセットシャドウ + 押下時に変位 |
| 出力パネル | secondary 色のシャドウ + タイトルバーにコンパイル時間を表示 |
| ショートカット | 実行ボタン下に 3 組（Ctrl+Enter 実行 / Ctrl+S 共有 / Ctrl+K フォーマット） |
| パス | Playground.vue のパス修正（`theme/index.js` の import パスと実際のファイル位置の不整合） |

### ダウンロードページ（Download.vue）

既存機能を維持し、カラーシステムを揃え、レイアウト構造は変えない。

**変更点一覧：**

| 要素 | 変更 |
|------|------|
| メインタイトル | 「YaoXiang をダウンロード」を追加（ゴシック体 `SimHei`） |
| サブタイトル | 「TYPE THE UNIVERSE」Space Mono font-black |
| 説明 | 「プラットフォームを選択して、世界の構築を始めましょう。」Space Mono、`>>_` を削除 |
| Hero badge | `rounded-full` → `rounded-none` + オフセットシャドウ |
| ターミナルウィンドウ | `border-l-2 border-success` を削除、シャドウを primary 色に変更。「近日公開」と注記し、クイックインストールがまだ開放されていないことを説明 |
| バージョンセレクター | `rounded-lg` → `rounded-none`、`GitHub Release からビルド` と注記 |
| プラットフォームカード | `bg-base-100` → `bg-base-200`、シャドウを secondary 色に変更、arch badge は維持 |
| WASM カードを新規追加 | accent 青緑色のボーダー、文言「インストール不要、ブラウザでそのまま」「ブラウザで試す ↗」、コードラボへのリンク |
| macOS 文言 | 「ユニバーサルバイナリ (.tar.gz) + M シリーズと Intel チップ両対応」 |
| Linux 文言 | 「静的バイナリ (.tar.gz) + musl コンパイル、ランタイム依存なし」 |
| フォント | Space Mono をグローバルに（旧 Download.vue には Microsoft YaHei なし） |
| 3D マウス追従 | 維持（グローバルのサスペンド項目を統一処理） |
| 底部 2 列 | `align-items: stretch` で高さを揃える |

## その他の実装項目

### グローバル CSS の書き換え（tailwind.css）

1. `@import "tailwindcss"`
2. `@plugin "daisyui/theme"` ライト `yaoxiang`（default, prefersdark: false）
3. `@plugin "daisyui/theme"` ダーク `yaoxiang-dark`（prefersdark: true）
4. VitePress の `--vp-c-brand` などの変数を primary トークンにマッピング
5. スクロールバーの thumb 色 `oklch(55% 0.24 28)`
6. ナビゲーションバー VPNavBarMenu の修正を維持

### マウス追従の最適化

Home.vue + Download.vue：`useIntersectionObserver` または `v-if` でラップし、コンポーネントが見える時のみ `useMouse` を有効化し、見えない時は 3D tilt の計算を一時停止する。

### Playground.vue のパス修正

`theme/index.js` L9 の import パスと実際のファイル位置が一致していないため、一箇所に統一する。

## 保留項目

- [x] カラーシステム再設計（墨朱カラーシステム：ライトは宣紙朱印 + ダークは碑石拓本）
- [x] 6 ページのレイアウト設計（ダウンロード/コミュニティ/コードラボ/ツール/ブログ、ホームページは変更なし）
- [x] コンポーネント抽出と再利用（4 コンポーネント：HeroSection / TerminalWindow / TimelineTrack / FeatureCard、その他は daisyUI を使用）
- [x] 6 プラグインのインストールと設定（6 プラグイン + nprogress 色 + git-changelog maxGitLogCount）
- [ ] blog-index 仮想モジュールの実装（設計あり、実装待ち）
- [ ] グローバル CSS の書き換え（tailwind.css → 2 つの `@plugin "daisyui/theme"`）
- [ ] グローバルマウス追従の削除（コンポーネントが見える時のみ `useMouse` を有効化）
- [ ] Playground.vue のパス修正（`theme/index.js` の import パス）

## 関連

- RFC-006: ドキュメントサイト最適化