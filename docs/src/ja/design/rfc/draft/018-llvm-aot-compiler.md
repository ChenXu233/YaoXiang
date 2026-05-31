---
title: "RFC-018：LLVM AOT コンパイラと L3 透過的並行処理設計"
---

# RFC-018：LLVM AOT コンパイラと L3 透過的並行処理設計

> **状態**: 草案
> **著者**: 晨煦
> **作成日**: 2026-02-15
> **最終更新**: 2026-03-10

> **参照**:
> - [RFC-001: 並列生成モデルとエラー処理システム](./accepted/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md)

## 摘要

このドキュメントは YaoXiang 言語の LLVM AOT コンパイラを設計するもので、事前コンパイルによって機械語コード + DAG メタデータを生成し、実行時に**グローバル DAG スケジューラ**が**ボトムアップ**の依存解析に基づいて実行することを目標としている。

**核心的革新**：
- 「関数呼び出しに遭遇したら Future を生成する」ではなく、**「結果が必要な場所」から逆方向に依存関係を解析する**
- **葉ノードが優先して並列実行**され、依存チェーンは順序通りに上方へ辿る
- **孤立 DAG は独立して並列**：コンシューマを持たないノードはメイン�フローをブロックしない
- **無限ループはバックグラウンド DAG として**：スケジューラがスライス実行し、フリーズしない

この設計は Rust async/await + tokio ランタイムモデルとは本質的に異なる：
- Rust：ユーザーが `async fn` を書き、コンパイラが状態機械を生成
- YaoXiang：ユーザーが通常の関数を書き、**コンパイラが自動的に DAG を解析**し、スケジューラがボトムアップで実行

RFC-001 の L3 透過的並行処理設計に従う：デフォルトは @auto（自動並列）、@block 同期は特例であり、色付き関数問題を解決する。

## 動機

### なぜ LLVM AOT コンパイラが必要か？

現在 YaoXiang は解釈実行のみをバックエンドとして持っているしており、以下の問題が存在する：

| 問題点 | 影響 |
|------|------|
| パフォーマンス瓶頸 | 解釈実行は機械語より 10-100x 遅い |
| デプロイが複雑 | 解釈器とランタイムを持ち歩く必要がある |
| 色付き関数問題 | 同期関数が並行関数を呼び出せない |

### 色付き関数問題と L3 透過的並行処理

**従来設計（現在）**：
- 同期関数（青色）→ 呼び出せない → 並行関数（赤色）
- デフォルトは同期、並行には `spawn` マークが必要
- 色が「感染」する：一回並行を使うと、同じ呼び出しチェーン的都是並行

**RFC-001 L3 透過的並行処理（目標）**：
- L3：デフォルト透過的並行処理（@auto）
- L2：明示的な spawn 並行処理
- L1：@block 同期モード

**逆転後の設計（RFC-018）**：
- デフォルトで L3 透過的並行処理、コンパイル時に自動的に DAG 依存関係を解析
- 色付き関数問題を解決：同期関数が「デフォルト並行」のコードを直接呼び出せる
- @block は特例としてのみシリアル実行を強制

### 核心的革新：ボトムアップ実行 + グローバル DAG

本設計の核心的革新は**ボトムアップ実行モデル**にある：

```
従来呼び出し（トップダウン）：
  call fetch(url) → 実行 → 結果を返す

ボトムアップ実行：
  print(a) ← 「結果が必要な場所」から開始
       ↑
  fetch(url0) ← 依存関係を解析し、逆方向に查找

  fetch(url1) ← 孤立、獨立並行実行
```

**重要な違い**：
- 「関数呼び出しに遭遇したら Future を生成する」ではない
- 「最終的な結果」から逆方向に依存関係を解析する
- コンシューマを持たないノード（孤立）は実行されないか、独立して並列実行
- 無限ループはバックグラウンド DAG として、スケジューラがスライス実行

