---
title: "RFC-015: YaoXiang 設定システム設計"
status: "受領済み"
author: "晨煦"
created: "2026-02-12"
updated: "2026-02-15"
---

# RFC-015: YaoXiang 設定システム設計

> **受領日**: 2026-02-15

> **前置 RFC**: [RFC-014: パッケージ管理システム設計](014-package-manager.md)

## 摘要

YaoXiang 言語の統一設定システムを設計し、ユーザーレベルとプロジェクトレベルの2階層をサポートし、パッケージマネージャー、コンパイラ、REPL、LSPなどのコンポーネントに共有の設定インフラを提供する。

## 動機

### なぜこの機能/変更が必要か？

YaoXiang ツールチェーンは複数のコンポーネントを含む：
- パッケージマネージャー（依存関係設定を読み取る）
- コンパイラフロントエンド（i18n設定を読み取る）
- REPL（対話設定を読み取る）
- LSP（fmt/lint/test設定を読み取る）
- ビルドシステム（ビルド設定を読み取る）

各コンポーネントは統一された設定インフラを必要とする。

###  現在の問題

- 各コンポーネントの設定が分散し、統一された仕様がない
- ユーザーが偏好設定を統一的に管理できない
- プロジェクト設定とユーザー設定に明確な階層がない

## 提案

### コア設計

**レイヤーアーキテクチャ**：
```
設定の優先度（高 → 低）：
┌─────────────────────────────────────────────┐
│ 1. プロジェクトレベル yaoxiang.toml          │ ← プロジェクトチームが管理
├─────────────────────────────────────────────┤
│ 2. ユーザーレベル ~/.config/yaoxiang/config.toml │ ← ユーザーの偏好
├─────────────────────────────────────────────┤
│ 3. コンパイラのデフォルト値                  │ ← 合理的な初期値
└─────────────────────────────────────────────┘
```

**ルール**：上位層が下位層を上書きし、設定されていないオプションは下位層にフォールバックする。

### 設定レイヤー制限

| 設定セクション | ユーザーレベル | プロジェクトレベル | コンシューマー |
|--------|--------|--------|--------|
| `[package].*` | ❌ | ✅ | パッケージマネージャー |
| `[yaoxiang]` | ❌ | ✅ | コンパイラ |
| `[dependencies]` | ❌ | ✅ | パッケージマネージャー |
| `[dev-dependencies]` | ❌ | ✅ | パッケージマネージャー |
| `[bin]` | ❌ | ✅ | パッケージマネージャー |
| `[lib]` | ❌ | ✅ | パッケージマネージャー |
| `[build]` | ✅ | ✅ | ビルドシステム |
| `[profile.*]` | ✅ | ✅ | ビルドシステム |
| `[install]` | ✅ | ❌ | パッケージマネージャー |
| `[i18n]` | ✅ | ✅ | コンパイラ |
| `[repl]` | ✅ | ✅ | REPL |
| `[fmt]` | ✅ | ✅ | LSP |
| `[lint]` | ✅ | ✅ | LSP |
| `[test]` | ✅ | ✅ | LSP |
| `[tasks]` | ✅ | ✅ | CLI |

### 例

**プロジェクトレベル設定**：
```toml
# yaoxiang.toml
[package]
name = "my-package"
version = "0.1.0"

[yaoxiang]
version = ">=0.1.0, <1.0.0"

[dependencies]
foo = "^1.0.0"

[build]
output = "dist/"

[tasks]
build = "yaoxiang build"
test = "yaoxiang test"
```

**ユーザーレベル設定**：
```toml
# ~/.config/yaoxiang/config.toml
[install]
dir = "~/.local/share/yaoxiang"

[i18n]
lang = "zh"
fallback = "en"

[repl]
history-size = 1000
prompt = "yx> "
colors = true

[fmt]
line-width = 120
indent-width = 4

[lint]
rules = ["recommended"]
```

## 詳細設計

### プロジェクトレベル専用設定

