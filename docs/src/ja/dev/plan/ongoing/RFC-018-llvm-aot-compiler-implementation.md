# RFC-018 LLVM AOT コンパイラと L3 透明并发（DAG 遅延スケジューリング）実装計画

> **タスク**：LLVM AOT バックエンド + 実行時 DAG スケジューラを実装し、`@auto/@eager/@block` の3つのスケジューリング戦略（L3/L2/L1）を展開する  
> **ベース RFC**：RFC-018（草案）  
> **依存 RFC**：RFC-001（並作モデルとエラー処理）、RFC-008（三層実行時）、RFC-009（所有権/Arc）  
> **日付**：2026-03-10  
> **状態**：進行中  
> **目標マイルストーン**：  
> - M1：LLVM AOT（コンパイル・実行可能、シリアル）  
> - M2：DAG メタデータ + 単一スレッドスケジューリング（Standard Runtime、num_workers=1）  
> - M3：マルチスレッドパラレルスケジューリング + 粒度制御（Full Runtime、num_workers>1）  
> - M4：遅延スケジューリング（Lazy Task Creation）+ **リソース型（Resource）副作用抽象** + **エラー伝播/エラーグラフ** + アノテーション統合

---

## 摘要（実装クローズ）

- `yaoxiang` に LLVM バックエンドを新規追加（feature gate）。`BytecodeModule` を機械語（COFF/ELF/Mach-O）にコンパイルし、実行時にロード・実行可能にする。
- **安定 ABI** を導入：AOT 生成コードと実行時は `extern "C"` の `RtValue/RtContext` を介してやり取りし、Rust enum ABI の不安定問題を回避する。
- RFC-018 の核心を展開：**関数ブロック内 DAG** + **遅延スケジューリング（Lazy Scheduling）**。并发/シリアルは **DAG エッジ（Data/Control/Spawn）** と **リソース型（Resource）ルール** で決定する。エラーは RFC-001 に沿って依存エッジを伝播し、エラーグラフを形成できる。

---

## 公共インターフェース/動作変更（外部に見えうる部分）

1. **Cargo features**
   - 新規 `llvm-aot` feature：LLVM/inkwell 依存と AOT バックエンドを有効化。デフォルトオフ（LLVM なし環境でもビルド可能を保証）。
2. **CLI**
   - `yaoxiang run` に `--backend {interpreter,llvm}` を追加（デフォルトは interpreter）。
   - 任意：`--runtime {embedded,standard,full}` と `--workers <N>` で実行時レベルと并发度を制御（RFC-008）：
     - `--runtime embedded`：即時実行（DAGなし、スケジューラ機能なし、組込み/最小限シナリオ向け）
     - `--runtime standard`：DAG + Scheduler（num_workers=1 は非同期、>1 はパラレル）
     - `--runtime full`：standard + WorkStealer（高度機能、オプション）
3. **実行時 ABI（内部だがモジュール間）**
   - 新規 `RtValue`（`#[repr(C)]`）と `RtContext`（ポインタ/基本型のみを含む）を追加し、AOT と runtime のやり取り境界とする。

---

## 重要設計制約（RFC-001 / RFC-008 / RFC-018 との整合）

### A. 並発セマンティクス（L1/L2/L3 はメンタルモデルのみ）

- **L3（デフォルト / @auto）**：透明な并发。DAG を構築。呼び出しに出会うと「遅延評価可能な値」を返す。**値が必要になった時点で評価をトリガー**する。
- **L1（@block）**：標準ライブラリが提供する（RFC-008）。セマンティクスは「強制急切評価」で、DAG 遅延キューに入らない。主にデバッグと重要な順序セクション向け。
- **L2（spawn）**：**@block スコープ内でのみ使用可能**（RFC-001/008）。同期コード内に並作を挿入する用途。Full Runtime の機能。

### B. 実行時三層（RFC-008）

- **Embedded Runtime**：即時実行。DAG を構築しない選択も可能（メモリ/起動時間削減）。制限された環境向け。
- **Standard Runtime**：DAG + Scheduler がコア（遅延評価は非同期/パラレルの自然なサポート）。
- **Full Runtime**：standard に WorkStealer を追加し、標準ライブラリレベルの `@block` / `spawn` などの機能を提供する。

### C. DAG 構築範囲とメモリ（RFC-001/018）

- DAG は**単一関数体/ブロック内**のみ構築。被呼び出し関数の本体を展開しない（エラーグラフと DAG ノード爆発の回避）。
- DAG メタデータには**ノード/エッジの安定 ID** と **Span** を持たせる（エラー伝播とエラーグラフのローカライズに使用）。

### D. 副作用抽象（RFC-001：リソース型）