### Rust async との比較

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async モード                            │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：状態機械 + 機械語を生成                                  │
│  実行時：tokio スケジューラが状態機械に基づいてスケジューリング                │
│  特徴：await 点はコンパイル時に確定、状態機械が実行を管理                     │
│  粒度：関数レベル                                                │
│  ユーザー体験：async/await キーワードを書く必要がある                       │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT モード                    │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：機械語コード + DAG メタデータを生成                             │
│  実行時：グローバル DAG スケジューラ、ボトムアップ実行                         │
│  特徴：「結果が必要な場所」から逆方向に依存関係を解析、葉ノードが並列             │
│  粒度：関数ブロック内の DAG + 関数間 DAG                                   │
│  ユーザー体験：通常の関数、自动並列                                            │
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
compute(x)   compute(y)  ←── 孤立 DAG ──────────┤
    │                                           │
fetch(url0)  fetch(url1)  fetch(url2)          │
    (済み)                                      │

同時にバックグラウンド DAG（while True）：
    ┌─────────────────────────────────────────┐ │
    │  while True:                            │ │
    │      update_ui()                        │ │
    │      fetch_new() ──→ process(data)      │ │
    └─────────────────────────────────────────┘ │
```

**スケジューラの動作方式**：
```
1. 「最終結果」から逆方向に解析：
   print(result) → process に依存 → fetch に依存

2. グローバル DAG を構築：
   - 葉ノード：fetch（依存なし）
   - 内部ノード：process, compute
   - ルートノード：print

3. 実行：
   - fetch を並行実行
   - process は fetch の完了を待機
   - print は process の完了を待機
   - 孤立 compute は独立して並列

4. 実行済みはスキップ：
   - あるノードが実行済みなら、依存する後続ノードは結果を再利用可
```

### 無限ループ処理

```
シナリオ 1：単一の while/for（スケジューリングオーバーヘッドなし）
──────────────────────────────────────────────
main: () -> () = {
    while True {
        update_ui()
        fetch_data()
    }
}
→ 无限ループは1つだけ
→ 直接同期実行、通常のコードと同じ

シナリオ 2：複数の while（自動スライス）
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

**バックグラウンド DAG 処理**：
```
メイン DAG（終了あり）：
    fetch → process → print → 終了

バックグラウンド DAG（无限ループ）：
    while True → update_ui → fetch_new → process → 最初から戻る

スケジューラ：
    - メイン DAG が実行完了したら終了
    - バックグラウンド DAG は永久に実行されるが、スケジューラは「スライス」方式で実行
    - ループでフリーズことはない
```

## 提案

### 核心設計

```
┌─────────────────────────────────────────────────────┐
│  コンパイル時                                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Parser  │→│DAG分析  │→│LLVM Codegen│→ 機械語コード │
│  └─────────┘  └─────────┘  └─────────┘           │
│                      ↓                           │
│              生成：DAG メタデータ                         │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  実行時                                              │
│  ┌─────────────────────────────────────────────┐ │
│  │  DAG スケジューラライブラリ                               │ │
│  │  • 機械語コードをロード                                │ │
│  │  • DAG メタデータを読み込み                           │ │
│  │  • 遅延スケジューリング：呼び出しを中断、按需実行              │ │
│  │  • 並行/串行実行をサポート                         │ │
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

フェーズ 1: ボトムアップ解析（コンパイル時）
─────────────────────────────────────────
save_results(results) から始める：
    "results が必要" → parse_page(results) に依存
    "page0 が必要" → fetch(url0) に依存
    "page1 が必要" → fetch(url1) に依存
    ...

グローバル DAG を構築：
    fetch(url0), fetch(url1), fetch(url2) ← 葉ノード
           ↓
    parse_page(page0), parse_page(page1)   ← 葉に依存
           ↓
    save_results                          ← ルートノード

フェーズ 2: 葉の並行実行（実行時）
─────────────────────────────────────────
スケジューラがすべての葉ノードを見つける：
    - fetch(url0), fetch(url1), fetch(url2) は依存なし → 並行実行
    - 並行数を制御（例えば 16個）

フェーズ 3: 上方へ辿る
─────────────────────────────────────────
parse_page が page0 を必要とするとき：
    - page0 が準備できているか確認
    - 準備できている → parse_page を実行
    - 準備できてない → 待機、完了後に継続

フェーズ 4: 孤立は独立して並列
─────────────────────────────────────────
ある fetch の結果が必要としていない場合：
    - 「孤立 DAG」として独立実行
    - 別のコアを使用可能、メインフローに影響しない
```

