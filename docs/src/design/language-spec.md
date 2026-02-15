# YaoXiang（爻象）编程语言规范

> 版本：v1.7.0
> 状态：规范
> 作者：晨煦
> 日期：2024-12-31
> 更新：2026-02-13 - RFC-010 统一类型语法：`Name: Type = value` 替换 `type Name = ...`

---

## 第一章：引言

### 1.1 范围

本文档定义了 YaoXiang 编程语言的语法和语义。它是语言的权威参考，面向编译器和工具实现者。

教程和示例代码请参考 [YaoXiang 指南](../guides/YaoXiang-book.md) 和 [tutorial/](../tutorial/) 目录。

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
| 关键字 | 语言预定义保留词 | `Type`, `pub`, `use` |
| 字面量 | 固定值 | `42`, `"hello"`, `true` |
| 运算符 | 运算符号 | `+`, `-`, `*`, `/` |
| 分隔符 | 语法分隔符 | `(`, `)`, `{`, `}`, `,` |

### 2.3 关键字

YaoXiang 定义了极少量的关键字：

```
Type   pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

**注意**：`Type` 是语言中唯一的元类型关键字（大写）。所有类型定义都使用统一语法 `Name: Type = ...`。

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

#### 2.6.1 整数

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 2.6.2 浮点数

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 2.6.3 字符串

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 2.6.4 集合

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Set         ::= '{' Expr (',' Expr)* '}'
```

#### 2.6.5 列表推导式

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

#### 2.6.6 成员检测

```
Membership  ::= Expr 'in' Expr
```

### 2.7 注释

```
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
              | EnumType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
              | ConstrainedType
              | AssociatedType
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

### 3.3 记录类型

**统一语法**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 # 接口约束
```

```yaoxiang
# 简单记录类型
Point: Type = { x: Float, y: Float }

# 空记录类型
Empty: Type = {}

# 带泛型的记录类型
Pair: Type[T] = { first: T, second: T }

# 实现接口的记录类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**规则**：
- 记录类型使用花括号 `{}` 定义
- 字段名后直接跟冒号和类型
- 接口名写在类型体内表示实现该接口

### 3.4 枚举类型（变体类型）

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**语法**：`Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
# 无参变体
Color: Type = { red | green | blue }

# 有参变体
Option: Type[T] = { some(T) | none }

# 混合
Result: Type[T, E] = { ok(T) | err(E) }

# 无参变体等价于无参构造器
Bool: Type = { true | false }
```

### 3.5 接口类型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**语法**：接口是字段全为函数类型的记录类型

```yaoxiang
# 接口定义
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

# 空接口
EmptyInterface: Type = {}
```

**接口实现**：类型通过在定义末尾列出接口名来实现接口

```yaoxiang
# 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        # 实现 Drawable 接口
    Serializable     # 实现 Serializable 接口
}
```

### 3.6 元组类型

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.7 函数类型

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

### 3.8 泛型类型

#### 3.8.1 泛型参数语法

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

#### 3.8.2 泛型类型定义

```yaoxiang
# 基础泛型类型
Option: Type[T] = {
    some: (T) -> Self,
    none: () -> Self
}

