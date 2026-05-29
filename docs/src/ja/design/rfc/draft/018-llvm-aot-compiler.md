---
title: RFC-018：LLVM AOT コンパイラと L3 透過的並行処理設計
---

# RFC-018：LLVM AOT コンパイラと L3 透過的並行処理設計

> **状態**: 草案
> **作者**: 晨煦
> **作成日**: 2026-02-15
> **最終更新**: 2026-03-10

> **参考**:
> - [RFC-001: 并作モデルと錯誤処理システム](./accepted/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並行処理モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md)

## 摘要

本文書は YaoXiang 言語の LLVM AOT コンパイラを設計する。目标是通过预先编译生成机器码 + DAG 元数据，由运行时**全局 DAG スケジューラ**根据**自底向上**依赖分析执行。

**核心的革新**：
- 不是"遇到函数调用就生成 Future"，而是**从"需要结果的地方"反向分析依赖**
- **叶子节点优先并行执行**，依赖链按序向上遍历
- **孤岛 DAG 独立并行**：没有消费者的节点不阻塞主流程
- **无限循环作为后台 DAG**：调度器切片执行，不会卡死

此设计与 Rust async/await + tokio 运行时模式有本质区别：
- Rust：用户写 `async fn`，编译器生成状态机
- YaoXiang：用户写普通函数，**编译器自动分析 DAG**，调度器自底向上执行

遵循 RFC-001 的 L3 透明并行処理設計：默认 @auto（自动并行），@block 同步是特例，解决颜色函数问题。

## 動機

### なぜ LLVM AOT コンパイラが必要か？

現在 YaoXiang は実行バックエンドとしてインタープリタのみが存在し、以下の問題がある：

| 問題 | 影響 |
|------|------|
| パフォーマンスボトルネック | インタープリタ実行は機械語より 10-100x 遅い |
| 配備が複雑 | インタープリタとランタイムの携带が必要 |
| 颜色関数問題 | 同步関数から并行関数を呼び出せない |

### 颜色関数問題と L3 透過的并行処理

**伝統的な設計（現在）**：
- 同步関数（青色）→ 呼び出せない → 并行関数（赤色）
- 同步がデフォルト、并行には `spawn` マークが必要
- 颜色が「伝染」する：一回并行を使うと、同じ呼び出しチェーン的都是并行

**RFC-001 L3 透過的并行処理（目標）**：
- L3：デフォルト透過的并行処理（@auto）
- L2：明示的な spawn 并行処理
- L1：@block 同步モード

**反转後の設計（RFC-018）**：
- デフォルトで L3 透過的并行処理、コンパイル時に自動的な DAG 依存分析
- 颜色関数問題を解決：同步関数が「デフォルト并行」のコードを直接呼び出せる
- @block は特例として强制串行実行のみに使用

### 核心的革新：自底向上実行 + グローバル DAG

本設計の核心的革新は**自底向上実行モデル**にある：

```
伝統的な呼び出し（トップダウン）：
  call fetch(url) → 実行 → 結果を返す

自底向上実行：
  print(a) ←「結果が必要な場所」から開始
       ↑
  fetch(url0) ← 依存関係を分析、反向検索

  fetch(url1) ← 孤島、独立並行実行
```

**重要な違い**：
- 「関数呼び出しに出会ったら Future を生成する」ではない
- 「最終的な結果が必要な場所」から反向的に依存関係を分析する
- 消費者を持たないノード（孤島）は実行されないか、独立して并行
- 無限ループはバックグラウンド DAG として、スケジューラがスライス実行する

