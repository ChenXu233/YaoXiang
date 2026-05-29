---
title: RFC-014：パッケージ管理システム設計
---

# RFC-014: パッケージ管理システム設計

> **ステータス**: 承認済み
> **著者**: 晨煦
> **作成日**: 2026-02-12
> **最終更新日**: 2026-02-14

## 概要

YaoXiang 言語のパッケージ管理システムを設計する。セマンティックバージョニング、GitHub ローカル依存関係と GitHub 依存関係、统一インポート構文、`yaoxiang.toml` 設定ファイル、`yaoxiang.lock` ロックファイルをサポートする。

## 動機

### この機能/変更がなぜ必要か？

パッケージ管理は сучасного プログラミング言語エコシステムの基盤である。現在の YaoXiang 言語には以下が不足している：

- 依存関係宣言メカニズム
- バージョンマネジメント機能
- 標準的な配布チャネル

### 現在の問題

```
my-project/
├── src/
│   └── main.yx          # コードが他のモジュールに依存
├── lib/                  # 手動でコピーされたモジュール
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
│              Source Trait                   │ ← 拡張可能なソース
├─────────────┬─────────────┬─────────────────┤
│   Local     │    Git      │   Registry     │
│   (ローカル) │  (GitHub)   │   (予約済み)    │
└─────────────┴─────────────┴─────────────────┘
```

**拡張メカニズム**：新しい Source 型を追加するには trait を実装するだけでよく、解決エンジンを修正する必要はない。

### 示例

```bash
# 1. プロジェクト作成
yaoxiang init my-project

# 2. yaoxiang.toml を編集して依存関係を追加
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. 依存関係インストール
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
description = "簡単な説明"

[dependencies]
foo = "1.2.3"           # 正確なバージョン
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
// 依存関係のソース（拡張可能）
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> }, // 予約済み
}

// 依存関係宣言
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
}

// 解決済み依存関係
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
}
```

### CLI コマンド設計

統合型アプローチを採用し、コンパイラ、パッケージマネージャー、REPL を単一の CLI ツールに統合する：

#### 単一ファイルモード vs プロジェクトモード

| コマンド | 単一ファイル | プロジェクトモード | 説明 |
|------|--------|---------|------|
| `yaoxiang run <file>` | ✅ | ✅ | ファイル/プロジェクトエントリを実行 |
| `yaoxiang build` | ❌ | ✅ | プロジェクトをビルド |
| `yaoxiang build <file>` | ✅ | ✅ | 単一ファイルをビルド |
| `yaoxiang init <name>` | ❌ | ✅ | プロジェクトを作成 |
| `yaoxiang add <dep>` | ❌ | ✅ | 依存関係を追加 |
| `yaoxiang update` | ❌ | ✅ | 依存関係を更新 |
| `yaoxiang fmt` | ✅ | ✅ | フォーマット |
| `yaoxiang check` | ✅ | ✅ | 型チェック |
| `yaoxiang` (引数なし) | ✅ | ✅ | 直接 REPL に入る |

#### コマンド詳細

| コマンド | 機能 | 例 |
|------|------|------|
| `yaoxiang` | 直接 REPL に入る | `yaoxiang` |
| `yaoxiang run <file>` | 単一ファイル/プロジェクトを実行 | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | 新規プロジェクト作成 | `yaoxiang init my-app` |
| `yaoxiang build` | プロジェクトをビルド | `yaoxiang build` |
| `yaoxiang build <file>` | 単一ファイルをビルド | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | 依存関係を追加 | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | 開発依存関係を追加 | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | 依存関係を削除 | `yaoxiang rm foo` |
| `yaoxiang update` | すべての依存関係を更新 | `yaoxiang update` |
| `yaoxiang update foo` | 指定した依存関係を更新 | `yaoxiang update foo` |
| `yaoxiang install` | すべての依存関係をインストール | `yaoxiang install` |
| `yaoxiang list` | 依存関係をリスト表示 | `yaoxiang list` |
| `yaoxiang outdated` | 古くなった依存関係をチェック | `yaoxiang outdated` |
| `yaoxiang fmt` | コードをフォーマット | `yaoxiang fmt` |
| `yaoxiang check` | 型チェック | `yaoxiang check` |
| `yaoxiang clean` | ビルド成果物をクリーン | `yaoxiang clean` |
| `yaoxiang task <name>` | カスタムタスクを実行 | `yaoxiang task lint` |

#### コマンド制約の説明

```bash
# 単一ファイルモード: yaoxiang.toml 不要
yaoxiang run hello.yx   # ✅ 正常に動作
yaoxiang add foo        # ❌ エラー: プロジェクトディレクトリではない

# プロジェクトモード: yaoxiang.toml 必要
cd my-project
yaoxiang run main.yx    # ✅ エントリファイルを実行
yaoxiang build          # ✅ プロジェクトをビルド
yaoxiang add foo        # ✅ 依存関係を追加
```

### 後方互換性

- ✅ 既存の `use` 構文は完全に維持
- ✅ 既存のモジュール解決ロジックは変更なし
- ✅ 新規の .yaoxiang/vendor ディレクトリは既存プロジェクトに影響しない

## トレードオフ

### メリット

- インポート構文が統一され、ユーザーが依存関係のソースを意識する必要がない
- 決定論的ビルド、ロックファイルによりビルドの一貫性を保証
- オフラインサポート、ローカルにダウンロード後はオフライン開発可能
- Source trait により将来の拡張が容易

### デメリット

- 追加のストレージ空間が必要（.yaoxiang/vendor ディレクトリ）
- バージョン衝突はユーザーが手動で解決する必要がある

## 代替案

| 方案 | 選択しなかった理由 |
|------|-----------|
| リアルタイム GitHub アクセス | セキュリティとキャッシュ再利用の保証が難しい |
| グローバルキャッシュ ($HOME/.yaoxiang) | 分離性が悪く、バージョン衝突が複雑 |
| レジストリの فقط サポート | GitHub は現在の的主流コードホスティングプラットフォーム |

## 実装戦略

### 段階的フェーズ

| 段階 | 内容 |
|------|------|
| **Phase 1** | toml 解析、ローカル依存関係、ロック生成、基础アルゴリズム |
| **Phase 2** | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール |
| **将来拡張** | Registry ソース、ワークスペース、完全性検証、依存関係オーバーライド |

### 依存関係

- 先行依存なし
- `ModuleGraph`（`middle/passes/module/`）との統合が必要

### リスク

| リスク | 軽減措施 |
|------|----------|
| 依存関係解決アルゴリズムが複雑 | まずシンプルバージョンを実装し、後で衝突検出を追加 |
| Git ダウンロードが不安定 | 再試行とキャッシュメカニズム |
| パフォーマンス問題 | 遅延ロード、インクリメンタル解決 |

## 開放問題

- [ ] `dev-dependencies` 条件付きコンパイル構文は？
- [ ] 完全性検証アルゴリズム（SHA-256 / BLAKE3）は？
- [ ] `excludes` で特定のファイルダウンロードを除外？

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)