Result: Type[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: Type[T] = {
    data: Array[T],
    length: Int,
    push: [T](self: List[T], item: T) -> Void,
    get: [T](self: List[T], index: Int) -> Option[T]
}
```

#### 3.8.3 类型推导

```yaoxiang
# 编译器自动推导泛型参数
numbers: List[Int] = List(1, 2, 3)  # 编译器推导 List[Int]
```

### 3.9 类型约束

#### 3.9.1 单一约束

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
# 接口类型定义（作为约束）
Clone: Type = {
    clone: (Self) -> Self
}

# 使用约束
clone: [T: Clone](value: T) -> T = value.clone()
```

#### 3.9.2 多重约束

```yaoxiang
# 多重约束语法
combine: [T: Clone + Add](a: T, b: T) -> T = {
    a.clone() + b
}

# 泛型容器的排序
sort: [T: Clone + PartialOrd](list: List[T]) -> List[T] = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

#### 3.9.3 函数类型约束

```yaoxiang
# 高阶函数约束
call_twice: [T, F: Fn() -> T](f: F) -> (T, T) = (f(), f())

compose: [A, B, C, F: Fn(A) -> B, G: Fn(B) -> C](a: A, f: F, g: G) -> C = g(f(a))
```

### 3.10 关联类型

#### 3.10.1 关联类型定义

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
# Iterator trait（使用记录类型语法）
Iterator: Type[T] = {
    Item: T,                    # 关联类型
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool
}

# 使用关联类型
collect: [T, I: Iterator[T]](iter: I) -> List[T] = {
    result = List[T]()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}
```

#### 3.10.2 泛型关联类型（GAT）

```yaoxiang
# 更复杂的关联类型
Container: Type[T] = {
    Item: T,
    IteratorType: Iterator[T],  # 关联类型也是泛型的
    iter: (Self) -> IteratorType
}
```

### 3.11 编译期泛型

#### 3.11.1 字面量类型约束

```
LiteralType   ::= Identifier ':' Int          # 编译期常量
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**核心设计**：用 `[n: Int]` 泛型参数 + `(n: n)` 值参数，区分编译期常量与运行时值。

```yaoxiang
# 编译期阶乘：参数必须是编译期已知的字面量
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# 编译期常量数组
StaticArray: Type[T, N: Int] = {
    data: T[N],      # 编译期已知大小的数组
    length: N
}

# 使用方式
arr: StaticArray[Int, factorial(5)]  # 编译器在编译期计算 factorial(5) = 120
```

#### 3.11.2 编译期常量数组

```yaoxiang
# 矩阵类型使用
Matrix: Type[T, Rows: Int, Cols: Int] = {
    data: Array[Array[T, Cols], Rows]
}

# 编译期维度验证
identity_matrix: [T: Add + Zero + One, N: Int](size: N) -> Matrix[T, N, N] = {
    # ...
}
```

### 3.12 条件类型

#### 3.12.1 If 条件类型

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
# 类型级 If
If: Type[C: Bool, T, E] = match C {
    True => T,
    False => E
}

# 示例：编译期分支
NonEmpty: Type[T] = If[T != Void, T, Never]

# 编译期验证
Assert: Type[C: Bool] = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

#### 3.12.2 类型族

```yaoxiang
# 编译期类型转换
AsString: Type[T] = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 3.13 类型联合

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 3.14 类型交集

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**语法**：类型交集 `A & B` 表示同时满足 A 和 B 的类型

```yaoxiang
# 接口组合 = 类型交集
DrawableSerializable: Type = Drawable & Serializable

# 使用交集类型
process: [T: Drawable & Serializable](item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

### 3.15 函数重载与特化

```yaoxiang
# 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array[Int]) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# 通用实现
sum: [T: Add](arr: Array[T]) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

### 3.16 平台特化

```yaoxiang
# 平台类型枚举（标准库定义）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

# P 是预定义泛型参数名，代表当前编译平台
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第三章（续）：语法设计说明

### 3.17 具名函数与 Lambda 的关系

**核心理解**：具名函数和 Lambda 表达式是同一个东西！唯一的区别是：具名函数给 Lambda 取了个名字。

```yaoxiang
# 这两者本质完全相同
add: (a: Int, b: Int) -> Int = a + b           # 具名函数（推荐）
add: (a: Int, b: Int) -> Int = (a, b) => a + b  # Lambda 形式（完全等价）
```

**语法糖模型**：

```
# 具名函数 = Lambda + 名字
name: (Params) -> ReturnType = body

# 本质上是
name: (Params) -> ReturnType = (params) => body
```

**关键点**：当签名完整声明了参数类型，Lambda 头部的参数名就变成了冗余，可以省略。

### 3.18 参数作用域规则

**参数覆盖外层变量**：签名中的参数作用域覆盖函数体，内部作用域优先级更高。

```yaoxiang
x = 10  # 外层变量
double: (x: Int) -> Int = x * 2  # ✅ 参数 x 覆盖外层 x，结果为 20
```

### 3.19 类型标注位置

类型标注可以在以下任一位置，**至少标注一处即可**：

| 标注位置 | 形式 | 说明 |
|----------|------|------|
| 仅签名 | `double: (x: Int) -> Int = x * 2` | ✅ 推荐 |
| 仅 Lambda 头 | `double = (x: Int) => x * 2` | ✅ 合法 |
| 两边都标 | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗余但允许 |

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

### 4.3 函数调用

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier ':' Expr
```

### 4.4 成员访问

```
MemberAccess::= Expr '.' Identifier
```

### 4.5 索引访问

```
IndexAccess ::= Expr '[' Expr ']'
```

### 4.6 类型转换

```
TypeCast    ::= Expr 'as' TypeExpr
```

### 4.7 条件表达式

```
IfExpr      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 4.8 模式匹配

```
MatchExpr   ::= 'match' Expr '{' MatchArm+ '}'
MatchArm    ::= Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
Pattern     ::= Literal
              | Identifier
              | Wildcard
              | StructPattern
              | TuplePattern
              | EnumPattern
              | OrPattern
```

### 4.9 块表达式

```
Block       ::= '{' Stmt* Expr? '}'
```

### 4.10 Lambda 表达式

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
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

### 5.3 return 语句

```
ReturnStmt  ::= 'return' Expr?
```

### 5.4 break 语句

```
BreakStmt   ::= 'break' Identifier?
```

### 5.5 continue 语句

```
ContinueStmt::= 'continue'
```

### 5.6 if 语句

```
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 5.7 match 语句

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 5.8 loop 语句

```
LoopStmt    ::= 'loop' Block
```

### 5.9 while 语句

```
WhileStmt   ::= 'while' Expr Block
```

### 5.10 for 语句

```
ForStmt     ::= 'for' Identifier 'in' Expr Block
```

---

## 第六章：函数

### 6.1 统一函数模型

**核心语法**：`name: type = value`

YaoXiang 采用**统一声明模型**：变量、函数、方法都使用相同的形式 `name: type = value`。

```
Declaration   ::= Identifier ':' Type '=' Expression
FunctionDef   ::= Identifier GenericParams? '(' Parameters? ')' '->' Type '=' (Expression | Block)
GenericParams ::= '[' Identifier (',' Identifier)* ']'
Parameters    ::= Parameter (',' Parameter)*
Parameter     ::= Identifier ':' TypeExpr
```

### 6.2 变量声明

```yaoxiang
# 基本语法
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

# 类型推导
y = 100  # 推断为 Int
```

### 6.3 函数定义

#### 6.3.1 完整语法

```yaoxiang
# 参数名在签名中声明
add: (a: Int, b: Int) -> Int = {
    return a + b
}

# 单参数
inc: (x: Int) -> Int = x + 1

# 无参函数
main: () -> Void = {
    print("Hello")
}

# 多行函数体
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" => x + y,
        "-" => x - y,
        _ => 0.0
    }
}
```

#### 6.3.2 返回规则

```yaoxiang
# 非 Void 返回类型 - 必须使用 return
add: (a: Int, b: Int) -> Int = {
    return a + b
}

# Void 返回类型 - 可选使用 return
print: (msg: String) -> Void = {
    # 不需要 return
}

# 单行表达式 - 直接返回值，无需 return
greet: (name: String) -> String = "Hello, ${name}!"
```

### 6.4 泛型函数

```yaoxiang
# 泛型函数定义
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = {
    result = List[R]()
    for item in list {
        result.push(f(item))
    }
    return result
}

# 使用泛型约束
clone: [T: Clone](value: T) -> T = value.clone()

# 多类型参数
combine: [T, U](a: T, b: U) -> (T, U) = (a, b)
```

### 6.5 方法定义

#### 6.5.1 类型方法

**语法**：`Type.method: (self: Type, ...) -> Return = ...`

```yaoxiang
# 类型方法：关联到特定类型
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

# 使用方法语法糖
p: Point = Point(1.0, 2.0)
p.draw(screen)           # 语法糖 → Point.draw(p, screen)
```

#### 6.5.2 普通方法

**语法**：`name: (Type, ...) -> Return = ...`（不关联类型）

```yaoxiang
# 普通方法：不关联类型，作为独立函数
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}
```

### 6.6 方法绑定

#### 6.6.1 手动绑定

**语法**：`Type.method = function[positions]`

```yaoxiang
# 绑定到第 0 位（默认）
Point.distance = distance[0]

# 绑定到第 1 位
Point.transform = transform[1]

# 多位置绑定
Point.scale = scale[0, 1]

# 使用占位符
Point.calc = func[0, _, 2]
```

#### 6.6.2 pub 自动绑定

使用 `pub` 声明的函数，编译器自动绑定到同文件定义的类型：

```yaoxiang
# 使用 pub 声明，编译器自动绑定
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 编译器自动推断：
# 1. Point 在当前文件定义
# 2. 函数参数包含 Point
# 3. 执行 Point.distance = distance[0]

# 调用
d = distance(p1, p2)           # 函数式
d2 = p1.distance(p2)           # OOP 语法糖
```

### 6.7 方法绑定规则

| 规则 | 说明 |
|------|------|
| 位置从 0 开始 | `func[0]` 绑定第 1 个参数（索引 0） |
| 最大位置 | 必须 < 函数参数个数 |
| 负数索引 | `[-1]` 表示最后一个参数 |
| 占位符 | `_` 跳过该位置，由用户提供 |

### 6.8 柯里化支持

绑定天然支持柯里化。当调用时提供的参数少于剩余参数时，返回一个接受剩余参数的函数：

```yaoxiang
# 原始函数：5 个参数
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

# 绑定：Point.calc = calculate[1, 2]
# 绑定后剩余参数：scale, x, y

# 调用场景
p1.calc(2.0, 10.0, 20.0)       # 提供 3 个参数 → 直接调用
p1.calc(2.0)                    # 提供 1 个参数 → 返回 (Float, Float) -> Float
p1.calc()                       # 提供 0 个参数 → 返回 (Float, Float, Float) -> Float
```

### 6.9 并作函数与注解

#### 6.9.1 spawn 函数（并作函数）

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**函数注解**：

| 注解 | 位置 | 行为 |
|------|------|------|
| `@block` | 返回类型后 | 禁用并发优化，完全顺序执行 |
| `@eager` | 返回类型后 | 强制急切求值 |

**语法示例**：

```
# 并作函数：可并发执行
fetch_data: (url: String) -> JSON spawn = { ... }

# @block 同步函数：完全顺序执行
main: () -> Void @block = { ... }

# @eager 急切函数：立即执行
compute: (n: Int) -> Int @eager = { ... }
```

#### 6.9.2 spawn 块

显式声明的并发疆域，块内任务将并作执行：

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**示例**：

```
# 并作块：显式并发
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

#### 6.9.3 spawn 循环

数据并行循环，循环体在所有数据元素上并作执行：

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**示例**：

```
# 并作循环：数据并行
results = spawn for item in items {
    process(item)
}
```

#### 6.9.4 错误传播运算符

```
ErrorPropagate ::= Expr '?'
```

**示例**：

```
process: (p: Point) -> Result[Data, Error] = {
    data = fetch_data()?      # 自动传播错误
    transform(data)?
}
```

---

## 第七章：模块

### 7.1 模块定义

模块使用文件作为边界。每个 `.yx` 文件就是一个模块。

```
# 文件名即为模块名
# Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 7.2 模块导入

```
Import      ::= 'use' ModuleRef ('as' Identifier)?
              | 'use' ModuleRef '{' ImportItems '}'
ImportItems ::= ImportItem (',' ImportItem)* ','?
ImportItem  ::= Identifier ('as' Identifier)?
```

---

## 第八章：内存管理

### 8.1 所有权模型

YaoXiang 采用**所有权模型**管理内存，每个值有唯一的所有者：

| 语义 | 说明 | 语法 |
|------|------|------|
| **Move** | 默认语义，所有权转移 | `p2 = p` |
| **ref** | 共享（Arc 引用计数） | `shared = ref p` |
| **clone()** | 显式复制 | `p2 = p.clone()` |

### 8.2 Move 语义（默认）

```yaoxiang
# 赋值 = Move（零拷贝）
p: Point = Point(1.0, 2.0)
p2 = p              # Move，p 失效

# 函数传参 = Move
process: (p: Point) -> Void = {
    # p 的所有权转移进来
}

# 返回值 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move，所有权转移
}
```

### 8.3 ref 关键字（Arc）

`ref` 关键字创建**引用计数指针**（Arc），用于安全共享：

```yaoxiang
# 创建 Arc
p: Point = Point(1.0, 2.0)
shared = ref p      # Arc，线程安全

# 共享访问
spawn(() => print(shared.x))   # ✅ 安全

# Arc 自动管理生命周期
# shared 离开作用域时，计数归零自动释放
```

**特点**：
- 线程安全引用计数
- 自动管理生命周期
- 跨 spawn 边界安全

### 8.4 clone() 显式复制

```yaoxiang
# 显式复制值
p: Point = Point(1.0, 2.0)
p2 = p.clone()      # p 和 p2 独立

# 两者都可修改，互不影响
p.x = 0.0           # ✅
p2.x = 0.0          # ✅
```

### 8.5 unsafe 代码块

`unsafe` 代码块允许使用裸指针，用于系统级编程：

```yaoxiang
# 裸指针类型
PtrType ::= '*' TypeExpr

# unsafe 代码块
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'
```

**示例**：

```yaoxiang
p: Point = Point(1.0, 2.0)

# 裸指针只能在 unsafe 块中使用
unsafe {
    ptr: *Point = &p     # 获取裸指针
    (*ptr).x = 0.0       # 解引用
}
```

**限制**：
- 裸指针只能在 `unsafe` 块中使用
- 用户保证不悬空、不释放后使用
- 不参与 Send/Sync 检查

### 8.7 所有权语法 BNF

```bnf
# === 所有权表达式 ===

# Move（默认）
MoveExpr     ::= Expr

# ref Arc
RefExpr      ::= 'ref' Expr

# clone
CloneExpr    ::= Expr '.clone' '(' ')'

# === 裸指针（仅 unsafe） ===

PtrType       ::= '*' TypeExpr
UnsafeBlock   ::= 'unsafe' '{' Stmt* '}'
```

### 8.8 Send / Sync 约束

| 约束 | 语义 | 说明 |
|------|------|------|
| **Send** | 可安全跨线程传输 | 值可以移动到另一个线程 |
| **Sync** | 可安全跨线程共享 | 不可变引用可以共享到另一个线程 |

**自动派生**：

```
# Send 派生规则
Struct[T1, T2]: Send ⇐ T1: Send 且 T2: Send

# Sync 派生规则
Struct[T1, T2]: Sync ⇐ T1: Sync 且 T2: Sync
```

**类型约束**：

| 类型 | Send | Sync | 说明 |
|------|------|------|------|
| `T`（值） | ✅ | ✅ | 不可变数据 |
| `ref T` | ✅ | ✅ | Arc 线程安全 |
| `*T` | ❌ | ❌ | 裸指针不安全 |

---

## 第八章（续）：类型系统约束

### 8.7 Send/Sync 约束

YaoXiang 使用 Rust 风格的类型约束来保证并发安全：

| 约束 | 语义 | 说明 |
|------|------|------|
| **Send** | 可安全跨线程传输 | 值可以移动到另一个线程 |
| **Sync** | 可安全跨线程共享 | 不可变引用可以共享到另一个线程 |

**约束层次**：

```
Send ──► 可安全跨线程传输
  │
  └──► Sync ──► 可安全跨线程共享
       │
       └──► 满足 Send + Sync 的类型可自动并发

Arc[T] 实现 Send + Sync（线程安全引用计数）
Mutex[T] 提供内部可变性
```

### 8.8 并发安全类型

| 类型 | 语义 | 并发安全 | 说明 |
|------|------|----------|------|
| `T` | 不可变数据 | ✅ 安全 | 默认类型，多任务读取无竞争 |
| `Ref[T]` | 可变引用 | ⚠️ 需同步 | 标记为可并发修改，编译检查锁使用 |
| `Atomic[T]` | 原子类型 | ✅ 安全 | 底层原子操作，无锁并发 |
| `Mutex[T]` | 互斥锁包装 | ✅ 安全 | 自动加锁解锁，编译保证 |
| `RwLock[T]` | 读写锁包装 | ✅ 安全 | 读多写少场景优化 |

**语法**：

```
Mutex[T]    # 互斥锁包装的可变数据
Atomic[T]   # 原子类型（仅限 Int、Float 等）
RwLock[T]   # 读写锁包装
```

**with 语法糖**：

```
with mutex.lock() {
    # 临界区：受 Mutex 保护
    ...
}
```

---

## 第九章：错误处理

### 9.1 Result 类型

```
Result: Type[T, E] = ok(T) | err(E)
```

**变体构造**：

| 变体 | 语法 | 说明 |
|------|------|------|
| `ok(T)` | `ok(value)` | 成功值 |
| `err(E)` | `err(error)` | 错误值 |

### 9.2 Option 类型

```
Option: Type[T] = some(T) | none
```

**变体构造**：

| 变体 | 语法 | 说明 |
|------|------|------|
| `some(T)` | `some(value)` | 有值 |
| `none` | `none` | 无值 |

### 9.3 错误传播

```
ErrorPropagate ::= Expr '?'
```

`?` 运算符自动传播 Result 类型的错误：

```
# 成功时返回值，失败时向上返回 err
data = fetch_data()?

# 等价于
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

---

## 附录A：语法速查

### A.1 类型定义

```
# === 记录类型（花括号） ===

# 结构体
Point: Type = { x: Float, y: Float }

# 枚举（变体类型）
Result: Type[T, E] = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

# === 接口类型（花括号，字段全为函数） ===

# 接口定义
Serializable: Type = { serialize: () -> String }

# 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    # 实现 Serializable 接口
}

