# Standard Library Specification

This document defines the standard library specification for the YaoXiang programming language,
including the core library, IO library, and math library.

---

## Chapter 1: Core Library

### 1.1 Basic Types

The standard library provides implementations for the following basic types:

| Type           | Module           | Description         |
| -------------- | ---------------- | ------------------- |
| `Option(T)`    | `std.option`     | Optional value type |
| `Result(T, E)` | `std.result`     | Error handling type |
| `List(T)`      | `std.collection` | Dynamic array       |
| `Map(K, V)`    | `std.collection` | Hash map            |
| `String`       | `std.string`     | String type         |
| `Array(T, N)`  | `std.array`      | Fixed-size array    |

### 1.2 Option Type

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**Variant Constructors**:

| Variant       | Syntax               | Description |
| ------------- | -------------------- | ----------- |
| `Option.some` | `Option.some(value)` | Has a value |
| `Option.none` | `Option.none()`      | No value    |

**Common Methods**:

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

### 1.3 Result Type

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**Variant Constructors**:

| Variant      | Syntax              | Description   |
| ------------ | ------------------- | ------------- |
| `Result.ok`  | `Result.ok(value)`  | Success value |
| `Result.err` | `Result.err(error)` | Error value   |

**Common Methods**:

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

**Error Carrier and Error Codes (#323 M4)**:

The `Error` Err carrier of each std module carries normalized error codes, reusing the E6xxx/E7xxx
segments of RFC-013 (e.g., E6009 = invalid Range step). This forms a stable cross-version
contract—programs can branch by code, and `yx explain E6009` looks up documentation. The code index
is found in the "Runtime Error Values and Code Mapping" section of RFC-013.

```yaoxiang
// Error 值形态：{ code: String, message: String }

// 取出 Err 载体（Ok 时报运行时错误）
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// 读取错误码 / 消息
code: (self: Error) -> String
message: (self: Error) -> String
```

**Branch-by-Code Example**:

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

User-defined error modeling uses the `E` generic parameter of `Result(T, E)` (a user-defined variant
set). The std `Error` is a convenient fallback carrier, and its code system does not constrain the
user's `E` type.

### 1.4 Error Propagation

```
ErrorPropagate ::= Expr '?'
```

The `?` operator automatically propagates Result-type errors:

```
// 成功时返回值，失败时向上返回 err
data = fetch_data()?

// 概念等价形式（注意：变体解构 match 尚未落地——RFC-010b 交付前
// 会报编译错误 E3008，`?` 是当前唯一可用的错误传播写法）
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Assertions (std.assert)

The `std.assert` module provides a unified assertion mechanism—the runtime `assert` and the
compile-time refinement type `Assert` are two faces of the same primitive.

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

**Dispatch**:

| Condition                                              | Behavior                                                     |
| ------------------------------------------------------ | ------------------------------------------------------------ |
| All free variables of `cond` are known at compile-time | Compiler evaluates: true → erased, false → compile error     |
| Free variables exist at runtime                        | Insert runtime check, inject flow-sensitive assumption set Γ |

`assert(false, "msg")` is equivalent to raise—no separate `throw`/`raise` keyword is required.

---

## Chapter 2: IO Library

### 2.1 Standard Input/Output

```yaoxiang
// 标准输出
print: (msg: String) -> Void
println: (msg: String) -> Void

// 标准输入
read_line: () -> String
read_char: () -> Char
```

### 2.2 File Operations

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

### 2.3 Directory Operations

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

## Chapter 3: Math Library

### 3.1 Basic Math Functions

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

### 3.2 Trigonometric Functions

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

### 3.3 Constants

```yaoxiang
// 数学常量
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## Chapter 4: String Library

### 4.1 String Operations

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

### 4.2 String Conversion

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

## Chapter 5: Collection Library

### 5.1 List Type

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

### 5.2 Map Type

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

## Chapter 6: Iterator Library

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

### 6.2 Iterator Adapters

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

> **`Range(Int)` is now officially implemented**—the named fields `r.start`/`r.end`/`r.step` are
> accessible; `x in r` at runtime goes through `std.range.contains` (boundary check + step
> alignment), and the proof pipeline identifies the interval proposition
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (intervals are kept as intervals, not
> materialized). A `step=0` literal is rejected at compile-time; a dynamic `step=0` is now
> Result-typed: `std.range.iter` returns `Result(Iterator, Error)`, `std.range.contains` returns
> `Result(Bool, Error)`. Consumption sites propagate via `?` up the call stack or branch explicitly
> via `result.unwrap`; the `for`/`in` sugar is unwrapped during ir_gen, and the Err branch (dynamic
> `step=0`) fails explicitly (`abort_invalid_step`) and never silently dead-loops.
>
> Interface instantiation (the `Iterator(Int)` declaration in the type body) and static dispatch
> have landed with RFC-011a phases 1-2: the type-body application item `Iterator(Int)` triggers
> `Self ↦ Range` substitution expansion and completeness checking, and an implementation proof is
> generated upon success.
>
> Dynamic dispatch has landed with phase 3: a type exists as soon as the interface name is not
> instantiated (`List(Animal)`), and a concrete value entering an existential-type position is
> automatically wrapped as a value variant, with element method calls dispatched by actual type
> (§6). The runtime protocol surface of the `std.range` module is still provided by native methods
> for now; migration to interface dispatch is future work.

---

## Appendix: Standard Library Module Index

| Module           | Description                                                                                    |
| ---------------- | ---------------------------------------------------------------------------------------------- |
| `std.assert`     | Assertion mechanism—runtime `assert` + compile-time `Assert` refinement type                   |
| `std.option`     | Option type                                                                                    |
| `std.result`     | Result type                                                                                    |
| `std.collection` | Collection types: List, Map, etc.                                                              |
| `std.string`     | String operations                                                                              |
| `std.array`      | Array operations                                                                               |
| `std.iterator`   | Iterator (protocol surface currently provided by `std.range`)                                  |
| `std.range`      | Range iterator, interval predicate, and adapters                                               |
| `std.test`       | Test assertion library (value semantics, RFC-036 §3)—the first pure-YaoXiang dogfooding module |

### A.2 IO Modules

| Module     | Description           |
| ---------- | --------------------- |
| `std.io`   | Standard input/output |
| `std.file` | File operations       |
| `std.dir`  | Directory operations  |

### A.3 Math Modules

| Module          | Description             |
| --------------- | ----------------------- |
| `std.math`      | Math functions          |
| `std.math.trig` | Trigonometric functions |
| `std.math.log`  | Logarithm functions     |

### A.4 Utility Modules

| Module       | Description                                                             |
| ------------ | ----------------------------------------------------------------------- |
| `std.random` | Random number generation                                                |
| `std.time`   | Time and date                                                           |
| `std.assert` | Unifies compile-time `Assert(C)` with runtime `assert(x > 0)` (RFC-030) |
| `std.regex`  | Regular expressions                                                     |