- 追加の「明示的副作用アノテーションシステム」は導入しない。副作用は**リソース操作**として統一：
  - 引数型に `Resource`（またはその派生リソース型）を含む関数呼び出しは、リソース操作とみなす。
  - 同一リソースへの操作は自動的に **ControlEdge** を形成（シリアル化）。異なるリソースへの操作はパラレル可能。
  - 同一リソースかどうかが静的に判断できない場合は、デフォルトで保守的にシリアル化（明示的 unsafe パラレルのヒントを拡張として後から導入可能）。

### E. エラー伝播（RFC-001）

- エラーは DAG の依存エッジを伝わって上昇する（実際の並列実行順序とは無関係）。伝播経路を記録しエラーグラフに使用する。

---

## フェーズ 0：前置条件と制約の確定（1〜2日）

### 0.1 LLVM/inkwell バージョンとビルド方式の確定

**目標**

- LLVM メジャーバージョン = **17** を選択（チーム環境の統一。Windows/Linux/macOS すべて対応バイナリが入手可能）。
- `Cargo.toml` に `inkwell` を追加（`llvm17-0` 対応の feature を有効化）し、`llvm-aot` feature 下に配置する。

**受入基準**

- [ ] `cargo build`（feature なし）成功（LLVM なし環境でもビルド可能）。
- [ ] `cargo build -F llvm-aot` LLVM 17 環境設定済みの場合に成功。

**テスト項目**

- [ ] CI/ローカル：2つのビルドマトリックス（`default` と `-F llvm-aot`）で少なくとも1プラットフォームが成功。
- [ ] 最小 smoke：`cargo test -F llvm-aot` が空テストモジュールで起動・実行可能（リンク検証のみ）。

---

### 0.2 LLVM 環境探测とエラーメッセージ

**目標**

- ビルド時/実行時探测の説明を追加：`llvm-config`/LLVM ディレクトリがない場合に操作可能なエラーを提示（インストール方法、接頭辞環境変数の設定方法）。

**受入基準**

- [ ] LLVM がない場合、エラーメッセージに以下の情報を含む：期待バージョン（17）、利用可能環境変数（`LLVM_SYS_170_PREFIX` または `LLVM_CONFIG_PATH` など）とサンプルパス。

**テスト項目**

- [ ] LLVM なしマシンで `cargo build -F llvm-aot` を実行し、完全なヒントを出力して panic しない（コンパイル時エラーで十分）。

---

### 0.3 並作モデル実装制約の確定（RFC-001/008 との整合）

**目標**

- 以下の実装制約を明確にして固定化（コードコメント/開発ドキュメントとテストケースに記述）：
  - `spawn` は `@block` スコープ内でのみ許容（パース/型検査/IR 段階でいずれも防御）。
  - `@block` のセマンティクスは「急切評価」。標準ライブラリ機能として提供（最初はコンパイラ組み込み実装でも可。ただし将来標準ライブラリへの委譲を前提としたインターフェースを保持）。
  - DAG は関数ブロック内でのみ構築。安定した `node_id` と `span` を持つ（エラー伝播/エラーグラフをサポート）。
  - リソース型（Resource）が ControlEdge 生成を駆動。追加のユーザー可见 effect アノテーション体系は導入しない。
  - **パラレル安全制約（RFC-001/009）**：ノードがキャプチャ/返す値が `Send + Sync`（または言語側の同等功能制約）を満たす場合のみクロススレッドパラレルを許可。そうでない場合はシリアル化降格（または単一 worker への固定が必要）。

**受入基準**

- [ ] コンパイラが不正な `spawn` シナリオで明確なエラー（含 Span）を出す。
- [ ] `@block/@eager/@auto` のセマンティクス差異が最小例で観測・テスト可能。
- [ ] この計画と RFC-001/008/018 の主要決定的合意がドキュメント上で一貫し、矛盾条目がない。

**テスト項目**

- [ ] `spawn` 位置制限テスト：@block 以外で spawn が出現するとエラー。
- [ ] DAG scope テスト：DAG が関数体を越えて展開しないことを確認（ノード数と呼び出しレベルが非依存）。
- [ ] Send/Sync 制約テスト：
  - `spawn` が非 `Send` 値をキャプチャするとエラー（含 Span）。
  - `@auto` 下で非 `Send + Sync` 値を含むノードはクロススレッドスケジューリングされない（`std.concurrent.thread_id` で検証）。

---

## フェーズ 1：LLVM バックエンドスケルトンと選択スイッチ（1〜2日）

### 1.1 新規バックエンドモジュールと RFC-018 ディレクトリ構造の整合

**目標**

- `src/backends/llvm/` を新規追加：`mod.rs / context.rs / types.rs / values.rs / codegen.rs / dag.rs / scheduler.rs / tests.rs` を含む（之後マージ/分割可）。
- `src/backends/mod.rs` に `#[cfg(feature = "llvm-aot")] pub mod llvm;` でモジュールを露出。

**受入基準**

- [ ] `cargo test`（デフォルト feature）成功。
- [ ] `cargo test -F llvm-aot` 成功（LLVM バックエンドの機能が未完成でも）。

**テスト項目**

