---
title: 'RFC-014a: Registry プロトコル仕様'
status: '承認済み'
author: '晨煦'
created: '2026-06-11'
updated: '2026-09-29'
group: 'rfc-014'
---

# RFC-014a: Registry プロトコル仕様

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
> のサブ RFC です。

## 2026-09-15 審査決議

以下の決議は所有者により 2026-09-15 に決定され、本文の関連セクションは「公式 Registry 公開後」の完全な仕様として保持されます：

1. **スコープ縮小（本文書で最も重要な項目）**：公式 Registry サーバー、認証（login/logout）、yank は**無期限延期**。Phase
   4 の実際の成果物 = GitHub Release/Git アダプタ層 + Registry trait の確定 + `.yxpkg`
   パッケージ化 +
   `publish --github`。理由：エコシステムのコールドスタートには git/GitHub チャネルのみで十分（Go の初期も同様）；Registry サーバーの運用、アカウント体系、および悪用ガバナンスのコストは、サードパーティ製パッケージが無い段階では純粋な負債です。
2. **素のパッケージ名での add は使用不可**：公式 Registry 公開前は `yaoxiang add <素のパッケージ名>`
   はエラーを返し、依存関係の追加には明示的なソース（`--git` /
   `--path`）が必要です。下記の「ソース優先度」のデフォルト検索チェーンは Registry 公開時から有効になります。
3. **パッケージフォーマットの統一**：`.yxpkg`
   はソースコードのみを含み（`yaoxiang.toml`/`src/`/`build.yx`/`SHA256SUMS`）、`build/native/`
   プレコンパイル成果物ディレクトリを削除；バイナリ配布は一律 RFC-014b の `[binaries]`
   外部リンク（Release/CDN）を経由し、パッケージサイズとグローバルキャッシュの膨張を回避します。
4. **ソース配布実装**：ビルトインの 4 ソース（Local/Git/Registry/GitHub）は閉じた集合であり、実装層では enum ディスパッチを使用します（dyn-async の Send 制約と
   `async-trait` 依存を回避）；`Source`
   trait 定義はセマンティック層に保持され、将来サードパーティソースを解放する場合は trait オブジェクト経由で統合します。
5. **API バージョニング**：URL パス
   `/api/v1/` + レスポンスヘッダーでプロトコルバージョンを含めます；破壊的変更は v2 に上げて並存させ、インプレース変更は行いません。
6. **レート制限**：GitHub アダプタ層は指数バックオフ +
   ETag 条件付きリクエストキャッシュ；Registry 側のレート戦略は公式 Registry と共に延期されます。
7. **パッケージサイズ上限**：ソースパッケージ 20
   MiB（初期値、調整可能）；GitHub チャネルはプラットフォーム自身が管理します。

## 概要

YaoXiang パッケージ管理システムの Registry プロトコルを定義します：オープンインターフェース設計、公式 Registry 仕様、GitHub アダプタ層、パッケージ公開/撤回フロー、認証モデル。

## 動機

RFC-014 全体綱領はパッケージ管理システムの全体アーキテクチャを定義していますが、Registry セクションは「予約」としてマークされているのみです。Registry プロトコルが無ければ、パッケージは配布できません——これはまるでストアの無いショッピングカートを設計するようなものです。

### 現在の問題

- `RegistrySource` はスタブコード（`source/mod.rs:150-203`）で、`resolve`
  は宣言されたバージョンを直接返し、`download` は空のパスを返します
- HTTP クライアントが無い（`reqwest` 依存なし）
- パッケージ公開メカニズムが無い
- 認証/認可が無い

## 提案

### コア設計：オープンプロトコル + アダプタ層

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← CLI レイヤー
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Registry trait                  │  ← プロトコル層（オープンインターフェース）
│  ┌─────────┬──────────┬────────────┐    │
│  │ .publish│ .search  │ .download  │    │
│  │ .yank   │ .info    │ .versions  │    │
│  └─────────┴──────────┴────────────┘    │
└──────────────────┬───────────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌─────────┐ ┌────────┐ ┌────────┐
   │ 公式    │ │ GitHub │ │ カスタム │
   │ Registry│ │ アダプタ│ │ Registry│
   └─────────┘ └────────┘ └────────┘
