# 並行モデル仕様

> **状態**：正式仕様。RFC-024（並行モデル）、RFC-009（所有権モデル）、RFC-008（ランタイムアーキテクチャ）に基づく。

本ドキュメントは YaoXiang プログラミング言語の並行モデル仕様を定義する。`{}` ブロックのセマンティクス、`spawn` 並行プリミティブ、所有権との相互作用、エラー処理、リソース型を含む。

**コア設計——1つのプリミティブ、1つのルール**：

```
spawn { ... }        ← 唯一の並行プリミティブ
直接子の代入でタスク作成    ← 唯一のルール
同期ブロッキングで結果待機  ← 唯一の動作
```

---

## 第1章：概要

### 1.1 {} ブロックの本質

YaoXiang では、`{}` は**依存駆動の計算ユニット**である。

| 属性         | 説明                                                                             |
| ------------ | -------------------------------------------------------------------------------- |
| 依存駆動     | ブロック実行時に内部のすべての変数が準備完了かを検査し、準備完了なら即時実行、でなければブロック待機 |
| 実行タイ밍   | 「即時」か「遅延」に関係なく、依存関係によって決定される                         |
| 戻り値       | `return` で明示的に戻り値を返す；`return` がない場合はデフォルトで `Void` を返す |
| 構文の統一   | 関数本体、変数初期化、`spawn` の後、どこに現れてもセマンティクスは一貫している   |
| スコープ隔離 | 変数は厳密に `{}` 内部に限定され、外側のスコープには波及しない                   |

```yaoxiang
// 依存駆動の例
x = compute_x()        // x が準備完了
y = compute_y()        // y が準備完了
result = {
    // x と y に依存、双方の準備完了後即時実行
    return x + y
}
```

### 1.2 戻り値のルール

| 写法                       | 戻り値                             | 説明             |
| -------------------------- | ---------------------------------- | ---------------- |
| `= expr`（波括弧なし）      | `expr` を直接返す                  | 式は値である     |
| `= { ... }`（波括弧あり）   | `return` が必要，否则返回 `Void`   | ブロックは明示的戻り値が必要 |

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

### 1.3 spawn ブロックのセマンティクス

`spawn { ... }` は YaoXiang における**唯一の並行プリミティブ**である。

**コアルール**：

- spawn ブロックの**直接子の代入**が並行タスクを生成する
- ネストされた `{}` 内の代入は独立タスクとしてカウントされない
- 整个 spawn ブロックは同期ブロッキングで、すべてのタスク完了を待機してから結果を返す
- コールバック、`await`、アノテーションはない

```yaoxiang
// 2つのタスクが並行実行
(a, b) = spawn {
    fetch("url1"),      // タスク 1
    fetch("url2")       // タスク 2
}
// 双方の完了を待機してから継続
```

### 1.4 ユーザーメンタルモデル

> 記述した通常のコードは順序実行される。複数のことを同時に行いたい場合、`spawn { ... }` ブロックに入れる。ブロック内の各直接代入は即時開始（並行）し、必要な結果は自動的に待機する。ブロック全体がすべての処理完了を待機し、最終結果を返す。コールバックはなく、`await` も変なアノテーションもない。

---

## 第2章：構文とセマンティクス

### 2.1 通常のコード

通常のコード（spawn ブロック外）は**順序実行**される。

```yaoxiang
a = compute_a()     // 先に実行
b = compute_b(a)    // a に依存、a 完了後実行
c = compute_c(b)    // b に依存、b 完了後実行
```

### 2.2 spawn ブロック

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**セマンティクス**：

1. spawn ブロック内の直接子の代入が独立タスクとして並行実行される
2. 各タスクの結果が対応するパターン変数にバインドされる
3. ブロック全体がすべてのタスク完了までブロッキングする
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

関数本体自身が `{}` ブロックであり、その中で `spawn` を使用できる。

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

**セマンティクス**：データ並列ループ。各反復が独立タスクとなる。

```yaoxiang
// リスト内の各要素を並行処理
results = spawn for item in items {
    result = process(item)
}
```

> **注意**：`spawn for` のループ本体は独立タスクであり、跨反復共有可変状態をサポートしない。結果を聚合する必要がある場合は、`spawn for` で結果を収集してから外部で処理すること。

