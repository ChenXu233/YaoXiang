# 语法规范

本文件定义 YaoXiang 编程语言的语法规范，包括词法结构、语法规则和运算符优先级。

---

## 第一章：词法结构

### 1.1 源文件

YaoXiang 源文件必须使用 UTF-8 编码。源文件通常以 `.yx` 为扩展名。

### 1.2 词法单元分类

| 类别   | 说明               | 示例                      |
| ------ | ------------------ | ------------------------- |
| 标识符 | 以字母或下划线开头 | `x`, `_private`, `my_var` |
| 关键字 | 语言预定义保留词   | `Type`, `pub`, `use`      |
| 字面量 | 固定值             | `42`, `"hello"`, `true`   |
| 运算符 | 运算符号           | `+`, `-`, `*`, `/`        |
| 分隔符 | 语法分隔符         | `(`, `)`, `{`, `}`, `,`   |

### 1.3 关键字

YaoXiang 定义了极少量的关键字：

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

这些关键字在任何上下文中都具有特殊含义，不能用作标识符。

### 1.4 保留字

YaoXiang 的"保留字"分三层，分别由解析器（parser）和类型检查器（type checker）在不同阶段识别：

#### 1.4.1 字面量保留字

解析器有独立 token 的字面量标识符，不能用作普通标识符：

| 标识符  | 所属类型 | 说明                                                                                |
| ------- | -------- | ----------------------------------------------------------------------------------- |
| `Type`  | —        | 元类型关键字                                                                        |
| `true`  | Bool     | 布尔真值                                                                            |
| `false` | Bool     | 布尔假值                                                                            |
| `void`  | Void     | Void 字面量（Unit 值）。小写 `void` 是值字面量；大写 `Void` 是类型名（见 §1.4.3）。 |

#### 1.4.2 构造子表达式

以下构造子在模式匹配和表达式上下文中由解析器识别：

| 构造子    | 所属类型 | 说明              |
| --------- | -------- | ----------------- |
| `some(T)` | Option   | Option 值变体构造 |
| `ok(T)`   | Result   | Result 成功变体   |
| `err(E)`  | Result   | Result 错误变体   |

#### 1.4.3 内建类型名

以下类型名由类型检查器预注册，无需导入即可在类型位置使用。解析器将它们视作普通标识符——**不是保留字，可以被局部绑定 shadow（不推荐）**。

| 类型名   | 逻辑对应       | 说明                                                                                                |
| -------- | -------------- | --------------------------------------------------------------------------------------------------- |
| `Void`   | ⊤（真/Unit）   | 零字段积类型，恰好一个居留者（`void` 字面量，见 §1.4.1）                                            |
| `Never`  | ⊥（假/空类型） | 零变体和类型，零个居留者。无任何表达式能产生 `Never` 值。`Never <: T` 对所有 `T` 成立（爆炸原理）。 |
| `Int`    | —              | 有符号整数                                                                                          |
| `Float`  | —              | 浮点数                                                                                              |
| `Bool`   | —              | 布尔值：`true` / `false`                                                                            |
| `Char`   | —              | Unicode 字符                                                                                        |
| `String` | —              | 字符串                                                                                              |

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
Array       ::= '[' Expr (',' Expr)* ']'   // 目标类型注解为 Array(T, N) 时字面量落定长数组
```

> #299/#300：Set 无字面量文法、无运行时表示——集合类型规划中，
> 需求出现时照 Dict 模式补全（std.set + HeapValue::Set）。
> List/Dict 字面量的落点由上下文类型注解决定：裸字面量与 `List(T)` 注解
> 落可增长列表；`Array(T, N)` 注解直接作用于字面量时落定长数组。
> 禁止隐式 List→Array 转换。
>
> Array 字面量语义（#300）：
>
> - 元素个数必须等于 N，不等则编译期 E1002；空字面量配非零 N 同样拒绝
> - 每个元素类型必须与 T 兼容，不符则编译期 E1002
> - N 的文法形态：仅整数字面量（可负）或常量名；复合表达式（如 `2+1`）解析期拒绝
> - N 为符号常量（函数 const 参数，如 `Array(Int, n)`）时个数校验推迟到精化类型阶段
> - v1 嵌套数组字面量（`Array(Array(Int,2),2) = [[1,2],[3,4]]`）编译期拒绝，
>   需逐层显式构造；递归落点留待后续版本

#### 1.6.5 列表推导式

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **#299 §3 行为收紧（迁移记录）**：迭代变量文法本就是 `'for' Identifier 'in'`，
> 但旧实现中 pattern 走完整 pratt 解析——`'in'` 注册为中缀运算符后，
> `x` 会把 `in items` 吞成 membership 表达式。修复后非标识符 pattern 直接解析失败，
> 不再像旧实现 fallback 到 `_`（静默吞错）。影响面：此前能解析的
> `[x for (a, b) in pairs]` 这类写法现在报错——该形态从未有定义行为
> （变量恒为 `_`），收紧方向正确，无语义迁移成本。

#### 1.6.6 成员检测

```
Membership  ::= Expr 'in' Expr
```

> #299 §3：`in` 是二元关系运算符，返回 `Bool`——命中 `true`、未命中
> `false`，不报错。语义切分：`[]` 是断言存在并取值（失败报错），
> `in` 是询问是否存在（未命中是正常 `false`）。
> 右操作数覆盖：List / Array / Dict(键集) / Tuple / String(子串) /
> Range(区间)。`in` 是一等霍尔谓词，精化类型阶段作为编译期可证命题的基底。
> （#300 决策4：Set 从右操作数列表除名——Set 无运行时表示，见 §1.6.4）

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
              | RangeExpr
              | ErrorPropagate
              | RefExpr
              | IfExpr
              | MatchExpr
              | Block
              | Lambda
```

