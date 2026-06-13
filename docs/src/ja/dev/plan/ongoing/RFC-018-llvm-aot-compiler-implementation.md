# RFC-018 LLVM AOT コンパイラと L3 透明並列（DAG 遅延スケジューリング）実装計画

> **⚠️ アラインメント注記**：本文書は旧並列モデル（`@block`/`@eager`/`@auto` アノテーション、`Send`/`Sync` trait、L1/L2/L3 階層）に基づいており、[RFC-024 新並列モデル](/design/rfc/accepted/024-concurrency-model.md) により置き換えられています。本文書は RFC-024 とアラインメントが取れてから推進を継続する必要があります。現在の並列モデルは `spawn {}` ブロックを唯一の並列プリミティブとし、アノテーションや Send/Sync は存在しません。

> **タスク**：LLVM AOT バックエンド + ランタイム DAG スケジューラの実装（~~`@auto/@eager/@block` の 3 種のスケジューリング戦略の具現化~~ 廃止済み、RFC-024 とのアラインメントが必要）  
> **ベース RFC**：RFC-018（ドラフト）  
> **依存 RFC**：RFC-001（並作モデルとエラーハンドリング）、RFC-008（3 層ランタイム）、RFC-009（所有権/Arc）  
> **日付**：2026-03-10  
> **ステータス**：進行中  
> **目標マイルストーン**：  
> - M1：LLVM AOT（実行可能ファイルをコンパイル可能、シリアル）  
> - M2：DAG メタデータ + 単一スレッドスケジューリング（Standard Runtime、num_workers=1）  
> - M3：マルチスレッド並列スケジューリング + 粒度制御（Full Runtime、num_workers>1）  
> - M4：遅延スケジューリング（Lazy Task Creation）+ **Resource 型（副作用抽象）** + **エラー伝播/エラーグラフ** + アノテーション貫通

---

## 要約（実装の閉ループ）

- `yaoxiang` に LLVM バックエンド（feature gate）を新設し、`BytecodeModule` を機械語（COFF/ELF/Mach-O）にコンパイルしてランタイムでロード・実行可能にする。
- **安定 ABI** を導入：AOT 生成コードとランタイムは `extern "C"` の `RtValue/RtContext` でやり取りし、Rust enum ABI の不安定問題を回避する。
- RFC-018 の核となる **関数ブロック内 DAG** + **遅延スケジューリング（Lazy Scheduling）** を具現化する。並列/シリアルは **DAG 辺（Data/Control/Spawn）** と **Resource 型ルール** により共同で決定される。エラーは RFC-001 に従い依存辺に沿って伝播し、エラーグラフを形成し得る。

---

## 公開インターフェース/振る舞いの変更（外部可視）

1. **Cargo features**
   - `llvm-aot` feature を新設：LLVM/inkwell 依存と AOT バックエンドを有効化；デフォルトは無効（LLVM 環境なしでもビルド可能であることを保証）。
2. **CLI**
   - `yaoxiang run` に `--backend {interpreter,llvm}` を追加（デフォルトは interpreter）。
   - 任意：`--runtime {embedded,standard,full}` と `--workers <N>` を追加し、ランタイム階層と並列度を制御（RFC-008）：
     - `--runtime embedded`：即時実行（DAG なし、スケジューラ機能なし、組込み/極小シーン向け）
     - `--runtime standard`：DAG + Scheduler（num_workers=1 で非同期；>1 で並列）
     - `--runtime full`：standard + WorkStealer（高度機能、任意）
3. **ランタイム ABI（内部だがモジュール横断）**
   - `RtValue`（`#[repr(C)]`）と `RtContext`（ポインタ/基本型のみ）を新設し、AOT とランタイムの境界とする。

---

## 重要な設計制約（RFC-001 / RFC-008 / RFC-018 とのアラインメント）

### A. 並列セマンティクス（L1/L2/L3 は単なるメンタルモデル）

- **L3（デフォルト / @auto）**：透明並列；DAG を構築；呼び出しに遭遇したら「遅延評価可能」な値をまず返し、**値が必要になった時点で評価を起動**する。
- **L1（@block）**：標準ライブラリで提供（RFC-008）、セマンティクスは「強制的な先行評価」、DAG 遅延キューには入らない；主にデバッグと重要な逐次セグメント用。
- **L2（spawn）**：**`@block` スコープ内でのみ使用可能**（RFC-001/008）、同期コード中に並列を挿入する用途；Full Runtime の機能。

### B. ランタイム 3 層（RFC-008）

- **Embedded Runtime**：即時実行；DAG の構築を完全に省略する選択も可能（メモリ/起動時間を節約）；制約のある環境向け。
- **Standard Runtime**：DAG + Scheduler が中核（遅延評価により非同期/並列を自然にサポート）。
- **Full Runtime**：standard を基盤に WorkStealer と、標準ライブラリ層の `@block` / `spawn` などの機能を追加。

### C. DAG 構築範囲とメモリ（RFC-001/018）

