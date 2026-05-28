# 语法规范

本文件定义 YaoXiang 编程语言的语法规范，包括词法结构、语法规则和运算符优先级。

---

## 第一章：词法结构

### 1.1 源文件

YaoXiang 源文件必须使用 UTF-8 编码。源文件通常以 `.yx` 为扩展名。

### 1.2 词法单元分类

| 类别 | 说明 | 示例 |
|------|------|------|
| 标识符 | 以字母或下划线开头 | `x`, `_private`, `my_var` |
| 关键字 | 语言预定义保留词 | `Type`, `pub`, `use` |
| 字面量 | 固定值 | `42`, `"hello"`, `true` |
| 运算符 | 运算符号 | `+`, `-`, `*`, `/` |
| 分隔符 | 语法分隔符 | `(`, `)`, `{`, `}`, `,` |

### 1.3 关键字

YaoXiang 定义了极少量的关键字：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

这些关键字在任何上下文中都具有特殊含义，不能用作标识符。

### 1.4 保留字

| 保留字 | 类型 | 说明 |
|--------|------|------|
| `Type` | Type | 元类型 |
| `true` | Bool | 布尔真值 |
| `false` | Bool | 布尔假值 |
| `void` | Void | 空值 |
| `some(T)` | Option | Option 值变体 |
| `ok(T)` | Result | Result 成功变体 |
| `err(E)` | Result | Result 错误变体 |

### 1.5 标识符

标识符以字母或下划线开头，后续字符可以是字母、数字或下划线。标识符大小写敏感。

特殊标识符：
- `_` 用作占位符，表示忽略某个值
- 以下划线开头的标识符表示私有成员

### 1.6 字面量

#### 1.6.1 整数

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 浮点数

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 1.6.3 字符串

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 1.6.4 集合

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Set         ::= '{' Expr (',' Expr)* '}'
```

#### 1.6.5 列表推导式

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

#### 1.6.6 成员检测

```
Membership  ::= Expr 'in' Expr
```

### 1.7 注释

```
// 单行注释

/* 多行注释
   可以跨越多行 */
```

### 1.8 缩进规则

代码必须使用 4 个空格缩进，禁止使用 Tab 字符。这是强制语法规则。

---

## 第二章：语法规则

### 2.1 表达式分类

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

### 2.2 运算符优先级

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

### 2.3 函数调用

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier ':' Expr
```

### 2.4 成员访问

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 索引访问

```
IndexAccess ::= Expr '[' Expr ']'
```

### 2.6 类型转换

```
TypeCast    ::= Expr 'as' TypeExpr
```

### 2.7 条件表达式

```
IfExpr      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 2.8 模式匹配

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

### 2.9 块表达式

```
Block       ::= '{' Stmt* Expr? '}'
```

### 2.10 Lambda 表达式

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

---

## 第三章：语句

### 3.1 语句分类

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

### 3.2 变量声明

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 3.3 return 语句

```
ReturnStmt  ::= 'return' Expr?
```

### 3.4 break 语句

```
BreakStmt   ::= 'break' Identifier?
```

### 3.5 continue 语句

```
ContinueStmt::= 'continue'
```

### 3.6 if 语句

```
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 3.7 match 语句

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 3.8 while 语句

```
WhileStmt   ::= 'while' Expr Block
```

### 3.9 for 语句

```
ForStmt     ::= 'for' 'mut'? Identifier 'in' Expr Block
```

#### 3.9.1 语义：每次迭代是绑定新值

YaoXiang 的 for 循环语义与传统语言不同：**每次迭代是绑定新值，而不是修改同一个变量**。

```yaoxiang
// 示例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**执行过程**：

| 迭代 | 循环变量的行为 |
|------|----------------|
| 第1次 | 创建新绑定 `i = 1`，循环体执行，打印 1 |
| 第2次 | 创建新绑定 `i = 2`（之前的绑定已销毁），循环体执行，打印 2 |
| 第3次 | 创建新绑定 `i = 3`，循环体执行，打印 3 |
| 第4次 | 创建新绑定 `i = 4`，循环体执行，打印 4 |
| 结束 | 循环体结束，绑定销毁 |

**关键点**：每次迭代结束后，当次迭代创建的绑定会被销毁。下一次迭代是一个全新的绑定，与上一次迭代的绑定没有任何关系。

#### 3.9.2 for 与 for mut 的区别

| 语法 | 循环变量可变性 | 说明 |
|------|----------------|------|
| `for i in 1..5` | 不可变 | 循环体内不能修改绑定 |
| `for mut i in 1..5` | 可变 | 循环体内可以修改绑定 |

```yaoxiang
// 合法：每次迭代绑定新值，不需要修改
for i in 1..5 {
    print(i)  // 读取 i 的值
}

// 错误：不可变绑定，不能修改
for i in 1..5 {
    i = i + 1  // 错误：不能修改不可变绑定
}

// 合法：使用 for mut 允许修改绑定
for mut i in 1..5 {
    i = i + 1  // 允许修改
}
```

#### 3.9.3 遮蔽检查

for 循环变量不能遮蔽外层作用域中已存在的变量：

```yaoxiang
// 错误：i 已经在外部声明
i = 10
for i in 1..5 {
    print(i)
}

// 正确：使用不同的变量名
i = 10
for j in 1..5 {
    print(j)
}
```

错误代码：`E2013 - Cannot shadow existing variable`

#### 3.9.4 与其他语言的对比

| 语言 | for 循环变量语义 |
|------|------------------|
| YaoXiang | 每次迭代绑定新值 |
| Rust | 修改同一个变量（需要 mut） |
| Python | 修改同一个变量（无需 mut） |
| C/C++ | 修改同一个变量（需要指针或引用） |

**设计理由**：YaoXiang 采用绑定语义是因为：
1. 每次迭代结束后循环体内的变量会销毁
2. 下一次迭代是一个全新的绑定
3. 这样更安全，不需要考虑迭代之间的状态

---

## 附录：语法速查

### A.1 控制流

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Identifier in Expr Block Expr Block
for
```

### A.2 match 语法

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```
