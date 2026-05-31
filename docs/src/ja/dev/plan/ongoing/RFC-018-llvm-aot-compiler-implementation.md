# RFC-018 LLVM AOT コンパイラと L3 透明并发（DAG 遅延スケジューリング）実装計画

> **タスク**：LLVM AOT バックエンド + ランタイム DAG スケジューラを実装し、`@auto/@eager/@block` の3つのスケジューリング戦略（L3/L2/L1）を展開する  
> **RFC に基づく**：RFC-018（草案）  
> **依存 RFC**：RFC-001（並作モデルとエラー処理）、RFC-008（三層ランタイム）、RFC-009（所有権/Arc）  
> **日付**：2026-03-10  
> **状態**：進行中  
> **目標マイルストーン**：  
> - M1：LLVM AOT（コンパイル可能で実行可能、シリアル）  
> - M2：DAG メタデータ + シングルスレッドスケジューリング（Standard Runtime、num_workers=1）  
> - M3：マルチスレッドパラレルスケジューリング + 粒度制御（Full Runtime、num_workers>1）  
> - M4：遅延スケジューリング（Lazy Task Creation）+ **リソース型（Resource）副作用抽象** + **エラー伝播/エラーズグラフ** + アノテーション贯通

---

## 摘要（実装闭环）

- `yaoxiang` に LLVM バックエンド（feature gate）を新增し、`BytecodeModule` を機械語（COFF/ELF/Mach-O）にコンパイルしてランタイム時にロード・実行可能にする。
- **安定 ABI** を導入：AOT 生成コードとランタイムは `extern "C"` の `RtValue/RtContext` を通じて相互連携し、Rust enum ABI の不安定問題を回避する。
- RFC-018 のコアを展開：**関数ブロック内の DAG** + **遅延スケジューリング（Lazy Scheduling）**。並行/シリアルは **DAG エッジ（Data/Control/Spawn）** と **リソース型（Resource）ルール** が共に決定する；エラーは RFC-001 に従い依存エッジに沿って伝播し、エラーズグラフを形成できる。

---

## 公共インターフェース/振る舞い変更（外部公開）

1. **Cargo features**
   - 新增 `llvm-aot` feature：LLVM/inkwell 依存と AOT バックエンドを有効化；デフォルトオフ（LLVM なし環境でもビルド可能を保証）。

2. **CLI**
   - `yaoxiang run` に `--backend {interpreter,llvm}` を追加（デフォルト interpreter）。
   - 任意：`--runtime {embedded,standard,full}` と `--workers <N>` を追加してランタイムレベルと并发度を制御（RFC-008）：
     - `--runtime embedded`：即時実行（DAG なし、スケジューラ機能なし、組込み/最小限シナリオ向け）
     - `--runtime standard`：DAG + Scheduler（num_workers=1 は非同期的；>1 はパラレル）
     - `--runtime full`：standard + WorkStealer（高度な機能、オプション）

3. **ランタイム ABI（内部だがモジュール間）**
   - 新增 `RtValue`（`#[repr(C)]`）と `RtContext`（ポインタ/基本类型のみを含む）を AOT と runtime の連携境界として定義。

---

## 重要設計制約（RFC-001 / RFC-008 / RFC-018 との整合）

### A. 並行セマンティクス（L1/L2/L3 はメンタルモデルのみ）

- **L3（デフォルト / @auto）**：透明并发；DAG を構築；呼び出しに遭遇すると「遅延評価可能」な値を返し、**値が必要になった時点で評価をトリガー**する。
- **L1（@block）**：標準ライブラリが提供する（RFC-008）、セマンティクスは「強制急切評価」、DAG 遅延キューに入らない；主にデバッグと重要な順序セグメント用。
- **L2（spawn）**：**@block スコープ内でのみ使用可能**（RFC-001/008）、同期コード内に並行を挿入するために使用；Full Runtime 機能に属する。

### B. ランタイム三層（RFC-008）

- **Embedded Runtime**：即時実行；DAG 完全構築回避を選択可能（メモリ/起動時間節約）；制約のある環境向け。
- **Standard Runtime**：DAG + Scheduler がコア（遅延評価が非同期的/パラレルのサポートを自然に提供）。
- **Full Runtime**：standard に加えて WorkStealer と、標準ライブラリ層の `@block` / `spawn` などの機能を提供。

### C. DAG 構築範囲とメモリ（RFC-001/018）

- DAG は**单个関数体/ブロック内**のみで構築；呼び出し先関数体に再帰展開しない（エラーズグラフと DAG ノード爆発を回避）。
- DAG メタデータは**ノード/エッジの安定 ID** と **Span** を携带する必要がある（エラー伝播とエラーズグラフのローカライズに使用）。

### D. 副作用抽象（RFC-001：リソース型）

