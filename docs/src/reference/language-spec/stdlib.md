# 标准库规范

本文件定义 YaoXiang 编程语言的标准库规范，包括核心库、IO库和数学库。

---

## 第一章：核心库

### 1.1 基础类型

标准库提供以下基础类型的实现：

| 类型 | 模块 | 说明 |
|------|------|------|
| `Option(T)` | `std.option` | 可选值类型 |
| `Result(T, E)` | `std.result` | 错误处理类型 |
| `List(T)` | `std.collection` | 动态数组 |
| `Map(K, V)` | `std.collection` | 哈希映射 |
| `String` | `std.string` | 字符串类型 |
| `Array(T, N)` | `std.array` | 固定大小数组 |

### 1.2 Option 类型

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**变体构造**：

| 变体 | 语法 | 说明 |
|------|------|------|
| `Option.some` | `Option.some(value)` | 有值 |
| `Option.none` | `Option.none()` | 无值 |

**常用方法**：

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

### 1.3 Result 类型

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**变体构造**：

| 变体 | 语法 | 说明 |
|------|------|------|
| `Result.ok` | `Result.ok(value)` | 成功值 |
| `Result.err` | `Result.err(error)` | 错误值 |

**常用方法**：

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

### 1.4 错误传播

```
ErrorPropagate ::= Expr '?'
```

`?` 运算符自动传播 Result 类型的错误：

```
// 成功时返回值，失败时向上返回 err
data = fetch_data()?

// 等价于
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```


### 1.5 断言（std.assert）

`std.assert` 模块提供统一的断言机制——运行时 `assert` 和编译期精化类型 `Assert` 是同一原语的两面。

```yaoxiang
# IsTrue：值到类型的桥接函数
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤，程序继续
    false => Never,    # ⊥，发散
}

# Assert：编译期精化类型原语
Assert: (cond: Bool) -> Type = IsTrue(cond)

# assert：运行时断言（Assert 的值引入子）
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

# Result 重载
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**dispatch 分派**：

| 条件 | 行为 |
|------|------|
| cond 的所有自由变量编译期已知 | 编译器求值，true → 擦除，false → 编译错误 |
| 存在运行时自由变量 | 插入运行时 check，注入流敏感假设集 Γ |

`assert(false, "msg")` 等价于 raise——不需要单独的 throw/raise 关键字。

---

## 第二章：IO 库

### 2.1 标准输入输出

```yaoxiang
// 标准输出
print: (msg: String) -> Void
println: (msg: String) -> Void

// 标准输入
read_line: () -> String
read_char: () -> Char
```

### 2.2 文件操作

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

### 2.3 目录操作

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

## 第三章：数学库

### 3.1 基础数学函数

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

### 3.2 三角函数

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

### 3.3 常量

```yaoxiang
// 数学常量
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## 第四章：字符串库

### 4.1 字符串操作

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

### 4.2 字符串转换

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

## 第五章：集合库

### 5.1 List 类型

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

### 5.2 Map 类型

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

## 第六章：迭代器库

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

### 6.2 迭代器适配器

```yaoxiang
// 范围迭代器
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

for i in 0..10 step 2 {
    print(i)
}
```

---

## 附录：标准库模块索引

### A.1 核心模块

| 模块 | 说明 |
|------|------|
| `std.assert` | 断言机制——运行时 assert + 编译期 Assert 精化类型 |
| `std.option` | Option 类型 |
| `std.result` | Result 类型 |
| `std.collection` | List、Map 等集合类型 |
| `std.string` | 字符串操作 |
| `std.array` | 数组操作 |
| `std.iterator` | 迭代器 |
### A.2 IO 模块

| 模块 | 说明 |
|------|------|
| `std.io` | 标准输入输出 |
| `std.file` | 文件操作 |
| `std.dir` | 目录操作 |

### A.3 数学模块

| 模块 | 说明 |
|------|------|
| `std.math` | 数学函数 |
| `std.math.trig` | 三角函数 |
| `std.math.log` | 对数函数 |

### A.4 工具模块

| 模块 | 说明 |
|------|------|
| `std.random` | 随机数生成 |
| `std.time` | 时间日期 |
| `std.regex` | 正则表达式 |
