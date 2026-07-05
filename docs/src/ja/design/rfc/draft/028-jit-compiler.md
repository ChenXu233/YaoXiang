---
title: "RFC-028：JIT コンパイラ — VM 内マルチレベル実行エンジン"
status: "ドラフト"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
issue: "#101"
---

# RFC-028：JIT コンパイラ — VM 内マルチレベル実行エンジン

> **参考**:
> - [RFC-018：LLVM AOT コンパイラ設計](../review/018-llvm-aot-compiler.md)
> - [RFC-024：spawn ブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime 並行モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)

## 概要

本文書では、YaoXiang の VM バックエンドに Cranelift JIT コンパイラを導入し、VM を純粋なインタープリタから**マルチレベル実行エンジン**へ昇格させることを提案する。コールドコードはインタープリタで実行し、ホット関数は Cranelift でネイティブコードにコンパイルされる。JIT パスは RFC-018 の LLVM AOT パスと IR 正規化パスを共有し、Cranelift は JIT の高速コンパイルを担当し、LLVM は AOT の深い最適化を担当する。互いの長所を活用する。

**核心的な位置付け：JIT は VM に奉仕するものであり、VM を代替するものではない。**

## 動機

### なぜ JIT が必要なのか？

現在の VM バックエンドは純粋なインタープリタであり、実行速度はネイティブコードより 10〜100 倍遅い。開発時には頻繁にテストやスクリプト、ローカルデバッグを実行する。これらのシナリオでは AOT の極限的な最適化は不要だが、インタープリタより明確に高速な実行が求められる。

### なぜ LLVM AOT だけでは不十分なのか？

LLVM AOT のコンパイルは時間がかかる（秒単位）ため、開発イテレーションには適していない。開発には「変更して即実行する」体験が求められる：1 行のコードを変更 → 再実行 → ほぼ即座に結果を確認。Cranelift JIT なら 1 関数あたり 1〜5ms しかかからず、ユーザーにはコンパイル遅延を感じさせない。

### なぜ Cranelift であって LLVM ORC JIT ではないのか？

| 次元 | Cranelift JIT | LLVM ORC JIT |
|------|--------------|--------------|
| コンパイル速度 | 1〜5ms/関数 | 10〜100ms/関数 |
| 依存サイズ | 小 | 大（完全な LLVM が必要） |
| コード品質 | LLVM -O2 の 70〜80% | 極めて高い |
| 適用シナリオ | 開発デバッグ、高速イテレーション | 適用不可（本稿のトレードオフ参照） |

Cranelift はコンパイルが速く、コード品質も十分。LLVM は AOT のオフライン深い最適化に任せる。1 つのツールが 1 つの仕事をきちんとやる。

## 提案

### コアアーキテクチャ

```
VM 実行エンジン
├── インタープリタ層
│   ├── バイトコード命令を実行
│   ├── ホットネスデータを収集（invocation count + loop backedge count）
│   └── 閾値到達 → コンパイルタスクを送信
│
├── JIT コンパイル層（Cranelift Backend）
│   ├── コンパイルキュー（バックグラウンドスレッド、インタープリタをブロックしない）
│   ├── IR → 正規化 → Cranelift IR → ネイティブコード
│   └── RFC-018 §4.0 の IR 正規化パス（スタック→SSA）を再利用
│
├── コードキャッシュ
│   ├── 関数テーブル：関数 ID → {インタープリタ入口, JIT 入口(オプション)}
│   ├── コンパイル済み関数の入口をアトミックに差し替え
│   └── モジュールごとにグループ化（ホットリロード用インターフェースを预留）
│
└── ホットネス分析
    ├── 関数ごとの呼び出しカウント + ループバックエッジカウント
    ├── 定期減衰（一括ウォームアップによるコンパイル発火を防止）
    └── 3 段階のホットネス：Cold → Warm → Hot → Compiled
```

### 既存アーキテクチャとの接続

