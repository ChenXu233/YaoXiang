---
title: 'RFC-006: ドキュメントサイト構築'
status: '承認済み'
author: '晨煦'
created: '2025-01-05'
updated: '2026-07-05'

issue: '#130'
---

# RFC-006: ドキュメントサイト構築

> **参考**: RFC の仕様については [RFC テンプレート](RFC_TEMPLATE.md) をご覧ください。

## 概要

YaoXiang ドキュメントサイトを構築し、分散したドキュメントを集約し、検索、ナビゲーション、多言語対応、バージョン切り替えをサポートします。

## 動機

### この機能が必要な理由

現在、ドキュメントは複数のディレクトリに散らばっており、GitHub の Readme のみで展示されているため、新しいユーザーは必要な情報を見つけにくく、検索もできず、中英語ドキュメントの同期も取れていません。

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

1. 統一された入口がなく、GitHub Readme のみに依存
2. 検索機能が없ない
3. バージョン切り替えがなく、ユーザーは古いドキュメントを読む可能性がある
4. .obsidian がバージョン管理に混在している

## 提案

### コアデザイン

```
┌─────────────────────────────────────────────────────────┐
│                    ドキュメントサイトフロントエンド               │
│  ┌───────────┐ ┌───────────┐ ┌─────────────────────┐   │
│  │ ナビゲーションバー│ │ サイドバー    │ │ バージョン切替ドロップダウン│   │
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
│              GitHub Pages（ホスティング）                        │
└─────────────────────────────────────────────────────────┘
```

### ディレクトリ構造（コアデザイン）

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
│   ├── index.md               # 中国語ホームページ
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

### URL パス規範（コアデザイン）

| シナリオ       | URL フォーマット                    | 説明           |
| ---------- | --------------------------- | -------------- |
| 最新中国語 | `/zh/getting-started/`      | 最新バージョンにリダイレクト |
| 最新英語 | `/en/getting-started/`      | 最新バージョンにリダイレクト |
| 指定バージョン   | `/v0.5/zh/getting-started/` | バージョン番号プレフィックス     |
| ホームページ       | `/zh/` または `/en/`            | 言語ホームページ       |

**バージョン切り替えデザイン**：

```
バージョン切替ドロップダウンメニュー：
├── v0.6 (latest)
├── v0.5
├── v0.4
└── v0.3
```

**バージョンパス規範**（重要な決定事項、後から変更困難）：

- 最新バージョン：`/zh/xxx/` → 最新バージョンにリダイレクト
- 指定バージョン：`/v0.5/zh/xxx/` → 固定バージョン
- ナビゲーションバーのバージョン切り替え：`/v0.5/` と `/zh/` の組み合わせ切り替え

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
      items: [{ text: '組み込み関数', link: '/zh/reference/builtins' }],
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
  { text: 'はじめに', link: '/zh/getting-started' },
  { text: 'チュートリアル', link: '/zh/tutorial/' },
  { text: 'リファレンス', link: '/zh/reference/' },
  { text: 'デザイン', link: '/zh/design/' },
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
  description: '未来を向けたプログラミング言語',

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
});
```

## トレードオフ

### メリット

- プロフェッショナルなドキュメントサイトによりプロジェクトイメージ向上
- ユーザーが所需情報を見つけやすく
- ローカル検索は無料で十分
- 多言語対応で国際コミュニティへのサービス
- バージョン切り替えで古いドキュメントを読むことを回避

### デメリット

- メンテナンスコスト：サイト設定のメンテナンスが必要
- 技術スタックの導入：Node.js

## 代替案

| 案        | 採用しない理由       |
| ----------- | ---------------- |
| GitHub Wiki | 検索が不便で、カスタマイズ性が低い |
| README のみ   | 検索がなく、ナビゲーションがない   |
| Docusaurus  | 重く、起動が遅い     |

## 実装戦略

### フェーズ分け

| フェーズ | 内容                              | ステータス |
| ---- | --------------------------------- | ---- |
| P0   | VitePress + Starlight 設定の初期化 | 未着手 |
| P0   | ディレクトリ構造、ナビゲーションバー、サイドバーの設定      | 未着手 |
| P0   | README + クイックスタートの移行            | 未着手 |
| P0   | CI/CD による GitHub Pages への自動デプロイ     | 未着手 |
| P1   | チュートリアル、リファレンスドキュメントの移行                | 未着手 |
| P1   | バージョン切替メニュー設定                  | 未着手 |
| P2   | 英語ドキュメント補完                      | 未着手 |

### 依存関係

外部 RFC への依存なし

### リスク

| リスク     | 影響           | 緩和策     |
| -------- | -------------- | ------------ |
| コンテンツ喪失 | 非常に大きい | 移行前の完全バックアップ |

## 開放問題

**なし** - 全决策都已确定

---

## 付録

### 付録A：設計決定記録

| 決定       | 決定事項                  | 日付       | 記録者 |
| ---------- | --------------------- | ---------- | ------ |
| SSG 選定   | VitePress + Starlight | 2025-02-07 | 晨煦   |
| ホスティングプラットフォーム   | GitHub Pages          | 2025-02-07 | 晨煦   |
| 検索方案   | ローカル検索              | 2025-02-07 | 晨煦   |
| 多言語構造 | `/zh/` と `/en/` プレフィックス | 2025-02-07 | 晨煦   |
| バージョンパス   | `/v0.5/zh/` フォーマット      | 2025-02-07 | 晨煦   |

---

## 参考文献

- [VitePress ドキュメント](https://vitepress.dev/)
- [Starlight ドキュメント](https://starlight.astro.build/)
