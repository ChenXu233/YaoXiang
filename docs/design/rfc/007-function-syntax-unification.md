# RFC-007: 函数定义语法统一方案

> **状态**: 审核中
> **作者**: 沫郁酱
> **创建日期**: 2025-01-05
> **最后更新**: 2026-01-06

## 摘要

本 RFC 确定 YaoXiang 语言**函数定义语法**的最终方案。确定使用新语法 `name:(params) -> type = lambda`，空参无返回函数可省略为 `name = () => { ... }`（默认填充 `-> Void`），旧语法退役。

## 动机

### 为什么需要这个特性？

1. **语法一致性**：消除旧语法的历史包袱，统一风格
2. **简洁性**：常见模式（空参无返回）可省略，避免样板代码
3. **类型安全**：一切皆类型，不引入隐式推断
4. **语言成熟度**：统一语法是语言走向成熟的重要标志

### 设计目标

```yaoxiang
# === 最终确定的语法 ===

# 标准形式：完整签名
add:(Int, Int) -> Int = (a, b) => { a + b }
main:() -> Int = () => { println("Hello"); 0 }

# 省略形式：空参无返回函数，默认填充 () -> Void
main = () => {                    # 等价于 main:() -> Void
    println("Hello")
}

# 函数就是 lambda，可通过签名实现递归
factorial:(Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}
```

### 语法规则

| 场景 | 语法 | 说明 |
|------|------|------|
| 完整函数 | `name:(T1, T2) -> T3 = (a, b) => { ... }` | 必须写签名 |
| 空参无返回 | `name = () => { ... }` | 默认填充 `-> Void` |
| 递归函数 | `name:Type = (params) => { ... name(...) ... }` | 通过签名声明递归 |

## 提案

### 核心设计：默认类型填充

**核心思想**：
1. 所有函数必须写完整签名 `name:(params) -> type = lambda`
2. 空参无返回函数可省略为 `name = () => { ... }`，编译器自动填充 `-> Void`
3. **变量支持类型推断**：普通变量赋值可省略类型，由编译器推断
4. 函数参数和返回类型必须显式（一切皆类型）
5. 旧语法退役，提供迁移工具

```yaoxiang
# === 函数定义：必须写签名 ===

# 标准函数定义
add:(Int, Int) -> Int = (a, b) => { a + b }
main:() -> Int = () => { println("Hello"); 0 }

# 空参无返回：可省略 -> Void
main = () => {                    # 编译器填充为 main:() -> Void
    println("Hello")
}

# 递归函数：通过签名声明
factorial:(Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}

# === 变量赋值：支持类型推断 ===

# 显式类型
x: Int = 42

# 编译器自动推断为 Int
y = 42                               # 推断为 Int

# 编译器自动推断为 String
name = "YaoXiang"                    # 推断为 String

# 编译器自动推断为 Float
pi = 3.14159                         # 推断为 Float
```

**类型填充规则**：

| 场景 | 语法 | 类型推断 | 示例 |
|------|------|----------|------|
| **函数定义** | `name:(T1, T2) -> T3 = ...` | ❌ 不推断，必须显式 | `add:(Int, Int) -> Int = ...` |
| **空参无返回** | `name = () => { ... }` | ✅ 默认填充 `-> Void` | `main = () => { ... }` |
| **变量赋值** | `name = value` | ✅ 编译器自动推断 | `y = 42` → `Int` |
| **变量赋值** | `name: Type = value` | ✅ 使用显式类型 | `x: Int = 42` |

**核心原则**：
- **函数签名**：一切皆类型，必须显式写
- **变量赋值**：简化语法，编译器自动推断
- 无隐式类型转换，避免 JavaScript 式混乱

## 详细设计

### 语法糖展开

无论省略与否，最终都规范化到统一中间表示：

```rust
// 省略形式
main = () => { println("Hello") }

// 展开后 IR
let main:() -> Void = |()| -> Void {
    println("Hello")
};

// 完整形式
add:(Int, Int) -> Int = (a, b) => { a + b }

// 展开后 IR
let add:(Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### 语法定义

```bnf
function_def ::= identifier ':' type '=' expression

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type ::= '()'                           // 空类型
       | type '->' type                 // 函数类型
       | '(' type (',' type)* ')' '->' type  // 多参数函数
       | identifier                     // 类型引用

