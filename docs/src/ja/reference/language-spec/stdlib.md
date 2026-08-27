# 標準ライブラリ仕様

本ドキュメントはYaoXiangプログラミング言語の標準ライブラリ仕様を定義する。コアライブラリ、IOライブラリ、数学ライブラリを含む。

---

## 第1章：コアライブラリ

### 1.1 基本型

標準ライブラリは以下の基本型の実装を提供する：

| 型             | モジュール       | 説明           |
| -------------- | ---------------- | -------------- |
| `Option(T)`    | `std.option`     | オプション値型 |
| `Result(T, E)` | `std.result`     | エラー処理型   |
| `List(T)`      | `std.collection` | 動的配列       |
| `Map(K, V)`    | `std.collection` | ハッシュマップ |
| `String`       | `std.string`     | 文字列型       |
| `Array(T, N)`  | `std.array`      | 固定サイズ配列 |

### 1.2 Option 型

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**バリアント構築**：

| バリアント    | 構文                 | 説明   |
| ------------- | -------------------- | ------ |
| `Option.some` | `Option.some(value)` | 値あり |
| `Option.none` | `Option.none()`      | 値なし |

**よく使われるメソッド**：

```yaoxiang
// 检查是否有值
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// 获取值（可能 panic）
unwrap: (self: Option(T)) -> T

// 获取值或默认值
unwrap_or: (self: Option(T), default: T) -> T

// 映射值
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Result 型

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**バリアント構築**：

| バリアント   | 構文                | 説明     |
| ------------ | ------------------- | -------- |
| `Result.ok`  | `Result.ok(value)`  | 成功値   |
| `Result.err` | `Result.err(error)` | エラー値 |

**よく使われるメソッド**：

```yaoxiang
// 检查是否成功
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// 获取值（可能 panic）
unwrap: (self: Result(T, E)) -> T

// 获取值或默认值
unwrap_or: (self: Result(T, E), default: T) -> T

// 映射成功值
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// 映射错误值
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

### 1.4 エラー伝播

```
ErrorPropagate ::= Expr '?'
```

`?`演算子はResult型のエラーを自動的に伝播する：

```
// 成功时返回值，失败时向上返回 err
data = fetch_data()?

// 等价于
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 アサーション（std.assert）

`std.assert`モジュールは統一されたアサーションメカニズムを提供する。実行時の`assert`とコンパイル時の精緻化型`Assert`は同じ原始の二つの側面である。

```yaoxiang
// IsTrue：值到类型的桥接函数
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤，程序继续
    false => Never,    // ⊥，发散
}

// Assert：编译期精化类型原语
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert：运行时断言（Assert 的值引入子）
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Result 重载
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**dispatchディスパッチ**：

| 条件                                     | 動作                                                    |
| ---------------------------------------- | ------------------------------------------------------- |
| condのすべての自由変数がコンパイル時既知 | コンパイラが評価、true → 消去、false → コンパイルエラー |
| 実行時自由変数が存在する                 | 実行時checkを挿入し、フロー敏感仮定集合Γを注入する      |

`assert(false, "msg")`はraiseと等価である。個別のthrow/raiseキーワードは不要。

---

## 第2章：IOライブラリ

### 2.1 標準入出力

```yaoxiang
// 标准输出
print: (msg: String) -> Void
println: (msg: String) -> Void

// 标准输入
read_line: () -> String
read_char: () -> Char
```

### 2.2 ファイル操作

```yaoxiang
// 文件类型
File: Type = {
    path: String,
    read: (self: File) -> Result(String, Error),
    write: (self: File, content: String) -> Result(Void, Error),
    append: (self: File, content: String) -> Result(Void, Error),
    close: (self: File) -> Void
}

// 文件操作
open: (path: String) -> Result(File, Error)
create: (path: String) -> Result(File, Error)
delete: (path: String) -> Result(Void, Error)
```

### 2.3 ディレクトリ操作