### 2.2 运算符优先级

| 优先级 | 运算符                      | 结合性 |
| ------ | --------------------------- | ------ |
| 1      | `()` `[]` `.` `?`           | 左到右 |
| 2      | `as`                        | 左到右 |
| 3      | 一元前缀 `!` `-` `+`         | 右到左 |
| 4      | `*` `/` `%`                 | 左到右 |
| 5      | `+` `-`                     | 左到右 |
| 6      | `..`                        | 左到右 |
| 7      | `<<` `>>`                   | 左到右 |
| 8      | `&` `\|` `^`                | 左到右 |
| 9      | `==` `!=` `<` `>` `<=` `>=` | 左到右 |
| 10     | `and` `or`                  | 左到右 |
| 11     | `if...else`                 | 右到左 |
| 12     | `=` `+=` `-=` `*=` `/=`     | 右到左 |

> **一元前缀运算符**（`!` `-` `+`）紧绑定：只低于调用与成员访问，高于所有二元运算符。
> 因此 `!a == b` ≡ `(!a) == b`（Zig 式语义）；`!` 是纯一元运算，不参与短路控制流，
> 与 `and`/`or` 关键字（短路）正交（RFC-010 权威定义）。

> **Range 绑定力（#299 §3 / #300 F 项）**：`..` 绑定力 (6, 7)——左 6 低于加法（7），
> 右 7 吞加法不吞同级 `..`。变更前后对照：
>
> | 表达式 | 变更前（级 1，右结合） | 变更后（(6,7)，左结合） |
> | ------ | --------------------- | --------------------- |
> | `x in 1..10` | `x in 1..10`（`in` 右操作数级 4，`..` 级 1 吞不掉，实际无法解析） | `x in (1..10)`——区间整体作为 `in` 右操作数 |
> | `0..n+2` | `(0..n)+2`（右结合陷阱：上界被吃掉，`for` 循环直接 E3004） | `0..(n+2)`——上界是算术表达式 |
> | `a == b..c` | `a == (b..c)`（`..` 级 1 < `==` 级 3，天然整体） | `a == (b..c)`——**语义不变**，`..` 仍高于比较级 |
> | `1..2*3` | `(1..2)*3` | `1..(2*3)`——上界是算术表达式 |
> | `a..b..c` | `a..(b..c)`（右结合链式，无意义 Range 套 Range） | `(a..b)..c`——**step 形态**（`c` 为步长，#300 I 项） |
>
> 净效果：复合上界 `for i in 0..n+2` 从「解析成功但 E3004」变为「直接可用」；
> `x in 1..10` 从「无法解析」变为「区间检查」；`a..b..c` 从「无意义嵌套」变为「step 分量」。
> 级 6 落在 `+`（级 5）与 `<<`（级 7）之间，数学惯例：区间是紧绑定构造，上界自然是完整算术表达式。

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
IfExpr      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
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

> **语句终止规则**：Stmt 之间的分隔与换行行为（`;` 显式分隔、换行终止、续行例外、行首 `(`/`[` 永不合并）
> 由 [RFC-038](../design/rfc/draft/038-statement-termination.md) 定义。

**统一语义**：所有 `{}` 块的 return 语义一致：

