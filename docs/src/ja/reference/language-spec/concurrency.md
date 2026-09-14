# 並行モデル仕様

> **ステータス**：正式仕様。RFC-024（並行モデル）、RFC-009（所有権モデル）、RFC-008（ランタイムアーキテクチャ）に基づく。

本文書は YaoXiang プログラミング言語の並行モデル仕様を定義するものであり、`{}`
ブロックのセマンティクス、`spawn`
並行プリミティブ、所有権との相互作用、エラーハンドリング、リソース型を含む。

**コア設計——1つのプリミティブ、1つのルール**：

```
spawn { ... }        ← 唯一の並列プリミティブ
直接子代入がタスクを生成  ← 唯一のルール
結果を同期的にブロック待機 ← 唯一の振る舞い
```

---

## 第一章：概要

### 1.1 {} ブロックの本質

YaoXiang において、`{}` は**依存駆動の計算ユニット**である。

| 属性           | 説明                                                                                                           |
| -------------- | -------------------------------------------------------------------------------------------------------------- |
| 依存駆動       | ブロックは実行時に内部の全変数が準備完了か確認し、揃っていれば即座に実行し、そうでなければブロックして待機する |
| 実行タイミング | 依存関係によって決定され、「即時」か「遅延」かには関係しない                                                   |
| 戻り値         | `return` で明示的に値を返す。`return` がない場合はデフォルトで `Void` を返す                                   |
| 構文の統一性   | 関数本体、変数初期化、`spawn` 後のどこに現れてもセマンティクスは一致する                                       |
| スコープの隔離 | 変数は厳密に `{}` 内部に限定され、外側のスコープに漏れ出さない                                                 |

```yaoxiang
// 依存駆動の例
x = compute_x()        // x は準備完了
y = compute_y()        // y は準備完了
result = {
    // x と y に依存し、両者が準備完了後即座に実行
    return x + y
}
```

### 1.2 返却ルール

| 書き方                    | 戻り値                              | 説明                           |
| ------------------------- | ----------------------------------- | ------------------------------ |
| `= expr`（波括弧なし）    | `expr` を直接返す                   | 式がそのまま値                 |
| `= { ... }`（波括弧あり） | `return` 必須。なければ `Void` 返却 | ブロックは明示的 return が必要 |

```yaoxiang
// 波括弧なし：直接返却
add: (a: Int, b: Int) -> Int = a + b

// 波括弧あり：return 必須
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

`spawn { ... }` は YaoXiang における**唯一の並列プリミティブ**である。

**コアルール**：

- spawn ブロックの**直接子代入**が並列タスクを生成する
- ネストした `{}` 内の代入は独立タスクとしてカウントされない
- spawn ブロック全体は同期的にブロックし、全タスクの完了を待ってから結果を返す
- コールバック、`await`、アノテーションは一切ない

```yaoxiang
// 2つのタスクが並列実行
(a, b) = spawn {
    fetch("url1"),      // タスク 1
    fetch("url2")       // タスク 2
}
// 両方とも完了するのを待って続行
```

### 1.4 ユーザーのメンタルモデル

> 通常のコードは順次実行される。複数の処理を同時に行いたいなら、それらを `spawn { ... }`
> ブロックにまとめる。ブロック内の各直接代入は即座に（並列で）開始され、必要な結果は自動的に待機される。ブロックは全処理の完了を待ってから最終結果を提供する。コールバックも
> `await` も、不思議なアノテーションもない。

---

## 第二章：構文とセマンティクス

### 2.1 通常コード

通常コード（spawn ブロック外）は**順次実行**される。

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

1. spawn ブロック内の直接子代入は独立タスクとして並列実行される
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

### 2.3 関数本体中の spawn

関数本体は `{}` ブロックであり、その中で `spawn` を使用できる。

```yaoxiang
fetch_and_parse: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        parsed = parse(fetch(url))
    }
    return results
}
```

### 2.4 ループ中の spawn

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
```

**セマンティクス**：データ並列ループ。各イテレーションが独立タスクとなる。

```yaoxiang
// リストの各要素を並列処理
results = spawn for item in items {
    result = process(item)
}
```

> **注意**：`spawn for`
> のループ本体は独立タスクであり、イテレーション間で共有される可変状態はサポートされない。結果を集約する必要がある場合は、`spawn for`
> で結果を集めた後、外部で処理すること。

```yaoxiang
// 正しい例：並列処理後に外部で集約
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // 順次集約
```

### 2.5 ネストした spawn

spawn ブロックはネストでき、内側の spawn は新しい並行ドメインを生成する。

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

内側の spawn の直接子代入のみがタスクとなり、外側の spawn はそれを貫通しない。

