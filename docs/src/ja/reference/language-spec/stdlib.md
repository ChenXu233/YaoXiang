# 標準ライブラリ仕様

この文書は YaoXiang プログラミング言語の標準ライブラリ仕様を定義し、コアライブラリ、IO ライブラリ、数学ライブラリを含みます。

---

## 第一章：コアライブラリ

### 1.1 基本型

標準ライブラリは以下の基本型の実装を提供します：

| 型             | モジュール       | 説明           |
| -------------- | ---------------- | -------------- |
| `Option(T)`    | `std.option`     | オプション型   |
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

**一般的なメソッド**：

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

**一般的なメソッド**：

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

**Error キャリアとエラーコード（#323 M4）**：

std の各モジュールの Err キャリア `Error`
は正規化されたエラーコードを保持し、コードは RFC-013 の E6xxx/E7xxx セグメント（例：E6009 =
Range ステップ非合法）を再利用します。これはバージョン間で安定した契約であり、プログラムはコードに基づいてプログラミング判定を行うことができ、`yaoxiang explain E6009`
でドキュメントを調べることができます。コードインデックスは RFC-013「ランタイムエラー値とコード貫通」の章を参照してください。

```yaoxiang
// Error 值形态：{ code: String, message: String }

// 取出 Err 载体（Ok 时报运行时错误）
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// 读取错误码 / 消息
code: (self: Error) -> String
message: (self: Error) -> String
```

**コードによる判定の例**：

```yaoxiang
use std.range
use std.result

r = range.iter(1..10..0)      // step=0 → Err(Error)
if result.is_err(r) {
    e = result.unwrap_err(r)
    if result.code(e) == "E6009" {
        // 按 Range 步长非法分支处理
        io.println(result.message(e))
    }
}
```

ユーザー定義のエラーモデリングは `Result(T, E)`
の E ジェネリクス引数（カスタムバリアントセット）を使用します。std `Error`
は便利なフォールバックキャリアであり、そのコード体系はユーザーの E 型を制約しません。

### 1.4 エラー伝播

```
ErrorPropagate ::= Expr '?'
```

`?` 演算子は Result 型のエラーを自動的に伝播します：

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

`std.assert` モジュールは統一されたアサーション機構を提供します——ランタイムの `assert`
とコンパイル時の精錬型 `Assert` は同じプリミティブの二つの側面です。

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

**dispatch ディスパッチ**：

| 条件                                        | 動作                                                      |
| ------------------------------------------- | --------------------------------------------------------- |
| cond のすべての自由変数がコンパイル時に既知 | コンパイラが評価、true → 消去、false → コンパイルエラー   |
| ランタイムの自由変数が存在する              | ランタイムチェックを挿入し、流れに敏感な仮定集合 Γ を注入 |

`assert(false, "msg")` は raise と等価です——個別の throw/raise キーワードは必要ありません。

---

## 第二章：IO ライブラリ

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

## 第三章：数学ライブラリ

### 3.1 基本的な数学関数

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

## 第四章：文字列ライブラリ

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

## 第五章：コレクションライブラリ

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

## 第六章：イテレータライブラリ

### 6.1 Iterator トレイト

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
// 范围迭代器（Range 是正式类型，运行时身份为三标量不可变记录，
// 不再借 Tuple 外壳；打印 `1..10` / `1..10..2`，结构相等，具名字段）
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// 使用（迭代器协议：std.range.iter/has_next/next，for 经静态类型派发）
for i in 0..10 {
    print(i)
}

// step 形态（双点，无新关键词）
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` は既に正式に実装されています**——名前付きフィールド `r.start`/`r.end`/`r.step`
> にアクセス可能； `x in r` はランタイムに `std.range.contains`
> を経由し（境界チェック + ステップ位置合わせ）、証明パイプラインは区間命題
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0`
> として認識されます（区間は区間を保持し、実体化しません）。step=0 のリテラルはコンパイル時に拒否されます；動的 step=0 は既に Result 化されています：
> `std.range.iter` → `Result(Iterator, Error)`、`std.range.contains` →
> `Result(Bool, Error)`、消費点は `?` を使用して呼び出しスタックに沿って伝播するか、`result.unwrap`
> で明示的に分岐します；`for`/`in`
> の糖衣構文は ir_gen で展開され、Err 分岐（動的 step=0）は明示的に失敗します（`abort_invalid_step`）、決して静かに無限ループに陥ることはありません。インターフェース実体化（型本体
> `Iterator(Int)`
> 宣言）の型構文と静的ディスパッチは RFC-011a のフェーズ 1-2 とともに既に実装されています：型本体の適用項目
> `Iterator(Int)` が `Self ↦ Range`
> の置換展開と完全性チェックをトリガーし、合格後に実装証明が生成されます。動的ディスパッチはフェーズ 3 とともに既に実装されています：インターフェース名が実体化されていなくても存在型が存在し（`List(Animal)`）、具体的な値が存在型の位置に入ると自動的にバリアント値としてラップされ、要素メソッドの呼び出しは実際の型に従ってディスパッチされます（§6）。std.range モジュールのランタイムプロトコル面は現在もネイティブメソッドによって提供されており、インターフェースディスパッチへの移行は今後の作業です。

---

## 付録：標準ライブラリモジュールインデックス

| モジュール       | 説明                                                                                                     |
| ---------------- | -------------------------------------------------------------------------------------------------------- |
| `std.assert`     | アサーション機構——ランタイム assert + コンパイル時 Assert 精錬型                                         |
| `std.option`     | Option 型                                                                                                |
| `std.result`     | Result 型                                                                                                |
| `std.collection` | List、Map などのコレクション型                                                                           |
| `std.string`     | 文字列操作                                                                                               |
| `std.array`      | 配列操作                                                                                                 |
| `std.iterator`   | イテレータ（プロトコル面は現在 `std.range` が提供）                                                      |
| `std.range`      | Range イテレータと区間述語、アダプタ                                                                     |
| `std.test`       | テストアサーションライブラリ（値意味論、RFC-036 §3）——最初の純粋な YaoXiang ドッグフーディングモジュール |

### A.2 IO モジュール

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

| モジュール   | 説明                                                                    |
| ------------ | ----------------------------------------------------------------------- |
| `std.random` | 乱数生成                                                                |
| `std.time`   | 日時                                                                    |
| `std.assert` | コンパイル時 `Assert(C)` とランタイム `assert(x > 0)` の統一（RFC-030） |
| `std.regex`  | 正規表現                                                                |
