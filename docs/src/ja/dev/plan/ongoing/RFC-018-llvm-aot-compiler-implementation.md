# RFC-018 LLVM AOT コンパイラと L3 透過的並行性（DAG 遅延スケジューリング）実装計画

> **タスク**：LLVM AOT バックエンド + ランタイム DAG スケジューラを実装し、`@auto/@eager/@block` の3つのスケジューリング戦略（L3/L2/L1）を実装する  
> **RFC ベース**：RFC-018（草案）  
> **依存 RFC**：RFC-001（並作モデルとエラー処理）、RFC-008（三層ランタイム）、RFC-009（所有権/Arc）  
> **日付**：2026-03-10  
> **状態**：進行中  
> **目標マイルストーン**：  
> - M1：LLVM AOT（コンパイル・実行可能、シリアル）  
> - M2：DAG メタデータ + 単一スレッドスケジューリング（Standard Runtime、num_workers=1）  
> - M3：マルチスレッド並列スケジューリング + 粒度制御（Full Runtime、num_workers>1）  
> - M4：遅延スケジューリング（Lazy Task Creation）+ **リソース型（Resource）の副作用抽象** + **エラー伝播/エラーグラフ** + アノテーション統合

---

## 摘要（実装のクローズ）

- `yaoxiang` に LLVM バックエンド（feature gate）を追加し、`BytecodeModule` を機械語コード（COFF/ELF/Mach-O）にコンパイルしてランタイムでロード・実行可能にする。
- **安定 ABI** を導入：AOT 生成コードとランタイムは `extern "C"` の `RtValue/RtContext` を通じてやり取りし、Rust enum ABI の不安定性问题を取り除く。
- RFC-018 の核心を実装：**関数ブロック内 DAG** + **遅延スケジューリング（Lazy Scheduling）**。並行/シリアルは **DAG エッジ（Data/Control/Spawn）** と **リソース型（Resource）ルール**で決定する；エラーは RFC-001 に従い依存エッジに沿って伝播し、エラーグラフを形成できる。

---

## 公共インターフェース/動作変更（外部に見えうるもの）

1. **Cargo features**
   - 新しい `llvm-aot` feature：LLVM/inkwell 依存関係と AOT バックエンドを有効化；デフォルトオフ（LLVM なしでもビルド可能を保証）。
2. **CLI**
   - `yaoxiang run` に `--backend {interpreter,llvm}` を追加（デフォルト interpreter）。
   - オプション：`--runtime {embedded,standard,full}` と `--workers <N>` を追加してランタイムレベルと並行度を制御（RFC-008）：
     - `--runtime embedded`：即時実行（DAGなし、スケジューラ機能なし、組込み/極限シナリオ向け）
     - `--runtime standard`：DAG + Scheduler（num_workers=1 は非同期；>1 は並列）
     - `--runtime full`：standard + WorkStealer（高度な機能、オプション）
3. **ランタイム ABI（内部だがモジュール間）**
   - 新しい `RtValue`（`#[repr(C)]`）と `RtContext`（ポインタ/基本型のみ含む）を追加し、AOT と runtime のやり取りの境界とする。

---

## 重要な設計制約（RFC-001 / RFC-008 / RFC-018 との整合）

### A. 並行セマンティクス（L1/L2/L3 はメンタルモデルのみ）

- **L3（デフォルト / @auto）**：透過的並行性；DAG を構築；呼び出しに遭遇すると「遅延評価可能」な値を返し、**値が必要になった時点で評価をトリガー**する。
- **L1（@block）**：標準ライブラリが提供する（RFC-008）、セマンティクスは「強制的な先行評価」、DAG 遅延キューに入らない；主にデバッグと重要な順序付きセクションに使用。
- **L2（spawn）**：**@block スコープ内でのみ使用可能**（RFC-001/008）、同期コード内に並行性を挿入するために使用；Full Runtime 機能。

### B. ランタイム三層（RFC-008）

- **組込みランタイム**：即時実行；DAG を完全に構築しない選択が可能（メモリ/起動時間節約）；制限された環境向け。
- **標準ランタイム**：DAG + Scheduler が核（遅延評価により非同期/並列が自然にサポート）。
- **フルランタイム**：standard 基础上增加 WorkStealer、標準ライブラリ層の `@block` / `spawn` などの能力。

### C. DAG 構築範囲とメモリ（RFC-001/018）

- DAG は**単一の関数体/ブロック内**のみで構築；呼び出された関数体に再帰的に展開しない（エラーグラフと DAG ノードの爆発を避けるため）。
- DAG メタデータには**ノード/エッジの安定 ID** と **Span**（エラー伝播とエラーグラフのローカライズ用）を必ず携带する。

### D. 副作用抽象（RFC-001：リソース型）