- 追加の「明示的副作用アノテーションシステム」を導入しない；副作用は統一的に**リソース操作**として扱う：
  - 引数型に `Resource`（またはその派生リソース型）を含む関数呼び出しは、リソース操作とみなす；
  - 同一リソースへの操作は自動的に **ControlEdge** を形成（シリアル）；異なるリソースは並行可能；
  - 静的判断で同一リソースかどうか判断つかない場合、デフォルトで保守的シリアル（将来的に明示的 unsafe 並行ヒントを拡張として導入可能）。

### E. エラー伝播（RFC-001）

- エラーは DAG の依存エッジに沿って上方伝播し（実際の並行実行順序とは無関係）、エラーズグラフ用の伝播経路を記録する。

---

## 段階 0：前置条件と制約ロック（1-2 日）

### 0.1 LLVM/inkwell バージョンとビルド方法のロック

**目標**
- LLVM メジャーバージョン = **17** を選択（チーム環境の統一；Windows/Linux/macOS すべて対応配布物を取得可能）。
- `Cargo.toml` に `inkwell` を追加（`llvm17-0` に対応する feature を有効化）し、`llvm-aot` feature 下に配置。

**受入基準**
- [ ] `cargo build`（feature なし）で成功（LLVM なし環境でもビルド可能）。
- [ ] `cargo build -F llvm-aot` は LLVM17 環境設定時に成功。

**テスト項目**
- [ ] CI/ローカル：2つのビルドマトリクス（`default` と `-F llvm-aot`）で少なくとも1プラットフォームが通る。
- [ ] 最小 smoke：`cargo test -F llvm-aot` が空テストモジュールを起動・実行できる（リンク確認のみ）。

---

### 0.2 LLVM 環境探测とエラーメッセージ

**目標**
- ビルド時/ランタイム時の探测説明を追加：`llvm-config`/LLVM ディレクトリ缺失時に操作可能なエラーメッセージを表示（インストール方法/接頭辞環境変数の設定方法）。

**受入基準**
- [ ] LLVM 缺失時、エラーメッセージに以下を含める：期望バージョン（17）、利用可能な環境変数（`LLVM_SYS_170_PREFIX` または `LLVM_CONFIG_PATH` 等）とサンプルパス。

**テスト項目**
- [ ] LLVM なしのマシンで `cargo build -F llvm-aot` を実行し、出力が完整で panic なし（コンパイルエラーで十分）。

---

### 0.3 並作モデル実装制約ロック（RFC-001/008 との整合）

**目標**
- 以下の実装制約を明確にして固定化する（コードコメント/開発ドキュメントとテストケースに記述）：
  - `spawn` は `@block` スコープ内でのみ許可（解析/型チェック/IR 段階でいずれも防御）。
  - `@block` セマンティクスは「急切評価」、標準ライブラリ機能として提供（最初はコンパイラ組み込みで MVP 実装可、未来の標準ライブラリへの下ろしを考慮したインターフェースを保持）。
  - DAG は関数ブロック内でのみ構築；安定した `node_id` と `span` を携带必须（エラー伝播/エラーズグラフサポート）。
  - リソース型（Resource）が ControlEdge 生成を驱动し、追加のユーザー可见 effect アノテーション体系を導入しない。
  - **並行安全制約（RFC-001/009）**：ノードのキャプチャ/戻り値が `Send + Sync`（または言語側の同等功能束）を滿たす場合にのみ跨スレッド並行を許可；そうでなければシリアルに降格（または単一 worker 上で固定実行）。

**受入基準**
- [ ] コンパイラが不合法の `spawn` シナリオで明確なエラー（含 Span）を出力。
- [ ] `@block/@eager/@auto` のセマンティクス差異が最小サンプルで観測・テスト可能。
- [ ] このドキュメントと RFC-001/008/018 の主要決議が一致し、自己矛盾条目がない。

**テスト項目**
- [ ] `spawn` 位置制限テスト：@block 以外で spawn が現れるとエラーを出す。
- [ ] DAG scope テスト：DAG が関数体を跨いで展開されないことを確認（ノード数と呼び出し階層が無関係）。
- [ ] Send/Sync 制約テスト：
  - `spawn` が非 `Send` 値をキャプチャするとエラーを出す（含 Span）。
  - `@auto` 下での非 `Send + Sync` 値を含むノードは跨スレッドスケジューリングされない（`std.concurrent.thread_id` で検証可）。

---

## 段階 1：LLVM バックエンドスケルトンと選択スイッチ（1-2 日）

### 1.1 新規バックエンドモジュールと RFC-018 ディレクトリ構造との整合

**目標**
- `src/backends/llvm/` を新規追加：`mod.rs / context.rs / types.rs / values.rs / codegen.rs / dag.rs / scheduler.rs / tests.rs` を含む（ 이후 マージ/分割を許可）。
- `src/backends/mod.rs` に `#[cfg(feature = "llvm-aot")] pub mod llvm;` を追加してモジュールを外部公開。

**受入基準**
- [ ] `cargo test`（デフォルト feature）が成功。
- [ ] `cargo test -F llvm-aot` が成功（LLVM バックエンドがまだ完全実装でなくても）。

**テスト項目**
- [ ] ユニット：`src/backends/llvm/tests.rs` に少なくとも1つのコンパイル時テストが実行可能（モジュールの参照のみ確認）。

