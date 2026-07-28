---
title: 'RFC-014: パッケージ管理システム設計'
status: '承認済み'
author: '晨煦'
created: '2026-02-12'
updated: '2026-06-11'
group: 'rfc-014' # 本 RFC はパッケージ管理システムの総綱であり、サブ RFC：014a/014b/014c
issue: '#88'
impl: '48%'
impl_status: '部分的'
---

# RFC-014: パッケージ管理システム設計（総綱）

> **サブ RFC：**
>
> - [RFC-014a: Registry プロトコル仕様](../draft/014a-registry-protocol.md)
> - [RFC-014b: ビルドシステムとバイナリ配布](../draft/014b-build-system.md)
> - [RFC-014c: ワークスペースサポート](../draft/014c-workspace.md)

## 概要

YaoXiang 言語のパッケージ管理システムを設計し、セマンティックバージョニング、ローカルおよび GitHub 依存関係、統一インポート構文、`yaoxiang.toml`
設定ファイル、および `yaoxiang.lock` ロックファイルをサポートする。

## 動機

### なぜこの機能/変更が必要なのか？

パッケージ管理は現代プログラミング言語エコシステムのインフラである。現在の YaoXiang 言語には以下が欠けている：

- 依存関係宣言メカニズム
- バージョン管理能力
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

**階層型アーキテクチャ**：

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

**拡張メカニズム**：新しい Source 型を追加するには trait を実装するだけで、解析エンジンを変更する必要はない。

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

解決順序は作業モード（`yaoxiang.toml` の有無）によって決まる。

#### プロジェクトモード（yaoxiang.toml あり）

```
use foo.bar.baz;

查找顺序:
0. 嵌入二进制                          (std/*.yx — 编译时嵌入，版本绑定，仅 std.* 命名空间)
1. ./.yaoxiang/std/foo/bar/baz.yx     (项目级标准库 — 存在时全局标准库完全失效)
2. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
3. ./src/foo/bar/baz.yx                       (本地模块)
4. ~/.yaoxiang/cache/foo/<ver>/src/foo/bar/baz.yx  (全局缓存)
5. $YXPATH/foo/bar/baz.yx                     (全局路径，预留)
```

**プロジェクトモードルール**：

- 埋め込みバイナリは `std.*` 名前空間に対してのみ有効で、優先度は最も高い
- プロジェクトレベル標準ライブラリ（`.yaoxiang/std/`）が存在する場合、グローバル標準ライブラリは完全にスキップされる — ビルドの決定性を保証する
- プロジェクトは `yaoxiang add std@1.0.1`
  で標準ライブラリを依存関係として管理し、バージョンを固定できる

#### 単一ファイルモード（yaoxiang.toml なし）

```
use foo.bar.baz;

查找顺序:
0. 嵌入二进制                          (std/*.yx — 编译时嵌入，版本绑定)
1. <yaoxiang-install-dir>/yx/<version>/std/foo/bar/baz.yx  (全局标准库)
2. ./src/foo/bar/baz.yx                                       (本地模块)
3. $YXPATH/foo/bar/baz.yx                                     (全局路径，预留)
```

**単一ファイルモードルール**：

- プロジェクトレベルの依存関係はなく、標準ライブラリはグローバルパスから直接ロードされる
- グローバル標準ライブラリパスはコンパイラバージョンに固定される：`<install-dir>/yx/<version>/std/`

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

