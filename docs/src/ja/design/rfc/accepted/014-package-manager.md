---
title: "RFC-014: パッケージ管理システム設計"
status: "承認済み"
author: "晨煦"
created: "2026-02-12"
updated: "2026-06-11"
group: "rfc-014"
issue: "#88"
impl: "48%"
impl_status: "部分的"
---

# RFC-014: パッケージ管理システム設計（総綱）

> **サブ RFC：**
> - [RFC-014a: Registry プロトコル仕様](../draft/014a-registry-protocol.md)
> - [RFC-014b: ビルドシステムとバイナリ配布](../draft/014b-build-system.md)
> - [RFC-014c: ワークスペースサポート](../draft/014c-workspace.md)

## 概要

YaoXiang 言語のパッケージ管理システムを設計する。セマンティックバージョニング、ローカルと GitHub への依存関係、统一されたインポート構文、`yaoxiang.toml` 設定ファイル、`yaoxiang.lock` ロックファイルをサポートする。

## 動機

### なぜこの機能/変更が必要なのか？

パッケージ管理は現代のプログラミング言語エコシステムの基盤である。現在の YaoXiang 言語には以下がない：
- 依存関係宣言メカニズム
- バージョンマネジメント機能
- 標準配布チャネル

### 現在の問題

```
my-project/
├── src/
│   └── main.yx          # コードが他のモジュールに依存している
├── lib/                  # 手動でコピーされたモジュール
│   ├── foo.yx
│   └── bar.yx
└── ???                   # 標準的な依存関係管理がない
```

## 提案

### コア設計

**階層化アーキテクチャ**：
```
┌─────────────────────────────────────────────┐
│           Resolution Engine                  │ ← 依存関係解決
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← 拡張可能なソース
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│  (ローカル)  │  (VCS)   │  (オープン)  │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**拡張メカニズム**：新しい Source 型は trait を実装するだけでよく、解決エンジンを変更する必要はない。

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
license = "MIT"
authors = ["Your Name <you@example.com>"]
repository = "https://github.com/you/my-package"
keywords = ["cli", "utility"]

[dependencies]
foo = "1.2.3"           # 正確バージョン
bar = "^1.0.0"          # 互換バージョン
baz = "~1.2.0"          # パッチバージョン
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
strategy = "none"       # none | cargo | cmake | custom

[binaries]
"linux-x86_64" = { url = "...", sha256 = "..." }

[workspace.members]     # ワークスペースルのみ
core = "packages/core/yaoxiang.toml"
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
2. ./src/foo/bar/baz.yx                     (ローカルモジュール)
3. ~/.yaoxiang/cache/foo/<ver>/src/foo/bar/baz.yx  (グローバルキャッシュ)
4. $YXPATH/foo/bar/baz.yx                   (グローバルパス、リザーブ)
5. $YXLIB/std/foo/bar/baz.yx                (標準ライブラリ)
```

### コアデータ構造

