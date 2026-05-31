# 標準ライブラリ仕様

本書は YaoXiang プログラミング言語の標準ライブラリ仕様を定義ものであり、コアライブラリ、IOライブラリ、数学ライブラリを含む。

---

## 第1章：コアライブラリ

### 1.1 基本型

標準ライブラリは以下の基本型を提供する：

| 型 | モジュール | 説明 |
|------|------|------|
| `Option[T]` | `std.option` | 値変体（オプショナル）型 |
| `Result[T, E]` | `std.result` | エラー処理型 |
| `List[T]` | `std.collection` | 動的配列 |
| `Map[K, V]` | `std.collection` | ハッシュマップ |
| `String` | `std.string` | 文字列型 |
| `Array[T, N]` | `std.array` | 固定長配列 |

### 1.2 Option 型

```
Option: Type[T] = some(T) | none
```

**値変体構築**：

| 値変体 | 構文 | 説明 |
|------|------|------|
| `some(T)` | `some(value)` | 値あり |
| `none` | `none` | 値なし |

**主要メソッド**：

```yaoxiang
// 値の有無を確認
is_some: (self: Option[T]) -> Bool
is_none: (self: Option[T]) -> Bool

// 値を取得（panic の可能性あり）
unwrap: (self: Option[T]) -> T

// 値またはデフォルト値を取得
unwrap_or: (self: Option[T], default: T) -> T

// 値のマップ
map: [R](self: Option[T], f: Fn(T) -> R) -> Option[R]
```

### 1.3 Result 型

```
Result: Type[T, E] = ok(T) | err(E)
```

**値変体構築**：

| 値変体 | 構文 | 説明 |
|------|------|------|
| `ok(T)` | `ok(value)` | 成功値 |
| `err(E)` | `err(error)` | エラー値 |

**主要メソッド**：

```yaoxiang
// 成功かどうかを確認
is_ok: (self: Result[T, E]) -> Bool
is_err: (self: Result[T, E]) -> Bool

// 値を取得（panic の可能性あり）
unwrap: (self: Result[T, E]) -> T

// 値またはデフォルト値を取得
unwrap_or: (self: Result[T, E], default: T) -> T

// 成功値のマップ
map: [R](self: Result[T, E], f: Fn(T) -> R) -> Result[R, E]

// エラー値のマップ
map_err: [F](self: Result[T, E], f: Fn(E) -> F) -> Result[T, F]
```

### 1.4 エラー伝播

```
ErrorPropagate ::= Expr '?'
```

`?` 演算子は Result 型のエラーを自動的に伝播する：

```
// 成功時は値を返し、失敗時は err を上位に返す
data = fetch_data()?

// 以下と同等
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

---

## 第2章：IO ライブラリ

### 2.1 標準入出力

```yaoxiang
// 標準出力
print: (msg: String) -> Void
println: (msg: String) -> Void

// 標準入力
read_line: () -> String
read_char: () -> Char
```

### 2.2 ファイル操作

```yaoxiang
// ファイル型
File: Type = {
    path: String,
    read: (self: File) -> Result[String, Error],
    write: (self: File, content: String) -> Result[Void, Error],
    append: (self: File, content: String) -> Result[Void, Error],
    close: (self: File) -> Void
}

// ファイル操作
open: (path: String) -> Result[File, Error]
create: (path: String) -> Result[File, Error]
delete: (path: String) -> Result[Void, Error]
```

### 2.3 ディレクトリ操作

```yaoxiang
// ディレクトリ型
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result[List[String], Error],
    create: (self: Dir) -> Result[Void, Error],
    delete: (self: Dir) -> Result[Void, Error]
}

// ディレクトリ操作
read_dir: (path: String) -> Result[Dir, Error]
create_dir: (path: String) -> Result[Void, Error]
delete_dir: (path: String) -> Result[Void, Error]
```

---

## 第3章：数学ライブラリ

### 3.1 基礎数学関数

```yaoxiang
// 絶対値
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// 最大最小値
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// 冪乗演算
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// 対数
log: (x: Float) -> Float
log2: (x: Float) -> Float
log10: (x: Float) -> Float
```

### 3.2 三角関数

```yaoxiang
// 三角関数
sin: (x: Float) -> Float
cos: (x: Float) -> Float
tan: (x: Float) -> Float

// 逆三角関数
asin: (x: Float) -> Float
acos: (x: Float) -> Float
atan: (x: Float) -> Float
atan2: (y: Float, x: Float) -> Float
```

### 3.3 定数

```yaoxiang
// 数学定数
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## 第4章：文字列ライブラリ