```
ソースコード → フロントエンド（共有）→ IR → ┬→ バイトコード codegen → VM インタープリタ → [ホット関数] → Cranelift JIT
                                              │
                                              └→ LLVM AOT codegen → .o → リンク → exe（本番）
```

JIT と AOT は **IR 正規化パス**（`middle/passes/ir_normalize.rs`）を共有し、バックエンドの codegen が LLVM から Cranelift に変わる。

### 実行フロー

```
関数呼び出し
  → fn_entry.code_ptr.load()
  → ┬─ インタープリタ stub（コールド状態）：バイトコードを一命令ずつ解釈
    └─ JIT ネイティブコード（ホット状態）：直接機械語を実行
  → 戻る
```

## 詳細設計

### 1. ディレクトリ構造

```
src/
├── backends/
│   ├── interpreter/              # 既存 — VM インタープリタ
│   │   └── executor/
│   │       ├── engine.rs         # 変更 — 呼び出し入口を直接解釈から FunctionEntry 分岐へ
│   │       └── ...
│   │
│   ├── jit/                      # 新規 — JIT コンパイル層
│   │   ├── mod.rs                # JIT モジュール入口、Cranelift コンテキストの初期化
│   │   ├── profiler.rs           # ホットネスカウント + 減衰 + 閾値判定
│   │   ├── entry.rs              # FunctionEntry + AtomicPtr 管理
│   │   ├── cache.rs              # コードキャッシュ（mmap 実行可能ページ管理）
│   │   ├── compiler.rs           # IR → Cranelift IR → ネイティブコード
│   │   ├── types.rs              # YaoXiang 型 → Cranelift 型マッピング
│   │   └── abi.rs                # 関数呼び出し規約（System V / Microsoft x64）
│   │
│   ├── llvm/                     # 計画中 — LLVM AOT（RFC-018）
│   ├── common/                   # 既存
│   └── runtime/                  # 既存
│
└── middle/
    └── passes/
        └── ir_normalize.rs       # 新規 — 共有 IR 正規化（スタック→SSA）
                                  #   JIT と LLVM AOT で共用
```

**重要な制約**：
- `backends/jit/` は `middle/`（IR 定義、正規化パス）、標準ライブラリ、Cranelift crate のみに依存
- `backends/jit/` は `backends/llvm/` に依存しない。両者は対等なバックエンド
- `backends/jit/` は `backends/interpreter/` に依存しない。`FunctionEntry` インターフェース経由で対話

### 2. ホットネス分析と階層的トリガ

#### 2.1 ホットネスステートマシン

```
Cold ──(invocation > 50 または backedge > 500)──→ Warm
Warm ──(invocation > 200)────────────────────→ Hot
Hot ──(コンパイルキュー送信、コンパイル完了)──────────────────→ Compiled
```

> 閾値は設定可能項目で、上記はデフォルト値。LuaJIT、JVM C1、V8 Sparkplug の実際の閾値範囲（50〜1000）を参考にした。

#### 2.2 カウンタ

各関数は `FunctionEntry`（詳細は §4.1 参照）内に 2 つのアトミックカウンタを保持する：

```rust
// FunctionEntry のホットネスフィールド（完全な定義は §4.1 参照）
invocation_count: AtomicU32,   // 関数が呼び出された回数
backedge_count: AtomicU32,     // ループバックエッジのジャンプ回数
state: AtomicU8,              // Cold | Warm | Hot | Compiled
```

#### 2.3 減衰メカニズム

5 秒ごとにすべてのカウンタを 1 ビット右シフトする（0.5 倍）。起動時に高頻度だが 1 回しか実行されないコード（初期化トラバースなど）が無意味な JIT コンパイルをトリガするのを防ぐ。

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

ビット演算を使い、除算のオーバーヘッドはゼロ。

#### 2.4 コンパイルキュー