- DAG は**単一の関数本体/ブロック内**でのみ構築；呼び出された関数本体を再帰的に展開しない（エラーグラフと DAG ノードの爆発を避ける）。
- DAG メタデータは **ノード/辺の安定 ID** と **Span**（エラー伝播とエラーグラフ位置特定用）を必ず携带する。

### D. 副作用抽象（RFC-001：Resource 型）

- 追加の「明示的副作用注釈システム」は導入しない；副作用は一律に **Resource 操作** として扱う：
  - 引数型に `Resource`（またはその派生 Resource 型）を含む関数呼び出しは Resource 操作とみなす；
  - 同一 Resource への操作は自動的に **ControlEdge** を形成（シリアル化）；異なる Resource 間は並列可能；
  - 静的に同一 Resource かどうか判定できない場合は、保守的にデフォルトでシリアル化（将来的に明示的な unsafe 並列ヒントを拡張として導入可能）。

### E. エラー伝播（RFC-001）

- エラーは DAG の依存辺に沿って上流へ伝播し（実際の並列実行順序に依存しない）、エラーグラフ用に伝播経路を記録する。

---

## フェーズ 0：前提と制約の固定（1-2 日）

### 0.1 LLVM/inkwell バージョンとビルド方式の固定

**目標**
- LLVM メジャーバージョン = **17** を選定（チーム環境を統一；Windows/Linux/macOS いずれも対応バイナリパッケージが取得可能）。
- `Cargo.toml` に `inkwell`（`llvm17-0` 対応 feature を有効化）を追加し、`llvm-aot` feature 配下にぶら下げる。

**受入基準**
- [ ] `cargo build`（feature なし）が通る（LLVM 環境なしでもビルド可能）。
- [ ] `cargo build -F llvm-aot` が LLVM17 環境設定済みの場合に通る。

**テスト項目**
- [ ] CI/ローカル：2 組のビルドマトリクス（`default` と `-F llvm-aot`）で少なくとも 1 プラットフォームが通る。
- [ ] 最小スモーク：`cargo test -F llvm-aot` が起動し、空テストモジュールを実行できる（リンク検証のみ）。

---

### 0.2 LLVM 環境検出とエラーメッセージ

**目標**
- ビルド時/ランタイム検出の説明を追加：`llvm-config`/LLVM ディレクトリが欠落している場合に操作可能なエラーメッセージ（インストール方法/プレフィックス変数の設定方法）を提示する。

**受入基準**
- [ ] LLVM 欠落時、エラーメッセージに以下が含まれる：想定バージョン（17）、利用可能な環境変数（例：`LLVM_SYS_170_PREFIX` または `LLVM_CONFIG_PATH`）とサンプルパス。

**テスト項目**
- [ ] LLVM 環境なしの端末で `cargo build -F llvm-aot` を実行し、メッセージが完全に出力され panic しない（コンパイル時エラーでよい）。

---

### 0.3 並作モデル実装制約の固定（RFC-001/008 とのアラインメント）

**目標**
- 以下の実装制約を明確化し固定する（コードコメント/開発ドキュメントとテストケースに記載）：
  - `spawn` は `@block` スコープ内でのみ許可（解析/型検査/IR 段階すべてで防御）。
  - `@block` のセマンティクスは「先行評価」、標準ライブラリの機能で提供（コンパイラ内蔵 MVP で先行実装可だが、将来的に標準ライブラリへ降ろせるインターフェースを残しておく）。
  - DAG は関数ブロック内のみ構築；安定な `node_id` と `span` を必ず携带（エラー伝播/エラーグラフを支える）。
  - Resource 型が ControlEdge 生成を駆動し、ユーザー可視の effect 注釈体系を追加導入しない。
  - **並列安全性制約（RFC-001/009）**：ノードのキャプチャ/戻り値が `Send + Sync`（または言語側等価制約）を満たす場合のみスレッド横断並列を許可；満たさない場合はシリアル（または単一 worker 固定実行）へ降格しなければならない。

**受入基準**
- [ ] 不正な `spawn` の場面でコンパイラが明確なエラー（Span 付き）を出力する。
- [ ] `@block/@eager/@auto` のセマンティクス差異が最小例で観測可能かつテスト可能。
- [ ] 本計画ドキュメントと RFC-001/008/018 の重要な決定が一致しており、自己矛盾する項目がない。

**テスト項目**
- [ ] `spawn` 位置制限テスト：@block 外で spawn が出現したら必ずエラー。
- [ ] DAG scope テスト：DAG が関数本体を跨いで展開しないこと（ノード数が呼び出し階層と無関係）を確認。
- [ ] Send/Sync 制約テスト：
  - `spawn` が非 `Send` 値をキャプチャしたら必ずエラー（Span 付き）。
  - `@auto` 下で非 `Send + Sync` 値を含むノードはスレッド横断スケジューリングされない（`std.concurrent.thread_id` 統計で検証可能）。

---

## フェーズ 1：LLVM バックエンドの骨格と選択スイッチ（1-2 日）

### 1.1 バックエンドモジュールの追加と RFC-018 ディレクトリ構造へのアラインメント