### Rust async との比較

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async モード                            │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：状態機械 + 機械語を生成                                    │
│  実行時：tokio スケジューラが状態機械に従ってスケジュール                        │
│  特徴：await 点はコンパイル時に確定、状態機械が実行を管理                          │
│  粒度：関数レベル                                                │
│  ユーザー体験：async/await キーワードを書く必要がある                           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT モード                    │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：機械語 + DAG メタデータを生成                               │
│  実行時：グローバル DAG スケジューラが自底向上で実行                              │
│  特徴：「結果が必要な場所」から反向的に依存関係を分析、叶子ノードが并行                     │
│  粒度：関数ブロック内の DAG + 関数間 DAG                               │
│  ユーザー体験： обычных関数、自动並行                                   │
└─────────────────────────────────────────────────────────────────┘
```

### グローバル DAG スケジューラ

```
プログラム全体の DAG ビュー：

        print(result) ─────────────────────────┐
           │                                    │
    ┌──────┴──────┐                             │
    │             │                             │
process(a)   process(b)                        │
    │             │                             │
compute(x)   compute(y)  ←── 孤島 DAG ──────────┤
    │                                           │
fetch(url0)  fetch(url1)  fetch(url2)          │
    (已実行)                                    │

同時にバックグラウンド DAG もある（while True）：
    ┌─────────────────────────────────────────┐ │
    │  while True:                            │ │
    │      update_ui()                        │ │
    │      fetch_new() ──→ process(data)      │ │
    └─────────────────────────────────────────┘ │
```

**スケジューラの動作方式**：
```
1.「最終結果」から反向的に分析：
   print(result) → process に依存 → fetch に依存

2. グローバル DAG を構築：
   - 叶子ノード：fetch（依存なし）
   - 内部ノード：process、compute
   - ルートノード：print

3. 実行：
   - fetch を並行実行
   - process は fetch の完了を待機
   - print は process の完了を待機
   - 孤島 compute は独立並行

4. 実行済みスキップ：
   - あるノードが実行済みなら、依存する後続ノードは結果を再利用可
```

### 無限ループの処理

```
シナリオ 1：単一の while/for（スケジュールオーバーヘッドなし）
──────────────────────────────────────────────
main: () -> () = {
    while True {
        update_ui()
        fetch_data()
    }
}
→ 无限ループは1つだけ
→ 直接同期実行、通常のコードとの違いはない

シナリオ 2：複数の while（自動スライス）
──────────────────────────────────────────────
main: () -> () = {
    while True { update_ui() }      # バックグラウンドタスク1
    while True { network_poll() }  # バックグラウンドタスク2
    server_loop()                   # メインタスク
}
→ 3つの独立タスク
→ スケジューラがスライスを切り替える
→ 真の并行処理

スケジューラの適応：
──────────────────────────────────────────────
if タスク数 == 1:
    直接実行（同期）
else:
    スライススケジュール（并行）
```

**バックグラウンド DAG の処理**：
```
メイン DAG（終了あり）：
    fetch → process → print → 終了

バックグラウンド DAG（无限ループ）：
    while True → update_ui → fetch_new → process → 开头に戻る

スケジューラ：
    - メイン DAG は実行完了後に終了
    - バックグラウンド DAG は常に実行だが、スケジューラは「スライス」方式で実行
    - ループで固まらない
```

## 提案

### コア設計

```
┌─────────────────────────────────────────────────────┐
│  コンパイル時                                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Parser  │→│DAG分析  │→│LLVM Codegen│→ 機械語   │
│  └─────────┘  └─────────┘  └─────────┘           │
│                      ↓                           │
│              生成：DAG メタデータ                         │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  実行時                                              │
│  ┌─────────────────────────────────────────────┐ │
│  │  DAG スケジューラライブラリ                               │ │
│  │  • 機械語の読み込み                               │ │
│  │  • DAG メタデータを読み取り                           │ │
│  │  • 遅延スケジュール：呼び出しを保留、必要時に実行          │ │
│  │  • 并行/串行実行をサポート                         │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### 自底向上実行フロー

