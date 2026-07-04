---
title: "RFC-014a: Registry 协议规范"
status: "审核中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014a: Registry 协议规范

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC です。

## 要約

YaoXiang パッケージ管理システムの Registry プロトコルを定義する：オープンインターフェース設計、公式 Registry 仕様、GitHub アダプタ層、パッケージ公開/取り下げフロー、認証モデル。

## 動機

RFC-014 総論ではパッケージ管理システムの全体アーキテクチャを定義しているが、Registry の部分は「予約」のみで示されている。Registry プロトコルがなければ、パッケージを配布できない──これはまるで店のないショッピングカートを設計するようなものだ。

### 現状の問題

- `RegistrySource` はスタブコード（`source/mod.rs:150-203`）であり、`resolve` は宣言されたバージョンをそのまま返し、`download` は空のパスを返す
- HTTP クライアントが存在しない（`reqwest` 依存関係なし）
- パッケージ公開機構が存在しない
- 認証/認可が存在しない

## 提案

### 中核設計：オープンプロトコル + アダプタ層

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← CLI レイヤー
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Registry Trait                  │  ← プロトコルレイヤー（オープンインターフェース）
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

### 非同期アーキテクチャの判断

`Source` trait を一律 async へと変更し、tokio を全面的に採用する：

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

すべての実装（`LocalSource`、`GitSource`、`RegistrySource`）を一律 async へと変更する。CLI エントリポイントは `#[tokio::main]` または `Runtime::block_on` によって駆動する。

**理由：**
- Registry は HTTP リクエストを必要とし、ブロッキングはインストールフロー全体を停止させる
- 複数依存の並列ダウンロード（`join_all`）がインストール速度を大幅に向上させる
- Git clone も I/O 操作であり、async の方が自然である
- tokio は既にプロジェクトの依存関係に含まれている

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// パッケージを公開する
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// 公開済みバージョンを削除する（復元不可、バージョン番号は永久ロック）
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// パッケージ情報を照会する
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// 利用可能なバージョン一覧を照会する
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// パッケージを検索する
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// 指定バージョンをダウンロードする
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// 認証
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### ソース優先順位（デフォルトの検索チェーン）

`yaoxiang add foo`（フラグなし）のデフォルト検索順序：

| 優先度 | 検索先 | 説明 |
|--------|------|------|
| 1 | グローバルキャッシュ | `~/.yaoxiang/cache/registry/foo-<ver>/` |
| 2 | 公式 Registry | バージョン照会 → ダウンロード |
| 3 | 失敗 | エラー報告、パッケージ名またはネットワークの確認をユーザに促す |

**明示的なオーバーライド（デフォルトチェーンを経由しない）：**

| flag | 動作 |
|------|------|
| `--git <url>` | Registry をスキップし、直接 Git clone（Release assets 優先 → tag/branch にフォールバック） |
| `--path <dir>` | Registry をスキップし、直接ローカルパスを使用 |
| `--registry <url>` | 公式 Registry をスキップし、指定 Registry を使用 |

### 公式 Registry

公式 Registry は crates.io に類似しており、パッケージ配布の主要チャネルである。

**API エンドポイント：**

| エンドポイント | メソッド | 説明 |
|------|------|------|
| `/api/v1/packages/{name}` | GET | パッケージ情報を照会 |
| `/api/v1/packages/{name}/versions` | GET | バージョン一覧を照会 |
| `/api/v1/packages/{name}/{version}` | GET | パッケージをダウンロード |
| `/api/v1/packages` | PUT | パッケージを公開 |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | バージョン取り下げ |
| `/api/v1/search?q={query}` | GET | パッケージ検索 |
| `/api/v1/login` | POST | 認証 |

### GitHub 統合

GitHub をパッケージソースとして用いる場合、Go modules スタイルの戦略を採用する：

1. **Release assets 優先**：GitHub Release ページにプラットフォームに適合するビルド済みアーティファクトがあるか確認
2. **main ブランチへのフォールバック**：Release がない場合は git clone

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

# プライベートリポジトリ（credentials.toml 内の GitHub トークンを使用）
private = { git = "https://github.com/my-org/private-lib" }
```

### パッケージフォーマット（.yxpkg）

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # パッケージメタデータ
├── src/                   # ソースコード
├── build/                 # ビルド成果物（存在する場合）
│   └── native/
│       └── linux-x86_64/
│           └── libfoo.so
├── build.yx               # ビルドスクリプト（存在する場合）
└── SHA256SUMS             # チェックサム
```