```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-project"

[yaoxiang]
version = ">=0.1.0, <1.0.0"

[dependencies]
foo = "^1.0.0"

[dev-dependencies]
test-utils = "0.1.0"

[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./foo" = "src/foo.yx"

[build]
script = "build.yx"
output = "dist/"

[profile.release]
optimize = true
lto = true

[run]
main = "src/main.yx"
args = ["--quiet"]

[tasks]
build = "yaoxiang build"
test = "yaoxiang test"
lint = "yaoxiang fmt && yaoxiang check"
```

### ユーザーレベル専用設定

```toml
[install]
dir = "~/.local/share/yaoxiang"
```

### 両方で使用可能な設定

| フィールド | 型 | デフォルト値 | 説明 |
|------|------|--------|------|
| `[i18n].lang` | String | "en" | 言語 |
| `[i18n].fallback` | String | "en" | フォールバック言語 |
| `[repl].history-size` | Number | 1000 | 履歴条数 |
| `[repl].history-file` | Path | ~ | 履歴ファイル |
| `[repl].prompt` | String | "yx> " | プロンプト |
| `[repl].colors` | Boolean | true | 構文ハイライト |
| `[repl].auto-imports` | [String] | [] | 自動インポート |
| `[fmt].line-width` | Number | 120 | 行幅 |
| `[fmt].indent-width` | Number | 4 | インデント |
| `[fmt].use-tabs` | Boolean | false | Tab インデント |
| `[fmt].single-quote` | Boolean | false | 単一引用符 |
| `[lint].rules` | [String] | ["recommended"] | ルールセット |
| `[lint].strict` | Boolean | false | 厳格モード |
| `[test].report` | String | "console" | テストレポート |
| `[build].output` | String | "dist/" | 出力ディレクトリ |

### コマンドラインと環境変数による上書き

```bash
# コマンドラインによる上書き
yaoxiang run main.yx --lang zh
yaoxiang fmt --config-indent-width=2

# 環境変数
export YAOXIANG_LANG=zh
export YAOXIANG_FMT_INDENT_WIDTH=2
```

**優先度**：`コマンドライン > 環境変数 > 設定ファイル`

### yaoxiang config コマンド

CLIコマンドで設定を管理する：

```bash
# ユーザーレベル設定を初期化（デフォルトオプションで生成）
yaoxiang config init

# ユーザーレベル設定を編集（エディタを開く）
yaoxiang config edit

# 現在の設定を表示（マージ後の有効な設定）
yaoxiang config show

# 設定ソースを表示
yaoxiang config show --source

# デフォルト設定にリセット
yaoxiang config reset
```

**初回実行**：ユーザーが初めて `yaoxiang` コマンドを実行する際、ユーザーレベル設定が存在するかを自動的に検出する。存在しない場合、デフォルトオプションで自動的に生成する。

**設定ファイルの位置**：
- プロジェクトレベル：`./yaoxiang.toml`（プロジェクトルートディレクトリ）
- ユーザーレベル：`~/.config/yaoxiang/config.toml`

### 設定マージセマンティクス

異なるレイヤーの設定は以下のルールでマージされる：

| 型 | 戦略 | 説明 |
|------|------|------|
| スカラー (String/Number/Boolean) | 置換 | プロジェクトレベルがユーザーレベルを上書き |
| 配列 (Array) | 置換 | プロジェクトレベルがユーザーレベルを完全に置換 |
| オブジェクト (Object) | 深いマージ | フィールドごとにマージし、未定義のフィールドは下位層から継承 |

**例 - オブジェクトの深いマージ**：
```toml
# ユーザーレベル
[lint]
rules = ["recommended"]
severity = "warn"

# プロジェクトレベル
[lint]
strict = true

# マージ結果
[lint]
rules = ["recommended"]    # ユーザーレベルから
severity = "warn"          # ユーザーレベルから
strict = true              # プロジェクトレベルから
```

### 後方互換性

