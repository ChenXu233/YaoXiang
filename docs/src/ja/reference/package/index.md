```markdown
---
title: パッケージマネージャー
description: YaoXiang パッケージマネージャー リファレンスドキュメント
---

# パッケージマネージャー

YaoXiang 組み込みのパッケージマネージャー。プロジェクト初期化、依存関係管理、バージョンロックなどの機能を提供します。

## 概要

YaoXiang パッケージマネージャー（略称 YPM）は、Cargo と同様の設計思想を採用しています：

- **宣言的な依存関係**：`yaoxiang.toml` で必要な依存関係を宣言
- **決定論的ビルド**：`yaoxiang.lock` でバージョンをロックし、再現可能なビルドを確保
- **ローカルキャッシュ**：依存関係を `vendor` ディレクトリにダウンロードし、オフライン利用をサポート

## クイックスタート

```bash
# 1. 新規プロジェクトの作成
yaoxiang init my-project

# 2. 依存関係の追加
cd my-project
yaoxiang add http

# 3. 依存関係のインストール
yaoxiang install

# 4. プロジェクトの実行
yaoxiang run src/main.yx
```

## コマンド一覧

| コマンド | 説明 |
|------|------|
| [`yaoxiang init`](./commands#yaoxiang-init) | 新規プロジェクトの初期化 |
| [`yaoxiang add`](./commands#yaoxiang-add) | 依存関係の追加 |
| [`yaoxiang rm`](./commands#yaoxiang-rm) | 依存関係の削除 |
| [`yaoxiang install`](./commands#yaoxiang-install) | 依存関係のインストール |
| [`yaoxiang update`](./commands#yaoxiang-update) | 依存関係の更新 |
| [`yaoxiang list`](./commands#yaoxiang-list) | 依存関係の一覧表示 |

## プロジェクト構造

```
my-project/
├── yaoxiang.toml      # プロジェクトマニフェスト（必須）
├── yaoxiang.lock      # 依存関係ロックファイル（自動生成）
├── vendor/            # 依存関係保管ディレクトリ（自動生成）
└── src/
    └── main.yx       # エントリーポイント
```

## ドキュメントインデックス

- [コマンドラインインターフェース](./commands) - 全コマンドの詳細説明
- [yaoxiang.toml フォーマット](./manifest) - プロジェクト設定ファイルの形式
- [yaoxiang.lock フォーマット](./lock) - ロックファイルの形式説明
- [エラーコード](./error-codes) - 一般的なエラーと対処方法
```