# 並行モデル仕様

> **ステータス**：正式仕様。RFC-024（並行モデル）、RFC-009（所有権モデル）、RFC-008（ランタイムアーキテクチャ）に基づく。

本ドキュメントは YaoXiang プログラミング言語の並行モデル仕様を定義する。`{}` ブロックのセマンティクス、`spawn` 並行プリミティブ、所有権との相互作用、エラーハンドリング、リソース型を含む。

**コア設計——1つのプリミティブ、1つのルール**：

```
spawn { ... }        ← 唯一のパラレルプリミティブ
直接子代入がタスクを生成  ← 唯一のルール
同期ブロックして結果を待機  ← 唯一の挙動
```

---

## 第一章：概要

### 1.1 {} ブロックの本質

YaoXiang において、`{}` は**依存駆動の計算ユニット**である。

| 属性 | 説明 |
|------|------|
| 依存駆動 | ブロックは実行時に内部の全変数の準備状況を確認し、揃っていれば即座に実行し、そうでなければブロックして待機する |
| 実行タイミング | 依存関係によって決定され、「即時」「遅延」とは無関係 |
| 戻り値 | `return` で明示的に値を返す；`return` がない場合はデフォルトで `Void` を返す |
| 構文の統一 | 関数本体・変数初期化・`spawn` 後のいずれに現れてもセマンティクスは同一 |
| スコープの隔離 | 変数は `{}` の内部に厳密に限定され、外側のスコープには漏れない |

```yaoxiang
// 依存駆動の例
x = compute_x()        // x 準備完了
y = compute_y()        // y 準備完了
result = {
    // x と y に依存し、両方が準備完了次第即座に実行される
    return x + y
}
```

### 1.2 返却ルール

| 書き方 | 戻り値 | 説明 |
|------|--------|------|
| `= expr`（波括弧なし） | `expr` を直接返す | 式がそのまま値となる |
| `= { ... }`（波括弧あり） | `return` 必須、なければ `Void` を返す | ブロックは明示的な return が必要 |

```yaoxiang
// 波括弧なし：直接返す
add: (a: Int, b: Int) -> Int = a + b

// 波括弧あり：return 必須
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 波括弧あり、return なし：Void を返す
log: (message: String) -> Void = {
    print(message)  // return なし、Void を返す
}
```

### 1.3 spawn ブロックのセマンティクス

`spawn { ... }` は YaoXiang における**唯一のパラレルプリミティブ**である。

**コアルール**：
- spawn ブロックの**直接子代入**がパラレルタスクを生成する
- ネストされた `{}` 内の代入は独立タスクとしてカウントされない
- spawn ブロック全体は同期的にブロックし、全タスクの完了を待ってから結果を返す
- コールバック・`await`・注釈は存在しない

```yaoxiang
// 2つのタスクがパラレル実行される
(a, b) = spawn {
    fetch("url1"),      // タスク 1
    fetch("url2")       // タスク 2
}
// 両方の完了を待って続行
```

### 1.4 ユーザのメンタルモデル

> 通常のコードは逐次実行される。
> 複数のことを同時に行いたいなら、それらを `spawn { ... }` ブロックに入れる。
> ブロック内の各直接代入は即座に開始され（パラレル）、必要な結果は自動的に待機される。
> ブロック全体は全処理の完了を待ち、最終結果を提供する。
> コールバックも `await` も奇妙な注釈も存在しない。

---

## 第二章：構文とセマンティクス

### 2.1 通常コード

通常コード（spawn ブロック外）は**逐次実行**される。

```yaoxiang
a = compute_a()     // 最初に実行
b = compute_b(a)    // a に依存、a 完了後に実行
c = compute_c(b)    // b に依存、b 完了後に実行
```

### 2.2 spawn ブロック

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**セマンティクス**：
1. spawn ブロック内の直接子代入が独立タスクとしてパラレル実行される
2. 各タスクの結果が対応するパターンバインドにバインドされる
3. ブロック全体は全タスクの完了までブロックする
4. 全結果のタプルを返す

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

関数本体それ自体が `{}` ブロックであり、その内部で `spawn` を使用できる。

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

**セマンティクス**：データパラレルループ、各イテレーションが独立タスクとなる。