- 追加の「明示的な副作用アノテーションシステム」は導入しない；副作用は統一的に**リソース操作**として表現：
  - パラメータ型に `Resource`（またはその派生リソース型）が含まれる関数呼び出しは、リソース操作とみなす；
  - 同一リソースへの操作は自動的に **ControlEdge** を形成（シリアル）；異なるリソースへの操作は並列可能；
  - 静的に同一リソースかどうか判定できない場合は、デフォルトで保守的にシリアル化（将来的に明示的な unsafe 並行ヒントを拡張として導入可能）。

### E. エラー伝播（RFC-001）

- エラーは DAG の依存エッジに沿って伝播し（実際の並列実行順序とは無関係）、エラーグラフ用の伝播パスを記録する。

---

## フェーズ 0：前置条件と制約のロック（1-2 日）

### 0.1 LLVM/inkwell バージョンとビルド方式のロック

**目標**
- LLVM 主バージョン = **17** を選択（チーム環境の統一；Windows/Linux/macOS 全てで対応する配布パッケージが入手可能）。
- `Cargo.toml` に `inkwell` を追加（`llvm17-0` に対応する feature を有効化）し、`llvm-aot` feature 下に配置。

**受け入れ基準**
- [ ] `cargo build`（feature なし）で成功（LLVM なし環境でもビルド可能）。
- [ ] `cargo build -F llvm-aot` が LLVM17 環境が設定されている場合に成功。

**テスト項目**
- [ ] CI/ローカル：2 つのビルドマトリックス（`default` と `-F llvm-aot`）で少なくとも1つのプラットフォームが通る。
- [ ] 最小 smoke：`cargo test -F llvm-aot` が起動して空のテストモジュールを実行できる（リンクのみ検証）。

---

### 0.2 LLVM 環境探测とエラーメッセージ

**目標**
- ビルド時/ランタイム時の探测说明を追加： недостающие `llvm-config`/LLVM ディレクトリがある場合、操作可能なエラーメッセージを提供（インストール方法、接頭辞変数の設定方法）。

**受け入れ基準**
- [ ] LLVM が 없을場合、エラーメッセージに以下を含める：期望バージョン（17）、利用可能な環境変数（例：`LLVM_SYS_170_PREFIX` または `LLVM_CONFIG_PATH`）とサンプルパス。

**テスト項目**
- [ ] LLVM がないマシンで `cargo build -F llvm-aot` を実行し、出力が完整で panic しない（コンパイル時エラーを報告すれば OK）。

---

### 0.3 並作モデルの実装制約のロック（RFC-001/008 との整合）

**目標**
- 以下の実装制約を明確にして固定化（コードコメント/開発ドキュメントとテストケースに明記）：
  - `spawn` は `@block` スコープ内でのみ許可（パース/型チェック/IR 段階で全て防守が必要）。
  - `@block` のセマンティクスは「先行評価」、標準ライブラリ機能として提供（まずはコンパイラ組み込みで MVP を実装してもよいが、将来標準ライブラリへの移管を想定したインターフェースを保持）。
  - DAG は関数ブロック内でのみ構築；安定した `node_id` と `span` を携带する必要がある（エラー伝播/エラーグラフをサポート）。
  - リソース型（Resource）が ControlEdge 生成を驱动し、追加のユーザー可见 effect アノテーション体系を導入しない。
  - **並列安全制約（RFC-001/009）**：ノードのキャプチャ/戻り値が `Send + Sync`（または言語側の同等の制約）を満たす場合にのみクロススレッド並列を許可；そうでなければシリアルにダウングレード（または単一 worker 上で実行）。

**受け入れ基準**
- [ ] コンパイラが不合法の `spawn` シナリオで明確なエラー出す（含 Span）。
- [ ] `@block/@eager/@auto` のセマンティクスの差異が最小例で観測・テスト可能。
- [ ] ドキュメント（本計画）と RFC-001/008/018 の主要な決議が一致し、自己矛盾がない。

**テスト項目**
- [ ] `spawn` 位置制限テスト：@block 之外出现 spawn 必须报错。
- [ ] DAG scope 测试：确认 DAG 不跨函数体展开（节点数与调用层级无关）。
- [ ] Send/Sync 约束测试：
  - `spawn` 捕获非 `Send` 值必须报错（含 Span）。
  - `@auto` 下包含非 `Send + Sync` 值的节点不得跨线程调度（可用 `std.concurrent.thread_id` 统计验证）。

---

## フェーズ 1：LLVM バックエンドスケルトンとセレクタスイッチ（1-2 日）

### 1.1 新しいバックエンドモジュールの追加と RFC-018 ディレクトリ構造との整合

**目標**
- `src/backends/llvm/` を新增し、以下のファイルを含む：`mod.rs / context.rs / types.rs / values.rs / codegen.rs / dag.rs / scheduler.rs / tests.rs`（后续合并/拆分可）。
- `src/backends/mod.rs` で `#[cfg(feature = "llvm-aot")] pub mod llvm;` を通じてモジュールを露出。

**受け入れ基準**
- [ ] `cargo test`（デフォルト feature）が成功。
- [ ] `cargo test -F llvm-aot` が成功（LLVM バックエンドが完全に実装されていなくても OK）。