### publish フロー

```bash
# 公式 Registry へ公開
yaoxiang publish

# 指定 Registry へ公開
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
4. 全ファイルの SHA-256 を計算
5. `.yxpkg`（tar.gz）としてパッケージング
6. Registry へアップロード

### yank セマンティクス

```bash
yaoxiang yank foo@1.2.3
```

**削除 + バージョン番号の永久ロック：**

- パッケージは完全に削除され、復元不可
- バージョン番号は永久に占有され、同一バージョン番号を再公開できない
- 当該バージョンを参照する既存の lockfile はエラーを発生させ、他のバージョンへの更新が必要
- **セキュリティ目的**：npm 型のサプライチェーン攻撃を防止する。攻撃者が削除されたパッケージのバージョン番号を乗っ取って悪意あるコードを注入した事例があり、yank によるバージョン番号ロックはこの経路を完全に断つ。

### 認証モデル

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**マッピングルール：** `yaoxiang login --registry <url>` は URL によって `[registries.*]` の `url` フィールドにマッチする。マッチしない場合、新規エントリを作成する（`reg-1` 等の名称を自動生成）。

**優先順位：** 環境変数 > 設定ファイル

| 環境変数 | 用途 |
|----------|------|
| `$YX_GITHUB_TOKEN` | GitHub 認証 |
| `$YX_REGISTRY_TOKEN` | Registry 認証（デフォルト Registry に使用） |
| `$YX_REGISTRY_URL` | デフォルト Registry アドレス |

**CLI コマンド：**

```bash
yaoxiang login --registry https://yxreg.example.com   # URL でマッチまたは新規作成
yaoxiang login --github                                # GitHub OAuth または token
yaoxiang logout --registry https://yxreg.example.com   # マッチしたエントリを削除
```

**セキュリティ制約：**
- トークンは決して `yaoxiang.toml` または `yaoxiang.lock` に書き込まない
- `credentials.toml` のファイルパーミッションは 600
- CI シナリオでは環境変数、開発シナリオではファイルを使用

## 詳細設計

### RegistrySource 実装

既存スタブコード（`source/mod.rs:150-203`）を置換する：

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

### 依存クレート

| crate | 用途 |
|-------|------|
| `reqwest` | HTTP クライアント |
| `sha2` | SHA-256 検証 |
| `flate2` + `tar` | パッケージフォーマット処理 |
| `async-trait` | async trait サポート |

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

    #[error("SHA-256 検証失敗: 期待値 {expected}, 実際 {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("権限不足: {0}")]
    Forbidden(String),
}
```

## トレードオフ

### 利点

- オープンプロトコルであり、特定のサーバに縛られない
- GitHub を軽量な配布チャネルとして活用でき、参入障壁を低減
- バージョン番号ロックによるセキュリティモデル
- ビルド済みアーティファクト優先のインストール戦略

### 欠点

- 公式 Registry は独立した運用が必要
- GitHub API にはレート制限がある
- バージョン番号ロックによりバージョン番号が浪費される可能性がある

## 代替案

| 案 | 採用しなかった理由 |
|------|-----------|
| GitHub のみをサポート | GitHub エコシステムに制約され、独自 Registry を構築できない |
| Cargo 風の crates.io | 過度に複雑であり、YaoXiang エコシステムの初期段階では不要 |
| npm 風の yank（フラグのみ） | セキュリティリスク、既知のサプライチェーン攻撃事例あり |

## 実装戦略

### 段階分け

| フェーズ | 内容 |
|------|------|
| Phase 3.5 | Source trait を async 化 + async-trait + 全実装の移行 |
| Phase 4a | Registry trait + reqwest 統合 + ローカル Registry モック |
| Phase 4b | GitHub Release アダプタ |
| Phase 4c | publish コマンド + パッケージフォーマットの生成 |
| Phase 4d | 認証 + yank |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ、semver 置換）に依存
- RFC-014b（ビルドシステム、`build/` ディレクトリ処理用）に依存

## オープンクエスチョン

- [ ] Registry API のバージョン管理は必要か（`/api/v1/` vs `/api/v2/`）？
- [ ] パッケージ名は namespace をサポートするか（例：`@org/pkg`）？
- [ ] レート制限の方針は？
- [ ] パッケージサイズの上限は？

---

## 参考文献

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
```