```
ユーザーコード：
    main: () -> () = {
        pages = urls.map(|url| fetch(url))
        results = pages.map(|page| parse_page(page))
        save_results(results)
    }

フェーズ 1: 自底向上分析（コンパイル時）
─────────────────────────────────────────
save_results(results) から開始：
    "results が必要" → parse_page(results) に依存
    "page0 が必要" → fetch(url0) に依存
    "page1 が必要" → fetch(url1) に依存
    ...

グローバル DAG を構築：
    fetch(url0), fetch(url1), fetch(url2) ← 叶子ノード
           ↓
    parse_page(page0), parse_page(page1)   ← 叶子に依存
           ↓
    save_results                          ← ルートノード

フェーズ 2: 叶子の並行実行（実行時）
─────────────────────────────────────────
スケジューラがすべての叶子ノードを検出：
    - fetch(url0), fetch(url1), fetch(url2) は依存なし → 並行実行
    - 并行数を制御（例：16個）

フェーズ 3: 上方向への走査
─────────────────────────────────────────
parse_page が page0 を必要とするとき：
    - page0 が準備完了かチェック
    - 準備完了 → parse_page を実行
    - 未完了 → 待機、完了後続行

フェーズ 4: 孤島の独立並行
─────────────────────────────────────────
ある fetch の結果を誰も必要としていない場合：
    - 「孤島 DAG」として独立実行
    - 別のコアを使用可能、メインストリームに影響しない
```

### コンパイル成果物構造

```rust
/// コンパイル成果物：機械語 + DAG メタデータ
pub struct CompiledArtifact {
    /// LLVM でコンパイルされた機械語（ELF/Mach-O/COFF）
    machine_code: Vec<u8>,

    /// DAG メタデータ：関数依存関係を記述
    dag: DAGMetadata,

    /// エントリーポイントテーブル
    entries: Vec<EntryPoint>,

    /// 型情報（FFI 用）
    type_info: TypeInfo,
}

/// DAG メタデータ
pub struct DAGMetadata {
    /// ノード：関数呼び出し
    nodes: Vec<DAGNode>,
    /// エッジ：依存関係 (from, to)
    edges: Vec<(usize, usize)>,
}

/// 単一の DAG ノード
pub struct DAGNode {
    /// 関数 ID
    pub function_id: usize,
    /// 依存するノード ID
    pub deps: Vec<usize>,
    /// 副作用タグ（@IO / @Pure）
    pub effect: EffectTag,
}
```

### 実行時スケジューラインターフェース

```rust
/// DAG スケジューラ trait
pub trait DAGScheduler: Send + Sync {
    /// スケジュール実行
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
    /// 最大並行数
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
        // 1. 関数体を走査、すべての呼び出しを保留
        // 2. 実行待機のタスクリストを構築
        // 3. 依存順序に従ってスケジュール実行（并行数を制御）
        // 4. 値が必要になったら実行をトリガー
        // 5. 結果を返す
    }
}
```

### DAG 例：ウェブクローラー

```
main 関数 DAG：
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
│ fetch(url0)      │ @IO       │ 並行ダウンロード                   │
│ fetch(url1)      │ @IO       │ 並行ダウンロード                   │
│ fetch(url2)      │ @IO       │ 並行ダウンロード                   │
│ parse_page       │ @Pure     │ 並行解析                   │
│ filter_links     │ @Pure     │ 並行フィルタリング                   │
│ save_result      │ @IO       │ 順序保存（I/O が順序を保証）    │
│ print            │ @IO       │ 最後に実行                   │
└──────────────────┴────────────┴────────────────────────────┘
```

### スケジューラ実行フェーズ

```
フェーズ 1: 並行ダウンロード
─────────────────────────────────────────
スレッド1: fetch(url0) ──────────┐
スレッド2: fetch(url1) ─────────┼──→ 3つの并行タスク（最大并发数を制限）
スレッド3: fetch(url2) ──────────┘

フェーズ 2: 並行解析
─────────────────────────────────────────
スレッド1: parse_page(page0) ──┐
スレッド2: parse_page(page1) ──┼──→ 3つの并行タスク
スレッド3: parse_page(page2) ──┘

フェーズ 3: 並行フィルタリング
─────────────────────────────────────────
スレッド1: filter_links(result0) ──┐
スレッド2: filter_links(result1) ──┼──→ 3つの并行タスク
スレッド3: filter_links(result2) ──┘

フェーズ 4: 順序保存
─────────────────────────────────────────
スレッド1: save_result(result0) → 完了待機
スレッド1: save_result(result1) → 完了待機
スレッド1: save_result(result2) → 完了待機

フェーズ 5: 出力
─────────────────────────────────────────
スレッド1: print("Fetched 3 pages")
```