```

### 非同期アーキテクチャ決定

`Source` trait を統一して async に変更し、tokio を全面採用します：

```rust
// 既存（同期）→ 変更後（非同期）
#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String>;
    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage>;
}
```

すべての実装（`LocalSource`、`GitSource`、`RegistrySource`）を統一して async に変更します。CLI エントリーポイントは
`#[tokio::main]` または `Runtime::block_on` で駆動します。

**理由：**

- Registry は HTTP リクエストが必要で、ブロッキングはインストールフロー全体をストールさせます
- 複数依存の並列ダウンロード（`join_all`）でインストール速度が著しく向上します
- Git clone も I/O 操作であり、async の方が自然です
- tokio は既にプロジェクト依存に含まれています

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// パッケージを公開
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// 公開済みバージョンを削除（復元不可、バージョン番号はロック）
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// パッケージ情報をクエリ
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// 利用可能なバージョンリストをクエリ
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// パッケージを検索
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// 指定されたバージョンをダウンロード
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// 認証
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### ソース優先度（デフォルト検索チェーン）

`yaoxiang add foo`（flag なし）時のデフォルト検索順序：

| 優先度 | 検索                 | 説明                                                                     |
| ------ | -------------------- | ------------------------------------------------------------------------ |
| 1      | グローバルキャッシュ | `~/.yaoxiang/cache/registry/foo-<ver>/`                                  |
| 2      | 公式 Registry        | バージョン照会 → ダウンロード                                            |
| 3      | 失敗                 | エラーを返し、パッケージ名またはネットワークを確認するようユーザーに通知 |

**明示的オーバーライド（デフォルトチェーンを使用しない）：**

| flag               | 動作                                                                                         |
| ------------------ | -------------------------------------------------------------------------------------------- |
| `--git <url>`      | Registry をスキップし、直接 Git clone（Release assets を優先 → tag/branch にフォールバック） |
| `--path <dir>`     | Registry をスキップし、直接ローカルパスを使用                                                |
| `--registry <url>` | 公式 Registry をスキップし、指定された Registry を使用                                       |

### 公式 Registry

公式 Registry は crates.io に類似しており、パッケージ配布の主要チャネルです。

**API エンドポイント：**

| エンドポイント                           | メソッド | 説明                     |
| ---------------------------------------- | -------- | ------------------------ |
| `/api/v1/packages/{name}`                | GET      | パッケージ情報をクエリ   |
| `/api/v1/packages/{name}/versions`       | GET      | バージョンリストをクエリ |
| `/api/v1/packages/{name}/{version}`      | GET      | パッケージをダウンロード |
| `/api/v1/packages`                       | PUT      | パッケージを公開         |
| `/api/v1/packages/{name}/{version}/yank` | DELETE   | バージョンを撤回         |
| `/api/v1/search?q={query}`               | GET      | パッケージを検索         |
| `/api/v1/login`                          | POST     | 認証                     |

### GitHub 統合

GitHub をパッケージソースとして使用する場合、Go modules スタイルの戦略を採用します：

1. **Release assets 優先**：GitHub
   Release ページにプラットフォームにマッチするプレコンパイル成果物が有るかを確認
2. **main ブランチにフォールバック**：Release が無い場合は git clone

```toml
[dependencies]
# 基本的な git 依存
foo = { git = "https://github.com/user/foo" }

# バージョン指定（tag にマッチ）
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# ブランチ指定
baz = { git = "https://github.com/user/baz", branch = "main" }

# commit 指定
qux = { git = "https://github.com/user/qux", rev = "abc123" }

# プライベートリポジトリ（credentials.toml の GitHub token を使用）
private = { git = "https://github.com/my-org/private-lib" }
```

### パッケージフォーマット（.yxpkg）

> 2026-09-15 決議：ソースコードのみを含み、`build/`
> プレコンパイル成果物は削除——バイナリは一律 RFC-014b の `[binaries]` 外部リンク経由で配布。

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # パッケージメタデータ
├── src/                   # ソースコード
├── build.yx               # ビルドスクリプト（存在する場合）
└── SHA256SUMS             # チェックサム
```

### publish フロー

```bash
# 公式 Registry に公開
yaoxiang publish

# 指定された Registry に公開
yaoxiang publish --registry my-company

# 同時に GitHub Release を作成
yaoxiang publish --github