**テスト項目**
- [ ] ユニット：`src/backends/llvm/tests.rs` で少なくとも 1 つのコンパイル時テストが実行可能（モジュールの参照のみ検証）。

---

### 1.2 バックエンド選択：CLI/ライブラリ側注入ポイント

**目標**
- CLI `Run` サブコマンドに新しい `--backend` パラメータを追加（ValueEnum）：`interpreter`（デフォルト）/ `llvm`（feature が必要）。
- `yaoxiang::run_*` パスにバックエンド選択分支を追加し、`fn make_executor(kind, config) -> Box<dyn Executor>`（または列挙子ディスパッチ、trait object 不要でも可）として抽象化。

**受け入れ基準**
- [ ] `yaoxiang run file.yx` は 여전히インタープリタを使用、動作不变。
- [ ] `yaoxiang run --backend llvm file.yx`：feature が有効になっていない場合、明確なエラー；有効になっている場合、LLVM 実行パスに入る（「未実装」と返っても可控のエラーである必要あり）。

**テスト項目**
- [ ] CLI パラメータ解析テスト（`tests/integration` に追加）。
- [ ] 負向テスト：feature なしで `--backend llvm` を渡した場合、 읽을 수 있는 오류 메시지 반환。

---

## フェーズ 2：安定 ABI（RtValue/RtContext）と Runtime API（3-5 日）

> このフェーズは「LLVM 生成コードが実行可能」にするための鍵：境界を越えた値の表現を安定させる必要がある。

### 2.1 `RtValue`（安定 C ABI）の定義

**目標**
- `#[repr(C)] struct RtValue { tag: u8, _pad: [u8; 7], payload: u64 }`（または 16 バイト構造、アライメント簡略化のため）を定義。
- `#[repr(u8)] enum RtTag { Unit, Bool, Int, Float, Handle, Async, Error }`（最小セット；後で拡張）を定義。
- 約束：
  - Int：`payload` = `i64` bits
  - Float：`payload` = `f64` bits
  - Bool：0/1
  - Handle：`payload` = `usize`（u64 に拡張）
  - Async：`payload` = `TaskId`（u64）
  - Error：`payload` = エラーコードまたはポインタ（MVP はエラーコード先用）

**受け入れ基準**
- [ ] `RtValue` が Rust 内部で安全に構築/読み取り可能（UB なし）、`Debug` 出力と基本アサーションUtil関数具备。
- [ ] LLVM IR との整合：inkwell で同レイアウトの struct type（フィールド順序/サイズ一致）を作成可能。

**テスト項目**
- [ ] `RtValue` roundtrip：int/float/bool/unit の encode/decode ユニットテスト。
- [ ] ABI サイズテスト：`size_of::<RtValue>()` と `align_of::<RtValue>()` が固定（ハードコードされたアサーションで将来の変更を防止）。

---

### 2.2 `RtContext`（ランタイムコンテキスト）の定義

**目標**
- `#[repr(C)] struct RtContext` を定義、ポインタ/整数のみ含む：
  - `heap: *mut Heap`
  - `ffi: *const FfiRegistry`
  - `scheduler: *mut DynScheduler`（または具体実装へのポインタ）
  - `max_parallelism: usize`
  - `error_graph: *mut ErrorGraph`（オプション：RFC-001 のエラー伝播記録用；MVP は null 可能）
  - 予約フィールド（バージョン番号/flags）、だが最小化を維持（KISS）。

**受け入れ基準**
- [ ] `RtContext` がインタープリタ/LLVM executor から構築され生成コードに渡せる。
- [ ] Rust の不安定レイアウトフィールドを直接含まない（`Heap`/`FfiRegistry` 値を直接埋め込み禁止）。

**テスト項目**
- [ ] `RtContext` の構築/破棄のメモリ安全性テスト（実際の LLVM 不要）。

---

### 2.3 Runtime C API：生成コードが呼び出す最小関数セット

**目標**
- `#[no_mangle] extern "C"` 関数を提供（命名統一接頭辞 `yx_rt_*`）、MVP 最小包含：
  - `yx_rt_force(ctx: *mut RtContext, v: RtValue) -> RtValue`
  - `yx_rt_lazy_call(ctx, callee: *const u8 /*関数ポインタ*/, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_native_call(ctx, name_ptr: *const u8, name_len: usize, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_panic(code: u32)` または `yx_rt_trap(msg_ptr, len)`（デバッグ用）
- 制約：AOT 生成コードは**上記の API を通じてのみやり取り**し、直接 Rust 構造体を操作しない。

**受け入れ基準**
- [ ] Runtime API は LLVM なしでもコンパイル可能（`llvm-aot` feature 制御：API は常駐または feature 下のみで提供可能だが、テスト可能である必要あり）。
- [ ] `yx_rt_native_call` が `FfiRegistry` のハンドラを呼び出せる（MVP は Int/Float/Bool/Unit パラメータと戻り値のみサポート；サポートしていない場合は Error RtValue を返す）、失敗時に `node_id/span_id` をエラーグラフに記録（有効な場合）。

