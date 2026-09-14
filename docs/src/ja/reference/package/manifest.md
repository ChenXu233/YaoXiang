---
title: 'yaoxiang.toml 形式'
description: 'プロジェクト設定ファイル形式の説明'
---

# yaoxiang.toml 形式

`yaoxiang.toml`
は YaoXiang プロジェクトのマニフェストファイルであり、プロジェクトのメタデータと依存関係を宣言します。

## ファイル構造

```toml
[package]
name = "プロジェクト名"
version = "0.1.0"
description = "プロジェクトの説明"
authors = ["著者名"]
license = "MIT"

[dependencies]
# 通常の依存関係

[dev-dependencies]
# 開発用依存関係
```

## package セクション

| フィールド    | 型     | 必須   | 説明                                                                   |
| ------------- | ------ | ------ | ---------------------------------------------------------------------- |
| `name`        | string | はい   | プロジェクト名（命名規則に従う：小文字アルファベット、数字、ハイフン） |
| `version`     | string | はい   | セマンティックバージョニング（semver 仕様に準拠）                      |
| `description` | string | いいえ | プロジェクトの簡単な説明                                               |
| `authors`     | array  | いいえ | 著者リスト                                                             |
| `license`     | string | いいえ | ライセンス識別子                                                       |

### 例

```toml
[package]
name = "my-awesome-app"
version = "1.2.3"
description = "素晴らしいアプリケーション"
authors = ["张三 <zhangsan@example.com>"]
license = "MIT"
```

## 依存関係の宣言

### 簡易バージョン

```toml
[dependencies]
http = "1.0.0"
json = "*"
```

### 詳細設定

```toml
[dependencies]
# Git 依存
http = { version = "1.0.0", git = "https://github.com/example/http" }

# ローカルパス依存
utils = { version = "0.1.0", path = "./utils" }

# ブランチ付き Git 依存
bleeding-edge = { git = "https://github.com/example/edge", branch = "main" }
```

### 依存関係フィールドの説明

| フィールド | 型     | 説明                               |
| ---------- | ------ | ---------------------------------- |
| `version`  | string | バージョン番号またはバージョン範囲 |
| `git`      | string | Git リポジトリの URL               |
| `branch`   | string | Git ブランチ名                     |
| `path`     | string | ローカル相対パス                   |

## バージョン番号の構文

| 構文              | 説明               | 例                  |
| ----------------- | ------------------ | ------------------- |
| `*`               | 任意バージョン     | `"*"`               |
| `1.0.0`           | 完全一致バージョン | `"1.0.0"`           |
| `>=1.0.0`         | 最低バージョン     | `">=1.0.0"`         |
| `<2.0.0`          | 最高バージョン     | `"<2.0.0"`          |
| `>=1.0.0, <2.0.0` | 範囲バージョン     | `">=1.0.0, <2.0.0"` |
| `~1.0.0`          | 互換バージョン     | `"~1.0.0"`          |
| `^1.0.0`          | caret バージョン   | `"^1.0.0"`          |

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
