---
title: 'RFC-014a: Registry プロトコル仕様'
status: '審査中'
author: '晨煦'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
---

# RFC-014a: Registry プロトコル仕様

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC です。

## 概要

YaoXiang パッケージ管理システムの Registry プロトコルを定義します：オープンインターフェース設計、公式 Registry 仕様、GitHub アダプタ層、パブリッシュ/撤回フロー、認証モデル。

## 動機

RFC-014 の総括ではパッケージ管理システムの全体アーキテクチャを定義しましたが、Registry の部分については「予約」としか記されていませんでした。Registry プロトコルがないと、パッケージの配布ができません——商店のないショッピングカートの設計のようなものです。

### 現在の問題

- `RegistrySource` はスタブコード（`source/mod.rs:150-203`）で、`resolve` は宣言されたバージョンをそのまま返し、`download` は空のパスを返します
- HTTP クライアントがない（`reqwest` 依存なし）
- パッケージのパブリッシュ機構がない
- 認証/認可がない

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

`Source` trait を unified に async に変更し、tokio を全面的に採用します：

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

すべての実装（`LocalSource`、`GitSource`、`RegistrySource`）を unified に async に変更します。CLI エントリポイントは `#[tokio::main]` または `Runtime::block_on` で駆動します。

**理由：**

- Registry は HTTP リクエストを必要とし、ブロッキングはインストールプロセス全体を停止させます
- 複数依存の並列ダウンロード（`join_all`）でインストール速度が大幅に向上します
- Git clone も I/O 操作であり、async の方がより自然です
- tokio はすでにプロジェクトの依存関係に含まれています

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// パッケージのパブリッシュ
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// パブリッシュされたバージョンの削除（不可逆で、バージョン番号がロックされます）
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// パッケージ情報のクエリ
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// 利用可能なバージョンリストのクエリ
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// パッケージの検索
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// 指定バージョンのダウンロード
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// 認証
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### ソースの優先順位（デフォルトの検索チェーン）

`yaoxiang add foo`（フラグなし）時のデフォルト検索順序：

| 優先順位 | 検索          | 説明                                    |
| -------- | ------------- | --------------------------------------- |
| 1        | グローバルキャッシュ | `~/.yaoxiang/cache/registry/foo-<ver>/` |
| 2        | 公式 Registry | バージョンのクエリ → ダウンロード                         |
| 3        | 失敗          | エラーを出力し、ユーザーがパッケージ名またはネットワークを確認するよう促す            |

**明示的なオーバーライド（デフォルトチェーンをバイパス）：**

| フラグ              | 動作                                                                          |
| ------------------- | ----------------------------------------------------------------------------- |
| `--git <url>`      | Registry をスキップして、直接 Git clone（Release assets を優先 → fallback は tag/branch） |
| `--path <dir>`     | Registry をスキップして、直接ローカルパスを使用                                                 |
| `--registry <url>` | 公式 Registry をスキップして、指定された Registry を使用                                            |

### 公式 Registry

公式 Registry は crates.io に類似しており、パackage 配布の主要チャネルです。

**API エンドポイント：**

| エンドポイント                               | メソッド  | 説明         |
| ---------------------------------------- | ------ | ------------ |
| `/api/v1/packages/{name}`                | GET    | パッケージ情報のクエリ   |
| `/api/v1/packages/{name}/versions`       | GET    | バージョンリストのクエリ |
| `/api/v1/packages/{name}/{version}`      | GET    | パッケージのダウンロード |
| `/api/v1/packages`                       | PUT    | パッケージのパブリッシュ |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | バージョンの撤回     |
| `/api/v1/search?q={query}`               | GET    | パッケージの検索     |
| `/api/v1/login`                          | POST   | 認証         |

### GitHub 統合

GitHub をパッケージソースとして使用する場合、Go modules スタイルの strategy を採用します：

1. **Release assets を優先**：GitHub Release ページに一致するプラットフォームのプリコンパイル成果物があるかをチェック
2. **main ブランチへの Fallback**：Release がない場合は git clone

```toml
[dependencies]
# 基本的な git 依存
foo = { git = "https://github.com/user/foo" }

# バージョンの指定（tag と一致）
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# ブランチの指定
baz = { git = "https://github.com/user/baz", branch = "main" }

# commit の指定
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
# 公式 Registry にパブリッシュ
yaoxiang publish

# 指定された Registry にパブリッシュ
yaoxiang publish --registry my-company

# GitHub Release も同時に作成
yaoxiang publish --github

# ドライラン
yaoxiang publish --dry-run
```

パブリッシュ前のバリデーション：

1. `yaoxiang.toml` には `name`、`version`、`description` がなければならない
2. バージョン番号は既に存在していてはならない
3. テストの実行（オプション、`--no-test` でスキップ可能）
4. 全ファイルの SHA-256 を計算
5. `.yxpkg`（tar.gz）にパッケージ化
6. Registry にアップロード