**目標**
- `src/backends/llvm/` を新設し、：`mod.rs / context.rs / types.rs / values.rs / codegen.rs / dag.rs / scheduler.rs / tests.rs` を含む（後続でマージ/分割可）。
- `src/backends/mod.rs` で `#[cfg(feature = "llvm-aot")] pub mod llvm;` によりモジュールを公開する。

**受入基準**
- [ ] `cargo test`（デフォルト feature）が通る。
- [ ] `cargo test -F llvm-aot` が通る（LLVM バックエンドがまだ完全実装でなくても可）。

**テスト項目**
- [ ] ユニット：`src/backends/llvm/tests.rs` に少なくとも 1 つのコンパイル時テストが実行できる（モジュールが参照可能であることの検証のみ）。

---

### 1.2 バックエンド選択：CLI/ライブラリ側注入点

**目標**
- CLI `Run` サブコマンドに `--backend` パラメータ（ValueEnum）を新設：`interpreter`（デフォルト）/ `llvm`（feature 必要）。
- `yaoxiang::run_*` パスにバックエンド選択分岐を追加し、`fn make_executor(kind, config) -> Box<dyn Executor>`（または trait object 回避のため enum ディスパッチ）に抽象化する。

**受入基準**
- [ ] `yaoxiang run file.yx` は引き続きインタープリタを通り、振る舞いが変わらない。
- [ ] `yaoxiang run --backend llvm file.yx`：feature が未有効なら明確なエラー；有効なら LLVM 実行パスに入る（一時的に "not implemented" を返すとしても、制御されたエラーでなければならない）。

**テスト項目**
- [ ] CLI パラメータ解析テスト（`tests/integration` に追加）。
- [ ] ネガティブテスト：feature 無効時に `--backend llvm` を渡すと可読なエラーメッセージを返す。

---

## フェーズ 2：安定 ABI（RtValue/RtContext）と Runtime API（3-5 日）

> このフェーズは「LLVM 生成コードが実行可能」になるための鍵：まず境界を跨ぐ値の表現を安定させなければならない。

### 2.1 `RtValue`（安定 C ABI）の定義

**目標**
- `#[repr(C)] struct RtValue { tag: u8, _pad: [u8; 7], payload: u64 }` を定義（または 16 バイト構造でアラインメントを簡単に保つ）。
- `#[repr(u8)] enum RtTag { Unit, Bool, Int, Float, Handle, Async, Error }` を定義（最小集合；後続拡張）。
- 規約：
  - Int：`payload` = `i64` bits
  - Float：`payload` = `f64` bits
  - Bool：0/1
  - Handle：`payload` = `usize`（u64 に拡張）
  - Async：`payload` = `TaskId`（u64）
  - Error：`payload` = エラーコードまたはポインタ（MVP はエラーコードで先行可）

**受入基準**
- [ ] `RtValue` が Rust 内部で安全に構築/読み出し可能（UB なし）で、`Debug` 出力と基本的なアサーション用ユーティリティを持つ。
- [ ] LLVM IR と整合：inkwell で同レイアウトの struct type を生成可能（フィールド順序/サイズ一致）。

**テスト項目**
- [ ] `RtValue` ラウンドトリップ：int/float/bool/unit のエンコード/デコードのユニットテスト。
- [ ] ABI サイズテスト：`size_of::<RtValue>()` と `align_of::<RtValue>()` を固定値アサート（将来の誤変更防止）。

---

### 2.2 `RtContext`（ランタイムコンテキスト）の定義

**目標**
- `#[repr(C)] struct RtContext` を定義、ポインタ/整数のみを含む：
  - `heap: *mut Heap`
  - `ffi: *const FfiRegistry`
  - `scheduler: *mut DynScheduler`（または具体実装へのポインタ）
  - `max_parallelism: usize`
  - `error_graph: *mut ErrorGraph`（任意：RFC-001 のエラー伝播記録用；MVP は null 可）
  - 予備フィールド（バージョン番号/flags）を確保するが最小化（KISS）。

**受入基準**
- [ ] `RtContext` をインタープリタ/LLVM executor から構築し、生成コードへ渡せる。
- [ ] Rust 非安定レイアウトのフィールドを含まない（`Heap`/`FfiRegistry` の値を直接埋め込まない）。

**テスト項目**
- [ ] `RtContext` の構築/破棄に関するメモリ安全性テスト（実 LLVM 不要）。

---

### 2.3 Runtime C API：生成コードが呼び出す最小関数セット

**目標**
- `#[no_mangle] extern "C"` 関数（命名は統一プレフィックス `yx_rt_*`）を提供、MVP の最小構成：
  - `yx_rt_force(ctx: *mut RtContext, v: RtValue) -> RtValue`
  - `yx_rt_lazy_call(ctx, callee: *const u8 /*関数ポインタ*/, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_native_call(ctx, name_ptr: *const u8, name_len: usize, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_panic(code: u32)` または `yx_rt_trap(msg_ptr, len)`（デバッグ用）
- 制約：AOT 生成コードは**上記 API 経由でのみやり取り**し、Rust 構造体を直接操作しない。

