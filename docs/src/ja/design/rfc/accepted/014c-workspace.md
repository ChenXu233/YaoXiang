---
title: 'RFC-014c: ワークスペースサポート'
status: '承認済み'
author: '晨煦'
created: '2026-06-11'
updated: '2026-09-29'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: ワークスペースサポート

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
> のサブ RFC です。

## 2026-09-15 審査決議

以下の決議は所有者により 2026-09-15 に決定されました：

1. **実施順序の前倒し**：ワークスペースはネットワークとビルドシステムに依存しない（`{ workspace = "key" }`
   のパス解決 + 共有 lockfile は純ローカル）ため、スケジュールを RFC-014b **より前**
   に前倒しする（総綱の実行順序を `3 → 3.5 → 6 → 4 → 5`
   に調整）。コンパイラリポジトリ自体（コンパイラ + vscode 拡張 + wasm +
   docs）が最初のユーザーとなる。
2. **workspace レベル
   `[build]`**：サポートしない。「ルートは調整のみ、メンバーは自己完結」という原則を堅持し、ビルド宣言はメンバーの toml にのみ記述する。
3. **メンバー独自の lockfile**：許可しない、ルート `yaoxiang.lock` のみとする。
4. **ネスト workspace**：初期はサポートしない。

## 要約

YaoXiang のワークスペース（workspace）メカニズムを定義する：複数の関連パッケージを一緒に開発する際の依存共有、パス参照、lockfile 統一、Cargo
workspace との統合。

## 動機

プロジェクトの規模が大きくなると、コードを複数のパッケージに分割する必要がある。これらのパッケージには以下が求められる：

- 相互参照（パス依存）
- 外部依存バージョンの共有（バージョンドリフトの回避）
- lockfile の統一（ビルド一貫性の保証）
- Cargo workspace との連携（FFI 部分）

### 現状の問題

- 各プロジェクトが独立して依存を管理しており、共有できない
- パス依存の公開時自動置換メカニズムがない
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

