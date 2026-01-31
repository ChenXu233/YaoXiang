# RFC-007: 函数定义语法统一方案

> **状态**: 已接受
> **作者**: 沫郁酱
> **创建日期**: 2025-01-05
> **最后更新**: 2026-01-06

## 摘要

本 RFC 确定 YaoXiang 语言**函数定义语法**的最终方案。确定使用新语法 `name = lambda`，通过HM算法自动推断类型，支持部分显式类型标注，空参无返回函数可省略为 `name = () => { ... }`（默认填充 `-> Void`），旧语法退役。

## 动机

### 为什么需要这个特性？

1. **语法一致性**：消除旧语法的历史包袱，统一风格
2. **简洁性**：HM算法自动推断类型，减少样板代码
3. **类型安全**：HM算法保证类型安全，无法推断时才显式标注
4. **语言成熟度**：HM算法是现代函数式语言的成熟方案

### 设计目标

```yaoxiang
# === 最终确定的语法 ===

# 标准形式：HM算法类型推断
add = (a, b) => { return a + b }
main = () => { println("Hello") }

# 省略形式：空参无返回函数，默认填充 () -> Void
main = () => {                    # 等价于 main:() -> Void
    println("Hello")
}

# 泛型函数：明确约束多态类型参数（使用RFC-011语法）
identity = [T](x: T) => { return x }
map = [T, R](f: (T) -> R, list: List[T]) => {
    # 实现 map 函数
    return List[R]()
}

# 函数就是 lambda，可通过签名实现递归
factorial = (n) => {
    if n <= 1 then { return 1 } else { return n * factorial(n - 1) }
}
```

### 语法规则