---

### 1.2 バックエンド選択：CLI/ライブラリ側注入ポイント

**目標**
- CLI `Run` サブコマンドに `--backend` パラメータを追加（ValueEnum）：`interpreter`（デフォルト）/ `llvm`（feature 要）。
- `yaoxiang::run_*` パスにバックエンド選択分支を追加し、`fn make_executor(kind, config) -> Box<dyn Executor>`（または枚举ディスパッチ、trait object 回避可）として抽象化。

**受入基準**
- [ ] `yaoxiang run file.yx` は引き続きインタープリタを使用、振る舞いが不变。
- [ ] `yaoxiang run --backend llvm file.yx`：feature 未有効化時は明確なエラー；有効化時は LLVM 実行パスに 진입（たとえ一時的に "not implemented" を返しても制御されたエラーであること）。

**テスト項目**
- [ ] CLI パラメータ解析テスト（`tests/integration` に追加）。
- [ ] 負向テスト：feature なし時に `--backend llvm` を渡すと読めるエラーメッセージを返す。

---

## 段階 2：安定 ABI（RtValue/RtContext）と Runtime API（3-5 日）

> この段階は「LLVM 生成コードが実行可能」の鍵：まずクロスパウリの値表現を安定させる必要がある。

### 2.1 `RtValue`（安定 C ABI）の定義

**目標**
- `#[repr(C)] struct RtValue { tag: u8, _pad: [u8; 7], payload: u64 }`（または 16 バイト構造体、简单なアライメントを保持）を定義。
- `#[repr(u8)] enum RtTag { Unit, Bool, Int, Float, Handle, Async, Error }`（最小セット； 이후 拡張）を定義。
- 約束：
  - Int：`payload` = `i64` ビット
  - Float：`payload` = `f64` ビット
  - Bool：0/1
  - Handle：`payload` = `usize`（u64 に拡張）
  - Async：`payload` = `TaskId`（u64）
  - Error：`payload` = エラーコードまたはポインタ（MVP ではエラーコード優先）

**受入基準**
- [ ] `RtValue` は Rust 内部で安全に構築/読み取り可能（UB なし）、`Debug` 出力と基本アサーションutility関数を具备。
- [ ] LLVM IR との整合：inkwell で同レイアウトの struct type（フィールド順序/サイズが一致）を作成可能。

**テスト項目**
- [ ] `RtValue` roundtrip：int/float/bool/unit の encode/decode ユニットテスト。
- [ ] ABI サイズテスト：`size_of::<RtValue>()` と `align_of::<RtValue>()` が固定（書き込みアサーション、未来の誤変更を防止）。

---

### 2.2 `RtContext`（ランタイムコンテキスト）の定義

**目標**
- `#[repr(C)] struct RtContext` を定義、ポインタ/整数のみを含む：
  - `heap: *mut Heap`
  - `ffi: *const FfiRegistry`
  - `scheduler: *mut DynScheduler`（または具体実装へのポインタ）
  - `max_parallelism: usize`
  - `error_graph: *mut ErrorGraph`（任意：RFC-001 のエラー伝播記録用；MVP では null 可）
  - 予約フィールド（バージョン番号/flags）、だが最小化（KISS）を維持。

**受入基準**
- [ ] `RtContext` はインタープリタ/LLVM executor から構築されて生成コードに渡せる。
- [ ] Rust 非安定レイアウトフィールドを直接含まない（`Heap`/`FfiRegistry` 値を直接埋め込み禁止）。

**テスト項目**
- [ ] `RtContext` の構築/破棄のメモリ安全テスト（実際の LLVM 不要）。

---

### 2.3 Runtime C API：生成コードが呼び出す最小関数セット

**目標**
- `#[no_mangle] extern "C"` 関数を提供（命名統一接頭辞 `yx_rt_*`）、MVP 最小 포함：
  - `yx_rt_force(ctx: *mut RtContext, v: RtValue) -> RtValue`
  - `yx_rt_lazy_call(ctx, callee: *const u8 /*関数ポインタ*/, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_native_call(ctx, name_ptr: *const u8, name_len: usize, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_panic(code: u32)` または `yx_rt_trap(msg_ptr, len)`（デバッグ用）
- 制約：AOT 生成コードは**上記の API を通じて만 相互連携**し、Rust 構造体を直接操作しない。

**受入基準**
- [ ] ランタイム API は LLVM なしでもコンパイル可能（`llvm-aot` feature 制御：API は常住または feature 下でのみ提供、だがテスト 가능必须）。
- [ ] `yx_rt_native_call` は `FfiRegistry` の handler を呼び出せる（MVP は Int/Float/Bool/Unit 引数と戻り値のみサポート；未サポート時は Error RtValue を返す）、失敗時は `node_id/span_id` をエラーズグラフに記録（有効な場合）。

