---
title: 'RFC-028: JITコンパイラ — VM内マルチレベル実行エンジン'
status: '草案'
author: '晨煦'
created: '2026-06-11'
updated: '2026-07-05'
issue: '#101'
---

# RFC-028: JITコンパイラ — VM内マルチレベル実行エンジン

> **参考**:
>
> - [RFC-018: LLVM AOTコンパイラ設計](../accepted/018-llvm-aot-compiler.md)
> - [RFC-024: spawnブロックベースの並行モデル](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime並行モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)

## 概要

本文書では、YaoXiangのVMバックエンドにCranelift
JITコンパイラを導入し、VMを純粋なインタプリタから**マルチレベル実行エンジン**に昇格させることを提案する。コールドコードはインタプリタ実行し、ホット関数はCraneliftによってネイティブコードにコンパイルされる。JITパスはRFC-018のLLVM
AOTパスとIR正規化パスを共有し、CraneliftはJITの高速コンパイルを担当し、LLVMはAOTの深い最適化を担当し、各々の長所を活かす。

**核心的な位置付け: JITはVMに奉仕するものであり、VMの代替ではない。**

## 動機

### なぜJITが必要なのか?

現在のVMバックエンドは純粋なインタプリタであり、実行速度はネイティブコードより10〜100倍遅い。開発時には頻繁にテストやスクリプト、ローカルデバッグを実行する — これらのシナリオではAOTの極限までの最適化は必要ないが、インタプリタより明らかに高速な実行速度が必要である。

### なぜLLVM AOTだけを使わないのか?

LLVM
AOTコンパイルは時間がかかり(秒単位)、開発のイテレーションには適していない。開発には「変更したらすぐ実行」という体験が必要だ:
1行コードを変更 → 再実行 → ほぼ即座に結果を確認できる。Cranelift
JITは単一関数のコンパイルに1〜5msしかかからず、ユーザーはコンパイルの遅延を感じない。

### なぜCraneliftでLLVM ORC JITではないのか?

| 観点           | Cranelift JIT                    | LLVM ORC JIT                     |
| -------------- | -------------------------------- | -------------------------------- |
| コンパイル速度 | 1〜5ms/関数                      | 10〜100ms/関数                   |
| 依存サイズ     | 小                               | 大(完全なLLVMが必要)             |
| コード品質     | LLVM -O2の70〜80%                | 極めて高い                       |
| 適用シナリオ   | 開発デバッグ、高速イテレーション | 適用不可(本文のトレードオフ参照) |

Craneliftはコンパイルが速く、コード品質も十分だ。LLVMはAOTのオフライン深い最適化のために残しておく。一つのツールが一つの仕事をうまくやる。

## 提案

### 中核アーキテクチャ

```
VM実行エンジン
├── インタプリタ層
│   ├── バイトコード命令を実行
│   ├── ホットネスデータを収集 (invocation count + loop backedge count)
│   └── 閾値に到達 → コンパイルタスクを投入
│
├── JITコンパイル層 (Craneliftバックエンド)
│   ├── コンパイルキュー(バックグラウンドスレッド、インタプリタをブロックしない)
│   ├── IR → 正規化 → Cranelift IR → ネイティブコード
│   └── RFC-018 §4.0のIR正規化パスを再利用(スタック→SSA)
│
├── コードキャッシュ
│   ├── 関数表: 関数ID → {インタプリタ入口, JIT入口(オプション)}
│   ├── コンパイル済み関数入口のアトミック置換
│   └── モジュールごとにグループ化(ホットリロード用インターフェースを予約)
│
└── ホットネス分析
    ├── 関数ごとの呼び出し回数 + ループバックエッジ回数
    ├── 定期減衰(一回のウォームアップでコンパイルがトリガーされるのを防ぐ)
    └── 3段階のホットネス: Cold → Warm → Hot → Compiled
```

### 既存アーキテクチャとの統合

```
ソースコード → フロントエンド(共有)→ IR → ┬→ バイトコードcodegen → VMインタプリタ → [ホット関数] → Cranelift JIT
                                            │
                                            └→ LLVM AOT codegen → .o → リンク → exe(本番)
```

JITとAOTは**IR正規化パス**(`middle/passes/ir_normalize.rs`)を共有し、低レベルのcodegenをLLVMからCraneliftに置き換える。

### 実行フロー

```
関数呼び出し
  → fn_entry.code_ptr.load()
  → ┬─ インタプリタスタブ(コールド状態): バイトコードを1命令ずつ解釈
    └─ JITネイティブコード(ホット状態): 直接機械コードを実行
  → 戻る
```

## 詳細設計

### 1. ディレクトリ構造

