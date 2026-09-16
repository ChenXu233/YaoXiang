---
title: 'RFC-014a: Registry プロトコル仕様'
status: 'レビュー中'
author: '晨煦'
created: '2026-06-11'
updated: '2026-09-15'
group: 'rfc-014'
---

# RFC-014a: Registry プロトコル仕様

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
> のサブ RFC です。

## 2026-09-15 レビュー決議

以下の決議は所有者によって 2026-09-15 に決定され、本文の該当章は「公式 Registry 公開後」の完全な仕様として保持されます：

1. **スコープ縮小（本文書で最も重要な項目）**：公式 Registry サーバー、認証（login/logout）、yank を**無期限に先送り**。Phase
   4 の実際の成果物 = GitHub Release/Git アダプタ層 + Registry trait 確定 + `.yxpkg`
   パッケージング +
   `publish --github`。理由：エコシステムのコールドスタートには git/GitHub チャネルのみで十分（Go 初期と同型）；Registry サーバーの運用、アカウント体系と悪用ガバナンスのコストは、サードパーティパッケージがない段階では完全な負債となる。
2. **素のパッケージ名 add は使用不可**：公式 Registry 公開前は `yaoxiang add <素のパッケージ名>`
   はエラーとなり、依存関係の追加には明示的なソース（`--git` /
   `--path`）が必要。後述の「ソース優先順位」のデフォルト検索チェーンは Registry 公開時から有効。
3. **パッケージ形式統一**：`.yxpkg`
   にはソースコードのみ（`yaoxiang.toml`/`src/`/`build.yx`/`SHA256SUMS`）を含め、`build/native/`
   ディレクトリに事前コンパイル成果物を含めない；バイナリ配布はすべて RFC-014b の `[binaries]`
   外部リンク（Release/CDN）経由で行い、パッケージサイズとグローバルキャッシュの肥大化を回避。
4. **Source 配布実装**：内蔵の 4 ソース（Local/Git/Registry/GitHub）は閉じた集合であり、実装層では enum によるディスパッチを使用（dyn-async の Send 制約と
   `async-trait` 依存を回避）；`Source`
   trait 定義はセマンティック層に保持し、将来サードパーティ Source を開放する場合は trait オブジェクト経由での接続とする。
5. **API バージョニング**：URL パス
   `/api/v1/` + レスポンスヘッダにプロトコルバージョンを含める；破壊的変更は v2 に昇格し並存させ、原地での変更は行わない。
6. **レート制限**：GitHub アダプタ層で指数バックオフ +
   ETag 条件付きリクエストキャッシュ；Registry 側のレートポリシーは公式 Registry とともに先送り。
7. **パッケージサイズ上限**：ソースパッケージ 20
   MiB（初期値、調整可能）；GitHub チャネルはプラットフォーム側で管理。

## 要約

YaoXiang パッケージ管理システムの Registry プロトコルを定義：オープンインターフェース設計、公式 Registry 仕様、GitHub アダプタ層、パッケージ公開/撤回フロー、認証モデル。

## 動機

RFC-014 総綱はパッケージ管理システムの全体アーキテクチャを定義しているが、Registry セクションは「予約」のみ。Registry プロトコルがなければ、パッケージを配布できない——これは店のないショッピングカートを設計するようなものだ。

### 現在の問題

- `RegistrySource` はスタブコード（`source/mod.rs:150-203`）で、`resolve`
  は宣言されたバージョンを直接返し、`download` は空のパスを返す
- HTTP クライアントがない（`reqwest` 依存なし）
- パッケージ公開メカニズムがない
- 認証/認可がない

## 提案

### コア設計：オープンプロトコル + アダプタ層

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← CLI 層
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Registry Trait                  │  ← プロトコル層（オープンインターフェース）
│  ┌─────────┬──────────┬────────────┐    │
│  │ .publish│ .search  │ .download  │    │
│  │ .yank   │ .info    │ .versions  │    │
│  └─────────┴──────────┴────────────┘    │
└──────────────────┬───────────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌─────────┐ ┌────────┐ ┌────────┐
   │ 公式    │ │ GitHub │ │ カスタム│
   │ Registry│ │ アダプタ│ │ Registry│
   └─────────┘ └────────┘ └────────┘
