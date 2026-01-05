# YaoXiang（爻象）编程语言规范

> 版本：v1.2.0
> 状态：规范
> 作者：晨煦
> 日期：2024-12-31
> 更新：2025-01-05 - 精简为纯规范，示例移至 tutorial/

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
| 关键字 | 语言预定义保留词 | `type`, `pub`, `use` |
| 字面量 | 固定值 | `42`, `"hello"`, `true` |
| 运算符 | 运算符号 | `+`, `-`, `*`, `/` |
| 分隔符 | 语法分隔符 | `(`, `)`, `{`, `}`, `,` |

### 2.3 关键字

YaoXiang 共定义 17 个关键字：

```
type   pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in
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
StructType  ::= Identifier '(' FieldList ')'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
```

### 3.4 枚举类型

```
EnumType    ::= Identifier '=' Variant ('|' Variant)*
Variant     ::= Identifier (':' TypeExpr)?
```

### 3.5 元组类型

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.6 函数类型

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

### 3.7 泛型类型

```
GenericType ::= Identifier '[' TypeArgList ']'
TypeArgList ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.8 依赖类型

```
DependentType ::= Identifier '[' TypeParamList ']' TypeExpr?
TypeParamList ::= Identifier ':' TypeExpr (',' Identifier ':' TypeExpr)*
```

### 3.9 类型联合

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 3.10 类型交集

```
TypeIntersection ::= TypeExpr '&' TypeExpr
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

### 6.1 函数定义语法

**形式一：类型集中式（推荐）**

```
FunctionDef ::= Identifier ':' FnType '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr
Lambda      ::= '(' ParamNames? ')' '=>' Expr
            |  '(' ParamNames? ')' '=>' Block
ParamNames  ::= Identifier (',' Identifier)*
ParamTypes  ::= TypeExpr (',' TypeExpr)*
```

**形式二：简写式**

```
FunctionDef ::= Identifier '(' ParamTypes? ')' '->' TypeExpr? '=' Lambda
```

### 6.2 并作函数与注解

#### 6.2.1 spawn 函数（并作函数）

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'blocking' | 'eager'
```

**函数注解**：

| 注解 | 位置 | 行为 |
|------|------|------|
| `@blocking` | 返回类型后 | 禁用并发优化，完全顺序执行 |
| `@eager` | 返回类型后 | 强制急切求值 |

**语法示例**：

```
# 并作函数：可并发执行
fetch_data: (String) -> JSON spawn = (url) => { ... }

# @blocking 同步函数：完全顺序执行
main: () -> Void @blocking = () => { ... }

# @eager 急切函数：立即执行
compute: (Int) -> Int @eager = (n) => { ... }
```

#### 6.2.2 spawn 块

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

#### 6.2.3 spawn 循环

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

#### 6.2.4 错误传播运算符

```
ErrorPropagate ::= Expr '?'
```

**示例**：

```
process() -> Result[Data, Error] = {
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

### 8.1 所有权

每个值有唯一的所有者。当所有者离开作用域时，值被自动销毁。

### 8.2 引用类型

```
ref T       # 不可变引用
mut T       # 可变引用
```

### 8.3 生命周期

```
Lifetime   ::= '\'' Identifier
```

---

## 第八章（续）：类型系统约束

### 8.4 Send/Sync 约束

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

### 8.5 并发安全类型

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
type Result[T, E] = ok(T) | err(E)
```

**变体构造**：

| 变体 | 语法 | 说明 |
|------|------|------|
| `ok(T)` | `ok(value)` | 成功值 |
| `err(E)` | `err(error)` | 错误值 |

### 9.2 Option 类型

```
type Option[T] = some(T) | none
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
# 简单类型
type Result[T, E] = ok(T) | err(E)

# 构造器类型
type Point = Point(x: Float, y: Float)

# 枚举类型
type Status = pending | processing | completed

# 函数类型
type Adder = (Int, Int) -> Int
```

### A.2 函数定义

```
# 形式一：类型集中式（推荐）
name: (ParamTypes) -> ReturnType = (params) => body

# 形式二：简写式
name(ParamTypes) -> ReturnType = (params) => body
```

### A.3 模块

```
# 模块即文件
# 文件名.yx 即为模块名
Import ::= 'use' ModuleRef
```

### A.4 控制流

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Identifier in Expr Block Expr Block
for
```

### A.5 match 语法

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
| `struct` | 有 | ❌ 无 | 类型定义使用构造器语法，无需此关键字 |
| `enum` | 有 | ❌ 无 | 使用变体语法 `type X = A | B | C` |

### B.2 语法差异

| 语法元素 | 规范 | 代码实现 | 说明 |
|---------|------|---------|------|
| match arm 分隔符 | `->` | `=>` | 使用 `=>` (FatArrow) |
| 函数定义 | `name(types) -> type = (params) => body` | 两种形式 | 支持类型集中式 `name: Type = (params) =>` |
| 类型定义 | `type Point = { x: Float }` | `type Point = Point(x: Float)` | 使用构造器语法 |

### B.3 待实现特性

以下规范中描述的特性尚未在代码中实现：

| 特性 | 优先级 | 说明 |
|------|--------|------|
| 列表推导式 | P2 | `[x for x in list if condition]` |
| `?` 错误传播 | P1 | Result 类型自动错误传播 |
| 生命周期 `'a` | P2 | 借用检查 |
| `@blocking` 注解 | P1 | 同步执行保证 |
| `spawn` 函数 | P1 | 并作函数标记 |
| `spawn {}` 块 | P1 | 显式并发疆域 |
| `spawn for` 循环 | P1 | 数据并行循环 |
| Send/Sync 约束 | P2 | 并发安全类型检查 |
| Mutex/Atomic 类型 | P2 | 并发安全数据类型 |
| 错误图可视化 | P3 | 并发错误传播追踪 |

---

## 版本历史

| 版本 | 日期 | 作者 | 变更说明 |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初始版本 |
| v1.1.0 | 2025-01-04 | 沫郁酱 | 修正 match arm 使用 `=>` 而非 `->`；更新函数定义语法；更新类型定义语法；添加与代码实现差异说明 |
| v1.2.0 | 2025-01-05 | 沫郁酱 | 精简为纯规范，示例代码移至 tutorial/ 目录 |
| v1.3.0 | 2025-01-05 | 沫郁酱 | 添加并作模型规范（三层并发架构、spawn语法、注解）；添加类型系统约束（Send/Sync）；添加并发安全类型（Mutex、Atomic）；更新错误处理（?运算符）；更新待实现特性列表 |

---

> 本规范定义了 YaoXiang 编程语言的核心语法和语义。
> 教程和示例代码请参考 [YaoXiang 指南](../guides/YaoXiang-book.md) 和 [tutorial/](../tutorial/) 目录。
