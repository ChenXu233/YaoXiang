---
title: 'RFC-024：spawnに基づく並列ランタイム意味論'
status: '承認済（改訂版）'
author: '晨煦'
created: '2026-06-05'
updated: '2026-07-05（RFC同期チェック：実装進捗約85%、コアランタイムとフロントエンド解析完了）'

issue: '#89'
---

# RFC-024：spawnに基づく並列ランタイム意味論

> **本文書は `spawn`
> のランタイム動作意味論を定義する**。構文の直交性、AST/IR 再構築、型システム拡張については
> [RFC-032](../review/032-spawn-unified-expression.md) を参照。
>
> 2つのRFCは協調して `spawn` を定義する——024は「何をするか」、032は「どう表現するか」を答える。

> **参考**:
>
> - [並列モデル仕様](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime並列モデルとスケジューラ疎結合設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [RFC-010: 統一型構文](./010-unified-type-syntax.md)
> - [RFC-032: spawn統一式修飾 — AST/IR再構築](../review/032-spawn-unified-expression.md)

## 要約

本文書はYaoXiangプログラミング言語の `spawn` の**ランタイム動作意味論**を定義する：`spawn <expr>`
は唯一の並列プリミティブであり、任意の式を修飾でき、呼び出し側は同期的にブロックする。式の形状がタスク分解の粒度を決定し、ランタイムはGMPモデルに従ってスケジュールする——依存関係のないタスクはワークキューに投入され、workerが奪い合う。

**核心設計——1つのプリミティブ、1組のルール**：

```
spawn <expr>               ← 唯一の並列プリミティブ
タスク分解は式の形状で決定    ← 唯一のルール
結果を同期的にブロック待機    ← 唯一の動作
```

**除去された複雑性**：

- ❌ `@block`/`@eager`/`@auto` 注釈
- ❌ `Send`/`Sync` trait
- ❌ `Mutex`/`RwLock`/`Atomic`
- ❌ `future`/ノンブロッキングハンドル
- ❌ プログラム全体DAG解析
- ❌ 関数カラーリング（async/await）

> **ユーザーメンタルモデル**：通常のコードは順次実行される。複数のことを同時に行いたい場合、それらを
> `spawn <expr>` の中に入れる。コールバックも `await` も奇妙な注釈もない。

## 設計の出所

| ドキュメント                                                             | 関係                                    |
| ------------------------------------------------------------------------ | --------------------------------------- |
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | 本文により置換                          |
| [RFC-008](./008-runtime-concurrency-model.md)                            | ランタイムアーキテクチャ、本文と直交    |
| [RFC-009](./009-ownership-model.md)                                      | 所有権モデル、変更なし                  |
| [RFC-010](./010-unified-type-syntax.md)                                  | 統一型構文                              |
| [RFC-032](../review/032-spawn-unified-expression.md)                     | AST/IR再構築、本文と協調してspawnを定義 |

## 動機

### なぜこの設計が必要か？

現在の主流言語の並列モデルには明らかな欠陥がある：

| 言語       | 並列モデル          | 問題                                         |
| ---------- | ------------------- | -------------------------------------------- |
| Rust       | async/await + tokio | 非同期伝染、関数カラーリング、学習曲線が急峻 |
| Go         | goroutine           | 型安全性なし、データ競合の検出が困難         |
| Python     | asyncio             | GIL制限、関数カラーリング                    |
| JavaScript | Promise/async       | コールバック地獄、関数カラーリング           |

### 旧設計（RFC-001）の問題

RFC-001で提案された3層並列アーキテクチャ（L1/L2/L3）には以下の問題がある：

| 問題                 | 説明                                                 |
| -------------------- | ---------------------------------------------------- |
| メンタルモデルが複雑 | L1/L2/L3の3層抽象が学習負担を増加                    |
| 注釈が冗長           | `@block`/`@eager`/`@auto` 注釈がコードを騒がしくする |
| 解析の複雑度が高い   | プログラム全体DAG解析のコンパイル時間コストが大きい  |
| 型制約が複雑         | `Send`/`Sync` traitが認知負荷を増加                  |
| 制御不能             | 自動並列動作の予測とデバッグが困難                   |

### 設計目標

1. **シンプル**：並列プリミティブは1つだけ（`spawn`）、任意の式を修飾可能
2. **明示的**：ユーザーはどこが並列でどこが逐次的かを明確に把握
3. **安全**：所有権ルールが自然に拡張され、追加の型制約が不要
4. **制御可能**：暗黙の並列なし、予期しない並列動作なし
5. **同期**：呼び出し側は同期的にブロック、コールバックや `await` なし

---

## 提案

### 1. {} ブロックの本質：依存駆動の計算ユニット

YaoXiangにおいて、`{}` は**依存駆動の計算ユニット**である。

| 属性           | 説明                                                                                                             |
| -------------- | ---------------------------------------------------------------------------------------------------------------- |
| 依存駆動       | ブロックは実行時に内部のすべての変数の準備状況を確認し、揃っていれば即座に実行、揃っていなければブロックして待機 |
| 実行タイミング | 依存関係により決定され、「即時」「遅延」とは無関係                                                               |
| 戻り値         | `return` で明示的に値を返す；`return` がない場合デフォルトで `Void` を返す                                       |
| 構文の統一性   | 関数本体、変数初期化、`spawn` 後のいずれに出現しても意味が一貫                                                   |
| スコープ分離   | 変数は厳密に `{}` 内部に限定され、外側のスコープに漏れない                                                       |

```yaoxiang
// 依存駆動の例
x = compute_x()        // x が準備完了
y = compute_y()        // y が準備完了
result = {
    // x と y に依存し、両方が準備完了後に即座に実行
    return x + y
}
```

### 2. spawn 式意味論

`spawn <expr>`
はYaoXiangにおける**唯一の並列プリミティブ**である。任意の式を修飾でき、式の形状がタスク分解の粒度を決定する。

#### 2.1 タスク生成ルール

| 式の形状                        | タスク分解                                      | 同期意味論                  |
| ------------------------------- | ----------------------------------------------- | --------------------------- |
| `spawn { a, b, c }`             | 直接の子式 → N個の独立タスク                    | 全タスクの完了を待機        |
| `spawn for x in items { body }` | 各反復 → 1タスク                                | 全反復の完了を待機          |
| `spawn while cond { body }`     | 各反復 → 1タスク（反復間は条件で駆動）          | 条件が false になるまで待機 |
| `spawn if c { a } else { b }`   | 条件cを順次評価、選択された分岐全体を → 1タスク | 選択された分岐の完了を待機  |
| `spawn call(x)`                 | 呼び出し自体 → 1タスク                          | 呼び出しの完了を待機        |
| `spawn expr`（任意の式）        | 式自体 → 1タスク                                | 式の完了を待機              |

> **設計動機**：なぜspawnは任意の式を修飾できるのか？詳細は
> [RFC-032 §核心設計](../review/032-spawn-unified-expression.md) を参照。
>
> **制御フローの直交性**：`spawn <expr>`（spawnが先）と
> `<expr> spawn { body }`（spawnが後）の意味論上の差異については
> [RFC-032 §制御フローの直交性](../review/032-spawn-unified-expression.md)（核心定義）を参照。逆方向のすべての組み合わせ（`for ... spawn { }`
> / `while ... spawn { }` /
> `if ... spawn { }`）のランタイム動作——エラー伝播、リソース型、ネストルール——は本RFC §2.4 / §2.5 /
> §2.6 のルールを継承する。

```yaoxiang
// spawn ブロック：直接の子式を並列化
(a, b) = spawn {
    t1 = fetch("url1")   // 直接の子式 → 並列タスク1
    t2 = fetch("url2")   // 直接の子式 → 並列タスク2
    return (t1, t2)      // タプルを明示的に返す
}

// spawn for：各反復を並列化
results = spawn for item in items {
    process(item)        // 各反復 → 独立タスク
}

// spawn while：各反復を並列化
spawn while has_next() {
    step()               // 各反復 → 独立タスク
}

// spawn if：選択された分岐全体を1タスクに
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 スコープ分離

spawn 式は独立したスコープを作成し、内部の変数は外部に影響しない：

```yaoxiang
x = 10
result = spawn {
    x = 20              // これはspawn式内のローカルx
    compute(x)
}
// x は依然として 10

result = spawn for item in items {
    item = item + 1     // 反復ローカルなitem、各反復で独立コピー
    process(item)
}
// 外側の item は影響を受けない
```

**反復変数**（forの `x`）は各反復で独立コピーされ、反復終了時に自動破棄される。

#### 2.3 所有権ルール

変数がspawn式に入った後、外部では使用できなくなる（Move意味論）：

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // data の所有権がspawn式に移動
}
// data はここでは使用不可（move済み）
```

複数のタスク間で共有する必要がある場合は `ref` を使用する：

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが自動的にRcまたはArcを選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**反復間共有**：`ref` でキャプチャして外側に保持し、反復間で同じ参照を共有する。

#### 2.4 エラー伝播ルール

##### `spawn { a, b, c }`（ブロック）

1. すべてのタスクの完了を待機（一部のタスクが失敗しても）
2. 最初に遭遇したエラーを伝播
3. `?` でエラー伝播点を明示

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗の可能性あり
    fetch("url2")?      // 失敗の可能性あり
}
// いずれかのタスクが失敗した場合、spawn式全体は最初のエラーを伝播
```

##### `spawn for x in items { body? }`

- すべての反復完了後に最初のエラーを返す
- 失敗した反復後も残りの反復は**実行継続**（キャンセルしない）
- `?` でエラー伝播点を明示

```yaoxiang
results = spawn for item in items {
    process(item)?      // いずれかの反復が失敗 → 全完了を待機 → 最初のエラーを伝播
}
```

##### `spawn while cond { body? }`

while自身のエラー意味論を継承：

- stepが `?` でエラーを伝播 → spawn while 全体が失敗、次の反復には進まない
- stepがエラーを伝播しない（エラーが飲み込まれる）→ 次の反復へ進む

```yaoxiang
spawn while has_next() {
    item = next()       // エラーを伝播しない場合、失敗しても次の反復へ
    process(item)
}
```

##### `spawn if c { a } else { b }`

- 条件cは**順次評価**
- cの評価エラー → 全体エラー
- 選択された分岐内のエラー → 全体エラー

```yaoxiang
result = spawn if cond()? {  // cond を順次評価、失敗 → 全体エラー
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 リソース型ルール

コンパイラはリソース型の使用を追跡し、並列安全性を保証する：

| リソース型 | 説明                 | コンパイラの動作                    |
| ---------- | -------------------- | ----------------------------------- |
| `FilePath` | ファイルシステムパス | 同一パスの操作は自動的に直列化      |
| `HttpUrl`  | HTTPエンドポイント   | 同一URLの操作は自動的に直列化       |
| `DBUrl`    | データベース接続     | 同一接続の操作は自動的に直列化      |
| `Console`  | 標準出力             | すべてのConsole操作を自動的に直列化 |

##### `spawn { ... }` ブロック内

```yaoxiang
// 同一ファイルの操作は自動的に直列化される
(a, b) = spawn {
    read_file("data.txt"),      // 先に実行
    write_file("data.txt", x)   // 読み取り完了を待機
}
```

##### `spawn for ... { ... }` 反復をまたぐ同一リソース

すべての反復が同一リソース型に対して操作する場合、コンパイラは**自動的に直列に降格**する（spawnが逐次forに降格、エラーなし）：

```yaoxiang
// すべての反復が同一ファイルパスへの書き込み → 自動的に直列に降格
results = spawn for item in items {
    write_file("data.txt", item)
}
// コンパイラがすべての反復を自動的に直列化する
```

> **設計理由**：spawnキーワードは依然として並列意図を表現する；リソース競合時にコンパイラが自動的に降格することは、直接拒否するよりも最小驚きの原則に適う。

##### `spawn while ... { ... }` における `&mut` キャプチャ

**コンパイル時エラー**：`spawn while` は外部の `&mut` 型変数のキャプチャを許可しない：

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // コンパイル時エラー
    item = iter.next()              // iter は &mut、反復をまたぐ可変共有 = データ競合
}
```

> **`Sync`
> trait を再導入しない**：RFC-024の「Send/Syncなし」コミットメントと一貫している。ユーザーに `ref`
> への変更または非spawn記述を要求する。

##### `spawn if c { ... } else { ... }` 両分岐の同一リソース

**合法、警告なし**：if条件は排他的であり、最大1つの分岐しか実行されないため、競合は発生しない：

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // 分岐1：cache 読み取り
} else {
    fetch(key)                      // 分岐2：URL 読み取り
}
```