- [ ] ユニット：`src/backends/llvm/tests.rs` に少なくとも1つのコンパイル時テストが存在（モジュール参照の検証のみ）。

---

### 1.2 バックエンド選択：CLI/ライブラリ側注入ポイント

**目標**

- CLI `Run` サブコマンドに `--backend` パラメータを追加（ValueEnum）：`interpreter`（デフォルト）/ `llvm`（feature 要）。
- `yaoxiang::run_*` パスにバックエンド選択分岐を追加。`fn make_executor(kind, config) -> Box<dyn Executor>`（または列挙型ディスパッチ。trait object 回避可）。

**受入基準**

- [ ] `yaoxiang run file.yx` は解釈器に回り、動作不变。
- [ ] `yaoxiang run --backend llvm file.yx`：feature なしの場合明確なエラー。feature ありの場合 LLVM 実行パスに進む（「未実装」返答でも制御されたエラーであるべき）。

**テスト項目**

- [ ] CLI パラメータ解析テスト（`tests/integration` に追加）。
- [ ] 負向テスト：feature なし時に `--backend llvm` を渡すと読みやすいエラーメッセージを返す。

---

## フェーズ 2：安定 ABI（RtValue/RtContext）と Runtime API（3〜5日）

> このフェーズは「LLVM 生成コードが実行可能」の鍵：クロスコンバウンダリの値表現をまず安定させる必要がある。

### 2.1 `RtValue`（安定 C ABI）の定義

**目標**

- `#[repr(C)] struct RtValue { tag: u8, _pad: [u8; 7], payload: u64 }` を定義（または16バイト構造体。アライメント簡素化）。
- `#[repr(u8)] enum RtTag { Unit, Bool, Int, Float, Handle, Async, Error }` を定義（最小セット。拡張可能）。
- 約束：
  - Int：`payload` = `i64` bits
  - Float：`payload` = `f64` bits
  - Bool：0/1
  - Handle：`payload` = `usize`（u64 に拡張）
  - Async：`payload` = `TaskId`（u64）
  - Error：`payload` = エラーコードまたはポインタ（MVP はエラーコード優先）

**受入基準**

- [ ] `RtValue` が Rust 内部で安全に構築/読み取り可能（UB なし）。`Debug` 出力と基本アサートユーティリティ関数を持つ。
- [ ] LLVM IR と整合：inkwell で同レイアウトの struct type を生成可能（フィールド順序/サイズ一致）。

**テスト項目**

- [ ] `RtValue` roundtrip：int/float/bool/unit の encode/decode ユニットテスト。
- [ ] ABI サイズテスト：`size_of::<RtValue>()` と `align_of::<RtValue>()` が固定（硬直アサート。未来の誤改変防止）。

---

### 2.2 `RtContext`（実行時コンテキスト）の定義

**目標**

- `#[repr(C)] struct RtContext` を定義。ポインタ/整数のみを含む：
  - `heap: *mut Heap`
  - `ffi: *const FfiRegistry`
  - `scheduler: *mut DynScheduler`（または具象実装を指すポインタ）
  - `max_parallelism: usize`
  - `error_graph: *mut ErrorGraph`（任意：RFC-001 のエラー伝播記録用。MVP は null 可）
  - 予約フィールド（バージョン番号/flags）。最小化を維持（KISS）。

**受入基準**

- [ ] `RtContext` が解釈器/LLVM executor から生成され生成コードに渡される。
- [ ] Rust 非安定レイアウトフィールドを直接含まない（`Heap`/`FfiRegistry` 値を直接埋め込み禁止）。

**テスト項目**

- [ ] `RtContext` 構築/破棄のメモリ安全性テスト（実際の LLVM 不要）。

---

### 2.3 Runtime C API：生成コードが呼び出す最小関数セット

**目標**

- `#[no_mangle] extern "C"` 関数を提供（命名統一接頭辞 `yx_rt_*`）。MVP 最低 포함：
  - `yx_rt_force(ctx: *mut RtContext, v: RtValue) -> RtValue`
  - `yx_rt_lazy_call(ctx, callee: *const u8 /*関数ポインタ*/, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_native_call(ctx, name_ptr: *const u8, name_len: usize, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_panic(code: u32)` または `yx_rt_trap(msg_ptr, len)`（デバッグ用）
- 制約：AOT 生成コードは**上記 API 経由でのみやり取り**。Rust 構造体を直接操作しない。

**受入基準**

- [ ] Runtime API は LLVM なしでもコンパイル可能（`llvm-aot` feature 制御：API 常駐または feature 下のみ提供均可。可テストであること）。
- [ ] `yx_rt_native_call` が `FfiRegistry` のハンドラを呼び出し可能（MVP は Int/Float/Bool/Unit 引数と返り値のみサポート。不支持時は Error RtValue を返す）。失敗時 `node_id/span_id` をエラーグラフに記録（有効な場合）。

**テスト項目**

