# ランタイム詳細エラートラッキング：分離ストレージと遅延レンダリング（Detached Storage & Deferred Loading）

> 状態：コアチェーンは実装済み（2026-03-15）；DebugSection / CLI スイッチは実装済み（2026-03）；より包括的な Span カバーは今後の課題として残されている。

## 1. 背景と目標
現在の姚相 (YaoXiang) インタープリタがランタイムエラー（例：`Runtime error: Function not found: int.to_string`）をスローする際、関数名と命令ポインタ（IP）のみを 출력できます。例えば `at main (ip: 0)` です。これにより、開発者はエラーを具体的なソースコードの行と列に直感的に紐付けることが困难になります。

Rust コンパイラのような华丽で詳細なコードハイライト付きエラー表示を提供的同时に、**決してコア解釈実行ループに时间オーバーヘッドとメモリ使用量を増加させてはいけません**。本計画では「分離ストレージと遅延加载（Detached Storage & Deferred Loading）」アーキテクチャを提案します。

## 2. コア設計原則

1. **ゼロランタイムコスト（Zero-Cost Runtime）**：仮想マシンの実行ロジック（Interpreter/Runtime）は複雑なソースファイル関連オブジェクト（Span、ファイル識別子など）を決して处理せず、異常発生時のポインタ `ip`（Instruction Pointer）のみを保持します。

2. **サイドテーブル分離ストレージ（Side-Table Storage）**：生成された `Span` 追踪情報は専用の `DebugSection / DebugMap` 構造に存储され、ホットスポットコードキャッシュに参加して命令読み取りパフォーマンスを低下させることを防ぎます。

3. **需要遅延レンダリング（Deferred Rendering）**：ソースコードフラグメントのアライメントとレンダリングは、「正確に异常が発生し、外向きにバブルアップしてトリガーポイント（CLI/TUI）に到达したその一瞬」にのみ処理されます。正常実行中には「万一出错に備えて」文字列を拼み立てる行动は発生しません。

## 3. 現在の実装概要（已実装）

- **DebugMap の贯通**：Codegen 段階でオプションとして `ip -> Span` マッピングを生成し、ランタイムの `BytecodeFunction.debug_map` に透過させます。

- **インタープリタのゼロオーバーヘッド**：インタープリタの主実行ループは `Span/debug_map` にアクセスしません。ランタイムエラーは引き続き `StackFrame { function_name, ip }` のみに依存してバブルアップします。

- **トップレベルの遅延レンダリング**：CLI トップレベルが `ExecutorError` をキャッチした後、`debug_map` を使用して `ip` を `Span` にマッピングし、既存の `TextEmitter` を呼び出してコードフラグメントハイライト付きの诊断を出力し、stack trace を附加します。

## 4. 具体的な実装ポイント（原計画の段階に対応）

### 段階 1：バイトコードデータ構造の拡張（Middle Core / Bytecode）✅
`BytecodeFunction` にはすでに `debug_map: HashMap<usize, Span>` フィールドが含まれており、今回の実装で Codegen から Runtime へのデータリンクが打通されました：

```rust
pub struct BytecodeFunction {
    // ... 既存のロジック
    /// Debug info: mapping from IP to source Span
    pub debug_map: std::collections::HashMap<usize, crate::util::span::Span>,
}
```

補足：Codegen 側の `FunctionCode` にも同名のフィールドが追加され、`BytecodeModule::from(BytecodeFile)` 変換時に `BytecodeFunction` に透過されます。

*今後*：バイトコードファイルのシリアライズ/デシリアライズ（`src/middle/passes/codegen/bytecode.rs`）の DebugSection 書き込み/読み取りはまだ実装されていません。

### 段階 2：中間コード生成期のマッピング収集（Codegen Translation）✅
`src/middle/passes/codegen/translator.rs` のワークフローにおいて：

- `FunctionCode.instructions.push(...)` に書き込む前に、`instructions.len()` を生成予定の `ip` として使用します。
- 現在の IR 命令から `Span` を抽出し、`debug_map[ip] = span` に書き込みます。
- `CodegenContext::set_generate_debug_info(bool)` により DebugMap 生成を制御します（关闭時、各関数には空の `HashMap` のみが保持され、追加割り当てを回避）。