#### 2.6 spawn のネスト

spawn 式はネスト可能で、内層は**独立した並列ドメイン**を作成する：

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**ネストの意味論**：

- 内側のspawnは独立した並列ドメイン（独立したタスクキュー、独立したエラー伝播）
- 内側のエラーは独立して外側に伝播される（外側タスクは内側完了時にエラーを受信）
- 内側のリソース型ルールは独立して追跡される（外側と結合してチェックされない）

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

| 旧設計（RFC-001）                 | 新設計（RFC-024 + RFC-032）               |
| --------------------------------- | ----------------------------------------- |
| プログラム全体自動DAG解析         | spawn式内のみ解析                         |
| `@block`/`@eager`/`@auto` 注釈    | 注釈なし、依存駆動                        |
| `Send`/`Sync` trait               | 不要、所有権 + ref で自動処理             |
| `future`/ノンブロッキングハンドル | 同期ブロック、コールバックなし            |
| `Mutex`/`RwLock`/`Atomic`         | `ref` で自動的にRc/Arcを選択              |
| L1/L2/L3 の3層メンタルモデル      | 通常コードは逐次、spawn式で並列           |
| 関数カラーリング（async/await）   | 関数カラーリングなし                      |
| `spawn` は `{}` ブロックのみ修飾  | `spawn` は任意の式を修飾（RFC-032を参照） |

