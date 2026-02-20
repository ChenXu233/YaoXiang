---
title: RFC-019：类型级同像性 (Typed Homoiconicity)
---

# RFC-019: 类型级同像性 (Typed Homoiconicity) - 语法即类型

> **状态**: 草案
>
> **作者**: 晨煦
>
> **创建日期**: 2026-02-20
>
> **⚠️ 实验性声明**: 这是一个**实验性设计**，在 `exp/typed-homoiconicity` 分支进行探索。**不保证会合并到 main 分支**，未来可能因为各种原因被拒绝或放弃。请勿在生产环境中使用此特性。

---

## 摘要

本 RFC 提出一种激进的语言设计实验：**让语言的语法结构本身成为类型系统的一部分**。

核心思想源于 Lisp 的"代码即数据"（同像性），但通过**静态类型系统**来实现：
- 语法树（AST）是类型
- 关键字是类型的预定义实例
- 用户可以通过定义类型来扩展语言语法

这意味着：语言本身变成可组合、可扩展的"积木"。

---

## 动机

### 为什么做这个实验？

1. **统一性追求**：消除"关键字"这个特殊的语法元素，让一切都是类型和函数
2. **语言可扩展性**：用户可以像定义函数一样定义新的语法结构
3. **类型安全的宏**：传统宏（文本替换）是危险的，类型级同像性可以提供编译期检查
4. **学习目的**：深入理解语言设计的本质

### 与 Lisp 的关系

Lisp 早已实现"代码即数据"：
```lisp
; Lisp 代码本身就是 S-expression
(if (> x 0) "positive" "negative")
```

本实验的差异在于：**用静态类型系统强化这个理念**。

---

## 提案

### 核心概念

#### 1. 语法树即类型（AST as Type）

```yaoxiang
// 语法树节点都是类型
If: Type = { condition: Expr, then: Block, else: Block }
While: Type = { condition: Expr, body: Block }
Return: Type = { value: Expr }
Block: Type = { statements: Array[Expr] }
Let: Type = { name: String, value: Expr, body: Expr }
Function: Type = { params: Array[Param], body: Expr }
Call: Type = { func: Expr, args: Array[Expr] }

// 基本类型
Literal: Type = { value: Int }
StringLiteral: Type = { value: String }
Variable: Type = { name: String }
```

#### 2. 关键字 = 处理类型的函数

```yaoxiang
// 求值器是处理这些类型的函数
eval_if: (node: If, env: Env) -> Value = ...
eval_while: (node: While, env: Env) -> Value = ...
eval_return: (node: Return, env: Env) -> Value = ...
eval_block: (node: Block, env: Env) -> Value = ...

// 编译器也可以是函数
compile_if: (node: If, ctx: CompileContext) -> IR = ...
compile_while: (node: While, ctx: CompileContext) -> IR = ...
```

#### 3. 类型携带解析规则（核心创新）

这是本实验的关键：**类型不仅描述数据，还携带如何解析代码的规则**。

```yaoxiang
// 语法规则类型
SyntaxRule: Type = {
    // 如何解析这个类型的代码
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // 如何将类型实例编译/求值
    compile: (node: Self, ctx: CompileContext) -> IR
    eval: (node: Self, env: Env) -> Value
}

// IF 类型的语法规则
IF: SyntaxRule = {
    // 解析 "if (cond) { then } else { else }"
    parse: (tokens: TokenStream) -> (If, remaining) = {
        consume("if")
        cond = parse_expression(tokens)
        consume("{")
        then_block = parse_block(tokens)
        consume("}")
        consume("else")
        consume("{")
        else_block = parse_block(tokens)
        consume("}")
        return If(cond, then_block, else_block), tokens
    }

    eval: (node: If, env: Env) -> Value = {
        if eval(node.condition, env) != 0 {
            return eval(node.then, env)
        } else {
            return eval(node.else, env)
        }
    }
}
```

#### 4. 用户自定义语法扩展

用户可以定义自己的"关键字"：

```yaoxiang
// 用户定义一个新的语法结构：unless
Unless: SyntaxRule = {
    parse: (tokens: TokenStream) -> (If, remaining) = {
        consume("unless")
        cond = parse_expression(tokens)
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // unless 等价于 if (!cond) { body }
        return If(Not(cond), body, Block([])), tokens
    }
}

// 使用
unless x > 0 {
    print("x is not positive")
}

// 展开为
if !(x > 0) {
    print("x is not positive")
}
```

### 示例

#### 完整示例：自定义循环语法

```yaoxiang
// 定义一个 "times" 循环：n.times { ... } 运行 n 次
TimesLoop: SyntaxRule = {
    parse: (tokens: TokenStream) -> (While, remaining) = {
        receiver = parse_expression(tokens)  // 获取数字
        consume(".times")
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // 转换为 while 循环
        counter_var = gensym("i")
        return While(
            Less(Variable(counter_var), receiver),
            Block([
                body,
                Assign(counter_var, Add(Variable(counter_var), Literal(1)))
            ])
        ), tokens
    }
}

// 使用
5.times {
    print("Hello!")
}

// 展开为
i = 0
while i < 5 {
    print("Hello!")
    i = i + 1
}
```

#### 示例：模式匹配语法

