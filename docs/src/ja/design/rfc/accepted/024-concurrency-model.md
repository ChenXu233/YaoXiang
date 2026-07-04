```markdown
---
title: "RFC-024:spawnベースの並行ランタイムセマンティクス"
status: "承認済み（改訂版）"
author: "晨煦"
created: "2026-06-05"
updated: "2026-07-04（RFC-032と統合改訂：spawn修飾を任意の式に拡張）"
---

# RFC-024:spawnベースの並行ランタイムセマンティクス

> **本ドキュメントは`spawn`のランタイム動作セマンティクスを定義する**。
> 構文の直交性、AST/IR再構築、型システムの拡張については[RFC-032](./032-spawn-unified-expression.md)を参照。
>
> 2つのRFCは協調して`spawn`を定義する——024は「何をするか」、032は「どう表現するか」に答える。

> **参考**:
> - [並行モデル仕様](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime並行モデルとスケジューラの疎結合設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [RFC-010: 統一型構文](./010-unified-type-syntax.md)
> - [RFC-032: spawn統一式修飾 — AST/IR再構築](./032-spawn-unified-expression.md)

## 概要

本ドキュメントはYaoXiangプログラミング言語の`spawn`の**ランタイム動作セマンティクス**を定義する：`spawn <expr>`は唯一の並列プリミティブであり、任意の式を修飾でき、呼び出し側は同期的にブロックする。式の形状がタスク分解の粒度を決定し、ランタイムはGMPモデルに従ってスケジュールする——依存関係のないタスクはワークキューに投入され、workerが奪い合う。

**コア設計——1つのプリミティブ、1組のルール**：

```
spawn <expr>               ← 唯一の並列プリミティブ
タスク分解は式の形状が決定  ← 唯一のルール
結果の同期ブロック          ← 唯一の動作
```

**除去された複雑さ**：
- ❌ `@block`/`@eager`/`@auto`アノテーションなし
- ❌ `Send`/`Sync`トレイトなし
- ❌ `Mutex`/`RwLock`/`Atomic`なし
- ❌ `future`/非ブロッキングハンドルなし
- ❌ プログラム全体のDAG解析なし
- ❌ 関数の色付け（async/await）なし

> **ユーザーのメンタルモデル**：通常のコードは順次実行される。複数のことを同時に行いたい場合、それらを`spawn <expr>`の中に入れる。コールバックも`await`も奇妙なアノテーションもない。

## 設計の出所

| ドキュメント | 関係 |
|------|------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | 本文書により置き換え |
| [RFC-008](./008-runtime-concurrency-model.md) | ランタイムアーキテクチャ、本文書と直交 |
| [RFC-009](./009-ownership-model.md) | 所有権モデル、不変 |
| [RFC-010](./010-unified-type-syntax.md) | 統一型構文 |
| [RFC-032](./032-spawn-unified-expression.md) | AST/IR再構築、本文書と協調してspawnを定義 |

## 動機

### なぜこの設計が必要か？

現在の主流言語の並行モデルには明らかな欠陥がある：

| 言語 | 並行モデル | 問題 |
|------|----------|------|
| Rust | async/await + tokio | 非同期伝染、関数の色付け、学習曲線が急峻 |
| Go | goroutine | 型安全性なし、データ競合の検出が困難 |
| Python | asyncio | GIL制限、関数の色付け |
| JavaScript | Promise/async | コールバック地獄、関数の色付け |

### 旧設計（RFC-001）の問題点

RFC-001が提案した3層並行アーキテクチャ（L1/L2/L3）には以下の問題がある：

| 問題 | 説明 |
|------|------|
| メンタルモデルが複雑 | L1/L2/L3の3層抽象が学習負担を増大 |
| アノテーションが冗長 | `@block`/`@eager`/`@auto`アノテーションがコードを騒がしくする |
| 解析の複雑さ | プログラム全体のDAG解析のコンパイル時間コストが大きい |
| 型制約が複雑 | `Send`/`Sync`トレイトが認知負担を増大 |
| 制御不能 | 自動並行動作の予測とデバッグが困難 |

### 設計目標

1. **シンプル**：1つの並列プリミティブ（`spawn`）のみ、任意の式を修飾可能
2. **明示的**：ユーザーはどこが並列で、どこが順次かを明確に把握
3. **安全**：所有権ルールが自然に拡張、追加の型制約不要
4. **制御可能**：暗黙の並行なし、予期しない並行動作なし
5. **同期**：呼び出し側は同期的にブロック、コールバックや`await`なし

---

## 提案

### 1. {}ブロックの本質：依存駆動の計算ユニット

YaoXiangにおいて、`{}`は**依存駆動の計算ユニット**である。

| 属性 | 説明 |
|------|------|
| 依存駆動 | ブロックは実行時、内部の全変数の準備状況を確認し、揃っていれば即座に実行、そうでなければブロックして待機 |
| 実行タイミング | 依存関係によって決定、「即時」「遅延」とは無関係 |
| 戻り値 | `return`で明示的に値を返す；`return`がない場合、デフォルトで`Void`を返す |
| 構文の統一 | 関数本体・変数初期化・`spawn`後のどこに現れてもセマンティクスは一貫 |
| スコープの隔離 | 変数は厳密に`{}`内部に限定され、外側のスコープに漏洩しない |

```yaoxiang
// 依存駆動の例
x = compute_x()        // xは準備完了
y = compute_y()        // yは準備完了
result = {
    // xとyに依存し、両方の準備完了後、即座に実行
    return x + y
}
```

### 2. spawn式のセマンティクス

`spawn <expr>`はYaoXiangにおける**唯一の並列プリミティブ**である。任意の式を修飾でき、式の形状がタスク分解の粒度を決定する。

#### 2.1 タスク生成ルール

| 式の形状 | タスク分解 | 同期セマンティクス |
|-----------|---------|---------|
| `spawn { a, b, c }` | 直接の子式 → N個の独立タスク | 全タスクの完了を待機 |
| `spawn for x in items { body }` | 各反復 → 1タスク | 全反復の完了を待機 |
| `spawn while cond { body }` | 各反復ラウンド → 1タスク（反復間は条件駆動） | 条件がfalseになるまで待機 |
| `spawn if c { a } else { b }` | 条件cを順次評価、選択された分岐全体 → 1タスク | 選択された分岐の完了を待機 |
| `spawn call(x)` | 呼び出し自体 → 1タスク | 呼び出しの完了を待機 |
| `spawn expr`（任意の式） | 式自体 → 1タスク | 式の完了を待機 |

> **設計動機**：なぜspawnが任意の式を修飾できるのか？詳細は[RFC-032 §コア設計](./032-spawn-unified-expression.md)を参照。
>
> **制御フローの直交性**：`spawn <expr>`（spawnが前）と`<expr> spawn { body }`（spawnが後）のセマンティクス差は、[RFC-032 §制御フローの直交性](./032-spawn-unified-expression.md)（コア定義）を参照。逆順のすべての組み合わせ（`for ... spawn { }` / `while ... spawn { }` / `if ... spawn { }`）のランタイム動作——エラー伝播、リソース型、ネストルール——は本文§2.4 / §2.5 / §2.6のルールを継承する。

```yaoxiang
// spawnブロック：直接の子式を並列実行
(a, b) = spawn {
    t1 = fetch("url1")   // 直接の子式 → 並列タスク1
    t2 = fetch("url2")   // 直接の子式 → 並列タスク2
    return (t1, t2)      // 明示的にタプルを返す
}