```
src/
├── backends/
│   ├── interpreter/              # 既存 — VMインタプリタ
│   │   └── executor/
│   │       ├── engine.rs         # 変更 — 呼び出し入口を直接解釈からFunctionEntryディスパッチに変更
│   │       └── ...
│   │
│   ├── jit/                      # 新規 — JITコンパイル層
│   │   ├── mod.rs                # JITモジュール入口、Craneliftコンテキストの初期化
│   │   ├── profiler.rs           # ホットネスカウント + 減衰 + 閾値判定
│   │   ├── entry.rs              # FunctionEntry + AtomicPtr管理
│   │   ├── cache.rs              # コードキャッシュ(mmap実行可能ページ管理)
│   │   ├── compiler.rs           # IR → Cranelift IR → ネイティブコード
│   │   ├── types.rs              # YaoXiang型 → Cranelift型マッピング
│   │   └── abi.rs                # 関数呼び出し規約(System V / Microsoft x64)
│   │
│   ├── llvm/                     # 計画中 — LLVM AOT(RFC-018)
│   ├── common/                   # 既存
│   └── runtime/                  # 既存
│
└── middle/
    └── passes/
        └── ir_normalize.rs       # 新規 — 共有IR正規化(スタック→SSA)
                                  #   JITとLLVM AOTで共有
```

**重要な制約**:

- `backends/jit/` は `middle/`(IR定義、正規化パス)、標準ライブラリ、Cranelift crateのみに依存
- `backends/jit/` は `backends/llvm/` に依存しない、両者は同レベルのバックエンド
- `backends/jit/` は `backends/interpreter/`
  に依存しない、`FunctionEntry`インターフェース経由でやりとり

### 2. ホットネス分析と階層的トリガー

#### 2.1 ホットネス状態機械

```
Cold ──(invocation > 50 または backedge > 500)──→ Warm
Warm ──(invocation > 200)────────────────────→ Hot
Hot ──(コンパイルキュー投入、コンパイル完了)──────────────────→ Compiled
```

> 閾値は設定可能であり、上記はデフォルト値。LuaJIT、JVM C1、V8
> Sparkplugの実閾値範囲(50〜1000)を参考。

#### 2.2 カウンタ

各関数は`FunctionEntry`(詳細は§4.1参照)で2つのアトミックカウンタを保持する:

```rust
// FunctionEntryのホットネスフィールド(完全な定義は§4.1を参照)
invocation_count: AtomicU32,   // 関数が呼び出された回数
backedge_count: AtomicU32,     // ループバックエッジのジャンプ回数
state: AtomicU8,              // Cold | Warm | Hot | Compiled
```

#### 2.3 減衰メカニズム

5秒ごとに全カウンタを1ビット右シフトする(0.5倍)。起動時に高頻度だが一度しか実行されないコード(初期化走査など)が無意味なJITコンパイルをトリガーするのを防ぐ。

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

ビット演算を使用し、除算のオーバーヘッドはゼロ。

#### 2.4 コンパイルキュー

```
インタプリタスレッド                    バックグラウンドJITスレッド
    │                                       │
    ├─ ホットネスがHotに到達                  │
    ├─ コンパイル要求を投入 ─────────────────→  │
    │  (インタプリタをブロックしない)             ├─ 関数のIRを取り出す
    │                                       ├─ IR正規化 (スタック→SSA)
    │                                       ├─ Craneliftコンパイル
    │                                       ├─ コードキャッシュに書き込み
    │                                       └─ 関数入口ポインタをアトミックに更新
    │  次回この関数を呼び出し ←─────────────────  │
    │  ネイティブコードを直接実行                 │
```

コンパイル中も関数はインタプリタで実行される。コンパイル完了後、次回の呼び出しでアトミックにJITコードに切り替わる。

### 3. IR → Craneliftコンパイルパイプライン

#### 3.1 パイプライン

```
YaoXiang IR(スタック形式)
  → IR正規化パス(スタック → レジスタ/SSA)    ← RFC-018 §4.0を再利用
  → Cranelift IR構築
  → Cranelift最適化 + 機械コード生成
  → コードキャッシュに書き込み
```

#### 3.2 YaoXiang型 → Cranelift型

