# YaoXiang（爻象）编程语言规范

> 版本：v1.0.0
> 状态：规范
> 作者：沫郁酱
> 日期：2024-12-31

---

## 第一章：引言

### 1.1 范围

本文档定义了 YaoXiang 编程语言的语法和语义。它是语言的权威参考，面向编译器和工具实现者。

### 1.2 符合性

一个程序或实现如果满足本文档中定义的所有规则，则认为是符合 YaoXiang 规范的。

---

## 第二章：词法结构

### 2.1 源文件

YaoXiang 源文件必须使用 UTF-8 编码。源文件通常以 `.yx` 为扩展名。

### 2.2 词法单元分类

| 类别 | 说明 | 示例 |
|------|------|------|
| 标识符 | 以字母或下划线开头 | `x`, `_private`, `my_var` |
| 关键字 | 语言预定义保留词 | `fn`, `type`, `mod` |
| 字面量 | 固定值 | `42`, `"hello"`, `true` |
| 运算符 | 运算符号 | `+`, `-`, `*`, `/` |
| 分隔符 | 语法分隔符 | `(`, `)`, `{`, `}`, `,` |

### 2.3 关键字

YaoXiang 共定义 18 个关键字：

```
type   fn     pub    mod    use
spawn  ref    mut    if     elif
else   match  while  for    return
break  continue as
```

### 2.4 保留字

| 保留字 | 类型 | 说明 |
|--------|------|------|
| `true` | Bool | 布尔真值 |
| `false` | Bool | 布尔假值 |
| `null` | Void | 空值 |
| `none` | Option | Option 无值变体 |
| `some(T)` | Option | Option 值变体 |
| `ok(T)` | Result | Result 成功变体 |
| `err(E)` | Result | Result 错误变体 |

### 2.5 标识符

标识符以字母或下划线开头，后续字符可以是字母、数字或下划线。标识符大小写敏感。

特殊标识符：
- `_` 用作占位符，表示忽略某个值
- 以下划线开头的标识符表示私有成员

### 2.6 字面量

#### 整数

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 浮点数

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 字符串

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 集合

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Set         ::= '{' Expr (',' Expr)* '}'
```

### 2.7 注释

```yaoxiang
# 单行注释

#! 多行注释
   可以跨越多行 !#
```

### 2.8 缩进规则

代码必须使用 4 个空格缩进，禁止使用 Tab 字符。这是强制语法规则。

---

## 第三章：类型

### 3.1 类型分类

```
TypeExpr    ::= PrimitiveType
              | StructType
              | UnionType
              | EnumType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
```

### 3.2 原类型

| 类型 | 描述 | 默认大小 |
|------|------|----------|
| `Void` | 空值 | 0 字节 |
| `Bool` | 布尔值 | 1 字节 |
| `Int` | 有符号整数 | 8 字节 |
| `Uint` | 无符号整数 | 8 字节 |
| `Float` | 浮点数 | 8 字节 |
| `String` | UTF-8 字符串 | 可变 |
| `Char` | Unicode 字符 | 4 字节 |
| `Bytes` | 原始字节 | 可变 |

带位宽的整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
带位宽的浮点：`Float32`, `Float64`

### 3.3 结构体类型

```
StructType  ::= 'struct' '{' FieldList '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
```

```yaoxiang
type Point = struct {
    x: Float
    y: Float
}

type Person = struct {
    name: String
    age: Int
}
```

### 3.4 联合类型

```
UnionType   ::= 'union' '{' VariantList '}'
VariantList ::= Variant (',' Variant)* ','?
Variant     ::= Identifier (':' TypeExpr)?
```

```yaoxiang
type Result[T, E] = union {
    ok: T
    err: E
}

type Option[T] = union {
    some: T
    none
}
```

### 3.5 枚举类型

```
EnumType    ::= 'enum' '{' Identifier (',' Identifier)* ','? '}'
```

```yaoxiang
type Color = enum {
    red
    green
    blue
}
```

### 3.6 元组类型

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

```yaoxiang
Point2D = (Float, Float)
Triple = (Int, String, Bool)
```

### 3.7 函数类型

```
FnType      ::= 'fn' '(' ParamList? ')' '->' TypeExpr
ParamList   ::= Param (',' Param)*
Param       ::= Identifier ':' TypeExpr
```

```yaoxiang
type Adder = fn(Int, Int) -> Int
type Callback = fn(T) -> Void
```

### 3.8 泛型类型

```
GenericType ::= Identifier '[' TypeArgList ']'
TypeArgList ::= TypeExpr (',' TypeExpr)* ','?
```

```yaoxiang
type List[T] = struct { elements: [T], length: Int }
type Map[K, V] = struct { keys: [K], values: [V] }