```yaoxiang
// リストの各要素をパラレル処理
results = spawn for item in items {
    result = process(item)
}
```

> **注意**：`spawn for` のループ本体は独立タスクであり、イテレーション間での共有可変状態はサポートされない。結果を集約する必要がある場合は、`spawn for` で結果を集めた後、外部で処理すること。

```yaoxiang
// 正しい方法：パラレル処理後に外部で集約
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // 逐次集約
```

### 2.5 ネストされた spawn

spawn ブロックはネスト可能で、内側の spawn は新たな並行ドメインを生成する。

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

内側 spawn の直接子代入のみがタスクであり、外側 spawn は貫通しない。

---

## 第三章：所有権モデルとの相互作用

### 3.1 Move セマンティクス

Move は YaoXiang のデフォルトセマンティクス（ゼロコピー）である。変数が spawn ブロックに入った後、外側では使用できなくなる。

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // data の所有権が spawn ブロックへ move される
}
// ここでは data は使用不可（move 済み）
```

### 3.2 借用トークン

`&T` と `&mut T` はゼロサイズのコンパイル時権限証明であり、**タスク境界を跨いではならない**。これは特別なルールではなく、トークンはコンパイル時の権限証明であり、タスクを跨いだ共有には `ref` を使用すること。

```yaoxiang
data = load_data()

// コンパイルエラー：借用トークンはタスクを跨げない
result = spawn {
    process(&data)   // エラー！&T はタスクを跨いで渡せない
}
```

**トークン型のプロパティ**：
- `&T`：ゼロサイズ、コピー可能（Dup）、読み取り専用権限を付与
- `&mut T`：ゼロサイズ、線形（non Dup）、排他的読み書き権限を付与

### 3.3 ref 共有

`ref` はスコープを跨いだ共有の唯一の方法である。コンパイラが自動的に `Rc`（単一タスク）または `Arc`（タスク間）を選択するため、ユーザが意識する必要はない。

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが Rc または Arc を自動選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**コンパイラの選択戦略**：

| 条件 | 選択 | 理由 |
|------|------|------|
| デフォルト（安全性を証明できない） | `Arc` | 安全性優先、データ競合を回避 |
| コンパイラがデータが単一タスク内でのみ使用されることを証明できる | `Rc` | アトミック操作のオーバーヘッドなし |

**ref vs 借用トークン**：

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 用途 | 覗き見／その場での変更 | 共有保持 |
| コスト | ゼロオーバーヘッド（ゼロサイズ型） | Rc または Arc（コンパイラが選択） |
| タスク境界 | 不可 | 可（コンパイラが Arc を自動選択） |

### 3.4 クロージャのキャプチャ

クロージャのキャプチャ = Move、1つのクロージャは1つのタスクにしか使えない。

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // クロージャが data を move キャプチャ

// コンパイルエラー：クロージャは1つのタスクにしか使えない
result = spawn {
    fn(1),      // クロージャを使用
    fn(2)       // エラー！クロージャは move 済み
}
```

**正しい方法**：各タスク用に独立したクロージャを作成するか、`ref` を使用する。

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

---

## 第四章：エラーハンドリング

### 4.1 ? 演算子

`?` 演算子は明示的なエラー伝搬に使用され、Rust のセマンティクスと一致する。

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // エラーの場合、即座に伝搬
    return content.read_all()
}
```

### 4.2 spawn ブロック内のエラー伝搬

**ルール**：
1. 全タスクの完了を待つ（一部のタスクが失敗していても）
2. 最初に出会ったエラーを伝搬する
3. `?` を使用してエラー伝搬点を明示的にマークする

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗する可能性がある
    fetch("url2")?      // 失敗する可能性がある
}
// いずれかのタスクが失敗した場合、spawn ブロックは最初のエラーを伝搬する
```

### 4.3 エラー型

**自動生成**：コンパイラが自動的にユニオンエラー型を生成する。

```yaoxiang
// コンパイラがエラー型を HttpError | IoError と推論
(a, b) = spawn {
    fetch("url"),           // HttpError を投げる可能性がある
    read_file("data.txt")  // IoError を投げる可能性がある
}
```

**手動オーバーライド**：ユーザは統一エラー型を手動で定義できる。

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

