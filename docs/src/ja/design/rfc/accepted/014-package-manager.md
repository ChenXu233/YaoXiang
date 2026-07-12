```markdown
---
title: "RFC-014: パッケージ管理システムの設計"
status: "承認済み"
author: "晨煦"
created: "2026-02-12"
updated: "2026-06-11"
group: "rfc-014"  # 本 RFC はパッケージ管理システムの総綱、子 RFC：014a/014b/014c
issue: "#88"
impl: "48%"
impl_status: "partial"
---

# RFC-014: パッケージ管理システムの設計（総綱）

> **子 RFC：**
> - [RFC-014a: Registry プロトコル仕様](../draft/014a-registry-protocol.md)
> - [RFC-014b: ビルドシステムとバイナリ配布](../draft/014b-build-system.md)
> - [RFC-014c: ワークスペースサポート](../draft/014c-workspace.md)

## 概要

YaoXiang 言語のパッケージ管理システムを設計し、セマンティックバージョニング、ローカルと GitHub 依存関係、统一インポート構文、`yaoxiang.toml` 設定ファイル、`yaoxiang.lock` ロックファイルをサポートする。

## 動機

### なぜこの機能/変更が必要なのか？

パッケージ管理は現代プログラミング言語のエコシステムにおける基礎インフラである。現在の YaoXiang 言語には以下が欠けている：
- 依存関係宣言の仕組み
- バージョン管理能力
- 標準的な配布チャネル

### 現状の問題

```
my-project/
├── src/
│   └── main.yx          # コードは他のモジュールに依存
├── lib/                  # 手動でコピーされたモジュール
│   ├── foo.yx
│   └── bar.yx
└── ???                   # 標準的な依存管理がない
```

## 提案

### 中核設計

**階層型アーキテクチャ**：
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
│  (ローカル)  │  (VCS)   │  (公開)  │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**拡張メカニズム**：新しい Source 型の追加は trait を実装するだけでよく、解決エンジンの修正は不要。

### 例

```bash
# 1. プロジェクトの作成
yaoxiang init my-project

# 2. yaoxiang.toml を編集して依存関係を追加
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. 依存関係のインストール
yaoxiang add foo

# 4. コード内で使用
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
foo = "1.2.3"           # 厳密なバージョン
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

[workspace.members]     # ワークスペースルートのみ
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
4. $YXPATH/foo/bar/baz.yx                   (グローバルパス、予備)
5. $YXLIB/std/foo/bar/baz.yx                (標準ライブラリ)
```

### 中核データ構造

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

// 解決済みの依存関係
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
    Cargo,         // cargo build を呼び出す
    Cmake,         // cmake を呼び出す
    Custom,        // build.yx スクリプトを実行
    Precompiled,   // コンパイル済み成果物を直接使用
}
```

### CLI コマンド設計

统一方案を採用し、コンパイラ、パッケージマネージャ、REPL を単一 CLI ツールに統合する：

#### 単一ファイルモード vs プロジェクトモード

| コマンド | 単一ファイル | プロジェクトモード | 説明 |
|------|--------|---------|------|
| `yaoxiang run <file>` | ✅ | ✅ | ファイル/プロジェクトのエントリを実行 |
| `yaoxiang build` | ❌ | ✅ | プロジェクトをビルド |
| `yaoxiang build <file>` | ✅ | ✅ | 単一ファイルをビルド |
| `yaoxiang init <name>` | ❌ | ✅ | プロジェクトを作成 |
| `yaoxiang add <dep>` | ❌ | ✅ | 依存関係を追加 |
| `yaoxiang update` | ❌ | ✅ | 依存関係を更新 |
| `yaoxiang fmt` | ✅ | ✅ | フォーマット |
| `yaoxiang check` | ✅ | ✅ | 型検査 |
| `yaoxiang` (引数なし) | ✅ | ✅ | 直接 REPL に入る |

#### コマンド詳細