### コンパイル成果物構造

```rust
/// コンパイル成果物：機械語コード + DAG メタデータ
pub struct CompiledArtifact {
    /// LLVM でコンパイルされた機械語コード（ELF/Mach-O/COFF）
    machine_code: Vec<u8>,

    /// DAG メタデータ：関数依存関係を記述
    dag: DAGMetadata,

    /// エントリポイント表
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

/// 単一 DAG ノード
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
/// DAG スケジューラ トレイト
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
        // 1. 関数を走査、呼び出しをすべて中断
        // 2. 実行待ちタスクリストを構築
        // 3. 依存順序でスケジューリング実行（並行数を制御）
        // 4. 値が必要になったら実行をトリガー
        // 5. 結果を返す
    }
}
```

### DAG 例：ウェブクローラ

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
│ save_result      │ @IO       │ 順序保存（I/Oが順序を保証）    │
│ print            │ @IO       │ 最後に実行                   │
└──────────────────┴────────────┴────────────────────────────┘
```

### スケジューラ実行フェーズ

```
フェーズ 1: 並行ダウンロード
─────────────────────────────────────────
スレッド1: fetch(url0) ──────────┐
スレッド2: fetch(url1) ─────────┼──→ 3つの並行タスク（最大並行数を制限）
スレッド3: fetch(url2) ──────────┘

フェーズ 2: 並行解析
─────────────────────────────────────────
スレッド1: parse_page(page0) ──┐
スレッド2: parse_page(page1) ──┼──→ 3つの並行タスク
スレッド3: parse_page(page2) ──┘

フェーズ 3: 並行フィルタリング
─────────────────────────────────────────
スレッド1: filter_links(result0) ──┐
スレッド2: filter_links(result1) ──┼──→ 3つの並行タスク
スレッド3: filter_links(result2) ──┘

フェーズ 4: 順序保存
─────────────────────────────────────────
スレッド1: save_result(result0) → 完了を待機
スレッド1: save_result(result1) → 完了を待機
スレッド1: save_result(result2) → 完了を待機

フェーズ 5: 出力
─────────────────────────────────────────
スレッド1: print("Fetched 3 pages")
```

## 詳細設計

### モジュール構造

```
src/backends/llvm/
├── mod.rs           # モジュールエントリ + Executor 実装
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

    // DAG スケジューリング
    fn dag_schedule(dag: *const DAGMetadata, entry: usize) -> RuntimeValue;
}
```

### スケジューリング戦略

| アノテーション | シナリオ | スケジューリング戦略 |
|------|------|----------|
| `@auto`（デフォルト、L3） | 透過的並行処理 | DAG 遅延スケジューリング、依存なしで並行実行 |
| `@block`（L1） | 強制同期 | DAG なし、純粋串行実行 |
| 循環依存 | 実行時検出 | エラーを報告 |

### 副作用処理：暗黙的 Effect System

ユーザーには副作用処理が感知できず、コンパイラが自動的に推断する：

```
ユーザーコード：
  print("a");
  print("b");
  let x = compute(1);
  let y = compute(2);

コンパイラが推断：
  print → @IO（外部呼び出し）
  compute → @Pure（純粋関数）

スケジューラ実行：
  print("a") ──→ 順序（すべて @IO）
  print("b") ──→ 順序
  compute(1) ─┬─→ 並行（DAG スケジューリング）
  compute(2) ─┘
```

### 3層ランタイムとの関係

RFC-008 は Embedded / Standard / Full の3層ランタイムアーキテクチャを定義している。LLVM AOT コンパイラと3層ランタイムの対応関係：

| ランタイム | LLVM AOT の動作 |
|--------|---------------|
| **Embedded** | DAG スケジューリングなし、直接順序機械語コードを生成 |
| **Standard** | DAG + 単一スレッドスケジューリング（num_workers=1） |
| **Full** | DAG + マルチスレッドスケジューリング（num_workers>1）、WorkStealing をサポート |

### スケジューラインターフェース設計

```rust
/// スケジューリング戦略
pub enum ScheduleStrategy {
    /// @block：強制串行、DAG なし
    Serial,
    /// @eager：積極的評価、依存の完了を待機
    Eager,
    /// @auto（デフォルト）：遅延スケジューリング、DAG 自動スケジューリング
    Lazy,
}