**テスト項目**
- [ ] 純粋 Rust ユニットテスト：直接 `yx_rt_native_call` を呼び出し、`std.io.println`（または自己登録関数）パスが使用可能であることを検証。
- [ ] エラーパステスト：存在しない native 名称を渡した場合、`Error` RtValue を返し `ExecutorError::FunctionNotFound` に変換可能。

---

## フェーズ 3：LLVM Codegen インフラストラクチャ（2-3 日）

### 3.1 LLVM コンテキスト/モジュール/TargetMachine 初期化

**目標**
- `context.rs`：inkwell `Context/Module/Builder` のライフサイクルをカプセル化。
- Target 初期化：`PlatformDetector`（`LLVM_TARGET` サポート）に従ってターゲット triple + data layout を設定。
- 出力サポート：
  - LLVM IR（`.ll`）デバッグ用
  - Object（`.o/.obj`）AOT 用

**受け入れ基準**
- [ ] 空の `BytecodeModule` に対して `main` を含む LLVM Module を生成可能（関数体が Unit を返すだけでも OK）。
- [ ] IR が検証可能（LLVM verify を呼び出す；失敗時は읽을 수 있는 오류 반환）。

**テスト項目**
- [ ] ユニット：最小 module を生成して verify 成功。
- [ ] スナップショットテスト（オプション）：`.ll` 关键片段に文字列包含アサーション（完全なスナップショットは brittle なので）。

---

### 3.2 `TypeMap`：YaoXiang Type → LLVM Type（MVP）

**目標**
- `types.rs`：`fn llvm_type(yao_type: &Type) -> BasicTypeEnum` を実装、以下を 우선覆盖：
  - `Int` → i64
  - `Float` → f64
  - `Bool` → i1
  - `()`/Void → void（または `RtValue(Unit)` で統一返戻）
- 戦略選択（ABI 面積削減のため）：**全関数は統一的に `RtValue` を返す**（型別ではなく）、codegen とスケジューラ/FFI が統一的に処理；型情報は静的検査と `RtValue` 構築/解構ロジック生成に使用。

**受け入れ基準**
- [ ] `TypeMap` が上記マッピングを安定させ、LLVM IR での関数シグネチャが一致：`fn(*mut RtContext, *const RtValue, usize) -> RtValue`。

**テスト項目**
- [ ] `TypeMap` 單測：与えられる `Type::Int/Float/Bool/Void` から LLVM 型を生成成功。
- [ ] 生成された関数シグネチャが LLVM module で検索可能で、パラメータ/戻り値型が一致するアサーション。

---

## フェーズ 4：命令翻訳 MVP（5-8 日）

### 4.1 レジスタから LLVM 値へのマッピング（SSA 化の最小実装）

**目標**
- `values.rs`：仮想レジスタ `Reg(u16)` → LLVM `Value` のマッピングテーブルを実装（基本ブロックスコープ管理）。
- 約束：全レジスタ値を `RtValue` で表現（型爆発/ABI 不一致を避ける）、演算/比較前にヘルパーで強制/アンパック。

**受け入れ基準**
- [ ] 生成コードが制御フローの分岐後にレジスタ値を正しくマージ可能（phi 使用または `RtValue` 層での統一型処理）。

**テスト項目**
- [ ] ユニット：if/else を含む `BytecodeFunction` から IR を生成して verify 成功。
- [ ] 回帰：同一レジスタへの複数回代入で use-before-def が発生しない（debug モードでは trap/エラーを挿入）。

---

### 4.2 核心命令サブセットの翻訳（「動く」を雰囲）

**目標**
- `codegen.rs`：少なくとも以下の `BytecodeInstr` を実装：
  - `LoadConst`（Int/Float/Bool/String は 우선限定：String は Error にデグレードまたはまず未サポート）
  - `Mov`
  - `BinaryOp`（Add/Sub/Mul/Div：Int と Float の各自パス）
  - `Compare`（Eq/Lt/Gt など）
  - `Jmp/JmpIf/JmpIfNot/Return/ReturnValue`
  - `CallNative`（`yx_rt_native_call` を通じて）
  - `CallStatic`（2 つの戦略：`@block` は直接 call；`@auto` は `yx_rt_lazy_call` を通じて Async を返す）
- 強制ルール：算術/比較/分岐に参加する全オペランドは事前に `yx_rt_force` が必要（「値が必要になった時点でトリガーする」透過的並行性）。

**受け入れ基準**
- [ ] AOT バックエンドが简单なプログラムを実行可能：
  - 純粋算術
  - if/else
  - `std.io.println` 呼び出しで出力
- [ ] サポートされていない命令は panic ではなく읽을 수 있는 오류 生成。

**テスト項目**
- [ ] 統合：`tests/integration/llvm_aot_smoke.rs` を追加（feature gate）、5 つのプログラム片段を実行して結果/出力をアサーション（出力は stdout リダイレクトで実現可能）。
- [ ] 負向：`MakeClosure/CallVirt/...` 遭遇時は明確な「未実装」エラーを返す。