| 块类型      | return 语义  | 默认返回 |
| ----------- | ------------ | -------- |
| 普通 `{}`   | 返回值       | Void     |
| `unsafe {}` | 返回类型定义 | Void     |
| `spawn {}`  | 返回结果     | Void     |

**核心原则**：

- `{}` 中的 `return` 总是将内容返回给上一作用域
- 默认没有 `return` 为返回 `Void`
- 表达式形式 `= expr` 直接返回值

```yaoxiang
// 普通 {} 块：return 返回值
result = {
    x = compute()
    return x  // 返回值给上一作用域
}

// unsafe {} 块：return 返回类型定义
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  // 返回类型定义给上一作用域
}

// spawn {} 块：return 返回结果
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  // 返回结果给上一作用域
}
```

### 2.10 Lambda 表达式

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 错误传播运算符

```
ErrorPropagate ::= Expr '?'
```

`?` 运算符是后缀运算符，优先级与 `.` 同级。对 `Result(T, E)` 类型：

- `Ok(v)` 时提取值 `v` 继续执行
- `Err(e)` 时将错误向上传播（`return Err(e)`）

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // 成功时提取值，失败时向上传播
    transform(validated)
}
```

### 2.12 范围表达式

```
RangeExpr   ::= Expr '..' Expr ('..' Expr)?
```

`..` 创建范围值（#300 I 项：Range 是一等值，非语法糖）。

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]

// Range 是值：绑定、传递、成员判断
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step 形态（第三分量，默认 1）
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **step 语义**：`a..b..c` 中 `c` 是步长。`c = 0` 字面量编译期拒绝；
> 动态 `c` 运行时零检查（E6001 家族，#301 错误系统落地后升格 Result）。
> `c < 0` 合法，区间方向随符号反转（`10..0..(-2)` 递减）。

### 2.13 ref 表达式

```
RefExpr     ::= 'ref' Expr
```

`ref` 创建共享持有。编译器自动选择 Rc（单任务）或 Arc（跨任务），用户不需要关心实现细节。

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // 跨任务：编译器自动选 Arc
```

### 2.14 unsafe 表达式

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` 块用于定义不透明类型和操作裸指针。使用 `return` 将类型定义返回给上一作用域。

**语义**：

- `unsafe {}` 中可以定义类型和操作裸指针
- 返回的类型在 `unsafe {}` 外可用
- 类型的字段访问需要 unsafe 权限

```yaoxiang
// 在 unsafe 块中定义不透明类型
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // 裸指针
    }
    return SqliteDb
}

// SqliteDb 在 unsafe 块外可用
db = sqlite3_open("test.db")
```

### 2.15 作用域

**基本规则**：

- 每个 `{}` 块创建一个作用域
- 内层作用域可以访问外层作用域的变量
- 外层作用域不能访问内层作用域的变量
- 变量声明遵循"赋值优先"原则

```yaoxiang
// 块作用域
{
    x = 10
    // x 在此作用域内可见
}
// x 在此作用域外不可见

// 函数作用域
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result 在函数外不可见
```

**变量声明与遮蔽**：

- `x = value`：沿作用域链向外查找 x，找到则赋值，找不到则新声明
- `mut x = value`：显式新可变声明，禁止与外层同名
- 同作用域内任何名字只能声明一次

> **详细定义**：作用域的完整规则、变量声明和遮蔽机制详见 [模块系统规范](./modules.md#第四章作用域)。

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
              | WhileStmt
              | ForStmt
              | SpawnStmt
```

### 3.2 变量声明

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 3.3 return 语句

```
ReturnStmt  ::= 'return' Expr?
```

**语义**：`return` 用于从代码块中返回值。若无 `return`，代码块默认返回 `Void`。

### 3.4 break 语句

```
BreakStmt   ::= 'break'
```

**语义**：立即终止所在的最内层 `while`/`for` 循环，控制流转到该循环体之后。

- **只出最近一层**：`break` 恒作用于包含它的最内层循环。嵌套循环中需要一次跳出多层时，
  把内层循环提取为函数用 `return` 返回，或使用标志位（#314 定案：break/continue 不带标签；
  若未来引入循环标签，将按循环声明侧语法走 RFC 流程，与证明管道的多出口设计一并裁决）
- **仅限循环体内**：`break` 只能出现在 `while`/`for` 循环体内（含体内嵌套的块/if/match），
  出现在循环外编译报错（E1102 `'break' outside of a loop`）
- **不影响终止性证明**：`while` 循环仍须可证明终止（decreases 度量）；`break` 不参与
  终止性论证，`while true { break }` 不会被接受
