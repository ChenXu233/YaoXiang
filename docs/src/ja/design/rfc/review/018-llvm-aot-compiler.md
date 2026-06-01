---
title: "RFC-018：LLVM AOT コンパイラとL3透明並行処理の設計"
---

# RFC-018：LLVM AOT コンパイラとL3透明並行処理の設計

> **状態**: 草案
> **著者**: 晨煦
> **作成日**: 2026-02-15
> **最終更新**: 2026-03-10

> **参照**:
> - [RFC-001: 並作モデルとエラー処理システム](./accepted/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md)

## 概要

このドキュメントは YaoXiang 言語の LLVM AOT コンパイラを設計するもので、事前コンパイルにより機械語 + DAG メタデータを生成し、実行時に**グローバル DAG スケジューラ**が**ボトムアップ**の依存関係分析に基づいて実行することを目指す。

**核心的な革新**：
- 「関数呼び出しに遭遇たら Future を生成」ではなく、**「結果が必要な場所」から逆方向に依存関係を分析**
- **リーフノードを優先して並列実行**、依存チェーンは順序通りに上方へ辿る
- **孤立 DAG は独立して並列**：消費者を持たないノードはメイン� 플로우を阻塞しない
- **無限ループはバックグラウンド DAG**：スケジューラがスライス実行するため、死角にならない

この設計は Rust async/await + tokio 実行時モデルと本質的に異なる：
- Rust：ユーザーが `async fn` を書き、コンパイラが状態機械を生成
- YaoXiang：ユーザーが通常の関数を書き、**コンパイラが DAG を自動分析**、スケジューラがボトムアップ実行

RFC-001 の L3 透明並行処理を遵守：デフォルト @auto（自動並列）、@block 同期は特例、色付き関数問題を解決。

## 動機

### なぜ LLVM AOT コンパイラが必要か？

現在の YaoXiang は解釈実行のみをバックエンドとしており、以下の問題がある：

| 問題 | 影響 |
|------|------|
| パフォーマンスのボトルネック | 解釈実行は機械語より10-100倍遅い |
| デプロイが複雑 | 解釈器と実行時を持参する必要がある |
| 色付き関数の問題 | 同期関数は並行関数を呼び出せない |

### 色付き関数の問題と L3 透明並行処理

**従来の設計（現在）**：
- 同期関数（青色）→ 呼び出せない → 並行関数（赤色）
- デフォルトは同期、並行には `spawn` マークが必要
- 色分けが「感染」する：一回並行を使うと、同一呼び出しチェーン的都是並行

**RFC-001 L3 透明並行処理（目標）**：
- L3：デフォルト透明並行処理（@auto）
- L2：明示的な spawn 並行処理
- L1：@block 同期モード

**反転後の設計（RFC-018）**：
- デフォルト L3 透明並行処理、コンパイル時に自動 DAG 分析
- 色付き関数の問題を解決：同期関数は「デフォルト並行」コードを直接呼び出せる
- @block は特例として強制シリアル実行のみ

### 核心的な革新：ボトムアップ実行 + グローバル DAG

本設計の核心的な革新は**ボトムアップ実行モデル**にある：

```
従来の呼び出し（トップダウン）：
  call fetch(url) → 実行 → 結果を返す

ボトムアップ実行：
  print(a) ← 「結果が必要な場所」から開始
       ↑
  fetch(url0) ← 依存関係を分析、逆方向に検索

  fetch(url1) ← 孤立、獨立並列実行
```

**重要な違い**：
- 「関数呼び出しに遭遇たら Future を生成」ではなく
- 「最終的な結果が必要な場所」から逆方向に依存関係を分析
- 消費者を持たないノード（孤立）は実行されないか、独立して並列
- 無限ループはバックグラウンド DAG、スケジューラがスライス実行