- [ ] 純粋 Rust ユニットテスト：`yx_rt_native_call` を直接呼び出し、`std.io.println`（または自己登録関数）パスが使用可能であることを検証。
- [ ] エラー経路テスト：存在しない native 名称を渡すと `Error` RtValue を返し `ExecutorError::FunctionNotFound` に変換可能。

---

## フェーズ 3：LLVM Codegen インフラ（2〜3日）

### 3.1 LLVM コンテキスト/モジュール/TargetMachine 初期化

**目標**

- `context.rs`：inkwell `Context/Module/Builder` のライフサイクルをラップ。
- Target 初期化：`PlatformDetector`（`LLVM_TARGET` サポート）に従ってホストトリプル + データレイアウトを設定。
- 出力サポート：
  - LLVM IR（`.ll`）デバッグ用
  - Object（`.o/.obj`）AOT 用

**受入基準**

- [ ] 任意の空 `BytecodeModule` に対し、`main` を含む LLVM Module を生成可能（関数本体が Unit を返すだけでも可）。
- [ ] IR 検証可能（LLVM verify を呼び出す。失敗時は読み取り可能なエラーを返す）。

**テスト項目**

- [ ] ユニット：最小 module 生成して verify 成功。
- [ ] スナップショットテスト（任意）：`.ll` の重要片段に対する文字列包含アサート（完全なスナップショット Brittle 回避）。

---

### 3.2 `TypeMap`：YaoXiang 型 → LLVM 型（MVP）

**目標**

- `types.rs`：`fn llvm_type(yao_type: &Type) -> BasicTypeEnum` を実装。初期対応：
  - `Int` → i64
  - `Float` → f64
  - `Bool` → i1
  - `()`/Void → void（または `RtValue(Unit)` で統一返り値）
- 戦略選択（ABI 面積削減のため）：**全関数を `RtValue` 統一返り値**（型別返り値にしない）。codegen とスケジューラ/FFI が統一的に処理。型情報は静的検査と `RtValue` 構築/分解ロジック生成に使用。

**受入基準**

- [ ] `TypeMap` の上記型マッピングが安定。LLVM IR での関数シグネチャが一貫：`fn(*mut RtContext, *const RtValue, usize) -> RtValue`。

**テスト項目**

- [ ] `TypeMap` 単体テスト：`Type::Int/Float/Bool/Void` が LLVM 型生成に成功。
- [ ] 生成関数シグネチャが LLVM module 内で検索可能で引数/返り型アサート一致。

---

## フェーズ 4：命令翻訳 MVP（5〜8日）

### 4.1 レジスタから LLVM 値へのマッピング（SSA 化の最小実装）

**目標**

- `values.rs`：仮想レジスタ `Reg(u16)` → LLVM `Value` のマッピングテーブルを実装（基本ブロックスコープ管理）。
- 約束：全レジスタ値を `RtValue` で表現（型爆発/ABI 不一致回避）。演算/比較前にヘルパーで強制/アンパック。

**受入基準**

- [ ] 生成コードが制御フロー分岐後にレジスタ値を正しくマージ可能（phi 使用または `RtValue` レベルで統一処理）。

**テスト項目**

- [ ] ユニット：if/else を含む `BytecodeFunction` から IR 生成して verify 成功。
- [ ] リグレッション：同一レジスタへの複数代入で use-before-def が発生しない（debug モードで trap/エラーを挿入）。

---

### 4.2 コア命令サブセットの翻訳（「実行可能」を満たす範囲）

**目標**

- `codegen.rs`：最低でも以下の `BytecodeInstr` を実装：
  - `LoadConst`（Int/Float/Bool/String。String は Error 降格または当面不支持可）
  - `Mov`
  - `BinaryOp`（Add/Sub/Mul/Div：Int と Float 各自パス）
  - `Compare`（Eq/Lt/Gt など）
  - `Jmp/JmpIf/JmpIfNot/Return/ReturnValue`
  - `CallNative`（`yx_rt_native_call` 経由）
  - `CallStatic`（2戦略：`@block` は直接 call。`@auto` は `yx_rt_lazy_call` 経由で Async を返す）
- 強制ルール：算術/比較/分岐に関わる全オペランドは事前に `yx_rt_force` が必要（透明な并发の「値が必要になったらトリガー」）。

**受入基準**

- [ ] AOT バックエンドが簡易プログラムを実行可能：
  - 純粋算術
  - if/else
  - `std.io.println` 呼び出しによる出力
- [ ] 未対応の命令は panic ではなく読み取り可能なエラーを発生させる。

**テスト項目**

- [ ] 統合：新規 `tests/integration/llvm_aot_smoke.rs`（feature gate）。5つのプログラム片段を実行して結果/出力をアサート（stdout リダイレクトで実現可）。
- [ ] 負向：`MakeClosure/CallVirt/...` に出会うと明確な「未実装」エラーを返す。

---

## フェーズ 5：機械語成果物と実行（AOT クローズ）（3〜6日）

### 5.1 成果物フォーマット：Object + メタデータ（2ファイル、最初はシンプルに）