- ✅ 既存の非設定ファイルモードもサポート継続（全コンポーネントが組み込みデフォルト値を使用）
- ✅ 新規設定項目はすべて合理的なデフォルト値を持つ
- ✅ ユーザーが初めてコマンドを実行する際、自動的にデフォルトオプションで設定を生成
- ✅ 設定解析失敗時、フレンドリーなエラーを表示し、具体的な行番号とエラー原因を提示

## トレードオフ

###  長所

- 統一された設定インフラで、重複コードを削減
- ユーザーの偏好がプロジェクト間で一貫
- LSP/REPL/コンパイラが同じ設定を共有
- 漸進的な設定で、必要に応じて宣言

### 短所

- 設定項目が多く、学習コストがやや増加
- 統一された設定パーサーが必要

## 代替案

| 案 | なぜ選択しなかったか |
|------|-----------|
| 各コンポーネントが独立した設定 | 重複コード、用户体验が分裂 |
| コマンドライン引数のみサポート | ユーザーの偏好を永続化できない |
| 環境変数のみサポート | プロジェクト設定がバージョン管理しにくい |

## 実装戦略

### フェーズ分け

| フェーズ | 内容 |
|------|------|
| **Phase 1** | 基本的な設定パーサー、tomlサポート、プロジェクトレベル設定、`yaoxiang config init` |
| **Phase 2** | ユーザーレベル設定、設定マージロジック、`yaoxiang config edit/show` |
| **Phase 3** | コマンドライン/環境変数による上書き、`platform` プラットフォーム制約、`[tool.*]` 拡張 |

### 依存関係

- RFC-014 パッケージ管理システムに依存

### リスク

| リスク | 軽減措施 |
|------|----------|
| 設定項目过多 | 合理的なデフォルト値を提供し、ユーザーに負担をかけない |
| パーサーが複雑 | 既存のtomlライブラリを使用 |

##  未解決問題

- [x] `features` 条件付きコンパイル構文？ → ** отдельное RFC に移動**、RFC-011  型システムに依存
- [x] `workspace` ワークスペース設計？ → ** отдельное RFC に移動**、複雑度が高く、独立した設計が必要

### 既に受領済みの機能（第三フェーズ）

#### `platform` プラットフォーム制約

> **注意**：以下の構文は `yaoxiang.toml` **設定ファイル** 用であり、YaoXiang ソースコード (`.yx` ファイル) の構文ではありません。ユーザーはコード内で `cfg(...)` 構文を書く必要はありません。

ターゲットOS/アーキテクチャに基づくプラットフォーム固有の設定をサポート：

```toml
# yaoxiang.toml（設定ファイル）

[target.'cfg(windows)'.build]
output = "dist/win32"

[target.'cfg(unix)'.build]
output = "dist/unix"

[target.'cfg(target_arch = "x86_64")'.build]
rustflags = ["-C target-cpu=native"]
```

**構文**：`[target.'<条件>'.<設定セクション>]`

**説明**：
- この構文は `yaoxiang.toml` 設定ファイルにのみ出現
- ビルド時に `--target` パラメータに基づいて対応する設定を選択
- ユーザーは `.yx` ソースコードで `cfg(...)` 構文を**書かず**、**書くべきではない**

**サポートされる条件**：
- `cfg(os = "windows")` - Windows システム
- `cfg(os = "linux")` - Linux システム
- `cfg(os = "macos")` - macOS システム
- `cfg(target_arch = "x86_64")` - 64ビット x86 アーキテクチャ
- `cfg(target_arch = "aarch64")` - ARM 64ビットアーキテクチャ

#### `[tool.*]` サードパーティツール設定拡張

サードパーティツールが `[tool.<名前>]` 下に設定を保存することを許可：

```toml
[tool.eslint]
extension = ["yx", "yxp"]
ignore = ["node_modules/", "dist/"]

[tool.prettier]
semi = false
singleQuote = true
```

**動作**：
- YaoXiang は未知の `[tool.*]` セクションを無視するが、設定ファイルには保持
- サードパーティツールは `yaoxiang tool run <名前>` 経由または直接アクセスで統合可能
- ツール固有の設定は検証されない

---

## 参考文献

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)