---
title: 'RFC-024：spawn を基にした並行ランタイム意味論'
status: '承認済み（改訂版）'
author: '晨煦'
created: '2026-06-05'
updated: '2026-07-05（RFC 同期チェック：実装進捗 ~85%、コアランタイムと前端解析は完了）'

issue: '#89'
---

# RFC-024：spawn を基にした並行ランタイム意味論

> **このドキュメントは `spawn` のランタイム動作意味論を定義します**。構文の直交性、AST/IR リファクタリング、型システム拡張については
> [RFC-032](../review/032-spawn-unified-expression.md) を参照してください。
>
> 2つの RFC が協調して `spawn` を定義します — 024 は「何をするか」に答え、032 は「どう表現するか」に答えます。

> **参考**:
>
> - [並行モデル仕様](../../../../reference/language-spec/concurrency.md)
> - [RFC-008: ランタイム並行モデルとスケジューラの分離設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [RFC-010: 統一型構文](./010-unified-type-syntax.md)
> - [RFC-032: spawn 統一式修飾子 — AST/IR リファクタリング](../review/032-spawn-unified-expression.md)

## 概要

このドキュメントは YaoXiang プログラミング言語の `spawn` に関する**ランタイム動作意味論**を定義します：`spawn <expr>`
は唯一の並列プリミティブであり、任意の式を修飾でき、呼び出し元は同期的にブロックします。式の形状がタスク分解の粒度を決定し、ランタイムは GMP
モデルに従ってスケジュールします — 依存関係のないタスクはワークキューに投入され、worker が奪い合って実行します。

**コア設計 — 1つのプリミティブ、1組のルール**：

```
spawn <expr>               ← 唯一の並列プリミティブ
タスク分解は式の形状で決定  ← 唯一のルール
同期的にブロックして結果を待つ ← 唯一の動作
```

**排除した複雑さ**：

- ❌ `@block`/`@eager`/`@auto` アノテーションなし
- ❌ `Send`/`Sync` trait なし
- ❌ `Mutex`/`RwLock`/`Atomic` なし
- ❌ `future`/非同期Handles なし
- ❌ 全プログラム DAG 分析なし
- ❌ 関数の色分けなし（async/await）

> **ユーザーメンタルモデル**：書いた普通のコードは順番に実行されます。複数のことを同時に行いたい時は、`spawn <expr>`
> の中に入れます。コールバックはなく、`await`はなく、おかしなアノテーションもありません。

## 設計の出どころ

| ドキュメント                                                                   | 関係                              |
| ------------------------------------------------------------------------------ | --------------------------------- |
| [RFC-001](../../../../design/rfc/deprecated/001-concurrent-model-error-handling.md)       | 本書に置き換え                    |
| [RFC-008](./008-runtime-concurrency-model.md)                                  | ランタイムアーキテクチャ、本書と直交 |
| [RFC-009](./009-ownership-model.md)                                            | 所有権モデル、変更なし             |
| [RFC-010](./010-unified-type-syntax.md)                                        | 統一型構文                        |
| [RFC-032](../review/032-spawn-unified-expression.md)                           | AST/IR リファクタリング、本書と協調して spawn を定義 |

## 動機

### なぜこの設計が必要なのか？

現在の主流言語の並行モデルには明らかな欠陥があります：

| 言語       | 並行モデル            | 問題                             |
| ---------- | --------------------- | -------------------------------- |
| Rust       | async/await + tokio   | 非同期伝染、関数の色分け、学習曲線が急峻 |
| Go         | goroutine            | 型安全性がない、データ競合の検出が難しい |
| Python     | asyncio              | GIL の制限、関数の色分け          |
| JavaScript | Promise/async         | コールバック地獄、関数の色分け    |

### 旧設計（RFC-001）の問題

RFC-001 で提案された3層並行アーキテクチャ（L1/L2/L3）には以下の問題がありました：

| 問題         | 説明                                         |
| ------------ | -------------------------------------------- |
| メンタルモデルが難しい | L1/L2/L3 の3層抽象が追加の学習負担になる              |
| アノテーション冗長   | `@block`/`@eager`/`@auto` アノテーションでコードがうるさくなる |
| 分析の複雑さが高い   | 全プログラム DAG 分析のコンパイル時間オーバーヘッドが大きい          |
| 型制約が複雑         | `Send`/`Sync` trait が認知負担を増やす           |
| 制御不能             | 自動並行処理の動作が予測・ デバッグしにくい          |

### 設計目標

1. **シンプル**：`spawn` という1つの並列プリミティブのみ、任意の式を修飾可能
2. **明示的**：ユーザーがどこが並行でどこが順序的かを明確に把握
3. **安全**：所有権ルールが自然に延伸、追加の型制約が不要
4. **制御可能**：暗黙の並行処理なし、予期せぬ並列動作なし
5. **同期**：呼び出し元が同期的にブロック、コールバックも `await` もない

---

## 提案

### 1. {} ブロックの本質：依存関係駆動の計算単位

YaoXiang では、`{}` は**依存関係駆動の計算単位**です。

| 属性           | 説明                                                               |
| -------------- | ------------------------------------------------------------------ |
| 依存関係駆動   | ブロックは実行時に内部のすべての変数が準備完了かどうかを確認し、準備完了なら即時実行、なければブロックして待機 |
| 実行タイミング | 依存関係で決定され、「即時」か「遅延」かは無関係                                   |
| 戻り値         | `return` で明示的に値を返す；`return` がない場合はデフォルトで `Void` を返す            |
| 構文の統一     | 関数本体、変数初期化、`spawn` の後、出現場所に関係なく意味論が一貫している              |
| スコープの隔離 | 変数は `{}` 内部に厳密に制限され、外側のスコープには漏れない                         |

```yaoxiang
// 依存関係駆動の例
x = compute_x()        // x が準備完了
y = compute_y()        // y が準備完了
result = {
    // x と y に依存、どちらも準備完了なので即時実行
    return x + y
}
```

### 2. spawn 式意味論

`spawn <expr>` は YaoXiang における**唯一の並列プリミティブ**です。任意の式を修飾でき、式の形状がタスク分解の粒度を決定します。

#### 2.1 タスク作成ルール

| 式形状                       | タスク分解                                 | 同期意味論         |
| ---------------------------- | ------------------------------------------ | ------------------ |
| `spawn { a, b, c }`          | 直接部分式 → N 個の独立タスク              | すべてのタスク完了を待機 |
| `spawn for x in items { body }` | 毎回の反復 → 1 個のタスク                      | すべての反復完了を待機 |
| `spawn while cond { body }`  | 毎回の反復 → 1 個のタスク（反復間の条件駆動）    | 条件が false になるまで待機 |
| `spawn if c { a } else { b }` | 条件 c を順序評価、選択された分岐全体 → 1 個のタスク | 選択された分岐完了を待機 |
| `spawn call(x)`              | 呼び出し自体 → 1 個のタスク                      | 呼び出し完了を待機     |
| `spawn expr`（任意の式）     | 式自体 → 1 個のタスク                    | 式の完了を待機   |

> **設計動機**：なぜ spawn が任意の式を修飾できるのか？詳細は
> [RFC-032 §コア設計](../review/032-spawn-unified-expression.md) を参照。
>
> **制御フロー直交性**：`spawn <expr>`（spawn が前）と
> `<expr> spawn { body }`（spawn が後）の意味論の違いは、
> [RFC-032 §制御フロー直交性](../review/032-spawn-unified-expression.md)（コア定義）を参照。（`for ... spawn { }`
> / `while ... spawn { }` /
> `if ... spawn { }` のような）逆順のすべての組み合わせのランタイム動作 — エラー伝播、リソース型、ネストルール — は本文の §2.4 / §2.5 /
> §2.6 のルールを継承します。

```yaoxiang
// spawn ブロック：直接部分式が並行
(a, b) = spawn {
    t1 = fetch("url1")   // 直接部分式 → 並行タスク 1
    t2 = fetch("url2")   // 直接部分式 → 並行タスク 2
    return (t1, t2)      // 明示的にタプルを返す
}

// spawn for：毎回の反復が並行
results = spawn for item in items {
    process(item)        // 毎回の反復 → 独立タスク
}

// spawn while：毎回の反復が並行
spawn while has_next() {
    step()               // 毎回の反復 → 独立タスク
}

// spawn if：選択された分岐全体がタスク
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 スコープ隔離

spawn 式は独立したスコープを作成し、内部変数は外部に影響しません：

```yaoxiang
x = 10
result = spawn {
    x = 20              // これは spawn 式内の局所的な x
    compute(x)
}
// x は引き続き 10

result = spawn for item in items {
    item = item + 1     // 反復局所的な item、毎回の反復で独立コピー
    process(item)
}
// 外側の item は影響されない
```

**反復変数**（for の `x`）は毎回復興独立コピー、反復終了時に自動的に破棄されます。

#### 2.3 所有権ルール

変数が spawn 式に入ると、外部では使用できなくなります（Move 意味論）：

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // data の所有権が spawn 式に移動する
}
// data はここでは使用不可（move 済み）
```

複数のタスク間で共有する必要がある場合は、`ref` を使用します：

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが Rc または Arc を自動選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**反復間共有**：`ref` を使用して外側をキャプチャし、反復間で同一参照を共有します。

#### 2.4 エラー伝播ルール

##### `spawn { a, b, c }`（ブロック）

1. すべてのタスク完了を待機（一部のタスクが失敗しても）
2. 最初に遭遇したエラーを伝播
3. `?` を使用してエラー伝播点を明示的にマーク

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗可能性あり
    fetch("url2")?      // 失敗可能性あり
}
// いずれかのタスクが失敗すると、整个 spawn 式が最初のエラーを伝播
```

##### `spawn for x in items { body? }`

- すべての反復完了を待ってから最初のエラーを返す
- 失敗した反復の後、残りの反復は**引き続き実行**（キャンセルしない）
- `?` を使用してエラー伝播点を明示的にマーク

```yaoxiang
results = spawn for item in items {
    process(item)?      // いずれかの反復が失敗 → すべて完了を待機 → 最初のエラーを伝播
}
```

##### `spawn while cond { body? }`

while 自身のエラー意味論を継承：

- step が `?` でエラーを伝播 → 整个 spawn while が失敗、次の回に進まない
- step がエラーを伝播しない（エラーが飲み込まれる）→ 次の回の反復に進む

```yaoxiang
spawn while has_next() {
    item = next()       // エラーを伝播しない場合、失敗しても次の回に進む
    process(item)
}
```

##### `spawn if c { a } else { b }`

- 条件 c は**順序評価**
- c の評価でエラー → 整体がエラー
- 選択された分岐内でエラー → 整体がエラー

```yaoxiang
result = spawn if cond()? {  // cond は順序評価、失敗 → 整体がエラー
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 リソース型ルール

コンパイラはリソース型の使用を追跡し、並行安全性を確保します：

| リソース型   | 説明           | コンパイラ動作                |
| ---------- | ------------ | ------------------------- |
| `FilePath` | ファイルシステムパス | 同一パス操作は自動シリアル化        |
| `HttpUrl`  | HTTP エンドポイント    | 同一 URL 操作は自動シリアル化       |
| `DBUrl`    | データベース接続   | 同一接続操作は自動シリアル化        |
| `Console`  | 標準出力     | すべての Console 操作は自動シリアル化 |

##### `spawn { ... }` ブロック内

```yaoxiang
// 同一ファイルの操作は自動シリアル化
(a, b) = spawn {
    read_file("data.txt"),      // 先に実行
    write_file("data.txt", x)   // 読み取り完了を待機
}
```

##### `spawn for ... { ... }` 跨反復の同一リソース

すべての反復が同一リソース型を操作する場合、コンパイラは**自動的にシリアルに降格**（spawn が順序 for に退化、エラーなし）：

```yaoxiang
// すべての反復が同一ファイルパスに書き込み → 自動的にシリアルに降格
results = spawn for item in items {
    write_file("data.txt", item)
}
// コンパイラがすべての反復を自動シリアル化
```

> **設計理由**：spawn キーワードは並行意図を仍然表明しています；リソース競合時にコンパイラが自動降格するのは、直接拒否するよりも最小驚異原則に従っています。

##### `spawn while ... { ... }` `&mut` のキャプチャ

**コンパイル時エラー**：`spawn while` は `&mut` 型の外部変数のキャプチャを許可しません：

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // コンパイル時エラー
    item = iter.next()              // iter は &mut、反復間での可変共有 = データ競合
}
```

> **`Sync` trait を再導入しない**：RFC-024 の「Send/Sync なし」という約束と一致します。ユーザーに `ref` または spawn なしの改善策を使用させます。

##### `spawn if c { ... } else { ... }` 両分岐の同一リソース

**合法、警告なし**：if 条件は相互排他的、最大で1つの分岐だけが実行され、並行競合は存在しません：

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // 分岐 1：cache を読み取り
} else {
    fetch(key)                      // 分岐 2：URL を読み取り
}
```