// spawn for：各反復を並列実行
results = spawn for item in items {
    process(item)        // 各反復 → 独立タスク
}

// spawn while：各反復ラウンドを並列実行
spawn while has_next() {
    step()               // 各反復ラウンド → 独立タスク
}

// spawn if：選択された分岐全体を1タスクとして実行
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 スコープの隔離

spawn式は独立したスコープを作成し、内部の変数は外部に影響しない：

```yaoxiang
x = 10
result = spawn {
    x = 20              // これはspawn式内のローカルx
    compute(x)
}
// xは依然として10

result = spawn for item in items {
    item = item + 1     // 反復ローカルitem、各反復で独立したコピー
    process(item)
}
// 外側のitemは影響を受けない
```

**反復変数**（forの`x`）は各ラウンドで独立したコピーが作成され、反復終了時に自動的に破棄される。

#### 2.3 所有権ルール

変数がspawn式に入った後、外部では使用できなくなる（Moveセマンティクス）：

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // dataの所有権がspawn式に移動
}
// ここでのdataは使用不可（move済み）
```

複数のタスク間で共有する必要がある場合、`ref`を使用：

```yaoxiang
data = load_data()
shared = ref data       // コンパイラがRcまたはArcを自動選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**反復間共有**：`ref`で外側にキャプチャし、反復間で同じ参照を共有する。

#### 2.4 エラー伝播ルール

##### `spawn { a, b, c }`（ブロック）