# === 函数类型 ===

Adder: Type = (Int, Int) -> Int
```

### A.2 函数定义

```
# 形式一：类型集中式（推荐）
name: (param1: Type1, param2: Type2) -> ReturnType = body

# 形式二：简写式（参数名省略）
name: (Type1, Type2) -> ReturnType = (params) => body

# 泛型函数
name: [T, R](param: T) -> R = body

# 泛型约束
name: [T: Clone + Add](a: T, b: T) -> T = body
```

### A.3 方法定义

```
# 类型方法
Type.method: (self: Type, ...) -> ReturnType = { ... }

# 普通方法
name: (Type, ...) -> ReturnType = { ... }
```

### A.4 方法绑定

```
# 单位置绑定
Type.method = func[0]

# 多位置绑定
Type.method = func[0, 1]

# pub 自动绑定
pub name: (Type, ...) -> ReturnType = { ... }  # 自动绑定到 Type
```

### A.5 泛型语法

```
# 泛型类型
List: Type[T] = { data: Array[T], length: Int }
Result: Type[T, E] = { ok(T) | err(E) }

# 泛型函数
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = { ... }

# 类型约束
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = body

# 关联类型
Iterator: Type[T] = { Item: T, next: () -> Option[T] }

# 编译期泛型
factorial: [n: Int](n: n) -> Int = { ... }
StaticArray: Type[T, N: Int] = { data: T[N], length: N }

