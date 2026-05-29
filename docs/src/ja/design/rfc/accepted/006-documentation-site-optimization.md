---
title: RFC-006：ドキュメントサイトの構築
---

# RFC-006：ドキュメントサイトの構築

> **状態**: 承認済み
> **著者**: 晨煦
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-12

> **参考**: RFC の仕様については [RFC テンプレート](RFC_TEMPLATE.md) を参照してください。

## 概要

YaoXiang ドキュメントサイトを設立し、分散したドキュメントを統合し、検索、ナビゲーション、多言語、版本切り替え機能を提供します。

## 動機

### この機能が必要な理由

現在、ドキュメントは複数のディレクトリに散らばっており、GitHub Readme でのみ表示されており、新規ユーザーは必要な情報を見つけることが困難です。検索もできず、中英語ドキュメントの同期も取れていません。

### 現在の問題

```
docs/
├── README.md              # メインインデックス（内容が限定的）
├── tutorial/              # チュートリアル
├── guides/               # ガイド
├── architecture/          # アーキテクチャドキュメント
├── design/               # デザインドキュメント
├── examples/             # 例
├── plans/                # 実施計画
├── implementation/       # 実装ドキュメント
├── maintenance/          # メンテナンスドキュメント
└── archived/             # アーカイブ
```

問題点：
1. 統合された入口がなく、GitHub Readme のみに依存
2. 検索機能がない
3. バージョン切り替え機能がないため、ユーザーは古いドキュメントを読む可能性がある
4. .obsidian がバージョン管理に混入

## 提案

### コア設計

```
┌─────────────────────────────────────────────────────────┐
│                    ドキュメントサイトフロントエンド        │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ ナビゲーションバー│ │ サイドバー │ │ バージョン切り替えドロップダウン│   │
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
│              GitHub Pages（ホスティング）                │
└─────────────────────────────────────────────────────────┘
```

### ディレクトリ構造（コア設計）

```
docs/
├── .vitepress/
│   ├── config.mts              # サイト設定
│   ├── navbar.ts              # ナビゲーションバー設定
│   └── sidebar/               # サイドバー設定
│       ├── zh.ts
│       └── en.ts
│
├── public/
│   ├── favicon.ico
│   └── logo.svg
│
├── zh/                        # 中国語ドキュメント
│   ├── index.md               # 中国語トップページ
│   ├── getting-started.md
│   ├── tutorial/
│   │   └── README.md
│   ├── reference/
│   │   └── README.md
│   ├── guide/
│   └── contributing.md
│
└── en/                        # 英語ドキュメント
    ├── index.md
    └── getting-started.md
```

### URL パス規範（コア設計）

| シナリオ | URL 形式 | 説明 |
|------|---------|------|
| 最新版の中国語 | `/zh/getting-started/` | 最新バージョンにリダイレクト |
| 最新版の英語 | `/en/getting-started/` | 最新バージョンにリダイレクト |
| 指定バージョン | `/v0.5/zh/getting-started/` | バージョン番号プレフィックス |
| トップページ | `/zh/` または `/en/` | 言語別トップページ |

**バージョン切り替え設計**：
```
バージョン切り替えドロップダウンメニュー：
├── v0.6 (latest)
├── v0.5
├── v0.4
└── v0.3
```

**バージョン_path規範**（重要な決定事項、後から変更困难）：
- 最新版：`/zh/xxx/` → 最新バージョンにリダイレクト
- 指定バージョン：`/v0.5/zh/xxx/` → 固定バージョン
- ナビゲーションバーのバージョン切り替え：`/v0.5/` と `/zh/` の組み合わせを切り替え

### サイドバー規範

```typescript
// docs/.vitepress/sidebar/zh.ts
export default {
  '/zh/tutorial/': [
    {
      text: 'チュートリアル',
      items: [
        { text: 'クイックスタート', link: '/zh/getting-started' },
        { text: '基礎', link: '/zh/tutorial/basics' },
      ],
    },
  ],
  '/zh/reference/': [
    {
      text: 'リファレンス',
      items: [
        { text: '組み込み関数', link: '/zh/reference/builtins' },
      ],
    },
  ],
}
```