## 詳細設計

### モジュール構造

```
src/backends/llvm/
├── mod.rs           # モジュール入口 + Executor 実装
├── context.rs       # LLVM コンテキスト管理
├── types.rs         # 型マッピング (YaoXiang → LLVM)
├── values.rs        # 値マッピング (レジスタ → LLVM Value)
├── codegen.rs       # コアコード生成
├── dag.rs           # DAG 分析と生成
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
| `ref T` | `ptr` (Arc ポインタ) |
| `List(T)` | `ptr` (動的配列) |
| `Struct` | `struct` (対応する構造体) |

### 命令翻訳

各 `BytecodeInstr` を対応する LLVM IR 命令に直接翻訳：

| BytecodeInstr | LLVM IR |
|---------------|---------|
| `BinaryOp { add }` | `llvm.add` |
| `CallStatic` | `llvm.call` |
| `ArcNew` | `call @Arc_new` |
| `LoadElement` | `llvm.getelementptr` + `llvm.load` |

### ランタイムライブラリ

```rust
// コアランタイム関数
extern "C" {
    // 参照カウント
    fn Arc_new(ptr: *mut u8) -> i32;
    fn Arc_clone(ref_count: *mut i32) -> i32;
    fn Arc_drop(ref_count: *mut i32);

    // ヒープ割り当て
    fn Alloc(size: usize) -> *mut u8;
    fn Dealloc(ptr: *mut u8);

    // DAG スケジュール
    fn dag_schedule(dag: *const DAGMetadata, entry: usize) -> RuntimeValue;
}
```

### スケジュール戦略

| アノテーション | シナリオ | スケジュール戦略 |
|------|------|----------|
| `@auto`（デフォルト、L3） | 透過的并行処理 | DAG 遅延スケジュール、依存なしは并行実行 |
| `@block`（L1） | 强制同步 | DAG なし、純粋に串行実行 |
| 循環依存 | 実行時検出 | エラー |

### 副作用処理：暗黙的 Effect System

ユーザーに副作用処理を意識させず、コンパイラが自動的に推断：

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
  print("a") ──→ 順序（的都是 @IO）
  print("b") ──→ 順序
  compute(1) ─┬─→ 並行（DAG スケジュール）
  compute(2) ─┘
```

### 三層ランタイムとの関係

RFC-008 は Embedded / Standard / Full の三層ランタイムアーキテクチャを定義している。LLVM AOT コンパイラと三層ランタイムの対応関係：

| ランタイム | LLVM AOT 動作 |
|--------|---------------|
| **Embedded** | DAG スケジュールなし、直接順序機械語生成 |
| **Standard** | DAG + 単一スレッドスケジュール（num_workers=1） |
| **Full** | DAG + マルチスレッドスケジュール（num_workers>1）、WorkStealing サポート |

### スケジューラインターフェース設計

```rust
/// スケジュール戦略
pub enum ScheduleStrategy {
    /// @block：强制串行、DAG なし
    Serial,
    /// @eager：积极的評価、依存完了を待機
    Eager,
    /// @auto（デフォルト）：遅延スケジュール、DAG 自動スケジュール
    Lazy,
}

/// 副作用タグ
pub enum EffectTag {
    /// 純粋関数、副作用なし
    Pure,
    /// I/O 副作用あり
    IO,
}

/// DAG スケジューラ trait
pub trait DAGScheduler: Send + Sync {
    /// スケジュール実行（戦略パラメータ付き）
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint], strategy: ScheduleStrategy) -> RuntimeValue;

    /// 単一関数実行
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}
```

## トレードオフ

### 优点