```
インタープリタスレッド                       バックグラウンド JIT スレッド
    │                                        │
    ├─ ホットネスが Hot に到達                  │
    ├─ コンパイル要求を送信 ────────────────→  │
    │  (インタープリタをブロックしない)             ├─ 関数の IR を取り出す
    │                                        ├─ IR 正規化 (スタック→SSA)
    │                                        ├─ Cranelift コンパイル
    │                                        ├─ コードキャッシュに書き込み
    │                                        └─ 関数入口ポインタをアトミックに更新
    │  次回の関数呼び出し ←─────────────────  │
    │  ネイティブコードへ直接進む                  │
```

コンパイル中も関数はインタープリタ経由で実行される。コンパイル完了後、次回の呼び出しでアトミックに JIT コードへ切り替わる。

### 3. IR → Cranelift コンパイルパイプライン

#### 3.1 パイプライン

```
YaoXiang IR (スタック形式)
  → IR 正規化パス (スタック → レジスタ/SSA)    ← RFC-018 §4.0 を再利用
  → Cranelift IR 構築
  → Cranelift 最適化 + 機械語生成
  → コードキャッシュに書き込み
```

#### 3.2 YaoXiang 型 → Cranelift 型

| YaoXiang 型 | Cranelift 型 | 説明 |
|---------------|---------------|------|
| `Int` | `i64` | |
| `Int32` | `i32` | |
| `Float` | `f64` | |
| `Float32` | `f32` | |
| `Bool` | `i8` | Cranelift に `i1` がないため `i8` を使用 |
| `Char` | `i32` | Unicode コードポイント |
| `String` | `{ i64, i64 }` | ポインタ + 長さ |
| `Void` | 空タプル | |
| `&T` | — | ゼロサイズ、コンパイル後消滅 |
| `&mut T` | — | ゼロサイズ、コンパイル後消滅 |
| `ref T` | `{ i64, i64 }` | 参照カウントポインタ + データポインタ |
| `*T` | `i64` | 生ポインタ |
| `List(T)` | `{ i64, i64, i64 }` | データポインタ + 長さ + 容量 |
| 構造体 | Cranelift struct | |
| レコード enum | `{ i64, [max_payload] }` | タグ + union |
| `?T` | `{ i8, T }` | 値存在フラグ + データ |

> RFC-018 §3 の LLVM 型表との比較：Cranelift はポインタ型を区別せず、`i1` もないため、全体としてよりシンプル。

#### 3.3 主要命令の翻訳

| IR 命令 | Cranelift IR |
|---------|-------------|
| `Add { dst, lhs, rhs }` | `iadd`（整数）/ `fadd`（浮動小数点） |
| `Sub { dst, lhs, rhs }` | `isub` / `fsub` |
| `Mul { dst, lhs, rhs }` | `imul` / `fmul` |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv` |
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp eq` |
| `Jmp(label)` | `jump` |
| `JmpIf(cond, label)` | `brnz` |
| `Ret(Some(v))` | `return` |
| `Call { dst, func, args }` | `call` |
| `Load { dst, src }` | `load` |
| `Store { dst, src }` | `store` |
| `Spawn { ... }` | ランタイム `task_spawn` + `task_wait_all` を呼び出し |

> 完全な翻訳表は RFC 本文を参照。核心原則：Cranelift 命令セットは YaoXiang IR のすべての操作をカバーし、意味的な欠落は存在しない。

#### 3.4 2 種類の正規化の共存

VM インタープリタはスタックセマンティクス（`Push`/`Pop`/`Dup`/`Swap`）を必要とし、Cranelift JIT と LLVM AOT はレジスタ/SSA を必要とする。IR 正規化パスが一度変換を行い（RFC-018 §4.0）、JIT と AOT が共有する。IR 自身の表現は変えない。各バックエンドが自分のニーズに応じて同じ IR を消費する。

### 4. 関数入口テーブルとアトミック置換

#### 4.1 FunctionEntry

