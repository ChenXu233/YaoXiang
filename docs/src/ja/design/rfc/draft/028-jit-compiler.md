---
title: 'RFC-028：JIT コンパイラ — VM 内多段実行エンジン'
status: '草案'
author: '晨煦'
created: '2026-06-11'
updated: '2026-07-05'
issue: '#101'
---

# RFC-028：JIT コンパイラ — VM 内多段実行エンジン

> **参考**:
>
> - [RFC-018：LLVM AOT コンパイラ設計](../review/018-llvm-aot-compiler.md)
> - [RFC-024：spawn ブロックに基づく並列モデル](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime 並列モデルとスケジューラの分離設計](../accepted/008-runtime-concurrency-model.md)

## 摘要

本文書は YaoXiang の VM バックエンドに Cranelift
JIT コンパイラを導入し、VM を純粋なインタープリタから**多段実行エンジン**にアップグレードすることを提案する：冷たいコードはインタープリタ実行、熱い関数は Cranelift でネイティブコードにコンパイル。JIT パスは RFC-018 の LLVM
AOT パスと IR 正規化 pass を共有し、Cranelift は JIT の高速コンパイルを担当し、LLVM は AOT の深い最適化を担当し、それぞれ得意分野を活かす。

**コアポジショニング：JIT は VM に奉仕するもの、VM を置き換えるものではない。**

## 動機

### なぜ JIT が必要か？

現在の VM バックエンドは純粋なインタープリタであり、実行速度はネイティブコードより 10-100 倍遅い。開発時にテスト、スクリプト、ローカルデバッグを頻繁に行う——これらのシナリオでは AOT の極限最適化は不要だが、インタープリタより明らかに高速な実行速度が必要。

### なぜ LLVM AOT だけ使わないのか？

LLVM
AOT コンパイルには長い時間（秒単位）がかかり、開発イテレーションに適さない。開発には「修正したら即実行」の体験が必要：1 行コード修正 → 再実行 → ほぼ即座に結果確認。Cranelift
JIT なら関数あたり 1-5ms でコンパイルでき、ユーザーはコンパイル遅延を感知しない。

### なぜ Cranelift で LLVM ORC JIT ではないのか？

| 観点           | Cranelift JIT                    | LLVM ORC JIT                     |
| -------------- | -------------------------------- | -------------------------------- |
| コンパイル速度 | 1-5ms/関数                       | 10-100ms/関数                    |
| 依存サイズ     | 小                               | 大（完全 LLVM が必要）           |
| コード品質     | LLVM -O2 の 70-80%               | 非常に高い                       |
| 適用シナリオ   | 開発デバッグ、快速イテレーション | 不適用（本文のトレードオフ参照） |

Cranelift はコンパイルが速く、コード品質は十分。LLVM は AOT のオフライン深最適化に残す。一つのツールは一つのことをうまくやる。

## 提案

### コアアーキテクチャ

```
VM 実行エンジン
├── インタープリタ層
│   ├── バイトコード命令を実行
│   ├── ホットデータ収集（呼び出し回数 + ループバックエッジ回数）
│   └── しきい値到達 → コンパイルタスクを提交
│
├── JIT コンパイル層（Cranelift バックエンド）
│   ├── コンパイルキュー（バックグラウンドスレッド、インタープリタを阻塞しない）
│   ├── IR → 正規化 → Cranelift IR → ネイティブコード
│   └── RFC-018 §4.0 の IR 正規化 pass を再利用（スタック→SSA）
│
├── コードキャッシュ
│   ├── 関数テーブル：関数 ID → {インタープリタエントリ、JITエントリ(省略可)}
│   ├── コンパイル済み関数エントリの原子置換
│   └── モジュール単位のグループ化（ホットリロード接口预留）
│
└── ホット分析
    ├── 関数ごとの呼び出し回数 + ループバックエッジ回数
    ├── 定期減衰（一回限りのプリヒートによるコンパイル発動を防止）
    └── 三段階ホット：Cold → Warm → Hot → Compiled
```