---

## フェーズ 5：機械語コード生成物と実行（AOT クローズ）（3-6 日）

### 5.1 生成物フォーマット：Object + メタデータ（2ファイル、まず简单）

**目標**
- `CompiledArtifact`（Rust 側構造）は少なくとも以下を含む：
  - `object_bytes: Vec<u8>`（COFF/ELF/Mach-O）
  - `dag_meta: DAGMetadata`（最初は空で OK）
  - `entries: Vec<EntryPoint>`（少なくとも main）
  - `type_info: TypeInfo`（MVP はまず空）
- 出力戦略：
  - `yaoxiang build-aot input.yx -o out/` で `program.obj` + `program.dag.ron`（または `.json`）を生成
  - `yaoxiang run --backend llvm` はデフォルトで「メモリコンパイル+直接実行」（ディスク書き込みなし）、開発効率向上

**受け入れ基準**
- [ ] build-aot が2つのファイルを生成し、メタデータがデシリアライズ可能。
- [ ] run/llvm パスがディスクファイルに依存せずとも実行可能。

**テスト項目**
- [ ] ファイル生成テスト：`.obj` が非空、`.dag.ron` が解析可能かつバージョン番号が一致することを検証。
- [ ] 互換性テスト：異なる build_mode（Debug/Release）で異なる最適化レベルが出力される（少なくとも区別可能）。

---

### 5.2 実行方式：「メモリ実行」→「ディスク読み込み」（2段階接受）

**目標**
- Step A（ 먼저 交付）：LLVM ExecutionEngine（または ORC JIT）を使用して既に生成された module を実行（セマンティクスクローズを検証、开发効率最高）。
- Step B（AOT 準拠）：TargetMachine で object bytes を生成し、「動的ライブラリリンク/ロード」パスで実行：
  - object を `.dll/.so/.dylib` にリンク（システムリンカまたは lld を呼び出す；`llvm-aot` feature の追加要件）
  - `libloading` でシンボルをロードしてエントリ関数を呼び出す

**受け入れ基準**
- [ ] Step A：`--backend llvm` が同一プロセス内で実行可能（外部リンカに依存しない）。
- [ ] Step B：`build-aot` が生成した生成物を `run-aot`（新しいサブコマンドまたは内部パス）がロードして実行可能。

**テスト項目**
- [ ] Step A：ユニット/統合テストはデフォルトで実行（開発が速い）。
- [ ] Step B：「外部リンカ必要」のオプション統合テストとしてマーク（CI に環境がある場合は有効；ローカルでは手動）。

---

## フェーズ 6：DAG メタデータ生成（4-7 日）

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
  - DataEdge + DataEdge：並列可能（他の依存がない場合）
  - 任意の ControlEdge を含む組み合わせ：シリアル化が必要（順序保持）
- `DAGNode` は少なくとも以下を含む：
  - `node_id: u32`（関数内唯一）
  - `ip: u32`（call 命令位置）
  - `strategy: ScheduleStrategy { Serial, Eager, Lazy }`（アノテーションまたはデフォルト戦略から）
  - `span_id: u32`（ローカライズとエラーグラフ用）
  - `thread_safety: ThreadSafety { SendSync, LocalOnly }`（型システムから導出；`LocalOnly` ノードはクロススレッドスケジューリング禁止）
- 約束：ノードは「スケジューリング可能な呼び出し点」のみを記述、パラメータはランタイム `yx_rt_lazy_call` 時にキャプチャ（静的式評価の複雑さを避ける）。

**受け入れ基準**
- [ ] `DAGMetadata` がシリアライズ/デシリアライズ可能（既存の `ron` または `serde_json` を使用）。
- [ ] `dag_version` が一致しない場合、ローディングがエラー。

**テスト項目**
- [ ] シリアライズ roundtrip 單測。
- [ ] バージョン不一致單測（手動で旧バージョンを作成）。
- [ ] `thread_safety` 導出テスト：少なくとも 1 つの `LocalOnly` シナリオをカバーし、`num_workers>1` でクロススレッド実行されないことを検証。

---

### 6.2 リソース型と ControlEdge 生成（副作用抽象の最小使用可能）

**目標**
> **更新**：RFC-001 に従い、副作用は追加の effect システムで表現せず、**リソース型（Resource）**で抽象化してリソース操作を表現し、ControlEdge を生成。

- リソース操作認識（MVP）：
  - 呼び出しのいずれかの引数型が `Resource` またはその派生リソース型（例：`Console/FilePath/HttpUrl/DBUrl`）の場合、その呼び出し点はリソース操作；
  - 標準ライブラリのリソース操作関数は認識可能な型制約を持つ必要がある（推奨做法：std エクスポートシグネチャにリソース型を明示的に携带；または FFI registry のエクスポートメタデータで「リソース操作」をマークしてリソース引数位置を関連付け）。
