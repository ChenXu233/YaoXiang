---
title: 'RFC-014: パッケージ管理システム設計'
status: '承認済み'
author: '晨煦'
created: '2026-02-12'
updated: '2026-09-15'
group: 'rfc-014' # 本 RFC はパッケージ管理システムのマスタードキュメントであり、サブ RFC：014a/014b/014c
issue: '#88'
impl: '48%'
impl_status: '部分的'
---

# RFC-014: パッケージ管理システム設計（マスタードキュメント）

> **サブ RFC：**
>
> - [RFC-014a: Registry プロトコル仕様](../review/014a-registry-protocol.md)
> - [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
> - [RFC-014c: ワークスペースサポート](../review/014c-workspace.md)

## 概要

YaoXiang 言語のパッケージ管理システムを設計し、セマンティックバージョニング、ローカルおよび GitHub 依存関係、統一インポート構文、`yaoxiang.toml`
設定ファイル、`yaoxiang.lock` ロックファイルに対応する。

## 動機

### なぜこの機能/変更が必要なのか？

パッケージ管理は現代プログラミング言語エコシステムのインフラである。現在の YaoXiang 言語には以下が欠けている：

- 依存関係宣言メカニズム
- バージョン管理機能
- 標準配布チャネル

### 現在の問題

```
my-project/
├── src/
│   └── main.yx          # 代码依赖其他模块
├── lib/                  # 手动复制的模块
│   ├── foo.yx
│   └── bar.yx
└── ???                   # 没有标准依赖管理
```

## 提案

### コア設計

**階層アーキテクチャ**：

```
┌─────────────────────────────────────────────┐
│           Resolution Engine                  │ ← 依赖解析
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← 可扩展源
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│  (本地)  │  (VCS)   │  (开放)  │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**拡張メカニズム**：新しい Source 型を追加するには trait を実装するだけでよく、解析エンジンの変更は不要。

### 例

```bash
# 1. 创建项目
yaoxiang init my-project

# 2. 编辑 yaoxiang.toml 添加依赖
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. 安装依赖
yaoxiang add foo

# 4. 代码中使用
use foo;
use bar.baz;
```

### プロジェクト構造

```
my-project/
├── yaoxiang.toml        # 包配置
├── yaoxiang.lock        # 锁定文件（自动生成）
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/              # 本地依赖
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
foo = "1.2.3"           # 精确版本
bar = "^1.0.0"          # 兼容版本
baz = "~1.2.0"          # 补丁版本
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
strategy = "none"       # none | cargo | cmake | custom

[binaries]
"linux-x86_64" = { url = "...", sha256 = "..." }

[workspace.members]     # 仅工作空间根
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

> **2026-09-15 決議：中核パッケージソースの排他セマンティクス（Python venv / Node
> node_modules 方式）。**
> 元の「5 層貫通検索チェーン」記述を廃止——vendor と全局は**二者択一**として唯一の中核パッケージソースとなり、std は中核ソースに埋め込まれ、ローカルモジュールはすべてを上書きできる。

**中核パッケージソース判定（排他、決して混在しない）**：

- プロジェクトに `.yaoxiang/vendor/` が存在する場合 →
  **vendor が唯一の中核パッケージソース**。ローカルモジュール以外のすべての `use`
  は vendor からのみ解決され、vendor にパッケージがない場合は直接エラーを返す（`yaoxiang install`
  を案内）、**全局へのフォールバックは行わない**。
- それ以外の場合 →
  **全局が中核パッケージソース**（インストールディレクトリの std + 全局キャッシュ）。

「vendor になければ全局キャッシュにフォールバック」といった、パッケージ単位での貫通は存在しない——2 つのソースを混在させることが、まさにバージョン漂流と「自分のマシンでは動く」の根本原因である。

#### プロジェクトモード（yaoxiang.toml あり）

```
use foo.bar.baz;

查找顺序:
1. ./src/foo/bar/baz.yx     本地模块 —— 最高优先级，可覆盖核心源中的同名模块
2. <核心包源>/foo/bar/baz.yx
   · vendor 模式: .yaoxiang/vendor/<pkg>-<ver>/src/foo/bar/baz.yx（std 也在 vendor 内）
   · 全局模式:   <install-dir>/yx/<ver>/std/foo/bar/baz.yx + ~/.yaoxiang/cache/...
3. std.* 专属兜底: 嵌入二进制（仅 std.* 命名空间；文件系统 std 落地前的过渡层，版本绑定编译器）
4. 报错（模块不存在）；vendor 模式下缺包提示 `yaoxiang install`
```

**プロジェクトモード規則**：

- `yaoxiang add std@1.0.1`
  は std を通常の依存関係として vendor にインストールし、バージョンをロックする；このとき埋め込みバイナリの std は無効となり（vendor 内の std が優先）
- vendor が存在するが `yaoxiang.lock` と一致しない場合、`run`/`build` はエラーを返し
  `yaoxiang install` を案内する（Node セマンティクス：静かに自動インストールしない）
- ローカルモジュールが中核ソースの同名モジュールを上書きする場合、デフォルトで W レベルの診断を発してシャドウを警告する（`--deny-shadowing`
  でエラーに昇格可能）；`std.*` を上書きする場合、診断メッセージで明示的に警告する
- `path` 依存はローカルモジュールの延長として扱い、中核パッケージソースを介さず直接パスで解決する

#### 単一ファイルモード（yaoxiang.toml なし）

```
use foo.bar.baz;

查找顺序:
1. ./src/foo/bar/baz.yx     本地模块
2. 全局核心包源: <install-dir>/yx/<version>/std/foo/bar/baz.yx
3. 嵌入二进制 std 兜底
4. $YXPATH/foo/bar/baz.yx   （全局路径，预留）
```

**単一ファイルモード規則**：

- プロジェクトレベルの依存関係という概念はなく、std は直接全局から取得される；全局標準ライブラリのパスはコンパイラのバージョンに紐付けられる：`<install-dir>/yx/<version>/std/`
- 単一ファイルモードは決して `.yaoxiang/` を読み取らない（vendor の概念なし）

### 標準ライブラリインストールディレクトリ構造

#### 全局標準ライブラリ

```
<yaoxiang-install-dir>/
├── yx/                          # YaoXiang 语言目录
│   ├── 1.0.1/                   # 版本目录
│   │   ├── std/
│   │   │   ├── test.yx          # 纯 YaoXiang 标准库模块
│   │   │   ├── math.yx          # 未来自举模块
│   │   │   └── ...
│   │   └── ...
│   └── 1.1.0/
│       └── std/
│           └── ...
└── bin/
    └── yaoxiang                 # 编译器二进制
```

#### プロジェクトレベル標準ライブラリ

> **2026-09-15 決議：独立した `.yaoxiang/std/` ディレクトリはもう設けない。**
> std は中核パッケージソース内の通常のパッケージである：`yaoxiang add std@1.0.1` で
> `.yaoxiang/vendor/std-<version>/`
> に格納され、他の依存関係と同じ規則で管理される。元の「プロジェクトレベル std が存在すれば全局 std が無効になる」は特別なルールを必要としない——中核パッケージソースの排他性により自然に保証される。

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── .yaoxiang/
│   └── vendor/
│       ├── std-1.0.1/           # std 作为普通 vendor 包
│       └── foo-1.2.3/
├── src/
│   └── main.yx
```

**設計のポイント**：

- 埋め込みバイナリを互換レイヤーとして提供：ファイルシステム標準ライブラリが完全に実装されるまで、埋め込みバイナリで std を提供する
- バージョンディレクトリによる分離：`yx/<version>/std/`
  により、異なるバージョンの標準ライブラリが共存し、相互に影響しない
- std は通常の依存関係と同じメカニズム（add/lock/vendor）で、特別なディレクトリも特別な検索階層もない
- 単一ファイルモードでは全局 std にフォールバック；vendor が存在する場合、std は vendor から取得しなければならない（または明示的に
  `add std@` でロック）

### 中核データ構造

```rust
// 依赖来源（可扩展）
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> },
    GitHub { owner: String, repo: String, ref_: GitRef },  // GitHub 原生
}