1. **パフォーマンス向上**：AOT コンパイルはインタープリタ実行より 10-100x 速い
2. **颜色関数の解決**：デフォルトで并行、同步は特例
3. **統一ランタイム**：インタープリタと LLVM が同じスケジューラを共有
5. **暗黙的副作用**：ユーザーは意識不要、コンパイラが自動処理
6. **所有権セキュリティ**：Rust スタイルの所有権モデルに依存、데이터 レ이스なし

### 欠点

1. **実装复杂度**：LLVM 統合の経験が必要
2. **コンパイル時間**：AOT コンパイルはインタープリタより遅い
3. **デバッグ困難**：AOT コードのデバッグはインタープリタより複雑

### RFC 設計との整合性

| RFC | 整合性 |
|-----|--------|
| RFC-001 并作モデル | ✅ DAG 依存分析がコア |
| RFC-008 ランタイムアーキテクチャ | ✅ ランタイムスケジューラ設計が一致 |
| RFC-009 所有権モデル | ✅ ARC ランタイムが正しく実装済み |

## 代替案

| 案 | 説明 | 選ばない理由 |
|------|------|-----------|
| インタープリタのみ使用 | AOT 不要 | パフォーマンス不足、颜色関数問題 |
| 純粋静的コンパイル | ランタイムスケジュールなし | 遅延スケジュールには実行時が必要 |
| 外部 LLVM runtime をリンク | LLVM の runtime を使用 | 追加の依存関係が必要 |

## 実装戦略

### フェーズ 구분

#### フェーズ 1：基本フレームワーク（1-2 日）

- [ ] `Cargo.toml` に inkwell 依存関係を追加
- [ ] `src/backends/llvm/` モジュールの作成
- [ ] LLVM コンテキスト初期化の実現

#### フェーズ 2：型マッピング（2-3 日）

- [ ] `TypeMap` の実装：YaoXiang 型 → LLVM 型
- [ ] 基本型：i32, i64, f32, f64, bool
- [ ] 複合型：struct, array, tuple
- [ ] 特殊型：Arc, ref, Option

#### フェーズ 3：命令翻訳（3-5 日）

- [ ] `codegen_instruction()` の実装
- [ ] 算術命令：add, sub, mul, div
- [ ] 制御フロー：jmp, jmp_if, ret
- [ ] 関数呼び出し：call, call_virt, call_dyn

#### フェーズ 4：DAG 収集（2-3 日）

- [ ] コード生成時に DAG 情報を収集
- [ ] 関数依存関係の記録
- [ ] 副作用推断（@IO / @Pure）
- [ ] DAG メタデータの生成

#### フェーズ 5：ランタイムライブラリ（3-5 日）

- [ ] 遅延スケジュールの実現
- [ ] DAG スケジューラの実現
- [ ] 粒度制御の実現
- [ ] ARC ランタイムの実現

#### フェーズ 6：統合とテスト（2-3 日）

- [ ] ランタイムライブラリのリンク
- [ ] エンドツーエンドテスト
- [ ] パフォーマンスベンチマーク

### 依存関係

- RFC-001：并作モデル（受付済み）
- RFC-008：Runtime 並行処理モデル（受付済み）
- RFC-009：所有権モデル（受付済み）

### リスク

1. **LLVM 統合复杂度**：inkwell API の深い理解が必要
2. **スケジューラと AOT コードの統合**：インターフェースを精心に設計する必要がある
3. **ABI 互換性**：インタープリタのランタイム ABI との互換性を確保する必要がある

## 関連研究

### Lazy Task Creation (1990)[^1]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 著者 | James R. Larus, Robert H. Halstead Jr. |
| コア | 遅延作成子タスク、オンデマンド作成 |
| 参考価値 | 技術的基盤、遅延スケジュールの概念の起源 |

**コアのアイデア**：即座にタスクを作成するのではなく、遅延作成する。亲タスクが子タスクの値を必要とするときにのみ子タスクを作成する。これにより、細粒度并行タスクのパフォーマンスオーバーヘッドの問題が解決される[^1]。

### Lazy Scheduling (2014)[^2]

