---
title: "RFC-014c: ワークスペースサポート"
status: "審査中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014c: ワークスペースサポート

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC である。

## 概要

YaoXiang のワークスペース（workspace）機構を定義する。複数の関連パッケージを同時に開発する際の依存共有、パス参照、lockfile 統一、Cargo workspace との統合を扱う。

## 動機

プロジェクトの規模が拡大すると、コードを複数のパッケージに分割する必要がある。これらのパッケージには以下が求められる：

- 相互参照（パス依存）
- 外部依存バージョンの共有（バージョンドリフトの防止）
- lockfile の統一（ビルド一貫性の保証）
- Cargo workspace との協調（FFI 部分）

### 現状の問題

- 各プロジェクトが独立して依存関係を管理しており、共有できない
- パス依存の公開時自動置換機構がない
- Cargo workspace との統合がない

## 提案

### 中核設計：調整層 + 自己完結メンバー

ルート workspace は調整のみを行い、各メンバーは完全に自己完結する。

### ルート yaoxiang.toml

```toml
# ルート yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**ルート toml は以下の 3 つのことだけを行う：**
1. メンバーリストの宣言（辞書形式、key はメンバー名、value は toml パス）
2. 共有 lockfile の提供（`yaoxiang.lock`）
3. 共有 vendor ディレクトリの提供（`.yaoxiang/vendor/`）

**ルート toml は dependencies を定義しない。** 各メンバーの依存はそれぞれの `yaoxiang.toml` に記述する。

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

### 依存解決

- 各メンバーは自分の `[dependencies]` を読み取る
- 解決時に全メンバーの依存をマージし、共有 lockfile を 1 つ生成する
- バージョン競合は lockfile 生成時にエラーとなる
- 同一パッケージは異なるメンバー間で必ず同じバージョンに解決されなければならない

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

**なぜ key を使うのか：**
- key はワークスペースが管理し、安定して一意である
- `[package].name` は公開名であり、公開時に変わり得る
- key は BTreeMap のキーとして本質的に一意である
- 公開時に workspace 参照はバージョン依存に置換され、key は公開 API に漏れない

### パス依存と公開

開発時はワークスペース参照を使用：

```toml
[dependencies]
utils = { workspace = "utils" }
```

公開時は自動的にバージョン依存に置換される：

```toml
[dependencies]
utils = "^0.2.0"
```

**バージョン取得元：** 参照されるメンバーの `[package].version` を読み取り、`^` プレフィックスを付加する。Registry は確認しない——バージョンの信頼源はメンバーの `yaoxiang.toml` であり、Registry は単なる配信チャネルである。

パッケージマネージャは `yaoxiang publish` 実行時にこの置換を自動的に行う。

### Cargo Workspace との統合

ワークスペース内に FFI パッケージがある場合、Cargo workspace を同時に定義できる：

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

`yaoxiang build` は自動的に検出し、`cargo build` を呼び出して native 部分をコンパイルする。

### CLI コマンド

| コマンド | 機能 |
|------|------|
| `yaoxiang workspace list` | ワークスペースメンバーを列挙する |
| `yaoxiang workspace add <path>` | メンバーを追加する |
| `yaoxiang workspace remove <name>` | メンバーを削除する |
| `yaoxiang build` | 全メンバーをビルドする（依存トポロジー順） |
| `yaoxiang build core` | 指定メンバーのみをビルドする |
| `yaoxiang test` | 全メンバーのテストを実行する |

**`yaoxiang build` の動作：** 全メンバーを依存トポロジー順にビルドする。core → utils → app の場合、ビルド順は core → utils → app となる。

## 詳細設計

### WorkspaceManifest 構造

ルート toml は `PackageManifest` を再利用せず、独立した `WorkspaceManifest` 型を使用する：

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

**検出ロジック：** toml 読み込み時に `[workspace]` セクションがあれば `WorkspaceManifest` として解析し、なければ `PackageManifest` として解析する。

### workspace 依存参照

`{ workspace = "member-name" }` のセマンティクス：
- `dependencies` 内で他のワークスペースメンバーを参照する
- 開発時はローカルパスに解決される
- 公開時は Registry バージョンに置換される
- メンバー名は `[workspace.members]` に存在しなければならない

### lockfile 共有

- ワークスペースには `yaoxiang.lock` が 1 つだけ存在する（ルートディレクトリ）
- 全メンバーの依存解決が同一 lockfile にマージされる
- バージョン競合は lockfile 生成時にエラーとなり、競合発生元の情報を付与する

## トレードオフ

### 利点

- マルチパッケージプロジェクトの統一管理
- 共有 lockfile による一貫性保証
- パス依存による優れた開発体験
- Cargo workspace とのシームレスな統合

### 欠点

- 全メンバーが同一の外部依存バージョンを使わなければならない（過度に厳格かもしれない）
- ルート toml は自身の依存を持てない（設計上の制約）
- Cargo workspace 統合が複雑さを増す

## 代替案

| 案 | 採用しなかった理由 |
|------|-----------|
| 独立プロジェクト + path 依存 | lockfile が統一されず、バージョンドリフトのリスクがある |
| npm workspaces 風 | npm の workspace には問題が多く、模倣する価値がない |
| Cargo workspace の直接再利用 | YaoXiang と Cargo は異なるパッケージエコシステムである |

## 実装戦略

### フェーズ分割

| フェーズ | 内容 |
|------|------|
| Phase 6a | `[workspace.members]` 解析 + WorkspaceManifest |
| Phase 6b | 共有 lockfile + 依存マージ解決 |
| Phase 6c | `{ workspace = "name" }` パス依存参照 |
| Phase 6d | 公開時のパス依存自動置換 |
| Phase 6e | Cargo workspace 統合 |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ）に依存
- 任意で RFC-014b（ビルドシステム、native メンバー用）に依存

## オープンな問題

- [ ] メンバー間の循環依存を許可するか？
- [ ] workspace レベルの `[build]` 設定をサポートするか？
- [ ] メンバーが独自の lockfile を持てるか（ルート lockfile を上書き）？
- [ ] ネストした workspace をサポートするか？

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)