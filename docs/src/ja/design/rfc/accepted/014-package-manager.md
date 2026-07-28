---
title: 'RFC-014: パッケージ管理システム設計'
status: 'accepted'
author: '晨煦'
created: '2026-02-12'
updated: '2026-06-11'
group: 'rfc-014' # 本 RFC はパッケージ管理システムの総綱、サブ RFC：014a/014b/014c
issue: '#88'
impl: '48%'
impl_status: 'partial'
---

# RFC-014: パッケージ管理システム設計（総綱）

> **サブ RFC：**
>
> - [RFC-014a: Registry プロトコル仕様](../draft/014a-registry-protocol.md)
> - [RFC-014b: ビルドシステムとバイナリ配布](../draft/014b-build-system.md)
> - [RFC-014c: ワークスペースサポート](../draft/014c-workspace.md)

## 概要

YaoXiang 言語のパッケージ管理系统を設計する。セマンティックバージョニング、ローカル・GitHub 依存、统一インポート構文、`yaoxiang.toml` 設定ファイル、`yaoxiang.lock` ロックファイルをサポートする。

## 動機

### この機能/変更が必要な理由

パッケージ管理は сучасного プログラミング言語エコシステムの基盤である。現在の YaoXiang 言語には以下が欠けている：

- 依存関係宣言メカニズム
- バージョンマネジメント機能
- 標準配布チャネル

### 現在抱えている問題

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
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← 拡張可能なソース
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│  (ローカル)│  (VCS)   │  (オープン)│ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**拡張メカニズム**：新しい Source 型を追加するには trait を実装するだけでよく、解決エンジンの修正は不要。

### 使用例

```bash
# 1. プロジェクト作成
yaoxiang init my-project

# 2. yaoxiang.toml を編集して依存関係を追加
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. 依存関係 설치
yaoxiang add foo

# 4. コード中使用
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

[workspace.members]     # ワークスペースルートの فقط
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

解決順序は動作モード（`yaoxiang.toml` があるかどうか）によって異なる。

#### プロジェクトモード（yaoxiang.toml あり）

```
use foo.bar.baz;

