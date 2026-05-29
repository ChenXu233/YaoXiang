```markdown
---
title: パッケージマネージャー
description: YaoXiang パッケージマネージャー リファレンスドキュメント
---

# パッケージマネージャー

YaoXiang 組み込みのパッケージマネージャーで、プロジェクトの初期化、依存関係管理、バージョンロックなどの機能を提供します。

## 概要

YaoXiang パッケージマネージャー（以下简称 YPM）は Cargo と同様の設計理念を採用しています：

- **宣言的依存関係**：`yaoxiang.toml` で必要な依存関係を宣言
- **確定的ビルド**：`yaoxiang.lock` でバージョンをロックし、再現可能なビルドを確保
- **ローカルキャッシュ**：依存関係は `vendor` ディレクトリにダウンロードされ、オフライン使用をサポート

## クイックスタート

```bash
# 1. 新規プロジェクト作成
yaoxiang init my-project

# 2. 依存関係を追加
cd my-project
yaoxiang add http

# 3. 依存関係をインストール
yaoxiang install

# 4. プロジェクトを実行
yaoxiang run src/main.yx
```

## コマンド一覧

| コマンド | 説明 |
|------|------|
| [`yaoxiang init`](./commands#yaoxiang-init) | 新規プロジェクトを初期化 |
| [`yaoxiang add`](./commands#yaoxiang-add) | 依存関係を追加 |
| [`yaoxiang rm`](./commands#yaoxiang-rm) | 依存関係を削除 |
| [`yaoxiang install`](./commands#yaoxiang-install) | 依存関係をインストール |
| [`yaoxiang update`](./commands#yaoxiang-update) | 依存関係を更新 |
| [`yaoxiang list`](./commands#yaoxiang-list) | 依存関係を一覧表示 |

## プロジェクト構造

```
my-project/
├── yaoxiang.toml      # プロジェクトマニフェスト（必須）
├── yaoxiang.lock      # 依存関係ロックファイル（自動生成）
├── vendor/            # 依存関係保存ディレクトリ（自動生成）
└── src/
    └── main.yx       # エントリーポイント
```

## ドキュメントインデックス

- [コマンドラインインターフェース](./commands) - 全コマンドの詳細説明
- [yaoxiang.toml フォーマット](./manifest) - プロジェクト設定ファイルの形式
- [yaoxiang.lock フォーマット](./lock) - ロックファイルの形式説明
- [エラーコード](./error-codes) - 一般的なエラーと対処方法
```