- ControlEdge 生成（MVP）：
  - **同一リソース値/ハンドル（同一 SSA 値または同一定数 житキー）**への複数リソース操作を、プログラム順序に従って ControlEdge を追加（自動シリアル）。
  - 同一リソースかどうか判定不能（エイリアス/複雑なソース）の場合、デフォルトで保守的にシリアル化（将来的に明示的な unsafe 並行ヒントを拡張として導入可能）。
  - **リソース識別子はデータフローに沿って伝播（RFC-001）**：リソース競合検出は「値の等価/同一ソース」を基準とし、「リソース型が同じ」を基準としない（2つの異なる `FilePath` 値は並列可能；同一 `FilePath` 値はシリアル必要）。

**受け入れ基準**
- [ ] 例：`log → save → log` は Console/FilePath リソースにより ControlEdge を 形成、稳定的にシリアル；異なるリソース操作は並列可能。
- [ ] リソース操作認識が安定（同一入力 module で複数回の結果が一致）。

**テスト項目**
- [ ] ユニット：リソース型パラメータ認識テスト（Resource パラメータが存在する場合、ControlEdge が生成される必要がある）。
- [ ] ユニット：同一リソース値（同一変数/同一定数）での2回リソース操作は ControlEdge を生成する必要がある；異なるリソース値（異なる変数/異なる定数）では生成しなくてよい。
- [ ] 統合：複数の `std.io.println/std.io.write_file` を含むサンプルを実行し、出力/書き込み順序がインタープリタと一致することをアサーション。

---

### 6.3 L1 自動フォールバック（小関数を @block にデグレード、スケジューリングオーバーヘッド回避）

> **来源**：RFC-001 5.2（L1 自動フォールバック）。  
> **目的**：意味論を変えない条件で、小さな関数の DAG/スケジューラオーバーヘッドを削減（特にインタープリタバックエンドと AOT バックエンドの統一動作）。

**目標**
- コンパイル時に関数に対して軽量な閾値判定を実行し、いずれかの条件を満たした場合、その関数（またはその関数内の特定のブロック）のデフォルト戦略を `Serial` にデグレード：
  - 命令数 `< 50`
  - DAG ノード数 `< 10`
- CLI/設定でスイッチを露出（MVP：内部設定のみでも可）：
  - `--l1-threshold=<N>` 閾値を調整
  - `--no-l1-fallback` 自動フォールバックを無効化

**受け入れ基準**
- [ ] 小関数が `@auto` 下で実際の実行時に DAG/スケジューリングキューに入らない（統計フィールドまたはトレースで検証可能）、結果はデグレード前と一致。
- [ ] 大関数には影響しない；強制アノテーション `@eager/@block` は自動フォールバックより優先。

**テスト項目**
- [ ] ユニット：境界値（49/50 命令、9/10 ノード）を構築してデグレードがトリガーされるかどうかを検証。
- [ ] 回帰：同一ソースでフォールバック on/off の両方で、出力と戻り値が一貫。

---

## フェーズ 7：ランタイム DAG スケジューラ（遅延スケジューリング核心）（6-10 日）

### 7.1 タスクモデルの実装（`RtValue::Async` との对接）

**目標**
- `scheduler.rs`（または共有のために `src/backends/runtime/` に移行して「インタープリタ/LLVM 共有」を実現）に以下を実装：
  - `TaskId` 割り当て
  - `TaskState { Pending, Running, Completed(RtValue), Failed(RtValue) }`
  - `spawn(task_fn_ptr, args, deps, node_id, span_id)`：タスクを作成するが延迟起動可能（エラー伝播/エラーグラフ用）
  - `force(task_id)`：依存トポロジに従って実行をトリガーして結果を待つ

**受け入れ基準**
- [ ] `yx_rt_lazy_call` が `Async(TaskId)` を返し、タスクが記録される（即時実行しない）。
- [ ] `yx_rt_force` がタスク実行をトリガーして結果を返す（依存チェーンを含む）。

**テスト項目**
- [ ] 純粋 Rust：モック「コンパイル済み fn」ポインタ（実際の Rust `extern "C"` 関数）を使用して3ノード DAG を構築し、依存順序と結果の正しさを検証。
- [ ] エラー伝播：下流の force が Error を得て、死鎖しない。

---

### 7.2 スケジューリング戦略の実装（Serial/Eager/Lazy）

**目標**
- `Serial`（`@block` に対応）：Async を作成しない；call を即時実行；スケジューラインターフェースをバイパス可能。
- `Eager`：タスクを作成するが即座に `force`（順序を保証）、デバッグ/意味論整合用。
- `Lazy`（デフォルト `@auto`）：値が必要な時にのみ `force`；スケジューラはバックグラウンドウィンドウ内で「準備完了」タスクを早期起動可能（並行数制限に従う）。
- 下から上へ（RFC-001/008）：ランタイム動作は「結果が必要な場所から逆算して評価をトリガーする」特性を体现する必要がある；**消費されず、リソース操作を含まない（ControlEdge なし）分支/孤立 DAG は実行しない**、オーバーヘッドを削減；リソース操作は ControlEdge に従って順序と完了を保証（インタープリタと一致）。
- バックグラウンド DAG（RFC-018）：同一スコープに複数の長時間実行/無限ループタスクが存在する場合、スケジューラは**協調的スライシング**を提供する必要がある（例：budget ベースまたは明示的な `yield_now`）、メイン DAG の飢餓と「ループ内でのスタック」を回避。