# 条件类型
If: Type[C: Bool, T, E] = match C { True => T, False => E }

# 函数特化
sum: (arr: Array[Int]) -> Int = { ... }
sum: (arr: Array[Float]) -> Float = { ... }
```

### A.6 模块

```
# 模块即文件
# 文件名.yx 即为模块名
Import ::= 'use' ModuleRef
```

### A.7 控制流

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Identifier in Expr Block Expr Block
for
```

### A.8 match 语法

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```

---

## 附录B：与代码实现差异说明

> 本节说明语言规范与当前代码实现之间的已知差异。

### B.1 关键字

| 关键字 | 规范状态 | 代码实现 | 说明 |
|--------|---------|---------|------|
| `struct` | 已移除 | ❌ 无 | 使用统一语法 `Name: Type = {...}` |
| `enum` | 已移除 | ❌ 无 | 使用变体语法 `Name: Type = { A \| B \| C }` |
| `type` | 已移除 | ❌ 无 | 使用 `Type`（大写）作为元类型关键字 |

### B.2 语法差异

| 语法元素 | 规范 | 代码实现 | 说明 |
|---------|------|---------|------|
| match arm 分隔符 | `->` | `=>` | 使用 `=>` (FatArrow) |
| 函数定义 | `name(types) -> type = (params) => body` | 两种形式 | 支持类型集中式 `name: Type = (params) =>` |
| 接口类型 | `type Serializable = [ serialize() -> String ]` | ❌ 未实现 | 方括号语法待实现 |

### B.3 待实现特性

以下规范中描述的特性尚未在代码中实现：

| 特性 | 优先级 | 说明 |
|------|--------|------|
| 统一类型语法 `Name: Type = {...}` | P0 | RFC-010：统一语法替换 `type Name = ...` |
| 花括号类型语法 | P0 | `Point: Type = { x: Float, y: Float }` |
| 接口类型 | P1 | `Drawable: Type = { draw() -> Void }` |
| 列表推导式 | P2 | `[x for x in list if condition]` |
| `?` 错误传播 | P1 | Result 类型自动错误传播 |
| `ref` 关键字 | P1 | Arc 引用计数共享 |
| `unsafe` 代码块 | P1 | 裸指针和系统级编程 |
| `*T` 裸指针类型 | P1 | 裸指针类型语法 |
| `clone()` 语义 | P1 | 显式复制 |
| `@block` 注解 | P1 | 同步执行保证 |
| `spawn` 函数 | P1 | 并作函数标记 |
| `spawn {}` 块 | P1 | 显式并发疆域 |
| `spawn for` 循环 | P1 | 数据并行循环 |
| Send/Sync 约束 | P2 | 并发安全类型检查 |
| Mutex/Atomic 类型 | P2 | 并发安全数据类型 |
| 错误图可视化 | P3 | 并发错误传播追踪 |
| **泛型类型系统** | P1 | RFC-011 |
| 基础泛型 `[T]` | P1 | 泛型类型参数和单态化 |
| 类型约束 `[T: Clone]` | P2 | 单一/多重约束系统 |
| 关联类型 `Item: T` | P3 | GAT 支持 |
| 编译期泛型 `[N: Int]` | P3 | 字面量类型约束 |
| 条件类型 `If[C, T, E]` | P3 | 类型级计算 |
| 函数重载特化 | P2 | 平台特化与类型特化 |
| 方法语法 `Type.method` | P1 | `Point.draw: (...) -> ... = ...` |

### B.4 不实现特性

以下 Rust 风格特性**不会实现**：

| 特性 | 原因 |
|------|------|
| 生命周期 `'a` | 无引用概念，无需生命周期 |
| 借用检查器 | ref = Arc 替代 |
| `&T` 借用语法 | 使用 Move 语义 |
| `&mut T` 可变借用 | 使用 mut + Move |

