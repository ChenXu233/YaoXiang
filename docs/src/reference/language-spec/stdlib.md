# 标准库规范

本文件定义 YaoXiang 编程语言的标准库规范。

> **本章是语言层面的规范**（设计约定与语义模型）。逐个模块的**签名与可运行示例**见
> [标准库参考](../stdlib/index.md)——那部分由 `StdModule::exports()` 派生并有门禁守护。
> 本文件的每个签名都逐字取自 `src/std/` 下的实现（`*.rs` / `*.yx`）。

---

## 零章：模块全景

`std` 下共 **18 个模块路径**，由两种实现组成：

| 实现形式           | 模块                                                                                          | 注册来源                                     |
| ------------------ | --------------------------------------------------------------------------------------------- | -------------------------------------------- |
| native `StdModule` | `assert` `concurrent` `convert` `dict` `fs` `io` `math` `net` `os` `range` `result` `string` `time` `weak` | `src/std/mod.rs:17-39`（模块声明）+ `:460-482`（`all_module_infos()`） |
| 纯 yx（`.yx` 嵌入） | `list` `test` `option` `json`                                                                | `src/std/yx_sources.rs:12-18`                |
| 双实现（合并）     | `result`                                                                                      | `src/std/result.rs`（工具族）+ `src/std/result.yx`（`Result` 类型），`src/frontend/module/registry.rs:315-334` 做并集合并 |

**平台可用性**：`concurrent` / `fs` / `net` / `os` / `weak` 以及 `io.read_line` /
`time.sleep` 依赖操作系统能力，在 `wasm32` 目标上**不导出**（`src/std/mod.rs` 各模块
声明上的 `#[cfg(not(target_arch = "wasm32"))]`）。

---

## 第一章：核心库

### 1.1 基础类型

| 类型                 | 模块                 | 说明                                    |
| -------------------- | -------------------- | --------------------------------------- |
| `Option(T)`          | `std.option`         | 可选值（和类型）                        |
| `Result(T, E)`       | `std.result`         | 错误处理（和类型）                      |
| `Vec(T)`             | 核心原语             | 运行时长度的原始缓冲；`std.list` 在其上实现 |
| `Dict(K, V)`         | 核心原语             | 字典；`std.dict` 提供模块级读写函数      |
| `String`             | `std.string`         | 字符串；`std.string` 提供操作函数        |
| `Array(T, N)`        | 核心原语             | 固定大小数组（语法见 [语法规范 §1.6.4](syntax.md)） |

> **`List(T)` / `Map(K, V)` 不是标准库类型**。`std.list` 是**模块级函数集合**，
> 底座是核心原语 `Vec(T)`（`src/std/list.yx:1-21`）；`std.dict` 同样是一组
> 操作 `Dict(K, V)` 的模块级函数（`src/std/dict.rs`）。写 `List(T)` /
> `Map(K, V)` 作为类型会在类型检查阶段失败。

### 1.2 Option 类型

`src/std/option.yx` 全文：