[workspace.dependencies]        # 2026-09-29 改訂：共有依存の権威バージョン
regex = "^1.0"
json = "^2.0"
```

**ルート toml が行うのは次の四つだけ：**

1. メンバーリストの宣言（辞書形式、key はメンバー名、value は toml パス）
2. 共有 lockfile の提供（`yaoxiang.lock`）
3. 共有 vendor ディレクトリの提供（`.yaoxiang/vendor/`）
4. （2026-09-29 改訂）共有依存バージョンの宣言（`[workspace.dependencies]`）

> **2026-09-29 改訂：`[workspace.dependencies]`
> 継承の追加（元「ルート toml は dependencies を定義しない」を緩和）。**
> バージョンを厳格に統一するルール下では、複数メンバーが同名パッケージをそれぞれ宣言する摩擦が予想される；Cargo/uv の先例に倣い、ルートが共有依存の権威バージョンを宣言し、メンバーは
> `{ workspace = true }`
> で個別に継承する（共有バージョンの昇格はルートの 1 か所のみ変更）。メンバーの参照は引き続き
> `{ workspace = "<key>" }`（文字列）——同じ key で異なる型という Cargo と同形のもの。 マージ解析：全メンバーの deps +
> dev-deps を共有 lockfile にマージ；同名パッケージは交差を要求し、空交差の場合は競合として出所を列挙して報告；git 依存の base
> URL 不一致は競合とみなす。

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

- 各メンバーは自分の `[dependencies]` を読む
- 解決時に全メンバーの依存をマージし、共有 lockfile を 1 つ生成する
- バージョン競合は lockfile 生成時にエラー
- 同じパッケージは異なるメンバー間で同じバージョンに解決されなければならない

### workspace 依存参照

`{ workspace = "member-name" }` は `[workspace.members]` の **key** を参照する（メンバーの
`[package].name` ではない）。

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

- key はワークスペースによって制御され、安定かつ一意
- `[package].name` は公開名であり、公開時に変わり得る
- key は BTreeMap の key であり、自然に一意
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

**バージョン出所：** 依存先メンバーの `[package].version` を読み取り、`^`
プレフィックスを付加する。Registry は確認しない——バージョンの権威出所はメンバーの `yaoxiang.toml`
であり、Registry は単なる配信チャネル。

パッケージマネージャーは `yaoxiang publish` 時にこの置換を自動的に行う。

### Cargo Workspace との統合

ワークスペース内に FFI パッケージがある場合、Cargo workspace も同時に定義可能：

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

`yaoxiang build` は native 部分を自動的に検出し、`cargo build` を呼び出してコンパイルする。

### CLI コマンド

| コマンド                           | 機能                                                                                                      |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `yaoxiang workspace list`          | ワークスペースメンバーを列挙                                                                              |
| `yaoxiang workspace add <path>`    | メンバーを追加（key は [package].name を取得、`--as` で上書き可；✅ 実装済み）                            |
| `yaoxiang workspace remove <name>` | メンバーを削除（登録からの摘み取りのみでディレクトリは削除しない；参照されている場合は通知；✅ 実装済み） |
| `yaoxiang build`                   | 全メンバーをビルド（依存トポロジカル順）                                                                  |
| `yaoxiang build core`              | 指定メンバーのみビルド                                                                                    |
| `yaoxiang test`                    | 全メンバーのテストを実行                                                                                  |

**`yaoxiang build` の挙動：** 全メンバーを依存トポロジカル順にビルドする。core → utils →
app の場合、ビルド順は core → utils → app。

## 詳細設計

### WorkspaceManifest 構造

ルート toml は独立した `WorkspaceManifest` 型を使用し、`PackageManifest` を再利用しない：

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

**検出ロジック：** toml 読み込み時、`[workspace]` セクションがあれば `WorkspaceManifest`
として解析し、なければ `PackageManifest` として解析する。

### workspace 依存参照

`{ workspace = "member-name" }` のセマンティクス：

- `dependencies` 内で別のワークスペースメンバーを参照
- 開発時はローカルパスに解決
- 公開時は Registry バージョンに置換
- メンバー名は `[workspace.members]` に存在しなければならない

### メンバー可視性（2026-09-29 落章：pnpm 式厳格）

メンバーのコードの `use` は **自分で宣言した依存**（バージョン類 + path 依存 +
workspace 参照）からのみ解決される——共有 vendor 内の他メンバーが宣言したパッケージは本メンバーから
**不可視**。宣言していないものはインストールされておらず、誤用は直接「モジュールが見つかりません」と宣言を促す形で報告される：ファントム依存（npm
hoisting の有名な落とし穴）はコンパイル時に露出し、パッケージ公開後に消費者の爆弾にはならない。

メンバー参照と path 依存はターゲットパッケージルートの `src/`
レイアウト（vendor エントリと同じ構造、バージョンサフィックスなし）にパス解決され、ターゲットのインポート面（RFC-029f）が適用される。一貫性チェックも同様にワークスペースルートを錨とする：マージ済み依存 vs ルート lockfile
vs ルート vendor。

### lockfile の共有

- ワークスペースには `yaoxiang.lock` が 1 つだけ（ルートディレクトリに）
- 全メンバーの依存解決は同じ lockfile にマージされる
- バージョン競合は lockfile 生成時にエラー報告され、競合出所情報が添付される

## トレードオフ

### 利点

- マルチパッケージプロジェクトを統一管理
- 共有 lockfile が一貫性を保証
- パス依存の開発体験が良好
- Cargo workspace とシームレスに統合

### 欠点

- 全メンバーが同じ外部依存バージョンを使用しなければならない（厳しすぎる可能性）
- ルート toml は独自の依存を持てない（設計上の制約）
- Cargo workspace 統合が複雑性を増加

## 代替案

| 案                           | 採用しなかった理由                               |
| ---------------------------- | ------------------------------------------------ |
| 独立プロジェクト + path 依存 | lockfile が統一されず、バージョンドリフトリスク  |
| npm workspaces 風            | npm の workspace 問題が多く、模倣する価値なし    |
| Cargo workspace の直接再利用 | YaoXiang と Cargo は異なるパッケージエコシステム |

## 実装戦略

### フェーズ分割

| フェーズ | 内容                                                                               | 状態                                                                                                                                                                                                 |
| -------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 6a | `[workspace.members]` 解析 + WorkspaceManifest                                     | ✅ 完了                                                                                                                                                                                              |
| Phase 6b | 共有 lockfile + 依存マージ解析（`[workspace.dependencies]` 継承、2026-09-29 改訂） | ✅ 完了                                                                                                                                                                                              |
| Phase 6c | `{ workspace = "name" }` パス依存参照（pnpm 式厳格可視性、2026-09-29 改訂）        | ✅ 完了                                                                                                                                                                                              |
| Phase 6d | 公開時のパス依存自動置換                                                           | ✅ 完了（publish に伴い：置換は **パッケージング時** にはアーカイブ内 manifest に具現化され、ディスク上 manifest はワークスペース形態を維持；`workspace = true` 継承も同様にルート宣言として具現化） |
| Phase 6e | Cargo workspace 統合                                                               | 後置                                                                                                                                                                                                 |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ）に依存
- オプションで RFC-014b（ビルドシステム、native メンバー用）に依存

## オープンクエスチョン

- [x] メンバー間の循環依存を許可するか？→ **許可しない。**
      メンバーは独立パッケージであり、パッケージ間の循環はコンパイルエラー。（RFC-029 決定、2026-07-30）
- [x] workspace レベルの `[build]`
      設定を許可するか？→ 許可しない、メンバーは自己完結（2026-09-15 決議 2）
- [x] メンバーが独自の lockfile（ルート lockfile を上書き）を持てるか？→ 許可しない、ルート lockfile のみ（2026-09-15 決議 3）
- [x] ネスト workspace をサポートするか？→ 初期はサポートしない（2026-09-15 決議 4）

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