---

## 第十章：方法绑定

### 10.1 绑定概述

YaoXiang 采用**纯函数式设计**，所有操作都通过函数实现。绑定机制将函数关联到类型，使调用者可以像调用方法一样调用函数。

```
绑定声明 ::= 类型 '.' 标识符 '=' 函数名 '[' 位置列表 ']'
位置列表 ::= 位置 (',' 位置)* ','?
位置     ::= 整数（从 0 开始） | 负整数 | 占位符
```

**核心规则**：
- 位置索引从 **0** 开始
- 默认绑定到第 **0** 位（首个参数）
- 支持负数索引 `[-1]` 表示最后一个参数
- 多位置联合绑定 `[0, 1, 2]`
- 占位符 `_` 表示跳过该位置

### 10.2 绑定语法

**语法**：
```
Type.method = func[position]
Type.method = func[0, 1, 2]    # 多位置绑定
Type.method = func[0, _, 2]   # 使用占位符
Type.method = func[-1]        # 负数索引（最后一个参数）
```

**语义**：
- `Type.method = func[0]` 表示调用 `obj.method(arg)` 时，`obj` 绑定到 `func` 的第 0 个参数
- 剩余参数按原顺序填入

### 10.3 绑定示例

```yaoxiang
# === 基础绑定 ===

# 原始函数
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# 绑定到 Point 类型（第 0 位）
Point.distance = distance[0]

# 使用
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)
d = p1.distance(p2)  # → distance(p1, p2)

# === 多位置绑定 ===

# 原始函数
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

# 绑定多个位置
Point.calc_scale = calculate[0]      # 只绑定 scale
Point.calc_both = calculate[1, 2]    # 绑定两个 Point 参数

# 使用
f = p1.calc_scale(2.0)  # → calculate(2.0, p1, _, _, _)
result = f(p2, 10.0, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)

# === 柯里化（参数不足时自动返回函数）===

# 绑定一个参数
Point.distance_to = distance[0]

# 使用 - 不提供第二个参数，返回柯里化函数
f = p1.distance_to(p2)  # → distance(p1, p2) 直接调用
f2 = p1.distance_to()   # → distance(p1, _) 返回函数 (Point) -> Float
result = f2(p2)         # → distance(p1, p2)
```