1. 全タスクの完了を待機（一部のタスクが失敗していても）
2. 最初に遭遇したエラーを伝播
3. `?`でエラー伝播点を明示的にマーク

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗の可能性あり
    fetch("url2")?      // 失敗の可能性あり
}
// いずれかのタスクが失敗した場合、spawn式全体は最初のエラーを伝播
```

##### `spawn for x in items { body? }`

- 全反復の完了を待ってから最初のエラーを返す
- 失敗した反復後も残りの反復は**実行継続**（キャンセルしない）
- `?`でエラー伝播点を明示的にマーク

```yaoxiang
results = spawn for item in items {
    process(item)?      // いずれかの反復が失敗 → 全完了を待機 → 最初のエラーを伝播
}
```

##### `spawn while cond { body? }`

while自体のエラーセマンティクスを継承：

- stepで`?`がエラーを伝播 → spawn while全体が失敗、次のラウンドには進まない
- stepがエラーを伝播しない（エラーが飲み込まれる） → 次の反復ラウンドに進む

```yaoxiang
spawn while has_next() {
    item = next()       // エラーを伝播しない場合、失敗しても次のラウンドへ
    process(item)
}
```

##### `spawn if c { a } else { b }`

- 条件cを**順次評価**
- cの評価エラー → 全体のエラー
- 選択された分岐内のエラー → 全体のエラー

```yaoxiang
result = spawn if cond()? {  // condを順次評価、失敗 → 全体のエラー
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 リソース型ルール

コンパイラはリソース型の使用を追跡し、並行安全性を保証する：

| リソース型 | 説明 | コンパイラの動作 |
|----------|------|-----------|
| `FilePath` | ファイルシステムパス | 同一パスへの操作は自動的にシリアル化 |
| `HttpUrl` | HTTPエンドポイント | 同一URLへの操作は自動的にシリアル化 |
| `DBUrl` | データベース接続 | 同一接続への操作は自動的にシリアル化 |
| `Console` | 標準出力 | すべてのConsole操作は自動的にシリアル化 |

##### `spawn { ... }`ブロック内

```yaoxiang
// 同一ファイルへの操作は自動的にシリアル化
(a, b) = spawn {
    read_file("data.txt"),      // 先に実行
    write_file("data.txt", x)   // 読み取り完了を待機
}
```

##### `spawn for ... { ... }`反復をまたぐ同一リソース

すべての反復が同一リソース型を操作する場合、コンパイラは**自動的にシリアル実行に降格**する（spawnは通常のforに降格し、エラーを報告しない）：

```yaoxiang
// すべての反復が同一ファイルパスに書き込み → 自動的にシリアル実行
results = spawn for item in items {
    write_file("data.txt", item)
}
// コンパイラがすべての反復を自動的にシリアル化
```

> **設計理由**：spawnキーワードは依然として並列意図を表現する；リソース競合時、コンパイラが自動的に降格する方が、直接拒否するより最小驚きの原則に適合する。

##### `spawn while ... { ... }`の`&mut`キャプチャ

**コンパイル時エラー**：`spawn while`は外部変数の`&mut`型をキャプチャできない：

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // コンパイル時エラー
    item = iter.next()              // iterは&mut、反復をまたぐ可変共有 = データ競合
}
```

> **`Sync`トレイトを再導入しない**：RFC-024の「Send/Syncなし」コミットメントと一致する。ユーザーが`ref`または非spawn構文の使用を要求する。

##### `spawn if c { ... } else { ... }`の2分岐で同一リソース

**合法で警告なし**：if条件は相互排他的、最大1つの分岐しか実行されず、競合は発生しない：

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // 分岐1：cache読み取り
} else {
    fetch(key)                      // 分岐2：URL読み取り
}
```

#### 2.6 ネストされたspawn

spawn式はネストでき、内側は**独立した並行ドメイン**を作成する：

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**ネストのセマンティクス**：
- 内側のspawnは独立した並行ドメイン（独立したタスクキュー、独立したエラー伝播）
- 内側のエラーは外側に独立して伝播（外側タスクは内側完了待機時にエラーを受信）
- 内側のリソース型ルールは独立して追跡（外側と統合してチェックしない）

