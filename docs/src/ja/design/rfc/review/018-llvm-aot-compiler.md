---
title: "RFC-018：LLVM AOT コンパイラ設計"
---

# RFC-018：LLVM AOT コンパイラ設計

> **状態**: 審査中
> **作者**: 晨煦
> **作成日**: 2026-02-15
> **最終更新**: 2026-06-05

> **参考**:
> - [RFC-024: spawn ブロックベースの並行モデル](./accepted/024-concurrency-model.md)
> - [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md)

## 摘要

本書は YaoXiang 言語の LLVM AOT コンパイラの設計を述べたものであり、事前コンパイルにより機械語 + DAG メタデータを生成し、ランタイムが **spawn ブロック内で DAG をスケジュール**し、**ボトムアップ**の依存関係解析に基づいて実行することを目標とする。

**コアイノベーション**：
- 「関数呼び出しに遭遇したら Future を生成する」ではなく、**「結果が必要な場所」から逆方向に依存関係を解析**
- **リーフノードを優先して並列実行**、依存チェーンは順序通りに上方へトラバース
- **孤立 DAG は独立して並列**：コンシューマのないノードはメイン�.Flow をブロックしない
- **無限ループはバックグラウンド DAG**：スケジューラがスライス実行するため停止しない
- **DAG 解析は spawn ブロック内に限定**：コンパイル効率が高く、動作が制御可能

本設計は Rust async/await + tokio ランタイムモデルとは本質的に異なる：
- Rust：ユーザーが `async fn` を記述、コンパイラが状態機械を生成
- YaoXiang：ユーザーが通常の関数を記述、**コンパイラが spawn ブロック内で自動的に DAG を解析**、スケジューラがボトムアップで実行

RFC-024 の spawn ブロック並行モデルに従う：通常コードは順序実行、spawn ブロック内では DAG スケジュールによる並列実行。

## 動機

### なぜ LLVM AOT コンパイラが必要か？

現在 YaoXiang はインタープリタのみを実行バックエンドとしており、以下の問題がある：

| 問題 | 影響 |
|------|------|
| パフォーマンス瓶小路 | インタープリタ実行は機械語より 10-100x 遅い |
| デプロイが複雑 | インタープリタとランタイムの持ち運びが必要 |
| カラー関数問題 | 同期関数が並行関数を呼び出せない |

### カラー関数問題と spawn ブロック並行

**従来設計**：
- 同期関数（青色）→ 呼び出せない → 並行関数（赤色）
- デフォルトは同期、并行には `spawn` マークが必要
- 色が「伝染」：一度並行を使うと、同じ呼び出しチェーンは全て並行になる

**RFC-024 spawn ブロック並行（目標）**：
- 通常コードは順序実行、関数に色を付けない
- `spawn { ... }` ブロック内で DAG 解析、並列実行
- 呼び出し元は同期的にブロック、コールバックや `await` はない

**翻转後の設計（RFC-018）**：
- 通常コードは直接順序機械語を生成、DAG オーバーヘッドなし
- spawn ブロック内でコンパイル時に DAG 依存関係を自動解析、ランタイムでボトムアップ実行
- カラー関数問題の解決：全関数を統一、同期/並行の区別不要

### コアイノベーション：ボトムアップ実行 + spawn ブロック内 DAG

本設計のコアイノベーションは**ボトムアップ実行モデル**（spawn ブロック内に限定）にある：

```
従来呼び出し（トップダウン）：
  call fetch(url) → 実行 → 結果を返す

ボトムアップ実行：
  print(a) ← 「結果が必要な場所」から開始
       ↑
  fetch(url0) ← 依存関係を解析、逆方向に検索

  fetch(url1) ← 孤立 DAG、独立して並列実行
```

**重要な違い**：
- 「関数呼び出しに遭遇したら Future を生成する」ではない
- 「最終的な結果」から逆方向に依存関係を解析
- コンシューマのないノード（孤立）は実行しないか独立して並列実行
- 無限ループはバックグラウンド DAG、スケジューラがスライス実行

