---
title: "RFC-018：LLVM AOT コンパイラ設計"
status: "レビュー中"
author: "晨煦"
created: "2026-02-15"
updated: "2026-06-05"
---

# RFC-018：LLVM AOT コンパイラ設計

> **参考**:
> - [RFC-024: spawn ブロックベースの並行モデル](./accepted/024-concurrency-model.md)
> - [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md)

## 概要

このドキュメントは YaoXiang 言語の LLVM AOT コンパイラを設計するものであり、事前コンパイルによりマシンコード + DAG メタデータを生成し、runtime が **spawn ブロック内で DAG をスケジュール**し、**ボトムアップ**の依存関係解析に基づいて実行することを目標とする。

**コアイノベーション**：
- 「関数呼び出しに遭遇したら Future を生成する」ではなく、**「結果が必要な場所」から逆方向に依存関係を解析**
- **リーフノードを優先して並行実行**、依存チェーンは順序通りに上方へ辿る
- **孤立 DAG は独立して並行**：コンシューマのないノードはメイン�フローをブロックしない
- **無限ループはバックグラウンド DAG として**：スケジューラがスライス実行し、フリーズしない
- **DAG 解析は spawn ブロック内に限定**：コンパイル効率向上、動作が制御可能

これは Rust async/await + tokio runtime パターンと本質的に異なる：
- Rust：ユーザーが `async fn` を書き、コンパイラが状態機械を生成
- YaoXiang：ユーザーが通常の関数を書き、**コンパイラが spawn ブロック内で自動的に DAG を解析**、スケジューラがボトムアップで実行

RFC-024 の spawn ブロック並行モデルに従う：通常のコードは順序実行、spawn ブロック内のみ DAG スケジュールで並行。

## 動機

### なぜ LLVM AOT コンパイラが必要か？

現在 YaoXiang はインタープリタのみを実行バックエンドとして持っており、以下の問題がある：

| 問題 | 影響 |
|------|------|
| パフォーマンスボトルネック | インタープリタ実行はマシンコードより 10-100 倍遅い |
| デプロイが複雑 | インタープリタと runtime の携带が必要 |
| カラー関数問題 | 同期関数が並行関数を呼び出せない |

### カラー関数問題と spawn ブロック並行

**従来の設計**：
- 同期関数（青色）→ 呼び出せない → 並行関数（赤色）
- 同期がデフォルト、並行には `spawn` マークが必要
- 色が「伝染」する：一度並行を使うと、同じ呼び出しチェーンはすべて並行になる

**RFC-024 spawn ブロック並行（目標）**：
- 通常コードは順序実行、関数着色なし
- `spawn { ... }` ブロック内で DAG 解析、並行実行
- 呼び出し元は同期ブロック、コールバックも `await` もない

**反転後の設計（RFC-018）**：
- 通常コードは直接順序マシンコードを生成、DAG オーバーヘッドなし
- spawn ブロック内でコンパイル時に DAG 依存関係を自動的に解析、runtime でボトムアップ実行
- カラー関数問題を解決：すべての関数を統一、同期/並行の区別不要

### コアイノベーション：ボトムアップ実行 + spawn ブロック内 DAG

本設計のコアイノベーションは **ボトムアップ実行モデル**（spawn ブロック内に限定）にある：

```
従来の呼び出し（トップダウン）：
  call fetch(url) → 実行 → 結果を返す

ボトムアップ実行：
  print(a) ← 「結果が必要な場所」から開始
       ↑
  fetch(url0) ← 依存関係を解析、逆方向に検索

  fetch(url1) ← 孤立 DAG、独立並行実行
```

**重要な違い**：
- 「関数呼び出しに遭遇したら Future を生成する」ではない
- 「最終的な結果が必要」から逆方向に依存関係を解析
- コンシューマのないノード（孤立）は実行しないか独立して並行実行
- 無限ループはバックグラウンド DAG として、スケジューラがスライス実行