| 属性 | 説明 |
|------|------|
| 機関 | University of Maryland |
| 著者 | Tzannes, Caragea |
| コア | 実行時適応スケジュール、追加状態なし |
| 参考価値 | スケジューラ設計、適応的粒度制御 |

**コアのアイデア**：「遅延実行」を通じて自動的に粒度を制御し、複雑な状態を維持する必要がない。システム繁忙時はタスクが自動的にマージされ、空いている時は自動的に分割される[^2]。

### SISAL 言語[^3]

| 属性 | 説明 |
|------|------|
| 機関 | Lawrence Livermore National Laboratory (LLNL) |
| コア | 単一代入言語、Dataflow グラフ、暗黙的并行 |
| 参考価値 | 可行性の証明、Fortran に近いパフォーマンス |

**コアの貢献**：SISAL は Dataflow モデルが産業级アプリケーションで Fortran に近いパフォーマンスを達成できることを証明した[^3]。

### Mul-T 並列 Scheme[^4]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| コア | Future 構築、Lazy Task Creation 実装 |
| 参考価値 | 具体的な実装の参考 |

**コアのメカニズム**：
```scheme
;; Multilisp / Mul-T 構文
(let ((a (future compute-a))      ;; 即座に future を返す
      (b (future compute-b)))      ;; 即座に future を返す
  (join a b))                      ;; 完了を待機
```

### 比較サマリー

| 技術 | 遅延作成 | DAG 分析 | 副作用処理 | 所有権 |
|------|----------|----------|------------|--------|
| Lazy Task Creation[^1] | ✅ | ❌ | ❌ | N/A |
| Lazy Scheduling[^2] | ✅ | ❌ | ❌ | N/A |
| SISAL[^3] | ✅ | ✅ (グローバル) | N/A (単一代入) | N/A |
| Mul-T[^4] | ✅ | ❌ | ❌ | N/A |
| **YaoXiang** | ✅ | ✅ (関数内) | ✅ (暗黙的) | ✅ (ARC) |

**YaoXiang の革新**：現代的な言語機能（所有権 + 暗黙的副作用）を使用して伝統的な設計を簡素化し、DAG 制約を関数ブロック内に限定して複雑さを軽減する。

## 従来の自動并行処理方法との比較

### 従来のコンパイラ：ループレベル並行化

商用车コンパイラ（Intel Fortran、Oracle Fortran）は**ループレベル自動并行化**を採用[^5]：

**コアフロー**：
```
1. 並行化可能なループを識別
2. ループ内の配列アクセスに対して依存分析を実行
3. ループ反復間に依存関係があるかどうかを確定
4. 依存関係がなければ、マルチスレッドコードを生成
```

**依存分析技術**：

| 技術 | 説明 |
|------|------|
| **データ依存** | 2つのアクセスが同じメモリ位置ににアクセスするか |
| **Use-Def** | 変数の定義と使用の関係 |
| **エイリアス分析** | ポインタが同じメモリを指しているか |

**ループが并行可能な条件**：
```fortran
! 並行可能
DO I = 1, N
  A(I) = C(I)
END DO

!  B(I) +並行動作不可（前回の反復に依存）
DO I = 2, N
  A(I) = A(I-1) + B(I)
END DO
```

### Haskell：Spark メカニズム

GHC (Glasgow Haskell Compiler) は純粋関数の并行のために **Spark メカニズム**を採用[^6]：

```haskell
-- rpar: 並行実行、spark を作成
-- rseq: 串行実行、完了を待機

example = do
  a <- rpar (f x)   -- spark を作成し、f x を並行実行
  b <- rpar (g y)   -- spark を作成し、g y を並行実行
  rseq a            -- a の完了を待機
  rseq b            -- b の完了を待機
  return (a, b)
```

**Spark プールメカニズム**：
- プールから spark を取得して、空いている処理コアに割り当て
- spark が使用されていない場合（結果を待つ者がいない）、GC によって回收される
- これにより粒度問題が解決される：小さすぎる spark は破棄される