| 场景 | 语法 | 说明 |
|------|------|------|
| **HM推断函数** | `name = (a, b) => ...` | HM算法自动推断类型 |
| **显式类型函数** | `name: (T1, T2) -> T3 = (a, b) => ...` | 显式标注类型（可选） |
| **泛型函数** | `name = [T](x: T) => ...` | 使用RFC-011泛型语法 |
| **空参无返回** | `name = () => { ... }` | 默认填充 `-> Void` |
| **递归函数** | `name = (params) => { ... name(...) ... }` | HM算法通过递归约束推断 |

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
call_twice = [T](f: (T) -> T, x: T) => {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # 推断为 call_twice[Int]

compose = [A, B, C](f: (B) -> C, g: (A) -> B, x: A) => {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # 推断为 compose[Int, Int, Int]

# ❌ 不支持：缺少泛型约束的高阶函数
# bad_hof = (f, x) => f(f(x))  # HM无法推断，缺少泛型参数
```

**HM推断过程**：
1. 识别高阶函数参数：`f: (T) -> T`
2. 创建泛型约束：`[T]`
3. 通过泛型实例化推断具体类型
4. 实现多态函数组合

### Lambda 表达式语法规则

**重要规则**：Lambda表达式的返回规则取决于使用的语法形式：

| 语法形式 | 语法 | 返回方式 | 示例 |
|---------|------|----------|------|
| **代码块形式** | `(params) => { statements }` | **默认返回Void，显式return返回指定值** | `(a, b) => { println(a + b) }` 返回Void<br>`(a, b) => { return a + b }` 返回Int |
| **表达式形式** | `(params) => expression` | **直接返回表达式值** | `(a, b) => a + b` |

**示例对比**：
```yaoxiang
# 正确：HM算法完全推断
add = (a, b) => { return a + b }     # 推断为多态函数，支持所有Add类型
print_sum = (a, b) => { println(a + b) }  # 推断为多态函数，支持所有Add类型，返回Void

# 正确：HM算法部分推断（参数显式，返回推断）
print_sum = (a:Int, b:Int) => { println(a + b) }  # 推断为 (Int, Int) -> Void
main = () => { println("Hello") }    # 推断为 () -> Void

# 正确：泛型Lambda表达式
identity_lambda = [T](x: T) => { return x }  # 推断为 T -> T
map_lambda = [T, R](f: (T) -> R, list: List[T]) => List[R]()  # 推断为 (T -> R) -> List[T] -> List[R]

# 正确：代码块形式（返回指定值）
main = () => { println("Hello"); return 0 }  # 推断为 () -> Int

# 正确：代码块形式（最后表达式返回值）
main = () => { println("Hello"); 0 }  # 推断为 () -> Int（最后表达式0作为返回值）

# 正确：表达式形式直接返回值
add = (a, b) => a + b               # 推断为多态函数 [T: Add](T, T) -> T
# 表达式形式：直接返回表达式值
main = () => 0                       # 推断为 () -> Int

# 错误：代码块形式返回类型不匹配
# 场景1：变量类型期望返回Int，但函数返回Void
add: (Int, Int) -> Int = (a, b) => { a + b }
# 错误：代码块返回Void，但变量类型期望返回Int

# 正确写法：
# 1. 使用显式return
add: (Int, Int) -> Int = (a, b) => { return a + b }

# 2. 表达式形式（直接返回值）
add: (Int, Int) -> Int = (a, b) => a + b

# 3. 让编译器自动推断（代码块最后表达式返回值）
add = (a, b) => { a + b }  # 推断为多态函数 [T: Add](T, T) -> T

# 4. 完整表达式形式
add = (a, b) => a + b     # 推断为多态函数 [T: Add](T, T) -> T
```

**核心思想**：
1. 函数定义通过HM算法进行类型推断，尽量推断，无法推断时显式报错
2. **HM算法工作原理**：通过操作符类型约束、函数调用关系等上下文信息自动推断类型
3. **泛型支持**：多态函数使用泛型语法 `[T]` 明确约束类型参数（RFC-011）
4. **部分显式支持**：参数可选择性标注类型，HM算法推断剩余部分
5. 空参无返回函数可省略为 `name = () => { ... }`，编译器自动填充 `-> Void`
6. 旧语法退役，提供迁移工具

**类型推断示例**：
```yaoxiang
# 泛型函数：显式类型参数（使用RFC-011语法）
identity = [T](x: T) => { return x }
map = [T, R](f: (T) -> R, list: List[T]) => List[R]

# HM算法推断：通过操作符类型约束推断多态函数
add = (a, b) => a + b  # 推断为多态函数 [T: Add](T, T) -> T
print_sum = (a:Int, b:Int) => { println(a + b) }  # 推断为 (Int, Int) -> Void（明确参数类型）

# 高阶多态：通过泛型类型注解实现HM支持高阶多态
# 高阶函数接受函数作为参数时，需要泛型约束函数类型
call_twice = [T](f: (T) -> T, x: T) => { return f(f(x)) }
compose = [A, B, C](f: (B) -> C, g: (A) -> B, x: A) => { return f(g(x)) }
map = [T, R](f: (T) -> R, list: List[T]) => {
    result = List[R]()
    for item in list {
        result.push(f(item))
    }
    return result
}
```

```yaoxiang
# === 函数定义：HM算法类型推断 ===

# 标准函数：HM算法推断参数和返回类型
add = (a, b) => { return a + b }     # 推断为多态函数 [T: Add](T, T) -> T
main = () => { println("Hello") }    # 推断为 () -> Void

# 部分显式参数：HM算法推断剩余部分
print_sum = (a:Int, b:Int) => { println(a + b) }  # 推断为 (Int, Int) -> Void
greet = (name:String) => { println("Hello " + name) }  # 推断为 (String) -> Void

# 泛型函数：明确约束多态类型参数（使用RFC-011语法）
identity = [T](x: T) => { return x }     # 推断为 T -> T
map = [T, R](f: (T) -> R, list: List[T]) => {
    # 实现 map 函数
    return List[R]()
}

# 递归函数：通过HM算法和递归约束推断
factorial = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}                                      # 推断为 (Int) -> Int

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

| 场景 | 语法 | 类型推断 | 示例 |
|------|------|----------|------|
| **函数定义** | `name = (a, b) => ...` | ✅ HM算法完全推断 | `add = (a, b) => a + b` → `[T: Add](T, T) -> T` |
| **泛型函数** | `name = [T](x: T) => ...` | ✅ 泛型约束，多态 | `identity = [T](x: T) => x` |
| **部分显式参数** | `name = (a:Type, b) => ...` | ✅ 参数显式，返回推断 | `print_sum = (a:Int, b) => { println(a + b) }` |
| **空参无返回** | `name = () => { ... }` | ✅ 默认填充 `-> Void` | `main = () => { ... }` |
| **变量赋值** | `name = value` | ✅ HM算法自动推断 | `y = 42` → `Int` |
| **显式类型** | `name: Type = value` | ✅ 使用显式类型 | `x: Int = 42` |

**核心原则**：
- **HM算法优先**：尽量通过HM算法推断类型，简化语法
- **无法推断时显式**：无法通过HM推断的类型必须显式标注
- 无隐式类型转换，避免 JavaScript 式混乱

## 详细设计

### 语法糖展开

无论省略与否，最终都规范化到统一中间表示：

```rust
// 代码块形式（返回Void）
print_sum:(Int, Int) -> Void = (a, b) => { println(a + b) }

// 展开后 IR
let print_sum:(Int, Int) -> Void = |a: Int, b: Int| -> Void {
    println(a + b)
};

// 省略形式
main = () => { println("Hello") }

// 展开后 IR
let main:() -> Void = |()| -> Void {
    println("Hello")
};

// 完整形式
add:(Int, Int) -> Int = (a, b) => { return a + b }

// 展开后 IR
let add:(Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};
```

### 语法定义

```bnf
function_def ::= identifier '=' expression
               | identifier ':' type '=' expression
               | identifier ':' generic_params type '=' expression

generic_params ::= '[' identifier (',' identifier)* ']'

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type ::= identifier                     # 类型引用
       | '()'                          # 空类型
       | type '->' type                # 函数类型
       | '(' type (',' type)* ')' '->' type  # 多参数函数

expression ::= '|' parameters '|' '->' type '=>' block
             | '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 类型推断
            | identifier ':' type      # 部分显式类型
            | identifier ':' generic_type  # 泛型类型

generic_type ::= identifier            # 类型引用
               | '[' identifier ']'   # 泛型类型引用

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 赋值语句
           | expression                  # 表达式语句（执行但不返回）
           | 'return' expression         # 返回语句（返回指定值）

# 注意：代码块如果没有return语句，默认返回Void
```

### 错误处理

```yaoxiang
# === 编译错误示例 ===

# 错误1：代码块形式返回类型不匹配
add:(Int, Int) -> Int = (a, b) => { a + b }
// 错误：代码块默认返回Void，但函数签名期望返回Int
// 正确：add:(Int, Int) -> Int = (a, b) => { return a + b }
// 或者：add:(Int, Int) -> Int = (a, b) => a + b
// 或者：add = (a, b) => { a + b }  # 推断为多态函数 [T: Add](T, T) -> T

# 错误2：多态函数需要泛型语法约束
identity = (x) => { return x }
// 错误：多态函数需要使用泛型语法明确约束类型参数（RFC-011语法）
// 正确：identity = [T](x: T) => { return x }  # 使用泛型T约束
// 或者：identity = (x:Int) => { return x }  # 指定具体类型（单态化）

# 错误3：多态函数缺少类型约束（纯多态）
identity = (x) => { return x }
// 错误：纯多态函数无法通过HM推断具体类型
// 正确：identity = [T](x: T) => { return x }  # 使用泛型语法约束

# 正确：HM算法通过操作符约束推断多态
double = (x) => { return x + x }  # 推断为 [T: Add](T, T) -> T
```

## 权衡

### 优点

- **语法统一**：HM算法推断类型，新语法 `name = lambda` 一致风格
- **简洁性**：空参无返回函数省略 `-> Void`，类型自动推断
- **类型安全**：HM算法保证类型安全，避免隐式类型转换
- **递归支持**：HM算法和递归约束自动推断类型
- **无推断歧义**：函数和 lambda 变量统一，函数就是 lambda

### 缺点

- **迁移成本**：旧代码需迁移工具转换
- **学习成本**：需理解"默认填充"规则

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

# 新语法（HM算法可以推断类型）
add = (a, b) => { return a + b }     # 推断为 [T: Add](T, T) -> T
main = () => { println("Hello"); return 0 }  # 推断为 () -> Int
main = () => { println("Hello") }     # 推断为 () -> Void

# 或者保留显式类型标注（与旧语法对应）
add:(Int, Int) -> Int = (a, b) => { return a + b }
main:() -> Int = () => { println("Hello"); return 0 }
main:() -> Void = () => { println("Hello") }
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

- ~~Q1: 是否应该保留 `main() = body` 这种极简写法？~~ → 已解决：保留为 `main = () => { ... }`
- ~~Q2: 函数名后的 `:` 是否保留？~~ → 已解决：可选保留，HM算法可完全推断
- ~~Q3: HM算法是否支持参数类型推断？~~ → 已解决：支持，通过操作符约束和上下文推断
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
| **YaoXiang** | `name = (a, b) => { ... }` | **函数 = lambda，HM算法类型推断** |

### 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 语法风格 | 新语法 `name = lambda` + HM算法推断 | 2026-01-06 | @沫郁酱 |
| 默认填充 | 空参无返回默认填充 `-> Void` | 2026-01-06 | @沫郁酱 |
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