**受け入れ基準**
- [ ] 同一プログラムが Serial/Eager/Lazy の3つの戦略で結果が一貫。
- [ ] Lazy 下では、ある call の結果が force/使用されない場合、タスクは実行しない（Lazy Task Creation）。

**テスト項目**
- [ ] 比較テスト：3つの戦略の出力の一致。
- [ ] Lazy スキップテスト：「計算するが 사용하지 않는」分支/変数を記述し、対応する task の実行カウントが 0 であることをアサーション（スケジューラ統計フィールド）。
- [ ] バックグラウンドスライシングテスト：2つの長時間実行タスク + 1つのメインタスクを構築し、時間帯内で3つ全てが進捗があることをアサーション（カウンタまたは `thread_id` + ログ統計を使用可能）。

---

### 7.3 並行制御と粒度制御

**目標**
- 並行上限：`max_parallelism = num_workers * 2`（RFC-018 推奨）。
- リソース制約：同一リソースへの操作は ControlEdge に従ってシリアル実行が必要（RFC-001 リソース型ルール）、スケジューラは ControlEdge 順序を乱さない。
- スレッド安全性制約（RFC-001/009）：スケジューラは `DAGNode.thread_safety` を尊重する必要がある：
  - `SendSync`：worker 間を跨いで実行可能（並行上限と依存制約に従う）
  - `LocalOnly`：スレッド間スケジューリング禁止/work-stealing による窃み取り禁止；必要に応じてシリアルにデグレード（または作成した worker 上でのみ実行）
- アダプティブ粒度（MVP）：実行待ち task 数が並行上限よりはるかに多い場合、「準備完了かつ**ControlEdge 制約なし**の複数のタスク」をマージして批量提交（同一 worker が一批をシリアル実行として実装、スケジューリングオーバーヘッドを削減）。

**受け入れ基準**
- [ ] 大量独立性/リソース制約なしタスク（1e4）がメモリ爆発を引き起こさない（タスク構造は O(並行数) または制御可能な上限）。
- [ ] `LocalOnly` ノードが `num_workers>1` でスレッド間を跨いで実行されない（`std.concurrent.thread_id` で検証可能）。
- [ ] リソース操作（例：`std.io.*`）の出力/副作用順序は厳密にインタープリタ順序を維持。

**テスト項目**
- [ ] 压測ユニット：10000 個のモックタスクを構築し、ピークメモリ/タスク数が制御されていることを統計アサーションで検証（精密なメモリ測定は不要）。
- [ ] LocalOnly 統合テスト：`LocalOnly` ノードを含むサンプルを構築し、`num_workers>1` で実行スレッド ID が変化しないことをアサーション。
- [ ] リソース順序統合テスト：複数リソース操作（println/write_file）は必ずソースコード順序で出力/ディスク書き込み。

---

### 7.4 エラー伝播とエラーグラフ記録（RFC-001 最小クローズ）

**目標**
- 最小 `ErrorGraph` データ構造を定義（デバッグ/トレース用のみまず）：
  - ノード：`node_id + span_id + message/error_code`
  - エッジ：`from_node_id -> to_node_id`（「エラーが依存ノードから消費者ノードに伝播した」ことを表す）
- 記録戦略（RFC-001 決議）：
  - エラーは依存エッジに沿って上游に伝播し、**実際の実行順序に依存しない**；
  - DAG は関数内でのみ構築されるため、エラーグラフも関数级别に制限され、メモリ爆発を回避。
- ABI との整合：
  - `yx_rt_lazy_call/yx_rt_native_call` は `node_id/span_id` を携带する必要がある（フェーズ 2.3 で既にロック済み）
  - タスク失敗と `force` がエラーを返す時、`ErrorGraph` に書き込む（`ctx.error_graph != null` の場合）

**受け入れ基準**
- [ ] 依存チェーンの底部ノードが失敗した時、顶层消費者点がエラーを受け取り（失敗ノードの span までローカライズ可能）。
- [ ] 並行実行下でも、エラー伝播パスが安定して再現可能（スケジューリング順序に依存しない）。

**テスト項目**
- [ ] ユニット：3ノード依存チェーンを構築し、中央ノードが失敗するシナリオをシミュレートし、ErrorGraph エッジが `leaf->mid->root` であることをアサーション。
- [ ] 並行回帰：num_workers>1 で複数回実行しても、ErrorGraph 構造が一致。

---

## フェーズ 8：構文アノテーション統合（@block/@eager/@auto）（5-8 日）

### 8.1 フロントエンドがアノテーションをサポートしてバイト码/メタデータに伝達