# 使用
numbers: List[Int] = [1, 2, 3]
```

### 3.9 依赖类型

```
DependentType ::= Identifier '[' TypeParamList ']' TypeExpr?
TypeParamList ::= Identifier ':' TypeExpr (',' Identifier ':' TypeExpr)*
```

```yaoxiang
type Vector[T, n: Nat] = struct {
    data: [T; n]  # 固定长度数组
}
```

### 3.10 类型联合

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

```yaoxiang
type Number = Int | Float
```

### 3.11 类型交集

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

```yaoxiang
type Printable = struct { to_string: fn() -> String }
type Serializable = struct { to_json: fn() -> String }
type Versatile = Printable & Serializable
```

---

## 第四章：表达式

### 4.1 表达式分类

```
Expr        ::= Literal
              | Identifier
              | FnCall
              | MemberAccess
              | IndexAccess
              | UnaryOp
              | BinaryOp
              | TypeCast
              | IfExpr
              | MatchExpr
              | Block
              | Lambda
```

### 4.2 运算符优先级

| 优先级 | 运算符 | 结合性 |
|--------|--------|--------|
| 1 | `()` `[]` `.` | 左到右 |
| 2 | `as` | 左到右 |
| 3 | `*` `/` `%` | 左到右 |
| 4 | `+` `-` | 左到右 |
| 5 | `<<` `>>` | 左到右 |
| 6 | `&` `\|` `^` | 左到右 |
| 7 | `==` `!=` `<` `>` `<=` `>=` | 左到右 |
| 8 | `not` | 右到左 |
| 9 | `and` `or` | 左到右 |
| 10 | `if...else` | 右到左 |
| 11 | `=` `+=` `-=` `*=` `/=` | 右到左 |

### 4.3 字面量表达式

```yaoxiang
42          # Int
3.14        # Float
true        # Bool
"hello"     # String
'a'         # Char
[1, 2, 3]   # List
{"a": 1}    # Dict
```

### 4.4 标识符表达式

标识符引用变量或函数：

```yaoxiang
x           # 变量引用
my_func     # 函数引用
```

### 4.5 函数调用

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier ':' Expr
```

```yaoxiang
add(1, 2)
greet(name: "Alice", formal: true)
math.sqrt(4.0)
```

### 4.6 成员访问

```
MemberAccess::= Expr '.' Identifier
```

```yaoxiang
point.x
person.name
list.length
```

### 4.7 索引访问

```
IndexAccess ::= Expr '[' Expr ']'
```

```yaoxiang
list[0]
dict["key"]
numbers[-1]
```

### 4.8 算术表达式

```yaoxiang
a = 10 + 5          # 加法
b = 10 - 3          # 减法
c = 4 * 6           # 乘法
d = 15 / 2          # 除法
e = 15 // 2         # 整除
f = 15 % 4          # 取模
```

### 4.9 比较表达式

```yaoxiang
equal = a == b
not_equal = a != b
less = a < b
greater = a > b
less_equal = a <= b
greater_equal = a >= b
```

### 4.10 逻辑表达式

```yaoxiang
and_result = flag1 and flag2
or_result = flag1 or flag2
not_result = not flag1
```

### 4.11 位运算

```yaoxiang
and_result = a & b
or_result = a | b
xor_result = a ^ b
not_result = not a
left_shift = a << 2
right_shift = a >> 1
```

### 4.12 类型转换

```yaoxiang
x = 42 as Float     # Int -> Float
y = 3.14 as Int     # Float -> Int
```

### 4.13 条件表达式

```
IfExpr      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

```yaoxiang
status = if code == 200 {
    "success"
} elif code == 404 {
    "not found"
} else {
    "error"
}
```

### 4.14 模式匹配

```
MatchExpr   ::= 'match' Expr '{' MatchArm+ '}'
MatchArm    ::= Pattern ('|' Pattern)* ('if' Expr)? '->' Expr ','
Pattern     ::= Literal
              | Identifier
              | Wildcard
              | StructPattern
              | TuplePattern
              | UnionPattern
              | OrPattern