```yaoxiang
// 用户定义模式匹配
Match: SyntaxRule = {
    parse: (tokens: TokenStream) -> (MatchNode, remaining) = {
        subject = parse_expression(tokens)
        consume("{")
        cases = []
        while !check("}") {
            pattern = parse_pattern(tokens)
            consume("=>")
            body = parse_expression(tokens)
            cases.push((pattern, body))
        }
        consume("}")
        return MatchNode(subject, cases), tokens
    }
}

// 使用
match x {
    0 => "zero",
    1 => "one",
    n if n > 10 => "big",
    _ => "other"
}
```

---

## 详细设计

### 系统架构

```
┌─────────────────────────────────────────────────────┐
│                    源代码                            │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              语法解析器 (Parser)                     │
│  - 识别关键字                                        │
│  - 找到对应的 SyntaxRule 类型                        │
│  - 调用类型的 parse 方法                             │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              AST (类型实例)                          │
│  If, While, Match, TimesLoop...                     │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              编译器/解释器                           │
│  - 调用类型的 compile/eval 方法                      │
│  - 生成目标代码或执行                                │
└─────────────────────────────────────────────────────┘
```

### 关键技术问题

#### 1. 控制流函数化

问题：`if` 需要只求值一个分支，不能用普通函数调用。

解决方案：传入 thunk（延迟求值）

```yaoxiang
// 编译后的内部表示
If: Type = {
    condition: Expr,
    then: () -> Value,  // thunk，延迟求值
    else: () -> Value
}
```

#### 2. return 的非局部返回

问题：`return` 需要跳出多层函数。

解决方案：
- 方案 A：编译期 CPS 变换
- 方案 B：使用 Result/Either monad
- 方案 C：限制 return 的作用域

#### 3. 语法歧义

问题：如何区分 `if(x > 0) { 1 }` 是函数调用还是关键字？

解决方案：
- 关键字使用特殊语法（如 `if ... { } else { }`）
- 或通过类型系统约束

#### 4. 无限递归

问题：用户可能定义自引用的语法规则。

解决方案：编译期检测循环依赖

---

## 与现有系统的关系

### 与 RFC-010（统一类型语法）的关系

RFC-010 实现了 `name: type = value` 的统一语法，本 RFC 是其延伸：

| RFC-010 | 本 RFC |
|----------|--------|
| 变量、函数、类型都是 `name: type = value` | 关键字也是 `name: type = value` |
| 类型是值 | 语法规则也是值 |
| `Type` 是元类型 | `SyntaxRule` 是语法的元类型 |

### 与 Lisp/宏的对比

| 特性 | Lisp 宏 | 本实验 |
|------|---------|--------|
| 代码表示 | S-expression (列表) | 类型实例 |
| 扩展方式 | defmacro | 定义 SyntaxRule 类型 |
| 类型安全 | 弱（文本替换） | 强（类型检查） |
| 解析时机 | 运行时/编译时 | 编译期 |
| IDE 支持 | 弱 | 强（类型信息） |

---

## 分支计划

### 实验分支

```
分支名: exp/typed-homoiconicity
从 dev 分支创建
```

**重要**：
- 这是一个**实验性分支**，不会频繁与 dev 合并
- 可能长期独立开发
- **不保证会合并到 main**
- 如果实验失败，分支将被废弃

### 开发阶段

| 阶段 | 目标 | 预期时间 |
|------|------|----------|
| Phase 1 | 概念验证：用现有语法实现 AST 类型 | 2 周 |
| Phase 2 | 实现基本的求值器 | 2 周 |
| Phase 3 | 实现 SyntaxRule 类型的解析规则 | 3 周 |
| Phase 4 | 用户自定义语法扩展 | 3 周 |
| Phase 5 | 优化和文档 | 2 周 |

---

## 权衡

### 优点

- **极致统一**：消除关键字和普通代码的界限
- **语言可扩展**：用户可以定义自己的语法
- **类型安全**：比传统宏更安全
- **学习价值**：深入理解语言本质

### 缺点

- **实现复杂**：需要大幅修改编译器
- **性能担忧**：运行时解释可能慢
- **学习曲线**：概念抽象，需要理解类型系统
- **实用性存疑**：可能过度工程

### 风险

- 实验可能失败，无法找到实用场景
- 实现难度超出预期
- 与现有特性冲突

---

## 开放问题

- [ ] 如何处理语法冲突（用户定义的规则与内置冲突）？
- [ ] 性能优化方案？
- [ ] 是否需要语法导入/导出机制？
- [ ] 如何与现有模块系统集成？

---

## 附录

### 术语表

| 术语 | 定义 |
|------|------|
| 同像性 (Homoiconicity) | 代码和数据使用同一种表示 |
| 语法树 (AST) | 程序的抽象语法树表示 |
| SyntaxRule | 携带语法解析规则的类型 |
| Thunk | 延迟求值的函数包装 |
| CPS | Continuation Passing Style，连续传递风格 |

### 参考文献

- [Lisp 维基：Homoiconicity](https://en.wikipedia.org/wiki/Homoiconicity)
- [Julia 元编程](https://docs.julialang.org/en/v1/manual/metaprogramming/)
- [Rust 过程宏](https://doc.rust-lang.org/book/ch19-06-macros.html)

---

## 生命周期与归宿

```
┌─────────────┐
│   草案      │  ← 当前状态
└──────┬──────┘
       │
       ▼
       ⚠️ 实验性分支独立开发
       (exp/typed-homoiconicity)

       可能的结果：
       ├─► 成功 → 考虑合并到 dev
       ├─► 失败 → 废弃分支
       └─► 无限期停滞
```

> **⚠️ 重要提醒**: 这是一个实验性 RFC，**不保证会合并到 main**。请勿在生产代码中依赖此特性。