expression ::= '|' parameters '|' '->' type '=>' block
             | '(' parameters ')' '=>' block
             | '(' ')' '=>' block

parameters ::= parameter (',' parameter)*
parameter ::= identifier [':' type]

block ::= '{' expression (',' expression)* '}'
        | expression
```

### 错误处理

```yaoxiang
# === 编译错误示例 ===

# 错误1：省略了必须写的签名
add = (a, b) => { a + b }
// 错误：无法推断参数类型，请显式指定：
// add:(Int, Int) -> Int = ...

# 错误2：参数类型不完整
add:(Int, _) -> Int = (a, b) => { a + b }
// 错误：参数类型必须完整，或使用占位符
```

## 权衡

### 优点

- **语法统一**：新语法 `name:(params) -> type` 一致风格
- **简洁性**：空参无返回函数省略 `-> Void`
- **类型安全**：一切皆类型，无隐式推断
- **递归支持**：通过签名声明，清晰无歧义
- **无推断歧义**：函数和 lambda 变量统一，函数就是 lambda

### 缺点

- **迁移成本**：旧代码需迁移工具转换
- **学习成本**：需理解"默认填充"规则

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| 函数参数类型推断 | 让编译器推断函数参数类型 | 会引入 JavaScript 式混乱，违反"一切皆类型" |
| 变量类型推断 | 编译器自动推断变量类型 | ✅ **已采用**，简化语法 |
| 保留旧语法 | 同时支持新旧语法 | 语法分裂，维护成本高 |
| fn 关键字 | 引入 fn 区分函数和变量 | 违反"函数就是 lambda"的设计 |

## 实现策略

### 阶段划分

1. **Phase 1: 语法解析**（v0.3）
   - 实现新语法 `name:(params) -> type = lambda`
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

# 新语法
add:(Int, Int) -> Int = (a, b) => { a + b }
main:() -> Int = () => { println("Hello"); 0 }
main = () => { println("Hello") }
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
- ~~Q2: 函数名后的 `:` 是否保留？~~ → 已解决：保留 `name:(params) -> type`
- ~~Q3: 自动推断是否应该支持省略参数类型？~~ → 已解决：不支持类型推断，仅空参无返回可默认填充
- ~~Q4: 是否引入 `fn` 关键字？~~ → 已解决：不引入，函数就是 lambda
- ~~Q5: 旧代码的迁移策略是什么？~~ → 已解决：提供 `yaoxiang-migrate` 工具
- ~~Q6: 推断失败时应该报错还是警告？~~ → 已解决：不推断，必须显式类型

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
| **YaoXiang** | `name:(T1, T2) -> T3 = (a, b) => { ... }` | **函数 = lambda，默认类型填充** |

### 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 语法风格 | 新语法 `name:(params) -> type = lambda` | 2026-01-06 | @沫郁酱 |
| 默认填充 | 空参无返回默认填充 `-> Void` | 2026-01-06 | @沫郁酱 |
| 类型推断 | 不支持，仅默认填充 | 2026-01-06 | @沫郁酱 |
| 旧语法 | 退役，提供迁移工具 | 2026-01-06 | @沫郁酱 |
| fn 关键字 | 不引入 | 2026-01-06 | @沫郁酱 |
| 递归声明 | 通过签名声明 | 2026-01-06 | @沫郁酱 |

### 附录C：术语表

| 术语 | 定义 |
|------|------|
| 函数签名 | 函数的参数类型和返回类型完整描述 `name:(T1, T2) -> T3` |
| 默认类型填充 | 空参无返回函数省略 `-> Void`，编译器自动填充 |
| 语法糖 | 使代码更易读的语法简化写法 |
| 规范化 | 将语法形式转换为统一内部表示 |
| 函数即 lambda | 函数本质是 lambda 变量，可通过签名实现递归 |

---

## 参考文献

- [MoonBit 语言设计](https://moonbitlang.com/)
- [Rust 函数语法](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 类型系统](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 类型推断](https://v2.ocaml.org/manual/)
