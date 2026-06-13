# 並行性モデル仕様

> **ステータス**：本文書は YaoXiang 言語の新しい並行性モデル設計について記述するものである。これは古い、`@block`/`@eager`/`@auto` 注釈、`Send`/`Sync` trait、`Mutex`/`RwLock` に基づく並行性方案を置き換える。一部の内容はまだ実装されておらず、実際のコンパイラの動作に従うこと。

本ファイルは YaoXiang プログラミング言語の並行性モデル仕様を定義するものであり、`{}` ブロックのセマンティクス、`spawn` 並行性プリミティブ、エラー処理、資源型を含む。

---

## 第一章：概要

### 1.1 {} ブロックの本質

YaoXiang において、`{}` は**依存駆動の計算ユニット**である。

| 属性 | 説明 |
|------|------|
| 依存駆動 | ブロックは実行時に内部のすべての変数が準備完了かをチェックし、揃っていれば即座に実行し、そうでなければブロックして待機する |
| 実行タイミング | 依存関係によって決定され、「即時」または「遅延」とは無関係 |
| 戻り値 | `return` を使用して明示的に値を返す；`return` がない場合はデフォルトで `Void` を返す |
| 構文の統一 | 関数本体、変数初期化、`spawn` 後のいずれに現れても、セマンティクスは一貫している |
| スコープの隔離 | 変数は厳密に `{}` 内部に限定され、外側のスコープに漏出しない |

```yaoxiang
// 依存駆動の例
x = compute_x()        // x は準備完了
y = compute_y()        // y は準備完了
result = {
    // x と y に依存し、両者が準備完了後に即座に実行
    return x + y
}
```

### 1.2 返却ルール

YaoXiang の返却ルールは統一され明確である：

| 書き方 | 戻り値 | 説明 |
|------|--------|------|
| `= expr`（波括弧なし） | `expr` を直接返す | 式がそのまま値 |
| `= { ... }`（波括弧あり） | `return` が必要、なければ `Void` を返す | ブロックは明示的な return を必要とする |

```yaoxiang
// 波括弧なし：直接返す
add: (a: Int, b: Int) -> Int = a + b

// 波括弧あり：return が必須
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 波括弧ありだが return なし：Void を返す
log: (message: String) -> Void = {
    print(message)  // return なし、Void を返す
}
```

### 1.3 spawn ブロックのセマンティクス

`spawn { ... }` は YaoXiang における唯一の並列プリミティブである。

**核心ルール**：
- spawn ブロックの**直接の子代入**は並列タスクを作成する
- ネストされた `{}` 内の代入は独立したタスクと見なされない
- spawn ブロック全体は同期的にブロックし、すべてのタスクが完了するまで待機して結果を返す
- コールバック、`await`、注釈は存在しない

```yaoxiang
// 2 つのタスクが並列実行される
(a, b) = spawn {
    fetch("url1"),      // タスク 1
    fetch("url2")       // タスク 2
}
// 両方の完了を待ってから続行
```

### 1.4 ユーザーのメンタルモデル

> 通常のコードは逐次実行される。
> 複数のことを同時に行いたいときは、それらを `spawn { ... }` ブロックに入れる。
> ブロック内の各直接代入は即座に開始され（並列）、必要な結果は自動的に待機される。
> ブロック全体はすべての作業が完了するのを待ち、最終結果を提供する。
> コールバックも `await` も、奇妙な注釈もない。

---

## 第二章：構文とセマンティクス

### 2.1 通常コード

通常コード（spawn ブロックの外部）は**逐次実行**される。

```yaoxiang
a = compute_a()     // 先に実行
b = compute_b(a)    // a に依存、a 完了後に実行
c = compute_c(b)    // b に依存、b 完了後に実行
```

### 2.2 spawn ブロック

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**セマンティクス**：
1. spawn ブロック内の直接の子代入は独立したタスクとして並列実行される
2. 各タスクの結果は対応するパターン変数に束縛される
3. ブロックはすべてのタスクが完了するまでブロックする
4. すべての結果のタプルを返す

```yaoxiang
// 単一タスク
result = spawn {
    fetch("url")
}

// 複数タスク
(a, b, c) = spawn {
    fetch("url1"),
    fetch("url2"),
    fetch("url3")
}
```

### 2.3 関数本体内の spawn

関数本体自体は `{}` ブロックであり、その中で `spawn` を使用できる。

```yaoxiang
fetch_and_parse: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        parsed = parse(fetch(url))
    }
    return results
}
```

