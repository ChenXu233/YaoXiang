# 並行モデル仕様

> **ステータス**: 正式仕様。RFC-024（並行モデル）、RFC-009（所有権モデル）、RFC-008（ランタイムアーキテクチャ）に基づく。

本ドキュメントはYaoXiangプログラミング言語の並行モデル仕様を定義する。`{}` ブロックのセマンティクス、`spawn` 並行プリミティブ、所有権との相互作用、エラー処理、リソース型を含む。

**コア設計——1つのプリミティブ、1つのルール**:

```
spawn { ... }        ← 唯一の並列プリミティブ
直接の子代入がタスクを生成  ← 唯一のルール
同期ブロックして結果を待機  ← 唯一の動作
```

---

## 第1章: 概要

### 1.1 {} ブロックの本質

YaoXiangにおいて、`{}` は**依存駆動の計算ユニット**である。

| 属性 | 説明 |
|------|------|
| 依存駆動 | ブロックは実行時に内部の全変数が準備済みかを確認し、揃っていれば即座に実行、そうでなければブロックして待機する |
| 実行タイミング | 依存関係によって決定され、「即時」か「遅延」かとは無関係 |
| 戻り値 | `return` を使って明示的に値を返す；`return` がない場合はデフォルトで `Void` を返す |
| 構文の統一 | 関数本体、変数初期化、`spawn` の後など、どこに現れても意味は一貫している |
| スコープの隔離 | 変数は `{}` の内部に厳格に限定され、外側のスコープに漏れない |

```yaoxiang
// 依存駆動の例
x = compute_x()        // x 準備完了
y = compute_y()        // y 準備完了
result = {
    // x と y に依存し、両者が準備完了次第即座に実行
    return x + y
}
```

### 1.2 返却ルール

| 書き方 | 戻り値 | 説明 |
|------|--------|------|
| `= expr`（中括弧なし） | `expr` を直接返す | 式がそのまま値 |
| `= { ... }`（中括弧あり） | `return` 必須、なければ `Void` を返す | ブロックは明示的な return が必要 |

```yaoxiang
// 中括弧なし: 直接返す
add: (a: Int, b: Int) -> Int = a + b

// 中括弧あり: return 必須
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 中括弧ありだが return なし: Void を返す
log: (message: String) -> Void = {
    print(message)  // return なし、Void を返す
}
```

### 1.3 spawn ブロックのセマンティクス

`spawn { ... }` はYaoXiangにおける**唯一の並列プリミティブ**である。

**コアルール**:
- spawn ブロックの**直接の子代入**が並列タスクを生成する
- ネストされた `{}` 内の代入は独立タスクとしてカウントされない
- spawn ブロック全体は同期的にブロックし、全タスク完了後に結果を返す
- コールバック、`await`、アノテーションは存在しない

```yaoxiang
// 2つのタスクが並列実行される
(a, b) = spawn {
    fetch("url1"),      // タスク 1
    fetch("url2")       // タスク 2
}
// 両方とも完了するまで待機してから続行
```

### 1.4 ユーザーメンタルモデル

> 通常のコードは逐次実行される。
> 複数の処理を同時に行いたい場合は、それらを `spawn { ... }` ブロックに入れる。
> ブロック内の各直接代入は即座に（並列に）開始され、必要な結果は自動的に待機される。
> ブロック全体は全処理の完了を待ち、最終結果を提供する。
> コールバックも `await` も、奇妙なアノテーションもない。

---

## 第2章: 構文と意味

### 2.1 通常のコード

通常のコード（spawn ブロック外）は**逐次実行**される。

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

**セマンティクス**:
1. spawn ブロック内の直接の子代入は独立したタスクとして並列実行される
2. 各タスクの結果は対応するパターン変数に束縛される
3. ブロック全体は全タスクが完了するまでブロックする
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

関数本体はそれ自体が `{}` ブロックであり、その中で `spawn` を使用できる。

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

**セマンティクス**: データ並列ループ、各イテレーションが独立タスクとして実行される。

```yaoxiang
// リストの各要素を並列処理
results = spawn for item in items {
    result = process(item)
}
```

