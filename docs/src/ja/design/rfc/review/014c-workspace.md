---
title: 'RFC-014c: ワークスペースサポート'
status: 'レビュー中'
author: '晨煦'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: ワークスペースサポート

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
> のサブ RFC です。

## 概要

YaoXiang のワークスペース（workspace）メカニズムを定義する。複数の関連パッケージを共同で開発する際の依存共有、パス参照、lockfile の統一、Cargo
workspace との統合を扱う。

## 動機

プロジェクトの規模が大きくなると、コードを複数のパッケージに分割する必要がある。これらのパッケージには以下が求められる：

- 相互参照（パス依存）
- 外部依存バージョンの共有（バージョンドリフトの防止）
- lockfile の統一（ビルドの一貫性確保）
- Cargo workspace との協調（FFI 部分）

### 現状の問題

- 各プロジェクトが独立して依存関係を管理しており、共有できない
- パス依存を公開時に自動置換する仕組みがない
- Cargo workspace との統合がない

## 提案

### 中核設計：調整レイヤー + 自己完結メンバー

ルート workspace は調整のみを行い、各メンバーは完全に自己完結する。

### ルート yaoxiang.toml

```toml
# 根 yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**ルート toml が行うのは 3 つだけ：**

1. メンバーリストを宣言する（辞書形式、key がメンバー名、value が toml パス）
2. 共有 lockfile（`yaoxiang.lock`）を提供する
3. 共有 vendor ディレクトリ（`.yaoxiang/vendor/`）を提供する

**ルート toml は dependencies を定義しない。** 各メンバーの依存関係は自身の `yaoxiang.toml`
に記述する。

### メンバー yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # 引用工作空间成员
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
├── yaoxiang.toml              # 工作空间根配置
├── yaoxiang.lock              # 共享 lockfile
├── .yaoxiang/
│   └── vendor/                # 共享 vendor 目录
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # 成员包配置
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # 可选：共享 Cargo workspace（FFI）
```

### 依存解決

- 各メンバーは自身の `[dependencies]` を読み込む
- 解決時に全メンバーの依存関係をマージし、単一の共有 lockfile を生成する
- バージョン競合は lockfile 生成時にエラーとなる
- 同じパッケージは異なるメンバーでも同じバージョンに解決されなければならない

### workspace 依存参照

`{ workspace = "member-name" }` は `[workspace.members]` の **key** を参照する（メンバーの
`[package].name` ではない）。

```toml
# 根 yaoxiang.toml
[workspace.members]
utils = "packages/utils/yaoxiang.toml"    # key = "utils"
```

```toml
# packages/app/yaoxiang.toml
[package]
name = "app"

[dependencies]
utils = { workspace = "utils" }   # ✅ 引用 key "utils"
# 即使 packages/utils/yaoxiang.toml 里写的是 name = "my-utils"
```

**なぜ key を使うのか（name ではなく）：**

- key はワークスペース側で管理され、安定かつ一意である
- `[package].name` は公開名であり、公開時に変更され得る
- key は BTreeMap の key であり、原理的に一意である
- 公開時に workspace 参照はバージョン依存に置換されるため、key は公開 API に漏れない

### パス依存と公開

開発時は workspace 参照を用いる：

```toml
[dependencies]
utils = { workspace = "utils" }
```

公開時に自動的にバージョン依存へ置換される：

```toml
[dependencies]
utils = "^0.2.0"
```

**バージョンの出典：** 参照されるメンバーの `[package].version` を読み取り、`^`
プレフィックスを付与する。Registry は参照しない——バージョンの権威はメンバーの `yaoxiang.toml`
であり、Registry は単なる配信チャネルに過ぎない。

パッケージマネージャーは `yaoxiang publish` 実行時にこの置換を自動的に行う。

### Cargo Workspace との統合

ワークスペース内に FFI パッケージがある場合、Cargo workspace も同時に定義できる：