### Rust async との比較

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async モード                          │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：状態機械 + 機械語を生成                          │
│  実行時：tokioスケジューラが状態機械に従ってスケジューリング    │
│  特徴：await点はコンパイル時に確定、状態機械が実行を管理        │
│  粒度：関数レベル                                                │
│  ユーザー体験：async/awaitキーワードを書く必要がある            │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT モード                   │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：機械語 + DAGメタデータを生成                      │
│  実行時：グローバルDAGスケジューラ、ボトムアップ実行              │
│  特徴：「結果が必要な場所」から逆方向に依存関係を分析、リーフ並列│
│  粒度：関数ブロック内のDAG + 関数間DAG                           │
│  ユーザー体験：通常の関数、自動並列                              │
└─────────────────────────────────────────────────────────────────┘
```

### グローバル DAG スケジューラ

```
プログラム全体のDAGビュー：

        print(result) ─────────────────────────┐
           │                                    │
    ┌──────┴──────┐                             │
    │             │                             │
process(a)   process(b)                        │
    │             │                             │
compute(x)   compute(y)  ←── 孤立DAG ──────────┤
    │                                           │
fetch(url0)  fetch(url1)  fetch(url2)          │
    (実行済み)                                    │

同時にバックグラウンドDAGが一つある（while True）：
    ┌─────────────────────────────────────────┐ │
    │  while True:                            │ │
    │      update_ui()                        │ │
    │      fetch_new() ──→ process(data)      │ │
    └─────────────────────────────────────────┘ │
```

**スケジューラの動作方式**：
```
1. 「最終結果」から逆方向に分析：
   print(result) → processに依存 → fetchに依存

2. グローバルDAGを構築：
   - リーフノード：fetch（依存なし）
   - 内部ノード：process, compute
   - ルートノード：print

3. 実行：
   - fetchは並列実行
   - processはfetchの完了を待つ
   - printはprocessの完了を待つ
   - 孤立computeは独立して並列

4. 実行済みスキップ：
   - あるノードが実行済みの場合、それを依存する後続ノードは結果を再利用可
```

### 無限ループの処理

```
シナリオ1：単一のwhile/for（スケジューリングオーバーヘッドなし）
──────────────────────────────────────────────
main: () -> () = {
    while True {
        update_ui()
        fetch_data()
    }
}
→ 無限ループは1つだけ
→ 直接同期実行、通常のコードとの違いはない

シナリオ2：複数のwhile（自動スライス）
──────────────────────────────────────────────
main: () -> () = {
    while True { update_ui() }      # バックグラウンドタスク1
    while True { network_poll() }  # バックグラウンドタスク2
    server_loop()                   # メインタスク
}
→ 3つの独立タスク
→ スケジューラがスライス切り替え
→ 本当の並行処理

スケジューラの適応：
──────────────────────────────────────────────
if タスク数 == 1:
    直接実行（同期）
else:
    スライススケジューリング（並行）
```

**バックグラウンドDAGの処理**：
```
メインDAG（終了あり）：
    fetch → process → print → 終了

バックグラウンドDAG（無限ループ）：
    while True → update_ui → fetch_new → process → 冒頭に戻る

スケジューラ：
    - メインDAGは実行完了後に終了
    - バックグラウンドDAGは永久に実行されるが、スケジューラは「スライス」方式で実行
    - ループで止まることはない
```

## 提案

### 核心設計

```
┌─────────────────────────────────────────────────────┐
│  コンパイル時                                          │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Parser  │→│DAG分析  │→│LLVM Codegen│→ 機械語  │
│  └─────────┘  └─────────┘  └─────────┘           │
│                      ↓                           │
│              生成：DAGメタデータ                        │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  実行時                                              │
│  ┌─────────────────────────────────────────────┐ │
│  │  DAGスケジューラライブラリ                          │ │
│  │  • 機械語を読み込む                              │ │
│  │  • DAGメタデータを読み取る                        │ │
│  │  • 遅延スケジューリング：呼び出しを中断、需給時に実行│ │
│  │  • 並行/串列実行をサポート                       │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### ボトムアップ実行フロー

