# YaoXiang 並发模型规范

> **状态**：本文档描述的是 YaoXiang 语言的新并发模型设计。它取代了旧的基于 `@block`/`@eager`/`@auto` 注解、`Send`/`Sync` trait 和 `Mutex`/`RwLock` 的并发方案。部分内容尚未实现，以实际编译器行为为准。

本文件定义 YaoXiang 编程语言的并发模型规范，包括 `{}` 块语义、`spawn` 并发原语、错误处理和资源类型。

---

## 第一章：概述

### 1.1 {} 块的本质

在 YaoXiang 中，`{}` 是一个**依赖驱动的计算单元**。

| 属性 | 说明 |
|------|------|
| 依赖驱动 | 块在执行时检查内部所有变量是否就绪，若齐备则立即执行，否则阻塞等待 |
| 执行时机 | 由依赖决定，与"立即"或"延迟"无关 |
| 返回值 | 使用 `return` 显式返回值；无 `return` 时默认返回 `Void` |
| 语法统一 | 无论出现在函数体、变量初始化还是 `spawn` 后，语义一致 |
| 作用域隔离 | 变量严格限于 `{}` 内部，不穿透到外层作用域 |

```yaoxiang
// 依赖驱动示例
x = compute_x()        // x 就绪
y = compute_y()        // y 就绪
result = {
    // 依赖 x 和 y，两者就绪后立即执行
    return x + y
}
```

### 1.2 返回规则

YaoXiang 的返回规则统一且明确：

| 写法 | 返回值 | 说明 |
|------|--------|------|
| `= expr`（无花括号） | 直接返回 `expr` | 表达式即值 |
| `= { ... }`（有花括号） | 必须用 `return`，否则返回 `Void` | 块需要显式返回 |

```yaoxiang
// 无花括号：直接返回
add: (a: Int, b: Int) -> Int = a + b

// 有花括号：必须用 return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 有花括号但无 return：返回 Void
log: (message: String) -> Void = {
    print(message)  // 无 return，返回 Void
}
```

### 1.3 spawn 块语义

`spawn { ... }` 是 YaoXiang 中唯一的并行原语。

**核心规则**：
- spawn 块的**直接子赋值**创建并行任务
- 嵌套 `{}` 内的赋值不算作独立任务
- 整个 spawn 块同步阻塞，等待所有任务完成后返回结果
- 没有回调、`await` 或注解

```yaoxiang
// 两个任务并行执行
(a, b) = spawn {
    fetch("url1"),      // 任务 1
    fetch("url2")       // 任务 2
}
// 等待两者都完成后继续
```

### 1.4 用户心智模型

> 你写的普通代码是顺序执行的。
> 当你希望多件事一起做时，把它们放进 `spawn { ... }` 块里。
> 块里的每个直接赋值都会立即开始（并行），你需要的结果会自动等待。
> 整个块会等所有事做完，然后把最终结果给你。
> 没有回调，没有 `await`，没有奇怪的注解。

---

## 第二章：语法与语义

### 2.1 普通代码

普通代码（spawn 块外部）是**顺序执行**的。

```yaoxiang
a = compute_a()     // 先执行
b = compute_b(a)    // 依赖 a，a 完成后执行
c = compute_c(b)    // 依赖 b，b 完成后执行
```

### 2.2 spawn 块

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**语义**：
1. spawn 块内的直接子赋值作为独立任务并行执行
2. 每个任务的结果绑定到对应的模式变量
3. 整个块阻塞直到所有任务完成
4. 返回所有结果的元组

```yaoxiang
// 单任务
result = spawn {
    fetch("url")
}

// 多任务
(a, b, c) = spawn {
    fetch("url1"),
    fetch("url2"),
    fetch("url3")
}
```

### 2.3 函数体中的 spawn

函数体本身是一个 `{}` 块，可以在其中使用 `spawn`。

```yaoxiang
fetch_and_parse: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        parsed = parse(fetch(url))
    }
    return results
}
```