enum GitRef {
    Tag(String),
    Branch(String),
    Rev(String),
    DefaultBranch,
}

// 依赖声明
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
    Workspace { member: String },  // 工作空间成员引用
}

// 解析后的依赖（2026-09-15 决议：完整性只用 integrity 单字段，格式 "sha256-<hex>"，不设重复的 checksum）
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
}

// 构建策略
enum BuildStrategy {
    None,          // 纯 .yx 包
    Cargo,         // 调用 cargo build
    Cmake,         // 调用 cmake
    Custom,        // 执行 build.yx 脚本
    Precompiled,   // 直接用预编译产物
}
```

### CLI コマンド設計

統一スキームを採用し、コンパイラ、パッケージマネージャ、REPL を単一の CLI ツールに統合する：

#### 単一ファイルモード vs プロジェクトモード

| コマンド                | 単一ファイル | プロジェクトモード | 説明                                          |
| ----------------------- | ------------ | ------------------ | --------------------------------------------- |
| `yaoxiang run <file>`   | ✅           | ✅                 | ファイル/プロジェクトのエントリポイントを実行 |
| `yaoxiang build`        | ❌           | ✅                 | プロジェクトをビルド                          |
| `yaoxiang build <file>` | ✅           | ✅                 | 単一ファイルをビルド                          |
| `yaoxiang init <name>`  | ❌           | ✅                 | プロジェクトを作成                            |
| `yaoxiang add <dep>`    | ❌           | ✅                 | 依存関係を追加                                |
| `yaoxiang update`       | ❌           | ✅                 | 依存関係を更新                                |
| `yaoxiang fmt`          | ✅           | ✅                 | フォーマット                                  |
| `yaoxiang check`        | ✅           | ✅                 | 型チェック                                    |
| `yaoxiang` (引数なし)   | ✅           | ✅                 | 直接 REPL を起動                              |

#### コマンド詳細

| コマンド                           | 機能                                 | 例                                                   |
| ---------------------------------- | ------------------------------------ | ---------------------------------------------------- |
| `yaoxiang`                         | 直接 REPL を起動                     | `yaoxiang`                                           |
| `yaoxiang run <file>`              | 単一ファイル/プロジェクトを実行      | `yaoxiang run main.yx`                               |
| `yaoxiang init <name>`             | 新規プロジェクトを作成               | `yaoxiang init my-app`                               |
| `yaoxiang build`                   | プロジェクトをビルド                 | `yaoxiang build`                                     |
| `yaoxiang build <file>`            | 単一ファイルをビルド                 | `yaoxiang build foo.yx`                              |
| `yaoxiang add <dep>`               | 依存関係を追加                       | `yaoxiang add foo`                                   |
| `yaoxiang add -D <dep>`            | 開発依存関係を追加                   | `yaoxiang add -D test`                               |
| `yaoxiang rm <dep>`                | 依存関係を削除                       | `yaoxiang rm foo`                                    |
| `yaoxiang update`                  | すべての依存関係を更新               | `yaoxiang update`                                    |
| `yaoxiang update foo`              | 指定した依存関係を更新               | `yaoxiang update foo`                                |
| `yaoxiang install`                 | すべての依存関係をインストール       | `yaoxiang install`                                   |
| `yaoxiang list`                    | 依存関係を一覧表示                   | `yaoxiang list`                                      |
| `yaoxiang outdated`                | 古い依存関係をチェック               | `yaoxiang outdated`                                  |
| `yaoxiang fmt`                     | コードをフォーマット                 | `yaoxiang fmt`                                       |
| `yaoxiang check`                   | 型チェック                           | `yaoxiang check`                                     |
| `yaoxiang clean`                   | ビルド成果物をクリーンアップ         | `yaoxiang clean`                                     |
| `yaoxiang task <name>`             | カスタムタスクを実行                 | `yaoxiang task lint`                                 |
| `yaoxiang publish`                 | Registry にパッケージを公開          | `yaoxiang publish`                                   |
| `yaoxiang publish --github`        | 公開して GitHub Release を作成       | `yaoxiang publish --github`                          |
| `yaoxiang yank <pkg>@<ver>`        | 公開済みバージョンを削除（復元不可） | `yaoxiang yank foo@1.2.3`                            |
| `yaoxiang login --registry <url>`  | Registry 認証                        | `yaoxiang login --registry https://reg.example.com`  |
| `yaoxiang login --github`          | GitHub 認証                          | `yaoxiang login --github`                            |
| `yaoxiang logout --registry <url>` | ログアウト                           | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean`             | グローバルキャッシュをクリーンアップ | `yaoxiang cache clean`                               |
| `yaoxiang workspace <cmd>`         | ワークスペース操作                   | `yaoxiang workspace list`                            |

#### コマンド制約の説明

```bash
# 单文件模式：不需要 yaoxiang.toml
yaoxiang run hello.yx   # ✅ 正常工作
yaoxiang add foo        # ❌ 报错：不是项目目录