> **注意**: `spawn for` のループ本体は独立タスクであり、イテレーション間での可変状態の共有はサポートされない。結果を集約する必要がある場合は、`spawn for` で結果を収集した後、外部で処理する必要がある。

```yaoxiang
// 正しい方法: 並列処理後に外部で集約
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // 逐次集約
```

### 2.5 ネストされた spawn

spawn ブロックはネスト可能で、内側の spawn は新しい並行ドメインを作成する。

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

内側の spawn の直接の子代入のみがタスクであり、外側の spawn は透過しない。

---

## 第3章: 所有権モデルとの相互作用

### 3.1 Move セマンティクス

Move はYaoXiangのデフォルトセマンティクスである（ゼロコピー）。変数が spawn ブロックに入った後、外部からは使用できなくなる。

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // data の所有権が spawn ブロックへ move される
}
// data はここでは使用不可（move 済み）
```

### 3.2 借用トークン

`&T` および `&mut T` はゼロサイズのコンパイル時権限証明であり、**タスク境界を跨ぐことはできない**。これは特別なルールではなく——トークンはコンパイル時権限証明であり、タスクを跨ぐ共有には `ref` を使用すること。

```yaoxiang
data = load_data()

// コンパイルエラー: 借用トークンはタスクを跨げない
result = spawn {
    process(&data)   // エラー！&T はタスクを跨いで渡せない
}
```

**トークンタイプの属性**:

| トークン | 主要なセマンティクス | 副次的な属性 |
|------|---------|---------|
| `&T` | **ソースデータを凍結する**——ReadToken の生存期間中、WriteToken(T) は取得不可 | ゼロサイズ、コピー可能（Dup）——凍結保証下では複数の読み取り専用ビューが自然に安全 |
| `&mut T` | **排他的な読み書き**——WriteToken の生存期間中、他のトークン（読み取りまたは書き込み）は共存不可 | ゼロサイズ、線形（Dup 不可）——排他アクセス下ではコピーは無意味 |

> **因果順序**: ReadToken の Dup は凍結保証の帰結であり、その逆ではない。データが凍結される（突然変異が不可能）→ 複数の読み取り専用ビューが安全 → Dup を実装可能。Dup を定義とし、競合チェックをパッチとすると、因果が逆転する。

### 3.3 ref 共有

`ref` はスコープを跨いで共有する唯一の方法である。コンパイラは `Rc`（単一タスク）または `Arc`（タスク跨ぎ）を自動選択し、ユーザーが気にする必要はない。

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが Rc または Arc を自動選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**コンパイラの選択戦略**:

| 条件 | 選択 | 理由 |
|------|------|------|
| デフォルト（安全性を証明できない） | `Arc` | 安全優先、データ競合を回避 |
| コンパイラがデータが単一タスク内でのみ使用されることを証明可能 | `Rc` | 原子操作のオーバーヘッドなし |

**ref vs 借用トークン**:

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 目的 | 一目見る/インプレース変更 | 共有保持 |
| コスト | ゼロオーバーヘッド（ゼロサイズ型） | Rc または Arc（コンパイラが選択） |
| タスク跨ぎ | 不可 | 可（コンパイラが Arc を自動選択） |

### 3.4 クロージャキャプチャ

クロージャキャプチャ = Move、1つのクロージャは1つのタスクにのみ使用可能。

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // クロージャが data を move キャプチャ

// コンパイルエラー: クロージャは1つのタスクにのみ使用可能
result = spawn {
    fn(1),      // クロージャを使用
    fn(2)       // エラー！クロージャは move 済み
}
```

**正しい方法**: 各タスク用に独立したクロージャを作成するか、`ref` を使用する。

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

---

## 第4章: エラー処理

### 4.1 ? 演算子

`?` 演算子は明示的なエラー伝播に使用され、Rust のセマンティクスと一致する。

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // エラーの場合、即座に伝播
    return content.read_all()
}
```

### 4.2 spawn ブロック内のエラー伝播

**ルール**:
1. 全タスクの完了を待つ（一部が失敗していても）
2. 最初に遭遇したエラーを伝播する
3. `?` を使用してエラー伝播点を明示的にマークする

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗の可能性あり
    fetch("url2")?      // 失敗の可能性あり
}
// いずれかのタスクが失敗した場合、spawn ブロック全体が最初のエラーを伝播
```