### Rust async との比較

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async モード                          │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：状態機械 + 機械語を生成                           │
│  ランタイム時：tokio スケジューラが状態機械に従ってスケジュール │
│  特徴：await 点はコンパイル時に確定、状態機械が実行を管理       │
│  粒度：関数レベル                                                │
│  ユーザー体験：async/await キーワードを書く必要がある            │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT モード                   │
├─────────────────────────────────────────────────────────────────┤
│  コンパイル時：機械語 + DAG メタデータを生成                     │
│  ランタイム時：spawn ブロック内 DAG スケジューラ、ボトムアップ実行│
│  特徴：「結果が必要な場所」から逆方向に依存関係を解析、リーフ並列 │
│  粒度：spawn ブロック内 DAG                                      │
│  ユーザー体験：通常関数、自動並列化                               │
└─────────────────────────────────────────────────────────────────┘
```

### spawn ブロック内 DAG スケジューラ

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

バックグラウンド DAG（while True）も存在）：
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
   - 内部ノード：process、compute
   - ルートノード：print

3. 実行：
   - fetch を並列実行
   - process は fetch の完了を待機
   - print は process の完了を待機
   - 孤立 compute は独立して並列実行

4. 実行済みスキップ：
   - あるノードが実行済みの場合、それを依存する後続ノードは結果を再利用可
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
→ 直接同期実行 通常コードと同じ

シナリオ 2：複数の while（自動スライス）
──────────────────────────────────────────────
main: () -> () = {
    while True { update_ui() }      # バックグラウンドタスク1
    while True { network_poll() }  # バックグラウンドタスク2
    server_loop()                   # メインタスク
}
→ 3つの独立タスク
→ スケジューラがスライス切り替え
→ 本当の並行実行

スケジューラの適応：
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
    - メイン DAG が実行を完了したら終了
    - バックグラウンド DAG は常に実行だが、スケジューラは「スライス」方式で実行
    - ループで停止することはない
```

## 提案

### コア設計

```
┌─────────────────────────────────────────────────────┐
│  コンパイル時                                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Parser  │→│DAG解析  │→│LLVM Codegen│→ 機械語  │
│  └─────────┘  └─────────┘  └─────────┘           │
│                      ↓                           │
│              生成：DAG メタデータ                      │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  ランタイム                                          │
│  ┌─────────────────────────────────────────────┐ │
│  │  DAG スケジューラライブラリ                     │ │
│  │  • 機械語のロード                              │ │
│  │  •  DAG メタデータを読み取り                     │ │
│  │  • 遅延スケジュール：呼び出しを遅延起動、按需実行│ │
│  │  •  並行/直列実行をサポート                      │ │
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
save_results(results) から開始：
    "results が必要" → parse_page に依存
    "page0 が必要" → fetch(url0) に依存
    "page1 が必要" → fetch(url1) に依存
    ...

spawn ブロック内の DAG を構築：
    fetch(url0), fetch(url1), fetch(url2) ← リーフノード
           ↓
    parse_page(page0), parse_page(page1)   ← リーフに依存
           ↓
    save_results                          ← ルートノード

フェーズ 2: リーフを並列実行（ランタイム時）
─────────────────────────────────────────
スケジューラが全リーフノードを検出：
    - fetch(url0), fetch(url1), fetch(url2) は依存なし → 並列実行
    - 並行数を制御（比如 16 個）

フェーズ 3: 上方へトラバース
─────────────────────────────────────────
parse_page が page0 を必要とするとき：
    - page0 が準備できているかチェック
    - 準備完了 → parse_page を実行
    - 未完了 → 待機、完了後继续

フェーズ 4: 孤立は独立して並列実行
─────────────────────────────────────────
ある fetch の結果が必要ない場合：
    - 「孤立 DAG」として独立実行
    - 別コア可以使用、メイン�.Flow に影響なし
```

### コンパイル生成物の構造

```rust
/// コンパイル生成物：機械語 + DAG メタデータ
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
    /// コンパイル生成物
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
        // 1. 関数体をトラバース、全呼び出しを遅延起動
        // 2. 実行待機のタスクリストを構築
        // 3. 依存順序に従ってスケジュール実行（並行数制御）
        // 4. 値が必要になった時点で実行をトリガー
        // 5. 結果を返す
    }
}
```