### CI/CD 統合

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

## 詳細な設計

### ナビゲーションバー設定

```typescript
// docs/.vitepress/navbar.ts
export default [
  { text: '始める', link: '/zh/getting-started' },
  { text: 'チュートリアル', link: '/zh/tutorial/' },
  { text: 'リファレンス', link: '/zh/reference/' },
  { text: 'デザイン', link: '/zh/design/' },
  { text: 'GitHub', link: 'https://github.com/yaoxiang-lang/yaoxiang' },
]
```

### サイト設定

```typescript
// docs/.vitepress/config.mts
import { defineConfig } from 'vitepress'
import starlight from '@astrojs/starlight'

export default defineConfig({
  title: 'YaoXiang',
  description: '未来志向のプログラミング言語',

  locales: {
    root: { label: '中国語', lang: 'zh-CN', link: '/zh/' },
    en: { label: 'English', lang: 'en-US', link: '/en/' },
  },

  // ローカル検索
  plugins: [
    starlight({
      title: 'YaoXiang',
      localSearch: {},
    }),
  ],

  // 編集リンク
  editLink: {
    pattern: 'https://github.com/yaoxiang-lang/yaoxiang/edit/main/docs/:path',
  },
})
```

## トレードオフ

### メリット

- プロフェッショナルなドキュメントサイトによりプロジェクトのイメージ向上
- ユーザーが素早く必要な情報を見つけられる
- ローカル検索は無料で十分
- 多言語対応で国際コミュニティへのサービス提供
- バージョン切り替えにより古いドキュメントを読むことを回避

### デメリット

- メンテナンスコスト：サイト設定のメンテナンスが必要
- テクノロジース택の導入：Node.js

## 代替案

| 方案 | 選定しない理由 |
|------|-----------|
| GitHub Wiki | 検索性が悪く、カスタマイズ性が低い |
| README のみ | 検索功能、ナビゲーションがない |
| Docusaurus | 重量级で、起動が遅い |

## 実装戦略

### フェーズ分け

| フェーズ | 内容 | 状態 |
|------|------|------|
| P0 | VitePress + Starlight 設定の初期化 | 未着手 |
| P0 | ディレクトリ構造、ナビゲーションバー、サイドバーの設定 | 未着手 |
| P0 | README + クイックスタートの移行 | 未着手 |
| P0 | CI/CD による GitHub Pages への自動デプロイ | 未着手 |
| P1 | チュートリアル、リファレンスドキュメントの移行 | 未着手 |
| P1 | バージョン切り替えメニューの設定 | 未着手 |
| P2 | 英語ドキュメントの補足 | 未着手 |

### 依存関係

外部 RFC への依存なし

### リスク

| リスク | 影響 | 軽減措置 |
|------|------|---------|
| コンテンツの損失 | 移行前の完全バックアップ | |

## オープンな問題

**なし** - 全決定事項は既に確定済み

---

## 付録

### 付録A：設計決定記録

| 決定事項 | 決定内容 | 日付 | 記録者 |
|------|------|------|--------|
| SSG の選定 | VitePress + Starlight | 2025-02-07 | 晨煦 |
| ホスティングプラットフォーム | GitHub Pages | 2025-02-07 | 晨煦 |
| 検索方案 | ローカル検索 | 2025-02-07 | 晨煦 |
| 多言語構造 | `/zh/` と `/en/` プレフィックス | 2025-02-07 | 晨煦 |
| バージョンパス | `/v0.5/zh/` 形式 | 2025-02-07 | 晨煦 |

---

## 参考文献

- [VitePress ドキュメント](https://vitepress.dev/)
- [Starlight ドキュメント](https://starlight.astro.build/)