| コマンド | 機能 | 例 |
|------|------|------|
| `yaoxiang` | 直接 REPL に入る | `yaoxiang` |
| `yaoxiang run <file>` | 単一ファイル/プロジェクトを実行 | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | 新規プロジェクトを作成 | `yaoxiang init my-app` |
| `yaoxiang build` | プロジェクトをビルド | `yaoxiang build` |
| `yaoxiang build <file>` | 単一ファイルをビルド | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | 依存関係を追加 | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | 開発依存関係を追加 | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | 依存関係を削除 | `yaoxiang rm foo` |
| `yaoxiang update` | すべての依存関係を更新 | `yaoxiang update` |
| `yaoxiang update foo` | 指定した依存関係を更新 | `yaoxiang update foo` |
| `yaoxiang install` | すべての依存関係をインストール | `yaoxiang install` |
| `yaoxiang list` | 依存関係を表示 | `yaoxiang list` |
| `yaoxiang outdated` | 古い依存関係を確認 | `yaoxiang outdated` |
| `yaoxiang fmt` | コードをフォーマット | `yaoxiang fmt` |
| `yaoxiang check` | 型検査 | `yaoxiang check` |
| `yaoxiang clean` | ビルド成果物をクリア | `yaoxiang clean` |
| `yaoxiang task <name>` | カスタムタスクを実行 | `yaoxiang task lint` |
| `yaoxiang publish` | Registry にパッケージを公開 | `yaoxiang publish` |
| `yaoxiang publish --github` | 公開して GitHub Release を作成 | `yaoxiang publish --github` |
| `yaoxiang yank <pkg>@<ver>` | 公開済みバージョンを削除（復元不可） | `yaoxiang yank foo@1.2.3` |
| `yaoxiang login --registry <url>` | Registry 認証 | `yaoxiang login --registry https://reg.example.com` |
| `yaoxiang login --github` | GitHub 認証 | `yaoxiang login --github` |
| `yaoxiang logout --registry <url>` | ログアウト | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean` | グローバルキャッシュをクリア | `yaoxiang cache clean` |
| `yaoxiang workspace <cmd>` | ワークスペース操作 | `yaoxiang workspace list` |

#### コマンド制約の説明

```bash
# 単一ファイルモード：yaoxiang.toml は不要
yaoxiang run hello.yx   # ✅ 正常動作
yaoxiang add foo        # ❌ エラー：プロジェクトディレクトリではない