```

### 非同期アーキテクチャの決定

`Source` trait を統一して async 化し、tokio を全面採用：

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

すべての実装（`LocalSource`、`GitSource`、`RegistrySource`）を統一して async 化。CLI エントリは
`#[tokio::main]` または `Runtime::block_on` で駆動する。

**理由：**

- Registry は HTTP リクエストを必要とするため、ブロッキングはインストールフロー全体を停止させる
- 複数依存関係の並列ダウンロード（`join_all`）により、インストール速度が大幅に向上
- Git clone も I/O 操作であり、async の方が自然
- tokio は既にプロジェクト依存に含まれている

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// パッケージを公開
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// 公開済みバージョンを削除（復元不可、バージョン番号は固定）
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// パッケージ情報を照会
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// 利用可能なバージョン一覧を照会
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// パッケージを検索
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// 指定バージョンをダウンロード
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// 認証
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### ソース優先順位（デフォルト検索チェーン）

`yaoxiang add foo`（フラグなし）のデフォルト検索順序：

| 優先度 | 検索                 | 説明                                                         |
| ------ | -------------------- | ------------------------------------------------------------ |
| 1      | グローバルキャッシュ | `~/.yaoxiang/cache/registry/foo-<ver>/`                      |
| 2      | 公式 Registry        | バージョン照会 → ダウンロード                                |
| 3      | 失敗                 | エラー、ユーザーへパッケージ名またはネットワークの確認を促す |

**明示的なオーバーライド（デフォルトチェーンを迂回）：**

| flag               | 動作                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------- |
| `--git <url>`      | Registry をスキップ、直接 Git clone（Release assets 優先 → tag/branch フォールバック） |
| `--path <dir>`     | Registry をスキップ、ローカルパスを直接使用                                            |
| `--registry <url>` | 公式 Registry をスキップ、指定された Registry を使用                                   |

### 公式 Registry

公式 Registry は crates.io に類似しており、パッケージ配布の主要チャネルである。

**API エンドポイント：**

| エンドポイント                           | メソッド | 説明                   |
| ---------------------------------------- | -------- | ---------------------- |
| `/api/v1/packages/{name}`                | GET      | パッケージ情報照会     |
| `/api/v1/packages/{name}/versions`       | GET      | バージョン一覧照会     |
| `/api/v1/packages/{name}/{version}`      | GET      | パッケージダウンロード |
| `/api/v1/packages`                       | PUT      | パッケージ公開         |
| `/api/v1/packages/{name}/{version}/yank` | DELETE   | バージョン撤回         |
| `/api/v1/search?q={query}`               | GET      | パッケージ検索         |
| `/api/v1/login`                          | POST     | 認証                   |

### GitHub 統合

GitHub をパッケージソースとして使用する場合、Go modules スタイルのポリシーを採用：

1. **Release assets 優先**：GitHub
   Release ページにプラットフォームにマッチする事前コンパイル成果物があるか確認
2. **main ブランチへフォールバック**：Release がない場合は git clone

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

### パッケージ形式（.yxpkg）

> 2026-09-15 決議：ソースコードのみを含み、`build/`
> の事前コンパイル成果物を削除——バイナリはすべて RFC-014b の `[binaries]` 外部リンク経由で配布。

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # パッケージメタデータ
├── src/                   # ソースコード
├── build.yx               # ビルドスクリプト（ある場合）
└── SHA256SUMS             # チェックサム
```

### publish フロー

```bash
# 公式 Registry へ公開
yaoxiang publish

# 指定した Registry へ公開
yaoxiang publish --registry my-company

# 同時に GitHub Release を作成
yaoxiang publish --github

