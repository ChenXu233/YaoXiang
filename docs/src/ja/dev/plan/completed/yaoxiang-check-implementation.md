# yaoxiang check コマンド実装記録（実装済み）

## 概要

`yaoxiang check` はCIおよびローカルでの素早い静的チェックに使用できるコマンドに拡張されました：マルチパス入力、テキスト/JSON出力、watch再検査、统一的なエラー计数に対応しています。

本文書は「実装計画」から「実装記録 + 今後の改善事項」に更新され、現在のコードと一致しています。

## 実装済み機能

- 1つまたは複数の入力パス（ファイルまたはディレクトリ）に対応：`yaoxiang check <PATH...>`
- ディレクトリからの`.yx`ファイルの再帰的収集
- `--json`出力による構造化診断に対応
- `--watch`による監視と自動再検査に対応
- `--color auto|always|never`に対応
- `--no-progress`による進捗/サマリー情報の停止に対応
- エラー時は終了コード`1`、成功時は終了コード`0`、入力に有効なファイルがない場合は終了コード`2`

## CLIパラメータ（現在）

`yaoxiang check [OPTIONS] <PATH>...`

- `--json`: JSON出力
- `--watch` / `-w`: ファイル変更を監視
- `--color <auto|always|never>`: カラー出力の制御
- `--no-progress`: 進捗とサマリーの出力を停止

## コア実装

### 1) CLI拡張

- `Commands::Check`は単一ファイルパラメータからマルチパスパラメータ`paths: Vec<PathBuf>`にアップグレード
- `json/watch/color/no_progress`オプションを追加
- チェックフローの分離：
  - `run_check_once(...)`
  - `run_check_watch(...)`

### 2) 診断集計

`util/diagnostic/mod.rs`に新規追加：

```rust
pub struct CheckDiagnostic {
    pub file: String,
    pub diagnostic: Diagnostic,
}

pub struct CheckResult {
    pub diagnostics: Vec<CheckDiagnostic>,
    pub source_files: HashMap<String, SourceFile>,
    pub error_count: usize,
    pub warning_count: usize,
}

pub fn check_files_with_diagnostics(files: &[PathBuf]) -> anyhow::Result<CheckResult>
```

動作：
- すべての入力ファイルを一個ずつコンパイルして診断を集計
- 「エラー即停止」を不要再化
- `check_file_with_diagnostics`の互換入口を保持（内部でマルチファイル実装を再利用）

### 3) 出力フォーマット

- テキスト出力：`TextEmitter`を使用し、カラー切り替えに対応
- JSON出力：以下を含みます
  - `error_count`
  - `warning_count`
  - `diagnostics[]`（`file/severity/code/message/line/column/...`を含む）
  - `lsp`フィールド（`JsonEmitter`に変換）

### 4) Watchモード

- `notify`ベースの`RecommendedWatcher`に依存
- 入力パス（ディレクトリは再帰的、ファイルは非再帰的）を監視
- `.yx`関連イベントのみチェックをトリガー
- デバウンスウィンドウ（250ms）を追加

## 元計画との差異

1. 現在のwatchは「デバウンス後全量再検査」を採用しており、「変更ファイルのみ再検査 + キャッシュ増分結果」は未実装です。
2. 現在のコンパイルパイプラインでは、失敗したファイルごとに主に最初の構造化診断のみ公開しており、「単一ファイルに対する完全診断リスト 반환」は未完了です。
3. クロースファイルグローバルシンボル統合分析は、`check`コマンドでは追加実装されておらず（既存のコンパイラ動作に依存）。

## 検証結果

検証完了：

- `cargo check --bin yaoxiang`が通過
- ユニットテストが通過：
  - `test_check_files_with_diagnostics_ok`
  - `test_check_files_with_diagnostics_error`
- スモークテストが通過：
  - `cargo run -- check tests/yaoxiang/list_test.yx --json --no-progress`
  - 一時的なエラーファイルのチェックは終了コード`1`を返し、ファイル名、行番号、列番号、レベル、メッセージが出力される

## 今後の提案

1. `check --watch`に増分キャッシュを追加し、毎回全量スキャンを回避する。
2. フロントエンド/パイプライン層でエラー収集を拡張し、各ファイルの複数診断の完全出力をサポートする。
3. 終了コード、JSON構造、ディレクトリ入力、watch動作をカバーするCLI統合テスト（プロセスレベル）を追加する。