### 4.3 エラー型

**自動生成**: コンパイラがユニオンエラー型を自動生成する。

```yaoxiang
// コンパイラがエラー型を HttpError | IoError と推論
(a, b) = spawn {
    fetch("url"),           // HttpError を投げる可能性あり
    read_file("data.txt")  // IoError を投げる可能性あり
}
```

**手動オーバーライド**: ユーザーは統一エラー型を手動で定義可能。

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

## 第5章: リソース型と副作用

### 5.1 組み込みリソース型

| リソース型 | 説明 | コンパイラの動作 |
|----------|------|-----------|
| `FilePath` | ファイルシステムパス | 同一パスの操作は自動的にシリアライズ |
| `HttpUrl` | HTTP エンドポイント | 同一 URL の操作は自動的にシリアライズ |
| `DBUrl` | データベース接続 | 同一接続の操作は自動的にシリアライズ |
| `Console` | 標準出力 | すべての Console 操作は自動的にシリアライズ |

```yaoxiang
// 同一ファイルの操作は自動的にシリアライズ
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

コンパイラはリソース型の使用を追跡し、並行安全性を確保する。

```yaoxiang
// コンパイラ警告: Console 操作がインターリーブする可能性あり
spawn {
    print("Hello"),     // 次の行とインターリーブする可能性あり
    print("World")
}

// 正しい方法: 明示的にシリアライズ
spawn {
    print("Hello\nWorld")
}
```

---

## 第6章: コンパイラの動作

### 6.1 DAG 分析

コンパイラはコンパイル時に spawn ブロック内の依存関係（DAG）を分析し、以下を決定する:
1. どの式が並列可能か
2. どれが逐次である必要があるか
3. タスクをどのように割り当てるか

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // タスク 1
    y = fetch("url2"),      // タスク 2（タスク 1 と並列）
    z = process(x, y)       // タスク 3（x と y に依存、待機必須）
}
```

### 6.2 Rc/Arc 選択（保守的戦略）

コンパイラは**保守的戦略**を採用し、スレッド安全性を確保するためにデフォルトで `Arc` を使用する。

- **デフォルト `Arc`**: コンパイラが `ref` が単一タスク内でのみ使用されるか判断できない場合、保守的に `Arc` を選択
- **`Rc` への降格**: コンパイラが DAG 分析を通じてデータがタスクを跨いで共有されないことを**証明**できる場合にのみ `Rc` に降格
- **遅くても間違わない方が良い**: `Arc` 選択の追加オーバーヘッドはデータ競合のリスクよりはるかに小さい

### 6.3 並列なし警告

spawn ブロック内のタスクに実際の並列機会がない場合、コンパイラは警告を発行する。

```yaoxiang
// コンパイラ警告: 並列機会なし
result = spawn {
    a = fetch("url")    // 唯一のタスク
}
// 推奨: 直接通常のコードを使用
result = fetch("url")
```

### 6.4 リソース競合検出

コンパイラはリソース型の潜在的な競合を検出する。

```yaoxiang
// コンパイルエラー: 同一ファイルへの並列書き込み
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // エラー！
}
```

---

## 第7章: ランタイム階層

コンパイル段階は完全に同じであり、違いがあるのはランタイム実行方法のみである（RFC-008）。

| 階層 | spawn サポート | DAG 分析 | 適用シナリオ |
|------|-----------|----------|----------|
| Embedded Runtime | ❌ | なし | WASM、ゲームスクリプト、ルールエンジン |
| Standard Runtime | ✅ | spawn ブロック内 | Web サービス、データパイプライン |
| Full Runtime | ✅ | spawn ブロック内 + ワークスティーリング | 科学計算、大規模並列 |

**Embedded Runtime**: 即時エグゼキュータ、spawn サポートなし、高性能・低フットプリント。

**Standard Runtime**: `spawn {}` ブロックをサポート、spawn ブロック内で DAG 分析と自動並列化を実行。`num_workers=1` はシングルスレッドモード。

**Full Runtime**: Standard + WorkStealer による負荷分散。

---

## 付録: 構文早見表

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