```
ユーザーコード：
    main: () -> () = {
        pages = urls.map(|url| fetch(url))
        results = pages.map(|page| parse_page(page))
        save_results(results)
    }

フェーズ1：ボトムアップ分析（コンパイル時）
─────────────────────────────────────────
save_results(results)から開始：
    "resultsが必要" → parse_page(results)に依存
    "page0が必要" → fetch(url0)に依存
    "page1が必要" → fetch(url1)に依存
    ...

グローバルDAGを構築：
    fetch(url0), fetch(url1), fetch(url2) ← リーフノード
           ↓
    parse_page(page0), parse_page(page1)   ← リーフに依存
           ↓
    save_results                          ← ルートノード

フェーズ2：リーフの並列実行（実行時）
─────────────────────────────────────────
スケジューラがすべてのリーフノードを見つける：
    - fetch(url0), fetch(url1), fetch(url2)は依存なし → 並列実行
    - 並行数を制御（比如16個）

フェーズ3：上方へ辿る
─────────────────────────────────────────
parse_pageがpage0を必要とするとき：
    - page0が準備できているか確認
    - 準備できたら → parse_pageを実行
    - 準備できていなければ → 待機、完了後に続行

フェーズ4：孤立は独立して並列
─────────────────────────────────────────
あるfetchの結果が谁にも必要とされていない場合：
    - 「孤立DAG」として独立実行
    - 別のコアを使用可能、メイン� 플로우に影響なし
```

### コンパイル成果物の構造

```rust
/// コンパイル成果物：機械語 + DAGメタデータ
pub struct CompiledArtifact {
    /// LLVMでコンパイルされた機械語（ELF/Mach-O/COFF）
    machine_code: Vec<u8>,

    /// DAGメタデータ：関数依存関係を記述
    dag: DAGMetadata,

    /// エントリポイントテーブル
    entries: Vec<EntryPoint>,

    /// 型情報（FFI用）
    type_info: TypeInfo,
}

/// DAGメタデータ
pub struct DAGMetadata {
    /// ノード：関数呼び出し
    nodes: Vec<DAGNode>,
    /// エッジ：依存関係 (from, to)
    edges: Vec<(usize, usize)>,
}

/// 单一のDAGノード
pub struct DAGNode {
    /// 関数ID
    pub function_id: usize,
    /// 依存するノードID
    pub deps: Vec<usize>,
    /// 副作用タグ（@IO / @Pure）
    pub effect: EffectTag,
}
```

### 実行時スケジューラインターフェース

```rust
/// DAGスケジューラtrait
pub trait DAGScheduler: Send + Sync {
    /// スケジューリング実行
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue;

    /// 単一関数実行
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}

/// スケジューラ実装
pub struct DefaultDAGScheduler {
    /// スレッドプール
    thread_pool: ThreadPool,
    /// コンパイル成果物
    artifact: CompiledArtifact,
    /// 最大並列数
    max_parallelism: usize,
}

impl DefaultDAGScheduler {
    pub fn new(artifact: CompiledArtifact, num_workers: usize) -> Self {
        Self {
            thread_pool: ThreadPool::new(num_workers),
            artifact,
            max_parallelism: num_workers * 2, // 適応的粒度制御
        }
    }
}

impl DAGScheduler for DefaultDAGScheduler {
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue {
        // 1. 関数体を走査、すべての呼び出しを中断
        // 2. 実行待タスクリストを構築
        // 3. 依存順序に従ってスケジューリング実行（並列数を制御）
        // 4. 値が必要になったら実行をトリガー
        // 5. 結果を返す
    }
}
```

### DAGの例：ウェブクローラー