### 2.4 ループ内の spawn

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
```

**セマンティクス**：データ並列ループで、各反復が独立したタスクとなる。

```yaoxiang
// リストの各要素を並列処理
results = spawn for item in items {
    result = process(item)
}
```

> **注意**：`spawn for` のループ本体は独立したタスクであり、反復をまたぐ可変状態の共有はサポートされない。結果を集約する必要がある場合は、`spawn for` で結果を集めた後、外部で処理すべきである（下記の例を参照）。

```yaoxiang
// 正しい方法：並列処理後に外部で集約
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // 逐次集約
```

### 2.5 ネストされた spawn

spawn ブロックはネストでき、内側の spawn は新しい並行性ドメインを作成する。

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**注意**：内側の spawn の直接の子代入のみがタスクであり、外側の spawn はそれを貫通しない。

---

## 第三章：所有権モデルとのインタラクション

### 3.1 Move セマンティクス

Move は YaoXiang のデフォルトのセマンティクスである（ゼロコピー）。

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // data の所有権が spawn ブロックへ移される
}
// data はここでは使用不可（move 済み）
```

**ルール**：
- 変数が spawn ブロックに入った後、外部では使用できない
- 複数のタスク間で共有する必要がある場合は、`ref` を使用する

### 3.2 借用トークン

`&T` と `&mut T` はゼロサイズのコンパイル時の権限証明であり、**タスク境界を跨ぐことはできない**。

```yaoxiang
data = load_data()
ref_data = &data

// コンパイルエラー：借用トークンはタスクを跨げない
result = spawn {
    process(ref_data)   // エラー！
}
```

### 3.3 ref 共有

`ref` はスコープを跨いで共有する唯一の方法である。

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが Rc または Arc を自動選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**コンパイラの選択（保守的戦略）**：
| 条件 | 選択 |
|------|------|
| デフォルト | `Arc`（安全性優先） |
| コンパイラが単一タスク内でのみ使用されると証明できる場合 | `Rc`（アトミック操作のオーバーヘッドなし） |

### 3.4 クロージャのキャプチャ

クロージャのキャプチャ = Move、1 つのクロージャは 1 つのタスクにのみ使用できる。

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // クロージャが data を move キャプチャ

// コンパイルエラー：クロージャは 1 つのタスクにのみ使用可能
result = spawn {
    fn(1),      // クロージャを使用
    fn(2)       // エラー！クロージャは move 済み
}
```

**正しい方法**：各タスクに独立したクロージャを作成するか、`ref` を使用する。

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

---

## 第四章：エラー処理

### 4.1 ? 演算子

`?` 演算子は明示的なエラー伝播に使用され、Rust のセマンティクスと一致する。

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // エラーの場合、即座に伝播
    return content.read_all()
}
```

### 4.2 spawn ブロック内のエラー伝播

**ルール**：
1. すべてのタスクの完了を待つ（一部のタスクが既に失敗していても）
2. 最初に出会ったエラーを伝播する
3. `?` を使用してエラー伝播点を明示的にマークする

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗の可能性あり
    fetch("url2")?      // 失敗の可能性あり
}
// いずれかのタスクが失敗した場合、spawn ブロック全体は最初のエラーを伝播する
```

### 4.3 エラー型

**自動生成**：コンパイラは自動的にユニオンエラー型を生成する（TypeScript のユニオン型に類似）。

```yaoxiang
// コンパイラはエラー型を HttpError | IoError と推論する
(a, b) = spawn {
    fetch("url"),           // HttpError をスローする可能性あり
    read_file("data.txt")  // IoError をスローする可能性あり
}
```

**手動オーバーライド**：ユーザーは統一エラー型を手動で定義できる。

```yaoxiang
AppError: Type = {
    Http: (http_error: HttpError) -> AppError,
    Io: (io_error: IoError) -> AppError,
    Parse: (parse_error: ParseError) -> AppError
}

process: (url: String, path: FilePath) -> Result(Data, AppError) = {
    (a, b) = spawn {
        fetch(url).map_err(AppError.Http)?,
        read_file(path).map_err(AppError.Io)?
    }
    return parse(a + b).map_err(AppError.Parse)?
}
```

---

## 第五章：資源型と副作用

### 5.1 組み込み資源型

| 資源型 | 説明 | コンパイラの動作 |
|----------|------|-----------|
| `FilePath` | ファイルシステムパス | 同じパスの操作は自動的に直列化 |
| `HttpUrl` | HTTP エンドポイント | 同じ URL の操作は自動的に直列化 |
| `DBUrl` | データベース接続 | 同じ接続の操作は自動的に直列化 |
| `Console` | 標準出力 | すべての Console 操作は自動的に直列化 |

```yaoxiang
// 同じファイルへの操作は自動的に直列化される
(a, b) = spawn {
    read_file("data.txt"),      // 先に実行
    write_file("data.txt", x)   // 読み取り完了を待機
}
```

### 5.2 ユーザー定義資源型

ユーザー定義の資源型は明示的にマークする必要がある。

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 副作用の追跡

コンパイラは資源型の使用を追跡し、並行性の安全性を保証する。

```yaoxiang
// コンパイラ警告：Console 操作がインターリーブされる可能性あり
spawn {
    print("Hello"),     // 次の行とインターリーブされる可能性あり
    print("World")
}