```rust
// 依存関係ソース（拡張可能）
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> },
    GitHub { owner: String, repo: String, ref_: GitRef },  // GitHub ネイティブ
}

enum GitRef {
    Tag(String),
    Branch(String),
    Rev(String),
    DefaultBranch,
}

// 依存関係宣言
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
    Workspace { member: String },  // ワークスペースメンバー参照
}

// 解決された依存関係
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
    checksum: Option<String>,  // SHA-256
}

// ビルド戦略
enum BuildStrategy {
    None,          // 純粋な .yx パッケージ
    Cargo,         // cargo build 呼び出し
    Cmake,         // cmake 呼び出し
    Custom,        // build.yx スクリプト実行
    Precompiled,   // 事前コンパイル済みを使用
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
| `yaoxiang init <name>` | ❌ | ✅ | プロジェクト作成 |
| `yaoxiang add <dep>` | ❌ | ✅ | 依存関係追加 |
| `yaoxiang update` | ❌ | ✅ | 依存関係更新 |
| `yaoxiang fmt` | ✅ | ✅ | フォーマット |
| `yaoxiang check` | ✅ | ✅ | 型チェック |
| `yaoxiang` (引数なし) | ✅ | ✅ | REPL に直接入る |

#### コマンド詳細

| コマンド | 機能 | 例 |
|------|------|------|
| `yaoxiang` | REPL に直接入る | `yaoxiang` |
| `yaoxiang run <file>` | 単一ファイル/プロジェクトを実行 | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | 新規プロジェクト作成 | `yaoxiang init my-app` |
| `yaoxiang build` | プロジェクトをビルド | `yaoxiang build` |
| `yaoxiang build <file>` | 単一ファイルをビルド | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | 依存関係追加 | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | 開発依存関係追加 | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | 依存関係削除 | `yaoxiang rm foo` |
| `yaoxiang update` | 全依存関係更新 | `yaoxiang update` |
| `yaoxiang update foo` | 指定依存関係更新 | `yaoxiang update foo` |
| `yaoxiang install` | 全依存関係インストール | `yaoxiang install` |
| `yaoxiang list` | 依存関係一覧 | `yaoxiang list` |
| `yaoxiang outdated` | 古い依存関係チェック | `yaoxiang outdated` |
| `yaoxiang fmt` | コードフォーマット | `yaoxiang fmt` |
| `yaoxiang check` | 型チェック | `yaoxiang check` |
| `yaoxiang clean` | ビルド成果物クリーン | `yaoxiang clean` |
| `yaoxiang task <name>` | カスタムタスク実行 | `yaoxiang task lint` |
| `yaoxiang publish` | Registry にパッケージ公開 | `yaoxiang publish` |
| `yaoxiang publish --github` | 公開して GitHub Release 作成 | `yaoxiang publish --github` |
| `yaoxiang yank <pkg>@<ver>` | 公開バージョン削除（不可逆） | `yaoxiang yank foo@1.2.3` |
| `yaoxiang login --registry <url>` | Registry 認証 | `yaoxiang login --registry https://reg.example.com` |
| `yaoxiang login --github` | GitHub 認証 | `yaoxiang login --github` |
| `yaoxiang logout --registry <url>` | ログアウト | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean` | グローバルキャッシュクリーン | `yaoxiang cache clean` |
| `yaoxiang workspace <cmd>` | ワークスペース操作 | `yaoxiang workspace list` |

#### コマンド制約説明

```bash
# 単一ファイルモード：yaoxiang.toml 不要
yaoxiang run hello.yx   # ✅ 正常に動作
yaoxiang add foo        # ❌ エラー：プロジェクトディレクトリではない