### 2.4 循环中的 spawn

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
```

**语义**：数据并行循环，每次迭代作为独立任务。

```yaoxiang
// 并行处理列表中的每个元素
results = spawn for item in items {
    result = process(item)
}
```

> **注意**：`spawn for` 的循环体是独立任务，不支持跨迭代共享可变状态。如需聚合结果，应使用 `spawn for` 收集结果后在外部处理（见下方示例）。

```yaoxiang
// 正确：并行处理后在外部聚合
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // 顺序聚合
```

### 2.5 嵌套 spawn

spawn 块可以嵌套，内层 spawn 创建新的并发域。

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**注意**：内层 spawn 的直接子赋值才是任务，外层 spawn 不会穿透。

---

## 第三章：与所有权模型的交互

### 3.1 Move 语义

Move 是 YaoXiang 的默认语义（零拷贝）。

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // data 的所有权移入 spawn 块
}
// data 在此处不可用（已 move）
```

**规则**：
- 变量进入 spawn 块后，外部不能再使用
- 如果需要在多个任务间共享，使用 `ref`

### 3.2 借用令牌

`&T` 和 `&mut T` 是零大小的编译期权限证明，**不能跨任务边界**。

```yaoxiang
data = load_data()
ref_data = &data

// 编译错误：借用令牌不能跨任务
result = spawn {
    process(ref_data)   // 错误！
}
```

### 3.3 ref 共享

`ref` 是跨作用域共享的唯一方式。

```yaoxiang
data = load_data()
shared = ref data       // 编译器自动选择 Rc 或 Arc

result = spawn {
    process_a(shared),  // 共享引用
    process_b(shared)   // 共享引用
}
```

**编译器选择（保守策略）**：

| 条件 | 选择 |
|------|------|
| 默认 | `Arc`（安全优先） |
| 编译器能证明仅单任务内使用 | `Rc`（无原子操作开销） |

### 3.4 闭包捕获

闭包捕获 = Move，一个闭包只能给一个任务用。

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // 闭包 move 捕获 data

// 编译错误：闭包只能用于一个任务
result = spawn {
    fn(1),      // 使用闭包
    fn(2)       // 错误！闭包已 move
}
```

**正确做法**：为每个任务创建独立闭包或使用 `ref`。

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

---

## 第四章：错误处理

### 4.1 ? 运算符

`?` 运算符用于显式错误传播，与 Rust 语义一致。

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // 如果错误，立即传播
    return content.read_all()
}
```

### 4.2 spawn 块内错误传播

**规则**：
1. 等待所有任务完成（即使某些任务已失败）
2. 传播第一个遇到的错误
3. 使用 `?` 显式标记错误传播点

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 可能失败
    fetch("url2")?      // 可能失败
}
// 如果任一任务失败，整个 spawn 块传播第一个错误
```

### 4.3 错误类型

**自动生成**：编译器自动生成联合错误类型（类似 TypeScript 联合类型）。

```yaoxiang
// 编译器推断出错误类型为 HttpError | IoError
(a, b) = spawn {
    fetch("url"),           // 可能抛出 HttpError
    read_file("data.txt")  // 可能抛出 IoError
}
```

**手动覆盖**：用户可手动定义统一错误类型。

```yaoxiang
AppError: Type = {
    Http: (HttpError) -> Self,
    Io: (IoError) -> Self,
    Parse: (ParseError) -> Self
}

process: (url: String, path: FilePath) -> Result(Data, AppError) = {
    (a, b) = spawn {
        fetch(url).map_err(AppError.Http)?,
        read_file(path).map_err(AppError.Io)?
    }
    return parse(a + b).map_err(AppError.Parse)?
}
```

---

## 第五章：资源类型与副作用

### 5.1 内置资源类型

| 资源类型 | 说明 | 编译器行为 |
|----------|------|-----------|
| `FilePath` | 文件系统路径 | 同路径操作自动串行 |
| `HttpUrl` | HTTP 端点 | 同 URL 操作自动串行 |
| `DBUrl` | 数据库连接 | 同连接操作自动串行 |
| `Console` | 标准输出 | 所有 Console 操作自动串行 |

```yaoxiang
// 同一文件的操作自动串行化
(a, b) = spawn {
    read_file("data.txt"),      // 先执行
    write_file("data.txt", x)   // 等待读取完成
}
```

### 5.2 用户自定义资源类型

用户自定义资源类型需显式标记。

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 副作用追踪

编译器追踪资源类型的使用，确保并发安全。

```yaoxiang
// 编译器警告：Console 操作可能交错
spawn {
    print("Hello"),     // 可能与下一行交错
    print("World")
}