**受入基準**
- [ ] ランタイム API は LLVM なしでもコンパイル可能（`llvm-aot` feature で制御：API は常時常駐または feature 下でのみ提供、いずれにせよテスト可能）。
- [ ] `yx_rt_native_call` が `FfiRegistry` の handler を呼び出せる（MVP は Int/Float/Bool/Unit 引数と戻り値のみサポート；非対応の場合は Error RtValue を返す）、失敗時に `node_id/span_id` を（有効なら）エラーログへ記録する。

**テスト項目**
- [ ] 純粋 Rust ユニットテスト：`yx_rt_native_call` を直接呼び出し、`std.io.println`（または自己登録関数）パスが動作することを検証。
- [ ] エラーパステスト：存在しないネイティブ名を渡し、`Error` RtValue を返し `ExecutorError::FunctionNotFound` に変換可能であることを確認。

---

## フェーズ 3：LLVM Codegen インフラ（2-3 日）

### 3.1 LLVM コンテキスト/モジュール/TargetMachine 初期化

**目標**
- `context.rs`：inkwell の `Context/Module/Builder` のライフサイクルをカプセル化。
- Target 初期化：`PlatformDetector`（`LLVM_TARGET` サポート）とホスト triple に応じて target triple + data layout を設定。
- 以下出力をサポート：
  - LLVM IR（`.ll`）デバッグ用
  - オブジェクト（`.o/.obj`）AOT 用

**受入基準**
- [ ] 任意の空 `BytecodeModule` に対し、`main` を含む LLVM Module を生成可能（関数本体が Unit を返すだけでも可）。
- [ ] IR を verify 可能（LLVM verify を呼び出す；失敗時に可読エラーを返す）。

**テスト項目**
- [ ] ユニット：最小 module を生成し verify 通過。
- [ ] スナップショットテスト（任意）：`.ll` のキー断片に対して文字列含有アサート（脆い全量スナップショットは避ける）。

---

### 3.2 `TypeMap`：YaoXiang Type → LLVM Type（MVP）

**目標**
- `types.rs`：`fn llvm_type(yao_type: &Type) -> BasicTypeEnum` を実装、まず以下をカバー：
  - `Int` → i64
  - `Float` → f64
  - `Bool` → i1
  - `()`/Void → void（または `RtValue(Unit)` で統一返却）
- 戦略選択（ABI 表面積を縮めるため）：**すべての関数は統一して `RtValue` を返す**（型別には返さない）こととし、codegen とスケジューラ/FFI が統一的に処理；型情報は静的検査と `RtValue` 構築/分解ロジック生成に使用。

**受入基準**
- [ ] `TypeMap` が上記型マッピングで安定し、LLVM IR 中の関数シグネチャが一貫：`fn(*mut RtContext, *const RtValue, usize) -> RtValue`。

**テスト項目**
- [ ] `TypeMap` 単測：`Type::Int/Float/Bool/Void` を与え LLVM 型生成に成功。
- [ ] 生成された関数シグネチャを LLVM module から検索し、仮引数/戻り型が一致することをアサート。

---

## フェーズ 4：命令翻訳 MVP（5-8 日）

### 4.1 レジスタから LLVM 値へのマッピング（SSA 化の最小実装）

**目標**
- `values.rs`：仮想レジスタ `Reg(u16)` → LLVM `Value` のマッピング表を実装（基本ブロックスコープで管理）。
- 規約：すべてのレジスタ値は `RtValue` で表現（型の爆発/ABI 不整合を回避）、演算/比較前に helper で強制/分解する。

**受入基準**
- [ ] 生成コードが制御フロー分岐後にレジスタ値を正しくマージできる（phi を使用するか、`RtValue` 層で統一型として扱う）。

**テスト項目**
- [ ] ユニット：if/else を含む `BytecodeFunction` に対し IR を生成し verify 通過。
- [ ] 回帰：同一レジスタへの複数代入で use-before-def が発生しないこと（debug モードで trap/エラーを挿入）。

---

### 4.2 コア命令サブセットの翻訳（「動かせる」をカバー）

**目標**
- `codegen.rs`：少なくとも以下の `BytecodeInstr` を実装：
  - `LoadConst`（Int/Float/Bool/String を先行限定：String は Error に降格または非サポート先行）
  - `Mov`
  - `BinaryOp`（Add/Sub/Mul/Div：Int と Float それぞれ）
  - `Compare`（Eq/Lt/Gt 等）
  - `Jmp/JmpIf/JmpIfNot/Return/ReturnValue`
  - `CallNative`（`yx_rt_native_call` 経由）
  - `CallStatic`（2 戦略：`@block` は直接 call；`@auto` は `yx_rt_lazy_call` 経由で Async を返す）
- 強制ルール：算術/比較/分岐に関わるオペランドは必ず先に `yx_rt_force` する（透明並列の「必要になった時点でトリガ」）。

**受入基準**
- [ ] AOT バックエンドが下記単純プログラムを実行可能：
  - 純粋算術
  - if/else
  - `std.io.println` 呼び出しでの出力
- [ ] 未サポート命令は panic ではなく可読エラーを生成。