# プロジェクトモード：yaoxiang.toml が必要
cd my-project
yaoxiang run main.yx    # ✅ エントリファイルを実行
yaoxiang build          # ✅ プロジェクトをビルド
yaoxiang add foo        # ✅ 依存関係追加
```

### 後方互換性

- ✅ 既存の `use` 構文は完全維持
- ✅ 既存のモジュール解決ロジックは変更なし
- ✅ 新規の .yaoxiang/vendor ディレクトリは既存プロジェクトに影響なし

### グローバルキャッシュ

ダウンロードした全依存関係を `~/.yaoxiang/cache/` にキャッシュし、プロジェクトの vendor ディレクトリはキャッシュからコピーする。

```
~/.yaoxiang/
├── cache/
│   ├── registry/
│   │   └── foo-1.2.3/
│   ├── git/
│   │   └── github.com-user-bar-abc123/
│   └── binaries/
│       └── foo-1.2.3-linux-x86_64.tar.gz
├── credentials.toml
└── config.toml
```

```toml
# ~/.yaoxiang/config.toml
[cache]
dir = "~/.yaoxiang/cache"
max_size = "2GB"
ttl = "30d"
```

キャッシュ無効ルール：
- Registry パッケージ：バージョン番号は不変で、失效しない
- Git 依存関係：tag/rev でキャッシュ、tag が変更されなければ失效しない
- `yaoxiang cache clean` で手動クリーン

### 認証

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

- 環境変数が優先：`$YX_GITHUB_TOKEN`、`$YX_REGISTRY_TOKEN`
- Token は絶対に `yaoxiang.toml` や `yaoxiang.lock` に書き込まない
- ファイルパーミッション 600

### yank セマンティクス

`yaoxiang yank foo@1.2.3` は**削除 + バージョン番号の固定**を実行：

- パッケージは完全に削除され不可逆
- バージョン番号は永久に占有され、同じバージョン番号を再公開できない
- 既存の lockfile がそのバージョンを参照しているプロジェクトはエラーになり、アップグレードが必要
- **セキュリティ目的**：npm 型のサプライチェーン攻撃を防止（攻撃者が削除されたバージョン番号を乗っ取って悪意のあるコードを注入）

### Registry プロトコル

詳細は [RFC-014a: Registry プロトコル仕様](../draft/014a-registry-protocol.md) を参照。

コア設計：オープンプrotocol + adapter 層。公式 Registry を主とし、GitHub Release/main ブランチを補完、カスタム Registry をサポート。

### ビルドシステム

詳細は [RFC-014b: ビルドシステムとバイナリ配布](../draft/014b-build-system.md) を参照。

コア設計：宣言的な `[build]` 設定、事前コンパイル優先/ソースコード fallback、cargo/cmake/custom 戦略をサポート。

### ワークスペース

詳細は [RFC-014c: ワークスペースサポート](../draft/014c-workspace.md) を参照。

コア設計：辞書形式の members 宣言、共有 lockfile、パス依存、Cargo workspace 統合。

## トレードオフ

### 利点

- 統一されたインポート構文で、ユーザーが依存関係のソースを意識する必要がない
- 決定論的ビルドで、lock ファイルがビルドの一貫性を保証
- オフラインサポートで、ダウンロード後はオフライン開発可能
- Source trait により後からの拡張が容易

### 欠点

- 追加のストレージ空間が必要（.yaoxiang/vendor ディレクトリ）
- バージョンの競合はユーザーが手動で解決する必要がある

## 代替案

| 案 | 選択しなかった理由 |
|------|-----------|
| リアルタイム GitHub アクセス | セキュリティとキャッシュ再利用の保証が困難 |
| グローバルキャッシュ ($HOME/.yaoxiang) | 分離性が悪く、バージョン競合が複雑 |
| Registry のみサポート | GitHub は現在の主要なコードホスティングプラットフォーム |

## 実装戦略

### フェーズ分け

| フェーズ | 内容 | ステータス |
|------|------|------|
| **Phase 1** | toml 解析、ローカル依存関係、lock 生成、基本アルゴリズム | ✅ 完了 |
| **Phase 2** | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール | ✅ 完了 |
| **Phase 3** | グローバルキャッシュ、semver crate 置換、CLI 完善 | 未開始 |
| **Phase 3.5** | Source trait async 化、async-trait 統合 | 未開始 |
| **Phase 4** | Registry プロトコル、publish、auth（RFC-014a） | 未開始 |
| **Phase 5** | ビルドシステム、事前コンパイルバイナリ（RFC-014b） | 未開始 |
| **Phase 6** | ワークスペースサポート（RFC-014c） | 未開始 |

### 依存関係

- 前置依存関係なし
- `ModuleGraph`（`middle/passes/module/`）との統合が必要

### リスク

| リスク | 軽減措施 |
|------|----------|
| 依存関係解決アルゴリズムが複雑 | まずシンプル版を実装し、後から競合検出を追加 |
| Git ダウンロードが不安定 | リトライとキャッシュメカニズム |
| パフォーマンス問題 | 遅延ロード、インクリメンタル解決 |

## 未解決問題

- [x] `dev-dependencies` 条件付きコンパイル構文？→ RFC-014b ビルドシステムで統一対応
- [x] 完全性検証アルゴリズム（SHA-256 / BLAKE3）？→ SHA-256
- [ ] 特定ファイルのダウンロード除外 `excludes`？
- [ ] パッケージ命名規則（namespace サポート是否、如 `@org/pkg`）？
- [ ] Registry API バージョン化管理戦略？

---

## 依存関係（Cargo.toml に追加が必要）

| 用途 | crate | 説明 |
|------|-------|------|
| セマンティックバージョニング | `semver` | 手書きパーサー置換 |
| HTTP クライアント | `reqwest` | Registry 通信 |
| SHA-256 | `sha2` | 完全性検証 |
| 圧縮 | `flate2` + `tar` | パッケージ形式処理 |

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)