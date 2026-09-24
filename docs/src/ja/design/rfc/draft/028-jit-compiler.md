---
title: 'RFC-028：JIT コンパイラ — VM 内のマルチレベル実行エンジン'
status: '草案'
author: '晨煦'
created: '2026-06-11'
updated: '2027-07-05'
issue: '#101'
---

# RFC-028：JIT コンパイラ — VM 内のマルチレベル実行エンジン

> **参考**:
>
> - [RFC-018：LLVM AOT コンパイラ設計](../accepted/018-llvm-aot-compiler.md)
> - [RFC-024：spawn ブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime 並行モデルとスケジューラの分離設計](../accepted/008-runtime-concurrency-model.md)

## 摘要

本文書は YaoXiang の VM バックエンドに Cranelift JIT コンパイラを導入し、VM を純粋なインタープリタから**マルチレベル実行エンジン**にアップグレードするを提案する：冷的コードはインタープリタ実行、熱い関数は Cranelift でネイティブコードにコンパイル。JIT パスは RFC-018 の LLVM AOT パスと IR 正規化 pass を共有し、Cranelift は JIT の高速コンパイルを担当し、LLVM は AOT の深い最適化を担当し、各々の長所を生かす。

**核心的な位置づけ：JIT は VM に奉仕するもの、VM を置き換えるものではない。**

## 動機

### なぜ JIT が必要か？

現在の VM バックエンドは純粋なインタープリタであり、実行速度はネイティブコードの 10-100 倍遅い。開発時には頻繁にテスト、スクリプト、ローカルデバッグを実行する——これらのシナリオでは AOT の極端な最適化は不要だが、インタープリタより明らかに速い実行速度が必要。

### なぜ LLVM AOT だけを使わないのか？

LLVM AOT コンパイルは時間がかる（秒レベル）ため、開発イテレーションに適さない。開発には「変更したらすぐに実行」の体験が必要：一行のコードを変更 → 再実行 → ほぼ即座に結果を確認。Cranelift JIT は単一関数のコンパイルに 1-5ms しかかからないため、ユーザーはコンパイル遅延を感知しない。

### なぜ Cranelift で LLVM ORC JIT ではないのか？

| 観点       | Cranelift JIT      | LLVM ORC JIT         |
| ---------- | ------------------ | -------------------- |
| コンパイル速度 | 1-5ms/関数         | 10-100ms/関数        |
| 依存サイズ   | 小                 | 大（完全な LLVM が必要）|
| コード品質   | LLVM -O2 の 70-80% | 極めて高い           |
| 適用シナリオ | 開発デバッグ、快速イテレーション | 不適用（本文のトレードオフ参照）|

Cranelift はコンパイルが速く、コード品質は十分。LLVM はオフライン深度最適化のために AOT に任せる。一つのツールは一つのことをうまくやる。

## 提案

### コアアーキテクチャ

```
VM 実行エンジン
├── インタープリタ層
│   ├── バイトコード命令を実行
│   ├── 热度データを収集（呼び出し回数 + ループバックエッジ回数）
│   └── 閾値到達 → コンパイルタスクを提出
│
├── JIT コンパイル層（Cranelift バックエンド）
│   ├── コンパイルキュー（バックグラウンドスレッド、インタープリタをブロックしない）
│   ├── IR → 正規化 → Cranelift IR → ネイティブコード
│   └── RFC-018 §4.0 の IR 正規化 pass を再利用（スタック→SSA）
│
├── コードキャッシュ
│   ├── 関数テーブル：関数 ID → {インタープリタ入口, JIT入口(任意)}
│   ├── コンパイル済み関数入口の原子置換
│   └── モジュール単位のグループ化（ホットリロードインターフェースを予約）
│
└── 热度分析
    ├── 各関数の呼び出し回数 + ループバックエッジ回数
    ├── 定期減衰（一度だけのウォームアップによるコンパイルを防止）
    └── 三段階の热度：Cold → Warm → Hot → Compiled
```

### 既存アーキテクチャとの接続

```
ソースコード → フロントエンド（共用）→ IR → ┬→ バイトコード codegen → VM インタープリタ → [熱関数] → Cranelift JIT
                           │
                           └→ LLVM AOT codegen → .o → リンク → exe（本番）
```

JIT と AOT は **IR 正規化 pass**（`middle/passes/ir_normalize.rs`）を共有し、バックエンドの codegen を LLVM から Cranelift に切り替える。

### 実行フロー

```
関数呼び出し
  → fn_entry.code_ptr.load()
  → ┬─ インタープリタスタブ（冷的状態）：バイトコードを逐条解釈
    └─ JIT ネイティブコード（熱的状態）：機械語を直接実行
  → 戻り
```