### 4. return ルール

YaoXiangのreturnルールは統一され明確である：

| 記述                      | 戻り値                                | 説明                           |
| ------------------------- | ------------------------------------- | ------------------------------ |
| `= expr`（波括弧なし）    | `expr` を直接返す                     | 式がそのまま値                 |
| `= { ... }`（波括弧あり） | `return` 必須、なければ `Void` を返す | ブロックは明示的なreturnが必要 |

```yaoxiang
// 波括弧なし：直接返す
add: (a: Int, b: Int) -> Int = a + b

// 波括弧あり：returnが必須
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 波括弧あり、return なし：Void を返す
log: (message: String) -> Void = {
    print(message)  // return なし、Void を返す
}
```

### 5. ユーザーのメンタルモデル

> **通常のコードは順次実行される。**
>
> **複数のことを同時に行いたい場合、それらを `spawn <expr>` の中に入れる。**
>
> 式の形状がタスクの分解方法を決定する：ブロック内の各直接の子式は並列、forの各反復は並列、ifの選択された分岐は1つのタスク。
>
> **spawn式全体は同期的にブロックし、すべてのタスクの完了を待機する。**
>
> **コールバックも `await` も奇妙な注釈もない。**

```yaoxiang
// 通常コード：順次実行
a = compute_a()         // 先に実行
b = compute_b(a)        // a に依存、a 完了後に実行
c = compute_c(b)        // b に依存、b 完了後に実行

// 並列が必要な場合：spawn を使用
(x, y, z) = spawn {
    fetch("url1"),      // 並列
    fetch("url2"),      // 並列
    fetch("url3")       // 並列
}
// すべて完了するまで待機してから続行
process(x, y, z)

// データ並列：spawn for
results = spawn for item in items {
    process(item)
}
```