### 10.4 绑定规则

**位置规则**：
| 规则 | 说明 |
|------|------|
| 位置从 0 开始 | `func[0]` 绑定第 1 个参数（索引 0） |
| 最大位置 | 必须 < 函数参数个数 |
| 负数索引 | `[-1]` 表示最后一个参数 |
| 占位符 | `_` 跳过该位置，由用户提供 |

**类型检查**：
```yaoxiang
# ✅ 合法绑定
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(Float, Point, Point, ...)

# ❌ 非法绑定（编译错误）
Point.wrong = distance[5]             # 5 >= 2（参数个数）
Point.wrong = distance[0, 0]          # 重复位置（如果不允许）
Point.wrong = distance[-2]            # -2 超出范围
```

### 10.5 自动绑定

对于在模块中定义且首参数为模块类型的函数，自动支持方法调用语法：

```yaoxiang
# === Point.yx ===
Point: Type = { x: Float, y: Float }

# 首参数是 Point，自动支持方法调用
distance: (a: Point, b: Point) -> Float = { ... }
add: (a: Point, b: Point) -> Point = { ... }

# === main.yx ===
use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# ✅ 自动绑定：p1.distance(p2) → distance(p1, p2)
d = p1.distance(p2)
# ✅ p1.add(p2) → add(p1, p2)
p3 = p1.add(p2)
```