### Rust async との比較

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async モード                          │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：状態機械 + マシンコードを生成                    │
│  ランタイム時：tokio スケジューラが状態機械に応じてスケジュール  │
│  特徴：await ポイントはコンパイル時に確定、状態機械が実行を管理  │
│  粒度：関数レベル                                               │
│  ユーザー体験：async/await キーワードを書く必要がある           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT モード                   │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：マシンコード + DAG メタデータを生成              │
│  ランタイム時：spawn ブロック内の DAG スケジューラ、ボトムアップ実行│
│  特徴：「結果が必要な場所」から逆方向に依存関係を解析、リーフノード並行│
│  粒度：spawn ブロック内の DAG                                  │
│  ユーザー体験：通常の関数、自動的に並行化                       │
└─────────────────────────────────────────────────────────────────┘
```

### spawn ブロック内の DAG スケジューラ

```
spawn ブロック内の DAG ビュー：

        print(result) ─────────────────────────┐
           │                                    │
    ┌──────┴──────┐                             │
    │             │                             │
process(a)   process(b)                        │
    │             │                             │
compute(x)   compute(y)  ←── 孤立 DAG ──────────┤
    │                                           │
fetch(url0)  fetch(url1)  fetch(url2)          │
    (実行済み)                                    │

同時にバックグラウンド DAG（while True）もある：
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

2. spawn ブロック内の DAG を構築：
   - リーフノード：fetch（依存なし）
   - 内部ノード：process, compute
   - ルートノード：print

3. 実行：
   - fetch を並行実行
   - process は fetch の完了を待機
   - print は process の完了を待機
   - 孤立 compute は独立して並行

4. 実行済みはスキップ：
   - あるノードが実行済みの場合、それらに依存する後続ノードは結果を再利用可
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
→ 無限ループは1つだけ
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
→ 真の並行実行

スケジューラの自适应：
──────────────────────────────────────────────
if タスク数 == 1:
    直接実行（同期）
else:
    スライススケジュール（並行）
```

**バックグラウンド DAG 処理**：
```
メイン DAG（終了あり）：
    fetch → process → print → 終了

バックグラウンド DAG（無限ループ）：
    while True → update_ui → fetch_new → process → 最初に戻る

スケジューラ：
    - メイン DAG は実行完了後終了
    - バックグラウンド DAG は常に実行だが、スケジューラは「スライス」方式で実行
    - ループで凍りつかない
```

## 提案

### コア設計

```
┌─────────────────────────────────────────────────────┐
│  コンパイル時                                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Parser  │→│DAG解析  │→│LLVM Codegen│→ マシンコード│
│  └─────────┘  └─────────┘  └─────────┘           │
│                      ↓                           │
│              生成：DAG メタデータ                      │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  ランタイム                                          │
│  ┌─────────────────────────────────────────────┐ │
│  │  DAG スケジューラライブラリ                       │ │
│  │  • マシンコードのロード                          │ │
│  │  • DAG メタデータを読み取り                      │ │
│  │  • 遅延スケジュール：呼び出しを停止、需要に応じて実行│ │
│  │  • 並行/直列実行をサポート                       │ │
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

Phase 1: ボトムアップ解析（コンパイル時）
─────────────────────────────────────────
save_results(results) から開始：
    "results が必要" → parse_page(results) に依存
    "page0 が必要" → fetch(url0) に依存
    "page1 が必要" → fetch(url1) に依存
    ...

spawn ブロック内の DAG を構築：
    fetch(url0), fetch(url1), fetch(url2) ← リーフノード
           ↓
    parse_page(page0), parse_page(page1)   ← リーフに依存
           ↓
    save_results                          ← ルートノード

Phase 2: リーフの並行実行（ランタイム時）
─────────────────────────────────────────
スケジューラがすべてのリーフノードを見つける：
    - fetch(url0), fetch(url1), fetch(url2) は依存なし → 並行実行
    - 並行数を制御（例：最大 16 個）

Phase 3: 上方へ辿る
─────────────────────────────────────────
parse_page が page0 を必要とする場合：
    - page0 が準備完了かチェック
    - 準備完了 → parse_page を実行
    - 未完了 → 待機、完了後続行

Phase 4: 孤立は独立して並行
─────────────────────────────────────────
ある fetch の結果を誰も必要としていない場合：
    - 「孤立 DAG」として独立実行
    - 別のコアを使用可能、メイン�フロー不影响