**テスト項目**
- [ ] 純粋 Rust ユニットテスト：`yx_rt_native_call` を直接呼び出し、`std.io.println`（または自己登録関数）パスが使用可能であることを検証。
- [ ] エラーパステスト：存在しない native 名称を渡すと `Error` RtValue を返し、`ExecutorError::FunctionNotFound` に変換可能。

---

## 段階 3：LLVM Codegen インフラストラクチャ（2-3 日）

### 3.1 LLVM コンテキスト/モジュール/TargetMachine 初期化

**目標**
- `context.rs`：inkwell `Context/Module/Builder` のライフサイクルをカプセル化。
- Target 初期化：`PlatformDetector`（`LLVM_TARGET` サポート）に基づき、ホスト三元組で target triple + data layout を設定。
- 出力サポート：
  - LLVM IR（`.ll`）デバッグ用
  - Object（`.o/.obj`）AOT 用

**受入基準**
- [ ] 任意の空 `BytecodeModule` に対して、`main` を含む LLVM Module を生成可能（関数体が Unit を返すだけでも）。
- [ ] IR は検証可能（LLVM verify を呼び出す；失敗時は読めるエラーを返す）。

**テスト項目**
- [ ] ユニット：最小 module を生成して verify が成功。
- [ ] スナップショットテスト（任意）：`.ll` の重要フラグメントに対して文字列包含アサーション（脆い全量スナップショット回避）。

---

### 3.2 `TypeMap`：YaoXiang Type → LLVM Type（MVP）

**目標**
- `types.rs`：`fn llvm_type(yao_type: &Type) -> BasicTypeEnum` を実装、まず以下をカバー：
  - `Int` → i64
  - `Float` → f64
  - `Bool` → i1
  - `()`/Void → void（または `RtValue(Unit)` で統一返戻）
- 戦略選択（ABI 面積削減のため）：**すべての関数が統一的に `RtValue` を返す**（型별返回值ではなく）、codegen とスケジューラ/FFI が统一的に処理；型情報は静的检查と `RtValue` 構築/分解ロジック生成に使用。

**受入基準**
- [ ] `TypeMap` が上記のマッピングを安定させ、LLVM IR での関数シグネチャを统一：`fn(*mut RtContext, *const RtValue, usize) -> RtValue`。

**テスト項目**
- [ ] `TypeMap` 単体テスト：`Type::Int/Float/Bool/Void` が指定された LLVM 型を生成することを確認。
- [ ] 生成された関数シグネチャが LLVM module 内で検索可能で引数/戻り値型が一致するアサーション。

---

## 段階 4：命令翻訳 MVP（5-8 日）

### 4.1 レジスタから LLVM 値へのマッピング（SSA 化の最小実装）

**目標**
- `values.rs`：仮想レジスタ `Reg(u16)` → LLVM `Value` のマッピングテーブルを実装（基本ブロックスコープ별로管理）。
- 約束：すべてのレジスタ値は `RtValue` で表現（型爆発/ABI 不整合回避）、演算/比較前に helper で強制/展開。

**受入基準**
- [ ] 生成コードが制御フロー分岐後にレジスタ値を正しくマージ可能（phi 使用または `RtValue` 層での統一型処理）。

**テスト項目**
- [ ] ユニット：if/else を含む `BytecodeFunction` から IR を生成し verify が成功。
- [ ] 回帰：同一レジスタへの複数代入が use-before-def を招かない（デバッグモードでは trap/エラーを挿入）。

---

### 4.2 核心命令サブセットの翻訳（「走り拔け」カバレッジ）

**目標**
- `codegen.rs`：少なくとも以下の `BytecodeInstr` を実装：
  - `LoadConst`（Int/Float/Bool/String は限定的に：String は Error に降格または一時未サポート可）
  - `Mov`
  - `BinaryOp`（Add/Sub/Mul/Div：Int と Float 各自的パス）
  - `Compare`（Eq/Lt/Gt 等）
  - `Jmp/JmpIf/JmpIfNot/Return/ReturnValue`
  - `CallNative`（`yx_rt_native_call` 経由）
  - `CallStatic`（2つの戦略：`@block` は直接 call；`@auto` は `yx_rt_lazy_call` を経由して Async を返す）
- 強制ルール：算術/比較/分岐に参加するオペランドは 모두 `yx_rt_force` が必要（透明并发の「値が必要時にトリガー」）。

**受入基準**
- [ ] AOT バックエンドで简单なプログラムを実行可能：
  - 純粋算術
  - if/else
  - `std.io.println` 呼び出しで出力
- [ ] 未サポートの命令は読みやすいエラー产生（panic ではない）。

**テスト項目**
- [ ] 統合：`tests/integration/llvm_aot_smoke.rs` を追加（feature gate）、5つのプログラムフラグメントを実行して結果/出力をアサーション（出力は stdout リダイレクトで実現可）。
- [ ] 負向：`MakeClosure/CallVirt/...` 遭遇時は明確な「未実装」エラーを返す。

---

## 段階 5：機械語成果物と実行（AOT 闭环）（3-6 日）

### 5.1 成果物フォーマット：Object + メタデータ（2ファイル、まずは简单に）