```toml
# 根 Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace（FFI 部分）
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # YaoXiang 代码
│   │   └── native/
│   │       ├── Cargo.toml # Rust FFI 代码
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` は自動的にこれを検出し、`cargo build` を呼び出して native 部分をコンパイルする。

### CLI コマンド

| コマンド                           | 機能                                       |
| ---------------------------------- | ------------------------------------------ |
| `yaoxiang workspace list`          | ワークスペースメンバーを一覧表示する       |
| `yaoxiang workspace add <path>`    | メンバーを追加する                         |
| `yaoxiang workspace remove <name>` | メンバーを削除する                         |
| `yaoxiang build`                   | 全メンバーをビルドする（依存トポロジー順） |
| `yaoxiang build core`              | 指定メンバーのみをビルドする               |
| `yaoxiang test`                    | 全メンバーのテストを実行する               |

**`yaoxiang build` の挙動：** 全メンバーを依存トポロジー順にビルドする。core → utils →
app であれば、ビルド順は core → utils → app となる。

## 詳細設計

### WorkspaceManifest 構造

ルート toml は独立した `WorkspaceManifest` 型を用い、`PackageManifest` は再利用しない：

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
    name: String,           // [workspace.members] 的 key
    root: PathBuf,
    manifest: PackageManifest,
}
```

**検出ロジック：** toml を読み込む際に `[workspace]` セクションがあれば `WorkspaceManifest`
として解析し、なければ `PackageManifest` として解析する。

### workspace 依存参照

`{ workspace = "member-name" }` の意味：

- `dependencies` 内で別のワークスペースメンバーを参照する
- 開発時はローカルパスとして解決される
- 公開時は Registry のバージョンへ置換される
- メンバー名は `[workspace.members]` に存在しなければならない

### lockfile の共有

- ワークスペースには `yaoxiang.lock` が 1 つだけ存在する（ルートディレクトリ直下）
- 全メンバーの依存解決結果は同一の lockfile にマージされる
- バージョン競合は lockfile 生成時にエラーとなり、競合の発生元情報も報告される

## トレードオフ

### メリット

- マルチパッケージプロジェクトの統合管理
- 共有 lockfile によるビルド一貫性の保証
- パス依存による快適な開発体験
- Cargo workspace とのシームレスな統合

### デメリット

- 全メンバーが同一の外部依存バージョンを強制される（厳格すぎる可能性）
- ルート toml は独自の依存関係を持てない（設計上の制約）
- Cargo workspace 統合が複雑性を増大させる

## 代替案

| 案                           | 不採用の理由                                      |
| ---------------------------- | ------------------------------------------------- |
| 独立プロジェクト + path 依存 | lockfile が統一されず、バージョンドリフトのリスク |
| npm workspaces 風            | npm の workspace は問題が多く、模倣に値しない     |
| Cargo workspace の直接再利用 | YaoXiang と Cargo は異なるパッケージエコシステム  |

## 実装戦略

### フェーズ分割

| フェーズ | 内容                                           |
| -------- | ---------------------------------------------- |
| Phase 6a | `[workspace.members]` 解析 + WorkspaceManifest |
| Phase 6b | 共有 lockfile + 依存関係マージ解決             |
| Phase 6c | `{ workspace = "name" }` パス依存参照          |
| Phase 6d | 公開時のパス依存自動置換                       |
| Phase 6e | Cargo workspace 統合                           |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ）に依存
- 任意で RFC-014b（ビルドシステム、native メンバー用）に依存

## 未解決の問題

- [x] メンバー間の循環依存を許可するか？→ **許可しない。**
      メンバーは独立したパッケージであり、パッケージ間の循環はコンパイルエラー。（RFC-029 決定、2026-07-30）
- [ ] workspace レベルの `[build]` 設定をサポートするか？
- [ ] メンバーが独自の lockfile を持てるか（ルート lockfile を上書きする）？
- [ ] ネストされた workspace をサポートするか？

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
