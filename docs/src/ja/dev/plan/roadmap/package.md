title: "パッケージ管理状態"
```

# パッケージ管理（Package）

> **モジュール状態**：安定（4項の改善待ち）
> **位置**：`src/package/`
> **最終更新**：2026-06-01

---

## モジュール概要

パッケージ管理モジュールは、プロジェクトの依存関係管理、パッケージ設定の解析、依存関係のダウンロードを担当します。RFC-014で定義されたPhase 1（toml解析、ローカル依存、lock生成）とPhase 2（GitHubサポート、.yaoxiang/vendor管理、ダウンロードツール）を実装しています。

**コード量**：約5000行（23のソースファイル）

---

## 機能一覧

### 実装済みの機能（12項目）

1. ✅ **yaoxiang.toml マニフェストファイル** — パッケージメタデータ（name, version, description, authors, license）、依存関係宣言（dependencies / dev-dependencies）、TOML直列化/逆直列化
2. ✅ **yaoxiang.lock ロックファイル** — 依存関係エントリのロック（version, source, checksum）、マニフェストからの同期、強制更新、期限切れ依存関係のクリーンアップ
3. ✅ **依存関係仕様解析 (DependencySpec)** — TOML値からの解析（文字列形式 `"1.0.0"` とテーブル形式 `{version, git, path}`）
4. ✅ **セマンティックバージョニング解析 (SemVer / VersionReq)** — `major.minor.patch[-pre]` 形式の解析、オペレータ `^`, `~`, `>=`, `>`, `<=`, `<`, 完全一致, `*` のサポート
5. ✅ **依存関係ソースの抽象化 (Source trait)** — `LocalSource`（ローカルパス）、`GitSource`（Gitリポジトリのクローン、tag/branch/revサポート）、`RegistrySource`（プレースホルダ、Phase 3）
6. ✅ **Git依存関係サポート** — URL解析（`?tag=`, `?branch=`, `?rev=` パラメータ）、`git ls-remote` タグリスト取得、semverタグマッチング、`git clone --depth 1` シャロークローン
7. ✅ **バージョン衝突検出** — 同一パッケージの非互換バージョン要求を自動検出
8. ✅ **モジュールリゾルバ (ModuleResolver)** — 優先順位による検索：vendor -> src -> YXPATH -> std
9. ✅ **Vendorディレクトリ管理 (VendorManager)** — `.yaoxiang/vendor/<name>-<version>/` ディレクトリ管理、インストール/アンインストール/一覧表示/クリーンアップ
10. ✅ **SHA-256チェックサム** — 自行実装のインラインSHA-256（外部依存なし）、ファイルレベルとディレクトリレベルのチェックサム
11. ✅ **バッチダウンローダ (fetcher)** — 統一された依存関係ダウンロードインターフェース、source/vendor/lockとの統合
12. ✅ **CLIコマンド（6つ）** — `init`、`add`、`rm`、`install`、`list`、`update`

### RFCで言及されているが未実装の機能（3項目）

- ❌ `outdated` コマンド — 古くなった依存関係のチェック
- ❌ `clean` コマンド — ビルド成果物のクリーンアップ（vendorレベルのみcleanメソッドあり）
- ❌ `task <name>` コマンド — カスタムタスクの実行

---

## テストカバレッジ

**137個のテスト、全て合格**

- 各モジュールに完全なユニットテストあり
- カバー範囲：正常解析、直列化往返、CRUD操作、エラーケース、エッジケース、決定論的チェックサム
- テストには `tempfile::TempDir` を使用してファイルシステム操作を隔離

---

## RFC比較（RFC-014）

### RFCに完全準拠の部分

- ✅ yaoxiang.toml 形式（[package], [dependencies], [dev-dependencies]）
- ✅ プロジェクト構造（src/, .yaoxiang/vendor/, yaoxiang.toml, yaoxiang.lock）
- ✅ モジュール解析順序（vendor -> src -> YXPATH -> std）
- ✅ Source trait拡張可能アーキテクチャ（Local, Git, Registry の3ソース）
- ✅ CLIコマンド（init, add, rm, install, update, list）
- ✅ セマンティックバージョニング（^, ~, 完全一致, 範囲オペレータ）

### RFCとの相違点

1. **ロックファイル形式の微調整** — RFCは `[[package]]` 配列形式を使用、実装では `[package.name]` map形式を使用、機能的に同等
2. **RFCを超えた設計** — 自動バージョン衝突検出、インラインSHA-256実装、initコマンドが追加で `.yaoxiang/std/` 標準ライブラリインターフェースファイルを生成

### 将来の拡張（Phase 3、RFCで「予約」と标注されたもの）

- ❌ Registryレジストリソース — プレースホルダ実装のみ
- ❌ ワークスペース（workspace）サポート
- ❌ 依存関係オーバーライド（override）メカニズム

---

## コード品質評価

| 次元 | 評価 | 説明 |
|------|------|------|
| 未完了事項 | 4 | outdated、clean、task コマンド、Phase 3 Registry |
| テストカバレッジ | 優秀 | 137個のテスト、全て合格 |
| ドキュメント品質 | 良好 | 全モジュールに `//!` ドキュメントコメント、公開関数に `///` ドキュメント |
| コードアーキテクチャ | 優秀 | commands/source/vendor/template の階層化が清晰 |
| RFC準拠 | 高度に準拠 | ロックファイル形式の微調整のみ |

---

## 改善待ち項目

1. **`outdated` コマンドの実装**
2. **`clean` CLIコマンドの実装**
3. **`task <name>` カスタムタスクの実装**
4. **Phase 3の開始：Registryレジストリソース**