---
title: "RFC-024: spawn ブロックベースの並行モデル"
status: "Accepted"
author: "晨煦"
created: "2026-06-05"
updated: "2026-06-11（spawn モジュールの構成と移行に関する記述を追加）"
---

# RFC-024: spawn ブロックベースの並行モデル

> **参考**:
> - [並行モデル仕様](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime 並行モデルとスケジューラの疎結合設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [RFC-010: 統一型構文](./010-unified-type-syntax.md)

## 要約

本文書は YaoXiang プログラミング言語の新しい並行モデルを定義するものである。`spawn {}` ブロックを唯一の並列 primitive とし、依存駆動による実行を行い、呼び出し側は同期的にブロックする。従来の `@block`/`@eager`/`@auto` 注釈、`Send`/`Sync` trait、プログラム全体に対する DAG 分析に基づく並行方式を置き換える。

**中核設計——1 つの primitive、1 つのルール**:

```
spawn { ... }        ← 唯一の並列 primitive
直接の子式がタスクを生成  ← 唯一のルール
呼び出し側は同期ブロック  ← 唯一の振る舞い
```

**除去された複雑性**:
- ❌ `@block`/`@eager`/`@auto` 注釈なし
- ❌ `Send`/`Sync` trait なし
- ❌ `Mutex`/`RwLock`/`Atomic` なし
- ❌ `future`/非ブロッキングハンドルなし
- ❌ プログラム全体に対する DAG 分析なし
- ❌ 関数の色付け（async/await）なし

> **ユーザのメンタルモデル**: 通常のコードは逐次実行される。複数のことを同時に行いたい場合は、それらを `spawn { ... }` ブロックに記述する。コールバックも `await` も奇妙な注釈もない。

## 設計の出典

| 文書 | 関係 |
|------|------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | 本文書により置換 |
| [RFC-008](./008-runtime-concurrency-model.md) | ランタイムアーキテクチャ。本文書とは直交する |
| [RFC-009](./009-ownership-model.md) | 所有権モデル。不変 |
| [RFC-010](./010-unified-type-syntax.md) | 統一型構文。return ルールを更新済み |
| [並行モデル仕様](/reference/language-spec/concurrency.md) | 本文書の正式な仕様リファレンス |

## 動機

### なぜこの設計が必要なのか？

現在の主流言語における並行モデルには明らかな欠陥がある。

| 言語 | 並行モデル | 問題 |
|------|----------|------|
| Rust | async/await + tokio | 非同期の伝染、関数の色付け、急峻な学習曲線 |
| Go | goroutine | 型安全性なし、データ競合の検出が困難 |
| Python | asyncio | GIL 制限、関数の色付け |
| JavaScript | Promise/async | コールバック地獄、関数の色付け |

### 旧設計（RFC-001）の問題点

RFC-001 で提案された 3 層並行アーキテクチャ（L1/L2/L3）には以下の問題がある。

| 問題 | 説明 |
|------|------|
| メンタルモデルが複雑 | L1/L2/L3 の 3 層抽象が学習負担を増大させる |
| 注釈が冗長 | `@block`/`@eager`/`@auto` 注釈がコードを騒がしくする |
| 分析の複雑度が高い | プログラム全体に対する DAG 分析はコンパイル時間のオーバーヘッドが大きい |
| 型制約が複雑 | `Send`/`Sync` trait が認知負荷を増大させる |
| 制御不能 | 自動並行動作の予測とデバッグが困難 |

### 設計目標

1. **シンプル**: 並列 primitive（`spawn`）は 1 つだけ、ルールは 1 つだけ（直接の子式がタスクを生成）
2. **明示的**: ユーザはどこが並列でどこが逐次かを明確に把握できる
3. **安全**: 所有権ルールが自然に拡張され、追加の型制約は不要
4. **制御可能**: 暗黙的な並行や予期しない並列動作がない
5. **同期**: 呼び出し側は同期的にブロックし、コールバックや `await` がない

---

## 提案

### 1. `{}` ブロックの本質：依存駆動の計算ユニット

YaoXiang において、`{}` は**依存駆動の計算ユニット**である。

| 属性 | 説明 |
|------|------|
| 依存駆動 | ブロックは実行時に内部のすべての変数が準備完了かをチェックし、すべて揃っていれば即座に実行し、そうでなければブロックして待機する |
| 実行タイミング | 「即時」「遅延」に関わらず、依存関係により決定される |
| 戻り値 | `return` で明示的に値を返す。`return` がない場合はデフォルトで `Void` を返す |
| 統一構文 | 関数本体、変数初期化、`spawn` 後のいずれに現れても意味は一貫している |
| スコープ分離 | 変数は `{}` の内部に厳密に限定され、外側のスコープには漏れない |

```yaoxiang
// 依存駆動の例
x = compute_x()        // x 準備完了
y = compute_y()        // y 準備完了
result = {
    // x と y に依存する。両方準備完了後、即座に実行
    return x + y
}
```

### 2. spawn ブロックのセマンティクス

`spawn { ... }` は YaoXiang における**唯一の並列 primitive** である。

#### 2.1 中核ルール

- spawn ブロックの**直接の子式**が並列タスクを生成する
- ネストされた `{}` 内の式は独立したタスクとは見なされない
- spawn ブロックは標準の return ルールに従う。`return` で明示的に値を返す必要があり、`return` がない場合は `Void` を返す
- spawn ブロック全体が同期的にブロックし、すべてのタスクの完了を待ってから戻る
- コールバック、`await`、注釈はない

```yaoxiang
// 2 つのタスクが並列実行される
(a, b) = spawn {
    t1 = fetch("url1")   // 直接の子式 → 並列タスク 1
    t2 = fetch("url2")   // 直接の子式 → 並列タスク 2
    return (t1, t2)      // タプルを明示的に返す
}

// ネストされた {} 内は直接の子式ではない
result = spawn {
    x = {               // このブロック全体が直接の子式 → 1 つのタスク
        inner_work()    // spawn の直接の子式ではないため、独立したタスクにはならない
    },
    process(x)          // 直接の子式 → 並列タスク
    return process(x)
}

#### 2.2 スコープ分離

spawn ブロックは独立したスコープを生成し、内部の変数は外部に影響しない。

```yaoxiang
x = 10
result = spawn {
    x = 20              // これは spawn ブロック内のローカルな x
    compute(x)
}
// x は依然として 10
```

#### 2.3 所有権ルール

変数が spawn ブロックに入った後、外部では使用できない（Move セマンティクス）。

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // data の所有権が spawn ブロックへ移動
}
// data はここでは使用不可（move 済み）
```

複数のタスク間で共有する必要がある場合は、`ref` を使用する。

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが自動的に Rc または Arc を選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

#### 2.4 エラー処理

spawn ブロック内のエラー伝播は以下のルールに従う。

1. すべてのタスクの完了を待つ（一部のタスクが失敗していても待つ）
2. 最初に出会ったエラーを伝播する
3. `?` を使用してエラー伝播点を明示的にマークする

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗する可能性あり
    fetch("url2")?      // 失敗する可能性あり
}
// いずれかのタスクが失敗した場合、spawn ブロックは最初のエラーを伝播する
```

#### 2.5 リソース型

コンパイラはリソース型の使用を追跡し、並行安全性を確保する。

| リソース型 | 説明 | コンパイラの振る舞い |
|----------|------|----------------|
| `FilePath` | ファイルシステムパス | 同一パスの操作を自動的に直列化 |
| `HttpUrl` | HTTP エンドポイント | 同一 URL の操作を自動的に直列化 |
| `DBUrl` | データベース接続 | 同一接続の操作を自動的に直列化 |
| `Console` | 標準出力 | すべての Console 操作を自動的に直列化 |

```yaoxiang
// 同一ファイルの操作は自動的に直列化される
(a, b) = spawn {
    read_file("data.txt"),      // 先に実行
    write_file("data.txt", x)   // 読み込み完了を待つ
}
```

#### 2.6 spawn for: データ並列ループ

```yaoxiang
// リストの各要素を並列処理する
results = spawn for item in items {
    result = process(item)
}
```

#### 2.7 spawn のネスト

spawn ブロックはネスト可能で、内側の spawn が新たな並行ドメインを生成する。

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

### 3. 旧設計との決別

| 旧設計（RFC-001） | 新設計（RFC-024） |
|------------------|------------------|
| プログラム全体の自動 DAG 分析 | spawn ブロック内のみの分析 |
| `@block`/`@eager`/`@auto` 注釈 | 注釈なし、依存駆動 |
| `Send`/`Sync` trait | 不要。所有権 + `ref` で自動的に処理 |
| `future`/非ブロッキングハンドル | 同期ブロック、コールバックなし |
| `Mutex`/`RwLock`/`Atomic` | `ref` が自動的に Rc/Arc を選択 |
| L1/L2/L3 の 3 層メンタルモデル | 通常コードは逐次、spawn ブロックは並列 |
| 関数の色付け（async/await） | 関数の色付けなし |

### 4. return ルール

YaoXiang の return ルールは統一され明確である。

| 書き方 | 戻り値 | 説明 |
|------|--------|------|
| `= expr`（波括弧なし） | `expr` を直接返す | 式がそのまま値 |
| `= { ... }`（波括弧あり） | `return` が必要。なければ `Void` を返す | ブロックは明示的な return を必要とする |

```yaoxiang
// 波括弧なし: 直接返す
add: (a: Int, b: Int) -> Int = a + b