#### 2.6 ネスト spawn

spawn 式はネスト可能で、内側の spawn は**独立した並行ドメイン**を作成します：

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**ネスト意味論**：

- 内側の spawn は独立した並行ドメイン（独立したタスクキュー、独立したエラー伝播）
- 内側のエラーは外側に独立して伝播（外側タスクが内側完了待機中にエラーを受け取る）
- 内側のリソース型ルールは独立して追跡（外側と jointly でチェックしない）

```yaoxiang
// spawn for 内に spawn while をネスト
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. 旧設計との決別

| 旧設計（RFC-001）              | 新設計（RFC-024 + RFC-032）          |
| ------------------------------ | ------------------------------------ |
| 全プログラム自動 DAG 分析            | spawn 式内のみ分析                |
| `@block`/`@eager`/`@auto` アノテーション | アノテーションなし、依存関係駆動                     |
| `Send`/`Sync` trait            | 不要、所有権 + ref で自動処理          |
| `future`/非同期Handles              | 同期ブロック、コールバックなし             |
| `Mutex`/`RwLock`/`Atomic`      | `ref` が Rc/Arc を自動選択                |
| L1/L2/L3 三層メンタルモデル          | 普通のコードは順序実行、spawn 式は並行       |
| 関数の色分け（async/await）        | 関数の色分けなし                           |
| `spawn` は `{}` ブロックのみ修飾可能         | `spawn` が任意の式を修飾可能（RFC-032 参照） |

### 4. 戻りルール

YaoXiang の戻りルールは統一されており明確です：

| 書き方                     | 戻り値                           | 説明           |
| ----------------------- | -------------------------------- | -------------- |
| `= expr`（波括弧なし）    | `expr` を直接返す                  | 式は値である     |
| `= { ... }`（波括弧あり） | `return` が必要、さもなくば `Void` | ブロックは明示的に返す必要がある |

```yaoxiang
// 波括弧なし：直接返す
add: (a: Int, b: Int) -> Int = a + b