/// 副作用タグ
pub enum EffectTag {
    /// 純粋関数、副作用なし
    Pure,
    /// I/O 副作用あり
    IO,
}

/// DAG スケジューラ トレイト
pub trait DAGScheduler: Send + Sync {
    /// スケジューリング実行（戦略パラメータ付き）
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint], strategy: ScheduleStrategy) -> RuntimeValue;

    /// 単一関数実行
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}
```

## 权衡

### 利点

1. **パフォーマンス向上**：AOT コンパイルは解釈実行より 10-100x 速い
2. **色付き関数の解決**：デフォルトで並行処理、同期は特例
3. **統一ランタイム**：解釈器と LLVM が同じスケジューラを共有
5. **暗黙的副作用**：ユーザーには感知できず、コンパイラが自動的に処理
6. **所有権の安全性**：Rust スタイルの所有権モデルに依存、データ競合なし

### 欠点

1. **実装の複雑さ**：LLVM 統合の経験が必要
2. **コンパイル時間**：AOT コンパイルは解釈器より遅い
3. **デバッグ困難**：AOT コードのデバッグは解釈器より複雑

### RFC 設計との整合性

| RFC | 整合性 |
|-----|--------|
| RFC-001 並列生成モデル | ✅ DAG 依存解析が核心 |
| RFC-008 ランタイムアーキテクチャ | ✅ 実行時スケジューラ設計が一致 |
| RFC-009 所有権モデル | ✅ ARC ランタイムが正しく実装 |

## 代替案

| 方案 | 説明 | 採用しない理由 |
|------|------|-----------|
| 解釈器のみ使用 | AOT が不要 | パフォーマンス不足、色付き関数問題 |
| 純粋静的コンパイル | 実行時スケジューリングなし | 遅延スケジューリングには実行時が必要 |
| 外部 LLVM runtime をリンク | LLVM の runtime を使用 | 追加の依存関係が必要 |

## 実装戦略

### フェーズ分け

#### フェーズ 1：基盤フレームワーク（1-2 日）

- [ ] `Cargo.toml` に inkwell 依存を追加
- [ ] `src/backends/llvm/` モジュールを作成
- [ ] LLVM コンテキスト初期化を実装

#### フェーズ 2：型マッピング（2-3 日）

- [ ] `TypeMap` を実装：YaoXiang 型 → LLVM 型
- [ ] 基本型：i32, i64, f32, f64, bool
- [ ] 複合型：struct, array, tuple
- [ ] 特殊型：Arc, ref, Option

#### フェーズ 3：命令翻訳（3-5 日）

- [ ] `codegen_instruction()` を実装
- [ ] 算術命令：add, sub, mul, div
- [ ] 制御フロー：jmp, jmp_if, ret
- [ ] 関数呼び出し：call, call_virt, call_dyn

#### フェーズ 4：DAG 収集（2-3 日）

- [ ] コード生成時に DAG 情報を収集
- [ ] 関数依存関係を記録
- [ ] 副作用推断（@IO / @Pure）
- [ ] DAG メタデータを生成

#### フェーズ 5：実行時ライブラリ（3-5 日）

- [ ] 遅延スケジューリングを実装
- [ ] DAG スケジューラを実装
- [ ] 粒度制御を実装
- [ ] ARC ランタイムを実装

#### フェーズ 6：統合とテスト（2-3 日）

- [ ] 実行時ライブラリをリンク
- [ ] エンドツーエンドテスト
- [ ] パフォーマンスベンチマーク

### 依存関係

- RFC-001：並列生成モデル（受領済み）
- RFC-008：Runtime 並行モデル（受領済み）
- RFC-009：所有権モデル（受領済み）

### リスク

1. **LLVM 統合の複雑さ**：inkwell API の深い理解が必要
2. **スケジューラと AOT コードの統合**：インターフェースを精心に設計する必要がある
3. **ABI 互換性**：解釈器ランタイムとの ABI 互換性を確保する必要がある

## 関連研究

### Lazy Task Creation (1990)[^1]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 著者 | James R. Larus, Robert H. Halstead Jr. |
| 核心 | 遅延サブタスク作成、必要时才创建、按需创建 |
| 参考価値 | 技術的基盤、遅延スケジューリング概念の起源 |

**核心思想**：タスクを直ちに作成するのではなく、遅延作成する。親タスクが子タスクの値を必要とするときになって初めて子タスクを作成する。これにより、細粒度並行タスクのパフォーマンスオーバーヘッド問題を解決する[^1]。

### Lazy Scheduling (2014)[^2]

| 属性 | 説明 |
|------|------|
| 機関 | University of Maryland |
| 著者 | Tzannes, Caragea |
| 核心 | 実行時適応スケジューリング、追加状態なし |
| 参考価値 | スケジューラ設計、適応的粒度制御 |

**核心思想**：「遅延実行」を通じて自動的に粒度を制御し、複雑な状態を維持する必要がない。システムが忙しいときはタスクが自動的にマージされ、暇なときは自動的に分割される[^2]。

### SISAL 言語[^3]

| 属性 | 説明 |
|------|------|
| 機関 | Lawrence Livermore National Laboratory (LLNL) |
| 核心 | 単一代入言語、Dataflow グラフ、暗黙的並行処理 |
| 参考価値 | 可行性の証明、パフォーマンスは Fortran に近い |

**核心貢献**：SISAL は Dataflow モデルが工業レベルアプリケーションで Fortran に近いパフォーマンスを達成できることを証明した[^3]。

### Mul-T 並列 Scheme[^4]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 核心 | Future 構築、Lazy Task Creation 実装 |
| 参考価値 | 具体的な実装の参照 |

**核心メカニズム**：
```scheme
;; Multilisp / Mul-T 構文
(let ((a (future compute-a))      ;; 直ちに future を返す
      (b (future compute-b)))      ;; 直ちに future を返す
  (join a b))                      ;; 完了を待機