# 项目模式：需要 yaoxiang.toml
cd my-project
yaoxiang run main.yx    # ✅ 运行入口文件
yaoxiang build          # ✅ 构建项目
yaoxiang add foo        # ✅ 添加依赖
```

### 後方互換性

- ✅ 既存の `use` 構文を完全に保持
- ✅ 既存のモジュール解決ロジックは変更なし
- ✅ 新規 `.yaoxiang/vendor` ディレクトリは既存プロジェクトに影響しない

### グローバルキャッシュ

ダウンロードしたすべての依存関係は `~/.yaoxiang/cache/`
にキャッシュされ、プロジェクトの vendor ディレクトリはキャッシュからコピーされる。

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

- Registry パッケージ：バージョン番号は不変で、決して無効化されない
- Git 依存関係：tag/rev でキャッシュされ、tag が変更されなければ無効化されない
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

- 環境変数を優先：`$YX_GITHUB_TOKEN`、`$YX_REGISTRY_TOKEN`
- Token は決して `yaoxiang.toml` や `yaoxiang.lock` に書き込まない
- ファイルパーミッション 600

### yank セマンティクス

`yaoxiang yank foo@1.2.3` は**削除 + バージョン番号の永久ロック**を実行する：

- パッケージは完全に削除され、復元不可
- バージョン番号は永久に占有され、同じバージョン番号を再公開できない
- 既にそのバージョンを参照している lockfile があるプロジェクトはエラーを返し、アップグレードが必要
- **セキュリティ目的**：npm 型のサプライチェーン攻撃を防ぐ（攻撃者が削除されたバージョン番号を横取りして悪意のあるコードを注入する）

### Registry プロトコル

詳細は [RFC-014a: Registry プロトコル仕様](../review/014a-registry-protocol.md) を参照。

中核設計：オープンプロトコル + アダプタ層。公式 Registry を主とし、GitHub
Release/main ブランチを補助とし、カスタム Registry をサポートする。

### ビルドシステム

詳細は [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md) を参照。

中核設計：宣言的 `[build]`
設定、プリコンパイル優先/ソースコードフォールバック、cargo/cmake/custom ストラテジをサポート。

### ワークスペース

詳細は [RFC-014c: ワークスペースサポート](../review/014c-workspace.md) を参照。

中核設計：辞書形式の members 宣言、共有 lockfile、パス依存関係、Cargo workspace 統合。

## トレードオフ

### メリット

- インポート構文が統一されており、ユーザーは依存ソースを気にする必要がない
- 決定論的ビルド、lock ファイルがビルドの一貫性を保証
- オフラインサポート、ローカルにダウンロード後はオフラインで開発可能
- Source trait により後続の拡張が容易

### デメリット

- 追加のストレージ容量が必要（.yaoxiang/vendor ディレクトリ）
- バージョンの競合はユーザーが手動で解決する必要がある

## 代替案

| 代替案                                 | 採用しなかった理由                                    |
| -------------------------------------- | ----------------------------------------------------- |
| リアルタイム GitHub アクセス           | セキュリティとキャッシュ再利用の保証が困難            |
| グローバルキャッシュ ($HOME/.yaoxiang) | 分離性が低く、バージョン競合が複雑                    |
| レジストリのみサポート                 | GitHub は現在の主流コードホスティングプラットフォーム |

## 実装戦略

### 段階区分

| フェーズ      | 内容                                                                                                                    | 状態    |
| ------------- | ----------------------------------------------------------------------------------------------------------------------- | ------- |
| **Phase 1**   | toml 解析、ローカル依存関係、lock 生成、基礎アルゴリズム                                                                | ✅ 完了 |
| **Phase 2**   | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール                                                              | ✅ 完了 |
| **Phase 3**   | グローバルキャッシュ、semver crate 置き換え、CLI 整備                                                                   | 未着手  |
| **Phase 3.5** | Source trait の async 化、async-trait 統合                                                                              | 未着手  |
| **Phase 4**   | GitHub アダプタ層、.yxpkg パッケージング、publish --github（RFC-014a 縮小後スコープ；公式 Registry/auth/yank は後回し） | 未着手  |
| **Phase 5**   | ビルドシステム、プリコンパイルバイナリ（RFC-014b）                                                                      | 未着手  |
| **Phase 6**   | ワークスペースサポート（RFC-014c）                                                                                      | 未着手  |

**実行順序の調整（2026-09-15）**：`3 → 3.5 → 6 → 4 → 5`。

- ワークスペース（Phase
  6）をビルドシステムより前に前倒し——ネットワークやビルドシステムに依存せず（純粋なローカルパス解決 + 共有 lockfile）、マルチパッケージ開発へのメリットが最も直接的。
- Phase 4 のスコープ縮小：**公式 Registry サーバーと auth/yank を無期限に後回し**、まず GitHub
  Release/Git アダプタ層 + `.yxpkg` パッケージング + `publish --github`
  を先に提供する。エコシステムのコールドスタートには git/GitHub チャネルがあれば十分（Go の初期と同型）、Registry サーバーの運用とガバナンスコストはサードパーティパッケージがない段階では完全な負債である。
- それに伴う制約：公式 Registry のリリース前は `yaoxiang add <ベアパッケージ名>`
  は使用不可、依存関係の追加には明示的なソース指定が必要（`--git` / `--path`）。

### 依存関係

- 前置依存なし
- `ModuleGraph`（`middle/passes/module/`）との統合が必要

### リスク

| リスク                           | 緩和策                                     |
| -------------------------------- | ------------------------------------------ |
| 依存関係解決アルゴリズムの複雑さ | まずシンプル版を実装し、競合検出は後で追加 |
| Git ダウンロードの不安定性       | リトライとキャッシュ機構                   |
| パフォーマンス問題               | 遅延読み込み、インクリメンタル解析         |

## オープンクエスチョン

- [x] `dev-dependencies` の条件付きコンパイル構文？→ RFC-014b のビルドシステムで統一処理
- [x] 完全性検証アルゴリズム（SHA-256 / BLAKE3）？→ SHA-256
- [x] パッケージ命名規則（namespace をサポートするか、`@org/pkg`
      のような形式）？→ 初期はフラットなパッケージ名、namespace は未サポート；`@org/pkg`
      は予約（2026-09-15 決議）
- [x] Registry API のバージョニング戦略？→ URL パス
      `/api/v1/` + レスポンスヘッダでプロトコルバージョンを伝達、破壊的変更は v2 にアップグレードして共存（2026-09-15 決議、RFC-014a 参照）
- [ ] `excludes` で特定ファイルのダウンロードを除外する？

---

## 依存関係（Cargo.toml に追加が必要）

| 用途                         | crate            | 説明                       |
| ---------------------------- | ---------------- | -------------------------- |
| セマンティックバージョニング | `semver`         | 手書きパーサを置き換え     |
| HTTP クライアント            | `reqwest`        | Registry 通信              |
| SHA-256                      | `sha2`           | 完全性検証                 |
| 圧縮                         | `flate2` + `tar` | パッケージフォーマット処理 |

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