```yaoxiang
// 正しい：並行処理後に外部で聚合
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // 順序聚合
```

### 2.5 ネスト spawn

spawn ブロックはネスト可能で、内側 spawn が新しい並行ドメインを生成する。

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

内側 spawn の直接子の代入のみがタスクであり、外側 spawn は浸透しない。

---

## 第3章：所有権モデルとの相互作用

### 3.1 Move セマンティクス

Move は YaoXiang のデフォルトセマンティクス（ゼロコピー）である。変数が spawn ブロックに入ると、外部からは使用できなくなる。

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // data の所有権が spawn ブロックに移動
}
// data は此处不可用（移動済み）
```

### 3.2 借用トークン

`&T` と `&mut T` はゼロサイズのコンパイル時権限証明であり、**タスク境界を跨げない**。これは特殊ルールではない——トークンはコンパイル時権限証明であり、跨タスク共有には `ref` を使用すること。

```yaoxiang
data = load_data()

// コンパイルエラー：借用トークンはタスクを跨げない
result = spawn {
    process(&data)   // 錯誤！&T はタスクを跨げない
}
```

**トークン型の属性**：

| トークン    | 主なセマンティクス                                                              | 副次的属性                                                       |
| ----------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| `&T`        | **ソースデータを凍結**——ReadToken の生存期間中は，任何 WriteToken(T) は取得不可 | ゼロサイズ、コピー可能（Dup）——冻结保証下で複数読み取りビューは本質的に安全 |
| `&mut T`    | **排他的読み書き**——WriteToken の生存期間中は、他のトークン（読み取りも書き込みも）は共存不可 | ゼロサイズ、線形（非Dup）——排他アクセス下ではコピーに意味がない |

> **因果順序**：ReadToken の Dup は冻结保证の帰結であり、その逆ではない。データが冻结される（変异的余地がない）→ 複数読み取りビューが安全 → Dup を実装可能。Dup を定義として冲突検査をパッチと見なすと、因果関係が逆転する。

### 3.3 ref 共有

`ref` は跨スコープ共有の唯一の方法である。コンパイラが自動的に `Rc`（単一タスク内）または `Arc`（跨タスク）を選択하며、ユーザーは気にする必要はない。

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが自動的に Rc または Arc を選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**コンパイラの選択戦略**：

| 条件                                   | 選択      | 理由                                 |
| -------------------------------------- | --------- | ------------------------------------ |
| デフォルト（安全性を証明できない）       | `Arc`     | 安全第一、データ競合を回避           |
| コンパイラがデータが単一タスク内のみ使用と証明可能 | `Rc`      | アトミック操作のオーバーヘッドなし   |

**ref と借用トークンの比較**：

|        | `&T` / `&mut T`         | `ref`                     |
| ------ | ----------------------- | ------------------------- |
| 做什么 | 覗き見る/その場で変更   | 共有保持                   |
| コスト | ゼロコスト（ゼロサイズ） | Rc または Arc（コンパイラ選択） |
| 跨タスク | 不可                    | 可（コンパイラが自動的に Arc を選択） |

### 3.4 クロージャキャプチャ

クロージャキャプチャ = Move。クロージャは1つのタスクにしか使用できない。

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // クロージャが data を move でキャプチャ

// コンパイルエラー：クロージャは1つのタスクにしか使用できない
result = spawn {
    fn(1),      // クロージャを使用
    fn(2)       // 錯誤！クロージャは移動済み
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

## 第4章：エラー処理

### 4.1 ? 演算子

`?` 演算子は明示的なエラー伝播に使用され、Ruby のセマンティクスと一致する。

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // エラーなら即時伝播
    return content.read_all()
}
```

### 4.2 spawn ブロック内のエラー伝播

**ルール**：

1. すべてのタスク完了を待機（一部のタスクが失敗しても）
2. 最初に遭遇したエラーを伝播
3. `?` でエラー伝播ポイントを明示的にマーク

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗する可能性あり
    fetch("url2")?      // 失敗する可能性あり
}
// いずれかのタスクが失敗すると、spawn ブロック全体が最初のエラーを伝播
```

### 4.3 エラー型

**自動生成**：コンパイラが自動的にユニオンエラー型を生成する。

```yaoxiang
// コンパイラがエラー型を HttpError | IoError と推論
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

