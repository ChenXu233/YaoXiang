---
title: "RFC-014a: Registry プロトコル仕様"
status: "レビュー中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
---

# RFC-014a: Registry プロトコル仕様

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC です。

## 概要

YaoXiang パッケージ管理システムの Registry プロトコルを定義します：オープンインターフェース設計、公式 Registry 仕様、GitHub アダプタ層、パッケージ公開/撤回フロー、認証モデル。

## 背景

RFC-014 の総論ではパッケージ管理システムの全体アーキテクチャを定義しましたが、Registry の部分は「予約」としてしかマークされていません。Registry プロトコルがないと、パケットの配信ができません——これは商店のないショッピングカートの設計するようなものです。

### 現在の問題

- `RegistrySource` はスタブコードです（`source/mod.rs:150-203`）。`resolve` は宣言されたバージョンをそのまま返し、`download` は空のパスを返します
- HTTP クライアントがありません（`reqwest` 依存なし）
- パッケージ公開メカニズムがありません
- 認証/認可がありません

## 提案

### コア設計：オープンプrotocol + アダプタ層

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

`Source` trait を一律で async に変更し、tokio を全面的に採用します：

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

すべての実装（`LocalSource`、`GitSource`、`RegistrySource`）を一律で async に変更します。CLI エントリポイントは `#[tokio::main]` または `Runtime::block_on` で駆動します。

**理由：**
- Registry は HTTP リクエストを必要とし、ブロッキングはインストールプロセス全体を停止させます
- 複数依存の並列ダウンロード（`join_all`）でインストール速度が大幅に向上します
- Git clone も I/O 操作なので、async の方がより自然です
- tokio はすでにプロジェクト依存関係に含まれています

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// パッケージを公開する
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// 公開済みバージョンを削除する（取り消し不可、バージョン番号は固定）
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// パッケージ情報をクエリする
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// 利用可能なバージョン一覧をクエリする
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// パッケージを検索する
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// 指定バージョンをダウンロードする
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// 認証
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### ソースの優先順位（デフォルト検索チェーン）

`yaoxiang add foo`（flag なし）時のデフォルト検索順序：

| 優先度 | 検索場所 | 説明 |
|--------|------|------|
| 1 | グローバルキャッシュ | `~/.yaoxiang/cache/registry/foo-<ver>/` |
| 2 | 公式 Registry | バージョン查询 → ダウンロード |
| 3 | 失敗 | エラーを返し、ユーザーにパッケージ名またはネットワークを確認するよう促す |

**明示的な上書き（デフォルトチェーンをスキップ）：**

| flag | 動作 |
|------|------|
| `--git <url>` | Registry をスキップして、直接 Git clone（Release assets を優先 → tag/branch にフォールバック） |
| `--path <dir>` | Registry をスキップして、ローカルパスを使用 |
| `--registry <url>` | 公式 Registry をスキップして、指定 Registry を使用 |

### 公式 Registry

公式 Registry は crates.io 类似で、パッケージ配信の主要な渠道です。

**API エンドポイント：**

| エンドポイント | メソッド | 説明 |
|------|------|------|
| `/api/v1/packages/{name}` | GET | パッケージ情報をクエリ |
| `/api/v1/packages/{name}/versions` | GET | バージョン一覧をクエリ |
| `/api/v1/packages/{name}/{version}` | GET | パッケージをダウンロード |
| `/api/v1/packages` | PUT | パッケージを公開 |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | バージョンを撤回 |
| `/api/v1/search?q={query}` | GET | パッケージを検索 |
| `/api/v1/login` | POST | 認証 |

### GitHub 統合

GitHub をパッケージソースとして使用する場合、Go modules スタイルのアプローチを採用します：

1. **Release assets を優先**：GitHub Release ページに一致するプラットフォームのプリコンパイル成果物があるかをチェック
2. **main ブランチへのフォールバック**：Release がない場合は git clone

```toml
[dependencies]
# 基本的な git 依存
foo = { git = "https://github.com/user/foo" }

# バージョンを指定（tag に一致）
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# ブランチを指定
baz = { git = "https://github.com/user/baz", branch = "main" }

# commit を指定
qux = { git = "https://github.com/user/qux", rev = "abc123" }

# プライベートリポジトリ（credentials.toml の GitHub token を使用）
private = { git = "https://github.com/my-org/private-lib" }
```

### パッケージフォーマット（.yxpkg）

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # パッケージメタデータ
├── src/                   # ソースコード
├── build/                 # ビルド成果物（該当する場合）
│   └── native/
│       └── linux-x86_64/
│           └── libfoo.so
├── build.yx               # ビルドスクリプト（該当する場合）
└── SHA256SUMS             # チェックサム
```

### publish フロー

```bash
# 公式 Registry に公開
yaoxiang publish

# 指定 Registry に公開
yaoxiang publish --registry my-company

# GitHub Release も作成
yaoxiang publish --github