現在精确定位可能な IR 命令（`Span` を持つ）：
- `Call / CallVirt / CallDyn`（関数呼び出し関連のランタイムエラー，如 `FunctionNotFound`）
- `Div / Mod`（除算エラーなど）
- `Store / StoreField / StoreIndex`（将来の更多なランタイムエラー種别向け予約）

上記のマッピングをサポートするため、IR 層は `src/middle/core/ir.rs` で `Call/CallVirt/CallDyn/Div/Mod` に `span: Span` フィールドを補完し、`src/middle/core/ir_gen.rs` で値を投入します（インタープリタのランタイムパフォーマンスに影響しません）。

### 段階 3：ランタイムスタックの軽量バブルアップ（Interpreter Execution）✅

- `src/backends/mod.rs` の `StackFrame` データを変更せずに維持します。携带するのは：
  ```rust
  pub struct StackFrame {
      pub function_name: String,
      pub ip: usize,
  }
  ```
- 例外（除算エラー、`FunctionNotFound` など）に遭遇すると、`ExecutorError` が発生し、軽量の `StackFrame` を連れて层层返回弹栈（Err mapping）します。

### 段階 4：最上位レベルのインタ셉トとハイライト付き遅延レンダリング（CLI / Diagnostics）✅
`src/util/diagnostic/mod.rs` に `render_runtime_error(...)` を新增し、`run_file_with_diagnostics(...)` でランタイムエラーをキャッチした後、呼び出します：

1. 最終的にスローされた `ExecutorError` をキャッチし、`stack_trace()` を解开します。
2. クラッシュをトリガーしたフレーム情報（第一フレーム）を取得し、その中の `function_name` と `ip` を取得します。
3. 現在の `BytecodeModule` で対応する `BytecodeFunction` を特定し、`debug_map.get(&ip)` を使用して `Span` を取り戻します。
4. **遅延レンダリング**：エラー発生時のみ `TextEmitter` を呼び出してコードフラグメントハイライトをレンダリングし、stack trace テキストを追加します。

説明：現在の `Span` は `file_id` を携带せず、単一ファイルエントリは CLI トップレベルでソースコードを読み込み、`SourceFile` を構築します（コンパイル所需）。ランタイムエラーレンダリングはこの `SourceFile` を再利用し、IO/文字列処理 interprinter の主ループへの参加を防ぎます。

---

## 5. タスク分解リスト（Checklist）

- [x] **データモデル準備**：`BytecodeFunction.debug_map` と `FunctionCode.debug_map` のデータリンク贯通。

- [x] **コード生成アライメント**：Translator がバイト碼を生成する際に `ip` ごとに `Span` を収集（オプションスイッチ付き）。

- [x] **設定とオプションシリアライズ**：`CodegenContext::set_generate_debug_info(bool)` + `.42` DebugSection 读写 + CLI `--debug-info` は実装済み（2026-03）。

- [x] **トップレベルエラーキャッチ層の構築**：`run_file_with_diagnostics` が `ExecutorError` をキャッチし、`render_runtime_error` を呼び出す。

- [x] **ターミナルレンダリング**：`TextEmitter` を使用してソースコードフラグメントハイライト付きのランタイムエラーを出力し、stack trace を附加。

## 6. 既知の制限事項と今後の課題

1. **Span カバー範囲**：段階 1 の基础上、`LoadField/LoadIndex` に Span を扩展し（DebugMap にも含める）ました。引き続き、より多くの IR 命令に `Span` を補完して、より多くのランタイムエラー種别をカバーできます。

2. **複数ファイル/モジュール**：`Span` 自体は引き続き行と列の情報のみを包含しますが、DebugMap は `ip -> (file_id + span)` にアップグレードされ、`SourceMap`（`file_id -> path/content`）が導入されてクロスファイルレンダリングが可能です。今後はコンパイラパイプラインに真实の複数ファイル `file_id` 割り当て戦略を導入できます。

3. **バイトコードファイル DebugSection**：`.42` フォーマットにオプションの DebugSection（sources + ip マッピング）を追加已經拡張され、读写が実装されました。オフライン実行/デバッグ時に位置情報保持に使用できます。

4. **CLI スイッチ**：`run` / `build` に `--debug-info` が追加され、DebugMap 生成と `.42` DebugSection 嵌入の制御に使用されます。