### DAG の例：ウェブスクレイピング

```
main 関数の DAG：
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
│ ノード             │ 副作用     │ 説明                        │
├──────────────────┼────────────┼────────────────────────────┤
│ fetch(url0)      │ @IO       │ 並行ダウンロード              │
│ fetch(url1)      │ @IO       │ 並行ダウンロード              │
│ fetch(url2)      │ @IO       │ 並行ダウンロード              │
│ parse_page       │ @Pure     │ 並列解析                     │
│ filter_links     │ @Pure     │ 並列フィルタリング            │
│ save_result      │ @IO       │ 順序保存（I/Oが順序を保証）  │
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

フェーズ 2: 並列解析
─────────────────────────────────────────
スレッド1: parse_page(page0) ──┐
スレッド2: parse_page(page1) ──┼──→ 3つの並行タスク
スレッド3: parse_page(page2) ──┘

フェーズ 3: 並列フィルタリング
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
| spawn ブロック内 | DAG 遅延スケジュール、依存なしで並列実行 |
| 循環依存 | ランタイムで検出、エラー |

### 副作用処理：暗黙的 Effect System

ユーザーの副作用処理への認知は不要、コンパイラが自動的に推論：

```
ユーザーコード：
  print("a")
  print("b")
  x = compute(1)
  y = compute(2)

コンパイラが推論：
  print → @IO（外部呼び出し）
  compute → @Pure（純粋関数）

スケジューラ実行：
  print("a") ──→ 順序（全是 @IO）
  print("b") ──→ 順序
  compute(1) ─┬─→ 並列（spawn ブロック内 DAG スケジュール）
  compute(2) ─┘