```

### コンパイル成果物構造

```rust
/// コンパイル成果物：マシンコード + DAG メタデータ
pub struct CompiledArtifact {
    /// LLVM コンパイルのマシンコード（ELF/Mach-O/COFF）
    machine_code: Vec<u8>,

    /// DAG メタデータ：関数依存関係を記述
    dag: DAGMetadata,

    /// エントリポイントテーブル
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

### ランタイムスケジューラインターフェース

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
            max_parallelism: num_workers * 2, // 自適応粒度制御
        }
    }
}

impl DAGScheduler for DefaultDAGScheduler {
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue {
        // 1. 関数体を辿り、すべての呼び出しを停止
        // 2. 実行待ちタスクリストを構築
        // 3. 依存順序でスケジュール実行（並行数を制御）
        // 4. 値が必要になったら実行をトリガー
        // 5. 結果を返す
    }
}
```

### DAG の例：Web クローラ

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
│ ノード              │ 副作用     │ 説明                        │
├──────────────────┼────────────┼────────────────────────────┤
│ fetch(url0)      │ @IO       │ 並行ダウンロード               │
│ fetch(url1)      │ @IO       │ 並行ダウンロード               │
│ fetch(url2)      │ @IO       │ 並行ダウンロード               │
│ parse_page       │ @Pure     │ 並行解析                      │
│ filter_links     │ @Pure     │ 並行フィルタリング             │
│ save_result      │ @IO       │ 順序保存（I/Oが順序を保証）    │
│ print            │ @IO       │ 最後に実行                    │
└──────────────────┴────────────┴────────────────────────────┘
```

### スケジューラ実行フェーズ

```
Phase 1: 並行ダウンロード
─────────────────────────────────────────
スレッド1: fetch(url0) ──────────┐
スレッド2: fetch(url1) ─────────┼──→ 3つの並行タスク（最大並行数を制限）
スレッド3: fetch(url2) ──────────┘

Phase 2: 並行解析
─────────────────────────────────────────
スレッド1: parse_page(page0) ──┐
スレッド2: parse_page(page1) ──┼──→ 3つの並行タスク
スレッド3: parse_page(page2) ──┘

Phase 3: 並行フィルタリング
─────────────────────────────────────────
スレッド1: filter_links(result0) ──┐
スレッド2: filter_links(result1) ──┼──→ 3つの並行タスク
スレッド3: filter_links(result2) ──┘

Phase 4: 順序保存
─────────────────────────────────────────
スレッド1: save_result(result0) → 完了待機
スレッド1: save_result(result1) → 完了待機
スレッド1: save_result(result2) → 完了待機

Phase 5: 出力
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
├── dag.rs           # DAG 解析と生成
├── scheduler.rs      # ランタイムスケジューラ
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

各 `BytecodeInstr` は対応する LLVM IR 命令に直接翻訳：

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

| シナリオ | スケジュール戦略 |
|------|----------|
| 通常コード（spawn ブロック外） | 順序実行、DAG オーバーヘッドなし |
| spawn ブロック内 | DAG 遅延スケジュール、依存なしで並行実行 |
| 循環依存 | ランタイムで検出、エラー |

### 副作用処理：暗黙 Effect System

ユーザーは副作用処理を認識せず、コンパイラが自動的に推論：

```
ユーザーコード：
  print("a")
  print("b")
  x = compute(1)
  y = compute(2)

コンパイラ推論：
  print → @IO（外部呼び出し）
  compute → @Pure（純粋関数）

スケジューラ実行：
  print("a") ──→ 順序（すべて @IO）
  print("b") ──→ 順序
  compute(1) ─┬─→ 並行（spawn ブロック内 DAG スケジュール）
  compute(2) ─┘