**テスト項目**
- [ ] 統合：`tests/integration/llvm_aot_smoke.rs`（feature gate）を追加し、5 個のプログラム断片を実行し結果/出力をアサート（出力は stdout リダイレクトで取得）。
- [ ] ネガティブ：`MakeClosure/CallVirt/...` 遭遇時に明確な「未実装」エラーを返す。

---

## フェーズ 5：機械語生成物と実行（AOT 閉ループ）（3-6 日）

### 5.1 生成物形式：オブジェクト + メタデータ（2 ファイル、先は単純）

**目標**
- `CompiledArtifact`（Rust 側構造）の最小構成：
  - `object_bytes: Vec<u8>`（COFF/ELF/Mach-O）
  - `dag_meta: DAGMetadata`（先空でも可）
  - `entries: Vec<EntryPoint>`（少なくとも main）
  - `type_info: TypeInfo`（MVP は先空）
- 出力戦略：
  - `yaoxiang build-aot input.yx -o out/` で `program.obj` + `program.dag.ron`（または `.json`）を生成
  - `yaoxiang run --backend llvm` はデフォルトで「メモリコンパイル+直接実行」（落書きしない）、開発容易

**受入基準**
- [ ] build-aot が 2 ファイルを生成し、メタデータがデシリアライズ可能。
- [ ] run/llvm パスが落書きファイルに依存せず実行可能。

**テスト項目**
- [ ] ファイル生成テスト：`.obj` が非空、`.dag.ron` が解析可能でバージョン番号一致を確認。
- [ ] 互換性テスト：異なる build_mode（Debug/Release）で異なる最適化レベルを出力（少なくとも区別可能）。

---

### 5.2 実行方式：先に「メモリ実行」、次に「落書きロード」（2 ステップ受入）

**目標**
- Step A（先行提供）：LLVM ExecutionEngine（または ORC JIT）使用して生成済み module を実行（セマンティクス閉ループ検証用、開発効率最高）。
- Step B（AOT 適合）：TargetMachine で object bytes を生成し、「動的ライブラリリンク/ロード」パスで実行：
  - object を `.dll/.so/.dylib` にリンク（システムリンカまたは lld を呼び出し；`llvm-aot` feature の追加要件とする）
  - `libloading` でシンボルをロードしエントリ関数を呼び出す

**受入基準**
- [ ] Step A：`--backend llvm` が同一プロセス内で実行可能（外部リンカに非依存）。
- [ ] Step B：`build-aot` 生成物が `run-aot`（新設サブコマンドまたは内部パス）でロード・実行可能。

**テスト項目**
- [ ] Step A：ユニット/統合テストをデフォルト実行（開発高速）。
- [ ] Step B：「外部リンカ要」の任意統合テストとマーク（CI 環境あり時のみ有効；ローカル手動可）。

---

## フェーズ 6：DAG メタデータ生成（4-7 日）

### 6.1 `DAGMetadata` の定義（バージョン管理）

**目標**
- `dag.rs` で以下を定義：
  - `dag_version: u32`
  - `nodes: Vec<DAGNode>`（`node_id` と `span_id` を携带、エラー伝播用）
  - `edges: Vec<DAGEdge>`（辺種別：Data/Control/Spawn 付き）
- `DAGEdge` 最低構成：
  - `from: u32`
  - `to: u32`
  - `kind: EdgeKind { Data, Control, Spawn }`
- 衝突/スケジューリングルール（RFC-001）：
  - DataEdge + DataEdge：並列可（他依存がなければ）
  - ControlEdge を含む任意の組み合わせ：必ずシリアル化（順序保持）
- `DAGNode` 最低構成：
  - `node_id: u32`（関数内一意）
  - `ip: u32`（call 命令位置）
  - `strategy: ScheduleStrategy { Serial, Eager, Lazy }`（アノテーションまたはデフォルト戦略から導出）
  - `span_id: u32`（位置特定とエラーログ用）
  - `thread_safety: ThreadSafety { SendSync, LocalOnly }`（型システムで導出；`LocalOnly` ノードはスレッド横断スケジューリング禁止）
- 規約：ノードは「スケジューリング可能な呼び出し点」のみを記述し、引数はランタイムの `yx_rt_lazy_call` 時点でキャプチャ（静的式評価の複雑さを回避）。

**受入基準**
- [ ] `DAGMetadata` がシリアライズ/デシリアライズ可能（既存 `ron` または `serde_json` 使用）。
- [ ] `dag_version` 不一致時にロードエラー。

**テスト項目**
- [ ] シリアライズラウンドトリップ単測。
- [ ] バージョン不一致単測（旧バージョンを手作業構築）。
- [ ] `thread_safety` 導出テスト：少なくとも 1 つの `LocalOnly` シナリオをカバーし、`num_workers>1` 環境でスレッド横断実行されないことを検証。

---

### 6.2 Resource 型と ControlEdge 生成（副作用抽象の最小実用）

**目標**
> **更新**：RFC-001 に従い、副作用は追加 effect システムで表現せず、**Resource 型**により Resource 操作として抽象化し、ControlEdge を生成する。

