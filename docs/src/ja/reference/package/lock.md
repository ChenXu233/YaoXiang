```markdown.ja
---
title: yaoxiang.lock フォーマット
description: 依存関係ロックファイル形式の解説
---

# yaoxiang.lock フォーマット

`yaoxiang.lock` は YaoXiang の依存関係ロックファイルであり、すべての依存関係の正確なバージョン情報を記録します。

## 概要

- **自動生成**：`yaoxiang install` と `yaoxiang update` によって自動的に生成および更新されます
- **手動での編集は禁止**：このファイルはパッケージマネージャーによって自動管理されます
- **バージョン管理に含めるべき**：チームメンバーと CI ビルドが同じ依存関係バージョンを使用することを保証します

## ファイル構造

```toml
# YaoXiang パッケージマネージャーによって自動生成

[package]
version = 1

[package.依存名]
version = "バージョン番号"
source = "ソースタイプ"
checksum = "チェックサム（オプション）"
```

## フィールド説明

### [package] セクション

| フィールド | タイプ | 説明 |
|------|------|------|
| `version` | integer | ロックファイル形式バージョン、現在は 1 |

### [package.\<name\>] セクション

| フィールド | タイプ | 説明 |
|------|------|------|
| `version` | string | 解決後の正確なバージョン番号 |
| `source` | string | 依存元のソース：`registry`、`git`、`path` |
| `checksum` | string | SHA-256 チェックサム（オプション） |

## 例

```toml
# YaoXiang パッケージマネージャーによって自動生成
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

| タイプ | 説明 | 設定例 |
|------|------|----------|
| `registry` | リモートレジストリから取得 | `http = "1.0.0"` |
| `git` | Git リポジトリから取得 | `{ git = "https://..." }` |
| `path` | ローカルパスから取得 | `{ path = "./lib" }` |

## manifest との関係

```
yaoxiang.toml          yaoxiang.lock
     │                       │
     │   yaoxiang install    │
     ├──────────────────────►│
     │                       │
     │   宣言 "http = *>"   │ ロック "http = 1.0.0"
     │                       │
```

- `yaoxiang.toml`：**欲しい**依存関係を宣言（範囲可以使用）
- `yaoxiang.lock`：**実際にインストール**されたバージョンを記録（正確なバージョン）
```