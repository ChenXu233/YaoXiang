---
title: RFC-007：函数定义语法统一方案
---

# RFC-007: 函数定义语法统一方案

> **状态**: 已接受
> **作者**: 沫郁酱
> **创建日期**: 2025-01-05
> **最后更新**: 2026-03-21（对齐类型构造器规则与代码块返回语义）

## 摘要

本 RFC 确定 YaoXiang 语言**函数定义语法**的最终方案。使用统一语法 `name: (params) -> Return = body`，与 RFC-010 的 `name: type = value` 模型完全一致。

为避免歧义：当函数存在输入参数时，参数类型必须在「签名」或「lambda 头」至少一处显式标注；两边都省略将被拒绝。

代码块 `{ ... }` 的最后一个表达式作为返回值；空块 `{}` 返回 `Void`。

## 动机

### 为什么需要这个特性？

1. **语法一致性**：消除旧语法的历史包袱，统一风格
2. **简洁性**：HM算法自动推断类型，减少样板代码
3. **类型安全**：HM算法保证类型安全，无法推断时才显式标注
4. **语言成熟度**：HM算法是现代函数式语言的成熟方案

### 统一语法模型

**核心原则**：`name: Signature = LambdaBody`

- **完整形式**：签名（含参数名 + 类型 + `->` + 返回类型） + Lambda头（含参数名）
- **简写规则**：在不引入歧义的前提下尽量省略
  - `->` 不能省略（函数类型的标志，否则会被解析为元组）
  - **有输入参数时**，参数类型必须在签名或 lambda 头至少一处显式出现
  - Lambda 头可省略 → 如果签名已声明参数名与类型
  - 返回类型可显式标注，也可在可推断时省略

```yaoxiang
# 完整形式（签名完整 + Lambda头完整）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 简写：省略 Lambda 头（签名已声明参数）
add: (a: Int, b: Int) -> Int = a + b

# 简写：省略签名（lambda 头标注参数类型）
add = (a: Int, b: Int) => a + b

# ❌ 错误：两边都没标注参数类型
# add = (a, b) => a + b
```

### 设计目标

```yaoxiang
# === 完整形式 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 简写形式 ===
add: (a: Int, b: Int) -> Int = a + b                 # 省略 Lambda 头
add = (a: Int, b: Int) => a + b                      # 省略签名

# === 空参函数 ===
main: () -> Void = () => { println("Hello") }          # 完整形式
main: () -> Void = { println("Hello") }                # 省略 Lambda 头
main = { println("Hello") }                            # 最简形式（推断为 () -> Void）

# === 泛型函数（使用 RFC-011 语法）===
identity: [T](x: T) -> T = x                           # 完整形式
identity: [T](x: T) -> T = { x }                       # 省略 Lambda 头
identity = [T](x: T) => x                              # 省略签名

# === 递归函数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
```

### 语法规则

| 场景 | 语法 | 说明 |
|------|------|------|
| **完整形式** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | 签名 + Lambda 头完整 |
| **省略 Lambda 头** | `name: (a: Type, b: Type) -> Ret = { ... }` | 签名已声明参数 |
| **省略签名** | `name = (a: Type, b: Type) => { ... }` | lambda 头标注参数类型 |
| **空参完整** | `name: () -> Void = () => { return ... }` | 空参函数完整 |
| **空参简写** | `name: () -> Void = { return ... }` | 省略 Lambda 头 |
| **空参最简** | `name = { return ... }` | 无参无返最简 |

**注意**：块 `{ ... }` 的最后一个表达式作为返回值；需要提前结束时使用 `return`。空块 `{}` 推断为 `Void`。

**注意**：`->` 是函数类型的标志，不能省略（否则会被解析为元组）。

**重要**：`if` 表达式使用花括号 `{}` 包裹分支，不支持 `then/else` 关键字：
```yaoxiang
# 正确：使用花括号
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# 错误：不支持 then/else 关键字
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HM算法与高阶多态支持

**核心特性**：HM算法通过泛型类型注解支持高阶多态（Higher-rank polymorphism）

**设计原理**：
- **高阶函数**：函数作为参数传递时，需要泛型约束其函数类型
- **类型注解形式**：`[T](f: (T) -> T, x: T) -> T` - 泛型参数约束函数类型
- **HM工作流程**：通过泛型参数推断函数类型，实现多态函数组合

**示例说明**：
```yaoxiang
# ✅ 支持高阶多态：泛型约束函数类型参数
call_twice: [T](f: (x: T) -> T, x: T) -> T = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # 推断为 call_twice[Int]