```

```yaoxiang
fn classify(x: Int) -> String {
    match x {
        0 -> "zero"
        1 -> "one"
        _ if x < 0 -> "negative"
        _ -> "positive"
    }
}

# 解构
type Point = struct { x: Float, y: Float }
match point {
    struct { x: 0, y: 0 } -> "origin"
    struct { x, y } -> "point"
}
```

### 4.15 块表达式

```
Block       ::= '{' Stmt* Expr? '}'
```

```yaoxiang
result = {
    a = 5
    b = 10
    a + b
}
```

### 4.16 Lambda 表达式

```
Lambda      ::= 'fn' Params? ('->' TypeExpr)? Block
```

```yaoxiang
double = fn(x: Int) -> Int { x * 2 }
numbers.map(fn(x) { x * 2 })
```

---

## 第五章：语句

### 5.1 语句分类

```
Stmt        ::= LetStmt
              | ExprStmt
              | ReturnStmt
              | BreakStmt
              | ContinueStmt
              | IfStmt
              | MatchStmt
              | LoopStmt
              | WhileStmt
              | ForStmt
```

### 5.2 变量声明

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

```yaoxiang
x = 42
count: Int = 100
mut counter = 0
```

### 5.3 表达式语句

任何表达式都可以作为语句：

```yaoxiang
print("Hello")
numbers.append(6)
```

### 5.4 return 语句

```
ReturnStmt  ::= 'return' Expr?
```

```yaoxiang
fn add(a: Int, b: Int) -> Int {
    return a + b
}
```

### 5.5 break 语句

```
BreakStmt   ::= 'break' Identifier?
```

```yaoxiang
for i in 0..10 {
    if i == 5 { break }
}
```

### 5.6 continue 语句

```
ContinueStmt::= 'continue'
```

```yaoxiang
for i in 1..10 {
    if i % 2 == 0 { continue }
    print(i)
}
```

### 5.7 if 语句

```
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

```yaoxiang
if x > 0 {
    print("positive")
} elif x < 0 {
    print("negative")
} else {
    print("zero")
}
```

### 5.8 match 语句

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

```yaoxiang
match value {
    ok: v -> print("Success: " + v)
    err: e -> print("Error: " + e)
}
```

### 5.9 loop 语句（无限循环）

```
LoopStmt    ::= 'loop' Block
```

```yaoxiang
loop {
    input = read_line()
    if input == "quit" { break }
    process(input)
}
```

### 5.10 while 语句

```
WhileStmt   ::= 'while' Expr Block
```

```yaoxiang
mut i = 0
while i < 10 {
    print(i)
    i += 1
}
```

### 5.11 for 语句

```
ForStmt     ::= 'for' Identifier 'in' Expr Block
```

```yaoxiang
for item in [1, 2, 3] {
    print(item)
}

for i in 0..10 {
    print(i)
}

for key, value in {"a": 1, "b": 2} {
    print(key + ": " + value)
}
```

---

## 第六章：函数

### 6.1 函数定义

```
FunctionDef ::= 'fn' Identifier Params? ('->' TypeExpr)? Block
Params      ::= '(' ParamList? ')'
ParamList   ::= Param (',' Param)*
Param       ::= Identifier (':' TypeExpr)?
```

```yaoxiang
fn greet(name: String) -> String {
    "Hello, " + name
}

fn add(a: Int, b: Int) -> Int {
    a + b
}

fn identity[T](x: T) -> T {
    x
}
```

### 6.2 参数类型

```yaoxiang
# 位置参数
fn add(a: Int, b: Int) -> Int { a + b }

# 命名参数
fn create_point(x: Float, y: Float) -> Point {
    Point(x, y)
}
pt = create_point(x: 3.0, y: 4.0)

# 默认参数值
fn greet(name: String, formal: Bool = false) -> String {
    if formal { "Good day, " + name } else { "Hi, " + name }
}

# 可变参数
fn sum(numbers: [Int]) -> Int {
    total = 0
    for n in numbers { total += n }
    total
}
```

### 6.3 泛型函数

```yaoxiang
fn first[T](list: List[T]) -> option[T] {
    if list.length > 0 { some(list[0]) } else { none }
}

fn pair[T, U](a: T, b: U) -> (T, U) {
    (a, b)
}
```

### 6.4 类型约束

