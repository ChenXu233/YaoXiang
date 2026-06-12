---
title: "RFC-014c: ワークスペースサポート"
status: "草案"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014c: ワークスペースサポート

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC です。

## 概要

YaoXiang のワークスペース（workspace）メカニズムを定義する：複数の関連パッケージを同時に開発する際の依存関係の共有、パス参照、lockfile の統一、Cargo workspace との統合。

## 動機

プロジェクトの規模が大きくなると、コードを複数のパッケージに分割する必要がある。これらのパッケージには以下が求められる：
- 相互参照（パス依存）
- 外部依存バージョンの共有（バージョンドリフトの回避）
- lockfile の統一（ビルドの一貫性保証）
- Cargo workspace との連携（FFI 部分）

### 現状の問題

- プロジェクトごとに依存関係を独立して管理しており、共有できない
- パス依存の公開時自動置換メカニズムがない
- Cargo workspace との統合がない

## 提案

### 中核設計：調整層 + 自己完結メンバー

ルート workspace は調整のみを行い、各メンバーは完全に自己完結している。

### ルート yaoxiang.toml

```toml
# ルート yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**ルート toml は次の 3 つのことのみを行う：**
1. メンバーリストを宣言する（辞書形式、key はメンバー名、value は toml パス）
2. 共有 lockfile を提供する（`yaoxiang.lock`）
3. 共有 vendor ディレクトリを提供する（`.yaoxiang/vendor/`）

**ルート toml は dependencies を定義しない。** 各メンバーの依存関係は自身の `yaoxiang.toml` に記述する。

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
- 解決時に全メンバーの依存関係をマージし、共有 lockfile を生成する
- バージョン衝突は lockfile 生成時にエラーを報告する
- 同じパッケージは異なるメンバー間で同じバージョンに解決されなければならない

### パス依存と公開

開発時はワークスペース参照を使用：

```toml
[dependencies]
utils = { workspace = "utils" }
```

公開時に自動的にバージョン依存に置換される：

```toml
[dependencies]
utils = "^0.2.0"
```

パッケージマネージャーは `yaoxiang publish` 時にこの置換を自動的に行う。

### Cargo Workspace との統合

ワークスペース内に FFI パッケージがある場合、Cargo workspace も同時に定義できる：

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
| `yaoxiang workspace list` | ワークスペースメンバーをリスト表示 |
| `yaoxiang workspace add <path>` | メンバーを追加 |
| `yaoxiang workspace remove <name>` | メンバーを削除 |
| `yaoxiang build` | 全メンバーをビルド |
| `yaoxiang build core` | 指定メンバーをビルド |
| `yaoxiang test` | 全メンバーのテストを実行 |

## 詳細設計

### WorkspaceManifest 構造

```rust
struct WorkspaceManifest {
    members: BTreeMap<String, String>,  // name -> toml path
}

struct Workspace {
    root: PathBuf,
    manifest: WorkspaceManifest,
    members: Vec<WorkspaceMember>,
    lock: LockFile,
}

struct WorkspaceMember {
    name: String,
    root: PathBuf,
    manifest: PackageManifest,
}
```

### workspace 依存参照

`{ workspace = "member-name" }` のセマンティクス：
- `dependencies` 内で他のワークスペースメンバーを参照する
- 開発時はローカルパスに解決される
- 公開時は Registry バージョンに置換される
- メンバー名は `[workspace.members]` 内に存在しなければならない

### lockfile 共有

- ワークスペースには `yaoxiang.lock` が 1 つだけ存在する（ルートディレクトリ内）
- 全メンバーの依存関係解決は同じ lockfile にマージされる
- バージョン衝突は lockfile 生成時にエラーを報告し、衝突元の情報を付随する

## トレードオフ

### 利点

- マルチパッケージプロジェクトを統一管理
- 共有 lockfile が一貫性を保証
- パス依存の開発体験が良好
- Cargo workspace とシームレスに統合

### 欠点

- 全メンバーが同じ外部依存バージョンを使用しなければならない（過度に厳格な可能性がある）
- ルート toml は自身の依存関係を持てない（設計上の制約）
- Cargo workspace 統合が複雑さを増す

## 代替案

| 案 | 採用しなかった理由 |
|------|-----------|
| 独立プロジェクト + path 依存 | lockfile が統一されず、バージョンドリフトのリスクあり |
| npm workspaces 風 | npm の workspace は問題が多く、模倣する価値がない |
| Cargo workspace の直接再利用 | YaoXiang と Cargo は異なるパッケージエコシステム |

## 実装戦略

### フェーズ分割

| フェーズ | 内容 |
|------|------|
| Phase 6a | `[workspace.members]` 解析 + WorkspaceManifest |
| Phase 6b | 共有 lockfile + 依存関係マージ解決 |
| Phase 6c | `{ workspace = "name" }` パス依存参照 |
| Phase 6d | 公開時のパス依存自動置換 |
| Phase 6e | Cargo workspace 統合 |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ）に依存
- オプションで RFC-014b（ビルドシステム、native メンバー用）に依存

## 未解決問題

- [ ] メンバー間の循環依存を許可するか？
- [ ] workspace レベルの `[build]` 設定をサポートするか？
- [ ] メンバーが独自の lockfile（ルート lockfile の上書き）を持てるか？
- [ ] ネストされた workspace をサポートするか？

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)