```
main関数のDAG：
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  fetch(url0) ──┐                                           │
│  fetch(url1) ──┼──→ parse_page ──→ filter_links ──┐      │
│  fetch(url2) ──┘                                       │      │
│                                                          │      │
│                     save_result ──→ print              │      │
│                          ↑                              │      │
│                          └──────────────────────────────┘      │
│                                                             │
└─────────────────────────────────────────────────────────────┘

ノード説明：
┌──────────────────┬────────────┬────────────────────────────┐
│ ノード              │ 副作用     │ 説明                       │
├──────────────────┼────────────┼────────────────────────────┤
│ fetch(url0)      │ @IO       │ 並行ダウンロード              │
│ fetch(url1)      │ @IO       │ 並行ダウンロード              │
│ fetch(url2)      │ @IO       │ 並行ダウンロード              │
│ parse_page       │ @Pure     │ 並列解析                     │
│ filter_links     │ @Pure     │ 並列フィルタリング            │
│ save_result      │ @IO       │ 順序保存（I/Oが順序を保証）   │
│ print            │ @IO       │ 最後に実行                   │
└──────────────────┴────────────┴────────────────────────────┘
```

### スケジューラ実行フェーズ

```
フェーズ1：並行ダウンロード
─────────────────────────────────────────
スレッド1: fetch(url0) ──────────┐
スレッド2: fetch(url1) ─────────┼──→ 3つの並行タスク（最大並列数を制限）
スレッド3: fetch(url2) ──────────┘

フェーズ2：並行解析
─────────────────────────────────────────
スレッド1: parse_page(page0) ──┐
スレッド2: parse_page(page1) ──┼──→ 3つの並行タスク
スレッド3: parse_page(page2) ──┘

フェーズ3：並行フィルタリング
─────────────────────────────────────────
スレッド1: filter_links(result0) ──┐
スレッド2: filter_links(result1) ──┼──→ 3つの並行タスク
スレッド3: filter_links(result2) ──┘

フェーズ4：順序保存
─────────────────────────────────────────
スレッド1: save_result(result0) → 完了を待つ
スレッド1: save_result(result1) → 完了を待つ
スレッド1: save_result(result2) → 完了を待つ

フェーズ5：出力
─────────────────────────────────────────
スレッド1: print("Fetched 3 pages")
```

## 詳細設計

### モジュール構造

```
src/backends/llvm/
├── mod.rs           # モジュールエントリ + Executor実装
├── context.rs       # LLVMコンテキスト管理
├── types.rs         # 型マッピング (YaoXiang → LLVM)
├── values.rs        # 値マッピング (レジスタ → LLVM Value)
├── codegen.rs       # コアコード生成
├── dag.rs           # DAG分析と生成
├── scheduler.rs      # 実行時スケジューラ
└── tests.rs         # テスト
```

### 型マッピング

| YaoXiang 型 | LLVM 型 |
|---------------|----------|
| `Int` | `i64` |
| `Float` | `f64` |
| `Bool` | `i1` |
| `String` | `ptr` (構造体) |
| `Arc(T)` | `{ i32, T }` (参照カウント構造体) |
| `ref T` | `ptr` (Arcポインタ) |
| `List(T)` | `ptr` (動的配列) |
| `Struct` | `struct` (対応する構造体) |

### 命令翻訳

各 `BytecodeInstr` は対応する LLVM IR 命令に直接翻訳：

| BytecodeInstr | LLVM IR |
|---------------|---------|
| `BinaryOp { add }` | `llvm.add` |
| `CallStatic` | `llvm.call` |
| `ArcNew` | `call @Arc_new` |
| `LoadElement` | `llvm.getelementptr` + `llvm.load` |

### 実行時ライブラリ

```rust
// コア実行時関数
extern "C" {
    // 参照カウント
    fn Arc_new(ptr: *mut u8) -> i32;
    fn Arc_clone(ref_count: *mut i32) -> i32;
    fn Arc_drop(ref_count: *mut i32);

    // ヒープ割り当て
    fn Alloc(size: usize) -> *mut u8;
    fn Dealloc(ptr: *mut u8);

    // DAGスケジューリング
    fn dag_schedule(dag: *const DAGMetadata, entry: usize) -> RuntimeValue;
}
```