// 波括弧あり：return が必要
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 波括弧ありだが return なし：Void を返す
log: (message: String) -> Void = {
    print(message)  // return なし、Void を返す
}
```

### 5. ユーザーメンタルモデル

> **書いた普通のコードは順番に実行されます。**
>
> **複数のことを同時に行いたい時は、`spawn <expr>` の中に入れます。**
>
> 式の形状がタスク分解の方法を決定します：ブロック内の各直接部分式は並行；for の各反復は並行；if の選択された分岐は1つのタスクとして。
>
> **整个 spawn 式は同期的にブロックし、すべてのタスク完了を待機します。**
>
> **コールバックはなく、`await`はなく、おかしなアノテーションもありません。**

```yaoxiang
// 普通のコード：順序実行
a = compute_a()         // 先に実行
b = compute_b(a)        // a に依存、a 完了後に実行
c = compute_c(b)        // b に依存、b 完了後に実行

// 並行が必要な時：spawn を使用
(x, y, z) = spawn {
    fetch("url1"),      // 並行
    fetch("url2"),      // 並行
    fetch("url3")       // 並行
}
// すべて完了後に続行
process(x, y, z)

// データ並列：spawn for
results = spawn for item in items {
    process(item)
}
```

---

## トレードオフ

### メリット

1. **シンプル**：`spawn` という1つの並列プリミティブのみ、任意の式を修飾可能
2. **明示的**：ユーザーがどこが並行でどこが順序的かを明確に把握、暗黙の並行処理なし
3. **安全**：所有権ルールが自然に延伸、`Send`/`Sync` などの追加型制約が不要
4. **制御可能**：自動並行処理がなく、予期せぬ並行問題が発生しない
5. **同期**：呼び出し元が同期的にブロック、コードの理解とデバッグが容易
6. **関数の色分けなし**：async/await の関数色分け問題が存在しない
7. **コンパイル効率が高い**：DAG 分析は spawn 式内に限定され、コンパイル時間が制御可能
8. **直交性**：spawn と任意の制御フロー構造が自然に組み合わせ可能（RFC-032 参照）

### デメリット

1. **明示的な spawn が必要**：自動並列化がなく、ユーザーが手動で並列点をマークする必要がある
2. **spawn 式内の DAG 分析**：コンパイラが spawn 式内で依存関係分析を行う必要がある
3. **旧コードとの互換性なし**：古い RFC-001 パターンを使用するコードは移行が必要

---

## 代替案

| 方案                       | なぜ選択しないか                                   |
| ------------------------- | ---------------------------------------------- |
| 全プログラム自動 DAG（RFC-001） | 複雑性が高い、コンパイル時間が長い、動作が制御不能               |
| async/await               | 関数の色分け、学習曲線が急峻、コード可読性が悪い           |
| goroutine                 | 型安全性がない、データ競合の検出が難しい                   |
| Actor モデル              | メッセージ渡しが複雑、デバッグが困難                     |
| CSP（Go channel）         | 型安全性がない、デッドロックの検出が難しい                  |
| `spawn` は `{}` ブロックのみ修飾可能    | 直交性を破壊し、`spawn for` が特例になる（RFC-032 参照） |

---

## 実装戦略

### コンパイル時分析

1. **式形状認識**：spawn 後の式の形状に応じてタスク分解を決定（RFC-032 §DAG 分析参照）
2. **DAG 構築**：spawn 式内の依存関係を分析
3. **トポロジカルソート**：spawn 式内の実行順序を決定
4. **並列認識**：spawn 式内で依存関係のないサブツリーを認識
5. **エスケープ分析**：`ref` → Rc か Arc か
6. **リソース競合検出**：リソース型の潜在的な競合を検出

### モジュール構成

spawn 関連コードは `frontend/core/spawn/` に統一して配置：

```
frontend/core/spawn/
├── mod.rs           # spawn モジュールエントリ
├── placement.rs     # spawn 出現位置の合法性チェック
└── analysis.rs      # タスク認識、依存関係分析、リソース競合検出
```

> **移行説明**（2026-06-11）：既存の `frontend/core/typecheck/passes/spawn_placement.rs` は
> `frontend/core/spawn/placement.rs` に移行します。`typecheck/passes/` ディレクトリ内の
> `spawn_placement` モジュール宣言も同時に削除する必要があります。

### ランタイム実行

[RFC-008](./008-runtime-concurrency-model.md) の Runtime アーキテクチャを参照：

- **Embedded Runtime**：spawn サポートなし、即時実行
- **Standard Runtime**：spawn 式をサポート
- **Full Runtime**：Standard + WorkStealer ロードバランシング

### 依存関係

- RFC-008（ランタイムアーキテクチャ）→ 完了
- RFC-009（所有権モデル）→ 完了
- RFC-010（統一型構文）→ 完了
- RFC-011（ジェネリクスシステム）→ 完了
- RFC-032（AST/IR リファクタリング）→ 本書と協調して spawn を定義

---

## 設計意思決定記録

| 意思決定                       | 決定                        | 理由                              | 日付       |
| ------------------------- | --------------------------- | --------------------------------- | ---------- |
| 並列プリミティブ                  | `spawn <expr>`              | シンプル、明示的、制御可能                  | 2026-06-05 |
| spawn 修飾範囲            | 任意の式                  | 構文の直交性、`spawn for` の特殊化を排除 | 2026-07-04 |
| タスク分解                  | 式の形状で決定            | 表現力が強く、ルールが統一                | 2026-07-04 |
| 実行モデル                  | 同期ブロック                    | 理解とデバッグが容易                    | 2026-06-05 |
| DAG 分析範囲              | spawn 式内のみ           | コンパイル効率が高い、動作が制御可能                | 2026-06-05 |
| 共有メカニズム                  | `ref` が Rc/Arc を自動選択         | ユーザーの判断を簡略化                      | 2026-06-05 |
| アノテーション                      | なし                          | コードノイズを削減                      | 2026-06-05 |
| Send/Sync                 | 削除                        | 所有権 + ref で十分                 | 2026-06-05 |
| Mutex/RwLock              | 削除                        | ref が自動処理                      | 2026-06-05 |
| future/Handles               | 削除                        | 同期ブロックの方がシンプル                    | 2026-06-05 |
| 関数の色分け                  | なし                          | async/await 問題を回避             | 2026-06-05 |
| リソース型                  | 組み込み + ユーザー定義           | 自動シリアル化                        | 2026-06-05 |
| `spawn {}` エラー          | すべて完了を待機、最初のエラーを伝播  | 決定論的動作                        | 2026-06-05 |
| `spawn for` エラー         | すべて完了を待機、最初のエラーを伝播  | `spawn {}` と一貫性                | 2026-07-04 |
| `spawn while` エラー       | while エラー意味論を継承         | while 標準動作                    | 2026-07-04 |
| `spawn if` 条件エラー      | c を順序評価、失敗 → 整体エラー | 直感に従っている                          | 2026-07-04 |
| `spawn for` 同一リソース   | 自動的にシリアルに降格              | 安全な降格、無理な拒否なし              | 2026-07-04 |
| `spawn while` `&mut` キャプチャ | コンパイル時エラー                  | データ競合を回避、Sync を導入しない             | 2026-07-04 |
| `spawn if` 同一リソース    | 合法、警告なし              | 相互排他的分岐は競合を構成しない                | 2026-07-04 |
| ネスト spawn               | 内側が独立した並行ドメイン              | 独立したタスクキュー、エラー、リソース          | 2026-07-04 |

---

## 参考文献

### YaoXiang 公式ドキュメント

- [並行モデル仕様](../../../../reference/language-spec/concurrency.md)
- [RFC-001 並作モデル（廃止）](../../../../design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 ランタイム並行モデル](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [RFC-010 統一型構文](./010-unified-type-syntax.md)
- [RFC-011 ジェネリクスシステム](./011-generic-type-system.md)
- [RFC-032 spawn 統一式修飾子 — AST/IR リファクタリング](../review/032-spawn-unified-expression.md)

### 外部参照

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## ライフサイクルと行き先

| 状態                 | 位置                        | 説明                                    |
| -------------------- | --------------------------- | --------------------------------------- |
| **承認済み（改訂版）** | `docs/design/rfc/accepted/` | RFC-032 と協調して spawn を定義（ランタイム意味論） |