### 既存アーキテクチャとの接続

```
ソースコード → フロントエンド（共有）→ IR → ┬→ バイトコード codegen → VM インタープリタ → [ホット関数] → Cranelift JIT
                                                │
                                                └→ LLVM AOT codegen → .o → リンク → exe（本番）
```

JIT と AOT は
**IR 正規化 pass**（`middle/passes/ir_normalize.rs`）を共有し、底層の codegen は LLVM から Cranelift に切り替わる。

### 実行フロー

```
関数呼び出し
  → fn_entry.code_ptr.load()
  → ┬─ インタープリタスタブ（冷たい状態）：バイトコードを逐次解釈
    └─ JIT ネイティブコード（ホット状態）：機械語を直接実行
  → 戻り値
```

## 詳細設計

### 1. ディレクトリ構造

```
src/
├── backends/
│   ├── interpreter/              # 既存 — VM インタープリタ
│   │   └── executor/
│   │       ├── engine.rs         # 変更 — 呼び出しエントリを直接解釈から FunctionEntry ディスパッチに変更
│   │       └── ...
│   │
│   ├── jit/                      # 新規 — JIT コンパイル層
│   │   ├── mod.rs                # JIT モジュールエントリ、Cranelift コンテキストを初期化
│   │   ├── profiler.rs           # ホットカウント + 減衰 + しきい値判定
│   │   ├── entry.rs              # FunctionEntry + AtomicPtr 管理
│   │   ├── cache.rs              # コードキャッシュ（mmap 実行可能ページ管理）
│   │   ├── compiler.rs           # IR → Cranelift IR → ネイティブコード
│   │   ├── types.rs              # YaoXiang 型 → Cranelift 型マッピング
│   │   └── abi.rs                # 関数呼出規約（System V / Microsoft x64）
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

- `backends/jit/` は `middle/`（IR 定義、正規化 pass）、標準ライブラリ、Cranelift crate にのみ依存
- `backends/jit/` は `backends/llvm/` に依存しない、両者は平列バックエンド
- `backends/jit/` は `backends/interpreter/` に依存しない、`FunctionEntry` 接口経由で相互作用

### 2. ホット分析と階層的トリガー

#### 2.1 ホット状態機械

```
Cold ──(呼び出し > 50 または backedge > 500)──→ Warm
Warm ──(呼び出し > 200)─────────────────────────→ Hot
Hot ──(コンパイルキュー提交、コンパイル完了)──────────→ Compiled
```

> しきい値は設定可能項目であり、上記はデフォルト値。LuaJIT、JVM C1、V8
> Sparkplug の実際のしきい値範囲（50-1000）を参照。

#### 2.2 カウンタ

各関数は `FunctionEntry`（詳細は §4.1）で2つのアトミックカウンタを維持する：

```rust
// FunctionEntry のホットフィールド（完全定義は §4.1 参照）
invocation_count: AtomicU32,   // 関数呼び出し回数
backedge_count: AtomicU32,     // ループバックエッジジャンプ回数
state: AtomicU8,              // Cold | Warm | Hot | Compiled
```

#### 2.3 減衰機構

5 秒ごとにすべてのカウンタを右シフト 1 回（0.5 倍）。起動時に高频だが一回しか実行しないコード（初期化走査など）が無意味な JIT コンパイルを引き起こすのを防止。

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

ビット演算を使用、除算开销为零。

#### 2.4 コンパイルキュー

```
インタープリタスレッド                         バックグラウンド JIT スレッド
    │                                             │
    ├─ ホットが Hot に達した                      │
    ├─ コンパイルリクエストを送信 ─────────────→  │
    │  （インタープリタを阻塞しない）                ├─ 関数 IR を取り出す
    │                                             ├─ IR 正規化（スタック→SSA）
    │                                             ├─ Cranelift コンパイル
    │                                             ├─ コードキャッシュに書き込み
    │                                             └─ 関数エントリポインタを原子更新
    │  次回この関数を呼び出す ←─────────────────  │
    │  ネイティブコードを直接使用                   │