// 正确：显式串行
spawn {
    print("Hello\nWorld")
}
```

---

## 第六章：编译器行为

### 6.1 DAG 分析

编译器在编译期分析 spawn 块内的依赖关系（DAG），确定：
1. 哪些表达式可以并行
2. 哪些必须串行
3. 如何分配任务

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // 任务 1
    y = fetch("url2"),      // 任务 2（与任务 1 并行）
    z = process(x, y)       // 任务 3（依赖 x 和 y，必须等待）
}
```

### 6.2 Rc/Arc 选择（保守策略）

编译器采用**保守策略**，默认使用 `Arc` 以确保线程安全：

| 条件 | 选择 | 原因 |
|------|------|------|
| 默认（无法证明安全性） | `Arc` | 安全优先，避免数据竞争 |
| 编译器能**证明**数据仅在单任务内使用 | `Rc` | 无原子操作开销 |

**策略说明**：
- **默认 `Arc`**：当编译器无法确定 `ref` 是否仅在单个任务内使用时，保守选择 `Arc`
- **降级为 `Rc`**：仅当编译器能通过 DAG 分析**证明**数据绝对不会跨任务共享时，才降级为 `Rc`
- **宁可慢，不可错**：选择 `Arc` 的额外开销远小于数据竞争的风险

```yaoxiang
data = load_data()

// 默认：编译器选择 Arc（保守策略）
result = spawn {
    shared = ref data
    process(shared)
}

// 仅当编译器能证明单任务内使用时：降级为 Rc
// （需要编译器的 DAG 分析能明确排除跨任务可能性）
```

### 6.3 无并行警告

如果 spawn 块内的任务没有实际并行机会，编译器发出警告。

```yaoxiang
// 编译器警告：无并行机会
result = spawn {
    a = fetch("url")    // 唯一任务
}
// 建议：直接使用普通代码
result = fetch("url")
```

### 6.4 资源冲突检测

编译器检测资源类型的潜在冲突。

```yaoxiang
// 编译错误：同一文件的并发写入
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // 错误！
}
```

---

## 第七章：与旧设计的对比

### 7.1 废弃的特性

| 旧特性 | 状态 | 替代方案 |
|--------|------|----------|
| `@block`、`@eager`、`@auto` 注解 | 废弃 | 无，依赖驱动自动处理 |
| 全程序自动 DAG 分析 | 废弃 | 仅 spawn 块内分析 |
| `Send`、`Sync` trait | 废弃 | 所有权 + ref 自动处理 |
| future/非阻塞句柄 | 废弃 | spawn 块同步返回 |
| `Mutex[T]`、`Atomic[T]`、`RwLock[T]` | 废弃 | ref 自动选择 Rc/Arc |

### 7.2 设计理念转变

**旧模型**：
- 显式注解控制并发行为
- 复杂的 trait 约束
- 异步编程模型

**新模型**：
- 依赖驱动，隐式并发
- 所有权 + ref 简化共享
- 同步编程模型，spawn 块阻塞返回

### 7.3 迁移指南

```yaoxiang
// 旧代码（伪代码，展示旧模型风格；@block/let/await 不是 YaoXiang 关键字）
@block async fetch_data(): Future(Data) = {
    let data = await fetch("url")
    return data
}

// 新代码
fetch_data: () -> Data = {
    data = fetch("url")     // 同步调用
    return data
}

// 并发版本
fetch_multiple: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        result = fetch(url)
    }
    return results
}
```

---

## 附录：语法速查

### A.1 spawn 语句

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
SpawnBody   ::= Assignment (',' Assignment)*
```

### A.2 错误处理

```
Expr '?'              // 错误传播（Result 类型）
```

### A.3 ref 表达式

```
RefExpr     ::= 'ref' Expr
```

### A.4 资源类型标记

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```