## 詳細設計

### 1. ディレクトリ構造

```
src/
├── backends/
│   ├── interpreter/              # 既存 — VM インタープリタ
│   │   └── executor/
│   │       ├── engine.rs         # 変更 — 呼び出し入口を直接解釈から FunctionEntry ディスパッチに変更
│   │       └── ...
│   │
│   ├── jit/                      # 新規 — JIT コンパイル層
│   │   ├── mod.rs                # JIT モジュール入口、Cranelift コンテキストを初期化
│   │   ├── profiler.rs           # 热度カウント + 減衰 + 閾値判定
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
        └── ir_normalize.rs       # 新規 — 共用 IR 正規化（スタック→SSA）
                                  #   JIT と LLVM AOT が共用
```

**重要な制約**：

- `backends/jit/` は `middle/`（IR 定義、正規化 pass）、標準ライブラリ、Cranelift crate にのみ依存
- `backends/jit/` は `backends/llvm/` に依存しない、両者は対等のバックエンド
- `backends/jit/` は `backends/interpreter/` に依存しない、`FunctionEntry` インターフェースを通じてやり取り

### 2. 热度分析と階層的トリガー

#### 2.1 热度状態機械

```
Cold ──(呼び出し > 50 または バックエッジ > 500)──→ Warm
Warm ──(呼び出し > 200)────────────────────────────→ Hot
Hot ──(コンパイルキュー提出、コンパイル完了)──────────→ Compiled
```

> 閾値は設定可能項目であり、ここではデフォルト値を示す。LuaJIT、JVM C1、V8 Sparkplug の実際の閾値範囲（50-1000）を参考。

#### 2.2 カウンタ

各関数は `FunctionEntry`（詳細は §4.1）で 2 つのアトミックカウンタを維持：

```rust
// FunctionEntry の热度フィールド（完全な定義は §4.1 を参照）
invocation_count: AtomicU32,   // 関数呼び出し回数
backedge_count: AtomicU32,     // ループバックエッジジャンプ回数
state: AtomicU8,               // Cold | Warm | Hot | Compiled
```

#### 2.3 減衰メカニズム

5 秒ごとに全カウンタを 1 ビット右シフト（0.5 倍）。起動時に高频だが一度だけ実行されるコード（初期化走査など）が無意味な JIT コンパイルを引き起こすのを防止。

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

ビット演算を使用し、除算オーバーヘッドをゼロに。

#### 2.4 コンパイルキュー

```
インタープリタスレッド                         バックグラウンド JIT スレッド
    │                                          │
    ├─ 热度 Hot 到達                           │
    ├─ コンパイルリクエストをプッシュ ─────────────→  │
    │  （インタープリタをブロックしない）            ├─ 関数 IR を取出
    │                                          ├─ IR 正規化 (スタック→SSA)
    │                                          ├─ Cranelift コンパイル
    │                                          ├─ コードキャッシュに書き込み
    │                                          └─ 関数入口ポインタを原子更新
    │  次回この関数を呼び出す ←─────────────────  │
    │  ネイティブコードを直接使用                     │
```

コンパイル中は関数はインタープリタ経由で実行。コンパイル完了後、次回呼び出しで JIT コードに原子切り替え。

### 3. IR → Cranelift コンパイルパイプライン

#### 3.1 パイプライン

```
YaoXiang IR（スタック形式）
  → IR 正規化 pass（スタック → レジスタ/SSA）    ← RFC-018 §4.0 を再利用
  → Cranelift IR 構築
  → Cranelift 最適化 + 機械語生成
  → コードキャッシュに書き込み
```

#### 3.2 YaoXiang 型 → Cranelift 型

| YaoXiang 型 | Cranelift 型             | 説明                       |
| ----------- | ------------------------ | -------------------------- |
| `Int`       | `i64`                    |                            |
| `Int32`     | `i32`                    |                            |
| `Float`     | `f64`                    |                            |
| `Float32`   | `f32`                    |                            |
| `Bool`      | `i8`                     | Cranelift は `i1` がないため `i8` を使用 |
| `Char`      | `i32`                    | Unicode コードポイント     |
| `String`    | `{ i64, i64 }`           | ポインタ + 長さ            |
| `Void`      | 空タプル                  |                            |
| `&T`        | —                        | ゼロサイズ、コンパイル後に消える |
| `&mut T`    | —                        | ゼロサイズ、コンパイル後に消える |
| `ref T`     | `{ i64, i64 }`           | 参照カウントポインタ + データポインタ |
| `*T`        | `i64`                    | 裸ポインタ                 |
| `List(T)`   | `{ i64, i64, i64 }`      | データポインタ + 長さ + 容量 |
| 構造体      | Cranelift 構造体         |                            |
| 記録列挙型  | `{ i64, [max_payload] }` | タグ + union               |
| `?T`        | `{ i8, T }`              | 値ありマーカー + データ    |