// 正しい方法：明示的に直列化
spawn {
    print("Hello\nWorld")
}
```

---

## 第六章：コンパイラの動作

### 6.1 DAG 分析

コンパイラはコンパイル時に spawn ブロック内の依存関係（DAG）を分析し、以下を決定する：
1. どの式が並列化可能か
2. どれが直列化必須か
3. タスクをどのように割り当てるか

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // タスク 1
    y = fetch("url2"),      // タスク 2（タスク 1 と並列）
    z = process(x, y)       // タスク 3（x と y に依存、待機必須）
}
```

### 6.2 Rc/Arc の選択（保守的戦略）

コンパイラは**保守的戦略**を採用し、スレッド安全性を保証するためにデフォルトで `Arc` を使用する：

| 条件 | 選択 | 理由 |
|------|------|------|
| デフォルト（安全性を証明できない場合） | `Arc` | 安全性優先、データ競合を回避 |
| コンパイラがデータが単一タスク内でのみ使用されると**証明できる**場合 | `Rc` | アトミック操作のオーバーヘッドなし |

**戦略の説明**：
- **デフォルト `Arc`**：`ref` が単一タスク内でのみ使用されるかコンパイラが判断できない場合、保守的に `Arc` を選択
- **`Rc` への降格**：DAG 分析によりデータがタスクを跨いで共有されないことが**証明**できる場合にのみ `Rc` に降格
- **遅くても間違いは許さない**：`Arc` を選択する追加オーバーヘッドは、データ競合のリスクよりはるかに小さい

```yaoxiang
data = load_data()

// デフォルト：コンパイラは Arc を選択（保守的戦略）
result = spawn {
    shared = ref data
    process(shared)
}

// コンパイラが単一タスク内でのみ使用されると証明できる場合のみ：Rc に降格
// （コンパイラの DAG 分析がタスクを跨ぐ可能性を明確に排除できる必要がある）
```

### 6.3 並列性なし警告

spawn ブロック内のタスクに実際の並列化の機会がない場合、コンパイラは警告を発する。

```yaoxiang
// コンパイラ警告：並列化の機会なし
result = spawn {
    a = fetch("url")    // 唯一のタスク
}
// 提案：通常のコードを直接使用する
result = fetch("url")
```

### 6.4 資源競合の検出

コンパイラは資源型の潜在的な競合を検出する。

```yaoxiang
// コンパイルエラー：同じファイルへの並列書き込み
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // エラー！
}
```

---

## 第七章：古い設計との比較

### 7.1 廃止された機能

| 古い機能 | ステータス | 代替案 |
|--------|------|----------|
| `@block`、`@eager`、`@auto` 注釈 | 廃止 | なし、依存駆動で自動処理 |
| プログラム全体の自動 DAG 分析 | 廃止 | spawn ブロック内のみ分析 |
| `Send`、`Sync` trait | 廃止 | 所有権 + ref で自動処理 |
| future / 非ブロッキングハンドル | 廃止 | spawn ブロックの同期返却 |
| `Mutex[T]`、`Atomic[T]`、`RwLock[T]` | 廃止 | ref が Rc/Arc を自動選択 |

### 7.2 設計理念の転換

**古いモデル**：
- 明示的な注釈による並行性挙動の制御
- 複雑な trait 制約
- 非同期プログラミングモデル

**新しいモデル**：
- 依存駆動、暗黙的な並行性
- 所有権 + ref による共有の簡素化
- 同期プログラミングモデル、spawn ブロックのブロッキング返却

### 7.3 移行ガイド

> **廃止に関する説明**：以下の古いコード例は移行の方向性を示すためのものである。`@block`、`@eager`、`@auto`、`let`、`await`、`Future` は YaoXiang のキーワードではなく、新しい設計から削除されている。

```yaoxiang
// 古いコード（擬似コード、古いモデルのスタイルを示す）
@block async fetch_data(): Future<Data> = {
    let data = await fetch("url")
    return data
}

// 新しいコード
fetch_data: () -> Data = {
    data = fetch("url")     // 同期呼び出し
    return data
}

// 並行バージョン
fetch_multiple: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        result = fetch(url)
    }
    return results
}
```

---

## 付録：構文クイックリファレンス

### A.1 spawn 文

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
SpawnBody   ::= Assignment (',' Assignment)*
```

### A.2 エラー処理

```
Expr '?'              // エラー伝播（Result 型）
```

### A.3 ref 式

```
RefExpr     ::= 'ref' Expr
```

### A.4 資源型のマーク

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```