**目標**

- `CompiledArtifact`（Rust 側構造体）最低 포함：
  - `object_bytes: Vec<u8>`（COFF/ELF/Mach-O）
  - `dag_meta: DAGMetadata`（最初は空可）
  - `entries: Vec<EntryPoint>`（最低 main）
  - `type_info: TypeInfo`（MVP は空可）
- 出力戦略：
  - `yaoxiang build-aot input.yx -o out/` で `program.obj` + `program.dag.ron`（または `.json`）を生成
  - `yaoxiang run --backend llvm` はデフォルトで「メモリコンパイル+直接実行」（ディスク保存なし）。開発効率優先。

**受入基準**

- [ ] build-aot が2ファイル生成し、メタデータがデシリアライズ可能。
- [ ] run/llvm パスがディスクファイルに依存せずとも実行可能。

**テスト項目**

- [ ] ファイル生成テスト：`.obj` が非空であること、`.dag.ron` がパース可能でバージョン番号が一致することを検証。
- [ ] 互換性テスト：ビルドモード（Debug/Release）ごとに異なる最適化レベル出力（少なくとも区別可能）。

---

### 5.2 実行方式：「メモリ実行」を先に、「ディスクロード」は後（2ステップで受入）

**目標**

- Step A（先にデリバリー）：LLVM ExecutionEngine（または ORC JIT）を使用して生成済み module を実行（セマンティクスクローズ確認用、開発効率最高）。
- Step B（AOT に準拠）：TargetMachine で object bytes を生成し、「動的ライブラリリンク/ロード」パスで実行：
  - object を `.dll/.so/.dylib` にリンク（システムリンカまたは lld を呼び出す。`llvm-aot` feature の追加要件）
  - `libloading` でシンボルをロードして入口関数を呼び出す

**受入基準**

- [ ] Step A：`--backend llvm` が同プロセス内で実行可能（外部リンカに依存しない）。
- [ ] Step B：`build-aot` 生成の成果物が `run-aot`（新規サブコマンドまたは内部パス）にロードされて実行可能。

**テスト項目**

- [ ] Step A：ユニット/統合テストがデフォルトで実行（開発が速い）。
- [ ] Step B：「外部リンカ要」とマークされたオプション統合テスト（CI に環境がある場合有効化。ローカルでは手動可）。

---

## フェーズ 6：DAG メタデータ生成（4〜7日）

### 6.1 `DAGMetadata`（バージョン管理）の定義

**目標**

- `dag.rs` で定義：
  - `dag_version: u32`
  - `nodes: Vec<DAGNode>`（`node_id` と `span_id` を含む。エラー伝播用）
  - `edges: Vec<DAGEdge>`（エッジタイプ付：Data/Control/Spawn）
- `DAGEdge` 最低 포함：
  - `from: u32`
  - `to: u32`
  - `kind: EdgeKind { Data, Control, Spawn }`
- 競合/スケジューリングルール（RFC-001）：
  - DataEdge + DataEdge：パラレル可能（他依存がない場合）
  - ControlEdge を含む組み合わせ：シリアル化必須（順序維持）
- `DAGNode` 最低 포함：
  - `node_id: u32`（関数内唯一）
  - `ip: u32`（call 命令位置）
  - `strategy: ScheduleStrategy { Serial, Eager, Lazy }`（アノテーションまたはデフォルト戦略から）
  - `span_id: u32`（ローカライズとエラーグラフ）
  - `thread_safety: ThreadSafety { SendSync, LocalOnly }`（型システムから導出。`LocalOnly` ノードはクロススレッドスケジューリング禁止）
- 約束：ノードは「スケジューリング可能な呼び出し点」のみを記述。引数は実行時に `yx_rt_lazy_call` でキャプチャ（静的式評価の複雑さを回避）。

**受入基準**

- [ ] `DAGMetadata` がシリアライズ/デシリアライズ可能（既存の `ron` または `serde_json` を使用）。
- [ ] `dag_version` が不一致の場合ロードエラー。

**テスト項目**

- [ ] シリアライズ roundtrip 単体テスト。
- [ ] バージョン不一致単体テスト（古いバージョンを手動構築）。
- [ ] `thread_safety` 導出テスト：最低1つの `LocalOnly` シナリオをカバーし、`num_workers>1` 下でクロススレッド実行されないことを検証。

---

### 6.2 リソース型と ControlEdge 生成（副作用抽象の最小使用可能セット）

**目標**
> **更新**：RFC-001 に従い、副作用は追加の effect システムで表現せず、**リソース型（Resource）** で抽象化してリソース操作とし、ControlEdge を生成する。

- リソース操作認識（MVP）：
  - 呼び出しの引数型に `Resource` またはその派生リソース型（`Console/FilePath/HttpUrl/DBUrl` など）が含まれる場合、その呼び出し点はリソース操作。
  - 標準ライブラリのリソース操作関数には認識可能な型制約が必要（推奨：std エクスポートシグネチャにリソース型を明示的に含むか、FFI registry のエクスポートメタデータに「リソース操作」マークと関連リソース引数位置を含める）。