### スケジューリング戦略

| アノテーション | シナリオ | スケジューリング戦略 |
|------|------|----------|
| `@auto`（デフォルト、L3） | 透明並行処理 | DAG遅延スケジューリング、依存なしで並列実行 |
| `@block`（L1） | 強制同期 | DAGなし、純粋串列実行 |
| 循環依存 | 実行時検出 | エラーを報告 |

### 副作用処理：暗黙的Effect System

ユーザーに副作用処理を意識させることはなく、コンパイラが自動的に推断：

```
ユーザーコード：
  print("a");
  print("b");
  let x = compute(1);
  let y = compute(2);

コンパイラの推断：
  print → @IO（外部呼び出し）
  compute → @Pure（純粋関数）

スケジューラ実行：
  print("a") ──→ 順序（両方とも@IO）
  print("b") ──→ 順序
  compute(1) ─┬─→ 並列（DAGスケジューリング）
  compute(2) ─┘
```

### 三層実行時との関係

RFC-008は Embedded / Standard / Full の三層実行時アーキテクチャを定義している。LLVM AOT コンパイラと三層実行時の対応関係：

| 実行時 | LLVM AOT の動作 |
|--------|---------------|
| **Embedded** | DAGスケジューリングなし、直接順序機械語を生成 |
| **Standard** | DAG + 単一スレッドスケジューリング（num_workers=1） |
| **Full** | DAG + マルチスレッドスケジューリング（num_workers>1）、WorkStealingをサポート |

### スケジューラインターフェース設計

```rust
/// スケジューリング戦略
pub enum ScheduleStrategy {
    /// @block：強制串列、DAGなし
    Serial,
    /// @eager：熱心な評価、依存の完了を待つ
    Eager,
    /// @auto（デフォルト）：遅延スケジューリング、DAG自動スケジューリング
    Lazy,
}

/// 副作用タグ
pub enum EffectTag {
    /// 純粋関数、副作用なし
    Pure,
    /// I/O副作用あり
    IO,
}

/// DAGスケジューラtrait
pub trait DAGScheduler: Send + Sync {
    /// スケジューリング実行（戦略パラメータ付き）
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint], strategy: ScheduleStrategy) -> RuntimeValue;

    /// 単一関数実行
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}
```

## トレードオフ

### 优点

1. **パフォーマンス向上**：AOTコンパイルは解釈実行より10-100倍高速
2. **色付き関数の解決**：デフォルトで並行処理、同期は特例
3. **統一実行時**：解釈器とLLVMは同一スケジューラを共有
5. **暗黙的副作用**：ユーザーに意識させることなし、コンパイラが自動処理
6. **所有権の安全性**：Rust流的所有権モデルに依存、データ競合なし

### 缺点

1. **実装の複雑さ**：LLVM統合の経験が必要
2. **コンパイル時間**：AOTコンパイルは解釈器より遅い
3. **デバッグの困難**：AOTコードのデバッグは解釈器より複雑

### RFC設計との整合性

| RFC | 整合性 |
|-----|--------|
| RFC-001 並作モデル | ✅ DAG依存関係分析がコア |
| RFC-008 実行時アーキテクチャ | ✅ 実行時スケジューラ設計と一致 |
| RFC-009 所有権モデル | ✅ ARC実行時を正しく実装 |

## 代替案

| 方案 | 説明 | 選定しない理由 |
|------|------|-----------|
| 解釈器のみ使用 | AOT不要 | パフォーマンス不足、色付き関数の問題 |
| 純粋静的コンパイル | 実行時スケジューリングなし | 遅延スケジューリングは実行時に必要 |
| 外部LLVM runtimeをリンク | LLVMのruntimeを使用 | 追加の依存関係が必要 |

## 実装戦略

