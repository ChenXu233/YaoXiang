```markdown
---
title: "RFC-014c: ワークスペースサポート"
status: "レビュー中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
issue: "#113"
---

# RFC-014c: ワークスペースサポート

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブレポートです。

## 概要

YaoXiang のワークスペース（workspace）機構を定義する。複数の関連パッケージを同時に開発する際の依存関係共有、パス参照、lockfile 統一、Cargo workspace との統合について述べる。

## 動機

プロジェクトの規模が大きくなると、コードを複数のパッケージに分割する必要が出てくる。これらのパッケージには以下が求められる。

- 相互参照（パス依存）
- 外部依存バージョンの共有（バージョンドリフトの防止）
- lockfile の統一（ビルド一貫性の保証）
- Cargo workspace との協調（FFI 部分）

### 現状の問題点

- 各プロジェクトが独立して依存関係を管理し、共有できない
- パス依存には公開時の自動置換機構がない
- Cargo workspace との統合がない

## 提案

### 中核設計：調整レイヤー + 自己完結メンバー

ルート workspace は調整のみを行い、各メンバーは完全に自己完結している。

### ルート yaoxiang.toml

```toml
# ルート yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**ルート toml は次の 3 つのことのみを行う。**

1. メンバーリストの宣言（辞書形式、key がメンバー名、value が toml パス）
2. 共有 lockfile の提供（`yaoxiang.lock`）
3. 共有 vendor ディレクトリの提供（`.yaoxiang/vendor/``）

**ルート toml は `dependencies` を定義しない。** 各メンバーの依存関係は自身の `yaoxiang.toml` に記述する。

### メンバー yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # ワークスペースメンバーを参照
regex = "^1.0.0"
```

```toml
# packages/utils/yaoxiang.toml
[package]
name = "utils"
version = "0.2.0"

[dependencies]
regex = "^1.0.0"
```

### ワークスペース構造

```
my-workspace/
├── yaoxiang.toml              # ワークスペースルート設定
├── yaoxiang.lock              # 共有 lockfile
├── .yaoxiang/
│   └── vendor/                # 共有 vendor ディレクトリ
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # メンバーパッケージ設定
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # オプション：共有 Cargo workspace（FFI）
```

### 依存関係解決

- 各メンバーは自身の `[dependencies]` を読む
- 解決時に全メンバーの依存関係をマージし、単一の共有 lockfile を生成する
- バージョン競合は lockfile 生成時にエラーとなる
- 同一パッケージは異なるメンバーでも同じバージョンに解決されなければならない

### workspace 依存参照

`{ workspace = "member-name" }` は `[workspace.members]` の **key** を参照する（メンバーの `[package].name` ではない）。

```toml
# ルート yaoxiang.toml
[workspace.members]
utils = "packages/utils/yaoxiang.toml"    # key = "utils"
```

```toml
# packages/app/yaoxiang.toml
[package]
name = "app"

[dependencies]
utils = { workspace = "utils" }   # ✅ key "utils" を参照
# packages/utils/yaoxiang.toml 内で name = "my-utils" と書かれていても問題ない
```

**なぜ key ではなく name を使うのか：**
- key はワークスペースが制御するため、安定的で一意
- `[package].name` は公開名であり、公開時に変わる可能性がある
- key は BTreeMap の key として本質的に一意
- 公開時に workspace 参照はバージョン依存に置き換えられ、key は公開 API には漏れない

### パス依存と公開

開発時はワークスペース参照を用いる。

```toml
[dependencies]
utils = { workspace = "utils" }
```

公開時には自動的にバージョン依存へ置換される。

```toml
[dependencies]
utils = "^0.2.0"
```

**バージョン取得元：** 参照先メンバーの `[package].version` を読み取り、`^` 接頭辞を付与する。Registry は確認しない——バージョンの権威ある情報源はメンバーの `yaoxiang.toml` であり、Registry は単なる配信チャネルである。

パッケージマネージャは `yaoxiang publish` 時にこの置換を自動的に行う。

### Cargo Workspace との統合