- **借用语义**：break 的控制流边参与 RFC-009a 反向 BFS 活性分析的结构性切断
  （跳出的迭代不参与回边活性推导）

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // 控制流转到循环之后，i == 3
    }
}

// 嵌套循环：break 只出内层
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // 只终止内层循环
    }
    j = j + 1                  // 每轮外层迭代都会执行到这里
}
```

### 3.5 continue 语句

```
ContinueStmt::= 'continue'
```

**语义**：跳过本次迭代中剩余的语句，直接进入所在的最内层循环的下一轮——
`while` 回到条件重判，`for` 取下一个元素。

- **只作用最近一层**：与 `break` 相同，不带标签（#314）
- **仅限循环体内**：出现在循环外编译报错（E1102）

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // 跳过下面的累加，n == 3 不计入
    }
    sum = sum + n
}
// sum == 12（1 + 2 + 4 + 5）
```

### 3.6 if 语句

```
IfStmt      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
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

| 迭代  | 循环变量的行为                                             |
| ----- | ---------------------------------------------------------- |
| 第1次 | 创建新绑定 `i = 1`，循环体执行，打印 1                     |
| 第2次 | 创建新绑定 `i = 2`（之前的绑定已销毁），循环体执行，打印 2 |
| 第3次 | 创建新绑定 `i = 3`，循环体执行，打印 3                     |
| 第4次 | 创建新绑定 `i = 4`，循环体执行，打印 4                     |
| 结束  | 循环体结束，绑定销毁                                       |

**关键点**：每次迭代结束后，当次迭代创建的绑定会被销毁。下一次迭代是一个全新的绑定，与上一次迭代的绑定没有任何关系。

#### 3.9.2 for 与 for mut 的区别

| 语法                | 循环变量可变性 | 说明                 |
| ------------------- | -------------- | -------------------- |
| `for i in 1..5`     | 不可变         | 循环体内不能修改绑定 |
| `for mut i in 1..5` | 可变           | 循环体内可以修改绑定 |

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

YaoXiang 禁止变量遮蔽。for 循环变量不能与外层作用域中的变量同名：

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

此规则适用于所有代码块，详见 [4.3 遮蔽规则](./modules.md#43-遮蔽规则)。

#### 3.9.4 与其他语言的对比

| 语言     | for 循环变量语义                 |
| -------- | -------------------------------- |
| YaoXiang | 每次迭代绑定新值                 |
| Rust     | 修改同一个变量（需要 mut）       |
| Python   | 修改同一个变量（无需 mut）       |
| C/C++    | 修改同一个变量（需要指针或引用） |

**设计理由**：YaoXiang 采用绑定语义是因为：

1. **更符合自然语义** 在自然语言中，"对于集合中的每个元素 x" 意味着每个 x 是独立的个体。YaoXiang 的
   `for i in 1..5`
   读作"对于 1 到 5 中的每个 i"，每次迭代的 i 是一个全新的绑定，这与人类的直觉理解一致。

2. **避免意外修改**
   默认不可变的绑定语义意味着循环体内无法意外修改循环变量。不需要担心在复杂循环体中某个地方不小心写了
   `i = ...` 导致难以追踪的 bug。

3. **高性能方案触手可及** 当确实需要在迭代间复用变量时（例如累加器、缓存），使用 `for mut`
   声明即可切换到可变绑定模式。这比隐式共享状态更清晰——意图通过语法显式表达，而不是藏在运行时行为里。

### 3.10 spawn 语句

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn 块**：显式声明并发疆域，块内表达式并发执行。

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn 循环**：数据并行循环。

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

**spawn 块捕获外层变量**（RFC-024 §2.3，值捕获语义）：

- 块体引用外层变量 = **Move 值捕获**：值在 spawn 创建点快照进闭包环境，
  块体经 env 读取（LoadUpvalue）
- **原语**（Int/Float/Bool/Char）值复制，外层变量不受影响
- **句柄类型**（Struct/String/List 等）快照 = 句柄复制，共享底层对象；
  Embedded 运行时（默认）同线程同堆，句柄有效
- 多个任务间共享需显式 `ref`（§2.13，编译器自动选 Rc/Arc）
- 块内 `return` 引用的外层变量同样捕获

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 值捕获，result == 6
}
```

---

## 附录：语法速查

### A.1 控制流

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // 仅循环体内（§3.4 / §3.5）
```

### A.2 错误处理

```
Expr '?'              // 错误传播（Result 类型）
```

### A.3 match 语法

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```