```yaoxiang
fn max[T: Comparable](a: T, b: T) -> T {
    if a > b { a } else { b }
}

fn process_items[T](items: [T]) where T: Printable {
    for item in items { print(item) }
}
```

### 6.5 高阶函数

```yaoxiang
fn apply[T, U](value: T, f: fn(T) -> U) -> U {
    f(value)
}

fn create_multiplier(factor: Int) -> fn(Int) -> Int {
    fn(x: Int) -> Int { x * factor }
}
```

### 6.6 闭包

```yaoxiang
fn create_counter() -> fn() -> Int {
    mut count = 0
    fn() -> Int {
        count += 1
        count
    }
}
```

### 6.7 spawn 函数

```
SpawnFn     ::= 'fn' Identifier Params? '->' TypeExpr 'spawn' Block
```

```yaoxiang
fn fetch_data(url: String) -> JSON spawn {
    HTTP.get(url).json()
}

fn main() {
    data = fetch_data("https://api.example.com")
    # 自动等待
    print(data)
}
```

---

## 第七章：模块

### 7.1 模块定义

```
ModuleDef   ::= 'mod' Identifier '{' ModuleBody '}'
ModuleBody  ::= (Import | FnDef | TypeDef | VarDecl)*
```

```yaoxiang
mod Math {
    pub fn sqrt(x: Float) -> Float { ... }
    pub pi = 3.14159

    fn internal_helper() { ... }
}
```

### 7.2 模块导入

```
Import      ::= 'use' ModuleRef ('as' Identifier)?
              | 'use' ModuleRef '{' ImportItems '}'
ImportItems ::= ImportItem (',' ImportItem)* ','?
ImportItem  ::= Identifier ('as' Identifier)?
```

```yaoxiang
use std.io
use std.io as IO
use std.io.{ read_file, write_file, File }
use std.list as ListLib
```

### 7.3 重新导出

```yaoxiang
mod MyMath {
    pub use std.math.{ sin, cos }
}
```

### 7.4 可见性

```yaoxiang
mod MyModule {
    pub fn public_function() { ... }
    fn private_function() { ... }

    pub type PublicType = struct { ... }
    type PrivateType = struct { ... }
}
```

---

## 第八章：内存管理

### 8.1 所有权

每个值有唯一的所有者。当所有者离开作用域时，值被自动销毁。

### 8.2 引用类型

```yaoxiang
fn process(data: ref Data) {     # 不可变引用
    print(data.field)
}

fn modify(data: mut Data) {      # 可变引用
    data.field = new_value
}
```

### 8.3 生命周期

```yaoxiang
# 显式生命周期
fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    if s1.length > s2.length { s1 } else { s2 }
}
```

### 8.4 智能指针

```yaoxiang
# Box - 堆分配
heap_data: Box[List[Int]] = Box.new([1, 2, 3])

# Rc - 引用计数
shared: Rc[Data] = Rc.new(data)

# Arc - 原子引用计数
thread_safe: Arc[Data] = Arc.new(data)
```

---

## 第九章：错误处理

### 9.1 Result 类型

```yaoxiang
type Result[T, E] = union {
    ok: T
    err: E
}

fn divide(a: Float, b: Float) -> Result[Float, String] {
    if b == 0.0 {
        err("Division by zero")
    } else {
        ok(a / b)
    }
}

# 使用
result = divide(10.0, 2.0)
match result {
    ok: value -> print("Result: " + value)
    err: msg -> print("Error: " + msg)
}
```

### 9.2 Option 类型

```yaoxiang
type Option[T] = union {
    some: T
    none
}

fn find_user(id: Int) -> Option[User] {
    if id > 0 { some(User(id)) } else { none }
}
```

### 9.3 ? 运算符

```yaoxiang
fn process() -> Result[Int, String] {
    a = read_number()?
    b = read_number()?
    c = divide(a, b)?
    ok(c * 2)
}
```

---

## 附录A：语法速查

### A.1 类型定义

```
TypeDef ::= 'type' Identifier '=' TypeExpr
```

### A.2 函数定义

```
FunctionDef ::= 'fn' Identifier Params? ('->' TypeExpr)? Block
```

### A.3 模块

```
ModuleDef ::= 'mod' Identifier '{' ... '}'
Import ::= 'use' ModuleRef
```

### A.4 控制流

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for Identifier in Expr Block
loop Block
```

---

> 本规范定义了 YaoXiang 编程语言的核心语法和语义。
