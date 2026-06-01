```markdown
---
title: "パッケージ管理状態"
---

# パッケージ管理（Package）

> **モジュール状態**：完成（Phase 1 + Phase 2）
> **位置**：`src/package/`
> **最終更新**：2026-06-01

---

## モジュール概要

パッケージ管理モジュールは、プロジェクトの依存関係管理、パッケージ設定の解析、依存関係のダウンロードを担当します。RFC-014で定義されたPhase 1（toml解析、ローカル依存関係、lock生成）とPhase 2（GitHubサポート、.yaoxiang/vendor管理、ダウンロードツール）を実装しています。

**コード量**：約5000行（23個のソースファイル）

---

## 機能一覧

### 実装済みの機能（12項目）

1. ✅ **yaoxiang.toml マニフェストファイル** — パッケージメタデータ（name, version, description, authors, license）、依存関係宣言（dependencies / dev-dependencies）、TOMLシリアライズ/デシリアライズ
2. ✅ **yaoxiang.lock ロックファイル** — 依存関係のエントリをロック（version, source, checksum）、マニフェストからの同期、強制更新、期限切れの依存関係のクリーンアップ
3. ✅ **依存関係仕様解析 (DependencySpec)** — TOML値からの解析（文字列形式 `"1.0.0"` とテーブル形式 `{version, git, path}`）
4. ✅ **セマンティックバージョニング解析 (SemVer / VersionReq)** — `major.minor.patch[-pre]` 形式の解析、オペレータ `^`, `~`, `>=`, `>`, `<=`, `<`, 完全一致, `*` をサポート
5. ✅ **依存関係ソース抽象化 (Source trait)** — `LocalSource`（ローカルパス）、`GitSource`（Gitリポジトリのクローン、tag/branch/rev対応）、`RegistrySource`（プレースホルダー、Phase 3）
6. ✅ **Git依存関係サポート** — URL解析（`?tag=`, `?branch=`, `?rev=` パラメータ）、`git ls-remote` タグリスト取得、semverタグマッチング、`git clone --depth 1` シャロークローン
7. ✅ **バージョンの競合検出** — 同一パッケージの互換性のないバージョン要求を自動検出
8. ✅ **モジュールリゾルバー (ModuleResolver)** — 優先順位での検索：vendor -> src -> YXPATH -> std
9. ✅ **Vendorディレクトリ管理 (VendorManager)** — `.yaoxiang/vendor/<name>-<version>/` ディレクトリ管理、インストール/アンインストール/一覧表示/クリーンアップ
10. ✅ **SHA-256チェックサム** — 外部依存関係なしのインラインSHA-256実装（ファイルとディレクトリレベルのチェックサム）
11. ✅ **一括ダウンローダー (fetcher)** — 統一された依存関係ダウンロードインターフェース、source/vendor/lockとの統合
12. ✅ **CLIコマンド（6個）** — `init`、`add`、`rm`、`install`、`list`、`update`

### RFCで言及されているが未実装の機能（3項目）

- ❌ `outdated` コマンド — 古くなった依存関係のチェック
- ❌ `clean` コマンド — ビルド成果物のクリーンアップ（vendorレベルのみcleanメソッドあり）
- ❌ `task <name>` コマンド — カスタムタスクの実行

---

## テストカバレッジ

**137件のテスト、すべて通過**

- 各モジュールに完全なユニットテストあり
- カバー範囲：正常解析、シリアライズ往返、CRUD操作、エラーケース、エッジケース、決定性チェックサム
- テストは `tempfile::TempDir` を使用してファイルシステム操作を隔離

---

## RFC比較（RFC-014）

### RFCに完全準拠の部分

- ✅ yaoxiang.toml形式（[package], [dependencies], [dev-dependencies]）
- ✅ プロジェクト構造（src/, .yaoxiang/vendor/, yaoxiang.toml, yaoxiang.lock）
- ✅ モジュール解析順序（vendor -> src -> YXPATH -> std）
- ✅ Source trait拡張可能アーキテクチャ（Local, Git, Registryの3種類のソース）
- ✅ CLIコマンド（init, add, rm, install, update, list）
- ✅ セマンティックバージョニング（^, ~, 完全一致, 範囲オペレータ）

### RFCとの差異

1. **ロックファイル形式の微調整** — RFCでは `[[package]]` 配列形式を使用、実装では `[package.name]` マップ形式を使用、機能的に同等
2. **RFCを超える設計** — 自動バージョンの競合検出、インラインSHA-256実装、initコマンドが追加で `.yaoxiang/std/` 標準ライブラリインターフェースファイルを生成

### 将来の拡張（Phase 3、RFCで「予約」と标注されたもの）

- ❌ Registryレジストリソース — プレースホルダーのみ実装
- ❌ ワークスペース（workspace）サポート
- ❌ 依存関係のオーバーライド（override）メカニズム

---

## コード品質評価

| 観点 | 評価 | 説明 |
|------|------|------|
| 機能完成度 | 100% | Phase 1 + Phase 2すべて完成 |
| テストカバレッジ | 優秀 | 137件のテスト、すべて通過 |
| ドキュメント品質 | 良好 | 全モジュールに `//!` ドキュメントコメントあり、公開関数に `///` ドキュメントあり |
| コードアーキテクチャ | 優秀 | commands/source/vendor/templateの階層が清晰 |
| RFC準拠 | 高度に準拠 | ロックファイル形式の微調整のみ |

---

## 改善待望項目

1. **`outdated` コマンドの実装**
2. **`clean` CLIコマンドの実装**
3. **`task <name>` カスタムタスクの実装**
4. **Phase 3の開始：Registryレジストリソース**
```