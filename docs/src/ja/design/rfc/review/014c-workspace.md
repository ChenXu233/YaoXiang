---
title: 'RFC-014c: ワークスペースサポート'
status: '審査中'
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

YaoXiang のワークスペース（workspace）メカニズムを定義します：複数の関連パッケージを一緒に開発する際の依存関係共有、パス参照、lockfile 統合、Cargo
workspace との統合。

## 動機

プロジェクト規模が拡大すると、コードを複数のパッケージに分割する必要があります。これらのパッケージには以下の要件があります：

- 互いに参照する（パス依存関係）
- 外部依存関係のバージョンを共有する（バージョンドリフトの回避）
- 統一された lockfile（ビルド一貫性の保証）
- Cargo workspace との協調（FFI 部分）

### 現在の問題

- 各プロジェクトが個別に依存関係を管理し、共有できない
- パス依存関係のリリース時における自動置換メカニズムがない
- Cargo workspace との統合がない

## 提案

### コア設計：調整レイヤー + 自己完結型メンバー

ルートワークスペースは調整のみを行い、各メンバーは完全に自己完結型です。

### ルート yaoxiang.toml

```toml
# ルート yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**ルート toml は三つのことだけを実行します：**

1. メンバーリストの宣言（辞書形式、key がメンバー名、value が toml パス）
2. 共有 lockfile の提供（`yaoxiang.lock`）
3. 共有 vendor ディレクトリの提供（`.yaoxiang/vendor/`）

**ルート toml は dependencies を定義しません。** 各メンバーの依存関係は自分の `yaoxiang.toml`
に記述します。

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
└── Cargo.toml                 # オプション：共有 Cargo workspace（FFI）
```

### 依存関係解決

- 各メンバーは自分の `[dependencies]` を読み取る
- 解決時に全メンバーの依存関係をマージし、共有 lockfile を生成
- バージョン衝突は lockfile 生成時にエラーとして報告
- 同一パッケージが不同メンバー）で異なるバージョンに解決されてはならない

### workspace 依存関係参照

`{ workspace = "member-name" }` は `[workspace.members]` の **key** を参照します（メンバーの
`[package].name` ではありません）。

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

**なぜ key を使うのか：**

- key はワークスペースが制御するため、安定して一意
- `[package].name` は公開名で、リリース時に変更可能性がある
- key は BTreeMap の key であり、aturally 一意
- リリース時に workspace 参照はバージョン依存関係に置き換えられ、key は公開 API に漏洩しない

### パス依存関係とリリース

開発時にはワークスペース参照を使用：

```toml
[dependencies]
utils = { workspace = "utils" }
```

リリース時にはバージョン依存関係に自動置き換え：

```toml
[dependencies]
utils = "^0.2.0"
```

**バージョンのソース：** 被依存メンバーの `[package].version` を読み取り、`^`
プレフィックスを付与。Registry のチェックは行わない——バージョンの権威あるソースはメンバーの
`yaoxiang.toml` であり、Registry は配布渠道にすぎない。

パッケージマネージャーは `yaoxiang publish` 時にこの置換を自動で行う。

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

`yaoxiang build` は native 部分をビルドするために `cargo build` を自動的に検出して呼び出す。

### CLI コマンド

| コマンド                           | 機能                                 |
| ---------------------------------- | ------------------------------------ |
| `yaoxiang workspace list`          | ワークスペースメンバーを一覧表示     |
| `yaoxiang workspace add <path>`    | メンバーを追加                       |
| `yaoxiang workspace remove <name>` | メンバーを削除                       |
| `yaoxiang build`                   | 全メンバーをビルド（依存トポロジ順） |
| `yaoxiang build core`              | 指定メンバーをビルド                 |
| `yaoxiang test`                    | 全メンバーのテストを実行             |

**`yaoxiang build` の動作：** 全メンバーをビルドし、依存トポロジ順にソートする。core → utils →
app の依存関係がある場合、ビルド順序は core → utils → app となる。

## 詳細な設計

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

**探测ロジック：** toml の読み込み時、`[workspace]` セクションがある場合は `WorkspaceManifest`
として解析し、そうでなければ `PackageManifest` として解析する。

### workspace 依存関係参照

`{ workspace = "member-name" }` のセマンティクス：

- `dependencies` で別のワークスペースメンバーを参照する
- 開発時にはローカルパスとして解決
- リリース時には Registry バージョンに置き換え
- メンバー名は `[workspace.members]` に存在しなければならない

### lockfile 共有

- ワークスペースには `yaoxiang.lock` が一つだけ（ルートディレクトリ）
- 全メンバーの依存関係解決は同じ lockfile にマージされる
- バージョン衝突は lockfile 生成時にエラーとして報告、衝突元情報を含む

## トレードオフ

### 优点

- マルチパッケージプロジェクトの統一管理
- 共有 lockfile による一貫性の保証
- パス依存関係による優れた開発体験
- Cargo workspace とのシームレスな統合

### 缺点

- 全メンバーが同じ外部依存関係バージョンを使用しなければならない（過度に厳格な可能性）
- ルート toml は独自の依存関係を持てない（設計制約）
- Cargo workspace 統合により複雑性が増す

## 代替案

| 方案                         | なぜ選択しなかったか                                |
| ---------------------------- | --------------------------------------------------- |
| 獨立プロジェクト + path 依存 | lockfile が統一されず、バージョンドリフトのリスク   |
| npm workspaces 类似          | npm の workspace には問題が多く、真似する価値がない |
| Cargo workspace 直接再利用   | YaoXiang と Cargo は異なるパッケージエコシステム    |

## 実装戦略

### フェーズ分け

| フェーズ | 内容                                           |
| -------- | ---------------------------------------------- |
| Phase 6a | `[workspace.members]` 解析 + WorkspaceManifest |
| Phase 6b | 共有 lockfile + 依存関係マージ解決             |
| Phase 6c | `{ workspace = "name" }` パス依存関係参照      |
| Phase 6d | リリース時パス依存関係の自動置き換え           |
| Phase 6e | Cargo workspace 統合                           |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ）に依存
- RFC-014b（ビルドシステム、native メンバー用）へのオプション依存

## 開放問題

- [ ] メンバー間の循環依存を許可するか？
- [ ] workspace レベルの `[build]` 設定をサポートするか？
- [ ] メンバーは独自の lockfile を持てるか（ルート lockfile をオーバーライド）？
- [ ] ネストされた workspace をサポートするか？

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