```yaoxiang
// spawn for内にspawn whileをネスト
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. 旧設計との決別

| 旧設計（RFC-001） | 新設計（RFC-024 + RFC-032） |
|------------------|---------------------------|
| プログラム全体の自動DAG解析 | spawn式内のみの解析 |
| `@block`/`@eager`/`@auto`アノテーション | アノテーションなし、依存駆動 |
| `Send`/`Sync`トレイト | 不要、所有権 + refで自動処理 |
| `future`/非ブロッキングハンドル | 同期ブロック、コールバックなし |
| `Mutex`/`RwLock`/`Atomic` | `ref`がRc/Arcを自動選択 |
| L1/L2/L3の3層メンタルモデル | 通常コードは順次、spawn式が並列 |
| 関数の色付け（async/await） | 関数の色付けなし |
| `spawn`は`{}`ブロックのみ修飾可能 | `spawn`は任意の式を修飾可能（RFC-032参照） |

### 4. returnルール

YaoXiangのreturnルールは統一され明確である：

| 書き方 | 戻り値 | 説明 |
|------|--------|------|
| `= expr`（波括弧なし） | `expr`を直接返す | 式が値そのもの |
| `= { ... }`（波括弧あり） | `return`が必須、なければ`Void`を返す | ブロックは明示的なreturnが必要 |

```yaoxiang
// 波括弧なし：直接返す
add: (a: Int, b: Int) -> Int = a + b

// 波括弧あり：returnが必須
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 波括弧あり、returnなし：Voidを返す
log: (message: String) -> Void = {
    print(message)  // returnなし、Voidを返す
}
```

### 5. ユーザーのメンタルモデル

> **通常のコードは順次実行される。**
>
> **複数のことを同時に行いたい場合、それらを`spawn <expr>`の中に入れる。**
>
> 式の形状がタスクの分解方法を決定する：ブロック内の各直接の子式は並列；forの各反復は並列；ifの選択された分岐は1タスクとして実行される。
>
> **spawn式全体は同期的にブロックし、全タスクの完了を待機する。**
>
> **コールバックも`await`も奇妙なアノテーションもない。**

```yaoxiang
// 通常コード：順次実行
a = compute_a()         // 先に実行
b = compute_b(a)        // aに依存、a完了後に実行
c = compute_c(b)        // bに依存、b完了後に実行

// 並列が必要な場合：spawnを使用
(x, y, z) = spawn {
    fetch("url1"),      // 並列
    fetch("url2"),      // 並列
    fetch("url3")       // 並列
}
// すべて完了を待機してから続行
process(x, y, z)

