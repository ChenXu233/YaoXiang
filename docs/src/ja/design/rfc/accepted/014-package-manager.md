---
title: "RFC-014：パッケージ管理システム設計"
---

# RFC-014：パッケージ管理システム設計

> **ステータス**: 承認済み
> **著者**: 晨煦
> **作成日**: 2026-02-12
> **最終更新**: 2026-02-14

## 摘要

YaoXiang 言語のパッケージ管理システムを設計する。セマンティックバージョニング、ローカルと GitHub 依存関係、统一インポート構文、`yaoxiang.toml` 設定ファイル、`yaoxiang.lock` ロックファイルをサポートする。

## 動機

### なぜこの機能/変更が必要なのか？

パッケージ管理は сучасного プログラミング言語エコシステムの 基幹インフラストラクチャである。現在 YaoXiang 言語には以下が欠けている：
- 依存関係宣言メカニズム
- バージョン管理機能
- 標準配布渠道

### 現在の問題

```
my-project/
├── src/
│   └── main.yx          # コードが他のモジュールに依存している
├── lib/                  # 手動でコピーしたモジュール
│   ├── foo.yx
│   └── bar.yx
└── ???                   # 標準的な依存関係管理がない
```

## 提案

### コア設計

**階層型アーキテクチャ**：
```
┌─────────────────────────────────────────────┐
│           Resolution Engine                  │ ← 依存関係解決
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← 拡張可能なソース
├─────────────┬─────────────┬─────────────────┤
│   Local     │    Git      │   Registry     │
│   (ローカル)│  (GitHub)   │   (予約済み)    │
└─────────────┴─────────────┴─────────────────┘
```

**拡張メカニズム**：新しい Source 型を追加するには trait を実装するだけで、解決エンジンを修正する必要はない。

### 例

```bash
# 1. プロジェクト作成
yaoxiang init my-project

# 2. yaoxiang.toml を編集して依存関係を追加
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. 依存関係 설치
yaoxiang add foo

# 4. コードで使用
use foo;
use bar.baz;
```

### プロジェクト構造

```
my-project/
├── yaoxiang.toml        # パッケージ設定
├── yaoxiang.lock        # ロックファイル（自動生成）
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/              # ローカル依存関係
        ├── foo-1.2.3/
        └── bar-0.5.0/
```

## 詳細設計

### 設定ファイル形式

**yaoxiang.toml**：
```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"

[dependencies]
foo = "1.2.3"           # 正確バージョン
bar = "^1.0.0"          # 互換バージョン
baz = "~1.2.0"          # パッチバージョン
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"
```

**yaoxiang.lock**：
```toml
version = 1

[[package]]
name = "foo"
version = "1.2.3"
source = "git"
resolved = "https://github.com/user/foo?tag=v1.2.3"
integrity = "sha256-xxxx"
```

### モジュール解決順序

```
use foo.bar.baz;

検索順序:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
2. ./src/foo/bar/baz.yx           (ローカルモジュール)
3. $YXPATH/foo/bar/baz.yx         (グローバルパス予約済み)
4. $YXLIB/std/foo/bar/baz.yx      (標準ライブラリ)
```

### コアデータ構造

```rust
// 依存関係ソース（拡張可能）
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> }, // 予約済み
}

// 依存関係仕様
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
}

// 解決された依存関係
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
}
```

### CLI コマンド設計

統一方案を採用.Compiler、包管理器、REPL を単一の CLI ツールに統合する：

#### 单ファイルモード vs プロジェクトモード

| コマンド | 单ファイル | プロジェクトモード | 説明 |
|------|--------|---------|------|
| `yaoxiang run <file>` | ✅ | ✅ | ファイル/プロジェクト入口を実行 |
| `yaoxiang build` | ❌ | ✅ | プロジェクトを構築 |
| `yaoxiang build <file>` | ✅ | ✅ | 单ファイルを構築 |
| `yaoxiang init <name>` | ❌ | ✅ | プロジェクトを作成 |
| `yaoxiang add <dep>` | ❌ | ✅ | 依存関係を追加 |
| `yaoxiang update` | ❌ | ✅ | 依存関係を更新 |
| `yaoxiang fmt` | ✅ | ✅ | フォーマット |
| `yaoxiang check` | ✅ | ✅ | 型チェック |
| `yaoxiang` (引数なし) | ✅ | ✅ | REPL に直接入る |

