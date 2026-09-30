---
title: 'RFC-014: パッケージ管理システムの設計'
status: '承認済み'
author: '晨煦'
created: '2026-02-12'
updated: '2026-09-29'
group: 'rfc-014' # 本 RFC はパッケージ管理システムの総綱であり、サブ RFC：014a/014b/014c
issue: '#88'
impl: '48%'
impl_status: 'partial'
---

# RFC-014: パッケージ管理システムの設計（総綱）

> **サブ RFC：**
>
> - [RFC-014a: Registry プロトコル仕様](../accepted/014a-registry-protocol.md)
> - [RFC-014b: ビルドシステムとバイナリ配布](../accepted/014b-build-system.md)
> - [RFC-014c: ワークスペースサポート](../accepted/014c-workspace.md)

## 要約

YaoXiang 言語のパッケージ管理システムを設計し、セマンティックバージョニング、ローカルおよび GitHub 依存関係、統一インポート構文、`yaoxiang.toml`
設定ファイル、`yaoxiang.lock` ロックファイルをサポートする。

## 動機

### なぜこの機能/変更が必要なのか？

パッケージ管理は現代プログラミング言語エコシステムのインフラである。現在、YaoXiang 言語には以下が欠けている：

- 依存関係宣言メカニズム
- バージョン管理能力
- 標準配布チャネル

### 現在の問題

```
my-project/
├── src/
│   └── main.yx          # コード依赖其他模块
├── lib/                  # 手动复制的模块
│   ├── foo.yx
│   └── bar.yx
└── ???                   # 没有标准依赖管理
```

## 提案

### 中核設計

**階層化アーキテクチャ**：

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

**拡張メカニズム**：新しい Source タイプを追加するには trait を実装するだけでよく、解決エンジンを変更する必要はない。

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
> 元の「5 階層透過検索チェーン」記述を廃止する——vendor とグローバルは**二者択一**で唯一の中核パッケージソースとし、std は中核ソースに埋め込まれ、ローカルモジュールはその他すべてを上書き可能。

**中核パッケージソース判定（排他、決して混在しない）**：

- プロジェクトに `.yaoxiang/vendor/` が存在する →
  **vendor が唯一の中核パッケージソース**。非ローカルモジュールのすべての `use`
  は vendor からのみ解決され、vendor に欠落パッケージがある場合は直接エラーを報告する（`yaoxiang install`
  を促す）、**グローバルへのフォールバックは行わない**。
- それ以外 →
  **グローバルが中核パッケージソース**（インストールディレクトリ std + グローバルキャッシュ）。

「vendor にない場合はグローバルキャッシュにフォールバック」というパッケージ単位の透過検索は存在しない——2 つのソースの混在こそが、バージョンドリフトと「私のマシンでは動く」の根本原因である。

#### プロジェクトモード（yaoxiang.toml あり）

```
use foo.bar.baz;

查找顺序:
1. ./src/foo/bar/baz.yx     本地模块 —— 最高优先级，可覆盖核心源中的同名模块
2. <核心包源>/foo/bar/baz.yx
   · vendor 模式: .yaoxiang/vendor/<pkg>-<ver>/src/foo/bar/baz.yx（2026-09-28 修订：std 不在 vendor 内，见项目模式规则）
   · 全局模式:   <install-dir>/yx/<ver>/std/foo/bar/baz.yx + ~/.yaoxiang/cache/...
3. std.* 专属兜底: 嵌入二进制（仅 std.* 命名空间；2026-09-28 修订：正式机制，见项目模式规则）
4. 报错（模块不存在）；vendor 模式下缺包提示 `yaoxiang install`
```

**プロジェクトモードルール**：

- vendor が存在するが `yaoxiang.lock` と一致しない場合、`run`/`build` はエラーを報告し
  `yaoxiang install` を促す（Node セマンティクス：サイレント自動インストールしない）
- ローカルモジュールが中核ソースの同名モジュールを上書きする場合、デフォルトで W 級診断でシャドウイングを通知する（`--deny-shadowing`
  でエラーに昇格可能）
- `path`
  依存関係はローカルモジュールの延長として扱われ、経路で直接解決され、中核パッケージソースを経由しない