```
Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

**变体构造**（表达式位置必须类型限定，见 [语法规范 §1.4.2](syntax.md)）：

| 变体           | 语法                    | 说明 |
| -------------- | ----------------------- | ---- |
| `Option.some`  | `Option(Int).some(5)`   | 有值 |
| `Option.none`  | `Option(Int).none()`    | 无值 |

**方法**（在类型体里声明，**不是模块级导出**——`option.is_failure(...)` 报 `E1042`）：

| 方法                  | 签名                                          | `some` 臂        | `none` 臂         |
| --------------------- | --------------------------------------------- | ---------------- | ----------------- |
| `is_failure`          | `(T: Type)(self: &Option(T)) -> Bool`          | `false`          | `true`            |
| `success`             | `(T: Type)(self: &Option(T)) -> T`             | 返回载荷         | `assert(false)`   |
| `residual`            | `(T: Type)(self: &Option(T)) -> Void`          | `assert(false)`  | 返回 `void`       |
| `from_error`          | `(T: Type)(v: Void) -> Option(T)`              | 恒返回 `none()`  | 恒返回 `none()`   |

`assert(false)` 分支是 `Never` 型的死路（`Never <: T`，见
[类型系统 §2.2](type-system.md)），运行时报 `E6005`。

> **没有 `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`**——这些名字在
> `src/` 全仓 0 命中。判定失败用 `is_failure()`，取载荷用 `success()`。
>
> **`?` 传播在 `Option` 上当前不可用**：类型体虽实例化了 `Try(Option(T), T, Void)`，
> 类型检查器只承认 `Result`（`o?` 报 `E1081`）。`from_error` 是 `?` 的内部桥，
> 因此也没有可达调用点（`o.from_error()` 运行时报 `E6006`）。

### 1.3 Result 类型

`Result` 类型本体（`src/std/result.yx:19-23`）：

```
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E),
    Try(Result(T, E), T, E),
}
```

**变体构造**（必须类型限定——裸 `ok(5)` 报 `E1001 Unknown variable: 'ok'`）：

| 变体           | 语法                            | 说明   |
| -------------- | ------------------------------- | ------ |
| `Result.ok`    | `Result(Int, String).ok(5)`     | 成功值 |
| `Result.err`   | `Result(Int, String).err("e")`  | 错误值 |

**方法**：类型体的 `Try` 四方法（`is_failure` / `success` / `residual` /
`from_error`，只能以方法语法调用）＋ native 的 8 个工具导出
（`src/std/result.rs:71-126`）：

```
is_ok:      (T: Type, E: Type)(self: &Result(T, E)) -> Bool
is_err:     (T: Type, E: Type)(self: &Result(T, E)) -> Bool
unwrap:     (T: Type, E: Type)(self: &Result(T, E)) -> T
unwrap_or:  (T: Type, E: Type)(self: &Result(T, E), default: T) -> T
unwrap_err: (T: Type, E: Type)(self: &Result(T, E)) -> E
code:       (self: &Error) -> String
message:    (self: &Error) -> String
error:      (code: &String, message: &String) -> Error
```

> **没有 `map` / `map_err`**。`Result` 上 `?` 传播是**可用**的（与 `Option` 不同）。

**Error 载体与错误码（#323 M4）**：

std 各模块的 Err 载体 `Error` 携带规范化错误码，码复用 RFC-013 的 E6xxx/E7xxx 段位，
为跨版本稳定契约——程序可按码编程判定，`yx explain E6009` 可查文档。运行时表示是
`Struct { code: String, message: String }`（`src/std/result.rs:52-68`）。已注册码见
`src/std/result.rs:26-32` 的 `RUNTIME_ERROR_CODES`：`E6009`（range 步长非法）、
`E6010`（parse_int 失败）、`E6011`（parse_float 失败）、`E6012`（非法码点）、
`E6013`（JSON 解析失败）。

`Error` 值只能由 `result.error(code, message)` 构造——类型族只注册类型身份，没有值空间
构造子。

### 1.4 错误传播

```
ErrorPropagate ::= Expr '?'
```

`?` 运算符自动传播 `Result` 类型的错误（`use std.result` 后，match 变体解构是
`?` 的显式等价形式——变体集随 `use` 导入，见 [语法规范 §2.8](syntax.md)）：

```
// 成功时返回值，失败时向上返回 err
data = fetch_data()?

// 概念等价形式
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

约束：`?` 只能出现在返回类型实现了 `Try` 的函数里，外层返回 `Result(T, E)`；
否则报 `E1081`。`Option` 的 `Try` 实例化尚未被检查器承认（见 §1.2）。

### 1.5 断言（std.assert）

`std.assert` 只有一个 native 导出（`src/std/assert.rs:23-30`）：

```
assert: (cond: Bool, ?msg: String) -> Never
```

另注册两个类型族（`src/std/assert.rs:32-51`）：

```
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤，程序继续
    false => Never,    // ⊥，发散
}

Assert: (cond: Bool) -> Type = IsTrue(cond)
```

**当前状态**：

- 运行时 `assert` 可用——条件为假报 `E6005`；条件为真时实现返回 `Void`
  （`src/std/assert.rs:82-83`），与声明的 `Never` 不矛盾（`Never <: T`）
- **不存在 `Assert(IsTrue(cond))` 作为返回类型的重载**，也不存在接收 `Result` 的重载
- 两个类型族**在类型位置无法书写**（`Assert(true)` 报 `E0010`，
  `assert.Assert(true)` 报 `E0012`）——编译期精化原语尚未接通

**dispatch 分派**：

| 条件                          | 行为                                      |
| ----------------------------- | ----------------------------------------- |
| cond 的所有自由变量编译期已知 | 编译器求值，true → 擦除，false → 编译错误 |
| 存在运行时自由变量            | 插入运行时 check，注入流敏感假设集 Γ      |