```

### 三層ランタイムとの関係

RFC-008 は Embedded / Standard / Full の三層ランタイムアーキテクチャを定義している。LLVM AOT コンパイラと三層ランタイムの対応関係（RFC-024 に整合）：

| ランタイム | LLVM AOT の動作 |
|--------|---------------|
| **Embedded** | spawn サポートなし、直接順序マシンコードを生成 |
| **Standard** | spawn ブロックをサポート、spawn ブロック内 DAG + 単一スレッドスケジュール（num_workers=1） |
| **Full** | spawn ブロックをサポート、spawn ブロック内 DAG + マルチスレッドスケジュール（num_workers>1）、WorkStealing をサポート |

### スケジューラインターフェース設計

```rust
/// 副作用タグ
pub enum EffectTag {
    /// 純粋関数、副作用なし
    Pure,
    /// I/O 副作用あり
    IO,
}

/// DAG スケジューラ trait
pub trait DAGScheduler: Send + Sync {
    /// spawn ブロック内の DAG をスケジュール実行
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue;

    /// 単一関数実行（通常コード、順序実行）
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}
```

## トレードオフ

### メリット

1. **パフォーマンス向上**：AOT コンパイルはインタープリタ実行より 10-100 倍高速
2. **カラー関数の解決**：関数着色なし、spawn ブロック内並行
3. **統一ランタイム**：インタープリタと LLVM が同一スケジューラを共有
5. **暗黙の副作用**：ユーザーは認識不要、コンパイラが自動処理
6. **所有権の安全性**：Rust スタイル所有権モデルに依存、データ競合なし

### デメリット

1. **実装複雑度**：LLVM 統合経験が必要
2. **コンパイル時間**：AOT コンパイルはインタープリタより遅い
3. **デバッグ困難**：AOT コードのデバッグはインタープリタより複雑

### RFC 設計との整合性

| RFC | 整合性 |
|-----|--------|
| RFC-024 spawn ブロック並行モデル | ✅ DAG 解析は spawn ブロック内に限定 |
| RFC-008 ランタイムアーキテクチャ | ✅ ランタイムスケジューラ設計が一致 |
| RFC-009 所有権モデル | ✅ ARC ランタイムが正しく実装 |

## 代替案

| 案 | 説明 | 選択しない理由 |
|------|------|-----------|
| インタープリタのみ使用 | AOT が不要 | パフォーマンス不足、spawn ブロック並行サポートなし |
| 純粋静的コンパイル | ランタイムスケジュールなし | 遅延スケジュールはランタイムで必要 |
| 外部 LLVM runtime をリンク | LLVM の runtime を使用 | 追加の依存関係が必要 |

## 実装戦略

### フェーズ分け

#### フェーズ 1：基本フレームワーク（1-2 日）

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

- [ ] コード生成時に spawn ブロック内の DAG 情報を収集
- [ ] spawn ブロック内の関数依存関係を記録
- [ ] 副作用推論（@IO / @Pure）
- [ ] spawn ブロック内 DAG メタデータを生成

#### フェーズ 5：ランタイムライブラリ（3-5 日）

- [ ] 遅延スケジュールを実装
- [ ] DAG スケジューラを実装
- [ ] 粒度制御を実装
- [ ] ARC ランタイムを実装

#### フェーズ 6：統合とテスト（2-3 日）

- [ ] ランタイムライブラリをリンク
- [ ] エンドツーエンドテスト
- [ ] パフォーマンスベンチマーク

### 依存関係

- RFC-024：spawn ブロックベースの並行モデル（承認済み）
- RFC-008：Runtime 並行モデル（承認済み）
- RFC-009：所有権モデル（承認済み）

### リスク

1. **LLVM 統合複雑度**：inkwell API の深い理解が必要
2. **スケジューラと AOT コードの統合**：インターフェースを精心に設計する必要
3. **ABI 互換性**：インタープリタランタイムとの ABI 互換性を確保する必要

## 関連作業

### Lazy Task Creation (1990)[^1]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 著者 | James R. Larus, Robert H. Halstead Jr. |
| コア | 遅延で子タスクを作成、需要に応じて作成 |
| 参考価値 | 技術的基盤、遅延スケジュール概念の起源 |

**コアアイデア**：タスクを即座に作成するのではなく、遅延作成する。親タスクが子タスクの値を必要とするときにだけ、子タスクを作成する。これは細粒度並行タスクのパフォーマンスオーバーヘッド問題を解決する[^1]。

### Lazy Scheduling (2014)[^2]

| 属性 | 説明 |
|------|------|
| 機関 | University of Maryland |
| 著者 | Tzannes, Caragea |
| コア | ランタイム自适应スケジュール、追加状態なし |
| 参考価値 | スケジューラ設計、自適応粒度制御 |

**コアアイデア**：「遅延実行」を通じて粒度を自動的に制御し、複雑な状態を維持する必要がない。システムが忙しい時はタスクが自動的にマージされ、暇な時は自動的に分割される[^2]。

### SISAL 言語[^3]

| 属性 | 説明 |
|------|------|
| 機関 | Lawrence Livermore National Laboratory (LLNL) |
| コア | 単一代入言語、Dataflow グラフ、暗黙の並行 |
| 参考価値 | 可行性の証明、Fortran に匹敵するパフォーマンス |

**コア貢献**：SISAL は Dataflow モデルがインダストリアルレベルのアプリケーションで Fortran に匹敵するパフォーマンスを達成できることを証明した[^3]。

### Mul-T 並列 Scheme[^4]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| コア | Future 構築、Lazy Task Creation 実装 |
| 参考価値 | 具体的な実装の参考 |

**コアメカニズム**：
```scheme
;; Multilisp / Mul-T 構文
(let ((a (future compute-a))      ;; 即座に future を返す
      (b (future compute-b)))      ;; 即座に future を返す
  (join a b))                      ;; 完了を待機