```

コンパイル期間中は関数はインタープリタ経由で実行される。コンパイル完了後、次回呼び出し時に原子的に JIT コードに切り替え。

### 3. IR → Cranelift コンパイルパイプライン

#### 3.1 パイプライン

```
YaoXiang IR（スタック形式）
  → IR 正規化 pass（スタック → レジスタ/SSA）      ← RFC-018 §4.0 を再利用
  → Cranelift IR 構築
  → Cranelift 最適化 + 機械語生成
  → コードキャッシュに書き込み
```

#### 3.2 YaoXiang 型 → Cranelift 型

| YaoXiang 型 | Cranelift 型             | 説明                                       |
| ----------- | ------------------------ | ------------------------------------------ |
| `Int`       | `i64`                    |                                            |
| `Int32`     | `i32`                    |                                            |
| `Float`     | `f64`                    |                                            |
| `Float32`   | `f32`                    |                                            |
| `Bool`      | `i8`                     | Cranelift には `i1` がないため `i8` を使用 |
| `Char`      | `i32`                    | Unicode コードポイント                     |
| `String`    | `{ i64, i64 }`           | ポインタ + 長さ                            |
| `Void`      | 空タプル                 |                                            |
| `&T`        | —                        | ゼロサイズ、コンパイル後に消失             |
| `&mut T`    | —                        | ゼロサイズ、コンパイル後に消失             |
| `ref T`     | `{ i64, i64 }`           | 参照カウントポインタ + データポインタ      |
| `*T`        | `i64`                    | 生ポインタ                                 |
| `List(T)`   | `{ i64, i64, i64 }`      | データポインタ + 長さ + 容量               |
| 構造体      | Cranelift struct         |                                            |
| 記録列挙型  | `{ i64, [max_payload] }` | タグ + union                               |
| `?T`        | `{ i8, T }`              | 値ありマーカー + データ                    |

> RFC-018 §3 の LLVM 型テーブルとの比較：Cranelift はポインタ型を区別しない、`i1`
> がないため、全体的によりシンプル。

#### 3.3 重要命令の翻訳

| IR 命令                    | Cranelift IR                                         |
| -------------------------- | ---------------------------------------------------- |
| `Add { dst, lhs, rhs }`    | `iadd`（整数）/ `fadd`（浮動小数点）                 |
| `Sub { dst, lhs, rhs }`    | `isub` / `fsub`                                      |
| `Mul { dst, lhs, rhs }`    | `imul` / `fmul`                                      |
| `Div { dst, lhs, rhs }`    | `sdiv` / `udiv` / `fdiv`                             |
| `Eq { dst, lhs, rhs }`     | `icmp eq` / `fcmp eq`                                |
| `Jmp(label)`               | `jump`                                               |
| `JmpIf(cond, label)`       | `brnz`                                               |
| `Ret(Some(v))`             | `return`                                             |
| `Call { dst, func, args }` | `call`                                               |
| `Load { dst, src }`        | `load`                                               |
| `Store { dst, src }`       | `store`                                              |
| `Spawn { ... }`            | ランタイム `task_spawn` + `task_wait_all` を呼び出し |

> 完全な翻訳テーブルは RFC 本文を参照。主要原則：Cranelift 命令セットは YaoXiang
> IR のすべての操作をカバーし、意味的なギャップはない。

#### 3.4 2 つの正規化共存

VM インタープリタはスタックセマンティクス（`Push`/`Pop`/`Dup`/`Swap`）を必要とし、Cranelift
JIT と LLVM AOT はレジスタ/SSA を必要とする。IR 正規化 pass は一回の変換を行い（RFC-018
§4.0）、JIT と AOT が共用、IR 本身的表現は変わらない。各バックエンドは各自のニーズに応じて同じ IR を利用する。

### 4. 関数エントリテーブルと原子置換

#### 4.1 FunctionEntry

```rust
struct FunctionEntry {
    /// 原子的に置換可能な実行ターゲット
    code_ptr: AtomicPtr<u8>,
    /// 不変メタデータ
    bytecode: &'static [u8],        // インタープリタ fallback
    ir: &'static FunctionIR,        // JIT コンパイルの入力
    /// 実行時統計
    invocation_count: AtomicU32,
    backedge_count: AtomicU32,
    state: AtomicU8,                // Cold | Warm | Hot | Compiled
}
```

#### 4.2 エントリディスパッチ

```
呼び出し側
  → fn_entry.code_ptr.load(Ordering::Acquire)
  → ┬─ インタープリタスタブアドレス → インタープリタを実行、バイトコードを逐次解釈
    └─ JIT コードアドレス       → ネイティブコードに直接ジャンプ