```rust
struct FunctionEntry {
    /// アトミックに差し替え可能な実行ターゲット
    code_ptr: AtomicPtr<u8>,
    /// 不変メタデータ
    bytecode: &'static [u8],        // インタープリタフォールバック
    ir: &'static FunctionIR,        // JIT コンパイルの入力
    /// ランタイム統計
    invocation_count: AtomicU32,
    backedge_count: AtomicU32,
    state: AtomicU8,                // Cold | Warm | Hot | Compiled
}
```

#### 4.2 入口分岐

```
呼び出し元
  → fn_entry.code_ptr.load(Ordering::Acquire)
  → ┬─ インタープリタ stub アドレス → インタープリタ実行、バイトコードを一命令ずつ解釈
    └─ JIT コードアドレス      → ネイティブコードへ直接ジャンプ
```

ポインタ間接参照は 1 回。近代的な CPU の分岐予測器の間接ジャンプ処理：最初の予測ミスの後はすべて正解。オーバーヘッドは約 1 サイクル。

#### 4.3 アトミック切り替え

コンパイル完了後、1 回の CAS：

```rust
fn install_jit_code(entry: &FunctionEntry, jit_code: *mut u8) -> bool {
    entry.code_ptr.compare_exchange(
        INTERPRETER_STUB,      // 期待値：まだインタープリタを指している
        jit_code,              // 置換後：JIT コード
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_ok()
}
```

インタープリタの停止なし、セーフポイント待機なし、呼び出し点走査なし。1 つのアトミック操作で切り替えが完了する。

### 5. コードキャッシュ

#### 5.1 構造

```
CodeCache:
  modules:
    "main.yao":
      functions:
        "compute"    → FunctionEntry (state: Compiled)
        "process"    → FunctionEntry (state: Cold)
        "init"       → FunctionEntry (state: Compiled)
      native_pages:   [ mmap'd executable memory pages ]
    "lib.yao":
      functions:
        "helper"     → FunctionEntry (state: Compiled)
      native_pages:   [ mmap'd executable memory pages ]
```

#### 5.2 実行可能メモリ管理

```rust
struct NativePage {
    ptr: *mut u8,
    size: usize,
    used: AtomicUsize,     // 使用済みバイト数
    remaining: usize,       // 残余容量
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // モジュール無効化時のみ呼び出し
}
```

各モジュールは連続した mmap 実行可能ページを割り当て、モジュール内のすべての JIT 関数は同じページから割り当てられる。モジュール無効化時はページ全体を回収し、関数ごとの解放は不要。

### 6. ホットリロード预留拡張ポイント

以下のインターフェースはコンパイルは通るが、ホットリロード実装前は呼び出さない。インターフェース設計原則：**JIT 実装時は `insert` と単関数 `compare_exchange` のみ必要で、モジュールレベルの操作はホットリロードに任せる。**

```rust
/// コードキャッシュ拡張インターフェース（预留、未実装）
trait CodeCacheExt {
    /// モジュール全体のすべての JIT コードを無効化し、インタープリタへフォールバック
    fn invalidate_module(&self, module_path: &str);

    /// ソース位置範囲に基づいて特定関数を無効化
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// モジュール全体の関数テーブルをアトミックに置換
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// コンパイルキュー拡張インターフェース（预留、未実装）
trait CompileQueueExt {
    /// 優先度割り込み（ホットリロードコンパイルは通常 JIT コンパイルより優先）
    fn submit_priority(&self, task: CompileTask);
}
```

**なぜモジュールごとにグループ化するか？** JIT 自身は関数しか必要としない。モジュール単位の組織化は完全にホットリロードのためである：モジュール再コンパイル後、モジュール全体の関数セットをアトミックに置換できる。関数ごとの CAS では、関数間に循環依存がある場合に不整合状態を引き起こす可能性がある。

## トレードオフ

### 利点