### yank セマンティクス

```bash
yaoxiang yank foo@1.2.3
```

**削除 + バージョン番号のロック：**

- パッケージは完全に削除され、不可逆
- バージョン番号は永続的に占有され、同じバージョン番号を再パブリッシュできない
- そのバージョンを参照する既存の lockfile を持つプロジェクトはエラーになり、他のバージョンにアップグレードする必要がある
- **セキュリティ目的**：npm 形式のサプライチェーン攻撃を防ぎます。攻撃者はかつて削除されたパッケージバージョン番号を奪い取り、悪意のあるコードを挿入していましたが、yank はバージョン番号をロックすることでこの手法を完全に塞ぎます。

### 認証モデル

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**マッピング規則：** `yaoxiang login --registry <url>` は URL によって `[registries.*]` の `url` フィールドとマッチします。マッチするものがない場合は、新しいエントリを作成します（自動生成された名前、例：`reg-1`）。

**優先順位：** 環境変数 > 設定ファイル

| 環境変数              | 用途                               |
| -------------------- | ---------------------------------- |
| `$YX_GITHUB_TOKEN`   | GitHub 認証                        |
| `$YX_REGISTRY_TOKEN` | Registry 認証（デフォルト Registry 用） |
| `$YX_REGISTRY_URL`   | デフォルト Registry アドレス                 |

**CLI コマンド：**

```bash
yaoxiang login --registry https://yxreg.example.com   # URL でマッチまたは新規作成
yaoxiang login --github                                # GitHub OAuth または token
yaoxiang logout --registry https://yxreg.example.com   # マッチするエントリを削除
```

**セキュリティ制約：**

- Token は絶対に `yaoxiang.toml` や `yaoxiang.lock` に書き込まない
- `credentials.toml` のファイル権限は 600
- CI シナリオでは環境変数を使用し、開発シナリオではファイルを使用

## 詳細な設計

### RegistrySource の実装

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

        // SHA-256 チェックサム
        let actual_hash = sha256_hex(&bytes);
        // ... dest に解凍 ...

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

| crate            | 用途             |
| ---------------- | ---------------- |
| `reqwest`        | HTTP クライアント      |
| `sha2`           | SHA-256 チェックサム     |
| `flate2` + `tar` | パッケージフォーマットの処理       |
| `async-trait`    | async trait のサポート |

### エラータイプ

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("パッケージ '{0}' が存在しません")]
    PackageNotFound(String),

    #[error("バージョン '{0}' が存在しません")]
    VersionNotFound(String),

    #[error("バージョン '{0}' は既に存在します")]
    VersionAlreadyExists(String),

    #[error("認証失敗: {0}")]
    AuthFailed(String),

    #[error("ネットワークエラー: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 チェックサム不一致: 期待 {expected}, 実際 {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("権限不足: {0}")]
    Forbidden(String),
}
```

## トレードオフ

### メリット

- オープンプrotocolであり、特定のサーバーに縛られない
- GitHub を軽量な配布チャネルとして使用でき、参入障壁が低い
- バージョン番号ロックのセキュリティモデル
- プリコンパイル成果物を優先するインストール strategy

### デメリット

- 公式 Registry は独立した運用が必要
- GitHub API にはレート制限がある
- バージョン番号のロックによりバージョン番号の無駄遣いにつながる可能性がある

## 代替案

| 案                    | 採用しなかった理由                            |
| --------------------- | ------------------------------------- |
| GitHub のみサポート           | GitHub エコシステムに依存し、自前で Registry を構築できない     |
| Cargo 形式の crates.io    | 複雑すぎ、YaoXiang エコシステムの初期段階では不要     |
| npm 形式の yank（マークのみ） | セキュリティリスクがあり、已知のサプライチェーン攻撃事例がある          |

## 実装 strategy

### フェーズ分け

| フェーズ      | 内容                                               |
| ----------- | -------------------------------------------------- |
| Phase 3.5 | Source trait を async に変更 + async-trait + 全実装の移行 |
| Phase 4a  | Registry trait + reqwest 統合 + ローカル Registry mock |
| Phase 4b  | GitHub Release アダプタ                                |
| Phase 4c  | publish コマンド + パッケージフォーマットの打包                          |
| Phase 4d  | 認証 + yank                                        |

### 依存関係

- RFC-014 Phase 3（グローバルキャッシュ、semver 置換）に依存
- RFC-014b（ビルドシステム、`build/` ディレクトリの処理用）に依存

## 未解決の問題

- [ ] Registry API はバージョン化管理が必要か（`/api/v1/` vs `/api/v2/`）？
- [ ] パッケージ名は namespace をサポートするか（例：`@org/pkg`）？
- [ ] レート制限 strategy は？
- [ ] パッケージサイズの上限は？

---

## 参考文献

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
