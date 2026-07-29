---
title: 'パッケージマネージャー'
description: YaoXiang パッケージマネージャーリファレンスドキュメント
---

# パッケージマネージャー

YaoXiangに組み込まれたパッケージマネージャーで、プロジェクトの初期化、依存関係管理、バージョンロックなどの機能を提供します。

## 概要

YaoXiang パッケージマネージャー（略称 YPM）は、Cargoに似た設計思想を採用しています：

- **宣言的依存関係**：`yaoxiang.toml` で必要な依存関係を宣言
- **決定論的ビルド**：`yaoxiang.lock` でバージョンをロックし、再現可能なビルドを保証
- **ローカルキャッシュ**：依存関係は `vendor` ディレクトリにダウンロードされ、オフライン利用が可能

## クイックスタート

```bash
# 1. 新規プロジェクト作成
yaoxiang init my-project

# 2. 依存関係追加
cd my-project
yaoxiang add http

# 3. 依存関係インストール
yaoxiang install

# 4. プロジェクト実行
yaoxiang run src/main.yx
```

## コマンド一覧

| コマンド                                          | 説明                   |
| ------------------------------------------------- | ---------------------- |
| [`yaoxiang init`](./commands#yaoxiang-init)       | 新規プロジェクト初期化 |
| [`yaoxiang add`](./commands#yaoxiang-add)         | 依存関係追加           |
| [`yaoxiang rm`](./commands#yaoxiang-rm)           | 依存関係削除           |
| [`yaoxiang install`](./commands#yaoxiang-install) | 依存関係インストール   |
| [`yaoxiang update`](./commands#yaoxiang-update)   | 依存関係更新           |
| [`yaoxiang list`](./commands#yaoxiang-list)       | 依存関係一覧           |

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

- [コマンドラインインターフェース](./commands) - 全コマンドの詳細な説明
- [yaoxiang.toml 形式](./manifest) - プロジェクト設定ファイルの形式
- [yaoxiang.lock 形式](./lock) - ロックファイルの形式説明
- [エラーコード](./error-codes) - 一般的なエラーと対処方法