# ドライラン
yaoxiang publish --dry-run
```

公開前検証：

1. `yaoxiang.toml` には `name`、`version`、`description` が必要
2. バージョン番号は既に存在してはならない
3. テスト実行（オプション、`--no-test` でスキップ）
4. 全ファイルの SHA-256 を計算
5. `.yxpkg`（tar.gz）にパッケージング
6. Registry へアップロード

### yank セマンティクス

```bash
yaoxiang yank foo@1.2.3
```

**削除 + バージョン番号ロック：**

- パッケージは完全に削除され、復元不可
- バージョン番号は永久に占有され、同じバージョン番号を再公開不可
- 既存の lockfile がそのバージョンを参照しているプロジェクトはエラーとなり、他のバージョンへのアップグレードが必要
- **セキュリティ目的**：npm 型のサプライチェーン攻撃を防止。攻撃者が削除されたパッケージのバージョン番号を乗っ取って悪意のあるコードを注入した事例があり、yank でバージョン番号をロックすることでこの経路を完全に塞ぐ。

### 認証モデル

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**マッピングルール：** `yaoxiang login --registry <url>` は URL で `[registries.*]` の `url`
フィールドとマッチング。マッチしない場合は新しいエントリを作成（自動生成名、例：`reg-1`）。

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
yaoxiang logout --registry https://yxreg.example.com   # マッチするエントリを削除
```

**セキュリティ制約：**

- Token は決して `yaoxiang.toml` や `yaoxiang.lock` に書き込まない
- `credentials.toml` のファイルパーミッションは 600
- CI シナリオでは環境変数、開発シナリオではファイルを使用

## 詳細設計

### RegistrySource 実装

既存のスタブコード（`source/mod.rs:150-203`）を置き換え：

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
        // ... dest へ展開 ...

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

| crate            | 用途                 |
| ---------------- | -------------------- |
| `reqwest`        | HTTP クライアント    |
| `sha2`           | SHA-256 検証         |
| `flate2` + `tar` | パッケージ形式処理   |
| `async-trait`    | async trait サポート |

### エラー型

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("パッケージ '{0}' が存在しません")]
    PackageNotFound(String),

    #[error("バージョン '{0}' が存在しません")]
    VersionNotFound(String),

    #[error("バージョン '{0}' は既に占有されています")]
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

### 利点

- オープンプロトコルで、特定のサーバーに縛られない
- GitHub が軽量な配布チャネルとして機能し、参入障壁を下げる
- バージョン番号ロックのセキュリティモデル
- 事前コンパイル優先のインストール戦略

### 欠点

- 公式 Registry は独立した運用が必要
- GitHub API にはレート制限がある
- バージョン番号ロックはバージョン番号の浪費につながる可能性

## 代替案

| 案                        | 採用しなかった理由                                           |
| ------------------------- | ------------------------------------------------------------ |
| GitHub のみサポート       | GitHub エコシステムに制限され、Registry を自前で立てられない |
| Cargo 風 crates.io        | 複雑すぎ、YaoXiang エコシステム初期には不要                  |
| npm 風 yank（マークのみ） | セキュリティリスク、既知のサプライチェーン攻撃事例あり       |

## 実装戦略

### フェーズ分け

| フェーズ  | 内容                                                   |
| --------- | ------------------------------------------------------ |
| Phase 3.5 | Source trait を async 化 + async-trait + 全実装移行    |
| Phase 4a  | Registry trait + reqwest 統合 + ローカル Registry mock |
| Phase 4b  | GitHub Release アダプタ                                |
| Phase 4c  | publish コマンド + パッケージ形式パッケージング        |
| Phase 4d  | 認証 + yank                                            |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ、semver 置換）に依存
- RFC-014b（`build/` ディレクトリ処理用のビルドシステム）に依存

## オープンな問題

- [x] Registry API はバージョニングが必要か（`/api/v1/` vs `/api/v2/`）？→ URL
      `/api/v1/` + バージョン応答ヘッダ、破壊的変更は v2 に昇格し並存（2026-09-15）
- [x] パッケージ名は namespace をサポートするか（例：`@org/pkg`）？→ 初期はサポートせず、フラットなパッケージ名（2026-09-15、総綱参照）
- [x] レート制限ポリシー？→
      GitHub アダプタ層のバックオフ + キャッシュ；Registry 側は公式 Registry とともに先送り（2026-09-15）
- [x] パッケージサイズ上限？→ ソースパッケージ 20 MiB を初期値（2026-09-15、調整可能）

---

## 参考文献

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