`assert(false, "msg")` 等价于 raise——不需要单独的 throw/raise 关键字。

---

## 第二章：IO 库

### 2.1 标准输入输出

`std.io` 共 4 个导出（`src/std/io.rs:53-77`）：

```
print: (...args) -> Void
println: (...args) -> ()
read_line: () -> String            // wasm32 上不导出
format_fallback: (value, type_name: &String) -> String
```

> **没有 `read_char`**。整文件读写（`read_file` / `write_file` / `append_file`）已迁往
> `std.fs`（`src/std/io.rs:69` 注明 #104）。

### 2.2 文件与目录

路径级操作全在 `std.fs`（22 个导出，`src/std/fs.rs:36-169`）；句柄级增量读写在
`std.os`（12 个导出，`src/std/os.rs:26-89`）。**不存在 `File` / `Dir` 记录类型，
也没有 `create` / `delete` / `create_dir` / `delete_dir`。**

`std.os`（句柄模型，形参一律 `&File`——句柄可反复使用）：

```
open:  (path: &String, mode: &String) -> File
close: (file: &File) -> Void
read:  (file: &File, n: Int) -> String
write: (file: &File, content: String) -> Int
seek:  (file: &File, offset: Int) -> Bool
tell:  (file: &File) -> Int
flush: (file: &File) -> Void
get_env:  (name: &String) -> String
set_env:  (name: &String, value: &String) -> Void
args:     () -> String
chdir:    (path: &String) -> Bool
getcwd:   () -> String
```

`std.fs`（路径模型）：

```
read_file / write_file / append_file      // 整文件读写
exists / is_file / is_dir / stat           // 元数据判定
mkdir / mkdir_all / rmdir / remove          // 目录与删除
copy / rename                              // 搬运
read_dir / walk                            // 目录枚举
temp_dir / mkdtemp / tmpfile               // 临时路径
path_join / path_basename / path_dirname / path_extension   // 纯路径运算
```

---

## 第三章：数学库

### 3.1 基础数学函数

`std.math` 共 18 个导出（`src/std/math.rs:20-75`）。**整数族与浮点族是两个独立的名字，
没有隐式转换与重载**：

```
abs:   (n: Int) -> Int          // 整数绝对值
max:   (a: Int, b: Int) -> Int
min:   (a: Int, b: Int) -> Int
clamp: (value: Int, min: Int, max: Int) -> Int

fabs:  (n: Float) -> Float      // 浮点绝对值
fmax:  (a: Float, b: Float) -> Float
fmin:  (a: Float, b: Float) -> Float
pow:   (base: Float, exp: Float) -> Float
sqrt:  (n: Float) -> Float
floor: (n: Float) -> Float
ceil:  (n: Float) -> Float
round: (n: Float) -> Float
```

> **`abs` / `max` / `min` 只接受 `Int`**——没有 `abs: (x: Float) -> Float` 重载，
> 浮点用 `fabs` / `fmax` / `fmin`。
>
> **`clamp` 在 `min > max` 时返回 `E6007`**（`src/std/math.rs:117-130`），不再 panic
> 掉解释器进程。
>
> **没有对数函数族**：`log` / `log2` / `log10` 全仓 0 命中。

### 3.2 三角函数

```
sin: (n: Float) -> Float
cos: (n: Float) -> Float
tan: (n: Float) -> Float
```

参数为**弧度**。**没有反三角函数**——`asin` / `acos` / `atan` / `atan2` 全仓 0 命中。

### 3.3 常量

```
PI:  Float = 3.141592653589793
E:   Float = 2.718281828459045
TAU: Float = 6.283185307179586
```

导出名是**大写**的 `PI` / `E` / `TAU`（`src/std/math.rs:71-73`），没有小写 `pi` / `e`。
按名导入即用：`use std.math.{PI, E, TAU}`。

---

## 第四章：字符串库

### 4.1 字符串操作

`std.string` 共 21 个导出（`src/std/string.rs:22-146`）。除 `format` 外全部只读借用
`&String`：

```
split:      (s: &String, sep: &String) -> Vec(String)
trim:       (s: &String) -> String
upper:      (s: &String) -> String
lower:      (s: &String) -> String
replace:    (s: &String, old: &String, new: &String) -> String
contains:   (s: &String, sub: &String) -> Bool
starts_with: (s: &String, prefix: &String) -> Bool
ends_with:  (s: &String, suffix: &String) -> Bool
index_of:   (s: &String, sub: &String) -> Int
substring:  (s: &String, start: Int, end: Int) -> String
is_empty:   (s: &String) -> Bool
len:        (s: &String) -> Int
chars:      (s: &String) -> Vec(String)
concat:     (s1: &String, s2: &String) -> String
repeat:     (s: &String, n: Int) -> String
reverse:    (s: &String) -> String
format:     (format: &String, ...args) -> String
```