#### コマンド詳細

| コマンド | 機能 | 例 |
|------|------|------|
| `yaoxiang` | REPL に直接入る | `yaoxiang` |
| `yaoxiang run <file>` | 单ファイル/プロジェクトを実行 | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | 新規プロジェクトを作成 | `yaoxiang init my-app` |
| `yaoxiang build` | プロジェクトを構築 | `yaoxiang build` |
| `yaoxiang build <file>` | 单ファイルを構築 | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | 依存関係を追加 | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | 開発用依存関係を追加 | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | 依存関係を削除 | `yaoxiang rm foo` |
| `yaoxiang update` | 全依存関係を更新 | `yaoxiang update` |
| `yaoxiang update foo` | 指定依存関係を更新 | `yaoxiang update foo` |
| `yaoxiang install` | 全依存関係をインストール | `yaoxiang install` |
| `yaoxiang list` | 依存関係を一覧表示 | `yaoxiang list` |
| `yaoxiang outdated` | 古い依存関係をチェック | `yaoxiang outdated` |
| `yaoxiang fmt` | コードをフォーマット | `yaoxiang fmt` |
| `yaoxiang check` | 型チェック | `yaoxiang check` |
| `yaoxiang clean` | ビルド成果物をクリーン | `yaoxiang clean` |
| `yaoxiang task <name>` | カスタムタスクを実行 | `yaoxiang task lint` |

#### コマンド制約の説明

```bash
# 单ファイルモード：yaoxiang.toml は不要
yaoxiang run hello.yx   # ✅ 正常に動作
yaoxiang add foo        # ❌ エラー：プロジェクトディレクトリではない

# プロジェクトモード：yaoxiang.toml が必要
cd my-project
yaoxiang run main.yx    # ✅ 入口ファイルを実行
yaoxiang build          # ✅ プロジェクトを構築
yaoxiang add foo        # ✅ 依存関係を追加
```

### 後方互換性

- ✅ 既存の `use` 構文は完全に維持
- ✅ 既存のモジュール解決ロジックは変更なし
- ✅ 新規の .yaoxiang/vendor ディレクトリは既存プロジェクトに影響しない

## 权衡

### メリット

- インポート構文が統一され、ユーザーが依存関係のソースを気にする必要がない
- 确定性の構築、ロックファイルがビルドの一貫性を保証
- オフラインサポート、ローカルにダウンロード后可离线開発
- Source trait により今後の拡張が容易

### デメリット

- 追加のストレージ空間が必要（.yaoxiang/vendor ディレクトリ）
- バージョンの競合はユーザーが手動で解決する必要がある

## 代替方案

| 方案 | 選擇しなかった理由 |
|------|-----------|
| リアルタイム GitHub アクセス | セキュリティとキャッシュ再利用の保証が困難 |
| グローバルキャッシュ ($HOME/.yaoxiang) | 分離性が悪く、バージョン競合が複雑 |
| レジストリの的直接サポートのみ | GitHub は現在の主流コード托管プラットフォーム |

## 実装策略

### 段階分け

| 段階 | 内容 |
|------|------|
| **Phase 1** | toml 解析、ローカル依存関係、ロック生成、基础アルゴリズム |
| **Phase 2** | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール |
| **将来の拡張** | Registry ソース、ワークスペース、完全性チェック、依存関係オーバーライド |

### 依存関係

- 前置依存関係なし
- `ModuleGraph`（`middle/passes/module/`）との統合が必要

### リスク

| リスク | 軽減措置 |
|------|----------|
| 依存関係解決アルゴリズムが複雑 | 先に简单バージョンを実装し、後で競合検出を追加 |
| Git ダウンロードが不安定 | 再試行とキャッシュメカニズム |
| パフォーマンス問題 | 遅延ロード、インクリメンタル解決 |

## 開放問題

- [ ] `dev-dependencies` 条件付きコンパイル構文？
- [ ] 完全性チェックアルゴリズム（SHA-256 / BLAKE3）？
- [ ] `excludes` 特定ファイルのダウンロード除外？

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)