**目標**
- `CompiledArtifact`（Rust 側構造体）は少なくとも以下を含む：
  - `object_bytes: Vec<u8>`（COFF/ELF/Mach-O）
  - `dag_meta: DAGMetadata`（まずは空可）
  - `entries: Vec<EntryPoint>`（少なくとも main）
  - `type_info: TypeInfo`（MVP はまずは空）
- 出力戦略：
  - `yaoxiang build-aot input.yx -o out/` で `program.obj` + `program.dag.ron`（または `.json`）を生成
  - `yaoxiang run --backend llvm` はデフォルトで「メモリ内コンパイル+直接実行」（ディスク書き込みなし）、開発効率向上

**受入基準**
- [ ] build-aot が2つのファイルを生成し、メタデータが逆シリアル化可能。
- [ ] run/llvm パスはディスク書き込みファイルに依存せずとも実行可能。

**テスト項目**
- [ ] ファイル生成テスト：`.obj` が非空、`.dag.ron` がパース可能かつバージョン番号が一致することを検証。
- [ ] 互換性テスト：異なる build_mode（Debug/Release）が異なる最適化レベルを出力（少なくとも区別可能）。

---

### 5.2 実行方式：「メモリ内実行」→「ディスク書き込み→ロード」（2ステップで受入）

**目標**
- Step A（ 먼저 納入）：LLVM ExecutionEngine（または ORC JIT）を使用して生成済み module を実行（セマンティクス闭环验证、開発効率最高）。
- Step B（AOT 準拠）：TargetMachine で object bytes を生成し、「動的ライブラリリンク/ロード」パスで実行：
  - object を `.dll/.so/.dylib` にリンク（システムリンカまたは lld を呼び出す；`llvm-aot` feature の追加要件として）
  - `libloading` でシンボルをロードして入口関数を呼び出す

**受入基準**
- [ ] Step A：`--backend llvm` が同一プロセス内で実行可能（外部リンカに依存しない）。
- [ ] Step B：`build-aot` が生成した成果物を `run-aot`（新規サブコマンドまたは内部パス）でロード・実行可能。

**テスト項目**
- [ ] Step A：ユニット/統合テストがデフォルトで実行（開発が速い）。
- [ ] Step B：外部リンカを「必要とする」オプション統合テストとしてマーク（CI に環境があれば有効；ローカルは手動可）。

---

## 段階 6：DAG メタデータ生成（4-7 日）

### 6.1 `DAGMetadata`（バージョン管理）の定義

**目標**
- `dag.rs` で以下を定義：
  - `dag_version: u32`
  - `nodes: Vec<DAGNode>`（`node_id` と `span_id` を携带、エラー伝播用）
  - `edges: Vec<DAGEdge>`（エッジタイプ付：Data/Control/Spawn）
- `DAGEdge` は少なくとも以下を含む：
  - `from: u32`
  - `to: u32`
  - `kind: EdgeKind { Data, Control, Spawn }`
- 競合/スケジューリングルール（RFC-001）：
  - DataEdge + DataEdge：他に依存がなければ並行可能
  - ControlEdge を含む任意の组合：シリアル化必须（順序保持）
- `DAGNode` は少なくとも以下を含む：
  - `node_id: u32`（関数内唯一）
  - `ip: u32`（call 命令位置）
  - `strategy: ScheduleStrategy { Serial, Eager, Lazy }`（アノテーションまたはデフォルト戦略から）
  - `span_id: u32`（ローカライズとエラーズグラフ）
  - `thread_safety: ThreadSafety { SendSync, LocalOnly }`（型システムから推导；`LocalOnly` ノードは跨スレッドスケジューリング禁止）
- 約束：ノードは「スケジューリング可能な呼び出し点」のみを記述、パラメータはランタイム時に `yx_rt_lazy_call` でキャプチャ（静的式評価の複雑さを回避）。

**受入基準**
- [ ] `DAGMetadata` がシリアル化/逆シリアル化可能（既存の `ron` または `serde_json` を使用）。
- [ ] `dag_version` が一致しない場合はロードエラー。

**テスト項目**
- [ ] シリアル化 roundtrip 単体テスト。
- [ ] バージョン不一致単体テスト（手動で旧バージョンを構築）。
- [ ] `thread_safety` 推导テスト：少なくとも1つの `LocalOnly` シナリオをカバーし、`num_workers>1` 下では跨スレッド実行されないことを検証。

---

### 6.2 リソース型と ControlEdge 生成（副作用抽象の最小使用可能セット）

**目標**
> **更新**：RFC-001 に従い、副作用は追加の effect システムで表現せず、**リソース型（Resource）** で抽象化してリソース操作として ControlEdge を生成する。

- リソース操作の識別（MVP）：
  - 呼び出しのいずれかのパラメータ型が `Resource` またはその派生リソース型（例：`Console/FilePath/HttpUrl/DBUrl`）である場合、その呼び出し点はリソース操作；
  - 標準ライブラリのリソース操作関数は識別可能な型制約を具备する必要がある（推奨做法：std エクスポートシグネチャにリソース型を明示的に携带；または FFI registry のエクスポートメタデータに「リソース操作」マークを付けてリソース入参ビット联系起来）。
