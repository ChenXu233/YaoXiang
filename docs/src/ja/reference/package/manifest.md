---
title: "yaoxiang.toml フォーマット"
description: "プロジェクト設定ファイルのフォーマット説明"
---

# yaoxiang.toml フォーマット

`yaoxiang.toml` は YaoXiang プロジェクトのマニフェストファイルであり、プロジェクトメタデータと依存関係を宣言します。

## ファイル構造

```toml
[package]
name = "プロジェクト名"
version = "0.1.0"
description = "プロジェクトの説明"
authors = ["著者名"]
license = "MIT"

[dependencies]
# 通常依存関係

[dev-dependencies]
# 開発用依存関係
```

## package 部

| フィールド | 型 | 必須 | 説明 |
|------|------|------|------|
| `name` | string | はい | プロジェクト名。命名規則（小文字、数字、ハイフン）に従うこと |
| `version` | string | はい | セマンティックバージョニング。semver 仕様に準拠 |
| `description` | string | いいえ | プロジェクト簡潔説明 |
| `authors` | array | いいえ | 著者リスト |
| `license` | string | いいえ | ライセンス識別子 |

### 例

```toml
[package]
name = "my-awesome-app"
version = "1.2.3"
description = "素晴らしいアプリ"
authors = ["張三 <zhangsan@example.com>"]
license = "MIT"
```

## 依存関係宣言

### シンプルバージョン

```toml
[dependencies]
http = "1.0.0"
json = "*"
```

### 詳細設定

```toml
[dependencies]
# Git 依存関係
http = { version = "1.0.0", git = "https://github.com/example/http" }

# ローカルパス依存関係
utils = { version = "0.1.0", path = "./utils" }

# ブランチ指定の Git 依存関係
bleeding-edge = { git = "https://github.com/example/edge", branch = "main" }
```

### 依存関係フィールド説明

| フィールド | 型 | 説明 |
|------|------|------|
| `version` | string | バージョン番号またはバージョン範囲 |
| `git` | string | Git リポジトリアドレス |
| `branch` | string | Git ブランチ名 |
| `path` | string | ローカル相対パス |

## バージョン番号構文

| 構文 | 説明 | 例 |
|------|------|------|
| `*` | 任意バージョン | `"*"` |
| `1.0.0` | 正確バージョン | `"1.0.0"` |
| `>=1.0.0` | 最低バージョン | `">=1.0.0"` |
| `<2.0.0` | 最高バージョン | `"<2.0.0"` |
| `>=1.0.0, <2.0.0` | 範囲バージョン | `">=1.0.0, <2.0.0"` |
| `~1.0.0` | 互換バージョン | `"~1.0.0"` |
| `^1.0.0` | caret バージョン | `"^1.0.0"` |

## 完全な例

```toml
[package]
name = "web-server"
version = "0.1.0"
description = "シンプルな Web サーバー"
authors = ["開発者 <dev@example.com>"]
license = "MIT"

[dependencies]
http = "1.0.0"
json = "2.0.0"
router = { version = "0.5.0", path = "./router" }

[dev-dependencies]
test-utils = "1.0.0"
benchmark = "0.1.0"
```