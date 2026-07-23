---
title: "RFC-014c: ワークスペースサポート"
status: "審査中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
issue: "#113"
---

# RFC-014c: ワークスペースサポート

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC です。

## 概要

YaoXiang のワークスペース（workspace）メカニズムを定義します。複数の関連パッケージを一緒に開発する際の、依存関係の共有、パス参照、lockfile の統一、Cargo workspace との統合を含みます。

## 動機

プロジェクトの規模が拡大すると、コードを複数のパッケージに分割する必要があります。これらのパッケージには以下が必要です：

- 相互参照（パス依存関係）
- 外部依存関係のバージョンの共有（バージョンドリフトの回避）
- 統一された lockfile（ビルド一貫性の保証）
- Cargo workspace との協調（FFI 部分）

### 現在の問題

- 各プロジェクトが独立して依存関係を管理しており、共有できない
- パス依存関係の公開時の自動置換メカニズムがない
- Cargo workspace との統合がない

## 提案

### コアデザイン：調整層 + 自己完結型メンバー

ルートワークスペースは調整のみを行い、各メンバーは完全に自己完結型です。

### ルート yaoxiang.toml

```toml
# ルート yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**ルート toml は3つのことだけを行います：**

1. メンバーリストの宣言（辞書形式、key がメンバー名、value が toml パス）
2. 共有 lockfile の提供（`yaoxiang.lock`）
3. 共有 vendor ディレクトリの提供（`.yaoxiang/vendor/`）

**ルート toml は dependencies を定義しません。** 各メンバーの依存関係はそれぞれの `yaoxiang.toml` に書きます。

### メンバー yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # ワークスペースメンバーの参照
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
└── Cargo.toml                 # 任意：共有 Cargo workspace（FFI）
```

### 依存関係解決

- 各メンバーは自分の `[dependencies]` を読み取る
- 解決時にすべてのメンバーの依存関係をマージし、共有 lockfile を生成する
- バージョンの競合は lockfile 生成時にエラーとして報告される
- 同じパッケージが複数のメンバーで異なるバージョンに解決されてはならない

### workspace 依存関係参照

`{ workspace = "member-name" }` は `[workspace.members]` の **key** を参照します（メンバーの `[package].name` ではありません）。

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
# packages/utils/yaoxiang.toml 内で name = "my-utils" と書いてあってもOK
```

**key 而不是 name の理由：**

- key はワークスペースが制御し、安定して一意
- `[package].name` は公開名で、公開時に変更される可能性がある
- key は BTreeMap の key であり、本質的に一意
- 公開時に workspace 参照はバージョンの依存関係に置き換えられ、key は公開 API に漏洩しない

### パス依存関係と公開

開発時はワークスペース参照を使用：

```toml
[dependencies]
utils = { workspace = "utils" }
```

公開時は自動的にバージョン依存関係に置き換え：

```toml
[dependencies]
utils = "^0.2.0"
```

**バージョンのソース：** 依存されるメンバーの `[package].version` を読んで、`^` 接頭辞を付加する。Registry はチェックしない——バージョンの権威あるソースはメンバーの `yaoxiang.toml` であり、Registry は配布渠道に過ぎない。

パッケージマネージャーは `yaoxiang publish` 時にこの置換を自動的に行う。

### Cargo Workspace との統合

ワークスペースに FFI パッケージがある場合、同時に Cargo workspace を定義できます：

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

`yaoxiang build` は自動的に検出して `cargo build` を呼び出し、native 部分をコンパイルする。

### CLI コマンド

| コマンド | 機能 |
|------|------|
| `yaoxiang workspace list` | ワークスペースメンバーの一覧表示 |
| `yaoxiang workspace add <path>` | メンバーの追加 |
| `yaoxiang workspace remove <name>` | メンバーの削除 |
| `yaoxiang build` | すべてのメンバーをビルド（依存関係のトポロジカルソート順） |
| `yaoxiang build core` | 指定メンバーのビルド |
| `yaoxiang test` | すべてのメンバーのテストを実行 |

**`yaoxiang build` の動作：** すべてのメンバーをビルドし、依存関係のトポロジカルソート順で行う。core → utils → app の場合、ビルド順序は core → utils → app となる。

## 詳細設計

### WorkspaceManifest 構造

ルート toml は独立した `WorkspaceManifest` 型を使用し、`PackageManifest` を再利用しない：

```rust
struct WorkspaceManifest {
    workspace: WorkspaceConfig,
}

struct WorkspaceConfig {
    members: BTreeMap<String, String>,  // key -> toml path
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

**検出ロジック：** toml をロードする際、`[workspace]` セクションがある場合は `WorkspaceManifest` として解析し、なければ `PackageManifest` として解析する。

### workspace 依存関係参照

`{ workspace = "member-name" }` のセマンティクス：

- `dependencies` 内で別のワークスペースメンバーを参照する
- 開発時はローカルパスとして解決
- 公開時は Registry バージョンに置き換え
- メンバー名は `[workspace.members]` に存在しなければならない

### lockfile 共有

- ワークスペースには `yaoxiang.lock` が1つだけ（ルートディレクトリにある）
- すべてのメンバーの依存関係解決が同じ lockfile にマージされる
- バージョンの競合は lockfile 生成時にエラーとして報告され、競合元情報が含まれる

## トレードオフ

### メリット

- マルチパッケージプロジェクトを一元管理
- 共有 lockfile が一貫性を保証
- パス依存関係で開発体験が良好
- Cargo workspace とシームレスに統合

### デメリット

- すべてのメンバーが同じ外部依存関係バージョンを使わなければならない（厳しすぎる可能性）
- ルート toml は独自の依存関係を持てない（設計上の制約）
- Cargo workspace 統合が複雑さを増す

## 代替案

| 方案 | 選擇しない理由 |
|------|-----------|
| 独立プロジェクト + path 依存関係 | lockfile が統一されず、バージョンドリフトのリスクがある |
| npm workspaces のようなもの | npm の workspace 問題が多く、模仿する価値がない |
| Cargo workspace を直接再利用 | YaoXiang と Cargo は異なるパッケージエコシステム |

## 実装戦略

### フェーズ分け

| フェーズ | 内容 |
|------|------|
| Phase 6a | `[workspace.members]` の解析 + WorkspaceManifest |
| Phase 6b | 共有 lockfile + 依存関係のマージ解決 |
| Phase 6c | `{ workspace = "name" }` パス依存関係参照 |
| Phase 6d | 公開時のパス依存関係の自動置き換え |
| Phase 6e | Cargo workspace 統合 |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ）に依存
- 任意で RFC-014b（ビルドシステム、native メンバー用）に依存

## 開放問題

- [ ] メンバー間の循環依存を許可するか？
- [ ] workspace レベルの `[build]` 設定をサポートするか？
- [ ] メンバーは独自の lockfile を持てるか（ルートの lockfile を上書き）？
- [ ] ネストされた workspace をサポートするか？

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)