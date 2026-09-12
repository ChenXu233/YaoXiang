---
title: 'yaoxiang.lock フォーマット'
description: '依存関係ロックファイルフォーマットの説明'
---

# yaoxiang.lock フォーマット

`yaoxiang.lock`
は YaoXiang の依存関係ロックファイルであり、すべての依存関係の正確なバージョン情報を記録します。

## 概要

- **自動生成**：`yx install` および `yx update` によって自動生成・更新されます
- **手動で編集しないでください**：このファイルはパッケージマネージャによって自動的に管理されます
- **バージョン管理にコミットすべき**：チームメンバーや CI ビルドが同じ依存関係バージョンを使用することを保証します

## ファイル構造

```toml
# YaoXiang パッケージマネージャによって自動生成

[package]
version = 1

[package.依存関係名]
version = "バージョン番号"
source = "ソースタイプ"
checksum = "チェックサム（オプション）"
```

## フィールドの説明

### [package] セクション

| フィールド | 型      | 説明                                           |
| ---------- | ------- | ---------------------------------------------- |
| `version`  | integer | ロックファイルフォーマットのバージョン、現在 1 |

### [package.\<name\>] セクション

| フィールド | 型     | 説明                                      |
| ---------- | ------ | ----------------------------------------- |
| `version`  | string | 解析後の正確なバージョン番号              |
| `source`   | string | 依存関係ソース：`registry`、`git`、`path` |
| `checksum` | string | SHA-256 チェックサム（オプション）        |

## 例

```toml
# YaoXiang パッケージマネージャによって自動生成
# 2024-01-15 10:30:00

[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"
checksum = "abc123..."

[package.json]
version = "2.0.0"
source = "registry"

[package.utils]
version = "0.1.0"
source = "path"

[package.bleeding]
version = "main"
source = "git"
```

## ソースタイプ

| タイプ     | 説明                       | 設定例                    |
| ---------- | -------------------------- | ------------------------- |
| `registry` | リモートレジストリから取得 | `http = "1.0.0"`          |
| `git`      | Git リポジトリから取得     | `{ git = "https://..." }` |
| `path`     | ローカルパスから取得       | `{ path = "./lib" }`      |

## manifest との関係

```
yaoxiang.toml          yaoxiang.lock
     │                       │
     │   yx install    │
     ├──────────────────────►│
     │                       │
     │   宣言 "http = *>"   │ ロック "http = 1.0.0"
     │                       │
```

- `yaoxiang.toml`：**欲しい**依存関係を宣言する（範囲指定可能）
- `yaoxiang.lock`：**実際にインストールされた**バージョンを記録する（正確なバージョン）
