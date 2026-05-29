# ランタイム詳細エラートラッキング（フェーズ 2）：Span カバー、多ファイル SourceMap、Bytecode DebugSection、CLI スイッチ

> 状態：計画中（実装と受入駆動用）  
> 関連：`detailed_runtime_errors.md`（フェーズ 1 で既に実装されたコアチェーン）

## 目標（本章で解決する 4 つの制約）
1. **Span カバー範囲**：より多くの IR 命令に `Span` を補完し、Codegen フェーズで DebugMap に組み込む。
2. **複数ファイル/モジュール**：DebugMap にソースファイルを記録；`SourceMap`（`file_id -> path/content`）を導入し、跨モジュール定位とレンダリングをサポート。
3. **バイトコードファイル DebugSection**：`.42`（BytecodeFile）フォーマットを拡張し、オプションの DebugSection を追加；オフライン実行/デバッグ時に定位情報を保持するための読み書きを実装。
4. **CLI スイッチ**：`run` / `build`（build-bytecode）コマンドに `--debug-info` を追加し、DebugMap の生成と（本フェーズで実装する）シリアライズを制御。

## 設計制約（必ず満たすこと）
- **インタープリタメインループのゼロコスト**：ランタイム実行ロジックは `ip` と `StackFrame` のバブリングのみに依存し、ホットパスでデバッグ情報にアクセス/解析しない。
- **オンデマンドレンダリング**：エラーが最終的に CLI 層にバブリングされたときのみ、ソースコードのロード/ルックアップ/レンダリングを行う。
- **オプショナルスイッチ**：`--debug-info` をオフにすると DebugMap が生成されず、DebugSection も書き込まれない。余分なアロケーションとファイル肥大化を避ける。

## データ構造方案（破壊的変更の回避）
既存の `Span`（現在、多くのコンポーネントが `Span { start, end }` に依存）に `file_id` を侵入させることを避けるため、本フェーズでは「コンポジション式定位」を採用：

- `Span` は引き続き行/列/オフセットのみを表現（現状維持）。
- 新たに `FileId`（整数 ID）と `DebugSpan { file_id, span }` を追加。
- 新たに `SourceMap`：`file_id` で `SourceFile { name, content, ... }` をインデックス。
- `BytecodeFunction.debug_map` の value を `Span` から `DebugSpan` にアップグレード。

> 好处：不會大面積重寫 Parser/LSP/TypeCheck の Span ロジック；デバッグチェーンのみアップグレード。

## BytecodeFile DebugSection（ファイルフォーマット拡張）
### トリガー条件
- `BytecodeFile.header.flags` に `DEBUG_INFO` ビットを設定（Codegen/CLI と揃える）。

### 存储内容（最小闭环）
- **SourceMap**：`file_count`、各ファイルに対して `path` と `content` を書き込み。
- **DebugMap**：関数単位で `ip -> DebugSpan` マッピングを格納：
  - `entry_count`
  - 各エントリ：`ip`、`file_id`、`Span.start(line/col/offset)`、`Span.end(line/col/offset)`

### 兼容性策略
- `DEBUG_INFO` がオフ：DebugSection を書き込まず、旧フォーマットは変更なし。
- 読み込み：flags がオフの場合は DebugSection をスキップ；オンの場合は解析して `FunctionCode.debug_map` に填充。

## CLI スイッチの動作（受入基準）
### `yaoxiang run <file> --debug-info`
- Codegen の DebugMap 収集を有効化。
- ランタイムエラー出力にソースコードスニペットのハイライトと、（複数ファイルの場合は）正しいファイル名/パスを 포함。

### `yaoxiang build <file> -o out.42 --debug-info`
- DebugSection（SourceMap + DebugMap を含む）を書き込み。
- スイッチをオフにすると `.42` には DebugSection が含まれない。

## Span カバー範囲の拡張（推奨優先度）
> 「ランタイムエラーをトリガーする可能性のある」IR 命令を優先して徐々に補完。

### P0（既存のエラー型と直接関連）
- `Call/CallVirt/CallDyn`：`FunctionNotFound`
- `Div/Mod`：`DivisionByZero`

### P1（将来のエラー型/チェックの予約）
- `LoadIndex/StoreIndex`：`IndexOutOfBounds`（将来的に `BoundsCheck` と組み合わせ）
- `LoadField/StoreField`：`FieldNotFound`
- `PtrDeref/PtrLoad/PtrStore`：`InvalidHandle` / unsafe エラー

## テストと受入
- `BytecodeFile`：DebugSection の書き込み後、`read_from` で SourceMap と DebugMap を完全に復元（round-trip）。
- `render_runtime_error`：`DebugSpan.file_id` に基づいて SourceMap から正しいファイルを選択してレンダリング（単一ファイル/複数ファイルのユースケース各 1 個）。
- CLI：`--debug-info` スイッチが 출력을明確に変化させる（オン：ソースコードハイライト；オフ：ip/関数名のみ）。