> **2026-09-28 改訂：std はパッケージ化しない（2026-09-15 決議の std パッケージ化を覆す）。**
> 埋め込みバイナリ std を「過渡的フォールバック」から**正式メカニズム**に昇格：std はコンパイラ ABI レベルで結合している（ネイティブ層/ランタイム組み込みは VM レイアウトと一致する必要がある）、パッケージで std バージョンをロックすると「std-1.0.1 + コンパイラ 1.0.2」のような隠れた不整合を生み出す；std ドリフトの正解はプロジェクトレベルのツールチェーンロック（必要なら別途議論）であり、std パッケージではない。Go/Rust/Python の先例と一致する——言語付属の std はツールチェーンに従う。
> `yaoxiang add std@<ver>` は使用不可；`std.*`
> は予約済み名前空間であり、ローカルモジュールは上書きできない（埋め込み std にはファイル形式がないため、シャドウイングは議論の余地がない）；RFC-037 の
> `.yaoxiang/vendor/std/`
> インタフェースファイルディレクトリ（LSP 検索チェーンの第 1 階層）は保持され、「ソースコード閲覧」役割を継続する。補足修正：ローカルモジュールのシャドウイング診断範囲は依存パッケージのみとなり、std.* 例外テキストはもはや含まれない。

#### 単一ファイルモード（yaoxiang.toml なし）

```
use foo.bar.baz;

查找顺序:
1. ./src/foo/bar/baz.yx     本地模块
2. 全局核心包源: <install-dir>/yx/<version>/std/foo/bar/baz.yx
3. 嵌入二进制 std 兜底
4. $YXPATH/foo/bar/baz.yx   （全局路径，预留）
```

**単一ファイルモードルール**：

- プロジェクトレベルの依存関係概念はなく、std は直接グローバルから取得される；グローバル標準ライブラリパスはコンパイラバージョンと結合される：`<install-dir>/yx/<version>/std/`
- 単一ファイルモードは決して `.yaoxiang/` を読み取らない（vendor 概念なし）

### 標準ライブラリインストールディレクトリ構造

#### グローバル標準ライブラリ

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
> **2026-09-28 改訂：std はパッケージ化しない**（推論は「プロジェクトモードルール」末尾参照）——std は埋め込みバイナリと RFC-037 インタフェースファイルディレクトリを維持し、`add std@<ver>`
> は使用不可、`std.*` は上書き不可として予約される。ディレクトリの排他結論（独立した
> `.yaoxiang/std/` なし）は有効。

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

**設計要点**：

- 埋め込みバイナリは互換性レイヤーとして：ファイルシステム標準ライブラリが完全に実装される前に、まず埋め込みバイナリで std モジュールを提供する
- バージョンディレクトリ隔離：`yx/<version>/std/`
  により、異なるバージョンの標準ライブラリが共存し、相互に影響しない
- std は通常依存関係と同じメカニズム（add/lock/vendor）で、特殊ディレクトリも特殊検索階層もない
- 単一ファイルモードはグローバル std にフォールバック；vendor 存在時も std は埋め込みバイナリ/インタフェースディレクトリから取得される（2026-09-28 改訂：std はパッケージ化されず、vendor に従わない）

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

統一方式を採用し、コンパイラ、パッケージマネージャ、REPL を単一の CLI ツールに統合する：

#### 単一ファイルモード vs プロジェクトモード

| コマンド                | 単一ファイル | プロジェクトモード | 説明                            |
| ----------------------- | ------------ | ------------------ | ------------------------------- |
| `yaoxiang run <file>`   | ✅           | ✅                 | ファイル/プロジェクト入口を実行 |
| `yaoxiang build`        | ❌           | ✅                 | プロジェクトをビルド            |
| `yaoxiang build <file>` | ✅           | ✅                 | 単一ファイルをビルド            |
| `yaoxiang init <name>`  | ❌           | ✅                 | プロジェクトを作成              |
| `yaoxiang add <dep>`    | ❌           | ✅                 | 依存関係を追加                  |
| `yaoxiang update`       | ❌           | ✅                 | 依存関係を更新                  |
| `yaoxiang fmt`          | ✅           | ✅                 | フォーマット                    |
| `yaoxiang check`        | ✅           | ✅                 | 型検査                          |
| `yaoxiang`（引数なし）  | ✅           | ✅                 | 直接 REPL に入る                |