- Resource 操作識別（MVP）：
  - 呼び出しのいずれか引数型が `Resource` またはその派生 Resource 型（例：`Console/FilePath/HttpUrl/DBUrl`）であれば、その呼び出し点は Resource 操作；
  - 標準ライブラリの Resource 操作関数は識別可能な型制約を持つこと（推奨：std エクスポートシグネチャに明示的に Resource 型を含める；または FFI registry のエクスポートメタデータに「Resource 操作」マークを付けて関連 Resource 引数位置と関連付ける）。
- ControlEdge 生成（MVP）：
  - **同一 Resource 値/ハンドル（同一 SSA 値/同一 constant interning key）** への複数 Resource 操作は、プログラム順に ControlEdge を追加（自動シリアル化）。
  - 同一 Resource かどうか判定不能（別名/複雑な由来）の場合、デフォルトで保守的にシリアル化（将来的に明示的な unsafe 並列ヒントを拡張として導入可能）。
  - **Resource 識別はデータ流に沿って伝播（RFC-001）**：Resource 衝突検出は「値同値/同一由来」を基準とし、「Resource 型が同じ」ではない（2 つの異なる `FilePath` 値は並列可；同一 `FilePath` 値は必ずシリアル）。

**受入基準**
- [ ] 例：`log → save → log` が Console/FilePath Resource により ControlEdge を形成し、安定してシリアル化；異なる Resource 操作は並列可能。
- [ ] Resource 操作識別が安定（同一入力 module に対し複数回結果一致）。

**テスト項目**
- [ ] ユニット：Resource 型引数識別テスト（Resource 引数が存在する場合必ず ControlEdge を生成）。
- [ ] ユニット：同一 Resource 値（同一変数/同一 constant）上の 2 回の Resource 操作は必ず ControlEdge を生成；異なる Resource 値（異なる変数/異なる constant）は生成不要。
- [ ] 統合：`std.io.println/std.io.write_file` を複数回含むサンプルを実行し、出力/書き込み順がインタープリタと一致することをアサート。

---

### 6.3 L1 自動フォールバック（小関数を @block に降格しスケジューリングオーバーヘッド回避）

> **出典**：RFC-001 5.2（L1 自動フォールバック）。  
> **目的**：セマンティクスを変えることなく、小関数の DAG/スケジューラオーバーヘッドを削減（特にインタープリタバックエンドと AOT バックエンドの挙動統一）。

**目標**
- コンパイル時に関数に対し軽量な閾値判定を行い、以下のいずれかを満たすなら当該関数（または当該関数内の一部のブロック）のデフォルト戦略を `Serial` に降格：
  - 命令数 `< 50`
  - DAG ノード数 `< 10`
- CLI/設定でスイッチを公開（MVP：内部設定のみでも可）：
  - `--l1-threshold=<N>` で閾値調整
  - `--no-l1-fallback` で自動フォールバック無効化

**受入基準**
- [ ] 小関数が `@auto` 下で DAG/スケジューラキューに入らない（統計フィールドまたはトレースで検証可能）、降格なしと結果が一致。
- [ ] 大関数は影響を受けない；強制アノテーション `@eager/@block` の優先度は自動フォールバックより高い。

**テスト項目**
- [ ] ユニット：境界値（49/50 命令、9/10 ノード）を構築し降格発生有無を検証。
- [ ] 回帰：同一ソースコードで降格有効/無効を切り替え、出力と戻り値が一致。

---

## フェーズ 7：ランタイム DAG スケジューラ（Lazy Scheduling コア）（6-10 日）

### 7.1 タスクモデル実装（`RtValue::Async` との接続）

**目標**
- `scheduler.rs`（または `src/backends/runtime/` へ移行し「インタープリタ/LLVM 共有」を実現）で実装：
  - `TaskId` 割当
  - `TaskState { Pending, Running, Completed(RtValue), Failed(RtValue) }`
  - `spawn(task_fn_ptr, args, deps, node_id, span_id)`：タスク作成するが起動は遅延可（エラー伝播/エラーグラフ用）
  - `force(task_id)`：依存トポロジに従い実行を起動し結果を待機

**受入基準**
- [ ] `yx_rt_lazy_call` が `Async(TaskId)` を返し、タスクが記録される（即時実行しない）。
- [ ] `yx_rt_force` がタスク実行を起動し結果（依存チェーン含む）を返せる。

**テスト項目**
- [ ] 純粋 Rust：モック「コンパイル済み fn」ポインタ（実体は Rust `extern "C"` 関数）で 3 ノード DAG を構築し、依存順序と結果の正当性を検証。
- [ ] エラー伝播：下流の force が Error を受け取り、デッドロックしない。

---

### 7.2 スケジューリング戦略の具現化（Serial/Eager/Lazy）

