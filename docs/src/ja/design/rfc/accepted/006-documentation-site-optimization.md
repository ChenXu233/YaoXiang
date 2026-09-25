---
title: 'RFC-006: ドキュメントサイトの構築'
status: '承認済み'
author: '晨煦'
created: '2025-01-05'
updated: '2026-07-05'

issue: '#130'
---

# RFC-006: ドキュメントサイトの構築

> **参考**: [RFC テンプレート](../RFC_TEMPLATE.md) を参照して RFC 仕様を確認してください。

## 概要

YaoXiang のドキュメントサイトを構築し、散在するドキュメントを統合して、検索・ナビゲーション・多言語・バージョン切り替えの機能を提供します。

## 動機

### なぜこの機能が必要なのか？

現在、ドキュメントは複数のディレクトリに散在しており、GitHub の Readme だけで表示されています。そのため、新規ユーザーは必要な情報を見つけにくく、検索もできず、中文と英文のドキュメントも同期されていません。

### 現在の問題点

```
docs/
├── README.md              # メイン索引（内容が限定的）
├── tutorial/              # チュートリアル
├── guides/               # ガイド
├── architecture/          # アーキテクチャドキュメント
├── design/               # 設計ドキュメント
├── examples/             # サンプル
├── plans/                # 実施計画
├── implementation/       # 実装ドキュメント
├── maintenance/          # メンテナンスドキュメント
└── archived/             # アーカイブ
```

問題点：

1. 統一された入口がなく、GitHub の Readme のみ
2. 検索できない
3. バージョン切り替えがなく、ユーザーが古いドキュメントを読む可能性がある
4. `.obsidian` がバージョン管理に混入している

## 提案

### 中核設計

```
┌─────────────────────────────────────────────────────────┐
│                  ドキュメントサイトフロントエンド          │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ ナビゲーション │ │ サイドバー │ │ バージョン切替ドロップダウン│
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

### ディレクトリ構造（中核設計）

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
├── zh/                        # 中文ドキュメント
│   ├── index.md               # 中文トップページ
│   ├── getting-started.md
│   ├── tutorial/
│   │   └── README.md
│   ├── reference/
│   │   └── README.md
│   ├── guide/
│   └── contributing.md
│
└── en/                        # 英文ドキュメント
    ├── index.md
    └── getting-started.md
```

### URL パス仕様（中核設計）

| シナリオ       | URL 形式                    | 説明                         |
| -------------- | --------------------------- | ---------------------------- |
| 最新版中文     | `/zh/getting-started/`      | 最新バージョンへリダイレクト |
| 最新版英文     | `/en/getting-started/`      | 最新バージョンへリダイレクト |
| 指定バージョン | `/v0.5/zh/getting-started/` | バージョン番号プレフィックス |
| トップページ   | `/zh/` または `/en/`        | 言語別トップページ           |

**バージョン切替設計**：

```
バージョン切替ドロップダウン：
├── v0.6 (latest)
├── v0.5
├── v0.4
└── v0.3
```

**バージョンパス仕様**（重要な決定事項、後で変更困難）：

- 最新版：`/zh/xxx/` → 最新バージョンへリダイレクト
- 指定バージョン：`/v0.5/zh/xxx/` → バージョン固定
- ナビゲーションバーのバージョン切替：`/v0.5/` と `/zh/` の組み合わせを切り替え

### サイドバー仕様

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

## 詳細設計

### ナビゲーションバー設定

```typescript
// docs/.vitepress/navbar.ts
export default [
  { text: '開始', link: '/zh/getting-started' },
  { text: '教程', link: '/zh/tutorial/' },
  { text: '参考', link: '/zh/reference/' },
  { text: '設計', link: '/zh/design/' },
  { text: 'GitHub', link: 'https://github.com/yaoxiang-lang/yaoxiang' },
];
```

### サイト設定

```typescript
// docs/.vitepress/config.mts
import { defineConfig } from 'vitepress';
import starlight from '@astrojs/starlight';

export default defineConfig({
  title: 'YaoXiang',
  description: '未来志向のプログラミング言語',

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
});
```

## トレードオフ

### メリット

- プロフェッショナルなドキュメントサイトでプロジェクトのイメージが向上
- ユーザーが必要な情報を迅速に見つけられる
- ローカル検索で十分かつ無料
- 多言語サポートで国際コミュニティに貢献
- バージョン切替により古いドキュメントの閲覧を防止

### デメリット

- メンテナンスコスト：サイト設定の保守が必要
- 技術スタックの導入：Node.js

## 代替案

| 案          | 採用しない理由                   |
| ----------- | -------------------------------- |
| GitHub Wiki | 検索が弱く、カスタマイズ性が低い |
| README のみ | 検索・ナビゲーションがない       |
| Docusaurus  | 重く、起動が遅い                 |

## 実装戦略

### 段階分け

| 段階 | 内容                                                   | 状態   |
| ---- | ------------------------------------------------------ | ------ |
| P0   | VitePress + Starlight の初期化設定                     | 未着手 |
| P0   | ディレクトリ構造・ナビゲーションバー・サイドバーの設定 | 未着手 |
| P0   | README + クイックスタートの移行                        | 未着手 |
| P0   | CI/CD による GitHub Pages への自動デプロイ             | 未着手 |
| P1   | チュートリアル・参考ドキュメントの移行                 | 未着手 |
| P1   | バージョン切替メニューの設定                           | 未着手 |
| P2   | 英文ドキュメントの補充                                 | 未着手 |

### 依存関係

外部 RFC への依存なし

### リスク

| リスク           | 影響 | 緩和策                           |
| ---------------- | ---- | -------------------------------- |
| コンテンツの損失 | 甚大 | 移行前に完全なバックアップを取得 |

## オープンな問題

**なし** - すべての決定は下されています

---

## 付録

### 付録A：設計決定の記録

| 決定                         | 選択                            | 日付       | 記録者 |
| ---------------------------- | ------------------------------- | ---------- | ------ |
| SSG の選定                   | VitePress + Starlight           | 2025-02-07 | 晨煦   |
| ホスティングプラットフォーム | GitHub Pages                    | 2025-02-07 | 晨煦   |
| 検索方式                     | ローカル検索                    | 2025-02-07 | 晨煦   |
| 多言語構造                   | `/zh/` と `/en/` プレフィックス | 2025-02-07 | 晨煦   |
| バージョンパス               | `/v0.5/zh/` 形式                | 2025-02-07 | 晨煦   |

---

## 参考文献

- [VitePress ドキュメント](https://vitepress.dev/)
- [Starlight ドキュメント](https://starlight.astro.build/)
