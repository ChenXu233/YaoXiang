# 標準ライブラリ仕様

本書は YaoXiang プログラミング言語の標準ライブラリ仕様を定義ものであり、コアライブラリ、IOライブラリ、数学ライブラリを含む。

---

## 第1章：コアライブラリ

### 1.1 基本型

標準ライブラリは以下の基本型を提供する：

| 型 | モジュール | 説明 |
|------|------|------|
| `Option(T)` | `std.option` | オプション値型 |
| `Result(T, E)` | `std.result` | エラー処理型 |
| `List(T)` | `std.collection` | 動的配列 |
| `Map(K, V)` | `std.collection` | ハッシュマップ |
| `String` | `std.string` | 文字列型 |
| `Array(T, N)` | `std.array` | 固定長配列 |

### 1.2 Option 型

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**値変体構築**：

| 値変体 | 構文 | 説明 |
|------|------|------|
| `Option.some` | `Option.some(value)` | 値あり |
| `Option.none` | `Option.none()` | 値なし |

**常用メソッド**：

```yaoxiang
// 値の有無を確認
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// 値を取得（panic の可能性あり）
unwrap: (self: Option(T)) -> T

// 値またはデフォルト値を取得
unwrap_or: (self: Option(T), default: T) -> T

// 値をマップ
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Result 型

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**値変体構築**：

| 値変体 | 構文 | 説明 |
|------|------|------|
| `Result.ok` | `Result.ok(value)` | 成功値 |
| `Result.err` | `Result.err(error)` | エラー値 |

**常用メソッド**：

```yaoxiang
// 成功かどうかを確認
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// 値を取得（panic の可能性あり）
unwrap: (self: Result(T, E)) -> T

// 値またはデフォルト値を取得
unwrap_or: (self: Result(T, E), default: T) -> T

// 成功値をマップ
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// エラー値をマップ
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
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
    read: (self: File) -> Result(String, Error),
    write: (self: File, content: String) -> Result(Void, Error),
    append: (self: File, content: String) -> Result(Void, Error),
    close: (self: File) -> Void
}

// ファイル操作
open: (path: String) -> Result(File, Error)
create: (path: String) -> Result(File, Error)
delete: (path: String) -> Result(Void, Error)
```

### 2.3 ディレクトリ操作

```yaoxiang
// ディレクトリ型
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result(List(String), Error),
    create: (self: Dir) -> Result(Void, Error),
    delete: (self: Dir) -> Result(Void, Error)
}

// ディレクトリ操作
read_dir: (path: String) -> Result(Dir, Error)
create_dir: (path: String) -> Result(Void, Error)
delete_dir: (path: String) -> Result(Void, Error)
```

---

## 第3章：数学ライブラリ

### 3.1 基本数学関数

```yaoxiang
// 絶対値
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// 最大値・最小値
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// べき乗演算
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
split: (s: String, delimiter: String) -> List(String)

// 文字列検索
find: (s: String, pattern: String) -> Option(Int)
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
parse_int: (s: String) -> Result(Int, Error)
parse_float: (s: String) -> Result(Float, Error)
```

---

## 第5章：コレクションライブラリ

### 5.1 List 型

```yaoxiang
// List 型
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T: Type) -> ((self: List(T), item: T) -> Void),
    pop: (T: Type) -> ((self: List(T)) -> Option(T)),
    get: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    set: (T: Type) -> ((self: List(T), index: Int, value: T) -> Void),
    insert: (T: Type) -> ((self: List(T), index: Int, item: T) -> Void),
    remove: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    clear: (T: Type) -> ((self: List(T)) -> Void),
    contains: (T: Type) -> ((self: List(T), item: T) -> Bool),
    sort: (T: Type) -> ((self: List(T)) -> List(T)),
    reverse: (T: Type) -> ((self: List(T)) -> List(T)),
    map: (T: Type, R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (T: Type) -> ((self: List(T), predicate: (T) -> Bool) -> List(T)),
    reduce: (T: Type, R: Type) -> ((self: List(T), initial: R, f: (R, T) -> R) -> R)
}
```

### 5.2 Map 型

```yaoxiang
// Map 型
Map: (K: Type, V: Type) -> Type = {
    data: Array((K, V)),
    length: Int,
    insert: (K: Type, V: Type) -> ((self: Map(K, V), key: K, value: V) -> Void),
    get: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    remove: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    contains_key: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Bool),
    keys: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(K)),
    values: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(V)),
    clear: (K: Type, V: Type) -> ((self: Map(K, V)) -> Void)
}
```

---

## 第6章：イテレータライブラリ

### 6.1 Iterator trait

```yaoxiang
// Iterator trait
Iterator: (T: Type) -> Type = {
    Item: T,
    next: () -> Option(T),
    has_next: () -> Bool,
    map: (R: Type) -> ((f: (T) -> R) -> Iterator(R)),
    filter: (predicate: (T) -> Bool) -> Iterator(T),
    collect: () -> List(T),
    reduce: (R: Type) -> ((initial: R, f: (R, T) -> R) -> R),
    for_each: (f: (T) -> Void) -> Void
}
```

### 6.2 イテレータアダプタ

```yaoxiang
// 範囲イテレータ
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
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

## 付録：標準ライブラリモジュールインデックス

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