```

1 回のポインタ間接参照。現代の CPU 분기予測器对间接跳转的处理：首次预测错误，之后全对。开销约 1
cycle。

#### 4.3 原子切り替え

コンパイル完了後に 1 回の CAS：

```rust
fn install_jit_code(entry: &FunctionEntry, jit_code: *mut u8) -> bool {
    entry.code_ptr.compare_exchange(
        INTERPRETER_STUB,      // 期待値：まだインタープリタを指している
        jit_code,              // 置換先：JIT コード
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_ok()
}
```

インタープリタを一時停止せず、安全点待機もなく、呼び出し箇所走査もない。1 つの原子操作で切り替え完了。

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
      native_pages:   [ mmap'd 実行可能メモリページ ]
    "lib.yao":
      functions:
        "helper"     → FunctionEntry (state: Compiled)
      native_pages:   [ mmap'd 実行可能メモリページ ]
```

#### 5.2 実行可能メモリ管理

```rust
struct NativePage {
    ptr: *mut u8,
    size: usize,
    used: AtomicUsize,     // 使用済みバイト数
    remaining: usize,       // 残り容量
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // モジュール失效時のみ呼び出す
}
```

各モジュールは連続した mmap 実行可能ページを割り当て、モジュール内のすべての JIT 関数は同じページから割り当てられる。モジュールが失效したらページ全体を回收、関数ごとの解放は不要。

### 6. ホットリロード预留拡張点

以下の接口はコンパイルは通るがホットリロード実装前は呼び出さない。接口設計原則：**JIT 実装時は
`insert` と単関数の `compare_exchange` のみ必要で、モジュールレベルの操作はホットリロードに残す。**

```rust
/// コードキャッシュ拡張接口（预留、実装せず）
trait CodeCacheExt {
    /// モジュール全体を失效させ、JIT コードをすべてインタープリタにFallback
    fn invalidate_module(&self, module_path: &str);

    /// ソースコード位置範囲に基づいて特定の関数を失效
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// モジュール全体の関数テーブルを原子置換
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// コンパイルキュー拡張接口（预留、実装せず）
trait CompileQueueExt {
    /// 優先度挿入（ホットリロードコンパイルは通常の JIT コンパイルより優先）
    fn submit_priority(&self, task: CompileTask);
}
```

**なぜモジュール単位のグループ化なのか？**
JIT 自体は関数のみ必要。モジュール単位の組織は完全にホットリロード服务平台：モジュールの再コンパイル後、関数テーブル全体を原子的に置換でき、関数ごとに CAS するものではない——後者は関数間に循環依存がある場合に不整合状態を引き起こす。

## トレードオフ

### 利点

1. **ゼロ知覚コンパイル遅延**：Cranelift は 1-5ms/関数、バックグラウンドスレッドでコンパイル、インタープリタは一時停止しない
2. **インフラ共有**：JIT と AOT は IR 正規化 pass（RFC-018 §4.0）を共有し、車輪の再開発なし
3. **非破壊的**：純粋な増分機能。VM は不变、インタープリタは不变、より高速なホットパスの追加のみ
4. **LLVM 依存なし**：VM に LLVM を導入せず、軽量を維持
5. **ネイティブなマルチプラットフォーム対応**：Cranelift は x86_64 と ARM64 をネイティブサポート、対象プラットフォームをすべてカバー
6. **ホットリロード预留**：コードキャッシュはモジュール単位 + 関数エントリ間接ジャンプで組織化、将来のホットリロードに構造的基盤を提供

### 欠点

1. **Cranelift の新規依存**：新しい外部 crate を導入し、その API に精通する必要がある
2. **デバッグ複雑度**：JIT が生成したコードのスタックフレームはインタープリタのスタックフレームと互換性が必要で、デバッグ情報マッピングに追加処理が必要
3. **コールドスタートのホット遅延**：プログラム起動後最初の数秒間は JIT 加速がなく、ホットデータの蓄積が必要
4. **プラットフォーム ABI**：異なるプラットフォーム（Linux/macOS/Windows）の mmap と呼出規約はそれぞれ適応が必要

### 関連 RFC との整合性

| RFC                              | 整合性                                                             |
| -------------------------------- | ------------------------------------------------------------------ |
| RFC-018 LLVM AOT                 | ✅ IR 正規化 pass を共有、JIT と AOT は平列バックエンド            |
| RFC-024 spawn ブロック並列       | ✅ spawn ブロックはランタイム関数呼び出しにコンパイル              |
| RFC-008 ランタイムアーキテクチャ | ✅ 三層ランタイム（Embedded/Standard/Full）はすべて JIT をサポート |

## 代替案

| 案                                     | なぜ選択しないか                                                             |
| -------------------------------------- | ---------------------------------------------------------------------------- |
| LLVM AOT のみ、JIT なし                | 開発時にプログラム全体を再コンパイルが必要、快速イテレーション体験を失う     |
| LLVM ORC JIT                           | コンパイル遅延が高い（10-100ms）、LLVM 依存が大きく VM に埋め込むのに不向き  |
| カスタム軽量 JIT（dynasm）             | 手書きバックエンドのメンテナンスコストが高く、Cranelift の成熟度にかなわない |
| テンプレート JIT                       | 最適化なし、コード品質が悪い、JIT コンパイル時間を無駄にする                 |
| 全プログラム JIT（インタープリタなし） | コールドスタートが遅く、シンプルなスクリプトはコンパイル不值得               |

## 依存関係

- RFC-018（LLVM AOT）→ 共有 IR 正規化 pass
- RFC-024（spawn ブロック並列）→ spawn ブロックの JIT コンパイル
- RFC-008（ランタイムアーキテクチャ）→ 三層ランタイム JIT サポート
- Cranelift crate → JIT バックエンド

## 参考文献

- [Cranelift IR Documentation](https://github.com/bytecodealliance/wasmtools/tree/main/cranelift)
- [RFC-018：LLVM AOT コンパイラ設計](../review/018-llvm-aot-compiler.md)
- [RFC-024：spawn ブロックに基づく並列モデル](../accepted/024-concurrency-model.md)
- [RFC-008：Runtime 並列モデルとスケジューラの分離設計](../accepted/008-runtime-concurrency-model.md)
- Hölzle, U. (1994). _Adaptive Optimization for Self: Reconciling High Performance with Exploratory
  Programming_. Stanford.

---

## ライフサイクルと行き先

| 状態           | 場所                            | 説明                                 |
| -------------- | ------------------------------- | ------------------------------------ |
| **草案**       | `docs/src/design/rfc/draft/`    | 著者草案、レビュー待ち               |
| **レビュー中** | `docs/src/design/rfc/review/`   | コミュニティ議論とフィードバック公開 |
| **承認済み**   | `docs/src/design/rfc/accepted/` | 正式設計ドキュメント                 |