- ControlEdge 生成（MVP）：
  - **同一リソース値/ハンドルの同一 SSA 値/同一定数定居キー）** に対する複数リソース操作を、プログラム順序に従って ControlEdge を追加（自動シリアル）。
  - 同一リソースかどうか判断つかない場合（エイリアス/複雉ソース）はデフォルトで保守的シリアル（未来は明示的 unsafe 並行ヒントを拡張として導入可）。
  - **リソース識別子はデータフローに沿って伝播（RFC-001）**：リソース競合検出は「値の等価/同一ソース」を基準とし、「リソース型の同一」を基準としない（2つの異なる `FilePath` 値は並行可能；同一 `FilePath` 値はシリアル必须）。

**受入基準**
- [ ] 示例：`log → save → log` は Console/FilePath リソースにより ControlEdge を形成し、稳定的にシリアル；異なるリソース操作は並行可能。
- [ ] リソース操作の識別が安定（同一入力 module で複数回の結果が一致）。

**テスト項目**
- [ ] ユニット：リソース型パラメータ識別テスト（Resource パラメータ存在時は ControlEdge 生成必须）。
- [ ] ユニット：同一リソース値（同一変数/同一定数）上の2回リソース操作は ControlEdge を生成必须；異なるリソース値（異なる変数/異なる定数）は生成任意。
- [ ] 統合：複数の `std.io.println/std.io.write_file` を含む示例を実行し、出力/書き込み順序がインタープリタと一致することをアサーション。

---

### 6.3 L1 自動フォールバック（小関数を @block に降格、スケジューリングオーバーヘッド回避）

> **来源**：RFC-001 5.2（L1 自動フォールバック）。  
> **目的**：セマンティクスを変えず、小関数の DAG/スケジューラオーバーヘッドを削減（特にインタープリタバックエンドと AOT バックエンドの统一動作）。

**目標**
- コンパイル時に関数に対して軽量閾値判定を実行し、いずれかの条件滿たす場合は해당 함수（または該当当関数内の特定ブロック）のデフォルト戦略を `Serial` に降格：
  - 命令数 `< 50`
  - DAG ノード数 `< 10`
- CLI/設定でスイッチを外部公開（MVP：内部設定のみも可）：
  - `--l1-threshold=<N>` 閾値調整
  - `--no-l1-fallback` 自動フォールバック無効化

**受入基準**
- [ ] 小関数が `@auto` 下では実際に DAG/スケジューラキューに入らない（統計フィールドまたは trace で検証可）、結果がフォールバックなしと一致。
- [ ] 大関数には影響なし；强制アノテーション `@eager/@block` は自動フォールバックより優先。

**テスト項目**
- [ ] ユニット：境界値（49/50 命令、9/10 ノード）を構築してフォールバック発動を検証。
- [ ] 回帰：同一ソースコードでフォールバック on/off を切り替えて、出力と戻り値が一貫。

---

## 段階 7：ランタイム DAG スケジューラ（遅延スケジューリングのコア）（6-10 日）

### 7.1 タスクモデルの実装（`RtValue::Async` との連携）

**目標**
- `scheduler.rs`（または `src/backends/runtime/` に移行して「インタープリタ/LLVM 共有」を実現）に以下を実装：
  - `TaskId` 割り当て
  - `TaskState { Pending, Running, Completed(RtValue), Failed(RtValue) }`
  - `spawn(task_fn_ptr, args, deps, node_id, span_id)`：タスクを作成するが起動を遅延可能（エラー伝播/エラーズグラフ用）
  - `force(task_id)`：依存トポロジに従って実行をトリガーして結果を待機

**受入基準**
- [ ] `yx_rt_lazy_call` が `Async(TaskId)` を返し、タスクが記録される（即時実行しない）。
- [ ] `yx_rt_force` がタスク実行をトリガーして結果を返す（依存チェーンを含む）。

**テスト項目**
- [ ] 純粋 Rust：モックされた「compiled fn」ポインタ（実際は Rust `extern "C"` 関数）で3ノード DAG を構築し、依存順序と結果の正しさを検証。
- [ ] エラー伝播：下流の force が Error を得、死鎖しない。

---

### 7.2 スケジューリング戦略の実装（Serial/Eager/Lazy）

**目標**
- `Serial`（`@block` に対応）：Async を作成しない；call を即時実行；スケジューラインターフェースはバイパス可。
- `Eager`：タスクを作成するが즉시 `force`（順序保証）、デバッグ/セマンティクス整合用。
- `Lazy`（デフォルト `@auto`）：値が必要になったときに만 `force`；スケジューラはバックグラウンドウィンドウ内で「準備完了」タスクを先行起動可能（并发数制限に従う）。
- ボトムアップ（RFC-001/008）：ランタイムの振る舞いは「結果が必要な场所から逆向に評価をトリガーする」特性を反映해야 한다；**消費されずリソース操作を含まない（ControlEdge なし）分岐/孤立 DAG 枝は実行されない**べきであり、オーバーヘッドを削減；リソース操作は ControlEdge に従って順序保証と完了必须的（インタープリタと一致）。
- バックグラウンド DAG（RFC-018）：同一スコープ内に複数の長時間実行/無限ループタスクがある場合、スケジューラは**協調的スライシング**を提供해야 한다（例：budget ベースまたは明示的 `yield_now`）、メイン DAG の饥饿と「ループでのスタック」を回避。