プロジェクトは `yaoxiang add std@1.0.1`
で標準ライブラリを依存関係としてプロジェクトに追加し、`.yaoxiang/std/` に保存できる：

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── .yaoxiang/
│   ├── std/                     # 项目级标准库（存在时全局标准库失效）
│   │   ├── test.yx
│   │   ├── math.yx
│   │   └── ...
│   └── vendor/                  # 其他依赖
│       └── ...
├── src/
│   └── main.yx
```

**設計のポイント**：

- 埋め込みバイナリは互換レイヤーとして機能する：ファイルシステム標準ライブラリが完全に実装されるまで、まず埋め込みバイナリを介して標準ライブラリモジュールを提供する
- バージョンディレクトリ分離：`yx/<version>/std/`
  により、異なるバージョンの標準ライブラリが共存し、相互に影響しない
- プロジェクトレベル標準ライブラリがグローバル標準ライブラリを上書きする：グローバル環境の変化に影響されないビルドの決定性を保証する
- yaoxiang.toml がない場合（単一ファイルモード）、グローバル標準ライブラリにフォールバックする
- `.yaoxiang/std/`
  の存在は「プロジェクトレベル標準ライブラリが有効」を意味し、グローバル標準ライブラリはもはや参加しない

### コアデータ構造

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

// 解析后的依赖
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
    checksum: Option<String>,  // SHA-256
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

統一スキームを採用：コンパイラ、パッケージマネージャー、REPL を単一の CLI ツールに統合する：

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
| `yaoxiang` (引数なし)   | ✅           | ✅                 | 直接 REPL に入る                              |

#### コマンド詳細

| コマンド                           | 機能                                 | 例                                                   |
| ---------------------------------- | ------------------------------------ | ---------------------------------------------------- |
| `yaoxiang`                         | 直接 REPL に入る                     | `yaoxiang`                                           |
| `yaoxiang run <file>`              | 単一ファイル/プロジェクトを実行      | `yaoxiang run main.yx`                               |
| `yaoxiang init <name>`             | 新しいプロジェクトを作成             | `yaoxiang init my-app`                               |
| `yaoxiang build`                   | プロジェクトをビルド                 | `yaoxiang build`                                     |
| `yaoxiang build <file>`            | 単一ファイルをビルド                 | `yaoxiang build foo.yx`                              |
| `yaoxiang add <dep>`               | 依存関係を追加                       | `yaoxiang add foo`                                   |
| `yaoxiang add -D <dep>`            | 開発依存関係を追加                   | `yaoxiang add -D test`                               |
| `yaoxiang rm <dep>`                | 依存関係を削除                       | `yaoxiang rm foo`                                    |
| `yaoxiang update`                  | すべての依存関係を更新               | `yaoxiang update`                                    |
| `yaoxiang update foo`              | 指定された依存関係を更新             | `yaoxiang update foo`                                |
| `yaoxiang install`                 | すべての依存関係をインストール       | `yaoxiang install`                                   |
| `yaoxiang list`                    | 依存関係をリスト                     | `yaoxiang list`                                      |
| `yaoxiang outdated`                | 古い依存関係を確認                   | `yaoxiang outdated`                                  |
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

- ✅ 既存の `use` 構文は完全に保持される
- ✅ 既存のモジュール解決ロジックは変更されない
- ✅ 新しい .yaoxiang/vendor ディレクトリは既存のプロジェクトに影響しない

### グローバルキャッシュ

ダウンロードされたすべての依存関係は `~/.yaoxiang/cache/`
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
- Token は `yaoxiang.toml` や `yaoxiang.lock` には決して書き込まれない
- ファイル権限 600

### yank セマンティクス

`yaoxiang yank foo@1.2.3` は**削除 + バージョン番号の永久ロック**を実行する：

- パッケージは完全に削除され、復元できない
- バージョン番号は永久に占有され、同じバージョン番号を再公開できない
- 既に lockfile がそのバージョンを参照しているプロジェクトはエラーになり、アップグレードが必要
- **セキュリティ目的**：npm 型のサプライチェーン攻撃を防ぐ（攻撃者が削除されたバージョン番号を乗っ取って悪意のあるコードを注入する）

### Registry プロトコル

詳細は [RFC-014a: Registry プロトコル仕様](../draft/014a-registry-protocol.md) を参照。

コアドザイン：オープンプロトコル + アダプタ層。公式 Registry を主とし、GitHub
Release/main ブランチを補助とし、カスタム Registry をサポートする。

### ビルドシステム

詳細は [RFC-014b: ビルドシステムとバイナリ配布](../draft/014b-build-system.md) を参照。

コアドデザイン：宣言型 `[build]`
設定、プリコンパイル優先/ソースコードフォールバック、cargo/cmake/custom 戦略をサポート。

### ワークスペース

詳細は [RFC-014c: ワークスペースサポート](../draft/014c-workspace.md) を参照。

コアドザイン：辞書形式の members 宣言、共有 lockfile、パス依存関係、Cargo workspace 統合。

## トレードオフ

### 利点

- 統一インポート構文により、ユーザーは依存関係のソースを気にする必要がない
- 決定的なビルド、lock ファイルがビルドの一貫性を保証
- オフラインサポート、ローカルにダウンロード後はオフラインで開発可能
- Source trait は後続の拡張に便利

### 欠点

- 追加のストレージスペースが必要（.yaoxiang/vendor ディレクトリ）
- バージョンの競合はユーザーが手動で解決する必要がある

## 代替案

| 案                                     | 採用しなかった理由                                    |
| -------------------------------------- | ----------------------------------------------------- |
| リアルタイム GitHub アクセス           | セキュリティとキャッシュ再利用の保証が困難            |
| グローバルキャッシュ ($HOME/.yaoxiang) | 分離性が低く、バージョン競合が複雑                    |
| レジストリのみサポート                 | GitHub は現在主流のコードホスティングプラットフォーム |

## 実装戦略

### フェーズ分割

| フェーズ      | 内容                                                       | 状態    |
| ------------- | ---------------------------------------------------------- | ------- |
| **Phase 1**   | toml 解析、ローカル依存関係、lock 生成、基本アルゴリズム   | ✅ 完了 |
| **Phase 2**   | GitHub サポート、.yaoxiang/vendor 管理、ダウンロードツール | ✅ 完了 |
| **Phase 3**   | グローバルキャッシュ、semver crate 置換、CLI 改善          | 未開始  |
| **Phase 3.5** | Source trait を async に変更、async-trait 統合             | 未開始  |
| **Phase 4**   | Registry プロトコル、publish、auth（RFC-014a）             | 未開始  |
| **Phase 5**   | ビルドシステム、プリコンパイル済みバイナリ（RFC-014b）     | 未開始  |
| **Phase 6**   | ワークスペースサポート（RFC-014c）                         | 未開始  |

### 依存関係

- 前置依存関係なし
- `ModuleGraph`（`middle/passes/module/`）との統合が必要

### リスク

| リスク                         | 緩和策                                               |
| ------------------------------ | ---------------------------------------------------- |
| 依存関係解決アルゴリズムが複雑 | まずシンプルなバージョンを実装し、後で競合検出を追加 |
| Git ダウンロードが不安定       | リトライとキャッシュメカニズム                       |
| パフォーマンス問題             | 遅延ロード、増分解析                                 |

## 未解決の問題

- [x] `dev-dependencies` 条件付きコンパイル構文？→ RFC-014b ビルドシステムで統一処理
- [x] 完全性検証アルゴリズム（SHA-256 / BLAKE3）？→ SHA-256
- [ ] `excludes` で特定のファイルのダウンロードを除外する？
- [ ] パッケージ命名規則（namespace サポート、例えば `@org/pkg`）？
- [ ] Registry API のバージョニング戦略？

---

## 依存関係（Cargo.toml に追加が必要）

| 用途                         | crate            | 説明                       |
| ---------------------------- | ---------------- | -------------------------- |
| セマンティックバージョニング | `semver`         | 手書きパーサーを置換       |
| HTTP クライアント            | `reqwest`        | Registry 通信              |
| SHA-256                      | `sha2`           | 完全性検証                 |
| 圧縮                         | `flate2` + `tar` | パッケージフォーマット処理 |

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