#### コマンド詳細

| コマンド                           | 機能                                                             | 例                                                                      |
| ---------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `yaoxiang`                         | 直接 REPL に入る                                                 | `yaoxiang`                                                              |
| `yaoxiang run <file>`              | 単一ファイル/プロジェクトを実行                                  | `yaoxiang run main.yx`                                                  |
| `yaoxiang init <name>`             | 新規プロジェクトを作成                                           | `yaoxiang init my-app`                                                  |
| `yaoxiang build`                   | プロジェクトをビルド                                             | `yaoxiang build`                                                        |
| `yaoxiang build <file>`            | 単一ファイルをビルド                                             | `yaoxiang build foo.yx`                                                 |
| `yaoxiang add <dep>`               | 依存関係を追加                                                   | `yaoxiang add foo`                                                      |
| `yaoxiang add -D <dep>`            | 開発依存関係を追加                                               | `yaoxiang add -D test`                                                  |
| `yaoxiang rm <dep>`                | 依存関係を削除                                                   | `yaoxiang rm foo`                                                       |
| `yaoxiang update`                  | すべての依存関係を更新                                           | `yaoxiang update`                                                       |
| `yaoxiang update foo`              | 指定された依存関係を更新                                         | `yaoxiang update foo`                                                   |
| `yaoxiang install`                 | すべての依存関係をインストール                                   | `yaoxiang install`                                                      |
| `yaoxiang list`                    | 依存関係を一覧表示                                               | `yaoxiang list`                                                         |
| `yaoxiang outdated`                | 古い依存関係を確認                                               | `yaoxiang outdated`                                                     |
| `yaoxiang fmt`                     | コードをフォーマット                                             | `yaoxiang fmt`                                                          |
| `yaoxiang check`                   | 型検査                                                           | `yaoxiang check`                                                        |
| `yaoxiang clean`                   | ビルド成果物をクリア                                             | `yaoxiang clean`                                                        |
| `yaoxiang task <name>`             | カスタムタスクを実行                                             | `yaoxiang task lint`                                                    |
| `yaoxiang publish`                 | Registry にパッケージを公開                                      | 後続：公式 Registry は無期限で後続、裸の publish はエラーを報告し指示   |
| `yaoxiang publish --dry-run`       | 検証 + `.yxpkg` を `target/yxpkg/` にパッケージ                  | `yaoxiang publish --dry-run`                                            |
| `yaoxiang publish --github`        | GitHub Release として公開（`.yxpkg` アセット；tag の存在が必要） | `yaoxiang publish --github`                                             |
| `yaoxiang yank <pkg>@<ver>`        | 公開済みバージョンを削除（復元不可）                             | 後続：公式 Registry と共に                                              |
| `yaoxiang login --registry <url>`  | Registry 認証                                                    | 後続：公式 Registry と共に（GitHub 側は現在 `$YX_GITHUB_TOKEN` を使用） |
| `yaoxiang login --github`          | GitHub 認証                                                      | 後続：同上                                                              |
| `yaoxiang logout --registry <url>` | ログアウト                                                       | 後続：同上                                                              |
| `yaoxiang cache clean`             | グローバルキャッシュをクリア                                     | `yaoxiang cache clean`                                                  |
| `yaoxiang workspace <cmd>`         | ワークスペース操作                                               | `yaoxiang workspace list`                                               |

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

- ✅ 既存の `use` 構文は完全に保持される
- ✅ 既存のモジュール解決ロジックは変更されない
- ✅ 新規 `.yaoxiang/vendor` ディレクトリは既存プロジェクトに影響しない

### グローバルキャッシュ

すべてのダウンロードされた依存関係は `~/.yaoxiang/cache/`
にキャッシュされ、プロジェクト vendor ディレクトリはキャッシュからコピーされる。

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