> **没有 `length`**——长度函数叫 `len`，返回 **UTF-8 字节长度**。
>
> **没有 `find`**——查找下标函数叫 `index_of`，返回**字节**偏移，未命中为 `-1`。
>
> **只有 `trim` 一个**——不存在 `trim_left` / `trim_right`。

### 4.2 字符串转换与码点

```
parse_int:      (s: &String) -> Result(Int, Error)      // 失败码 E6010
parse_float:    (s: &String) -> Result(Float, Error)    // 失败码 E6011
char_code:      (s: &String, i: Int) -> Int            // 越界返回 -1
from_char_code: (n: Int) -> Result(String, Error)      // 非法码点码 E6012
```

值转字符串走 `std.convert`（`to_string` 等 11 个导出）。

---

## 第五章：集合库

### 5.1 列表（std.list）

`std.list` 是**定义在 `Vec(T)` 上的模块级函数集合**（`src/std/list.yx`），
**不是**一个记录类型。25 个导出（24 个函数 + 1 个 `Iter` 记录类型）：

```
empty:      (T: Type) -> Vec(T)
of:         (T: Type) -> (data: Vec(T)) -> Vec(T)

// 消耗源列表，返回新列表
push:       (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
append:     (A: Type) -> (list: Vec(A), item: A) -> Vec(A)     // push 的别名
prepend:    (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
set:        (A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)
pop:        (A: Type) -> (list: Vec(A)) -> Vec(A)
remove_at:  (A: Type) -> (list: Vec(A), index: Int) -> Vec(A)

// 只读借用，返回新列表 / 标量
len:        (A: Type) -> (list: &Vec(A)) -> Int
is_empty:   (A: Type) -> (list: &Vec(A)) -> Bool
get:        (A: Type) -> (list: &Vec(A), index: Int) -> A
first:      (A: Type) -> (list: &Vec(A)) -> A
last:       (A: Type) -> (list: &Vec(A)) -> A
contains:   (A: Type) -> (list: &Vec(A), item: &A) -> Bool
find_index: (A: Type) -> (list: &Vec(A), item: &A) -> Int
slice:      (A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)
reverse:    (A: Type) -> (list: &Vec(A)) -> Vec(A)
concat:     (A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)
map:        (T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)
filter:     (T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)
reduce:     (T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc

// 迭代器
Iter:       (T: Type) -> Type                 // { buf: Vec(T), pos: Int }
iter:       (T: Type) -> (list: Vec(T)) -> Iter(T)
has_next:   (T: Type) -> (it: &Iter(T)) -> Bool
next:       (T: Type) -> (it: &mut Iter(T)) -> T
```

**边界语义**（`src/std/list.yx:76-205`）：取值与切片直接用 `[]` 下标，**越界即 `E6003`**，
既不返回哨兵值也不钳制边界。`first` / `last` 作用于空列表同样报 `E6003`。
`remove_at` 的下标**为负**报 `E6003`，但**≥ 长度时静默截断**（只把 `length` 减 1）。

**不存在** `insert` / `clear` / `sort`，也没有原地改写形态的方法——`pop` / `remove_at`
按值接收并**返回缩短后的新列表**（值语义，源列表被消费）。

### 5.2 字典（std.dict）

`std.dict` 是**操作 `Dict(K, V)` 的模块级函数集合**（`src/std/dict.rs`），**不是**记录类型。
11 个导出：

```
new:     (K: Type, V: Type)() -> Dict(K, V)
get:     (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any    // 缺键报 E6008
set:     (K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)
has:     (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool
delete:  (K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)
keys:    (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
values:  (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
entries: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
len:     (K: Type, V: Type)(dict: &Dict(K, V)) -> Int
is_empty:(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool
merge:   (A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)
```

> **不存在** `insert` / `clear`（`Dict` 是核心原语，容量策略不开放）。
> 空字典用 `dict.new()`——`{}` 是**空块**（值 `Void`），不是空字典。

---

## 第六章：迭代器库

### 6.1 Iterator 接口

`Iterator` 是**语言接口**（RFC-011a），不是标准库模块：