検索順序:
0. 組み込みバイナリ                          (std/*.yx — コンパイル時埋め込み、バージョン束縛、std.* 名前空間のみ)
1. ./.yaoxiang/std/foo/bar/baz.yx     (プロジェクトレベル標準ライブラリ — 存在時グローバル標準ライブラリ是完全無効)
2. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
3. ./src/foo/bar/baz.yx                       (ローカルモジュール)
4. ~/.yaoxiang/cache/foo/<ver>/src/foo/bar/baz.yx  (グローバルキャッシュ)
5. $YXPATH/foo/bar/baz.yx                     (グローバルパス、今後対応)
```

**プロジェクトモードルール**：

- 組み込みバイナリは `std.*` 名前空間のみに適用され、優先度最高
- プロジェクトレベル標準ライブラリ（`.yaoxiang/std/`）が存在する場合、グローバル標準ライブラリは完全にスキップ——ビルドの再現性を保証
- プロジェクトは `yaoxiang add std@1.0.1` で標準ライブラリを依存関係として管理し、バージョンをロック可能

#### 単一ファイルモード（yaoxiang.toml なし）

```
use foo.bar.baz;

検索順序:
0. 組み込みバイナリ                          (std/*.yx — コンパイル時埋め込み、バージョン束縛)
1. <yaoxiang-install-dir>/yx/<version>/std/foo/bar/baz.yx  (グローバル標準ライブラリ)
2. ./src/foo/bar/baz.yx                                       (ローカルモジュール)
3. $YXPATH/foo/bar/baz.yx                                     (グローバルパス、今後対応)
```

**単一ファイルモードルール**：

- プロジェクトレベル依存関係はなく、標準ライブラリは直接グローバルパスからロード
- グローバル標準ライブラリパスはコンパイラバージョンと紐づく：`<install-dir>/yx/<version>/std/`

### 標準ライブラリ設置ディレクトリ構造

#### グローバル標準ライブラリ

```
<yaoxiang-install-dir>/
├── yx/                          # YaoXiang 言語ディレクトリ
│   ├── 1.0.1/                   # バージョン別ディレクトリ
│   │   ├── std/
│   │   │   ├── test.yx          # 純粋 YaoXiang 標準ライブラリモジュール
│   │   │   ├── math.yx          # 将来セルフホストモジュール
│   │   │   └── ...
│   │   └── ...
│   └── 1.1.0/
│       └── std/
│           └── ...
└── bin/
    └── yaoxiang                 # コンパイラバイナリ
```

#### プロジェクトレベル標準ライブラリ

プロジェクトは `yaoxiang add std@1.0.1` で標準ライブラリをプロジェクトの依存関係に追加でき、`.yaoxiang/std/` に保存：

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── .yaoxiang/
│   ├── std/                     # プロジェクトレベル標準ライブラリ（存在時グローバル標準ライブラリは無効）
│   │   ├── test.yx
│   │   ├── math.yx
│   │   └── ...
│   └── vendor/                  # その他の依存関係
│       └── ...
├── src/
│   └── main.yx
```

**設計ポイント**：

- 組み込みバイナリは互換レイヤーとして：ファイルシステム標準ライブラリが完全に整備される前に、まず組み込みバイナリで標準ライブラリモジュールを提供
- バージョン別ディレクトリ隔離：`yx/<version>/std/` により異なるバージョンの標準ライブラリを共存させ、互いの影響を防ぐ
- プロジェクトレベル標準ライブラリはグローバル標準ライブラリをオーバーライド：ビルドの再現性を確保し、グローバル環境の変化に影響されない
- yaoxiang.toml がない場合（単一ファイルモード）、グローバル標準ライブラリにフォールバック
- `.yaoxiang/std/` の存在は「プロジェクトレベル標準ライブラリ有効」を意味し、グローバル標準ライブラリはもう参加しない

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

// 依存関係仕様
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
    Workspace { member: String },  // ワークスペースメンバー参照
}

// 解決済み依存関係
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
    checksum: Option<String>,  // SHA-256
}

// ビルド戦略
enum BuildStrategy {
    None,          // 純粋 .yx パッケージ
    Cargo,         // cargo build 呼び出し
    Cmake,         // cmake 呼び出し
    Custom,        // build.yx スクリプト実行
    Precompiled,   // 事前コンパイル成果物を直接使用
}
```

### CLI コマンド設計

統合アプローチを採用し、コンパイラ、パケージマネージャー、REPL を単一 CLI ツールに統合：

#### 単一ファイルモード vs プロジェクトモード

| コマンド                  | 単一ファイル | プロジェクトモード | 説明             |
| ----------------------- | ------ | -------- | ----------------- |
| `yaoxiang run <file>`   | ✅     | ✅       | ファイル/プロジェクト入口を実行 |
| `yaoxiang build`        | ❌     | ✅       | プロジェクトをビルド          |
| `yaoxiang build <file>` | ✅     | ✅       | 単一ファイルをビルド          |
| `yaoxiang init <name>`  | ❌     | ✅       | プロジェクトを作成            |
| `yaoxiang add <dep>`    | ❌     | ✅       | 依存関係を追加              |
| `yaoxiang update`       | ❌     | ✅       | 依存関係を更新              |
| `yaoxiang fmt`          | ✅     | ✅       | フォーマット                |
| `yaoxiang check`        | ✅     | ✅       | 型チェック                  |
| `yaoxiang` (引数なし)   | ✅     | ✅       | 直接 REPL に入る             |

#### コマンド詳細

| コマンド                               | 機能                       | 例                                                   |
| ---------------------------------- | -------------------------- | ---------------------------------------------------- |
| `yaoxiang`                         | 直接 REPL に入る              | `yaoxiang`                                           |
| `yaoxiang run <file>`              | 単一ファイル/プロジェクトを実行 | `yaoxiang run main.yx`                               |
| `yaoxiang init <name>`             | 新規プロジェクトを作成         | `yaoxiang init my-app`                               |
| `yaoxiang build`                   | プロジェクトをビルド           | `yaoxiang build`                                     |
| `yaoxiang build <file>`            | 単一ファイルをビルド           | `yaoxiang build foo.yx`                              |
| `yaoxiang add <dep>`               | 依存関係を追加               | `yaoxiang add foo`                                   |
| `yaoxiang add -D <dep>`            | 開発依存関係を追加           | `yaoxiang add -D test`                               |
| `yaoxiang rm <dep>`                | 依存関係を削除               | `yaoxiang rm foo`                                    |
| `yaoxiang update`                  | すべての依存関係を更新         | `yaoxiang update`                                    |
| `yaoxiang update foo`              | 指定した依存関係を更新         | `yaoxiang update foo`                                |
| `yaoxiang install`                 | すべての依存関係をインストール   | `yaoxiang install`                                   |
| `yaoxiang list`                    | 依存関係を一覧表示             | `yaoxiang list`                                      |
| `yaoxiang outdated`                | 古くなった依存関係を検査        | `yaoxiang outdated`                                  |
| `yaoxiang fmt`                     | コードをフォーマット           | `yaoxiang fmt`                                       |
| `yaoxiang check`                   | 型チェック                  | `yaoxiang check`                                     |
| `yaoxiang clean`                   | ビルド成果物をクリーンアップ     | `yaoxiang clean`                                     |
| `yaoxiang task <name>`             | カスタムタスクを実行           | `yaoxiang task lint`                                 |
| `yaoxiang publish`                 | Registry にパッケージを公開    | `yaoxiang publish`                                   |
| `yaoxiang publish --github`        | 公開して GitHub Release を作成 | `yaoxiang publish --github`                          |
| `yaoxiang yank <pkg>@<ver>`        | 公開済みバージョンを削除（取り消し不可） | `yaoxiang yank foo@1.2.3`                            |
| `yaoxiang login --registry <url>`  | Registry 認証              | `yaoxiang login --registry https://reg.example.com`  |
| `yaoxiang login --github`          | GitHub 認証                | `yaoxiang login --github`                            |
| `yaoxiang logout --registry <url>` | ログアウト                       | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean`             | グローバルキャッシュをクリーンアップ   | `yaoxiang cache clean`                               |
| `yaoxiang workspace <cmd>`         | ワークスペース操作               | `yaoxiang workspace list`                            |

#### コマンド制約の説明

```bash
# 単一ファイルモード：yaoxiang.toml は不要
yaoxiang run hello.yx   # ✅ 正常に動作
yaoxiang add foo        # ❌ エラー：プロジェクトディレクトリではない

# プロジェクトモード：yaoxiang.toml が必要
cd my-project
yaoxiang run main.yx    # ✅ 入口ファイルを実行
yaoxiang build          # ✅ プロジェクトをビルド
yaoxiang add foo        # ✅ 依存関係を追加
```

### 後方互換性

- ✅ 既存の `use` 構文は完全維持
- ✅ 既存のモジュール解決ロジックは変更なし
- ✅ 新規追加の .yaoxiang/vendor ディレクトリは既存プロジェクトに影響なし

### グローバルキャッシュ

ダウンロードしたすべての依存関係を `~/.yaoxiang/cache/` にキャッシュし、プロジェクト vendor ディレクトリはキャッシュからコピーされる。

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

- Registry パッケージ：バージョン番号は不変、決して無効にならない
- Git 依存関係：tag/rev でキャッシュ、tag が変更されなければ無効にならない
- `yaoxiang cache clean` で手動クリーンアップ

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
- Token は決して `yaoxiang.toml` や `yaoxiang.lock` に書き込まない
- ファイルパーミッション 600

### yank セマンティクス

`yaoxiang yank foo@1.2.3` は**削除 + バージョン番号のロック**を実行：

- パッケージは完全に削除され、取り消し不可
- バージョン番号は恒久的に占有され、同じバージョン番号を再公開できない
- 既存の lockfile がそのバージョンを参照しているプロジェクトはエラーになり、アップグレードが必要
- **セキュリティ目的**：npm 型のサプライチェーン攻撃を防止（攻撃者が削除されたバージョン番号を奪い取って悪意のあるコードを注入）

### Registry プロトコル

詳細は [RFC-014a: Registry プロトコル仕様](../draft/014a-registry-protocol.md) を参照。

コア設計：オープンプ Rotokoll + 適応層。公式 Registry を主役に、GitHub Release/main ブランチを補完とし、カスタム Registry をサポート。

### ビルドシステム

詳細は [RFC-014b: ビルドシステムとバイナリ配布](../draft/014b-build-system.md) を参照。

コア設計：宣言型 `[build]` 設定、事前コンパイル優先/ソースコードフォールバック、cargo/cmake/custom 戦略をサポート。

### ワークスペース

詳細は [RFC-014c: ワークスペースサポート](../draft/014c-workspace.md) を参照。

コア設計：辞書形式の members 宣言、共有 lockfile、パス依存、Cargo workspace 統合。

## トレードオフ

### 利点

- 统一インポート構文で、ユーザーが依存関係の出所を気にする必要がない
- ロックファイルでビルドの再現性を保証
- オフラインサポート、ローカルにダウンロード後はオフライン開発可能
- Source trait により将来の拡張が容易

### 欠点

- 追加ストレージが必要（.yaoxiang/vendor ディレクトリ）
- バージョンの競合はユーザーが手動で解決する必要がある

## 代替案

| 案                       | 採用しなかった理由                    |
| -------------------------- | ----------------------------- |
| リアルタイム GitHub アクセス           | セキュリティとキャッシュ再利用の保証が困難      |
| グローバルキャッシュ ($HOME/.yaoxiang) | 分離性が悪く、バージョン競合が複雑        |
| Registry のみサポート               | GitHub は現在の主要なコードホスティングプラットフォーム |

## 実装戦略

### フェーズ分け

| フェーズ          | 内容                                         | ステータス      |
| ------------- | -------------------------------------------- | --------- |
| **Phase 1**   | toml 解析、ローカル依存関係、ロック生成、基础アルゴリズム     | ✅ 完了 |
| **Phase 2**   | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール | ✅ 完了 |
| **Phase 3**   | グローバルキャッシュ、semver crate 置換、CLI 完善        | 予定    |
| **Phase 3.5** | Source trait を async に変更、async-trait 統合      | 予定    |
| **Phase 4**   | Registry プロトコル、publish、認証（RFC-014a）     | 予定    |
| **Phase 5**   | ビルドシステム、事前コンパイルバイナリ（RFC-014b）           | 予定    |
| **Phase 6**   | ワークスペースサポート（RFC-014c）                     | 予定    |

### 依存関係

- 前置依存なし
- `ModuleGraph`（`middle/passes/module/`）との統合が必要

### リスク

| リスク             | 軽減措置                     |
| ---------------- | ---------------------------- |
| 依存関係解決アルゴリズムが複雑 | まずシンプルバージョンを実装し、後から競合検出を追加 |
| Git ダウンロードが不安定   | リトライとキャッシュメカニズム               |
| パフォーマンス問題         | 遅延ロード、インクリメンタル解決               |

## オープン問題

- [x] `dev-dependencies` 条件付きコンパイル構文？→ RFC-014b ビルドシステムで統一対応
- [x] 完全性検証アルゴリズム（SHA-256 / BLAKE3）？→ SHA-256
- [ ] `excludes` 特定のファイルを除外してダウンロードしない？
- [ ] パッケージ命名規則（namespace のサポート有無、`@org/pkg` など）？
- [ ] Registry API バージョニング戦略？

---

## 依存関係（Cargo.toml に追加が必要）

| 用途        | crate            | 説明           |
| ----------- | ---------------- | -------------- |
| セマンティックバージョニング  | `semver`         | 手書きパーサーを置換 |
| HTTP クライアント | `reqwest`        | Registry 通信  |
| SHA-256     | `sha2`           | 完全性検証     |
| 圧縮        | `flate2` + `tar` | パッケージ形式処理     |

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