### フェーズ分け

#### フェーズ1：基本フレームワーク（1-2日）

- [ ] `Cargo.toml`にinkwell依存を追加
- [ ] `src/backends/llvm/`モジュールを作成
- [ ] LLVMコンテキスト初期化を実装

#### フェーズ2：型マッピング（2-3日）

- [ ] `TypeMap`を実装：YaoXiang型 → LLVM型
- [ ] 基本型：i32, i64, f32, f64, bool
- [ ] 複合型：struct, array, tuple
- [ ] 特殊型：Arc, ref, Option

#### フェーズ3：命令翻訳（3-5日）

- [ ] `codegen_instruction()`を実装
- [ ] 算術命令：add, sub, mul, div
- [ ] 制御フロー：jmp, jmp_if, ret
- [ ] 関数呼び出し：call, call_virt, call_dyn

#### フェーズ4：DAG収集（2-3日）

- [ ] コード生成時にDAG情報を収集
- [ ] 関数依存関係を記録
- [ ] 副作用推断（@IO / @Pure）
- [ ] DAGメタデータを生成

#### フェーズ5：実行時ライブラリ（3-5日）

- [ ] 遅延スケジューリングを実装
- [ ] DAGスケジューラを実装
- [ ] 粒度制御を実装
- [ ] ARC実行時を実装

#### フェーズ6：統合とテスト（2-3日）

- [ ] 実行時ライブラリをリンク
- [ ] エンドツーエンドテスト
- [ ] パフォーマンスベンチマーク

### 依存関係

- RFC-001：並作モデル（承認済み）
- RFC-008：Runtime並行モデル（承認済み）
- RFC-009：所有権モデル（承認済み）

### リスク

1. **LLVM統合の複雑さ**：inkwell APIの深い理解が必要
2. **スケジューラとAOTコードの統合**：インターフェースを精心に設計する必要がある
3. **ABI互換性**：解釈器実行時とのABI互換性を確保する必要がある

## 関連作業

### Lazy Task Creation (1990)[^1]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 著者 | James R. Larus, Robert H. Halstead Jr. |
| コア | 遅延タスク生成、需給時に子タスクを作成 |
| 参照価値 | 技術的基盤、遅延スケジューリング概念の起源 |

**核心的なアイデア**：タスクを即座に作成するのではなく、遅延作成する。親タスクが子タスクの値を必要とするときにのみ子タスクを作成する。これにより、細粒度並行タスクのパフォーマンスオーバーヘッドの問題を解決[^1]。

### Lazy Scheduling (2014)[^2]

| 属性 | 説明 |
|------|------|
| 機関 | University of Maryland |
| 著者 | Tzannes, Caragea |
| コア | 実行時適応スケジューリング、追加状態なし |
| 参照価値 | スケジューラ設計、適応的粒度制御 |

**核心的なアイデア**：「遅延実行」を通じて自動的に粒度を制御し、複雑な状態を維持する必要がない。システムがビジーの場合はタスクが自動的にマージされ、アイドルの場合は自動的に分割[^2]。

### SISAL言語[^3]

| 属性 | 説明 |
|------|------|
| 機関 | Lawrence Livermore National Laboratory (LLNL) |
| コア | 単一代入言語、Dataflowグラフ、暗黙的並列処理 |
| 参照価値 | 可行性の証明、Fortranに近いパフォーマンス |

**核心的な貢献**：SISALは、Dataflowモデルが工業レベルのアプリケーションでFortranに近いパフォーマンスを達成できることを証明[^3]。

### Mul-T並列Scheme[^4]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| コア | Future構築、Lazy Task Creation実装 |
| 参照価値 | 具体的な実装の参照 |

**核心的なメカニズム**：
```scheme
;; Multilisp / Mul-T 構文
(let ((a (future compute-a))      ;; 即座にfutureを返す
      (b (future compute-b)))      ;; 即座にfutureを返す
  (join a b))                      ;; 完了を待つ
```