---

## トレードオフ

### 利点

1. **シンプル**：並列プリミティブは1つだけ（`spawn`）、任意の式を修飾可能
2. **明示的**：ユーザーはどこが並列でどこが逐次的かを明確に把握、暗黙の並列なし
3. **安全**：所有権ルールが自然に拡張され、`Send`/`Sync` などの追加の型制約が不要
4. **制御可能**：自動並列動作なし、予期しない並列問題を回避
5. **同期**：呼び出し側は同期的にブロック、コードが理解・デバッグしやすい
6. **関数カラーリングなし**：async/await の関数カラーリング問題が存在しない
7. **コンパイル効率が高い**：DAG解析はspawn式内に限定され、コンパイル時間が制御可能
8. **直交性**：spawnは任意の制御フロー構造と自然に組み合わせ可能（RFC-032を参照）

### 欠点

1. **明示的なspawnが必要**：自動並列化できず、ユーザーが手動で並列点をマークする必要
2. **spawn式内DAG解析**：コンパイラはspawn式内で依存解析を行う必要がある
3. **旧コードとの非互換性**：旧RFC-001パターンを使用するコードは移行が必要

---

## 代替案

| 案                               | 選択しない理由                                       |
| -------------------------------- | ---------------------------------------------------- |
| プログラム全体自動DAG（RFC-001） | 複雑、コンパイル時間長、動作の予測不能               |
| async/await                      | 関数カラーリング、学習曲線が急峻、コード可読性が悪い |
| goroutine                        | 型安全性なし、データ競合の検出が困難                 |
| Actorモデル                      | メッセージパッシングが複雑、デバッグが困難           |
| CSP（Go channel）                | 型安全性なし、デッドロックの検出が困難               |
| `spawn` は `{}` ブロックのみ修飾 | 直交性の破壊、`spawn for` が特例化（RFC-032を参照）  |

---

## 実装戦略

### コンパイル時解析

1. **式形状の識別**：spawn後の式形状に応じてタスク分解を決定（RFC-032 §DAG解析を参照）
2. **DAG構築**：spawn式内の依存関係を解析
3. **トポロジカルソート**：spawn式内の実行順序を決定
4. **並列識別**：spawn式内の依存関係のない部分木を識別
5. **エスケープ解析**：`ref` → Rc か Arc か
6. **リソース競合検出**：リソース型の潜在的競合を検出

### モジュール構成

spawn関連コードは `frontend/core/spawn/` に統一配置する：