> RFC-018 §3 の LLVM 型テーブルとの比較：Cranelift はポインタ型を区別しない、`i1` がないため、全体的によりシンプル。

#### 3.3 主要命令の翻訳

| IR 命令                  | Cranelift IR                              |
| ------------------------ | ----------------------------------------- |
| `Add { dst, lhs, rhs }`  | `iadd`（整数）/ `fadd`（浮動小数点）      |
| `Sub { dst, lhs, rhs }`  | `isub` / `fsub`                           |
| `Mul { dst, lhs, rhs }`  | `imul` / `fmul`                           |
| `Div { dst, lhs, rhs }`  | `sdiv` / `udiv` / `fdiv`                  |
| `Eq { dst, lhs, rhs }`   | `icmp eq` / `fcmp eq`                     |
| `Jmp(label)`             | `jump`                                    |
| `JmpIf(cond, label)`      | `brnz`                                    |
| `Ret(Some(v))`           | `return`                                  |
| `Call { dst, func, args }` | `call`                                    |
| `Load { dst, src }`      | `load`                                    |
| `Store { dst, src }`     | `store`                                   |
| `Spawn { ... }`          | ランタイム `task_spawn` + `task_wait_all` を呼び出し |

> 完全な翻訳テーブルは RFC 本文を参照。核心原則：Cranelift 命令セットは YaoXiang IR の全操作をカバーし、意味的なギャップはない。

#### 3.4 二種類の正規化の共存

VM インタープリタはスタックセマンティクス（`Push`/`Pop`/`Dup`/`Swap`）を必要とし、Cranelift JIT と LLVM AOT はレジスタ/SSA を必要とする。IR 正規化 pass は一度の変換を行い（RFC-018 §4.0）、JIT と AOT が共用し、IR 自体の表現を変えない。各バックエンドは自分の必要に応じて同じ IR を利用する。

### 4. 関数入口テーブルと原子置換

#### 4.1 FunctionEntry

```rust
struct FunctionEntry {
    /// 原子的に置換可能な実行ターゲット
    code_ptr: AtomicPtr<u8>,
    /// 不変メタデータ
    bytecode: &'static [u8],        // インタープリタ fallback
    ir: &'static FunctionIR,        // JIT コンパイルの入力
    /// ランタイム統計
    invocation_count: AtomicU32,
    backedge_count: AtomicU32,
    state: AtomicU8,                // Cold | Warm | Hot | Compiled
}
```

#### 4.2 入口ディスパッチ

```
呼び出し元
  → fn_entry.code_ptr.load(Ordering::Acquire)
  → ┬─ インタープリタスタブのアドレス → インタープリタを実行、バイトコードを逐条解釈
    └─ JIT コードアドレス           → ネイティブコードに直接ジャンプ
```

一回だけのポインタ逆参照。现代的な CPU ブランチプレディクタの間接ジャンプ処理：初回の予測ミスの後、その後はすべて正解。オーバーヘッドは約 1 cycle。

#### 4.3 原子切り替え

コンパイル完了後、一回の CAS：

```rust
fn install_jit_code(entry: &FunctionEntry, jit_code: *mut u8) -> bool {
    entry.code_ptr.compare_exchange(
        INTERPRETER_STUB,      // 期待値：インタープリタを指したまま
        jit_code,              // 置換先：JIT コード
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_ok()
}
```

インタープリタを一時停止せず、セーフティポイントを待たず、呼び出し点を走査しない。一つの原子操作で切り替え完了。

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
    remaining: usize,      // 残り容量
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // モジュール失效時にのみ呼び出す
}
```

各モジュールは連続した mmap 実行可能ページを割り当て、モジュール内の全 JIT 関数は同一ページから割り当てを受ける。モジュール失效時はページ全体を回収し、関数ごとの解放は不要。

### 6. ホットリロード予約拡張ポイント

以下のインターフェースはコンパイルが通るが、ホットリロード実装前は呼び出さない。インターフェース設計原則：**JIT 実装時には `insert` と単一関数の `compare_exchange` のみ必要であり、モジュールレベルの操作はホットリロードに委ねる。**

```rust
/// コードキャッシュ拡張インターフェース（予約、未実装）
trait CodeCacheExt {
    /// モジュール全体の JIT コードを失效し、インタープリタにロールバック
    fn invalidate_module(&self, module_path: &str);