**目標**
- `Serial`（`@block` に対応）：Async を作成しない；call を即時実行；スケジューラインターフェースをバイパス可。
- `Eager`：タスクを作成後ただちに `force`（順序保証）、デバッグ/セマンティクス整列用。
- `Lazy`（デフォルト `@auto`）：値が必要になった時点で `force`；スケジューラはバックグラウンドウィンドウ内で「準備完了」タスクを前倒し起動可（並列数制限に従う）。
- ボトムアップ（RFC-001/008）：ランタイム挙動は「結果を必要とする箇所から逆算して評価を起動する」特性を体现すること；**消費されず Resource 操作にも関与しない（ControlEdge がない）分岐/孤立 DAG は実行されない**ようにし、オーバーヘッドを削減；Resource 操作は ControlEdge により順序を保証し完了させる（インタープリタと一致）。
- バックグラウンド DAG（RFC-018）：同一スコープ内に複数の長時間実行/無限ループタスクが存在する場合、スケジューラは**協調スライス**（例：budget ベースまたは明示的 `yield_now`）を提供し、主 DAG の飢餓と「ループ内停止」を回避。

**受入基準**
- [ ] 同一プログラムが Serial/Eager/Lazy の 3 戦略下で結果一致。
- [ ] Lazy 下で call 結果が一度も force/使用されない場合、タスクが実行されない（Lazy Task Creation）。

**テスト項目**
- [ ] 比較テスト：3 戦略の出力一致。
- [ ] Lazy スキップテスト：「計算するが使用しない」分岐/変数を書き、対応するタスク実行カウントが 0 になる（スケジューラ統計フィールド使用）。
- [ ] バックグラウンドスライステスト：2 個の長時間実行タスク + 1 個の主タスクを構築し、タイムウィンドウ内で 3 者とも進捗すること（カウンタまたは `thread_id` + ログ統計使用）。

---

### 7.3 並列制御と粒度制御

**目標**
- 並列上限：`max_parallelism = num_workers * 2`（RFC-018 推奨）。
- Resource 制約：同一 Resource への操作は ControlEdge に従いシリアル実行（RFC-001 Resource 型ルール）、スケジューラは ControlEdge 順序を乱してはならない。
- スレッド安全性制約（RFC-001/009）：スケジューラは `DAGNode.thread_safety` を尊重しなければならない：
  - `SendSync`：worker を横断して実行可（並列上限と依存制約に従う）
  - `LocalOnly`：スレッド横断スケジューリング禁止/work-stealing による盗難禁止；必要ならシリアル降格（または作成元 worker 固定実行）
- 適応粒度（MVP）：待機中タスク数が並列上限を大きく上回る場合、「準備完了かつ**ControlEdge 制約なし**の複数タスク」をまとめてバッチ投入（同一 worker 内で逐次実行するバッチとして実装、スケジューリングオーバーヘッド削減）。

**受入基準**
- [ ] 大量の独立かつ Resource 制約なしタスク（1e4）でメモリ爆発しない（タスク構造が O(並列数) または制御可能上界）。
- [ ] `LocalOnly` ノードが `num_workers>1` 環境でスレッド横断実行されない（`std.concurrent.thread_id` で検証可能）。
- [ ] Resource 操作（例：`std.io.*`）の出力/副作用順序がインタープリタ順序を厳格に保持。

**テスト項目**
- [ ] ストレステスト：10000 個のモックタスクを構築し、ピークメモリ/タスク数が制御下（統計アサート、厳密メモリ測定は不要でも可）。
- [ ] LocalOnly 統合テスト：`LocalOnly` ノードを含むサンプルを構築し、`num_workers>1` 環境で実行スレッド ID が変わらないことをアサート。
- [ ] Resource 順序統合テスト：複数 Resource 操作（println/write_file）がソースコード順に出力/書き出し。

---

### 7.4 エラー伝播とエラーログ記録（RFC-001 最小閉ループ）

**目標**
- 最小 `ErrorGraph` データ構造を定義（まずはデバッグ/トレース用途）：
  - ノード：`node_id + span_id + message/error_code`
  - 辺：`from_node_id -> to_node_id`（「エラーが依存ノードからコンシューマノードへ伝播」を表現）
- 記録戦略（RFC-001 決定）：
  - エラーは依存辺に沿って上流へ伝播し、**実際の実行順序に依存しない**；
  - DAG は関数内でのみ構築されるため、エラーログも関数レベルに限定しメモリ爆発を回避。
- ABI とのアラインメント：
  - `yx_rt_lazy_call/yx_rt_native_call` は必ず `node_id/span_id` を携带（フェーズ 2.3 で固定済み）
  - タスク失敗と `force` 時のエラー返却で、`ErrorGraph` へ書き込み（`ctx.error_graph != null` の場合）

**受入基準**
- [ ] 依存チェーン末端ノード失敗時、トップレベル消費箇所がエラーを受け取り（失敗ノードの span 位置特定可能）。
- [ ] 並列実行下で、エラー伝播経路が安定して再現可能（スケジューリング順序に非依存）。

**テスト項目**
- [ ] ユニット：3 ノード依存チェーンを構築、中間ノード失敗をシミュレート、ErrorGraph 辺が `leaf->mid->root` となることをアサート。
- [ ] 並列回帰：`num_workers>1` 環境で複数回実行し、ErrorGraph 構造が一致。