- Registry パッケージ：バージョン番号は不変、無効化されない
- Git 依存関係：tag/rev でキャッシュ、tag が不変なら無効化されない
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

- 環境変数を優先：`$YX_GITHUB_TOKEN`、`$YX_REGISTRY_TOKEN`
- Token は決して `yaoxiang.toml` や `yaoxiang.lock` に書き込まれない
- ファイルパーミッション 600

### yank セマンティクス

`yaoxiang yank foo@1.2.3` は**削除 + バージョン番号永久ロック**を実行する：

- パッケージは完全に削除され、復元できない
- バージョン番号は永久に占有され、同じバージョン番号を再公開できない
- 既にこのバージョンを参照している lockfile を持つプロジェクトはエラーを報告し、アップグレードが必要
- **セキュリティ目的**：npm 型のサプライチェーン攻撃を防止する（攻撃者が削除されたバージョン番号を奪い、悪意のあるコードを注入する）

### Registry プロトコル

詳細は [RFC-014a: Registry プロトコル仕様](../accepted/014a-registry-protocol.md) を参照。

中核設計：開放型プロトコル + アダプタ層。公式 Registry がメイン、GitHub
Release/メインブランチが補助、カスタム Registry をサポート。

### ビルドシステム

詳細は [RFC-014b: ビルドシステムとバイナリ配布](../accepted/014b-build-system.md) を参照。

中核設計：宣言型 `[build]`
設定、事前コンパイル優先/ソースコードフォールバック、cargo/cmake/custom 戦略をサポート。

### ワークスペース

詳細は [RFC-014c: ワークスペースサポート](../accepted/014c-workspace.md) を参照。

中核設計：辞書形式 members 宣言、共有 lockfile、経路依存、Cargo workspace 統合。

## トレードオフ

### 利点

- 統一インポート構文により、ユーザーは依存関係ソースを気にする必要がない
- 確定的ビルド、lock ファイルがビルド一貫性を保証
- オフラインサポート、ローカルにダウンロード後はオフライン開発可能
- Source trait が将来の拡張を容易にする

### 欠点

- 追加のストレージ容量が必要（.yaoxiang/vendor ディレクトリ）
- バージョン競合はユーザーが手動で解決する必要がある

## 代替案

| 案                                     | 選ばなかった理由                                      |
| -------------------------------------- | ----------------------------------------------------- |
| リアルタイム GitHub アクセス           | セキュリティとキャッシュ再利用を保証しにくい          |
| グローバルキャッシュ ($HOME/.yaoxiang) | 隔離性が低く、バージョン競合が複雑                    |
| レジストリのみサポート                 | GitHub は現在の主流コードホスティングプラットフォーム |

## 実装戦略

### 段階区分

| 段階          | 内容                                                                                                            | 状態                                        |
| ------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| **Phase 1**   | toml 解析、ローカル依存関係、lock 生成、基本アルゴリズム                                                        | ✅ 完了                                     |
| **Phase 2**   | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール                                                      | ✅ 完了                                     |
| **Phase 3**   | グローバルキャッシュ、semver crate 置換、CLI 改善                                                               | ✅ 完了                                     |
| **Phase 3.5** | Source 分離 enum 化 + ネイティブ async（014a 決議 4、async-trait を導入しない）                                 | ✅ 完了                                     |
| **Phase 4**   | GitHub アダプタ層、.yxpkg パッケージング、publish --github（RFC-014a 縮小範囲；公式 Registry/auth/yank は後続） | ✅ 完了                                     |
| **Phase 5**   | ビルドシステム、事前コンパイル済みバイナリ（RFC-014b）                                                          | ✅ 完了                                     |
| **Phase 6**   | ワークスペースサポート（RFC-014c）                                                                              | ✅ 6a-c + メンバー管理 + 6d 完了（6e 後続） |

**実行順序調整（2026-09-15）**：`3 → 3.5 → 6 → 4 → 5`。

- ワークスペース（Phase
  6）をビルドシステムの前に前倒し——ネットワークやビルドシステムに依存しない（純粋なローカルパス解決 + 共有 lockfile）、マルチパッケージ開発への利益が最も直接的。