```

### 比較まとめ

| 技術 | 遅延作成 | DAG 分析 | 副作用処理 | 所有権 |
|------|----------|----------|------------|--------|
| Lazy Task Creation[^1] | ✅ | ❌ | ❌ | N/A |
| Lazy Scheduling[^2] | ✅ | ❌ | ❌ | N/A |
| SISAL[^3] | ✅ | ✅ (グローバル) | N/A (単一代入) | N/A |
| Mul-T[^4] | ✅ | ❌ | ❌ | N/A |
| **YaoXiang** | ✅ | ✅ (関数内) | ✅ (暗黙的) | ✅ (ARC) |

**YaoXiang の革新**：現代的な言語機能（所有権 + 暗黙的副作用）を使用して従来設計を簡素化し、DAG 制約を関数ブロック内に限定して複雑さを軽減。

## 従来の自動並列化手法との比較

### 従来のコンパイラ：ループレベル並列化

商用コンパイラ（Intel Fortran、Oracle Fortran など）は**ループレベル自動並列化**を採用[^5]：

**核心フロー**：
```
1. 並列化可能なループを識別
2. ループ内の配列アクセスに対して依存解析を実行
3. ループ反復の間に依存関係があるかどうかを決定
4. 依存関係がなければ、マルチスレッドコードを生成
```

**依存解析技術**：

| 技術 | 説明 |
|------|------|
| **データ依存** | 2つのアクセスが同じメモリ位置にアクセスするか |
| **Use-Def** | 変数の定義と使用の関係 |
| **エイリアス解析** | ポインタが同じメモリを指しているか |

**ループが並列化可能な条件**：
```fortran
! 並列化可能
DO I = 1, N
  A(I) = C(I)
END DO

! B(I) は不可並列化（前の一反復に依存）
DO I = 2, N
  A(I) = A(I-1) + B(I)