    /// ソースコード位置範囲に基づいて特定の関数を失效
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// モジュール全体の関数テーブルを原子置換
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// コンパイルキュー拡張インターフェース（予約、未実装）
trait CompileQueueExt {
    /// 優先挿入（ホットリロードコンパイルは通常の JIT コンパイルより優先）
    fn submit_priority(&self, task: CompileTask);
}
```

**なぜモジュール単位でのグループ化なのか？**
JIT 自体は関数のみを必要とする。モジュール単位での組織化はホットリロード 전용であり、モジュールの再コンパイル後、関数テーブル全体を原子的に置換でき、関数ごとに CAS する（旧関数間に循環依存がある場合、不整合状態を引き起こす可能性がある）ため。

## トレードオフ

### 利点

1. **コンパイル遅延のゼロ知覚**：Cranelift は 1-5ms/関数、バックグラウンドスレッドでコンパイル、インタープリタは一時停止しない
2. **インフラ共有**：JIT と AOT は IR 正規化 pass を共有（RFC-018 §4.0）、車輪の再開発なし
3. **非破壊的**：純粋な増分機能。VM は変更なし、インタープリタは変更なし、より高速な熱パスのみが追加
4. **LLVM 依存なし**：VM に LLVM を導入せず、軽量を維持
5. **マルチプラットフォーム対応**：Cranelift は x86_64 と ARM64 をネイティブサポート、全ターゲットプラットフォームをカバー
6. **ホットリロード予約**：コードキャッシュはモジュール単位のグループ化 + 関数入口の間接ジャンプにより、将来のホットリロードのための構造的基盤を提供

### 欠点

1. **Cranelift の新規依存**：新しい外部 crate を導入し、その API に熟悉する必要がある
2. **デバッグの複雑化**：JIT が生成したコードのスタックフレームはインタープリタのスタックフレームと互換性が必要であり、デバッグ情報マッピングに追加処理が必要
3. **コールドスタートの热度遅延**：プログラム起動後の最初の数秒間は JIT 高速化がなく、热度が蓄積されるまで待つ必要がある
4. **プラットフォーム ABI**：異なるプラットフォーム（Linux/macOS/Windows）の mmap と呼び出し規約はそれぞれ適応が必要

### 関連 RFC との整合性

| RFC                  | 整合性                                            |
| -------------------- | ------------------------------------------------- |
| RFC-018 LLVM AOT     | ✅ IR 正規化 pass を共有、JIT と AOT は対等なバックエンド     |
| RFC-024 spawn ブロック並行 | ✅ spawn ブロックはランタイム関数呼び出しにコンパイル                   |
| RFC-008 ランタイムアーキテクチャ | ✅ 三層ランタイム（Embedded/Standard/Full）はすべて JIT をサポート |

## 代替案

| 方案                     | 为什么不选                                         |
| ------------------------ | -------------------------------------------------- |
| LLVM AOT のみ、JIT なし  | 開発時にプログラム全体を再コンパイル必要、快速イテレーション体験を失う       |
| LLVM ORC JIT             | コンパイル遅延が高い（10-100ms）、LLVM 依存が大きい、VM に組み込むのに不適切 |
| カスタム軽量 JIT（dynasm）| ハンドコードバックエンドのメンテナンスコストが高い不如 Cranelift が成熟している |
| テンプレート JIT         | 最適化ゼロ、コード品質が悪い、JIT コンパイル時間を無駄にする           |
| 全プログラム JIT（インタープリタなし）   | コールドスタートが遅く、简单なスクリプトはコンパイルする価値がない                       |

## 依存関係

- RFC-018（LLVM AOT）→ IR 正規化 pass を共有
- RFC-024（spawn ブロック並行）→ spawn ブロックの JIT コンパイル
- RFC-008（ランタイムアーキテクチャ）→ 三層ランタイム JIT サポート
- Cranelift crate → JIT バックエンド

## 参考文献

- [Cranelift IR Documentation](https://github.com/bytecodealliance/wasmtools/tree/main/cranelift)
- [RFC-018：LLVM AOT コンパイラ設計](../accepted/018-llvm-aot-compiler.md)
- [RFC-024：spawn ブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
- [RFC-008：Runtime 並行モデルとスケジューラの分離設計](../accepted/008-runtime-concurrency-model.md)
- Hölzle, U. (1994). _Adaptive Optimization for Self: Reconciling High Performance with Exploratory
  Programming_. Stanford.

---

## ライフサイクルと行き先

| 状態       | 位置                            | 説明                   |
| ---------- | ------------------------------- | ---------------------- |
| **草案**   | `docs/src/design/rfc/draft/`    | 著者草案、レビュー提出待ち |
| **レビュー中** | `docs/src/design/rfc/review/`   | コミュニティ議論とフィードバックを募集中     |
| **承認済み** | `docs/src/design/rfc/accepted/` | 正式な設計ドキュメントとして成立       |