| YaoXiang型   | Cranelift型              | 説明                                  |
| ------------ | ------------------------ | ------------------------------------- |
| `Int`        | `i64`                    |                                       |
| `Int32`      | `i32`                    |                                       |
| `Float`      | `f64`                    |                                       |
| `Float32`    | `f32`                    |                                       |
| `Bool`       | `i8`                     | Craneliftには`i1`がないため`i8`を使用 |
| `Char`       | `i32`                    | Unicodeコードポイント                 |
| `String`     | `{ i64, i64 }`           | ポインタ + 長さ                       |
| `Void`       | 空タプル                 |                                       |
| `&T`         | —                        | ゼロサイズ、コンパイル後消滅          |
| `&mut T`     | —                        | ゼロサイズ、コンパイル後消滅          |
| `ref T`      | `{ i64, i64 }`           | 参照カウントポインタ + データポインタ |
| `*T`         | `i64`                    | 生ポインタ                            |
| `List(T)`    | `{ i64, i64, i64 }`      | データポインタ + 長さ + 容量          |
| 構造体       | Cranelift struct         |                                       |
| レコードenum | `{ i64, [max_payload] }` | タグ + union                          |
| `?T`         | `{ i8, T }`              | 値ありマーカー + データ               |

> RFC-018 §3のLLVM型表との比較:
> Craneliftはポインタ型を区別せず、`i1`もなく、全体としてよりシンプル。

#### 3.3 主要命令の翻訳

| IR命令                     | Cranelift IR                                         |
| -------------------------- | ---------------------------------------------------- |
| `Add { dst, lhs, rhs }`    | `iadd`(整数)/ `fadd`(浮動小数点)                     |
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
| `Spawn { ... }`            | ランタイムの`task_spawn` + `task_wait_all`を呼び出し |

> 完全な翻訳表はRFC本文を参照。核心原則: Cranelift命令セットはYaoXiang
> IRの全操作をカバーし、意味的な欠落は存在しない。

#### 3.4 2種類の正規化の共存

VMインタプリタはスタックセマンティクス(`Push`/`Pop`/`Dup`/`Swap`)が必要で、Cranelift JITとLLVM
AOTはレジスタ/SSAが必要。IR正規化パスが一度変換を行い(RFC-018
§4.0)、JITとAOTで共有し、IR自体の表現は変更しない。各バックエンドは自身のニーズに応じて同じIRを消費する。

### 4. 関数入口テーブルとアトミック置換

#### 4.1 FunctionEntry

```rust
struct FunctionEntry {
    /// アトミックに交換可能な実行ターゲット
    code_ptr: AtomicPtr<u8>,
    /// 不変メタデータ
    bytecode: &'static [u8],        // インタプリタfallback
    ir: &'static FunctionIR,        // JITコンパイルの入力
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
  → ┬─ インタプリタスタブアドレス → インタプリタ実行、バイトコードを1命令ずつ解釈
    └─ JITコードアドレス           → ネイティブコードに直接ジャンプ
```

ポインタ間接参照は1回。近代的なCPUの分岐予測器の間接ジャンプ処理: 最初は予測ミス、それ以降は全て正解。オーバーヘッドは約1サイクル。

#### 4.3 アトミック切り替え

コンパイル完了後、1回のCAS:

```rust
fn install_jit_code(entry: &FunctionEntry, jit_code: *mut u8) -> bool {
    entry.code_ptr.compare_exchange(
        INTERPRETER_STUB,      // 期待値: まだインタプリタを指している
        jit_code,              // 置換値: JITコード
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_ok()
}
```

インタプリタの一時停止なし、セーフポイント待ちなし、呼び出し点の走査なし。1つのアトミック操作で切り替えが完了。

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
      native_pages:   [ mmap済み実行可能メモリページ ]
    "lib.yao":
      functions:
        "helper"     → FunctionEntry (state: Compiled)
      native_pages:   [ mmap済み実行可能メモリページ ]
```

#### 5.2 実行可能メモリ管理

```rust
struct NativePage {
    ptr: *mut u8,
    size: usize,
    used: AtomicUsize,     // 使用済みバイト数
    remaining: usize,       // 残りの容量
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // モジュール無効化時のみ呼び出し
}
```

各モジュールは連続するmmap実行可能ページを割り当て、モジュール内の全JIT関数は同じページから割り当てられる。モジュール無効化時はページ全体を回収し、関数ごとの解放は不要。

### 6. ホットリロード用拡張ポイントの予約

以下のインターフェースはコンパイルは通すが、ホットリロード実装前は呼び出されない。インターフェース設計原則:
**JIT実装時は`insert`と単一関数の`compare_exchange`のみ必要で、モジュールレベルの操作はホットリロードに任せる。**

```rust
/// コードキャッシュ拡張インターフェース(予約、実装なし)
trait CodeCacheExt {
    /// モジュール全体の全JITコードを無効化し、インタプリタにフォールバック
    fn invalidate_module(&self, module_path: &str);