---

## フェーズ 8：構文アノテーション貫通（@block/@eager/@auto）（5-8 日）

### 8.1 フロントエンドがアノテーションをサポートしバイトコード/メタデータへ伝搬

**目標**
- 解析層：関数/ブロックアノテーション `@block`、`@eager` を識別；デフォルトは `@auto`。
- 解析/型検査：`spawn` は `@block` スコープ内でのみ許可を強制（RFC-001/008）。
- IR/Bytecode：`BytecodeFunction` または追加サイドテーブルにデフォルト戦略を携带；call-site で lazy/eager/direct を判定可能。

**受入基準**
- [ ] アノテーションなし：デフォルト Lazy（@auto）。
- [ ] `@block`：当該スコープ内で Async を作成せず、挙動がインタープリタのシリアル実行と一致。
- [ ] `@eager`：タスク作成後ただちに force（結果一致、デバッグ容易）。

**テスト項目**
- [ ] フロントエンド：アノテーションを含むテストを解析/IR 生成（AST/IR アサート）。
- [ ] バックエンド：同一ソースコードにそれぞれアノテーションを付与し、実行挙動が戦略に適合。

---

### 8.2 標準ライブラリ：`@block` と `spawn` のランタイムインターフェース（Full Runtime）

> **出典**：RFC-008（@block は標準ライブラリ提供）、RFC-001（spawn の待機セマンティクスは標準ライブラリ制御）。

**目標**
- 標準ライブラリのランタイムモジュール（推奨パス：`std.runtime` または `std.full`）を追加し、以下を提供：
  - `block`：強制先行評価（スコープ戦略を `Serial` 設定/DAG キューに入れないことと等価）
  - `spawn`/`join_all`（または暗黙 join）：`@block` スコープ内で並列タスクを作成し完了を待機
- コンパイラは MVP を内蔵先行実装可だが、必ず標準ライブラリへ降ろせるインターフェースを抽象化（将来のリファクタコスト回避）。

**受入基準**
- [ ] `@block` 関数内の `spawn { ... }` ブロックが並列実行され、スコープ終了前に完了する（「バックグラウンドタスクの静かなリーク」なし）。
- [ ] `@block` の挙動が L3 デフォルト並列挙動と明確に区別可能（例：DAG キューに入るか、Async 値を生成するか）。

**テスト項目**
- [ ] 統合：2 個の `spawn { std.concurrent.sleep(50) }` サンプルが複数 worker 下で単一 sleep 程度の所要時間（粗粒度並列検証）。
- [ ] ネガティブ：@block 外での spawn 使用はエラー（0.3/8.1 と整合）。

## フェーズ 9：エンドツーエンドと性能ベンチマーク（継続推進）

### 9.1 インタープリタとの一貫性テスト（セマンティクス整合）

**目標**
- 「命令サブセットをカバー」する用例セットを選定：算術、分岐、関数呼び出し、native IO。
- 同一ソースコードをそれぞれ interpreter と llvm backend で実行し、以下を比較：
  - 戻り値（あれば）
  - stdout 出力（注入/リダイレクト必要）
  - エラー種別（`ExecutorError` への整列を志向）

**受入基準**
- [ ] 用例セットにおいて、LLVM backend とインタープリタの結果が一致。

**テスト項目**
- [ ] `tests/integration/llvm_vs_interpreter.rs`（feature gate）で少なくとも 10 用例。
- [ ] 回帰：新規用例追加時は必ず両バックエンドを実行。

---

### 9.2 ベンチマーク：インタープリタ vs AOT（粗粒度）

**目標**
- `benches/` にベンチマーク追加：純粋計算（IO なし）、大量 call タスク（並列利益）、混合 IO（順序制約）。

**受入基準**
- [ ] AOT が純粋計算用例でインタープリタより明確に高速（具体的な倍率は約束しないが、明らかに遅くはない）。
- [ ] Lazy スケジューリングのオーバーヘッドが観測・位置特定可能（scheduler stats 出力）。

**テスト項目**
- [ ] criterion bench（手動/CI 任意）でレポート生成、ベースライン記録。

---

## 仮定とデフォルト（業務要件でカバーされない部分の選択）

- デフォルト LLVM メジャーバージョンは **17** を選択；チームツールチェーンが異なる場合は、`inkwell` feature とドキュメントを統一修正すればよい。
- AOT 実行パスは「2 ステップ」方式：先にメモリ実行（開発検証）、次に落書きリンク/ロード（真の AOT）。
- 初期 `llvm-aot` は MVP 命令サブセットのみを保証；クロージャ/動的ディスパッチ/例外などの高度機能は後続で必要に応じて拡張（遭遇時は明確な「未実装」エラーを返す）。
- DAG 依存辺は**ランタイムが args の Async TaskId から動的導出可能**；コンパイル時 edges フィールドはまず任意の最適化とデバッグ検証として扱い、M2 達成をブロックしない。
  - **補足（RFC-001）**：ControlEdge の主な由来は Resource 型ルール；Resource 情報が欠落している場合はデフォルトで保守的にシリアル化し正当性を保証。