**自动绑定规则**：
- 函数定义在模块文件中
- 函数的第 0 个参数类型与模块名匹配
- 函数必须为 `pub` 才能在模块外自动绑定

### 10.6 绑定与柯里化的关系

绑定天然支持柯里化。当调用时提供的参数少于剩余参数时，返回一个接受剩余参数的函数：

```yaoxiang
# 原始函数：5 个参数
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

# 绑定：Point.calc = calculate[1, 2]
# 绑定后剩余参数：scale, x, y

# 调用场景
p1.calc(2.0, 10.0, 20.0)              # 提供 3 个参数 → 直接调用
p1.calc(2.0)                          # 提供 1 个参数 → 返回 (Float, Float) -> Float
p1.calc()                             # 提供 0 个参数 → 返回 (Float, Float, Float) -> Float
```

---

## 附录C：绑定语法速查

### C.1 绑定声明

```
# 单位置绑定（默认绑定到第 0 位）
Type.method = func[0]

# 多位置绑定
Type.method = func[0, 1, 2]

# 使用占位符
Type.method = func[0, _, 2]

# 负数索引（最后一个参数）
Type.method = func[-1]
```

### C.2 位置索引说明

```
函数参数：    (p0, p1, p2, p3, p4)
              ↑  ↑  ↑  ↑  ↑
索引：        0  1  2  3  4

# 绑定 [1, 3]
Type.method = func[1, 3]
# 调用：obj.method(p0, p2, p4)
# 映射：func(p0_bound, obj, p2, p3_bound, p4)
```