- Phase 4 範囲縮小：**公式 Registry サーバーと auth/yank は無期限で後続**、まず GitHub
  Release/Git アダプタ層 + `.yxpkg` パッケージング + `publish --github`
  を配信。生態系のコールドスタートには git/GitHub チャネルのみ必要（Go 初期と同型）、Registry サーバーの運用とガバナンスコストはサードパーティパッケージがない段階では純負債である。
- 付随する制約：公式 Registry リリース前、`yaoxiang add <裸のパッケージ名>`
  は使用不可、依存関係追加は明示的なソース（`--git` / `--path`）が必要。

**Phase 3 実装説明（2026-09-28）**：

- Phase 3.5 実装説明（2026-09-29）：Source 分離は決議 4 に従い `AnySource`
  4 ソースクローズド集合を採用（Local/Git/Registry/GitHub、後ろ 2 つは Phase
  4 プレースホルダ）；`Source` trait の resolve/download はネイティブ async fn in
  trait、コマンド層は `futures::executor::block_on`
  で駆動（ランタイムなし、Git サブプロセスは std::process を維持；Phase
  4 で reqwest を接続する際に真のエグゼキュータに交換）；新規 `futures`
  依存（wasm32 互換）。install の並列ダウンロードは真のランタイム導入時にまとめて行う。
- Phase 4 実装説明（2026-09-29、コミット 234dfea5/e1873133/5ffee636）：
  - **GitHub アダプタ層**（4a）：github.com の git 依存関係を `GitHubSource`
    にルーティング——バージョン解析は REST
    API を経由（releases エンドポイント、空なら tags にフォールバック）、ダウンロードは Release の
    `.yxpkg` アセットを優先（解凍検証後 `cache/github/`
    に入れてから vendor にコピー）、アセットがなければ git クローンにフォールバック（SourceKind は如实
    `Git` と報告）。API アクセスは指数バックオフ付き（1s/2s/4s、Retry-After 優先）+
    **ETag 条件付きリクエストキャッシュ**（`cache/github/*.etag|body`、304 は GitHub レートクォータを消費しない）；403+`x-ratelimit-remaining: 0`
    を主レート制限として識別し、リトライしない。
  - **`.yxpkg` パッケージ形式**（4b）：tar.gz + `SHA256SUMS`
    マニフェスト（coreutils 二重スペース形式）、パッケージング決定性（エントリソート、mtime/uid/gid ゼロ化）；解凍は強制検証（マニフェスト欠如/改竄/マニフェスト外ファイル/パスエスケープ/解凍総量超過は一律エラー）；コンテンツ総量 20
    MiB 上限（決議 7）。除外項目はブラックリスト（`.git`/`.yaoxiang`/`target`/`node_modules`/`*.yxpkg`
    など）であり、ホワイトリストではない——`[exports]`
    で src/ 外のファイルをエクスポート面に含めることができる、ホワイトリストは静かに漏らす。
  - **`publish`**（4c）：裸の `publish` はエラーを報告し指示（Registry 後続）；`--dry-run`
    は「検証（description 必須）→ パッケージング → SHA-256」を完了；`--github`
    は続いて Release 重複チェック →
    tag 存在検証（Cargo 同型セマンティクス：tag を打つのはユーザーの責任）→
    Release 作成 → アセットアップロード。対象リポジトリは `[package].repository`
    を取得、`git remote origin` にフォールバック；認証は `$YX_GITHUB_TOKEN`
    を読み取る（credentials.toml は公式 Registry 着地と共に）。HTTP スタックは reqwest（rustls）+ パッケージ管理独自の tokio
    current_thread ランタイム（`package::runtime::drive`）、`futures` 依存は随之削除。
  - 公開前テスト実行（014a 検証チェックリストステップ 3）は Phase 5 接続と共に（03929ffa）。