### 比較まとめ

| 技術 | 遅延作成 | DAG分析 | 副作用処理 | 所有権 |
|------|----------|----------|------------|--------|
| Lazy Task Creation[^1] | ✅ | ❌ | ❌ | N/A |
| Lazy Scheduling[^2] | ✅ | ❌ | ❌ | N/A |
| SISAL[^3] | ✅ | ✅ (グローバル) | N/A (単一代入) | N/A |
| Mul-T[^4] | ✅ | ❌ | ❌ | N/A |
| **YaoXiang** | ✅ | ✅ (関数内) | ✅ (暗黙的) | ✅ (ARC) |

**YaoXiangの革新**：モダンな言語機能（所有権 + 暗黙的副作用）を使用して従来の設計を簡素化し、DAG制約を関数ブロック内に置いて複雑さを軽減。

## 従来の自動並列処理手法との比較

### 従来のコンパイラ：ループレベル並列化

商用コンパイラ（Intel Fortran、Oracle Fortranなど）は**ループレベル自動並列化**を採用[^5]：

**コアフロー**：
```
1. 並列化可能なループを識別
2. ループ内の配列アクセスに対して依存関係分析を実行
3. ループ反復間に依存関係があるかどうかを決定
4. 依存関係がなければ、マルチスレッドコードを生成
```

**依存関係分析技術**：

| 技術 | 説明 |
|------|------|
| **データ依存** | 2つのアクセスが同じメモリ位置にアクセスするか |
| **Use-Def** | 変数の定義と使用の関係 |
| **エイリアス分析** | ポインタが同じメモリを指しているか |

**ループが並列化可能な条件**：
```fortran
! 並列化可能
DO I = 1, N
  A(I) = C(I)
END DO

!  B(I) +不可並列化（前方の反復に依存）
DO I = 2, N
  A(I) = A(I-1) + B(I)
END DO
```

### Haskell：Sparkメカニズム

GHC (Glasgow Haskell Compiler) は純粋関数の並列化のために**Sparkメカニズム**を採用[^6]：

```haskell
-- rpar: 並列実行、sparkを作成
-- rseq: 串列実行、完了を待つ

example = do
  a <- rpar (f x)   -- sparkを作成、f xを並列実行
  b <- rpar (g y)   -- sparkを作成、g yを並列実行
  rseq a            -- aの完了を待つ
  rseq b            -- bの完了を待つ
  return (a, b)
```

**Sparkプールメカニズム**：
- プールからsparkを取り出してアイドルな処理コアに割り当て
- sparkが使用されていない（誰も結果を待っていない）場合、GCによって回収される
- これにより粒度の問題が解決：小さすぎるsparkは破棄される

### Clean言語：一意性型

Clean言語は**一意性型（Uniqueness Types）**を通じて並行安全性を実現[^7]：

```clean
-- *Arrayは一意性を表し、安全に変更可能
modify :: *Array Int -> *Array Int
```

**核心的なアイデア**：値が単一参照であれば、並行環境下で安全に修改可能である。中间状態を他の参照が見ることがないため。

### プログラムスライシングと依存グラフ

**プログラム依存グラフ（PDG）**は並列性検出の基盤：

```
ノード：文
エッジ：データ依存 + 制御依存

並列性検出：
  2つのノード間に到達可能なパスがなければ → 並列化可能
```

### 総合比較

| 方法 | 依存関係分析 | 粒度 | 副作用処理 | 典型的なシナリオ |
|------|----------|------|------------|----------|
| Intel/Oracle Fortran[^5] | 複雑な配列分析 | ループ反復 | N/A | 科学計算 |
| GHC Spark[^6] | 純粋関数の仮定 | 式 | N/A | 関数型プログラミング |
| Clean[^7] | 一意性型 | グラフ書き換え | N/A | 関数型プログラミング |
| **YaoXiang** | 所有権保証 | 関数呼び出し | 暗黙的推断 | 汎用 |