// 波括弧あり: return が必須
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 波括弧ありで return なし: Void を返す
log: (message: String) -> Void = {
    print(message)  // return なし、Void を返す
}
```

### 5. ユーザのメンタルモデル

> **通常のコードは逐次実行される。**
>
> **複数のことを同時に行いたい場合は、それらを `spawn { ... }` ブロックに記述する。**
>
> ブロック内の各直接の子式は即座に開始され（並列）、`return` で結果を明示的に返す。
> ブロック全体はすべての完了を待ち、最終結果を取得する。
>
> **コールバックも `await` も奇妙な注釈もない。**

```yaoxiang
// 通常コード: 逐次実行
a = compute_a()         // 先に実行
b = compute_b(a)        // a に依存、a 完了後に実行
c = compute_c(b)        // b に依存、b 完了後に実行

// 並列が必要な場合: spawn を使用
(x, y, z) = spawn {
    fetch("url1"),      // 並列
    fetch("url2"),      // 並列
    fetch("url3")       // 並列
}
// すべて完了を待ってから続行
process(x, y, z)
```

---

## トレードオフ

### 利点

1. **シンプル**: 並列 primitive（`spawn`）は 1 つだけ、ルールは 1 つだけ（直接の子式がタスクを生成）
2. **明示的**: ユーザはどこが並列でどこが逐次かを明確に把握でき、暗黙的な並行がない
3. **安全**: 所有権ルールが自然に拡張され、`Send`/`Sync` などの追加の型制約が不要
4. **制御可能**: 自動的な並列動作がなく、予期しない並行問題を回避
5. **同期**: 呼び出し側は同期的にブロックし、コードの理解とデバッグが容易
6. **関数の色付けなし**: async/await における関数の色付け問題が存在しない
7. **コンパイル効率**: DAG 分析は spawn ブロック内に限定され、コンパイル時間が制御可能

### 欠点

1. **明示的な spawn が必要**: 自動並列化はできず、ユーザが手動で並列点をマークする必要がある
2. **spawn ブロック内の DAG 分析**: コンパイラは spawn ブロック内で依存分析を行う必要がある
3. **旧コードとの非互換性**: 旧 RFC-001 パターンを使用したコードは移行が必要

---

## 代替案

| 案 | 採用しない理由 |
|------|--------------|
| プログラム全体の自動 DAG（RFC-001） | 複雑度が高く、コンパイル時間が長く、動作が制御不能 |
| async/await | 関数の色付け、学習曲線が急峻、コード可読性が低い |
| goroutine | 型安全性なし、データ競合の検出が困難 |
| Actor モデル | メッセージパッシングが複雑、デバッグが困難 |
| CSP（Go チャネル） | 型安全性なし、デッドロックの検出が困難 |

---

## 実装戦略

### コンパイル時分析

1. **DAG 構築**: spawn ブロック内の依存関係を分析
2. **トポロジカルソート**: spawn ブロック内の実行順序を決定
3. **並列識別**: spawn ブロック内の非依存サブツリーを識別
4. **エスケープ分析**: `ref` → Rc か Arc かの判断
5. **リソース競合検出**: リソース型の潜在的競合を検出

### モジュール構成

spawn 関連のコードは `frontend/core/spawn/` に統一配置される。

```
frontend/core/spawn/
├── mod.rs           # spawn モジュールのエントリポイント
├── placement.rs     # spawn の出現位置の正当性チェック
└── analysis.rs      # タスク識別、依存分析、リソース競合検出（RFC-018 フェーズ 4 で必要）
```

> **移行に関する説明**（2026-06-11）: 既存の `frontend/core/typecheck/passes/spawn_placement.rs` は `frontend/core/spawn/placement.rs` へ移行される。`typecheck/passes/` ディレクトリ下の `spawn_placement` モジュール宣言も同期して削除する必要がある。この移行は RFC-018（LLVM AOT コンパイラ）によって推進される。LLVM バックエンドは spawn 分析結果を消費する必要があり、spawn 分析を typecheck に埋め込むよりも、独立したフロントエンド共有モジュールとする方が合理的である。

### ランタイム実行

[RFC-008](./008-runtime-concurrency-model.md) の Runtime アーキテクチャを参照。

- **Embedded Runtime**: spawn サポートなし、即时実行
- **Standard Runtime**: spawn ブロックをサポート、spawn ブロック内は並行実行
- **Full Runtime**: Standard + WorkStealer による負荷分散

### 依存関係

- RFC-008（Runtime アーキテクチャ）→ 完了済み
- RFC-009（所有権モデル）→ 完了済み
- RFC-010（統一型構文）→ 完了済み
- RFC-011（generics システム）→ 完了済み

---

## 設計上の決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| 並列 primitive | `spawn {}` ブロック | シンプル、明示的、制御可能 | 2026-06-05 |
| タスク生成 | 直接の子式 | 明確、曖昧さなし | 2026-06-05 |
| 実行モデル | 同期ブロック | 理解とデバッグが容易 | 2026-06-05 |
| DAG 分析範囲 | spawn ブロック内のみ | コンパイル効率、動作の制御性 | 2026-06-05 |
| 共有メカニズム | `ref` が Rc/Arc を自動選択 | ユーザの意思決定を簡素化 | 2026-06-05 |
| 注釈 | なし | コードのノイズを削減 | 2026-06-05 |
| Send/Sync | 削除 | 所有権 + `ref` で十分 | 2026-06-05 |
| Mutex/RwLock | 削除 | `ref` で自動処理 | 2026-06-05 |
| future/ハンドル | 削除 | 同期ブロックがよりシンプル | 2026-06-05 |
| 関数の色付け | なし | async/await 問題を回避 | 2026-06-05 |
| エラー伝播 | 全タスクを待ち、最初のエラーを伝播 | 決定論的な振る舞い | 2026-06-05 |
| リソース型 | 組み込み + ユーザ定義 | 自動直列化 | 2026-06-05 |

---

## 参考文献

### YaoXiang 公式ドキュメント

- [並行モデル仕様](/reference/language-spec/concurrency.md)
- [RFC-001 並行モデル（廃止）](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime 並行モデル](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [RFC-010 統一型構文](./010-unified-type-syntax.md)
- [RFC-011 generics システム](./011-generic-type-system.md)

### 外部リファレンス

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## ライフサイクルと帰属

| 状態 | 位置 | 説明 |
|------|------|------|
| **Accepted** | `docs/design/rfc/accepted/` | 正式な設計文書 |