- Phase 5 実装説明（2026-09-30、feat/rfc014）：
  - **インストール決定木接続**（b6a5b98f 前後複数コミット）：`install/update`
    で依存関係ダウンロード後、`[build]`/`[binaries]` を持つパッケージは `build::run_install_build`
    を経由——事前コンパイル優先（パッケージ全体 SHA-256 + 安全な解凍）→
    headers（026b 前に明確にエラー）→ 戦略実行（cargo 真実装 / cmake 未実装 /
    custom 信頼ゲート）。ビルド宣言のないパッケージはゼロコストで直行。
  - **cargo 戦略**：`[build.cargo]`
    でコマンド + プラットフォームオーバーライドをマージ；スクラッチは `CARGO_TARGET_DIR` を介して
    `.yaoxiang/build/` に隔離、FFI 成果物は vendor `build/native/<triple>/`
    にコピー；vendor 整合性セマンティクスはソースツリー整合性として明確化（`build/`
    はチェックサムに含まない）。
  - **信頼ゲート**（014b 決議 1）：信頼記録はユーザー設定 `[trust] build-scripts`；`--trust`
    で許可すれば永続化；非インタラクティブ環境はデフォルトで拒否。
  - publish 公開前テストはデフォルトで実行（RFC-036 発見メカニズム）、`--no-test` でスキップ。
  - cmake 実行と yx-bindgen ジェネレータ（RFC-026b）は後続；詳細は 014b 実装説明参照。

- グローバルキャッシュはまず
  **git チャネル**をカバー（`cache/git/<url>-<tag|rev|commit>/`、ブランチは `ls-remote`
  で commit を解決してキャッシュに入れ、オフラインフォールバック用ポインタファイルを書き込む）；`cache/registry/`、`cache/binaries/`
  はディレクトリ予約。vendor コピーは `.git`
  を除去、ディレクトリ名は依存関係 manifest で検出された実バージョンで命名（vendor/lock/クリアランスは同源）。
- `semver` と `sha2` crate は依存関係表に従って手書き実装を置き換え；`is_compatible`
  は 10 万回列挙から区間交差判定に変更。
- CLI に `outdated` / `clean` / `cache clean` を追加し、`add` に `--git` / `--path`
  明示ソースを追加（前述制約の実装）。`clean` は `.yaoxiang/build/`
  以外に、lock で参照されていない vendor の残留パッケージも裁断。
- キャッシュ `[cache] dir` 設定は既存の `~/.config/yaoxiang/config.toml`
  ユーザー設定体系に統合（RFC 起草時の `~/.yaoxiang/config.toml`
  は存在しなかった）、キャッシュデータのデフォルト位置は引き続き `~/.yaoxiang/cache`。

### 依存関係

- 前提依存なし
- `ModuleGraph`（`middle/passes/module/`）と統合する必要がある

### リスク

| リスク                         | 緩和策                                           |
| ------------------------------ | ------------------------------------------------ |
| 依存関係解決アルゴリズムが複雑 | まず簡単なバージョンを実装し、後で競合検出を追加 |
| Git ダウンロードが不安定       | リトライとキャッシュメカニズム                   |
| パフォーマンス問題             | 遅延読み込み、増分解析                           |

## 未解決の問題

- [x] `dev-dependencies` 条件コンパイル構文？→ RFC-014b ビルドシステムで統一処理
- [x] 整合性検証アルゴリズム（SHA-256 / BLAKE3）？→ SHA-256
- [x] パッケージ命名規則（namespace サポート、例：`@org/pkg`）？→ 初期はフラットパッケージ名、namespace なし；`@org/pkg`
      は予約（2026-09-15 決議）
- [x] Registry API バージョニング戦略？→ URL パス
      `/api/v1/` + レスポンスヘッダでプロトコルバージョンを伝達、破壊的変更は v2 にアップグレードし共存（2026-09-15 決議、RFC-014a 参照）
- [ ] `excludes` で特定ファイルのダウンロードを除外する？

---

## 依存関係（Cargo.toml に新規追加）

| 用途              | crate            | 説明                     |
| ----------------- | ---------------- | ------------------------ |
| 意味的バージョン  | `semver`         | 手書きパーサーを置き換え |
| HTTP クライアント | `reqwest`        | Registry 通信（Phase 4） |
| SHA-256           | `sha2`           | 整合性検証               |
| 圧縮              | `flate2` + `tar` | パッケージ形式処理       |

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