---

## 第三章：所有権モデルとの相互作用

### 3.1 Move セマンティクス

Move は YaoXiang のデフォルトセマンティクス（ゼロコピー）である。変数が spawn ブロックに入ると、外部では使用できなくなる。

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // data の所有権が spawn ブロックへ move される
}
// data はここでは使用不可（move 済み）
```

### 3.2 借用トークン

`&T` および `&mut T`
はゼロサイズのコンパイル時権限証明であり、**タスク境界を跨いで渡せない**。これは特別なルールではなく、トークンはコンパイル時の権限証明であり、タスク間共有には
`ref` を使用すること。

```yaoxiang
data = load_data()

// コンパイルエラー：借用トークンはタスクを跨げない
result = spawn {
    process(&data)   // エラー！&T はタスク境界を渡れない
}
```

**トークン型の属性**：

| トークン | 第一のセマンティクス                                                                    | 二次的属性                                                                      |
| -------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `&T`     | **ソースデータを凍結**——ReadToken の生存期間中、いかなる WriteToken(T) も取得不可       | ゼロサイズ、コピー可能（Dup）——凍結保証下で複数枚の読み取り専用ビューは本来安全 |
| `&mut T` | **排他的読み書き**——WriteToken の生存期間中、他のトークン（読み・書き問わず）は共存不可 | ゼロサイズ、線形（Dup 不可）——排他アクセス下ではコピーは無意味                  |

> **因果順序**：ReadToken の Dup は凍結保証の帰結であり、その逆ではない。データが凍結される（突然変異の可能性がない）→ 複数の読み取り専用ビューが安全 →
> Dup を実装できる。もし Dup を定義とし、衝突チェックをパッチとして扱うなら、因果が逆転する。

### 3.3 ref 共有

`ref` はスコープを跨いで共有する唯一の方法である。コンパイラは自動的に `Rc`（単一タスク）または
`Arc`（タスク間）を選択し、ユーザーが気にする必要はない。

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが Rc または Arc を自動選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**コンパイラの選択戦略**：

| 条件                                                         | 選択  | 理由                               |
| ------------------------------------------------------------ | ----- | ---------------------------------- |
| デフォルト（安全性を証明できない）                           | `Arc` | 安全性優先、データ競合回避         |
| コンパイラがデータが単一タスク内でのみ使用されると証明できる | `Rc`  | アトミック操作のオーバーヘッドなし |

**ref vs 借用トークン**：

|          | `&T` / `&mut T`            | `ref`                                 |
| -------- | -------------------------- | ------------------------------------- |
| 用途     | ちょっと見る/その場で変更  | 共有保持                              |
| コスト   | ゼロコスト（ゼロサイズ型） | Rc または Arc（コンパイラが選択）     |
| タスク間 | 不可                       | 可（コンパイラが自動的に Arc を選択） |

### 3.4 クロージャとキャプチャ

**クロージャは外側の変数を暗黙的にキャプチャしない**（RFC-009 2026-06-16 決議、SPEC
§12.3）。Lambda は明示的なパラメータと自身のローカル変数のみを使用する。外側のデータが必要な場合は、明示的なパラメータとして渡すか、カリー化によって作成時点で固定する。暗黙的キャプチャのコンパイルエラーコードは E1001。

> 禁止理由：クロージャ定義時点での外側スコープは、クロージャがエスケープした後には死んでいる可能性があり、暗黙的にキャプチャされた参照の生存を保証できない。カリー化で固定された値は作成時点（呼び出し時点のスコープが生存）で取得されるため、安全かつ隠れたコストがない。

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // ❌ コンパイルエラー E1001：data を暗黙的にキャプチャ
```

```yaoxiang
// ✅ 正しい方法その 1：明示的パラメータで渡す
add: (data: Data, x: Int) -> Int = data.value + x

// ✅ 正しい方法その 2：カリー化で固定（コンテキストを作成時点で固定し、クロージャはパラメータのみ受け取る）
mk: (data: Data) -> (x: Int) -> Int = (data) => (x) => data.value + x
```

**spawn 内の共有**：タスク間で共有が必要な場合は、`ref` を使って明示的に共有保持する（RFC-024）。

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

`spawn while` は `&mut` 型の外部変数のキャプチャを禁止する（RFC-024
2026-07-04 決議、コンパイル時エラー、データ競合を回避し Sync を導入しない）。

---

## 第四章：エラーハンドリング

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