- ControlEdge 生成（MVP）：
  - **同一リソース値/ハンドル（同一 SSA 値または同一定数保持キー）** に対する複数リソース操作を、プログラム順序で ControlEdge を追加（自動シリアル化）。
  - 同一リソースかどうかが判断できない場合（エイリアス/複雑ソース）はデフォルトで保守的にシリアル化（将来的に明示的 unsafe パラレルのヒントを拡張として導入可）。
  - **リソース識別子はデータフローに沿って伝播（RFC-001）**：リソース競合検出は「値等価/同一ソース」を基準とし、「リソース型同一」を基準としない（2つの異なる `FilePath` 値はパラレル可能。同一 `FilePath` 値はシリアル化必須）。

**受入基準**

- [ ] 例：`log → save → log` が Console/FilePath リソースにより ControlEdge を形成し、シリアル化が安定。
- [ ] 異なるリソース操作はパラレル可能。
- [ ] リソース操作認識が安定（同一入力 module に対する複数回の結果が一致）。

**テスト項目**

- [ ] ユニット：リソース型引数認識テスト（Resource 引数存在時に ControlEdge が生成される必要がある）。
- [ ] ユニット：同一リソース値（同一変数/同一定数）上の2つのリソース操作が ControlEdge を生成する必要がある。異なるリソース値（異なる変数/異なる定数）は ControlEdge 生成不要。
- [ ] 統合：複数の `std.io.println/std.io.write_file` を含むサンプルを実行し、出力/書き込み順序が解釈器と一致することをアサート。

---

### 6.3 L1 自動フォールバック（小関数を `@block` に降格、スケジューリングオーバーヘッド回避）

> **ソース**：RFC-001 5.2（L1 自動フォールバック）。  
> **目的**：セマンティクスを変えないまま、小関数の DAG/スケジューラオーバーヘッドを削減（特に解釈器バックエンドと AOT バックエンドの統一動作）。

**目標**

- コンパイル時に関数に対して軽量閾値判定を実行。条件のいずれかを満たす場合、該関数のデフォルト戦略を `Serial` に降格（または関数内の特定ブロックを降格）：
  - 命令数 `< 50`
  - DAG ノード数 `< 10`
- CLI/設定でスイッチを露出（MVP：内部設定のみでも可）：
  - `--l1-threshold=<N>` 閾値調整
  - `--no-l1-fallback` 自動フォールバック無効化

**受入基準**

- [ ] 小関数が `@auto` 下では実際に DAG/スケジューラキューに入らない（統計フィールドまたはトレースで検証可）。フォールバックなしと結果が一貫。
- [ ] 大関数は影響を受けない。強制アノテーション `@eager/@block` は自動フォールバックより優先。

**テスト項目**

- [ ] ユニット：境界値（49/50 命令、9/10 ノード）を構築してフォールバック発火を検証。
- [ ] リグレッション：同一ソースに対してフォールバック有効/無効の双方で出力と返り値が一貫。

---

## フェーズ 7：実行時 DAG スケジューラ（Lazy Scheduling のコア）（6〜10日）

### 7.1 タスクモデルの実装（`RtValue::Async` との对接）

**目標**

- `scheduler.rs`（または `src/backends/runtime/` に移行して「解釈器/LLVM 共有」を実現）で実装：
  - `TaskId` 割り当て
  - `TaskState { Pending, Running, Completed(RtValue), Failed(RtValue) }`
  - `spawn(task_fn_ptr, args, deps, node_id, span_id)`：タスク作成。ただし起動遅延可能（エラー伝播/エラーグラフ用）
  - `force(task_id)`：依存トポロジに従って実行トリガーし結果を待機

**受入基準**

- [ ] `yx_rt_lazy_call` が `Async(TaskId)` を返し、タスクが記録される（即時実行ではない）。
- [ ] `yx_rt_force` がタスク実行をトリガーし結果を返す（依存チェーンを含む）。

**テスト項目**

- [ ] 純粋 Rust：モック "compiled fn" ポインタ（実際は Rust `extern "C"` 関数）を使用して3ノード DAG を構築し、依存順序と結果正確性を検証。
- [ ] エラー伝播：下流の force が Error を取得し、デッドロックしない。

---

### 7.2 スケジューリング戦略の実装（Serial/Eager/Lazy）

**目標**