### 4.1 文字列操作

```yaoxiang
// 文字列長
length: (s: String) -> Int

// 文字列連結
concat: (a: String, b: String) -> String

// 文字列分割
split: (s: String, delimiter: String) -> List[String]

// 文字列検索
find: (s: String, pattern: String) -> Option[Int]
contains: (s: String, pattern: String) -> Bool

// 文字列置換
replace: (s: String, old: String, new: String) -> String

// 文字列トリム
trim: (s: String) -> String
trim_left: (s: String) -> String
trim_right: (s: String) -> String
```

### 4.2 文字列変換

```yaoxiang
// 型変換
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// パース
parse_int: (s: String) -> Result[Int, Error]
parse_float: (s: String) -> Result[Float, Error]
```

---

## 第5章：コレクションライブラリ

### 5.1 List 型

```yaoxiang
// List 型
List: Type[T] = {
    data: Array[T],
    length: Int,
    push: [T](self: List[T], item: T) -> Void,
    pop: [T](self: List[T]) -> Option[T],
    get: [T](self: List[T], index: Int) -> Option[T],
    set: [T](self: List[T], index: Int, value: T) -> Void,
    insert: [T](self: List[T], index: Int, item: T) -> Void,
    remove: [T](self: List[T], index: Int) -> Option[T],
    clear: [T](self: List[T]) -> Void,
    contains: [T](self: List[T], item: T) -> Bool,
    sort: [T](self: List[T]) -> List[T],
    reverse: [T](self: List[T]) -> List[T],
    map: [T, R](self: List[T], f: Fn(T) -> R) -> List[R],
    filter: [T](self: List[T], predicate: Fn(T) -> Bool) -> List[T],
    reduce: [T, R](self: List[T], initial: R, f: Fn(R, T) -> R) -> R
}
```

### 5.2 Map 型

```yaoxiang
// Map 型
Map: Type[K, V] = {
    data: Array[(K, V)],
    length: Int,
    insert: [K, V](self: Map[K, V], key: K, value: V) -> Void,
    get: [K, V](self: Map[K, V], key: K) -> Option[V],
    remove: [K, V](self: Map[K, V], key: K) -> Option[V],
    contains_key: [K, V](self: Map[K, V], key: K) -> Bool,
    keys: [K, V](self: Map[K, V]) -> List[K],
    values: [K, V](self: Map[K, V]) -> List[V],
    clear: [K, V](self: Map[K, V]) -> Void
}
```

---

## 第6章：イテレータライブラリ

### 6.1 Iterator trait

```yaoxiang
// Iterator trait
Iterator: Type[T] = {
    Item: T,
    next: (self: Self) -> Option[T],
    has_next: (self: Self) -> Bool,
    map: [R](self: Self, f: Fn(T) -> R) -> Iterator[R],
    filter: (self: Self, predicate: Fn(T) -> Bool) -> Iterator[T],
    collect: (self: Self) -> List[T],
    reduce: [R](self: Self, initial: R, f: Fn(R, T) -> R) -> R,
    for_each: (self: Self, f: Fn(T) -> Void) -> Void
}
```

### 6.2 イテレータアダプタ

```yaoxiang
// レンジイテレータ
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator[Int]
}

// 使用例
for i in 0..10 {
    print(i)
}

for i in 0..10 step 2 {
    print(i)
}
```

---

## 付録：標準ライブラリモジュール索引

### A.1 コアモジュール

| モジュール | 説明 |
|------|------|
| `std.option` | Option 型 |
| `std.result` | Result 型 |
| `std.collection` | List、Map などのコレクション型 |
| `std.string` | 文字列操作 |
| `std.array` | 配列操作 |
| `std.iterator` | イテレータ |

### A.2 IO モジュール

| モジュール | 説明 |
|------|------|
| `std.io` | 標準入出力 |
| `std.file` | ファイル操作 |
| `std.dir` | ディレクトリ操作 |

### A.3 数学モジュール

| モジュール | 説明 |
|------|------|
| `std.math` | 数学関数 |
| `std.math.trig` | 三角関数 |
| `std.math.log` | 対数関数 |

### A.4 ユーティリティモジュール

| モジュール | 説明 |
|------|------|
| `std.random` | 乱数生成 |
| `std.time` | 日時 |
| `std.regex` | 正規表現 |