// データ並列：spawn for
results = spawn for item in items {
    process(item)
}
```

---

## トレードオフ

### 利点

1. **シンプル**：1つの並列プリミティブ（`spawn`）のみ、任意の式を修飾可能
2. **明示的**：ユーザーはどこが並列で、どこが順次かを明確に把握、暗黙の並行なし
3. **安全**：所有権ルールが自然に拡張、`Send`/`Sync`などの追加型制約不要
4. **制御可能**：自動並行動作なし、予期しない並行問題を回避
5. **同期**：呼び出し側は同期的にブロック、コードが理解しやすくデバッグしやすい
6. **関数の色付けなし**：async/awaitの関数の色付け問題が存在しない
7. **コンパイル効率**：DAG解析はspawn式内のみ、コンパイル時間が制御可能
8. **直交性**：spawnは任意の制御フロー構造と自然に組み合わせ可能（RFC-032詳細）

### 欠点

1. **明示的なspawnが必要**：自動並列化は不可、ユーザーが手動で並列ポイントをマークする必要がある
2. **spawn式内のDAG解析**：コンパイラがspawn式内で依存解析を行う必要がある
3. **旧コードとの非互換性**：旧RFC-001パターンを使用するコードは移行が必要

---

## 代替案

| 案 | 採用しない理由 |
|------|--------------|
| プログラム全体の自動DAG（RFC-001） | 複雑度が高く、コンパイル時間が長く、動作が制御不能 |
| async/await | 関数の色付け、学習曲線が急峻、コード可読性が低い |
| goroutine | 型安全性なし、データ競合の検出が困難 |
| Actorモデル | メッセージパッシングが複雑、デバッグが困難 |
| CSP（Go channel） | 型安全性なし、デッドロックの検出が困難 |
| `spawn`は`{}`ブロックのみ修飾可能 | 直交性を破壊し、`spawn for`が特殊ケースになる（RFC-032参照） |

---

## 実装戦略

### コンパイル時解析

1. **式形状の識別**：spawn後の式形状に従ってタスク分解を決定（RFC-032 §DAG解析詳細）
2. **DAG構築**：spawn式内の依存関係を解析
3. **トポロジカルソート**：spawn式内の実行順序を決定
4. **並列識別**：spawn式内の依存関係のないサブツリーを識別
5. **エスケープ解析**：`ref` → RcかArcか
6. **リソース競合検出**：リソース型の潜在的競合を検出

### モジュール構成

spawn関連コードは`frontend/core/spawn/`に統一配置：

```
frontend/core/spawn/
├── mod.rs           # spawnモジュールエントリ
├── placement.rs     # spawn出現位置の合法性チェック
└── analysis.rs      # タスク識別、依存解析、リソース競合検出
```

> **移行説明**（2026-06-11）：既存の`frontend/core/typecheck/passes/spawn_placement.rs`は`frontend/core/spawn/placement.rs`に移行される。`typecheck/passes/`ディレクトリ下の`spawn_placement`モジュール宣言は同期して削除する必要がある。

### ランタイム実行

[RFC-008](./008-runtime-concurrency-model.md)のRuntimeアーキテクチャを参照：

- **Embedded Runtime**：spawnサポートなし、即時実行
- **Standard Runtime**：spawn式をサポート
- **Full Runtime**：Standard + WorkStealer負荷分散

### 依存関係

- RFC-008（Runtimeアーキテクチャ）→ 完了
- RFC-009（所有権モデル）→ 完了
- RFC-010（統一型構文）→ 完了
- RFC-011（ジェネリクスシステム）→ 完了
- RFC-032（AST/IR再構築）→ 本文書と協調してspawnを定義

---

## 設計決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| 並列プリミティブ | `spawn <expr>` | シンプル、明示的、制御可能 | 2026-06-05 |
| spawn修飾範囲 | 任意の式 | 構文の直交性、`spawn for`の特殊化を除去 | 2026-07-04 |
| タスク分解 | 式の形状が決定 | 表現力が強い、ルールが統一 | 2026-07-04 |
| 実行モデル | 同期ブロック | 理解しやすく、デバッグしやすい | 2026-06-05 |
| DAG解析範囲 | spawn式内のみ | コンパイル効率、動作の制御可能性 | 2026-06-05 |
| 共有メカニズム | `ref`がRc/Arcを自動選択 | ユーザーの意思決定を簡素化 | 2026-06-05 |
| アノテーション | なし | コードのノイズを削減 | 2026-06-05 |
| Send/Sync | 削除 | 所有権 + refで十分 | 2026-06-05 |
| Mutex/RwLock | 削除 | refが自動処理 | 2026-06-05 |
| future/ハンドル | 削除 | 同期ブロックの方がシンプル | 2026-06-05 |
| 関数の色付け | なし | async/await問題を回避 | 2026-06-05 |
| リソース型 | 内蔵 + ユーザー定義 | 自動シリアル化 | 2026-06-05 |
| `spawn {}`エラー | 全完了待機、最初のエラーを伝播 | 決定論的動作 | 2026-06-05 |
| `spawn for`エラー | 全完了待機、最初のエラーを伝播 | `spawn {}`と一貫 | 2026-07-04 |
| `spawn while`エラー | whileエラーセマンティクスを継承 | while標準動作 | 2026-07-04 |
| `spawn if`条件エラー | cを順次評価、失敗 → 全体のエラー | 直感に適合 | 2026-07-04 |
| `spawn for`同一リソース | 自動的にシリアル実行に降格 | 安全な降格、粗暴な拒否を避ける | 2026-07-04 |
| `spawn while` `&mut`キャプチャ | コンパイル時エラー | データ競合を回避、Syncを再導入しない | 2026-07-04 |
| `spawn if`同一リソース | 合法で警告なし | 相互排他分岐は競合を構成しない | 2026-07-04 |
| ネストされたspawn | 内側は独立した並行ドメイン | 独立したタスクキュー、エラー、リソース | 2026-07-04 |

---

## 参考文献

### YaoXiang公式ドキュメント

- [並行モデル仕様](/reference/language-spec/concurrency.md)
- [RFC-001 並行モデル（廃止済み）](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime並行モデル](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [RFC-010 統一型構文](./010-unified-type-syntax.md)
- [RFC-011 ジェネリクスシステム](./011-generic-type-system.md)
- [RFC-032 spawn統一式修飾 — AST/IR再構築](./032-spawn-unified-expression.md)

### 外部参考

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## ライフサイクルと帰属

| 状態 | 位置 | 説明 |
|------|------|------|
| **承認済み（改訂版）** | `docs/design/rfc/accepted/` | RFC-032と協調してspawnを定義（ランタイムセマンティクス） |
```