1. 全タスクの完了を待つ（一部のタスクが既に失敗していても）
2. 最初に出会ったエラーを伝播する
3. `?` を使ってエラー伝播点を明示的にマークする

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗する可能性あり
    fetch("url2")?      // 失敗する可能性あり
}
// いずれかのタスクが失敗した場合、spawn ブロックは最初のエラーを伝播する
```

### 4.3 エラー型

**自動生成**：コンパイラが自動的にユニオン型を生成する。

```yaoxiang
// コンパイラがエラー型を HttpError | IoError と推論
(a, b) = spawn {
    fetch("url"),           // HttpError を投げる可能性あり
    read_file("data.txt")  // IoError を投げる可能性あり
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

## 第五章：リソース型と副作用

### 5.1 組み込みリソース型

| リソース型 | 説明                   | コンパイラの振る舞い                        |
| ---------- | ---------------------- | ------------------------------------------- |
| `FilePath` | ファイルシステムのパス | 同じパスへの操作は自動的にシリアライズ      |
| `HttpUrl`  | HTTP エンドポイント    | 同じ URL への操作は自動的にシリアライズ     |
| `DBUrl`    | データベース接続       | 同じ接続への操作は自動的にシリアライズ      |
| `Console`  | 標準出力               | すべての Console 操作は自動的にシリアライズ |

```yaoxiang
// 同一ファイルへの操作は自動的にシリアライズされる
(a, b) = spawn {
    read_file("data.txt"),      // 先に実行
    write_file("data.txt", x)   // 読み取り完了を待つ
}
```

### 5.2 ユーザー定義リソース型

ユーザー定義のリソース型は明示的にマークする必要がある。

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 副作用の追跡

コンパイラはリソース型の使用を追跡し、並行安全性を確保する。

```yaoxiang
// コンパイラ警告：Console 操作がインターリーブされる可能性あり
spawn {
    print("Hello"),     // 次の行とインターリーブされる可能性あり
    print("World")
}

// 正しい：明示的にシリアライズ
spawn {
    print("Hello\nWorld")
}
```

---

## 第六章：コンパイラの振る舞い

### 6.1 DAG 分析

コンパイラはコンパイル時に spawn ブロック内の依存関係（DAG）を分析し、以下を決定する：

1. どの式が並列可能か
2. どの式が逐次でなければならないか
3. タスクをどのように割り当てるか

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // タスク 1
    y = fetch("url2"),      // タスク 2（タスク 1 と並列）
    z = process(x, y)       // タスク 3（x と y に依存、待機が必要）
}
```

### 6.2 Rc/Arc 選択（保守的戦略）

コンパイラは**保守的戦略**を採用し、デフォルトで `Arc` を使用してスレッド安全性を確保する：

- **デフォルト `Arc`**：コンパイラが `ref`
  が単一タスク内でのみ使用されるか判断できない場合、保守的に `Arc` を選択
- **`Rc` への降格**：DAG 分析によってデータが絶対にタスク間で共有されないと**証明できる**場合のみ
  `Rc` に降格
- **遅くても、間違わない**：`Arc` 選択の追加オーバーヘッドはデータ競合のリスクよりはるかに小さい

### 6.3 並行性なし警告

spawn ブロック内のタスクに実際の並列化機会がない場合、コンパイラは警告を発する。

```yaoxiang
// コンパイラ警告：並列化機会なし
result = spawn {
    a = fetch("url")    // 唯一のタスク
}
// 推奨：直接通常のコードを使用
result = fetch("url")
```

### 6.4 リソース競合検出

コンパイラはリソース型の潜在的な競合を検出する。

```yaoxiang
// コンパイルエラー：同一ファイルへの同時書き込み
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // エラー！
}
```

---

## 第七章：ランタイム階層

コンパイルフェーズは完全に同じであり、違いはランタイム実行方法のみである（RFC-008）。

| 階層             | spawn サポート | DAG 分析                                | 適用シナリオ                           |
| ---------------- | -------------- | --------------------------------------- | -------------------------------------- |
| Embedded Runtime | ❌             | なし                                    | WASM、ゲームスクリプト、ルールエンジン |
| Standard Runtime | ✅             | spawn ブロック内                        | Web サービス、データパイプライン       |
| Full Runtime     | ✅             | spawn ブロック内 + ワークスティーリング | 科学計算、大規模並列処理               |

**Embedded Runtime**：即時エグゼキュータ、spawn サポートなし、高性能・低フットプリント。

**Standard Runtime**：`spawn {}`
ブロックをサポートし、spawn ブロック内で DAG 分析と自動並列化を行う。`num_workers=1`
でシングルスレッドモード。

**Full Runtime**：Standard + WorkStealer によるロードバランシング。

---

## 付録：構文クイックリファレンス

### A.1 spawn 文

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
SpawnBody   ::= Assignment (',' Assignment)*
```

### A.2 エラーハンドリング

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