# ドライラン
yaoxiang publish --dry-run
```

公開前検証：

1. `yaoxiang.toml` には `name`、`version`、`description` が必須
2. バージョン番号が既に存在してはならない
3. テストを実行（オプション、`--no-test` でスキップ）
4. すべてのファイルの SHA-256 を計算
5. `.yxpkg`（tar.gz）としてパッケージ化
6. Registry にアップロード

### yank セマンティクス

```bash
yaoxiang yank foo@1.2.3
```

**削除 + バージョン番号ロック：**

- パッケージは完全に削除され、復元不可
- バージョン番号は永久に占拠され、同じバージョン番号を再公開することはできない
- 既に lockfile が該当バージョンを参照しているプロジェクトはエラーとなり、他のバージョンにアップグレードが必要
- **セキュリティ目的**：npm スタイルのサプライチェーン攻撃を防止。攻撃者がかつて削除されたパッケージのバージョン番号を先取りして悪意のあるコードを注入した事例があり、yank でバージョン番号をロックすることでこの抜け道を完全に塞ぎます。

### 認証モデル

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**マッピングルール：** `yaoxiang login --registry <url>` は URL により `[registries.*]` の `url`
フィールドをマッチします。マッチしない場合は、新規エントリを作成します（名前を自動生成、例：`reg-1`）。

**優先度：** 環境変数 > 設定ファイル

| 環境変数             | 用途                                    |
| -------------------- | --------------------------------------- |
| `$YX_GITHUB_TOKEN`   | GitHub 認証                             |
| `$YX_REGISTRY_TOKEN` | Registry 認証（デフォルト Registry 用） |
| `$YX_REGISTRY_URL`   | デフォルト Registry アドレス            |

**CLI コマンド：**

```bash
yaoxiang login --registry https://yxreg.example.com   # URL でマッチまたは新規作成
yaoxiang login --github                                # GitHub OAuth または token
yaoxiang logout --registry https://yxreg.example.com   # マッチしたエントリを削除
```

**セキュリティ制約：**

- Token は決して `yaoxiang.toml` や `yaoxiang.lock` に書き込まない
- `credentials.toml` ファイル権限は 600
- CI シナリオでは環境変数を使用し、開発シナリオではファイルを使用

## 詳細設計

### RegistrySource 実装

既存スタブコード（`source/mod.rs:150-203`）を置き換え：

```rust
pub struct RegistrySource {
    client: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl Source for RegistrySource {
    fn name(&self) -> &str { "registry" }
    fn kind(&self) -> SourceKind { SourceKind::Registry }

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String> {
        let url = format!("{}/api/v1/packages/{}/versions", self.base_url, spec.name);
        let versions: Vec<Version> = self.client.get(&url).send().await?.json().await?;
        let req = parse_version_req(&spec.version)?;
        select_best(&req, &versions)
            .map(|v| v.to_string())
            .ok_or(PackageError::DependencyNotFound(spec.name.clone()))
    }

    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage> {
        let version = self.resolve(spec).await?;
        let url = format!("{}/api/v1/packages/{}/{}/download", self.base_url, spec.name, version);
        let bytes = self.client.get(&url).send().await?.bytes().await?;

        // SHA-256 検証
        let actual_hash = sha256_hex(&bytes);
        // ... dest に展開 ...

        Ok(ResolvedPackage {
            name: spec.name.clone(),
            version,
            source_kind: SourceKind::Registry,
            source_url: self.base_url.clone(),
            local_path: dest.to_path_buf(),
            checksum: Some(actual_hash),
        })
    }
}
```

### 依存関係

| crate             | 用途                                                                                    |
| ----------------- | --------------------------------------------------------------------------------------- |
| `reqwest`         | HTTP クライアント                                                                       |
| `sha2`            | SHA-256 検証                                                                            |
| `flate2` + `tar`  | パッケージフォーマット処理                                                              |
| ~~`async-trait`~~ | 廃止——決議 4 で enum ディスパッチ + ネイティブ async fn in trait を採用、この依存は不要 |

### エラー型

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("パッケージ '{0}' が存在しません")]
    PackageNotFound(String),

    #[error("バージョン '{0}' が存在しません")]
    VersionNotFound(String),

    #[error("バージョン '{0}' は既に占拠されています")]
    VersionAlreadyExists(String),

    #[error("認証失敗: {0}")]
    AuthFailed(String),

    #[error("ネットワークエラー: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 検証失敗: 期待 {expected}, 実際 {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("権限不足: {0}")]
    Forbidden(String),
}
```

## トレードオフ

### メリット

- オープンプロトコルで、特定のサーバーに縛られない
- GitHub を軽量な配布チャネルとして使用し、参入障壁を低下
- バージョン番号ロックのセキュリティモデル
- プレコンパイル優先のインストール戦略

### デメリット

- 公式 Registry は独立した運用が必要
- GitHub API にはレート制限がある
- バージョン番号ロックはバージョン番号の浪費につながる可能性がある

## 代替案

| 案                              | 選ばなかった理由                                                 |
| ------------------------------- | ---------------------------------------------------------------- |
| GitHub のみサポート             | GitHub エコシステムに制限され、自前 Registry が不可能            |
| Cargo スタイル crates.io        | 過度に複雑で、YaoXiang エコシステム初期には不要                  |
| npm スタイル yank（マークのみ） | セキュリティリスクがあり、既知のサプライチェーン攻撃ケースがある |

## 実装戦略

### フェーズ分割

| フェーズ  | 内容                                                                                                                                                                                                                                                                 | 状態                                       |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| Phase 3.5 | Source 配布 enum 化（決議 4）+ ネイティブ async fn in trait + 全実装移行                                                                                                                                                                                             | ✅ 完了                                    |
| Phase 4a  | GitHub アダプタ層（元の「Registry trait + reqwest + ローカル mock」は決議 1 により縮小：github.com の git 依存を `GitHubSource` にルーティング、API 解析 + `.yxpkg` アセットダウンロード + git フォールバック；指数バックオフ + ETag 条件キャッシュで決議 6 を実装） | ✅ 完了                                    |
| Phase 4b  | `.yxpkg` パッケージフォーマット（tar.gz + SHA256SUMS + 20 MiB 上限、決議 3/7；パッケージ化の決定性、展開時の強制検証）                                                                                                                                               | ✅ 完了                                    |
| Phase 4c  | publish コマンド（`--dry-run` ローカル全チェーン；`--github` 重複チェック → tag 検証 → Release → アセットアップロード；6d workspace 参照の置換はパッケージ化時にマテリアライズ）                                                                                     | ✅ 完了                                    |
| Phase 4d  | 認証（login/logout/credentials.toml）+ yank                                                                                                                                                                                                                          | 無期限延期（公式 Registry に伴い、決議 1） |

**実装説明（2026-09-29）：**

- HTTP スタック：reqwest（rustls、OpenSSL クロスコンパイル不要）+ パッケージ管理独自の tokio
  current_thread ランタイム（`package::runtime::drive`）；POST 系リクエスト（Release 作成/アセットアップロード）は自動リトライを行わない——非冪等であり、5xx 後の再送で重複作成される可能性があるため。
- publish の対象リポジトリ解決：`[package].repository` 優先、フォールバックで
  `git remote origin`；tag の存在を要求（Cargo と同じセマンティクス：publish は tag の作成を代行しません）。
- 公開前テスト実行（上記検証リスト第 3 ステップ）は接続済み（2026-09-30、RFC-014b に伴い）：デフォルトで
  `[tool.test]` で検出されたテストを実行、失敗時は公開を中止、`--no-test` でスキップ。
- `credentials.toml` と `login`/`logout`/`yank` コマンドは公式 Registry とともに延期；現在の認証は
  `$YX_GITHUB_TOKEN` 環境変数のみ（優先度ルール不変：環境変数 > 設定ファイル）。

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ、semver 置換）に依存
- RFC-014b（ビルドシステム、`build/` ディレクトリ処理用）に依存

## オープン問題

- [x] Registry API はバージョニングが必要か（`/api/v1/` vs `/api/v2/`）？→ URL
      `/api/v1/` + バージョンヘッダー、破壊的変更は v2 に上げて並存（2026-09-15）
- [x] パッケージ名は namespace をサポートするか（例：`@org/pkg`）？→ 初期は非サポート、フラットパッケージ名（2026-09-15、全体綱領を参照）
- [x] レート制限戦略？→
      GitHub アダプタ層のバックオフ + キャッシュ；Registry 側は公式 Registry とともに延期（2026-09-15）
- [x] パッケージサイズ上限？→ ソースパッケージ 20 MiB 初期値（2026-09-15、調整可能）

---

## 参考文献

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
