---
title: 'RFC-014c: ワークスペースサポート'
status: 'レビュー中'
author: '晨煦'
created: '2026-06-11'
updated: '2026-09-15'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: ワークスペースサポート

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
> のサブ RFC です。

## 2026-09-15 レビュー決議

以下の決議はオーナーによって 2026-09-15 に決定されました：

1. **実施順序の繰り上げ**：ワークスペースはネットワークやビルドシステムに依存しない（`{ workspace = "key" }`
   パス解決 + 共有 lockfile は完全にローカル）、RFC-014b の **前**
   にスケジュール（全体スケジュールの実行順序を `3 → 3.5 → 6 → 4 → 5`
   に調整）。コンパイラリポジトリ自身（コンパイラ + vscode 拡張 + wasm +
   docs）が最初のユーザーとなります。
2. **workspace レベルの
   `[build]`**：サポートしない。「ルートは調整のみ、メンバーは自己完結」という原則を堅持し、ビルド宣言はメンバーの toml にのみ記載します。
3. **メンバー独自の lockfile**：許可しない、ルートの `yaoxiang.lock` のみ。
4. **ネストされたワークスペース**：初期はサポートしない。

## 概要

YaoXiang のワークスペース（workspace）メカニズムを定義します：複数の関連パッケージを一緒に開発する際の依存関係の共有、パス参照、lockfile の統合、Cargo
workspace との連携。

## 動機

プロジェクト規模が拡大すると、コードを複数のパッケージに分割する必要があります。これらのパッケージには以下が求められます：

- 相互参照（パス依存）
- 外部依存バージョンの共有（バージョンドリフトの防止）
- lockfile の統合（ビルド一貫性の保証）
- Cargo workspace との協調（FFI 部分）

### 現状の問題

- 各プロジェクトが独立して依存関係を管理し、共有できない
- パス依存の公開時自動置換メカニズムがない
- Cargo workspace との統合がない

## 提案

### コア設計：調整層 + 自己完結メンバー

ルートワークスペースは調整のみを行い、各メンバーは完全に自己完結します。

### ルート yaoxiang.toml

```toml
# ルート yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**ルート toml が行うのは 3 つのことだけです：**

1. メンバーリストの宣言（辞書形式、key はメンバー名、value は toml パス）
2. 共有 lockfile の提供（`yaoxiang.lock`）
3. 共有 vendor ディレクトリの提供（`.yaoxiang/vendor/`）

**ルート toml は dependencies を定義しません。** 各メンバーの依存関係は自身の `yaoxiang.toml`
に記述します。

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
- 解決時に全メンバーの依存関係をマージし、単一の共有 lockfile を生成
- バージョン競合は lockfile 生成時にエラー
- 同じパッケージが異なるメンバーで同じバージョンに解決されなければならない

### workspace 依存参照

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
# packages/utils/yaoxiang.toml 内で name = "my-utils" と書かれていても
```

**なぜ key ではなく name を使うのか：**

- key はワークスペースによって制御され、安定した唯一無二の存在
- `[package].name` は公開名であり、公開時に変わり得る
- key は BTreeMap の key であり、本質的に唯一無二
- 公開時に workspace 参照はバージョン依存に置換され、key は公開 API に漏れない

### パス依存と公開

開発時はワークスペース参照を使用：

```toml
[dependencies]
utils = { workspace = "utils" }
```

公開時は自動的にバージョン依存に置換：

```toml
[dependencies]
utils = "^0.2.0"
```

**バージョンの出典：** 依存されるメンバーの `[package].version` を読み取り、`^`
プレフィックスを付加。Registry は確認しません——バージョンの信頼できる出典はメンバーの
`yaoxiang.toml` であり、Registry は単なる配布チャネルです。

パッケージマネージャーは `yaoxiang publish` 時にこの置換を自動的に行います。

### Cargo Workspace との統合