## 第五章：リソース型と副作用

### 5.1 組み込みリソース型

| リソース型 | 説明 | コンパイラの挙動 |
|----------|------|-----------|
| `FilePath` | ファイルシステムパス | 同一パスへの操作は自動的にシリアライズ |
| `HttpUrl` | HTTP エンドポイント | 同一 URL への操作は自動的にシリアライズ |
| `DBUrl` | データベース接続 | 同一接続への操作は自動的にシリアライズ |
| `Console` | 標準出力 | すべての Console 操作は自動的にシリアライズ |

```yaoxiang
// 同一ファイルへの操作は自動的にシリアライズされる
(a, b) = spawn {
    read_file("data.txt"),      // 最初に実行
    write_file("data.txt", x)   // 読み取り完了を待つ
}
```

### 5.2 ユーザ定義リソース型

ユーザ定義のリソース型は明示的にマークする必要がある。

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 副作用の追跡

コンパイラはリソース型の使用を追跡し、並行安全性を保証する。

```yaoxiang
// コンパイラ警告：Console 操作が交互に発生する可能性
spawn {
    print("Hello"),     // 次行と交互になる可能性あり
    print("World")
}

// 正しい方法：明示的にシリアライズ
spawn {
    print("Hello\nWorld")
}
```

---

## 第六章：コンパイラの挙動

### 6.1 DAG 分析

コンパイラはコンパイル時に spawn ブロック内の依存関係（DAG）を分析し、以下を決定する：
1. どの式がパラレル実行可能か
2. どれが逐次でなければならないか
3. タスクの割り当て方法

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // タスク 1
    y = fetch("url2"),      // タスク 2（タスク 1 とパラレル）
    z = process(x, y)       // タスク 3（x と y に依存、待機必須）
}
```

### 6.2 Rc/Arc の選択（保守的戦略）

コンパイラは**保守的戦略**を採用し、デフォルトでスレッド安全性を保証するために `Arc` を使用する：

- **デフォルト `Arc`**：コンパイラが `ref` が単一タスク内のみで使用されることを判定できない場合、保守的に `Arc` を選択する
- **`Rc` への降格**：コンパイラが DAG 分析によってデータがタスクを跨いで共有されないことを**証明できる**場合にのみ `Rc` へ降格する
- **遅くても間違えない**：`Arc` 選択の追加オーバーヘッドはデータ競合のリスクよりはるかに小さい

### 6.3 並行性なし警告

spawn ブロック内のタスクに実際のパラレル機会がない場合、コンパイラは警告を発する。

```yaoxiang
// コンパイラ警告：パラレル機会なし
result = spawn {
    a = fetch("url")    // 唯一のタスク
}
// 推奨：直接通常コードを使用
result = fetch("url")
```

### 6.4 リソース競合検出

コンパイラはリソース型の潜在的な競合を検出する。

```yaoxiang
// コンパイルエラー：同一ファイルへの並行書き込み
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // エラー！
}
```

---

## 第七章：ランタイム階層

コンパイル段階は完全に同一であり、違いはランタイム実行方法のみである（RFC-008）。

| 階層 | spawn サポート | DAG 分析 | 適用シナリオ |
|------|-----------|----------|----------|
| Embedded Runtime | ❌ | なし | WASM、ゲームスクリプト、ルールエンジン |
| Standard Runtime | ✅ | spawn ブロック内 | Web サービス、データパイプライン |
| Full Runtime | ✅ | spawn ブロック内 + ワークスティーリング | 科学計算、大規模並列 |

**Embedded Runtime**：即時実行器、spawn サポートなし、高性能・低フットプリント。

**Standard Runtime**：`spawn {}` ブロックをサポート、spawn ブロック内で DAG 分析と自動並列化を実行。`num_workers=1` でシングルスレッドモード。

**Full Runtime**：Standard + WorkStealer ロードバランシング。

---

## 付録：構文早見表

### A.1 spawn 文

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
SpawnBody   ::= Assignment (',' Assignment)*
```

### A.2 エラーハンドリング

```
Expr '?'              // エラー伝搬（Result 型）
```

### A.3 ref 式

```
RefExpr     ::= 'ref' Expr
```

### A.4 リソース型のマーク

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```