**受入基準**
- [ ] 同一プログラムが Serial/Eager/Lazy の3戦略で結果が一致。
- [ ] Lazy 下では某 call の結果が force/使用されていない場合、タスクは実行されない（Lazy Task Creation）。

**テスト項目**
- [ ] 比較テスト：3戦略の出力の一致性。
- [ ] Lazy スキップテスト：「計算するが使用しない」分岐/変数を記述し、対応するタスク実行カウントが 0 であることをアサーション（スケジューラ統計フィールド使用）。
- [ ] バックグラウンドスライシングテスト：2つの長時間実行タスク + 1つのメインタスクを構築し、 時間窓内で3者が 모두進捗があることをアサーション（カウンターまたは `thread_id` + ログ統計使用）。

---

### 7.3 並行制御と粒度制御

**目標**
- 並行上限：`max_parallelism = num_workers * 2`（RFC-018 推奨）。
- リソース制約：同一リソースへの操作は ControlEdge に従ってシリアル実行必须的（RFC-001 リソース型ルール）、スケジューラは ControlEdge 順序を乱さない。
- スレッド安全制約（RFC-001/009）：スケジューラは `DAGNode.thread_safety` を尊重해야 한다：
  - `SendSync`：worker 間跨いで実行可能（并发上限と依存制約に従う）
  - `LocalOnly`：跨スレッドスケジューリング/ワークスティーリングによる横取り禁止；必要时シリアルに降格（またはそれを生成した worker 上に固定実行）
- アダプティブ粒度（MVP）：実行待ちタスク数が并发上限に大きく超过する場合、「準備完了かつ**ControlEdge 制約なし**の複数のタスク」をマージして批量提交（同一 worker がシーケンシャルに一批を実行、scheduling overhead 削減）。

**受入基準**
- [ ] 大量独立・無リソース制約タスク（1e4）がメモリ爆発を招かない（タスク構造は O(并发数) または制御可能な上界）。
- [ ] `LocalOnly` ノードが `num_workers>1` 下では跨スレッド実行されない（`std.concurrent.thread_id` で検証可）。
- [ ] リソース操作（例：`std.io.*`）の出力/副作用順序が严格にインタープリタ順序を維持。

**テスト項目**
- [ ] 壓力テストユニット：10000個のモックタスクを構築し、peak メモリ/タスク数量が制御されていることを検証（統計アサーション、精密メモリ測定不要も可）。
- [ ] LocalOnly 統合テスト：`LocalOnly` ノードを含む示例を構築し、`num_workers>1` 下では実行スレッド ID が変化しないことをアサーション。
- [ ] リソース順序統合テスト：複数のリソース操作（println/write_file）はソースコード順序で出力/ディスク書き込み必须的。

---

### 7.4 エラー伝播とエラーズグラフ記録（RFC-001 最小闭环）

**目標**
- 最小 `ErrorGraph` データ構造を定義（まずはデバッグ/trace 用のみ）：
  - ノード：`node_id + span_id + message/error_code`
  - エッジ：`from_node_id -> to_node_id`（「エラーが依存ノードからconsumerノードへ伝播した」ことを表す）
- 記録戦略（RFC-001 決議）：
  - エラーは依存エッジに沿って上流へ伝播し、**実際の実行順序に依存しない**；
  - DAG は関数内でのみ構築されるため、エラーズグラフも関数级别に制限され、メモリ爆発を回避。
- ABI との整合：
  - `yx_rt_lazy_call/yx_rt_native_call` は `node_id/span_id` を携带해야 한다（段階 2.3 で既にロック済み）
  - タスク失敗と `force` がエラーを返すとき、`ErrorGraph` に書き込み（`ctx.error_graph != null` の場合）

**受入基準**
- [ ] 依存チェーンの末端ノードが失敗的时候、顶层consumerがエラーを受け取り（失敗ノードの span までローカライズ可能）。
- [ ] 並行実行下でもエラー伝播パスが安定して再現可能（スケジューリング順序に依存しない）。

**テスト項目**
- [ ] ユニット：3ノード依存チェーンを構築し、中間ノード失敗をシミュレートし、ErrorGraph エッジが `leaf->mid->root` であることをアサーション。
- [ ] 並行回帰：num_workers>1 で複数回実行し、ErrorGraph 構造が一貫。

---

## 段階 8：構文アノテーション贯通（@block/@eager/@auto）（5-8 日）

### 8.1 フロントエンドでのアノテーションサポートとバイト码/メタデータへの伝達