```

### 比較まとめ

| 技術 | 遅延作成 | DAG 解析 | 副作用処理 | 所有権 |
|------|----------|----------|------------|--------|
| Lazy Task Creation[^1] | ✅ | ❌ | ❌ | N/A |
| Lazy Scheduling[^2] | ✅ | ❌ | ❌ | N/A |
| SISAL[^3] | ✅ | ✅ (グローバル) | N/A (単一代入) | N/A |
| Mul-T[^4] | ✅ | ❌ | ❌ | N/A |
| **YaoXiang** | ✅ | ✅ (spawn ブロック内) | ✅ (暗黙) | ✅ (ARC) |

**YaoXiang のイノベーション**：モダンな言語機能（所有権 + 暗黙の副作用）を使用して従来設計を簡素化し、DAG を spawn ブロック内に制約して複雑度を低下。

## 従来自動並行化手法との比較

### 従来コンパイラ：ループレベル並列化

商用コンパイラ（Intel Fortran、Oracle Fortran）は**ループレベル自動並列化**を採用[^5]：

**コアフロー**：
```
1. 並行化可能なループを識別
2. ループ内の配列アクセスに対して依存関係解析を実行
3. ループ反復間に依存があるかを決定
4. 依存がない場合、マルチスレッドコードを生成
```

**依存解析技術**：

| 技術 | 説明 |
|------|------|
| **データ依存** | 2つのアクセスが同じメモリ位置にアクセスするか |
| **Use-Def** | 変数の定義と使用の関係 |
| **エイリアス解析** | ポインタが同じメモリを指しているか |

**ループが並列化可能な条件**：
```fortran
! 並行化可能
DO I = 1, N
  A(I) = C(I)
END DO

! B(I) + は並列化不可（前の反復に依存）
DO I = 2, N
  A(I) = A(I-1) + B(I)
END DO
```

### Haskell：Spark メカニズム

GHC (Glasgow Haskell Compiler) は**Spark メカニズム**を使用して純粋関数の並列化を実現[^6]：

```haskell
-- rpar: 並行実行、spark を作成
-- rseq: 直列実行、完了を待機

example = do
  a <- rpar (f x)   -- spark を作成、f x を並行実行
  b <- rpar (g y)   -- spark を作成、g y を並行実行
  rseq a            -- a の完了を待機
  rseq b            -- b の完了を待機
  return (a, b)