compose: [A, B, C](f: (x: B) -> C, g: (x: A) -> B, x: A) -> C = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # 推断为 compose[Int, Int, Int]

# ❌ 不支持：缺少泛型约束的高阶函数
# bad_hof: (f, x) => f(f(x))  # HM无法推断，缺少泛型参数
```

**HM推断过程**：
1. 识别高阶函数参数：`f: (T) -> T`
2. 创建泛型约束：`[T]`
3. 通过泛型实例化推断具体类型
4. 实现多态函数组合

### Lambda 表达式语法规则

**重要规则**：代码块 `{ ... }` 返回其最后一个表达式的值；需要提前结束时使用 `return`。空块 `{}` 返回 `Void`。

| 语法形式 | 语法 | 返回方式 |
|---------|------|----------|
| **代码块形式** | `{ statements }` | 最后一个表达式作为返回值；可用 `return` 提前返回 |
| **表达式形式** | `expression` | 直接返回表达式值 |

**示例**：
```yaoxiang
main: () -> Void = { println("Hello") }         # 返回 Void（最后表达式为 println）
add: (a: Int, b: Int) -> Int = { a + b }        # 返回 Int（最后表达式为 a + b）
empty: () -> Void = {}                          # 空块返回 Void

# 提前返回：使用 return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)
}

# 表达式形式：直接返回值（无需 return）
add: (a: Int, b: Int) -> Int = a + b            # 正确：表达式形式
main: () -> Void = println("Hello")               # 正确：表达式形式
```

**核心思想**：
1. 函数定义通过HM算法进行类型推断，尽量推断，无法推断时显式报错
2. **HM算法工作原理**：通过操作符类型约束、函数调用关系等上下文信息自动推断类型
3. **泛型支持**：多态函数使用泛型语法 `[T]` 明确约束类型参数（RFC-011）
4. **推断边界**：返回类型与局部变量可推断；有参函数的参数类型需显式标注（签名或 lambda 头其一）
5. 空参无返回函数使用 `name: () -> Void = { ... }`，与 RFC-010 统一
6. 旧语法退役，提供迁移工具

**类型推断示例**：
```yaoxiang
# 泛型函数：显式类型参数（使用RFC-011语法）
identity: [T](x: T) -> T = x
map: [T, R](f: (T) -> R, list: List[T]) -> List[R] = List[R]

# 多态函数：通过显式泛型约束定义（RFC-011）
add: [T: Add](a: T, b: T) -> T = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # 推断为 (Int, Int) -> Void

# 高阶多态：通过泛型类型注解实现HM支持高阶多态
# 高阶函数接受函数作为参数时，需要泛型约束函数类型
call_twice: [T](f: (T) -> T, x: T) -> T = { return f(f(x)) }
compose: [A, B, C](f: (B) -> C, g: (A) -> B, x: A) -> C = { return f(g(x)) }
map: [T, R](f: (T) -> R, list: List[T]) -> List[R] = {
    result = List[R]()
    for item in list {
        result.push(f(item))
    }
    return result
}
```

```yaoxiang
# === 函数定义：HM算法类型推断 ===

# 标准函数：HM算法推断返回类型（参数类型需显式）
add = (a: Int, b: Int) => a + b            # 推断为 (a: Int, b: Int) -> Int
main = { println("Hello") }                # 推断为 () -> Void

# 部分显式参数：HM算法推断剩余部分
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # 推断为 (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # 推断为 (String) -> Void

# 泛型函数：明确约束多态类型参数（使用RFC-011语法）
identity: [T](x: T) -> T = x               # 推断为 T -> T
map: [T, R](f: (T) -> R, list: List[T]) -> List[R] = {
    # 实现 map 函数
    return List[R]()
}