### Clean 言語：一意性型

Clean 言語は**一意性型（Uniqueness Types）**を通じて並行安全性を実現[^7]：

```clean
-- *Array は一意性を示し、安全に変更可能
modify :: *Array Int -> *Array Int
```

**コアのアイデア**：值が一意に参照されている場合、並行環境での安全に変更できる。他の参照が中間状態を見ることはないため。

### プログラムスライシングと依存グラフ

**プログラム依存グラフ (PDG)** は并行性検出の基礎：

```
ノード：ステートメント
エッジ：データ依存 + 制御依存

並行性検出：
  2つのノード間に到達可能なパスがない場合 → 並行可能
```

### 包括的な比較

| 方法 | 依存分析 | 粒度 | 副作用処理 | 典型シナリオ |
|------|----------|------|------------|----------|
| Intel/Oracle Fortran[^5] | 複雑な配列分析 | ループ反復 | N/A | 科学計算 |
| GHC Spark[^6] | 純粋関数の仮定 | 式 | N/A | 関数型プログラミング |
| Clean[^7] | 一意性型 | グラフ書き換え | N/A | 関数型プログラミング |
| **YaoXiang** | 所有権保証 | 関数呼び出し | 暗黙的推断 | 汎用 |
---

## 付録

### 付録 A：Rust async との詳細な比較

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| コンパイル成果物 | 状態機械 + 機械語 | 機械語 + DAG |
| ランタイム | tokio | DAG Scheduler |
| スケジュールタイミング | コンパイル時に await 点が確定 | 実行時にオンデマンドでスケジュール |
| 并行制御 | 状態機械の状態 | DAG 依存エッジ |
| 颜色関数 | async が伝染 | **L3 透過的并行処理、@block は特例** |
| アノテーション | async/await | @auto/@eager/@block |

### 付録 B：スケジューラ最適化例

**シナリオ 1：スケジューラが実行のマージが可能임을検出**

```
元の DAG:
  compute_a() ──┐
  compute_b() ──┼──→ compute_c()

スケジューラ最適化後:
  compute_a + compute_b を単一タスクにマージ
  → スケジュールオーバーヘッドを削減
```

**シナリオ 2：依存関係が使用されていない**

```
let a = expensive_compute(); // 計算済み
let b = other_thing();       // a が不要
print(b);                    // b を直接返す、a をスキップ
```

### 付録 C：設計議論記録

| 意思決定 | 決定 | 日付 |
|------|------|------|
| LLVM AOT の採用 | 直接 Codegen、過度な抽象化なし | 2026-02-15 |
| DAG スコープ | 関数ブロック内、関数間なし | 2026-02-15 |

| 実行モデル | **自底向上**：結果から反向的に依存関係を分析、叶子が并行 | 2026-03-10 |
| 孤島 DAG | 消費者なしのノードが独立して并行 | 2026-03-10 |
| 無限ループ | バックグラウンド DAG、スケジューラがスライス実行 | 2026-03-10 |
| 副作用処理 | 暗黙的 Effect System、ユーザーは意識不要 | 2026-02-15 |
| 粒度制御 | 并行数制限 + 適応的 | 2026-02-16 |
| 論文引用 | Lazy Task Creation などの追加 | 2026-02-16 |

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
- [tokio ランタイム設計](https://tokio.rs/)
- [RFC-001: 并作モデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 並行処理モデル](./accepted/008-runtime-concurrency-model.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [Implicit Parallelism - Wikipedia](https://en.wikipedia.org/wiki/Implicit_parallelism)

---

## ライフサイクルと行き先

| 状態 | 場所 | 説明 |
|------|------|------|
| **草案** | `docs/design/rfc/` |  著者の草案、社区のレビュー待ち |
| **レビュー中** | `docs/design/rfc/` |  社区の議論とフィードバックを開放 |
| **受付済み** | `docs/design/accepted/` |  正式な設計ドキュメントとして採用 |
| **拒否済み** | `docs/design/rfc/` |  RFC ディレクトリに保存 |