```yaoxiang
// 目录类型
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result(List(String), Error),
    create: (self: Dir) -> Result(Void, Error),
    delete: (self: Dir) -> Result(Void, Error)
}

// 目录操作
read_dir: (path: String) -> Result(Dir, Error)
create_dir: (path: String) -> Result(Void, Error)
delete_dir: (path: String) -> Result(Void, Error)
```

---

## 第3章：数学ライブラリ

### 3.1 基本数学関数

```yaoxiang
// 绝对值
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// 最大最小值
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// 幂运算
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// 对数
log: (x: Float) -> Float
log2: (x: Float) -> Float
log10: (x: Float) -> Float
```

### 3.2 三角関数

```yaoxiang
// 三角函数
sin: (x: Float) -> Float
cos: (x: Float) -> Float
tan: (x: Float) -> Float

// 反三角函数
asin: (x: Float) -> Float
acos: (x: Float) -> Float
atan: (x: Float) -> Float
atan2: (y: Float, x: Float) -> Float
```

### 3.3 定数

```yaoxiang
// 数学常量
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## 第4章：文字列ライブラリ

### 4.1 文字列操作

```yaoxiang
// 字符串长度
length: (s: String) -> Int

// 字符串拼接
concat: (a: String, b: String) -> String

// 字符串分割
split: (s: String, delimiter: String) -> List(String)

// 字符串查找
find: (s: String, pattern: String) -> Option(Int)
contains: (s: String, pattern: String) -> Bool

// 字符串替换
replace: (s: String, old: String, new: String) -> String

// 字符串修剪
trim: (s: String) -> String
trim_left: (s: String) -> String
trim_right: (s: String) -> String
```

### 4.2 文字列変換

```yaoxiang
// 类型转换
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// 解析
parse_int: (s: String) -> Result(Int, Error)
parse_float: (s: String) -> Result(Float, Error)
```

---

## 第5章：コレクションライブラリ

### 5.1 List 型

```yaoxiang
// List 类型
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
// Map 类型
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
// 范围迭代器（#300 I 项：Range 是一等值，step 为第三分量）
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// 使用
for i in 0..10 {
    print(i)
}

// step 形态（双点，无新关键词）
for i in 0..10..2 {
    print(i)
}
```

> **#300
> I項**：Rangeは第一級値である——`r = 1..10`は合法であり、`x in r`のメンバ判定、`for i in r`のイテレーションはいずれも静的型で脱糖される。step=0リテラルはコンパイル時に拒否され、動的step=0は実行時エラー（将来のエラーシステム実装後にResultに格上げ、#301）。

---

## 付録：標準ライブラリモジュール索引

| モジュール       | 説明                                                              |
| ---------------- | ----------------------------------------------------------------- |
| `std.assert`     | アサーションメカニズム——実行時assert + コンパイル時Assert精緻化型 |
| `std.option`     | Option型                                                          |
| `std.result`     | Result型                                                          |
| `std.collection` | List、Map等のコレクション型                                       |
| `std.string`     | 文字列操作                                                        |
| `std.array`      | 配列操作                                                          |
| `std.iterator`   | イテレータ                                                        |

### A.2 IOモジュール

| モジュール | 説明             |
| ---------- | ---------------- |
| `std.io`   | 標準入出力       |
| `std.file` | ファイル操作     |
| `std.dir`  | ディレクトリ操作 |

### A.3 数学モジュール

| モジュール      | 説明     |
| --------------- | -------- |
| `std.math`      | 数学関数 |
| `std.math.trig` | 三角関数 |
| `std.math.log`  | 対数関数 |

### A.4 ユーティリティモジュール

| モジュール   | 説明                                                            |
| ------------ | --------------------------------------------------------------- |
| `std.random` | 乱数生成                                                        |
| `std.time`   | 日時                                                            |
| `std.assert` | コンパイル時`Assert(C)`と実行時`assert(x > 0)`の統一（RFC-030） |
| `std.regex`  | 正規表現                                                        |