    /// ソースコードの位置範囲に基づいて特定の関数を無効化
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// モジュール全体の関数表をアトミックに置換
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// コンパイルキュー拡張インターフェース(予約、実装なし)
trait CompileQueueExt {
    /// 優先度插队(ホットリロードコンパイルが通常のJITコンパイルより優先)
    fn submit_priority(&self, task: CompileTask);
}
```

**なぜモジュールごとにグループ化するか?**
JIT自体には関数しか必要ない。モジュール単位の組織化は完全にホットリロードのため: モジュール再コンパイル後、関数ごとにCASする代わりに、モジュール全体の関数セットをアトミックに置換できる — 関数ごとにCASすると関数間に循環依存がある場合に不整合状態になる。

## トレードオフ

### 利点

1. **ゼロ認識コンパイル遅延**: Cranelift
   1〜5ms/関数、バックグラウンドスレッドでコンパイル、インタプリタは停止しない
2. **インフラ共有**: JITとAOTはIR正規化パス(RFC-018 §4.0)を共有、車輪の再発明をしない
3. **非破壊的**: 純粋な追加機能。VMは変わらず、インタプリタは変わらず、より高速なホットパスが増えるだけ
4. **LLVM依存なし**: VMにLLVMを導入しない、軽量さを維持
5. **自然にマルチプラットフォーム対応**:
   Craneliftはx86_64とARM64をネイティブサポート、全ターゲットプラットフォームをカバー
6. **ホットリロード予約**: コードキャッシュのモジュール別グループ化 + 関数入口の間接ジャンプで、将来のホットリロードの構造的基盤を構築

### 欠点

1. **Craneliftの新しい依存**: 新しい外部crateを導入し、そのAPIに習熟が必要
2. **デバッグの複雑さ**:
   JIT生成コードのスタックフレームをインタプリタのスタックフレームと互換性を持たせる必要があり、デバッグ情報のマッピングに追加処理が必要
3. **コールドスタートのホットネス遅延**: プログラム起動後の最初の数秒はJITアクセラレーションがなく、ホットネスの蓄積が必要
4. **プラットフォームABI**: 異なるプラットフォーム(Linux/macOS/Windows)のmmapと呼び出し規約をそれぞれ対応付ける必要

### 関連RFCとの一貫性

| RFC                              | 一貫性                                                    |
| -------------------------------- | --------------------------------------------------------- |
| RFC-018 LLVM AOT                 | ✅ IR正規化パスを共有、JITとAOTは同レベルのバックエンド   |
| RFC-024 spawnブロック並行        | ✅ spawnブロックはランタイム関数呼び出しにコンパイル      |
| RFC-008 ランタイムアーキテクチャ | ✅ 3層ランタイム(Embedded/Standard/Full)すべてJITサポート |

## 代替案

| 案                                  | 選ばない理由                                                              |
| ----------------------------------- | ------------------------------------------------------------------------- |
| LLVM AOTのみ、JITなし               | 開発時にプログラム全体の再コンパイルが必要、高速イテレーション体験を失う  |
| LLVM ORC JIT                        | コンパイル遅延が大きい(10〜100ms)、LLVM依存が大きい、VMへの埋め込みに不適 |
| カスタム軽量JIT(dynasm)             | 手書きバックエンドの保守コストが高い、Craneliftほど成熟していない         |
| テンプレートJIT                     | ゼロ最適化、コード品質が悪い、JITコンパイルの時間を無駄にする             |
| フルプログラムJIT(インタプリタなし) | コールドスタートが遅く、簡単なスクリプトはコンパイルする価値がない        |

## 依存関係

- RFC-018(LLVM AOT)→ IR正規化パスを共有
- RFC-024(spawnブロック並行)→ spawnブロックのJITコンパイル
- RFC-008(ランタイムアーキテクチャ)→ 3層ランタイムのJITサポート
- Cranelift crate → JITバックエンド

## 参考文献

- [Cranelift IR Documentation](https://github.com/bytecodealliance/wasmtools/tree/main/cranelift)
- [RFC-018: LLVM AOTコンパイラ設計](../accepted/018-llvm-aot-compiler.md)
- [RFC-024: spawnブロックベースの並行モデル](../accepted/024-concurrency-model.md)
- [RFC-008: Runtime並行モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)
- Hölzle, U. (1994). _Adaptive Optimization for Self: Reconciling High Performance with Exploratory
  Programming_. Stanford.

---

## ライフサイクルと帰属

| 状態         | 位置                            | 説明                                       |
| ------------ | ------------------------------- | ------------------------------------------ |
| **草案**     | `docs/src/design/rfc/draft/`    | 作者の草稿、提出審査待ち                   |
| **審査中**   | `docs/src/design/rfc/review/`   | オープンなコミュニティ議論とフィードバック |
| **承認済み** | `docs/src/design/rfc/accepted/` | 正式な設計ドキュメントになる               |