ワークスペース内に FFI パッケージがある場合、Cargo workspace を同時に定義できる。

```toml
# ルート Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace（FFI 部分）
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # YaoXiang コード
│   │   └── native/
│   │       ├── Cargo.toml # Rust FFI コード
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` はネイティブ部分を自動的に検出し、`cargo build` の呼び出しを行う。

### CLI コマンド

| コマンド | 機能 |
|------|------|
| `yaoxiang workspace list` | ワークスペースメンバーを一覧表示 |
| `yaoxiang workspace add <path>` | メンバーを追加 |
| `yaoxiang workspace remove <name>` | メンバーを削除 |
| `yaoxiang build` | 全メンバーを構築（依存トポロジ順） |
| `yaoxiang build core` | 指定メンバーのみ構築 |
| `yaoxiang test` | 全メンバーのテストを実行 |

**`yaoxiang build` の挙動：** 全メンバーを依存トポロジ順に構築する。core → utils → app のような関係であれば、構築順序は core → utils → app となる。

## 詳細設計

### WorkspaceManifest 構造

ルート toml は独立した `WorkspaceManifest` 型を使用し、`PackageManifest` を再利用しない。

```rust
struct WorkspaceManifest {
    workspace: WorkspaceConfig,
}

struct WorkspaceConfig {
    members: BTreeMap<String, String>,  // key -> toml パス
}

struct Workspace {
    root: PathBuf,
    manifest: WorkspaceManifest,
    members: Vec<WorkspaceMember>,
    lock: LockFile,
}

struct WorkspaceMember {
    name: String,           // [workspace.members] の key
    root: PathBuf,
    manifest: PackageManifest,
}
```

**検出ロジック：** toml 読み込み時に `[workspace]` セクションがあれば `WorkspaceManifest` として解析し、なければ `PackageManifest` として解析する。

### workspace 依存参照

`{ workspace = "member-name" }` の意味：
- `dependencies` 内で他のワークスペースメンバーを参照する
- 開発時はローカルパスとして解決される
- 公開時は Registry のバージョンに置き換えられる
- メンバー名は `[workspace.members]` 内に必ず存在しなければならない

### lockfile 共有

- ワークスペースには `yaoxiang.lock` が 1 つだけ存在する（ルートディレクトリ内）
- 全メンバーの依存関係解決は同一の lockfile にマージされる
- バージョン競合は lockfile 生成時にエラーとなり、競合発生元の情報を付与する

## トレードオフ

### 利点

- マルチパッケージプロジェクトの統一管理
- 共有 lockfile による一貫性保証
- パス依存による優れた開発体験
- Cargo workspace とのシームレスな統合

### 欠点

- 全メンバーが同一の外部依存バージョンを使う必要がある（厳しすぎる可能性がある）
- ルート toml は自身の依存関係を持てない（設計上の制約）
- Cargo workspace 統合が複雑性を増す

## 代替案

| 案 | 不採用理由 |
|------|-----------|
| 独立プロジェクト + path 依存 | lockfile が統一されず、バージョンドリフトのリスクがある |
| npm workspaces 風 | npm の workspace には問題が多く、模倣する価値がない |
| Cargo workspace の直接再利用 | YaoXiang と Cargo は異なるパッケージエコシステムである |

## 実装戦略

### 段階分け

| 段階 | 内容 |
|------|------|
| Phase 6a | `[workspace.members]` 解析 + WorkspaceManifest |
| Phase 6b | 共有 lockfile + 依存関係マージ解決 |
| Phase 6c | `{ workspace = "name" }` パス依存参照 |
| Phase 6d | 公開時のパス依存自動置換 |
| Phase 6e | Cargo workspace 統合 |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ）に依存
- RFC-014b（ビルドシステム、ネイティブメンバー用）にオプションで依存

## 未解決問題

- [ ] メンバー間の循環依存を許可するか？
- [ ] workspace レベルの `[build]` 設定をサポートするか？
- [ ] メンバーが独自の lockfile（ルート lockfile を上書き）を持てるか？
- [ ] ネストされた workspace をサポートするか？

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
```