# プロジェクトモード：yaoxiang.toml が必要
cd my-project
yaoxiang run main.yx    # ✅ エントリファイルを実行
yaoxiang build          # ✅ プロジェクトをビルド
yaoxiang add foo        # ✅ 依存関係を追加
```

### 後方互換性

- ✅ 既存の `use` 構文は完全に保持
- ✅ 既存のモジュール解決ロジックは不変
- ✅ 新規 .yaoxiang/vendor ディレクトリは既存プロジェクトに影響しない

### グローバルキャッシュ

ダウンロードされたすべての依存関係は `~/.yaoxiang/cache/` にキャッシュされ、プロジェクトの vendor ディレクトリはキャッシュからコピーされる。

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

キャッシュ無効化ルール：
- Registry パッケージ：バージョン番号は不変、永久に無効化されない
- Git 依存関係：tag/rev ごとにキャッシュ、tag 不変なら無効化されない
- `yaoxiang cache clean` で手動クリア

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
- Token は決して `yaoxiang.toml` や `yaoxiang.lock` に書き込まれない
- ファイルパーミッション 600

### yank セマンティクス

`yaoxiang yank foo@1.2.3` は**削除 + バージョン番号の永久ロック**を実行する：

- パッケージは完全に削除され、復元不可
- バージョン番号は永久に占有され、同じバージョン番号を再公開できない
- 既にそのバージョンを参照する lockfile を持つプロジェクトはエラーになり、アップグレードが必要
- **安全上の目的**：npm 型のサプライチェーン攻撃を防ぐ（攻撃者が削除されたバージョン番号を先取りして悪意あるコードを注入することを防止）

### Registry プロトコル

詳細は [RFC-014a: Registry プロトコル仕様](../draft/014a-registry-protocol.md) を参照。

中核設計：オープンプロトコル + アダプタ層。公式 Registry が主、GitHub Release/main ブランチが従、カスタム Registry をサポート。

### ビルドシステム

詳細は [RFC-014b: ビルドシステムとバイナリ配布](../draft/014b-build-system.md) を参照。

中核設計：宣言的な `[build]` 設定、プリコンパイル優先/ソースコードフォールバック、cargo/cmake/custom 戦略をサポート。

### ワークスペース

詳細は [RFC-014c: ワークスペースサポート](../draft/014c-workspace.md) を参照。

中核設計：辞書形式の members 宣言、共有 lockfile、パス依存関係、Cargo workspace 統合。

## トレードオフ

### 利点

- 統一インポート構文により、ユーザーは依存ソースを気にする必要がない
- 決定論的ビルド、lock ファイルがビルドの一貫性を保証
- オフラインサポート、ローカルにダウンロード後はオフライン開発が可能
- Source trait により将来の拡張が容易

### 欠点

- 追加のストレージスペースが必要（.yaoxiang/vendor ディレクトリ）
- バージョン競合はユーザーが手動で解決する必要がある

## 代替案

| 案 | 選択しなかった理由 |
|------|-----------|
| リアルタイム GitHub アクセス | セキュリティとキャッシュ再利用の保証が困難 |
| グローバルキャッシュ ($HOME/.yaoxiang) | 分離性が低く、バーsジョン競合が複雑 |
| Registry のみサポート | GitHub は現在の主流コードホスティングプラットフォーム |

## 実装戦略

### フェーズ分け

| フェーズ | 内容 | 状態 |
|------|------|------|
| **Phase 1** | toml 解析、ローカル依存関係、lock 生成、基礎アルゴリズム | ✅ 完了 |
| **Phase 2** | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール | ✅ 完了 |
| **Phase 3** | グローバルキャッシュ、semver crate 置換、CLI 完成 | 未着手 |
| **Phase 3.5** | Source trait を async に変更、async-trait 統合 | 未着手 |
| **Phase 4** | Registry プロトコル、publish、auth（RFC-014a） | 未着手 |
| **Phase 5** | ビルドシステム、プリコンパイルバイナリ（RFC-014b） | 未着手 |
| **Phase 6** | ワークスペースサポート（RFC-014c） | 未着手 |

### 依存関係

- 先行依存なし
- `ModuleGraph`（`middle/passes/module/`）との統合が必要

### リスク

| リスク | 緩和策 |
|------|----------|
| 依存関係解決アルゴリズムが複雑 | まず簡単なバージョンを実装し、競合検出を後で追加 |
| Git ダウンロードが不安定 | リトライとキャッシュの仕組み |
| パフォーマンスの問題 | 遅延読み込み、増分解析 |

## オープンな問題

- [x] `dev-dependencies` の条件付きコンパイル構文？→ RFC-014b ビルドシステムで統一処理
- [x] 完全性検証アルゴリズム（SHA-256 / BLAKE3）？→ SHA-256
- [ ] `excludes` で特定ファイルのダウンロードを除外？
- [ ] パッケージ命名規則（namespace サポート、例：`@org/pkg`）？
- [ ] Registry API のバージョニング戦略？

---

## 依存関係（Cargo.toml に追加が必要）

| 用途 | crate | 説明 |
|------|-------|------|
| セマンティックバージョニング | `semver` | 手書きパーサーを置換 |
| HTTP クライアント | `reqwest` | Registry 通信 |
| SHA-256 | `sha2` | 完全性検証 |
| 圧縮 | `flate2` + `tar` | パッケージ形式処理 |

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
```