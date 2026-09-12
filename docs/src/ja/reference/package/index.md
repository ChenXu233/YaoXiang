---
title: 'パッケージマネージャー'
description: YaoXiang パッケージマネージャーリファレンスドキュメント
---

# パッケージマネージャー

YaoXiang 内蔵のパッケージマネージャーが、プロジェクトの初期化、依存関係管理、バージョンロックなどの機能を提供します。

## 概要

YaoXiang パッケージマネージャー（略称 YPM）は、Cargo に類似した設計思想を採用しています：

- **宣言型依存関係**：`yaoxiang.toml` で必要な依存関係を宣言
- **決定論的ビルド**：`yaoxiang.lock` でバージョンをロックし、再現可能なビルドを保証
- **ローカルキャッシュ**：依存関係は `vendor` ディレクトリにダウンロードされ、オフライン使用に対応

## クイックスタート

```bash
# 1. 新規プロジェクトを作成
yx init my-project

# 2. 依存関係を追加
cd my-project
yx add http

# 3. 依存関係をインストール
yx install

# 4. プロジェクトを実行
yx run src/main.yx
```

## コマンド一覧

| コマンド                              | 説明                     |
| ------------------------------------- | ------------------------ |
| [`yx init`](./commands#yx-init)       | 新規プロジェクトを初期化 |
| [`yx add`](./commands#yx-add)         | 依存関係を追加           |
| [`yx rm`](./commands#yx-rm)           | 依存関係を削除           |
| [`yx install`](./commands#yx-install) | 依存関係をインストール   |
| [`yx update`](./commands#yx-update)   | 依存関係を更新           |
| [`yx list`](./commands#yx-list)       | 依存関係を一覧表示       |

## プロジェクト構造

```
my-project/
├── yaoxiang.toml      # プロジェクトマニフェスト（必須）
├── yaoxiang.lock      # 依存関係ロックファイル（自動生成）
├── vendor/            # 依存関係保存ディレクトリ（自動生成）
└── src/
    └── main.yx       # エントリーファイル
```

## ドキュメント一覧

- [コマンドラインインターフェース](./commands) - すべてのコマンドの詳細な説明
- [yaoxiang.toml フォーマット](./manifest) - プロジェクト設定ファイルの形式
- [yaoxiang.lock フォーマット](./lock) - ロックファイル形式の説明
- [エラーコード](./error-codes) - 一般的なエラーとその処理方法
