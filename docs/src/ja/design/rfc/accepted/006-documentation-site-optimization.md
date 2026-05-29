---
title: RFC-006：文書サイトの構築
---

# RFC-006: 文書サイトの構築

> **状態**: 承認済み
> **作成者**: 晨煦
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-12

> **参照**: RFC の仕様については [RFC テンプレート](RFC_TEMPLATE.md) をご覧ください。

## 概要

YaoXiang 文書サイトを確立し、分散した文書を集約し、検索、ナビゲーション、多言語対応、バージョン切り替えのサポートを提供します。

## 動機

### この機能がなぜ必要なのか？

現在、文書は複数のディレクトリに散らばっており、GitHub の Readme でのみ表示されているため、新規ユーザーは必要な情報を見つけることが難しく、検索もできず、中英語ドキュメントが同期していません。

### 現在の問題

```
docs/
├── README.md              # メインインデックス（内容が限定的）
├── tutorial/              # チュートリアル
├── guides/               # ガイド
├── architecture/          # アーキテクチャ文書
├── design/               # 設計文書
├── examples/             # 例
├── plans/                # 実施計画
├── implementation/       # 実装文書
├── maintenance/          # メンテナンス文書
└── archived/             # アーカイブ
```

問題点：
1. 統一されたエントリーポイントがなく、GitHub Readme のみに依存
2. 検索機能ががない
3. バージョンの切り替えがなく、ユーザーは古いドキュメントを読む可能性がある
4. `.obsidian` がバージョン管理に混在している

## 提案

### コア設計

```
┌─────────────────────────────────────────────────────────┐
│                    文書サイトフロントエンド               │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ ナビゲーションバー│ │ サイドバー    │ │ バージョン切替ドロップダウン │   │
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
│              GitHub Pages（ホスティング）                  │
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
├── zh/                        # 中国語文書
│   ├── index.md               # 中国語トップページ
│   ├── getting-started.md
│   ├── tutorial/
│   │   └── README.md
│   ├── reference/
│   │   └── README.md
│   ├── guide/
│   └── contributing.md
│
└── en/                        # 英語文書
    ├── index.md
    └── getting-started.md
```

### URL パス仕様（コア設計）

| シナリオ | URL 形式 | 説明 |
|------|---------|------|
| 最新版の中国語 | `/zh/getting-started/` | 最新バージョンにリダイレクト |
| 最新版の英語 | `/en/getting-started/` | 最新バージョンにリダイレクト |
| 指定バージョン | `/v0.5/zh/getting-started/` | バージョン番号プレフィックス |
| トップページ | `/zh/` または `/en/` | 言語別トップページ |

**バージョン切り替え設計**：
```
バージョン切替ドロップダウンメニュー：
├── v0.6 (最新)
├── v0.5
├── v0.4
└── v0.3
```

**バージョンパス仕様**（重要な決定事項であり、後から変更困難）：
- 最新版：`/zh/xxx/` → 最新バージョンにリダイレクト
- 指定バージョン：`/v0.5/zh/xxx/` → 固定バージョン
- ナビゲーションバーのバージョン切替：`/v0.5/` と `/zh/` の組み合わせを切り替え

### サイドバー仕様

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
name: 文書のデプロイ

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

## 詳細設計

### ナビゲーションバー設定

```typescript
// docs/.vitepress/navbar.ts
export default [
  { text: 'はじめに', link: '/zh/getting-started' },
  { text: 'チュートリアル', link: '/zh/tutorial/' },
  { text: 'リファレンス', link: '/zh/reference/' },
  { text: '設計', link: '/zh/design/' },
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
  description: '未来を向けたプログラミング言語',

  locales: {
    root: { label: '中文', lang: 'zh-CN', link: '/zh/' },
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

### 利点

- 専門的な文書サイトがプロジェクトの印象を向上
- ユーザーがすぐに必要な情報を見つけられる
- ローカル検索は無料で十分
- 多言語対応で国際コミュニティを支援
- バージョン切り替えで古いドキュメントを読むことを 방지

### 欠点

- メンテナンスコスト：サイト設定のメンテナンスが必要
- 技術スタックの導入：Node.js

## 代替案

| 案 | 選択しない理由 |
|------|-----------|
| GitHub Wiki | 検索機能が劣り、カスタマイズ性が低い |
| README のみ | 検索機能がなく、ナビゲーションがない |
| Docusaurus | 重量级であり、起動が遅い |

## 実装戦略

### 段階的アプローチ

| 段階 | 内容 | 状態 |
|------|------|------|
| P0 | VitePress + Starlight 設定の初期化 | 未着手 |
| P0 | ディレクトリ構造、ナビゲーションバー、サイドバーの設定 | 未着手 |
| P0 | README + クイックスタートの移行 | 未着手 |
| P0 | CI/CD による GitHub Pages への自動デプロイ | 未着手 |
| P1 | チュートリアル、リファレンス文書の移行 | 未着手 |
| P1 | バージョン切替メニュー設定 | 未着手 |
| P2 | 英語文書の補完 | 未着手 |

### 依存関係

外部 RFC への依存なし

### リスク

| リスク | 影響 | 緩和策 |
|------|------|---------|
| コンテンツの消失 | 移行前の完全バックアップ |

## 未解決の問題

**なし** - すべての決定はすでに完了しています

---

## 付録

### 付録A：設計決定記録

| 決定事項 | 決定内容 | 日付 | 記録者 |
|------|------|------|--------|
| SSG の選定 | VitePress + Starlight | 2025-02-07 | 晨煦 |
| ホスティングプラットフォーム | GitHub Pages | 2025-02-07 | 晨煦 |
| 検索ソリューション | ローカル検索 | 2025-02-07 | 晨煦 |
| 多言語構造 | `/zh/` と `/en/` プレフィックス | 2025-02-07 | 晨煦 |
| バージョンパス | `/v0.5/zh/` 形式 | 2025-02-07 | 晨煦 |

---

## 参考文献

- [VitePress 文書](https://vitepress.dev/)
- [Starlight 文書](https://starlight.astro.build/)