- `Serial`（`@block` に対応）：Async 生成なし。call 即時実行。スケジューラインターフェースをバイパス可。
- `Eager`：タスク作成後即座に `force`（順序保証）。デバッグ/セマンティクス整合用。
- `Lazy`（デフォルト `@auto`）：値が必要になった時点で `force`。スケジューラはバックグラウンドウィンドウ内で「準備完了」タスクを先回り起動可能（并发数制限に従う）。
-  ボトムアップ（RFC-001/008）：実行時動作は「結果が必要とされている箇所から逆方向に評価トリガーする」特性を反映。未消費かつリソース操作を含まない（ControlEdge なし）ブランチ/孤立 DAG は実行しない（オーバーヘッド削減）。リソース操作は ControlEdge により順序保証と完了を遵守（解釈器と整合）。
-  バックグラウンド DAG（RFC-018）：同一スコープに複数の長時間実行/無限ループタスクが存在する場合、スケジューラは**協調的スライシング**を提供（例如 based on budget or explicit `yield_now`）。メインデ DAG の飢餓と「ループ内で固着」を回避。

**受入基準**

- [ ] 同一プログラムが Serial/Eager/Lazy の3戦略で結果が一貫。
- [ ] Lazy 下では call 結果が force/使用されていない場合、タスクは実行されない（Lazy Task Creation）。

**テスト項目**

- [ ] 比較テスト：3戦略の出力が一貫。
- [ ] Lazy スキップテスト：「計算するが未使用」のブランチ/変数を記述し、対応するタスク実行カウントが0であることをアサート（スケジューラ統計フィールド使用）。
- [ ] バックグラウンドスライシングテスト：2つの長時間実行タスク + 1つのメインタスクを構築し、時間窓内で3者すべてが進行していることをアサート（カウンタまたは `thread_id` + ログ統計使用）。

---

### 7.3 並発制御と粒度制御

**目標**

- 並発上限：`max_parallelism = num_workers * 2`（RFC-018 推奨）。
- リソース制約：同一リソースへの操作は ControlEdge によりシリアル実行必须（RFC-001 リソース型ルール）。スケジューラは ControlEdge 順序を乱さない。
- スレッド安全制約（RFC-001/009）：スケジューラは `DAGNode.thread_safety` を遵守：
  - `SendSync`：worker 間パラレル実行可（并发上限と依存制約に従う）
  - `LocalOnly`：クロススレッドスケジューリング/ワークスチール狩り禁止。必要に応じてシリアル降格（または生成した worker への固定）
- アダプティブ粒度（MVP）：実行待ちタスク数が并发上限大幅超過時、「準備完了かつ**ControlEdge 制約なし**の複数タスク」を一括マージしてバッチ提交（同一 worker がシーケンシャルに一批执行、スケジューリングオーバーヘッド削減）。

**受入基準**

- [ ] 大量独立・非リソース制約タスク（1e4）がメモリ爆発を引き起こさない（タスク構造は O(并发数) または制御可能上限）。
- [ ] `LocalOnly` ノードが `num_workers>1` 下でクロススレッド実行されない（`std.concurrent.thread_id` で検証）。
- [ ] リソース操作（`std.io.*` など）の出力/副作用順序が解釈器順序と厳密一致。

**テスト項目**

- [ ] 負荷テストユニット：10000個のモックタスクを構築。ピークメモリ/タスク数が制御可能であることを検証（統計アサート、正確なメモリ測定不要）。
- [ ] LocalOnly 統合テスト：`LocalOnly` ノードを含むサンプルを構築。`num_workers>1` 下でその実行スレッド ID が変わらないことをアサート。
- [ ] リソース順序統合テスト：複数リソース操作（println/write_file）はソースコード順序で出力/ディスク保存される必要がある。

---

### 7.4 エラー伝播とエラーグラフ記録（RFC-001 最小クローズ）

**目標**

- 最小 `ErrorGraph` データ構造を定義（デバッグ/トレース専用にまず実装可）：
  - ノード：`node_id + span_id + message/error_code`
  - エッジ：`from_node_id -> to_node_id`（「エラーが依存ノードから consumer ノードへ伝播した」ことを示す）
- 記録ストラテジー（RFC-001 合意）：
  - エラーは依存エッジを伝播して上流に戻る。**実際の実行順序に依存しない**。
  - DAG は関数内でのみ構築のため、エラーグラフも関数级别に制限され、メモリ爆発を回避。
- ABI との整合：
  - `yx_rt_lazy_call/yx_rt_native_call` は `node_id/span_id` を携带必须（フェーズ 2.3 で確定済み）
  - タスク失敗と `force` がエラーを返す場合、`ErrorGraph` に書き込み（`ctx.error_graph != null` の場合）

**受入基準**

- [ ] 依存チェーンの末端ノード失敗時、top 層 consumer がエラーを受信（失敗ノードの span ローカライズ可能）。
- [ ] 並列実行下でもエラー伝播パスが安定で再現可能（スケジューリング順序と無関係）。

**テスト項目**

- [ ] ユニット：3ノード依存チェーンを構築。中央ノード失敗をシミュレート。`ErrorGraph` エッジが `leaf->mid->root` であることをアサート。
- [ ] 並列リグレッション：num_workers>1 で複数実行して `ErrorGraph` 構造が一貫。

---

## フェーズ 8：構文アノテーション統合（@block/@eager/@auto）（5〜8日）