```
frontend/core/spawn/
├── mod.rs           # spawn モジュールエントリ
├── placement.rs     # spawn 出現位置の正当性チェック
└── analysis.rs      # タスク識別、依存解析、リソース競合検出
```

> **移行説明**（2026-06-11）：既存の `frontend/core/typecheck/passes/spawn_placement.rs` は
> `frontend/core/spawn/placement.rs` に移行される。`typecheck/passes/` ディレクトリ下の
> `spawn_placement` モジュール宣言は同期して削除する必要がある。

### ランタイム実行

[RFC-008](./008-runtime-concurrency-model.md) のRuntimeアーキテクチャを参照：

- **Embedded Runtime**：spawnサポートなし、即時実行
- **Standard Runtime**：spawn式をサポート
- **Full Runtime**：Standard + WorkStealer ロードバランシング

### 依存関係

- RFC-008（Runtimeアーキテクチャ）→ 完了
- RFC-009（所有権モデル）→ 完了
- RFC-010（統一型構文）→ 完了
- RFC-011（ジェネリクスシステム）→ 完了
- RFC-032（AST/IR再構築）→ 本文と協調してspawnを定義

---

## 設計決定記録

| 決定                            | 決定内容                         | 理由                                     | 日付       |
| ------------------------------- | -------------------------------- | ---------------------------------------- | ---------- |
| 並列プリミティブ                | `spawn <expr>`                   | シンプル、明示的、制御可能               | 2026-06-05 |
| spawn修飾範囲                   | 任意の式                         | 構文の直交性、`spawn for` の特殊化を除去 | 2026-07-04 |
| タスク分解                      | 式の形状で決定                   | 表現力が強い、ルールが統一               | 2026-07-04 |
| 実行モデル                      | 同期ブロック                     | 理解・デバッグが容易                     | 2026-06-05 |
| DAG解析範囲                     | spawn式内のみ                    | コンパイル効率が高い、動作が制御可能     | 2026-06-05 |
| 共有メカニズム                  | `ref` で自動的にRc/Arcを選択     | ユーザー判断の簡略化                     | 2026-06-05 |
| 注釈                            | なし                             | コードのノイズを削減                     | 2026-06-05 |
| Send/Sync                       | 削除                             | 所有権 + ref で十分                      | 2026-06-05 |
| Mutex/RwLock                    | 削除                             | ref で自動処理                           | 2026-06-05 |
| future/ハンドル                 | 削除                             | 同期ブロックの方がシンプル               | 2026-06-05 |
| 関数カラーリング                | なし                             | async/await の問題を回避                 | 2026-06-05 |
| リソース型                      | 組み込み + ユーザー定義          | 自動直列化                               | 2026-06-05 |
| `spawn {}` エラー               | 全完了を待機、最初のエラーを伝播 | 決定的な動作                             | 2026-06-05 |
| `spawn for` エラー              | 全完了を待機、最初のエラーを伝播 | `spawn {}` と一貫                        | 2026-07-04 |
| `spawn while` エラー            | while のエラー意味論を継承       | while の標準動作                         | 2026-07-04 |
| `spawn if` 条件エラー           | c を順次評価、失敗 → 全体エラー  | 直感に適う                               | 2026-07-04 |
| `spawn for` 同一リソース        | 自動的に直列に降格               | 安全な降格、粗暴な拒否ではない           | 2026-07-04 |
| `spawn while` `&mut` キャプチャ | コンパイル時エラー               | データ競合回避、Syncを再導入しない       | 2026-07-04 |
| `spawn if` 同一リソース         | 合法、警告なし                   | 排他的分岐は競合を構成しない             | 2026-07-04 |
| spawn のネスト                  | 内側は独立した並列ドメイン       | 独立したタスクキュー、エラー、リソース   | 2026-07-04 |

---

## 参考文献

### YaoXiang 公式ドキュメント

- [並列モデル仕様](/reference/language-spec/concurrency.md)
- [RFC-001 spawnモデル（廃止済）](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime並列モデル](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [RFC-010 統一型構文](./010-unified-type-syntax.md)
- [RFC-011 ジェネリクスシステム](./011-generic-type-system.md)
- [RFC-032 spawn統一式修飾 — AST/IR再構築](../review/032-spawn-unified-expression.md)

### 外部参考

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## ライフサイクルと帰属

| 状態                 | 位置                        | 説明                                             |
| -------------------- | --------------------------- | ------------------------------------------------ |
| **承認済（改訂版）** | `docs/design/rfc/accepted/` | RFC-032と協調してspawnを定義（ランタイム意味論） |