```

### 三層ランタイムとの関係

RFC-008 は Embedded / Standard / Full の三層ランタイムアーキテクチャを定義している。LLVM AOT コンパイラと三層ランタイムの対応関係（RFC-024 との整合）：

| ランタイム | LLVM AOT の動作 |
|--------|---------------|
| **Embedded** | spawn サポートなし、直接順序機械語を生成 |
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

## 权衡

### 优点

1. **パフォーマンス向上**：AOT コンパイルはインタープリタ実行より 10-100x 高速
2. **カラー関数の解決**：関数に色を付けない、spawn ブロック内で並列化
3. **統一ランタイム**：インタープリタと LLVM が同じスケジューラを共有
5. **暗黙的副作用**：ユーザーの認知不要、コンパイラが自動処理
6. **所有権の安全性**：Rust スタイルの所有権モデルに依存、データ競合なし

### 欠点

1. **実装複雑度**：LLVM 統合経験が必要
2. **コンパイル時間**：AOT コンパイルはインタープリタより遅い
3. **デバッグが難しい**：AOT コードのデバッグはインタープリタより複雑

### RFC 設計との整合性

| RFC | 整合性 |
|-----|--------|
| RFC-024 spawn ブロック並行モデル | ✅ DAG 解析は spawn ブロック内に限定 |
| RFC-008 ランタイムアーキテクチャ | ✅ ランタイムスケジューラ設計が一致 |
| RFC-009 所有権モデル | ✅ ARC ランタイムが正しく実装 |

## 代替案

| 方案 | 記述 | 为什么不選 |
|------|------|-----------|
| インタープリタのみ | AOT が不要 | パフォーマンス不足、spawn ブロック並列化サポートなし |
| 純粋静的コンパイル | ランタイムスケジュールなし | 遅延スケジュールはランタイムで必要 |
| 外部 LLVM runtime をリンク | LLVM の runtime を使用 | 追加の依存関係が必要 |

## 実装戦略

### フェーズ分け

#### フェーズ 1：基本フレームワーク（1-2 日）

- [ ] `Cargo.toml` に inkwell 依存関係を追加
- [ ] `src/backends/llvm/` モジュールの作成
- [ ] LLVM コンテキスト初期化の実装

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

- [ ] コード生成時に spawn ブロック内の DAG 情報を収集
- [ ] spawn ブロック内の関数依存関係を記録
- [ ] 副作用推論（@IO / @Pure）
- [ ] spawn ブロック内 DAG メタデータを生成

#### フェーズ 5：ランタイムライブラリ（3-5 日）

- [ ] 遅延スケジュールの実装
- [ ] DAG スケジューラの実装
- [ ] 粒度制御の実装
- [ ] ARC ランタイムの実装

#### フェーズ 6：統合とテスト（2-3 日）

- [ ] ランタイムライブラリをリンク
- [ ] エンドツーエンドテスト
- [ ] パフォーマンスベンチマーク

### 依存関係

- RFC-024：spawn ブロックベースの並行モデル（受理済み）
- RFC-008：Runtime 並行モデル（受理済み）
- RFC-009：所有権モデル（受理済み）

### リスク

1. **LLVM 統合複雑度**：inkwell API の深い理解が必要
2. **スケジューラと AOT コードの統合**：インターフェースを精心設計する必要あり
3. **ABI 互換性**：インタープリタのランタイム ABI との互換性を確保する必要あり

## 関連研究

### Lazy Task Creation (1990)[^1]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 作者 | James R. Larus, Robert H. Halstead Jr. |
| コア | 遅延で子タスクを作成、按需で作成 |
| 参考価値 | 技術基盤、遅延スケジュールの概念の起源 |

**コアアイデア**：タスクを即座に作成するのではなく、遅延して作成する。親タスクが子タスクの値を必要とした時点で子タスクを作成する。これにより、細粒度並行タスクのパフォーマンスオーバーヘッド問題を解決[^1]。

### Lazy Scheduling (2014)[^2]

| 属性 | 説明 |
|------|------|
| 機関 | University of Maryland |
| 作者 | Tzannes, Caragea |
| コア | ランタイム適応スケジュール、追加状態なし |
| 参考価値 | スケジューラ設計、適応的粒度制御 |

**コアアイデア**：「遅延実行」を通じて粒度を自動制御し、複雑な状態を維持する必要はない。システム繁忙時はタスクが自動マージ、暇時は自動分割[^2]。

### SISAL 言語[^3]

| 属性 | 説明 |
|------|------|
| 機関 | Lawrence Livermore National Laboratory (LLNL) |
| コア | 単一代入言語、Dataflow グラフ、暗黙的並列化 |
| 参考価値 | 可行性の証明、Floating Fortran に匹敵するパフォーマンス |

**コア貢献**：SISAL は Dataflow モデルが産業規模アプリケーションで Floating Fortran に匹敵するパフォーマンスを達成できることを証明[^3]。

### Mul-T 並列 Scheme[^4]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| コア | Future 構築、Lazy Task Creation 実装 |
| 参考価値 | 具体的な実装の参照 |

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
| **YaoXiang** | ✅ | ✅ (spawn ブロック内) | ✅ (暗黙的) | ✅ (ARC) |

**YaoXiang のイノベーション**：現代的な言語機能（所有権 + 暗黙的副作用）を使用して従来設計を簡素化し、DAG を spawn ブロック内に制約して複雑さを軽減。

## 従来の自動並列化手法との比較

### 従来のコンパイラ：ループレベル並列化

商用コンパイラ（Intel Fortran、Oracle Fortran）は**ループレベル自動並列化**を採用[^5]：

**コアフロー**：
```
1. 並列化可能なループを識別
2. ループ内の配列アクセスに対して依存関係解析を実行
3. ループ反復間に依存関係があるかどうかを決定
4. 依存関係がない場合、マルチスレッドコードを生成
```

**依存関係解析技術**：

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

!  B(I) 不可並列化（前の反復に依存）
DO I = 2, N
  A(I) = A(I-1) + B(I)
END DO
```

### Haskell：Spark メカニズム

GHC (Glasgow Haskell Compiler) は純粋関数の並列化のために **Spark メカニズム**を採用[^6]：