END DO
```

### Haskell：Spark メカニズム

GHC (Glasgow Haskell Compiler) は **Spark メカニズム**を使用して純粋関数の並列化を実装[^6]：

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
- プールから spark を取って空きプロセッサコアに分配
- spark が使用されなかった場合（結果を待つ者がいない）は GC によって回収
- これにより粒度問題が解決：小さすぎる spark は破棄される

### Clean 言語：一意性型

Clean 言語は**一意性型（Uniqueness Types）**を通じて並列安全性を実装[^7]：

```clean
-- *Array は一意性を示し、安全に変更可能
modify :: *Array Int -> *Array Int
```

**核心思想**：値が単一参照であれば、並行環境で安全に修改できる。他の参照が中間状態を見ることはないため。

### プログラムスライシングと依存グラフ

**プログラム依存グラフ (PDG)** は並列性検出の基礎：

```
ノード：ステートメント
エッジ：データ依存 + 制御依存

並列性検出：
  2つのノード間に到達可能なパスがない場合 → 並列化可能
```

### 総合比較

| 方法 | 依存解析 | 粒度 | 副作用処理 | 典型シナリオ |
|------|----------|------|------------|----------|
| Intel/Oracle Fortran[^5] | 複雑な配列解析 | ループ反復 | N/A | 科学計算 |
| GHC Spark[^6] | 純粋関数の仮定 | 式 | N/A | 関数型プログラミング |
| Clean[^7] | 一意性型 | グラフ書き換え | N/A | 関数型プログラミング |
| **YaoXiang** | 所有権保証 | 関数呼び出し | 暗黙的推断 | 汎用 |

---

## 付録

### 付録 A：Rust async との詳細な比較

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| コンパイル成果物 | 状態機械 + 機械語 | 機械語 + DAG |
| 実行時 | tokio | DAG Scheduler |
| スケジューリングタイミング | コンパイル時に await 点が確定 | 実行時に必要に応じてスケジューリング |
| 並行制御 | 状態機械の状態 | DAG 依存エッジ |
| 色付き関数 | async が感染 | **L3 透過的並行処理、@block は特例** |
| アノテーション | async/await | @auto/@eager/@block |

### 付録 B：スケジューラ最適化例

**シナリオ 1：スケジューラが実行をマージできると検出**

```
元の DAG:
  compute_a() ──┐
  compute_b() ──┼──→ compute_c()

スケジューラ最適化後:
  compute_a + compute_b を単一タスクにマージ
  → スケジューリングオーバーヘッドを削減
```

**シナリオ 2：依存が使用されていない**

```
let a = expensive_compute(); // 計算した
let b = other_thing();       // a は不要
print(b);                    // b を直に返す、a をスキップ
```

### 付録 C：設計議論記録

| 意思決定 | 決定 | 日付 |
|------|------|------|
| LLVM AOT を採用 | 直接 Codegen、過度な抽象化なし | 2026-02-15 |
| DAG のスコープ | 関数ブロック内、関数間をまたがない | 2026-02-15 |

| 実行モデル | **ボトムアップ**：「結果」から逆方向に依存関係を解析、葉が並列 | 2026-03-10 |
| 孤立 DAG | コンシューマのないノードは独立して並列 | 2026-03-10 |
| 無限ループ | バックグラウンド DAG、スケジューラがスライス実行 | 2026-03-10 |
| 副作用処理 | 暗黙的 Effect System、ユーザーは感知しない | 2026-02-15 |
| 粒度制御 | 並行数制限 + 適応的 | 2026-02-16 |
| 論文引用 | Lazy Task Creation などを追加 | 2026-02-16 |

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
- [RFC-001: 並列生成モデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 並行モデル](./accepted/008-runtime-concurrency-model.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [Implicit Parallelism - Wikipedia](https://en.wikipedia.org/wiki/Implicit_parallelism)

---

## ライフサイクルと行く末

| 状態 | 場所 | 説明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 著者の草稿、レビュー提出待ち |
| **レビュー中** | `docs/design/rfc/` | コミュニティ議論とフィードバック募集中 |
| **受領済み** | `docs/design/accepted/` | 正式設計ドキュメントとして採用 |
| **拒否済み** | `docs/design/rfc/` | RFC ディレクトリに保留 |