```
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

协议面的**运行时实现**由 `std.range` 提供（`src/std/range.rs:29-90`），
`std.list` 另有一套自己的 `Iter` 记录。两侧的**形参形态相反**：
`range.has_next` 无 `&`（按值消耗迭代器）而 `range.next` 有 `&`；
`list.has_next` / `list.next` 都有 `&` / `&mut`。日常遍历直接用 `for ... in`。

> **`Iterator` 协议没有对应的独立模块**——`std` 下不存在名为 `iterator` 的模块路径
> （该名字在 `src/` 全仓 0 命中）。

### 6.2 迭代器适配器

```
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}
```

```
// 使用（迭代器协议：std.range.iter/has_next/next，for 经静态类型派发）
for i in 0..10 {
    print(i)
}

// step 形态（双点，无新关键词）
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` 已正式落地**——具名字段 `r.start`/`r.end`/`r.step` 可访问；
> `x in r` 运行时走 `std.range.contains`（界检查 + 步长对齐），证明管道识别为区间命题
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0`（区间保持区间，不物化）。
> step=0 字面量编译期拒绝；动态 step=0 已 Result 化：
> `std.range.iter` → `Result(Iterator, Error)`（码 `E6009`）、`std.range.contains` →
> `Result(Bool, Error)`，消费点用 `?` 沿调用栈传播或 `result.unwrap` 显式分流；
> `for`/`in` 糖降级在 ir_gen 解包，Err 分支（动态 step=0）显式失败
> （`abort_invalid_step`），绝不静默死循环。

---

## 附录 A：标准库模块索引

下表只列**真实存在**的模块路径（与 [标准库参考](../stdlib/index.md) 的两张表对应）。

| 模块            | 实现   | 说明                                                       |
| --------------- | ------ | ---------------------------------------------------------- |
| `std.assert`    | native | 运行时 `assert`（+ 两个尚未可用的类型族 `IsTrue` / `Assert`） |
| `std.option`    | yx     | `Option(T)` 可选值与和类型                                 |
| `std.result`    | 双实现 | `Result(T, E)` 与 `Error`；`?` 传播当前唯一可用处          |
| `std.list`      | yx     | `Vec(T)` 上的列表操作与 `Iter` 迭代器                      |
| `std.dict`      | native | `Dict(K, V)` 读写、键值视图与合并                          |
| `std.string`    | native | 字符串查找、切分、格式化、解析与码点                        |
| `std.math`      | native | 整数族、浮点族、三角函数与 `PI` / `E` / `TAU`              |
| `std.time`      | native | Unix 时间戳、UTC 格式化与解析、`datetime_*` 访问器          |
| `std.range`     | native | `Range` 迭代器协议、区间谓词与惰性适配器                   |
| `std.json`      | yx     | JSON 解析与序列化（RFC 8259）                              |
| `std.convert`   | native | 任意值到 `String` 的转换                                   |
| `std.test`      | yx     | 测试断言库（值语义，RFC-036 §3），首个纯 yx dogfooding 模块 |

### A.2 平台相关模块

| 模块            | 实现   | 依赖         | wasm32  |
| --------------- | ------ | ------------ | ------- |
| `std.io`        | native | 标准 I/O     | 仅 `read_line` 不导出 |
| `std.fs`        | native | 文件系统     | 不导出  |
| `std.os`        | native | 文件/进程    | 不导出  |
| `std.net`       | native | 网络（`ureq` + rustls TLS，同步阻塞） | 不导出 |
| `std.concurrent`| native | 线程         | 不导出  |
| `std.weak`      | native | 原子引用计数 | 不导出  |

### A.3 已移除的模块名

以下名字曾是 `std` 的模块名，但**在 `src/` 全仓 0 命中**，不得再使用（查证方式：
对 `src/` 目录全文检索这些标识符）：

| 旧名           | 现由谁承担                        |
| -------------- | --------------------------------- |
| `collection`   | `std.list`（`Vec(T)`）＋ `std.dict`（`Dict(K, V)`） |
| `array`        | 核心原语 `Array(T, N)`            |
| `iterator`     | `std.range` / `std.list` 的协议面 |
| `file`         | `std.fs`                          |
| `dir`          | `std.fs`                          |
| `math.trig`    | `std.math`（`sin` / `cos` / `tan`）|
| `math.log`     | 无对数函数族                      |
| `random`       | 无                                |
| `regex`        | 无                                |