### C.3 调用形式

```yaoxiang
# 直接调用（提供所有剩余参数）
result = p.method(arg1, arg2, arg3)

# 柯里化（不提供或部分提供剩余参数）
f = p.method(arg1)          # 返回接受剩余参数的函数
result = f(arg2, arg3)
```

---

## 版本历史

| 版本 | 日期 | 作者 | 变更说明 |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初始版本 |
| v1.1.0 | 2025-01-04 | 沫郁酱 | 修正 match arm 使用 `=>` 而非 `->`；更新函数定义语法；更新类型定义语法；添加与代码实现差异说明 |
| v1.2.0 | 2025-01-05 | 沫郁酱 | 精简为纯规范，示例代码移至 tutorial/ 目录 |
| v1.3.0 | 2025-01-05 | 沫郁酱 | 添加并作模型规范（三层并发架构、spawn语法、注解）；添加类型系统约束（Send/Sync）；添加并发安全类型（Mutex、Atomic）；更新错误处理（?运算符）；更新待实现特性列表 |
| v1.4.0 | 2025-01-15 | 晨煦 | 更新所有权模型（默认Move + 显式ref=Arc）；添加unsafe关键字；删除生命周期 `'a` 和借用检查器；更新待实现特性列表 |
| v1.5.0 | 2025-01-20 | 晨煦 | 添加方法绑定规范（RFC-004）：位置索引从 0 开始；默认绑定到第 0 位；支持负数索引和多位置绑定 |
| v1.6.0 | 2025-02-06 | 晨煦 | 整合 RFC-010（统一类型语法）：更新 `type Name = {...}` 语法、参数名在签名中的函数定义、Type.method 方法语法；整合 RFC-011（泛型系统）：添加泛型类型 `[T]`、类型约束 `[T: Clone]`、关联类型 `Item: T`、编译期泛型 `[N: Int]`、条件类型 `If[C, T, E]`、函数重载特化、平台特化 |
| v1.7.0 | 2026-02-13 | 晨煦 | RFC-010 更新：`Name: Type = {...}` 替换 `type Name = {...}`；仅 `Type`（大写）为元类型关键字；所有声明使用统一语法 |

---

> 本规范定义了 YaoXiang 编程语言的核心语法和语义。
> 教程和示例代码请参考 [YaoXiang 指南](../guides/YaoXiang-book.md) 和 [tutorial/](../tutorial/) 目录。