```haskell
-- rpar: 並列実行、spark を作成
-- rseq: 直列実行、完了を待機

example = do
  a <- rpar (f x)   -- spark を作成、f x を並列実行
  b <- rpar (g y)   -- spark を作成、g y を並列実行
  rseq a            -- a の完了を待機
  rseq b            -- b の完了を待機
  return (a, b)
```

**Spark プールメカニズム**：
- プールから spark を取得して空き処理コアに分配
- spark が使用されていない場合（誰も結果を待っていない）、GC によって回収される
- これは粒度問題を解決：小さすぎる spark は破棄される

### Clean 言語：一意性型

Clean 言語は**一意性型（Uniqueness Types）**を通じて並列安全性を実現[^7]：

```clean
-- *Array は一意性を表し、安全に変更可能
modify :: *Array Int -> *Array Int
```

**コアアイデア**：値が単一参照の場合、並列環境で安全に修改できる。他の参照が中間状態を見ることはないため。

### プログラムスライシングと依存グラフ

**プログラム依存グラフ (PDG)** は並列性検出の基礎：

```
ノード：文
エッジ：データ依存 + 制御依存

並列性検出：
  2つのノード間に到達可能なパスがない場合 → 並列化可能
```

### 包括的比較

| 方法 | 依存関係解析 | 粒度 | 副作用処理 | 典型シナリオ |
|------|----------|------|------------|----------|
| Intel/Oracle Fortran[^5] | 複雑な配列解析 | ループ反復 | N/A | 科学計算 |
| GHC Spark[^6] | 純粋関数仮定 | 式 | N/A | 関数型プログラミング |
| Clean[^7] | 一意性型 | グラフ書き換え | N/A | 関数型プログラミング |
| **YaoXiang** | 所有権で保証 | 関数呼び出し | 暗黙的推論 | 汎用 |

---

## 付録

### 付録 A：Rust async との詳細な比較

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| コンパイル生成物 | 状態機械 + 機械語 | 機械語 + DAG |
| ランタイム | tokio | DAG Scheduler |
| スケジュールタイミング | コンパイル時に await 点が確定 | ランタイムで按需スケジュール（spawn ブロック内） |
| 並行制御 | 状態機械状態 | DAG 依存エッジ |
| カラー関数 | async が伝染 | **関数に色を付けない、spawn ブロック内で並列化** |
| アノテーション | async/await | なし（spawn ブロックが唯一の並行プリミティブ） |

### 付録 B：スケジューラ最適化例

**シナリオ 1：スケジューラが実行をマージできることを検出**

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
a = expensive_compute()  // 計算済み
b = other_thing()        // a が不要
print(b)                 // 直接 b を返す、a をスキップ
```

### 付録 C：設計議論記録

| 決定 | 結論 | 日付 |
|------|------|------|
| LLVM AOT を採用 | 直接 Codegen、過度な抽象化なし | 2026-02-15 |
| DAG スコープ | spawn ブロック内、spawn ブロックをまたがない | 2026-06-05 |
| 実行モデル | **ボトムアップ**：「結果」から逆方向に依存関係を解析、リーフを並列化 | 2026-03-10 |
| 孤立 DAG | コンシューマのないノードは独立して並列化 | 2026-03-10 |
| 無限ループ | バックグラウンド DAG、スケジューラがスライス実行 | 2026-03-10 |
| 副作用処理 | 暗黙的 Effect System、ユーザーの認知なし | 2026-02-15 |
| 粒度制御 | 並行数制限 + 適応的 | 2026-02-16 |
| 論文引用 | Lazy Task Creation などを追加 | 2026-02-16 |
| 並行モデル整合 | RFC-024 spawn ブロック並行モデルと整合、旧アノテーションを削除 | 2026-06-05 |

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
| **草案** | `docs/design/rfc/` | 作者の草案、社区審査への提出を待機 |
| **審査中** | `docs/design/rfc/` | 社区議論とフィードバックを募集中 |
| **受理済み** | `docs/design/rfc/accepted/` | 正式な設計文書となる |
| **拒否済み** | `docs/design/rfc/` | RFC ディレクトリに保持 |

> 現在の状態：**審査中** — RFC-024 spawn ブロック並行モデルと整合済み