**目標**
- 解析層：関数/ブロックアノテーション `@block`、`@eager` を認識；デフォルト `@auto`。
- 解析/型チェック：`spawn` は `@block` スコープ内でのみ出現可能を強制（RFC-001/008）。
- IR/Bytecode：`BytecodeFunction` または追加の side-table でデフォルト戦略を携带；call-site で lazy/eager/direct を選択可能。

**受け入れ基準**
- [ ] アノテーションなし：デフォルト Lazy（@auto）。
- [ ] `@block`： 해당 スコープ内では Async を作成せず、動作はインタープリタのシリアルと一致。
- [ ] `@eager`：タスクを作成してから即座に force（結果が一貫でデバッグ容易）。

**テスト項目**
- [ ] フロントエンド：アノテーションを含むパース/IR 生成テスト（AST/IR アサーション）。
- [ ] バックエンド：同一ソースコードに別々にアノテーションを付けて実行し、動作が戦略に準拠することを検証。

---

### 8.2 標準ライブラリ：`@block` と `spawn` のランタイムインターフェース（Full Runtime）

> **来源**：RFC-008（@block は標準ライブラリが提供する）、RFC-001（spawn の待機意味論は標準ライブラリが制御）。

**目標**
- 標準ライブラリランタイムモジュールを追加（推奨パス：`std.runtime` または `std.full`）、以下を提供：
  - `block`：先行評価を強制（スコープ戦略を `Serial` /DAG キュー不入に設定するのと同等）
  - `spawn`/`join_all`（または暗黙的 join）：`@block` スコープ内で並行タスクを作成して完了を待つ
- コンパイラはまず組み込み実装で MVP を实现してもいが、標準ライブラリへの移管可能な抽象化が必要（将来の リファクタリングコストを避ける）。

**受け入れ基準**
- [ ] `@block` 関数内の `spawn { ... }` ブロックが並行実行可能で、スコープ終了前に完了する（「サイレントバックグラウンドリークタスク」なし）。
- [ ] `@block` の動作が L3 デフォルト並行動作と明確に区別可能（例如：是否进入 DAG キュー、是否产生 Async 值）。

**テスト項目**
- [ ] 統合：2つの `spawn { std.concurrent.sleep(50) }` のサンプルが複数 worker 下で単一 sleep 時間に近い時間で完了（粗粒度並行検証）。
- [ ] 負向：@block 外での spawn 使用でエラー（0.3/8.1 と一致）。

## フェーズ 9：エンドツーエンドとパフォーマンスベンチマーク（継続推進）

### 9.1 インタープリタとの一貫性テスト（意味論整合）

**目標**
- 「命令サブセットをカバーできる」一组のテストケースを選択：算術、分岐、関数呼び出し、native IO。
- 同一ソースコードをインタープリタと llvm バックエンドで分别実行し、以下を比較：
  - 戻り値（もしあれば）
  - stdout 出力（注入/リダイレクトが必要）
  - エラータイプ（尽量整合 `ExecutorError`）

**受け入れ基準**
- [ ] テストケースセットで、LLVM バックエンドとインタープリタの結果が一致。

**テスト項目**
- [ ] `tests/integration/llvm_vs_interpreter.rs`（feature gate）至少 10 個のテストケース。
- [ ] 回帰：新しいテストケースを追加する場合、両バックエンドで実行する必要がある。

---

### 9.2 ベンチマーク：インタープリタ vs AOT（粗粒度）

**目標**
- `benches/` にベンチマークを追加：純粋計算（IOなし）、大量 call タスク（並行效益）、混合 IO（順序制約）。

**受け入れ基準**
- [ ] AOT が純粋計算テストケースでインタープリタより 著しく高速（具体的な倍数は約束しないが、明らかに遅くなってはならない）。
- [ ] Lazy スケジューリングのオーバーヘッドが観測・特定可能（スケジューラ stats 出力を通じて）。

**テスト項目**
- [ ] criterion ベンチマーク（手動/CI オプション）レポート生成、ベースラインを記録。

---

## 前提条件とデフォルト（ビジネス要件でカバーされていない場合の選択）

- デフォルト LLVM 主バージョンは **17** を選択；チームツールチェーンが異なる場合は、`inkwell` feature とドキュメントを一括修正すればよい。
- AOT 実行パスは「二段階アプローチ」を採用：まずはメモリ実行（開発検証）、次はディスク書き込みリンク/ロード（本当の AOT）。
- 初期 `llvm-aot` は MVP 命令サブセットのみをカバーすることを約束；クロージャ/動的ディスパッチ/例外などの高度な機能は требования に応じて後で拡張（「未実装」エラーを明確に返す）。
- DAG 依存エッジは** 런타임時に args の Async TaskId から動的に導出可能**；コンパイル時の edges フィールドはオプションの最適化とデバッグ検証としてまず使用可能、M2 交付を阻塞しない。
  - **補足（RFC-001）**：ControlEdge の主要な来源はリソース型ルール；リソース情報が不足している場合、デフォルトで 正しく保证するため保守的にシリアル化。