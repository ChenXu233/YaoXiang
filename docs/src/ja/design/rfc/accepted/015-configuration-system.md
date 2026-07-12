---
title: "RFC-015: YaoXiang 設定システム設計"
status: "承認済み"
author: "晨煦"
created: "2026-02-12"
updated: "2026-02-15"
issue: "#133"
---

# RFC-015: YaoXiang 設定システム設計

> **承認日**: 2026-02-15

> **前提 RFC**: [RFC-014: パッケージ管理システム設計](014-package-manager.md)

## 概要

YaoXiang 言語の統一設定システムを設計し、ユーザーレベルとプロジェクトレベルの2つの階層をサポートし、パッケージマネージャー、コンパイラ、REPL、LSP などのコンポーネントに共有される設定インフラを提供する。

## 動機

### なぜこの機能/変更が必要なのか？

YaoXiang ツールチェーンには複数のコンポーネントが含まれる：
- パッケージマネージャー（依存関係設定を読み取る）
- コンパイラフロントエンド（i18n 設定を読み取る）
- REPL（対話設定を読み取る）
- LSP（fmt/lint/test 設定を読み取る）
- ビルドシステム（ビルド設定を読み取る）

各コンポーネントは統一された設定インフラを必要とする。

### 現在の問題

- 各コンポーネントの設定が分散しており、統一された規範がない
- ユーザーが環境設定を一元管理できない
- プロジェクト設定とユーザー設定の間に明確な階層がない

## 提案

### 中核設計

**階層アーキテクチャ**：
```
設定の優先度（高 → 低）：
┌─────────────────────────────────────────────┐
│ 1. プロジェクトレベル yaoxiang.toml           │ ← プロジェクトチームが管理
├─────────────────────────────────────────────┤
│ 2. ユーザーレベル ~/.config/yaoxiang/config.toml │ ← ユーザー設定
├─────────────────────────────────────────────┤
│ 3. コンパイラのデフォルト値                  │ ← 適切な初期値
└─────────────────────────────────────────────┘
```

**ルール**：上層が下層を上書きし、未設定のオプションは下層にフォールバックする。

### 設定階層の制限

| 設定セクション | ユーザーレベル | プロジェクトレベル | 消費側 |
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

### 両方で利用可能な設定

| フィールド | 型 | デフォルト値 | 説明 |
|------|------|--------|------|
| `[i18n].lang` | String | "en" | 言語 |
| `[i18n].fallback` | String | "en" | フォールバック言語 |
| `[repl].history-size` | Number | 1000 | 履歴件数 |
| `[repl].history-file` | Path | ~ | 履歴ファイル |
| `[repl].prompt` | String | "yx> " | プロンプト |
| `[repl].colors` | Boolean | true | シンタックスハイライト |
| `[repl].auto-imports` | [String] | [] | 自動インポート |
| `[fmt].line-width` | Number | 120 | 行幅 |
| `[fmt].indent-width` | Number | 4 | インデント |
| `[fmt].use-tabs` | Boolean | false | タブインデント |
| `[fmt].single-quote` | Boolean | false | シングルクォート |
| `[lint].rules` | [String] | ["recommended"] | ルールセット |
| `[lint].strict` | Boolean | false | 厳密モード |
| `[test].report` | String | "console" | テストレポート |
| `[build].output` | String | "dist/" | 出力ディレクトリ |

### コマンドラインと環境変数による上書き

```bash
# コマンドライン上書き
yaoxiang run main.yx --lang zh
yaoxiang fmt --config-indent-width=2

# 環境変数
export YAOXIANG_LANG=zh
export YAOXIANG_FMT_INDENT_WIDTH=2
```

**優先度**：`コマンドライン > 環境変数 > 設定ファイル`

### yaoxiang config コマンド

設定管理用の CLI コマンドを提供：

```bash
# ユーザーレベル設定を初期化（デフォルトオプションで生成）
yaoxiang config init

# ユーザーレベル設定を編集（エディタを開く）
yaoxiang config edit

# 現在の設定を表示（マージ後の有効な設定）
yaoxiang config show

# 設定の出所を表示
yaoxiang config show --source

# デフォルト設定にリセット
yaoxiang config reset
```

**初回実行**：ユーザーが初めて `yaoxiang` コマンドを実行したとき、ユーザーレベル設定が存在するかを自動検出する。存在しない場合は、デフォルトオプションで自動生成される。

**設定ファイルの場所**：
- プロジェクトレベル：`./yaoxiang.toml`（プロジェクトルートディレクトリ）
- ユーザーレベル：`~/.config/yaoxiang/config.toml`