ワークスペース内に FFI パッケージがある場合、同時に Cargo workspace を定義できます：

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

`yaoxiang build` は自動的に検出し、`cargo build` を呼び出して native 部分をコンパイルします。

### CLI コマンド

| コマンド                           | 機能                                         |
| ---------------------------------- | -------------------------------------------- |
| `yaoxiang workspace list`          | ワークスペースメンバーをリスト表示           |
| `yaoxiang workspace add <path>`    | メンバーを追加                               |
| `yaoxiang workspace remove <name>` | メンバーを削除                               |
| `yaoxiang build`                   | 全メンバーをビルド（依存トポロジーでソート） |
| `yaoxiang build core`              | 指定メンバーをビルド                         |
| `yaoxiang test`                    | 全メンバーのテストを実行                     |

**`yaoxiang build` の動作：** 全メンバーを依存トポロジーでソートしてビルド。core → utils →
app の場合、ビルド順序は core → utils → app。

## 詳細設計

### WorkspaceManifest 構造

ルート toml は独立した `WorkspaceManifest` 型を使用し、`PackageManifest` を再利用しません：

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

**検出ロジック：** toml をロードする際、`[workspace]` セクションがあれば `WorkspaceManifest`
としてパースし、なければ `PackageManifest` としてパースします。

### workspace 依存参照

`{ workspace = "member-name" }` のセマンティクス：

- `dependencies` 内で別のワークスペースメンバーを参照
- 開発時はローカルパスに解決
- 公開時は Registry バージョンに置換
- メンバー名は `[workspace.members]` に存在しなければならない

### lockfile 共有

- ワークスペースには 1 つの `yaoxiang.lock` のみ（ルートディレクトリ）
- 全メンバーの依存関係解決を同じ lockfile にマージ
- バージョン競合は lockfile 生成時にエラー、競合発生元の情報を付与

## トレードオフ

### 利点

- マルチパッケージプロジェクトの統合管理
- 共有 lockfile による一貫性保証
- パス依存による優れた開発体験
- Cargo workspace とのシームレスな統合

### 欠点

- 全メンバーが同じ外部依存バージョンを使用しなければならない（厳しすぎる可能性）
- ルート toml は独自の依存関係を持てない（設計上の制約）
- Cargo workspace 統合が複雑性を増加させる

## 代替案

| 案                           | 採用しなかった理由                                |
| ---------------------------- | ------------------------------------------------- |
| 独立プロジェクト + path 依存 | lockfile が統一されず、バージョンドリフトのリスク |
| npm workspaces 風            | npm の workspace には問題が多く、模倣する価値なし |
| Cargo workspace の直接再利用 | YaoXiang と Cargo は異なるパッケージエコシステム  |

## 実装戦略

### 段階区分

| 段階     | 内容                                             |
| -------- | ------------------------------------------------ |
| Phase 6a | `[workspace.members]` パース + WorkspaceManifest |
| Phase 6b | 共有 lockfile + 依存関係マージ解決               |
| Phase 6c | `{ workspace = "name" }` パス依存参照            |
| Phase 6d | 公開時のパス依存自動置換                         |
| Phase 6e | Cargo workspace 統合                             |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ）に依存
- オプションで RFC-014b（ビルドシステム、native メンバー用）に依存

## オープンな問題

- [x] メンバー間の循環依存を許可するか？→ **許可しない。**
      メンバーは独立したパッケージであり、パッケージ間の循環はコンパイルエラー。（RFC-029 決議、2026-07-30）
- [x] workspace レベルの `[build]`
      設定をサポートするか？→ サポートしない、メンバーは自己完結（2026-09-15 決議 2）
- [x] メンバーが独自の lockfile を持てるか（ルート lockfile を上書き）？→ 許可しない、ルート lockfile のみ（2026-09-15 決議 3）
- [x] ネストされたワークスペースをサポートするか？→ 初期はサポートしない（2026-09-15 決議 4）

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