## 第5章：リソース型と副作用

### 5.1 組み込みリソース型

| リソース型    | 説明          | コンパイラ動作                  |
| ------------- | ------------- | ------------------------------- |
| `FilePath`    | ファイルパス  | 同一パス操作を自動的にシリアル化 |
| `HttpUrl`     | HTTP エンドポイント | 同一 URL 操作を自動的にシリアル化 |
| `DBUrl`       | データベース接続 | 同一接続操作を自動的にシリアル化 |
| `Console`     | 標準出力      | すべての Console 操作を自動的にシリアル化 |

```yaoxiang
// 同一ファイルの操作が自動的にシリアル化
(a, b) = spawn {
    read_file("data.txt"),      // 先に実行
    write_file("data.txt", x)   // 読み取り完了を待機
}
```

### 5.2 ユーザー定義リソース型

ユーザー定義リソース型は明示的にマークする必要がある。

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 副作用追跡

コンパイラがリソース型の使用を追跡し、並行安全性を確保する。

```yaoxiang
// コンパイラ警告：Console 操作が交错する可能性がある
spawn {
    print("Hello"),     // 下一行と交错する可能性がある
    print("World")
}

// 正しい：明示的にシリアル化
spawn {
    print("Hello\nWorld")
}
```

---

## 第6章：コンパイラ動作

### 6.1 DAG 分析

コンパイラはコンパイル時に spawn ブロック内の依存関係を分析（DAG）し、以下を確定する：

1. どの式が並行可能か
2. 哪个必须串行
3. 如何分配任务

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // タスク 1
    y = fetch("url2"),      // タスク 2（タスク 1 と並行）
    z = process(x, y)       // タスク 3（x と y に依存、待機が必要）
}
```

### 6.2 Rc/Arc 選択（コンサーバティブ戦略）

コンパイラは**コンサーバティブ戦略**を採用し、スレッド安全性を確保するためにデフォルトで `Arc` を使用する：

- **デフォルト `Arc`**：`ref` が単一タスク内でのみ使用されることをコンパイラが確定できない場合、コンサーバティブに `Arc` を選択
- **`Rc` への降格**：コンパイラが DAG 分析を 통해データ绝对不会跨タスク共有されると**証明**できた場合にのみ `Rc` に降格
- **宁可慢，不可错**：`Arc` の追加オーバーヘッドはデータ競合のリスクよりはるかに小さい

### 6.3 並行なし警告

spawn ブロック内のタスクに実際の並行機会がない場合、コンパイラが警告を発する。

```yaoxiang
// コンパイラ警告：並行機会なし
result = spawn {
    a = fetch("url")    // 唯一のタスク
}
// 提案：通常のコードを使用すれば十分
result = fetch("url")
```

### 6.4 リソース競合検出

コンパイラがリソース型の潜在的な競合を検出する。

```yaoxiang
// コンパイルエラー：同一ファイルへの並行書き込み
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // 錯誤！
}
```

---

## 第7章：ランタイムレイヤー

コンパイル段階は完全に同一で、違いになるのはランタイム実行方式のみ（RFC-008）。

| レイヤー              | spawn サポート | DAG 分析           | 適用シナリオ               |
| -------------------- | -------------- | ------------------ | -------------------------- |
| Embedded Runtime     | ❌             | なし               | WASM、ゲームスクリプト、ルールエンジン |
| Standard Runtime     | ✅             | spawn ブロック内   | Web サービス、データパイプライン       |
| Full Runtime         | ✅             | spawn ブロック内 + ワークスチール | 科学計算、大規模並行       |

**Embedded Runtime**：即时実行器、spawn サポートなし、高性能低オーバーヘッド。

**Standard Runtime**：`spawn {}` ブロックをサポート、spawn ブロック内で DAG 分析と自動並行化を行う。`num_workers=1` でシングルスレッドモード。

**Full Runtime**：Standard + ワークスチールによるロードバランシング。

---

## 付録：構文早見表

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

### A.4 リソース型マーク

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```