### 設定マージのセマンティクス

異なる階層の設定は以下のルールに従ってマージされる：

| 型 | 戦略 | 説明 |
|------|------|------|
| スカラー (String/Number/Boolean) | 置換 | プロジェクトレベルがユーザーレベルを上書き |
| 配列 (Array) | 置換 | プロジェクトレベルがユーザーレベルを完全に置換 |
| オブジェクト (Object) | ディープマージ | フィールドごとにマージし、未定義フィールドは下層を継承 |

**例 - オブジェクトのディープマージ**：
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
strict = true             # プロジェクトレベルから
```

### 後方互換性

- ✅ 既存の設定ファイルなしモードを引き続きサポート（全コンポーネントが内蔵デフォルト値を使用）
- ✅ 新規設定項目にはすべて適切なデフォルト値がある
- ✅ ユーザーが初めてコマンドを実行したとき、デフォルトオプションで設定を自動生成
- ✅ 設定解析失敗時、わかりやすいエラーを表示し、具体的な行番号とエラー理由を示す

## トレードオフ

### 利点

- 統一された設定インフラにより、コードの重複を削減
- ユーザー設定がプロジェクト間で一貫している
- LSP/REPL/コンパイラが同じ設定を共有
- 漸進的な設定、必要に応じて宣言

### 欠点

- 設定項目が多く、学習コストがやや高い
- 統一された設定パーサーが必要

## 代替案

| 案 | 選択しなかった理由 |
|------|-----------|
| 各コンポーネントが独立した設定 | コードの重複、ユーザー体験の分断 |
| コマンドライン引数のみサポート | ユーザー設定を永続化できない |
| 環境変数のみサポート | プロジェクト設定のバージョン管理が困難 |

## 実装戦略

### フェーズ区分

| フェーズ | 内容 |
|------|------|
| **Phase 1** | 基本設定パーサー、toml サポート、プロジェクトレベル設定、`yaoxiang config init` |
| **Phase 2** | ユーザーレベル、設定マージロジック、`yaoxiang config edit/show` |
| **Phase 3** | コマンドライン/環境変数による上書き、`platform` プラットフォーム制約、`[tool.*]` 拡張 |

### 依存関係

- RFC-014 パッケージ管理システムに依存

### リスク

| リスク | 緩和策 |
|------|----------|
| 設定項目が多すぎる | 適切なデフォルト値を提供し、ユーザーに無感覚 |
| パーサーが複雑 | 既存の toml ライブラリを使用 |

## 未解決問題

- [x] `features` 条件コンパイル構文？ → **別の RFC に移す**、RFC-011 ジェネリクスシステムに依存
- [x] `workspace` ワークスペース設計？ → **別の RFC に移す**、複雑度が高く、独立した設計が必要

### 承認済み機能（第三フェーズ）

#### `platform` プラットフォーム制約

> **注意**：以下の構文は `yaoxiang.toml` **設定ファイル**用であり、YaoXiang ソースコード（`.yx` ファイル）内の構文ではない。ユーザーはコード内に `cfg(...)` を書く必要はない。

ターゲット OS/アーキテクチャに基づくプラットフォーム固有設定をサポート：

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
- この構文は `yaoxiang.toml` 設定ファイル内にのみ出現する
- ビルド時に `--target` パラメータに基づいて対応する設定を選択する
- ユーザーは `.yx` ソースコード内に `cfg(...)` 構文を**書く必要がない**、**書くべきでもない**

**サポートされる条件**：
- `cfg(os = "windows")` - Windows システム
- `cfg(os = "linux")` - Linux システム
- `cfg(os = "macos")` - macOS システム
- `cfg(target_arch = "x86_64")` - 64 ビット x86 アーキテクチャ
- `cfg(target_arch = "aarch64")` - ARM 64 ビットアーキテクチャ

#### `[tool.*]` サードパーティツール設定拡張

サードパーティツールが `[tool.<名前>]` の下に設定を保存することを許可する：

```toml
[tool.eslint]
extension = ["yx", "yxp"]
ignore = ["node_modules/", "dist/"]

[tool.prettier]
semi = false
singleQuote = true
```

**動作**：
- YaoXiang は未知の `[tool.*]` セクションを無視するが、設定ファイル内に保持する
- サードパーティツールは `yaoxiang tool run <名前>` で統合するか、直接アクセスできる
- ツール固有設定は検証されない

---

## 参考文献

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)