### 8.1 フロントエンドのアノテーション対応とバイト码/メタデータへの伝播

**目標**

- 解析層：関数/ブロックアノテーション `@block`、`@eager` を認識。デフォルトは `@auto`。
- 解析/型検査：`spawn` は `@block` スコープ内でのみ出現可能を強制（RFC-001/008）。
- IR/Bytecode：`BytecodeFunction` または追加サイドテーブルでデフォルト戦略を携带。call-site レベルで lazy/eager/direct のいずれか決定可能。

**受入基準**

- [ ] アノテーションなし：デフォルト Lazy（@auto）。
- [ ] `@block`：該スコープ内で Async 生成なし。動作が解釈器シリアルと一致。
- [ ] `@eager`：タスク作成後即座に force（結果整合、デバッグ容易）。

**テスト項目**

- [ ] フロントエンド：アノテーションを含むパース/IR 生成テスト（AST/IR アサート）。
- [ ] バックエンド：同一ソースに対して各アノテーションを付与して実行。動作が策略に従うことを検証。

---

### 8.2 標準ライブラリ：`@block` と `spawn` の実行時インターフェース（Full Runtime）

> **ソース**：RFC-008（@block は標準ライブラリで提供）、RFC-001（spawn の待機セマンティクスは標準ライブラリが制御）。

**目標**

- 標準ライブラリ実行時モジュールを追加（推奨パス：`std.runtime` または `std.full`）。提供：
  - `block`：强制急切評価（スコープ戦略を `Serial` にする/DAG キュー不入相当）
  - `spawn`/`join_all`（または暗黙 join）：`@block` スコープ内で並列タスク作成して完了まで待機
- コンパイラは MVP で組み込み実装しても可。ただし標準ライブラリに委譲可能な抽象化インターフェースを保持（将来リファクタリングコスト回避）。

**受入基準**

- [ ] `@block` 関数内の `spawn { ... }` ブロックが並列実行され、スコープ終了前に完了する（「サイレントバックグラウンドタスクリーク」なし）。
- [ ] `@block` の動作と L3 デフォルト并发動作が明確に区別可能（例如：DAG キューに入るか否か、Async 値を生成するか否か）。

**テスト項目**

- [ ] 統合：2つの `spawn { std.concurrent.sleep(50) }` を含むサンプルが複数 worker 下で所要時間が1回の sleep に近い（並列の粗粒度検証）。
- [ ] 負向：@block 外で spawn を使用するとエラー（0.3/8.1 と一致）。

---

## フェーズ 9：エンドツーエンドとパフォーマン基準（継続推進）

### 9.1 解釈器との整合性テスト（セマンティクス整合）

**目標**

- 「命令サブセットをカバーする」テストスイートを選択：算術、分岐、関数呼び出し、native IO。
- 同一ソースを interpreter と llvm backend でそれぞれ実行し、以下を比較：
  - 返り値（存在する場合）
  - stdout 出力（注入/リダイレクト必要）
  - エラータイプ（`ExecutorError` 整合尽量）

**受入基準**

- [ ] テストスイートにおいて LLVM backend と 解釈器の結果が一致。

**テスト項目**

- [ ] `tests/integration/llvm_vs_interpreter.rs`（feature gate）最低10用例。
- [ ] リグレッション：新用例追加時は両バックエンドで実行必須。

---

### 9.2 基準：解釈器 vs AOT（粗粒度）

**目標**

- `benches/` に基準を追加：純粋計算（IO なし）、大量 call タスク（並列效益）、混合 IO（順序制約）。

**受入基準**

- [ ] AOT が純粋計算用例で 解釈器より有意に高速（具体的な倍率は約束しないが、明らかに低速ではいけない）。
- [ ] Lazy スケジューリングのオーバーヘッドが観測・特定可能（スケジューラ統計出力が可能）。

**テスト項目**

- [ ] criterion bench（手動/CI 任意）レポート生成。ベースライン記録。

---

## 仮定とデフォルト（ビジネス要件でカバーされていない場合の選択）

- デフォルト LLVM メジャーバージョン = **17**。チームツールチェーンが異なる場合は `inkwell` feature とドキュメントを統一修正即可。
- AOT 実行パスは「2段階方式」：まずメモリ実行（開発検証）、次にディスク保存/リンク/ロード（本当の AOT）。
- 初期 `llvm-aot` は1セットの MVP 命令サブセットのみ対応を約束。クロージャ/動的ディスパッチ/例外などの高度機能は不要時に拡張（対応時は明確な「未実装」エラーを返す）。
- DAG 依存エッジは**実行時に args の Async TaskId から動的に導出可能**。コンパイル時 edges フィールドはまずオプションな最適化とデバッグ検証用。M2 デニバレーをブロックしない。
  - **補足（RFC-001）**：ControlEdge の主要なソースはリソース型ルール。リソース情報がない場合、正確性を保証するためデフォルトで保守的にシリアル化。