```

**Spark プールメカニズム**：
- プールから spark を取り出し、空いている処理コアに分配
- spark が使用されない場合（結果を待つ人がいない）、GC で回収される
- これにより粒度問題を解決：小さすぎる spark は破棄される

### Clean 言語：一意性型

Clean 言語は**一意性型（Uniqueness Types）**を通じて並列安全性を実現[^7]：

```clean
-- *Array は一意性を表す、安全に変更可能
modify :: *Array Int -> *Array Int
```

**コアアイデア**：値が一意に参照されている場合、並行環境で安全に修改できる。他の参照が中間状態を見ることがないため。

### プログラムスライシングと依存グラフ

**プログラム依存グラフ (PDG)** は並列性検出の基盤：

```
ノード：文
エッジ：データ依存 + 制御依存

並列性検出：
  2つのノード間に到達可能なパスがない場合 → 並行化可能
```

### 総合比較

| 方法 | 依存解析 | 粒度 | 副作用処理 | 典型シナリオ |
|------|----------|------|------------|----------|
| Intel/Oracle Fortran[^5] | 複雑な配列解析 | ループ反復 | N/A | 科学計算 |
| GHC Spark[^6] | 純粋関数の仮定 | 式 | N/A | 関数型プログラミング |
| Clean[^7] | 一意性型 | グラフ書き換え | N/A | 関数型プログラミング |
| **YaoXiang** | 所有権で保証 | 関数呼び出し | 暗黙推論 | 汎用 |

---

## 付録

### 付録 A：Rust async との比較詳細

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| コンパイル成果物 | 状態機械 + マシンコード | マシンコード + DAG |
| ランタイム | tokio | DAG Scheduler |
| スケジュールタイミング | コンパイル時に await ポイントを確定 | ランタイムで需要に応じてスケジュール（spawn ブロック内） |
| 並行制御 | 状態機械の状態 | DAG 依存エッジ |
| カラー関数 | async の伝染 | **関数着色なし、spawn ブロック内並行** |
| 注釈 | async/await | なし（spawn ブロックが唯一の並行プリミティブ） |

### 付録 B：スケジューラ最適化例

**シナリオ 1：スケジューラが実行のマージ 가능を検出**

```
元の DAG:
  compute_a() ──┐
  compute_b() ──┼──→ compute_c()

スケジューラ最適化後:
  compute_a + compute_b を単一タスクにマージ
  → スケジュールオーバーヘッドを削減
```

**シナリオ 2：依存が使用されていない**

```
a = expensive_compute()  // 計算済み
b = other_thing()        // a は不要
print(b)                 // b を直接返す、a をスキップ
```

### 付録 C：設計議論記録

| 意思決定 | 決定 | 日付 |
|------|------|------|
| LLVM AOT を採用 | 直接 Codegen、過度な抽象化なし | 2026-02-15 |
| DAG スコープ | spawn ブロック内、spawn ブロックをまたがない | 2026-06-05 |
| 実行モデル | **ボottomアップ**：結果から逆方向に依存関係を解析、リーフを並行実行 | 2026-03-10 |
| 孤立 DAG | コンシューマのないノードは独立して並行実行 | 2026-03-10 |
| 無限ループ | バックグラウンド DAG、スケジューラがスライス実行 | 2026-03-10 |
| 副作用処理 | 暗黙 Effect System、ユーザーは認識不要 | 2026-02-15 |
| 粒度制御 | 並行数制限 + 自適応 | 2026-02-16 |
| 論文引用 | Lazy Task Creation などを追加 | 2026-02-16 |
| 並行モデルとの整合 | RFC-024 spawn ブロック並行モデルと整合、旧注釈を削除 | 2026-06-05 |

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
- [RFC-024: spawn ブロックベースの並行モデル](./accepted/024-concurrency-model.md)
- [RFC-008: Runtime 並行モデル](./accepted/008-runtime-concurrency-model.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [Implicit Parallelism - Wikipedia](https://en.wikipedia.org/wiki/Implicit_parallelism)

---

## ライフサイクルと行き先

| 状態 | 場所 | 説明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作成者草案、レビュー提出待ち |
| **レビュー中** | `docs/design/rfc/` | コミュニティ議論とフィードバックを募集中 |
| **承認済み** | `docs/design/rfc/accepted/` | 正式設計ドキュメントとして採用 |
| **拒否済み** | `docs/design/rfc/` | RFC ディレクトリに保持 |

> 現在の状態：**レビュー中** — RFC-024 spawn ブロック並行モデルと整合済み