# 递归函数：通过HM算法和递归约束推断
factorial: (n: Int) -> Int = {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

# === 变量赋值：HM算法类型推断 ===

# 显式类型
x: Int = 42

# HM算法自动推断为 Int
y = 42                               # 推断为 Int

# HM算法自动推断为 String
name = "YaoXiang"                    # 推断为 String

# HM算法自动推断为 Float
pi = 3.14159                         # 推断为 Float
```

**HM类型推断规则**：

| 场景 | 语法 | 可省略部分 | 示例 |
|------|------|----------|------|
| **完整形式** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | 无 | 签名 + Lambda 头完整 |
| **省略 Lambda 头** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda 头 | 签名已声明参数 |
| **省略签名** | `name = (a: Type, b: Type) => ...` | 签名 | lambda 头提供参数类型 |
| **省略返回 Ret** | `name: (a: Type, b: Type) -> = ...` | 返回类型 | HM 推断返回类型 |
| **空参完整** | `name: () -> Void = () => { ... }` | 无 | 空参函数完整 |
| **空参简写** | `name: () -> Void = { ... }` | Lambda 头 | 省略 `() =>` |
| **空参最简** | `name = { ... }` | 全部 | 无参无返最简 |
| **变量赋值** | `name = value` | 类型 | HM 推断类型 |
| **显式变量** | `name: Type = value` | 无 | 显式类型标注 |

**核心原则**：
- `->` 是函数类型的标志，不能省略（否则会被解析为元组）
- 返回类型 `Ret` 可省略，由 HM 根据函数体推断
- 当存在输入参数时，参数类型必须显式出现（在签名或 lambda 头其一）
- 其余部分在可推断且不引入歧义时可省略
- 无隐式类型转换，避免 JavaScript 式混乱

## 详细设计

### 语法糖展开

无论省略与否，最终都规范化到统一中间表示：

```rust
// 完整形式
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展开后 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// 省略 Lambda 头
add: (a: Int, b: Int) -> Int = a + b

// 展开后 IR（与完整形式相同）
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// 省略签名（lambda 头标注参数类型）
add = (a: Int, b: Int) => a + b

// 展开后 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### 语法定义

```bnf
function_def ::= identifier ':' type_expr '=' expression
               | identifier '=' expression
               | identifier ':' generic_params type_expr '=' expression
               | identifier '=' block                    # 最简形式：无参无返回

generic_params ::= '[' identifier (',' identifier)* ']'

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 类型引用
       | '()'                          # 空类型
       | '(' parameters ')' '->' type_expr   # 函数类型（参数名在签名中）
       | type_expr '->' type_expr            # 简单函数类型

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 类型推断
            | identifier ':' type_expr      # 部分显式类型
            | identifier ':' generic_type  # 泛型类型

generic_type ::= identifier            # 类型引用
               | '[' identifier ']'   # 泛型类型引用

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 赋值语句
           | expression                  # 表达式语句（执行但不返回）
           | 'return' expression         # 返回语句（返回指定值）

# 注意：代码块返回其最后一个表达式的值；空块 {} 推断为 Void
# 例如：{ 1 + 1 } 返回 Int；{ println("Hello") } 返回 Void
```

### 错误处理

```yaoxiang
# === 编译错误示例 ===

# 错误1：代码块返回类型不匹配
add: (a: Int, b: Int) -> Int = { println(a + b) }
// 错误：块的最后一个表达式是 println(...)，返回 Void，但签名期望 Int
// 正确：add: (a: Int, b: Int) -> Int = a + b
// 或者：add: (a: Int, b: Int) -> Int = { a + b }

# 错误2：使用未声明的类型参数
identity: (x: T) -> T = x
// 错误：T 未声明；需要显式泛型参数（RFC-011）
// 正确：identity: [T](x: T) -> T = x

# 正确：HM算法推断返回类型
double = (x: Int) => x + x

# 完整形式（逐步简写）
double: (x: Int) -> Int = (x) => x + x                # 完整
double: (x: Int) -> Int = x + x                       # 省略 Lambda 头
double = (x: Int) => x + x                            # 省略返回类型（HM 推断返回）
# double = (x) => x + x                               # ❌ 参数类型不允许两边都省略
```

## 权衡

### 优点

- **语法统一**：`name: Signature = LambdaBody` 模型覆盖所有场景
- **灵活简写**：任意部分可通过 HM 推断时即可省略
- **类型安全**：HM算法保证类型安全，避免隐式类型转换
- **递归支持**：HM算法和递归约束自动推断类型
- **零负担**：从完整到最简平滑过渡

### 缺点

- **迁移成本**：旧代码需迁移工具转换
- **学习成本**：需理解"完整形式 + 任意简写"模型

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| HM算法类型推断 | 使用Hindley-Milner算法推断类型 | ✅ **已采用**，现代函数式语言标准 |
| 显式类型声明 | 所有类型必须显式写 | 违反简化语法原则，增加样板代码 |
| 保留旧语法 | 同时支持新旧语法 | 语法分裂，维护成本高 |
| fn 关键字 | 引入 fn 区分函数和变量 | 违反"函数就是 lambda"的设计 |

## 实现策略

### 阶段划分

1. **Phase 1: 语法解析和HM算法**（v0.3）
   - 实现新语法 `name = lambda` + HM算法类型推断
   - 实现空参无返回的默认填充

2. **Phase 2: 迁移工具**（v0.3）
   - 开发 `yaoxiang-migrate --old-to-new` 工具
   - 自动转换旧语法代码

3. **Phase 3: 验证和文档**（v0.3）
   - 旧代码迁移完成验证
   - 文档更新

### 迁移工具

```bash
# 迁移单个文件
yaoxiang-migrate --old-to-new src/main.yaoxiang

# 迁移整个项目
yaoxiang-migrate --old-to-new --recursive src/

# 预览迁移（不修改文件）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

迁移规则：
```yaoxiang
# 旧语法
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新语法：完整形式（签名完整 + Lambda 头完整）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 简写：省略 Lambda 头 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 简写：HM 推断 ===
add = (a: Int, b: Int) => a + b              # 推断为 (a: Int, b: Int) -> Int
main = { println("Hello") }                  # 推断为 () -> Void

# === 最简形式 ===
main = {                                      # 等价于 main: () -> Void = { ... }
    println("Hello")
}
```

### 依赖关系

- 无外部依赖
- 可独立实现

### 风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 迁移遗漏 | 旧代码编译失败 | 提供迁移工具，覆盖所有旧语法模式 |
| 解析器错误 | 语法解析不稳定 | 充分的测试覆盖 |

## 开放问题

> 以下问题已在设计中解决，记录在附录A。

- ~~Q1: 是否应该保留 `main() = body` 这种极简写法？~~ → 已解决：保留为 `main = { ... }`
- ~~Q2: 函数名后的 `:` 是否保留？~~ → 已解决：可选保留；但有参函数仍需在签名或 lambda 头标注参数类型
- ~~Q3: HM算法是否支持参数类型推断？~~ → 已解决：返回值/局部可推断；有参函数的参数类型需显式标注
- ~~Q4: 是否引入 `fn` 关键字？~~ → 已解决：不引入，函数就是 lambda
- ~~Q5: 旧代码的迁移策略是什么？~~ → 已解决：提供 `yaoxiang-migrate` 工具
- ~~Q6: 泛型函数如何使用？~~ → 已解决：使用RFC-011语法 `[T]`

---

## 附录

### 附录A：各语言函数定义语法参考

| 语言 | 语法风格 | 特点 |
|------|---------|------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | 关键字 + 类型标注 |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | 类型签名分离 |
| OCaml | `let add a b = ...` | 参数类型可省略 |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | 简洁类型标注 |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda 风格 |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def 关键字 |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **函数 = lambda，HM 推断返回值** |

### 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 语法风格 | 新语法 `name: (params) -> Return = body` + HM推断 | 2026-02-03 | @沫郁酱 |
| 参数位置 | 参数名在签名中声明，与 RFC-010 统一 | 2026-02-03 | @沫郁酱 |
| 默认填充 | 空参函数可省略签名，空块 `{}` 推断为 `Void` | 2026-02-03 | @沫郁酱 |
| 类型推断 | HM算法自动推断，无法推断时显式 | 2026-01-06 | @沫郁酱 |
| 旧语法 | 退役，提供迁移工具 | 2026-01-06 | @沫郁酱 |
| fn 关键字 | 不引入 | 2026-01-06 | @沫郁酱 |
| 递归声明 | HM算法和递归约束自动推断 | 2026-01-06 | @沫郁酱 |

### 附录C：术语表

| 术语 | 定义 |
|------|------|
| HM算法 | Hindley-Milner类型推断算法，自动推断函数和变量类型 |
| 泛型 | 使用类型参数 `[T]` 约束多态函数，如 `identity = [T](x: T) => x`（RFC-011） |
| 默认类型填充 | 空参无返回函数省略 `-> Void`，编译器自动填充 |
| 语法糖 | 使代码更易读的语法简化写法 |
| 规范化 | 将语法形式转换为统一内部表示 |
| 函数即 lambda | 函数本质是 lambda 变量，类型通过HM算法自动推断 |

---

## 参考文献

- [MoonBit 语言设计](https://moonbitlang.com/)
- [Rust 函数语法](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 类型系统](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 类型推断](https://v2.ocaml.org/manual/)