---

## 付録

### 付録A：Rust asyncとの比較詳細

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| コンパイル成果物 | 状態機械 + 機械語 | 機械語 + DAG |
| 実行時 | tokio | DAG Scheduler |
| スケジューリングタイミング | コンパイル時にawait点を確定 | 実行時に需給に応じてスケジューリング |
| 並行制御 | 状態機械の状態 | DAG依存エッジ |
| 色付き関数 | async感染 | **L3透明並行処理、@block特例** |
| アノテーション | async/await | @auto/@eager/@block |

### 付録B：スケジューラ最適化例

**シナリオ1：スケジューラが実行をマージ可能と検出**

```
元のDAG:
  compute_a() ──┐
  compute_b() ──┼──→ compute_c()

スケジューラ最適化後:
  compute_a + compute_bを単一タスクにマージ
  → スケジューリングオーバーヘッドを削減
```

**シナリオ2：依存関係が使用されていない**

```
let a = expensive_compute(); // 計算済み
let b = other_thing();       // aは不要
print(b);                    // bを直接返す、aをスキップ
```

### 付録C：設計議論記録

| 意思決定 | 決定 | 日付 |
|------|------|------|
| LLVM AOTを採用 | 直接Codegen、過度な抽象化なし | 2026-02-15 |
| DAGスコープ | 関数ブロック内、関数間なし | 2026-02-15 |

| 実行モデル | **ボトムアップ**：結果から逆方向に依存関係を分析、リーフ並列 | 2026-03-10 |
| 孤立DAG | 消費者を持たないノードは独立して並列 | 2026-03-10 |
| 無限ループ | バックグラウンドDAG、スケジューラがスライス実行 | 2026-03-10 |
| 副作用処理 | 暗黙的Effect System、ユーザーに意識させない | 2026-02-15 |
| 粒度制御 | 並行数制限 + 適応的 | 2026-02-16 |
| 論文引用 | Lazy Task Creationなどを追加 | 2026-02-16 |

---

## 参考文献

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT. Retrieved from https://people.csail.mit.edu/riastradh/t/halstead90lazy-task.pdf

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland. Retrieved from https://user.eng.umd.edu/~barua/tzannes-TOPLAS-2014.pdf

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory. Retrieved from https://www.sciencedirect.com/science/article/abs/pii/074373159090035N

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT. Retrieved from https://link.springer.com/content/pdf/10.1007/bfb0024163.pdf

[^5]: Intel Corporation. *Automatic Parallelization with Intel Compilers*. Retrieved from https://www.intel.com/content/www/us/en/developer/articles/technical/automatic-parallelization-with-intel-compilers.html

[^6]: Marlow, S. (2010). *Parallel and Concurrent Programming in Haskell*. Retrieved from https://www.cse.chalmers.se/edu/year/2015/course/pfp/Papers/strategies-tutorial-v2.pdf

[^7]: Plasmeijer, R., & van Eekelen, M. (2011). *Clean Language Documentation*. University of Nijmegen. Retrieved from https://clean.cs.ru.nl/Documentation

- [Rust async book](https://rust-lang.github.io/async-book/)
- [inkwell LLVM bindings](https://cranelift.dev/)
- [tokio 実行時設計](https://tokio.rs/)
- [RFC-001: 並作モデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime並行モデル](./accepted/008-runtime-concurrency-model.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [Implicit Parallelism - Wikipedia](https://en.wikipedia.org/wiki/Implicit_parallelism)

---

## ライフサイクルと去向

| 状態 | 場所 | 説明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 著者の下書き、社区レビュー待ち |
| **レビュー中** | `docs/design/rfc/` | 社区議論とフィードバックを募集中 |
| **承認済み** | `docs/design/accepted/` | 正式な設計ドキュメントになる |
| **拒否済み** | `docs/design/rfc/` | RFCディレクトリに保存 |