**目標**
- 解析層：関数/ブロックアノテーション `@block`、`@eager` を識別；デフォルト `@auto`。
- 解析/型チェック：`spawn` は `@block` スコープ内にのみ出現可能ことを強制（RFC-001/008）。
- IR/Bytecode：`BytecodeFunction` または追加 side-table にデフォルト戦略を携带；call-site で lazy/eager/direct のどれを通るか決定可能。

**受入基準**
- [ ] アノテーションなし：デフォルト Lazy（@auto）。
- [ ] `@block`：해당 スコープ内で Async を作成せず、インタープリタシリアルと動作が一致。
- [ ] `@eager`：タスク作成後즉시 force（結果整合かつデバッグ容易）。

**テスト項目**
- [ ] フロントエンド：アノテーションを含む解析/IR 生成テスト（AST/IR アサーション）。
- [ ] バックエンド：同一ソースコードにアノテーション别々に施加し、戦略に合った振る舞いを確認。

---

### 8.2 標準ライブラリ：`@block` と `spawn` のランタイムインターフェース（Full Runtime）

> **来源**：RFC-008（@block は標準ライブラリが提供する）、RFC-001（spawn の待機セマンティクスは標準ライブラリが制御）。

**目標**
- 標準ライブラリランタイムモジュールを追加（推奨パス：`std.runtime` または `std.full`）：
  - `block`：强制急切評価（スコープ戦略を `Serial` に設定/DAG キュー不进录と同等）
  - `spawn`/`join_all`（または暗黙的 join）：`@block` スコープ内で並行タスクを作成して完了を待機
- コンパイラはまずは組み込み実装で MVP を実現可能だが、标准ライブラリに下ろせるインターフェースを抽象化해야 한다（未来のリファクタリングコスト回避）。

**受入基準**
- [ ] `@block` 関数内の `spawn { ... }` ブロックが並行実行され、スコープ終了前に完了（「サイレントバックグラウンドリークタスク」なし）。
- [ ] `@block` の振る舞いと L3 デフォルト並行動作が明確に区別可能（例如：DAG キューに入るかどうか、Async 値を生成するかどうか）。

**テスト項目**
- [ ] 統合：2つの `spawn { std.concurrent.sleep(50) }` 示例が複数 worker 下で単回 sleep 時間に近い時間で完了（粗粒度并行検証）。
- [ ] 負向：@block 外で spawn を使用するとエラー（0.3/8.1 と一貫）。

## 段階 9：エンドツーエンドとパフォーマンスベンチマーク（継続推進）

### 9.1 インタープリタとの一貫性テスト（セマンティクス整合）

**目標**
- 「命令サブセットをカバーできる」テストスイートを選定：算術、分岐、関数呼び出し、native IO。
- 同一ソースコードをインタープリタと llvm バックエンドでそれぞれ実行し、以下を比較：
  - 戻り値（もしあれば）
  - stdout 出力（注入/リダイレクトが必要）
  - エラータイプ（できるだけ `ExecutorError` を整合）

**受入基準**
- [ ] テストスイート全体で LLVM バックエンドとインタープリタの結果が一致。

**テスト項目**
- [ ] `tests/integration/llvm_vs_interpreter.rs`（feature gate）至少 10 のテストケース。
- [ ] 回帰：新テストケース追加時は両バックエンドで実行必须。

---

### 9.2 ベンチマーク：インタープリタ vs AOT（粗粒度）

**目標**
- `benches/` にベンチマークを追加：純粋計算（IO なし）、大量 call タスク（并行収益）、混合 IO（順序制約）。

**受入基準**
- [ ] AOT が純粋計算テストケースでインタープリタより明らかに高速（具体的な倍数は約束なし、だが明らかに低速ではない）。
- [ ] Lazy スケジューリングのオーバーヘッドが観測・特定可能（スケジューラ stats 出力を使用）。

**テスト項目**
- [ ] criterion bench（手動/CI 任意）がレポートを生成し、ベースラインを記録。

---

## 前提とデフォルト（未被覆うビジネス要件の場合の選択）

- デフォルト LLVM メジャーバージョン = **17**；チームツールチェーンが異なる場合は `inkwell` feature とドキュメントを一括修正即可。
- AOT 実行パスは「2ステップ方式」：まずはメモリ内実行（開発検証用）、次にディスク書き込み→リンク/ロード（本当の AOT）。
- 初期 `llvm-aot` は MVP 命令サブセットのみカバレッジを約束；クロージャ/動的ディスパッチ/例外などの高度な機能は 이후 必要性に応じて拡張（遭遇時は明確な「未実装」エラーを返す）。
- DAG 依存エッジは**ランタイム時に args の Async TaskId から動的に推导可能**；コンパイル時 edges フィールドはまずはオプションな最適化とデバッグ校验として、M2 納入をブロックしない。
  - **補足（RFC-001）**：ControlEdge の主要な出所はリソース型ルール；リソース情報がない場合は正しさを保証するためにデフォルトで保守的シリアル。