# ドライラン
yaoxiang publish --dry-run
```

公開前の検証：
1. `yaoxiang.toml` には `name`、`version`、`description` がなければならない
2. バージョン番号がすでに存在していてはならない
3. テストを実行（オプション、`--no-test` でスキップ可能）
4. すべてのファイルの SHA-256 を計算
5. `.yxpkg`（tar.gz）にパッケージング
6. Registry にアップロード

### yank 意味論

```bash
yaoxiang yank foo@1.2.3
```

**削除 + バージョン番号の固定：**

- パッケージは完全に削除され、取り消し不可
- バージョン番号は永久に占有され、同じバージョン番号を再公開できない
- 該当バージョンを参照する既存の lockfile を持つプロジェクトはエラーになり、他のバージョンにアップグレードする必要がある
- **セキュリティ目的**：npm 型のサプライチェーン攻撃を防止。攻撃者は削除されたパッケージのバージョン番号を先取りして悪意のあるコードを注入していたが、yank でバージョン番号を固定することでこの手法を完全に塞ぎます。

### 認証モデル

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**マッピングルール：** `yaoxiang login --registry <url>` は URL で `[registries.*]` の `url` フィールドにマッチさせる。マッチしない場合、新しいエントリを作成する（自動生成された名前、 例：`reg-1`）。

**優先順位：** 環境変数 > 設定ファイル

| 環境変数 | 用途 |
|----------|------|
| `$YX_GITHUB_TOKEN` | GitHub 認証 |
| `$YX_REGISTRY_TOKEN` | Registry 認証（デフォルト Registry 用） |
| `$YX_REGISTRY_URL` | デフォルト Registry アドレス |

**CLI コマンド：**

```bash
yaoxiang login --registry https://yxreg.example.com   # URL でマッチまたは新規作成
yaoxiang login --github                                # GitHub OAuth または token
yaoxiang logout --registry https://yxreg.example.com   # マッチしたエントリを削除
```

**セキュリティ制約：**
- Token は決して `yaoxiang.toml` や `yaoxiang.lock` に書き込まない
- `credentials.toml` のファイル権限は 600
- CI シナリオでは環境変数を使用し、開発シナリオではファイルを使用

## 詳細な設計

### RegistrySource 実装

既存のスタブコード（`source/mod.rs:150-203`）を置き換えます：

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

        // SHA-256 チェック
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

| crate | 用途 |
|-------|------|
| `reqwest` | HTTP クライアント |
| `sha2` | SHA-256 チェックサム |
| `flate2` + `tar` | パッケージフォーマットの処理 |
| `async-trait` | async trait サポート |

### エラータイプ

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("パッケージ '{0}' が存在しません")]
    PackageNotFound(String),

    #[error("バージョン '{0}' が存在しません")]
    VersionNotFound(String),

    #[error("バージョン '{0}' はすでに存在します")]
    VersionAlreadyExists(String),

    #[error("認証失敗: {0}")]
    AuthFailed(String),

    #[error("ネットワークエラー: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 チェックサム不一致: 期待値 {expected}, 実際 {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("権限不足: {0}")]
    Forbidden(String),
}
```

## トレードオフ

### メリット

- オープンプrotocolで、特定のサーバーに縛られない
- GitHub を軽量な配信渠道として使用でき、参入障壁が低い
- バージョン番号固定のセキュリティモデル
- プリコンパイル成果物を優先したインストール戦略

### デメリット

- 公式 Registry は独立した運用が必要
- GitHub API にはレート制限がある
- バージョン番号の固定により、バージョン番号の浪費が発生する可能性がある

## 代替案

| 案 | 選択しなかった理由 |
|------|-----------|
| GitHub のみサポート | GitHub エコシステムに制限され、カスタム Registry を構築できない |
| Cargo 式の crates.io | 複雑すぎ、YaoXiang エコシステムの初期段階では不要 |
| npm 式の yank（マークのみ） | セキュリティリスクがあり、既知のサプライチェーン攻撃事例がある |

## 実装戦略

### フェーズ区分

| フェーズ | 内容 |
|------|------|
| Phase 3.5 | Source trait を async に変更 + async-trait + 全実装の移行 |
| Phase 4a | Registry trait + reqwest 統合 + ローカル Registry モック |
| Phase 4b | GitHub Release アダプタ |
| Phase 4c | publish コマンド + パッケージフォーマットのパッキング |
| Phase 4d | 認証 + yank |

### 依存関係

- RFC-014 Phase 3 に依存（グローバルキャッシュ、semver 置換）
- RFC-014b に依存（ビルドシステム、`build/` ディレクトリ処理用）

## オープンクエスチョン

- [ ] Registry API はバージョン化管理が必要か（`/api/v1/` vs `/api/v2/`）？
- [ ] パッケージ名は namespace をサポートするか（例：`@org/pkg`）？
- [ ] レート制限戦略は？
- [ ] パッケージサイズの上限は？

---

## 参考文献

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)