1. **ゼロ知覚コンパイル遅延**：Cranelift は 1〜5ms/関数、バックグラウンドスレッドでコンパイル、インタープリタは停止しない
2. **共有インフラ**：JIT と AOT は IR 正規化パス（RFC-018 §4.0）を共有し、車輪の再発明を避ける
3. **非破壊的**：純粋な増分機能。VM は変わらず、インタープリタも変わらず、より高速なホットパスが追加されるだけ
4. **LLVM 依存なし**：VM は LLVM を導入せず、軽量さを維持
5. **マルチプラットフォームを自然にサポート**：Cranelift は x86_64 と ARM64 をネイティブサポートし、すべてのターゲットプラットフォームをカバー
6. **ホットリロード预留**：コードキャッシュのモジュール別グループ化 + 関数入口の間接ジャンプにより、将来のホットリロードの構造的基盤を整える

### 欠点

1. **Cranelift の新規依存**：新しい外部 crate を導入し、その API に習熟する必要がある
2. **デバッグの複雑さ**：JIT 生成コードのスタックフレームがインタープリタのスタックフレームと互換である必要があり、デバッグ情報のマッピングに追加処理が必要
3. **コールドスタートのホットネス遅延**：プログラム起動後の数秒間は JIT 加速がなく、ホットネスの蓄積が必要
4. **プラットフォーム ABI**：異なるプラットフォーム（Linux/macOS/Windows）の mmap と呼び出し規約をそれぞれ適応させる必要がある

### 関連 RFC との一貫性

| RFC | 一貫性 |
|-----|--------|
| RFC-018 LLVM AOT | ✅ IR 正規化パスを共有、JIT と AOT は対等なバックエンド |
| RFC-024 spawn ブロック並行 | ✅ spawn ブロックはランタイム関数呼び出しへコンパイル |
| RFC-008 ランタイムアーキテクチャ | ✅ 3 層ランタイム（Embedded/Standard/Full）すべてで JIT をサポート |

## 代替案

| 代替案 | 選択しない理由 |
|------|--------|
| LLVM AOT のみ、JIT なし | 開発時にプログラム全体の再コンパイルが必要で、高速イテレーション体験を喪失 |
| LLVM ORC JIT | コンパイル遅延が高い（10〜100ms）、LLVM 依存が大きい、VM への埋め込みに適さない |
| カスタム軽量 JIT（dynasm） | 手書きバックエンドのメンテナンスコストが高く、Cranelift ほど成熟していない |
| テンプレート JIT | 最適化ゼロ、コード品質が低く、JIT コンパイルの時間を浪費 |
| 全プログラム JIT（インタープリタなし） | コールドスタートが遅く、簡単なスクリプトはコンパイルする価値がない |

## 依存関係

- RFC-018（LLVM AOT）→ IR 正規化パスを共有
- RFC-024（spawn ブロック並行）→ spawn ブロックの JIT コンパイル
- RFC-008（ランタイムアーキテクチャ）→ 3 層ランタイムの JIT サポート
- Cranelift crate → JIT バックエンド

## 参考文献

- [Cranelift IR Documentation](https://github.com/bytecodealliance/wasmtools/tree/main/cranelift)
- [RFC-018：LLVM AOT コンパイラ設計](../review/018-llvm-aot-compiler.md)
- [RFC-024：spawn ブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
- [RFC-008：Runtime 並行モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)
- Hölzle, U. (1994). *Adaptive Optimization for Self: Reconciling High Performance with Exploratory Programming*. Stanford.

---
## ライフサイクルと帰属

| 状態 | 場所 | 説明 |
|------|------|------|
| **ドラフト** | `docs/src/design/rfc/draft/` | 著者の草稿、レビュー提出待ち |
| **レビュー中** | `docs/src/design/rfc/review/` | コミュニティの議論とフィードバックを公開 |
| **承認済み** | `docs/src/design/rfc/accepted/` | 正式な設計ドキュメントとなる |