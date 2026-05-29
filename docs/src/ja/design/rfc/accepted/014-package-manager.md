```markdown
---
title: "RFC-014：パッケージ管理システム設計"
---

# RFC-014：パッケージ管理システム設計

> **状態**: 承認済み
> **著者**: 晨煦
> **作成日**: 2026-02-12
> **最終更新**: 2026-02-14

## 要約

YaoXiang 言語のパッケージ管理システムを設計する。セマンティックバージョニング、ローカルと GitHub への依存対応、统一されたインポート構文、`yaoxiang.toml` 設定ファイル、`yaoxiang.lock` ロックファイルをサポートする。

## 動機

### なぜこの機能/変更が必要なのか？

パッケージ管理は сучасного プログラミング言語エコシステムの基盤である。現在の YaoXiang 言語には以下が欠けている：
- 依存関係宣言メカニズム
- バージョニング機能
- 標準的な配布渠道

### 現在の問題

```
my-project/
├── src/
│   └── main.yx          # コードが他のモジュールに依存
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
│              Source Trait                   │ ← 拡張可能なソース
├─────────────┬─────────────┬─────────────────┤
│   Local     │    Git      │   Registry     │
│   (ローカル) │  (GitHub)   │   (レジストリ)  │
└─────────────┴─────────────┴─────────────────┘
```

**拡張メカニズム**：新しい Source 型を追加するには trait を実装するだけでよく、解決エンジンの変更は不要。

### 例

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
description = "A short description"

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

```yaoxiang
use foo.bar.baz;

検索順序:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
2. ./src/foo/bar/baz.yx           (ローカルモジュール)
3. $YXPATH/foo/bar/baz.yx         (グローバルパス、レジストリ)
4. $YXLIB/std/foo/bar/baz.yx      (標準ライブラリ)
```

### コアデータ構造

```rust
// 依存関係のソース（拡張可能）
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> }, // 将来拡張
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

統一されたアプローチを採用し、コンパイラ、パッケージマネージャー、REPL を単一の CLI ツールに統合する：

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
| `yaoxiang` (引数なし) | ✅ | ✅ | REPL に直接入る |

#### コマンド詳細

| コマンド | 機能 | 例 |
|------|------|------|
| `yaoxiang` | REPL に直接入る | `yaoxiang` |
| `yaoxiang run <file>` | 単一ファイル/プロジェクトを実行 | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | 新規プロジェクトを作成 | `yaoxiang init my-app` |
| `yaoxiang build` | プロジェクトをビルド | `yaoxiang build` |
| `yaoxiang build <file>` | 単一ファイルをビルド | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | 依存関係を追加 | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | 開発用依存関係を追加 | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | 依存関係を削除 | `yaoxiang rm foo` |
| `yaoxiang update` | すべての依存関係を更新 | `yaoxiang update` |
| `yaoxiang update foo` | 指定した依存関係を更新 | `yaoxiang update foo` |
| `yaoxiang install` | すべての依存関係をインストール | `yaoxiang install` |
| `yaoxiang list` | 依存関係を一覧表示 | `yaoxiang list` |
| `yaoxiang outdated` | 古い依存関係をチェック | `yaoxiang outdated` |
| `yaoxiang fmt` | コードをフォーマット | `yaoxiang fmt` |
| `yaoxiang check` | 型チェック | `yaoxiang check` |
| `yaoxiang clean` | ビルド成果物をクリーン | `yaoxiang clean` |
| `yaoxiang task <name>` | カスタムタスクを実行 | `yaoxiang task lint` |

#### コマンド制約の説明

```bash
# 単一ファイルモード：yaoxiang.toml は不要
yaoxiang run hello.yx   # ✅ 正常に動作
yaoxiang add foo        # ❌ エラー：プロジェクトディレクトリではない

# プロジェクトモード：yaoxiang.toml が必要
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

### 長所

- 統一されたインポート構文で、ユーザーが依存関係のソースを気にする必要がない
- 確定的なビルド：lock ファイルがビルドの一貫性を保証
- オフラインサポート：ローカルにダウンロードすればオフライン開発可能
- Source trait により後からの拡張が容易

### 短所

- 追加のストレージ空間が必要（.yaoxiang/vendor ディレクトリ）
- バージョン衝突はユーザーが手動で解決する必要がある

## 代替案

| 案 | なぜ選ばなかったか |
|------|-----------|
| リアルタイム GitHub アクセス | セキュリティとキャッシュ再利用の保証が難しい |
| グローバルキャッシュ ($HOME/.yaoxiang) | 分離性が悪く、バージョン衝突が複雑 |
| レジストリのみサポート | GitHub は現在の主要なコードホスティングプラットフォーム |

## 実装戦略

### フェーズ分け

| フェーズ | 内容 |
|------|------|
| **Phase 1** | toml 解析、ローカル依存関係、lock 生成、基本アルゴリズム |
| **Phase 2** | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール |
| **将来拡張** | Registry ソース、ワークスペース、整合性チェック、依存関係オーバーライド |

### 依存関係

- 先行依存なし
- `ModuleGraph`（`middle/passes/module/`）との統合が必要

### リスク

| リスク | 軽減策 |
|------|----------|
| 依存関係解決アルゴリズムが複雑 | まずシンプル版を実装し、後で衝突検出を追加 |
| Git ダウンロードが不安定 | 再試行とキャッシュメカニズム |
| パフォーマンス問題 | 遅延読み込み、インクリメンタル解析 |

## 未解決の問題

- [ ] `dev-dependencies` 条件付きコンパイル構文は？
- [ ] 整合性チェックアルゴリズム（SHA-256 / BLAKE3）は？
- [ ] `excludes` で特定のファイルをダウンロードから除外？

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
```