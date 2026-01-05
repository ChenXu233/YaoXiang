# RFC-007: 函数定义语法统一方案

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2025-01-05

> **讨论中**: 本 RFC 旨在收集社区意见，确定最终语法方案。

## 摘要

本 RFC 讨论 YaoXiang 语言中**函数定义语法**的统一问题。当前存在新旧两种语法风格，且在无参无返回函数处理上存在不统一性。我们需要确定：是保留旧语法兼容、还是强制迁移新语法、或是引入更智能的自动推断方案。

## 动机

### 为什么需要这个特性？

1. **语法一致性**：当前存在两种函数定义风格，造成学习困惑
2. **新用户友好**：简洁直观的语法降低入门门槛
3. **代码美观性**：避免冗余语法符号
4. **语言成熟度**：统一语法是语言走向成熟的重要标志

### 当前的问题

#### 问题描述

```yaoxiang
# === 当前存在的两种语法 ===

# 旧语法（面向过程风格）
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = {
    println("Hello")
    0
}

# 新语法（函数式风格）
add:(Int, Int) -> Int = (a, b) => { a + b }
main:() -> Int = () => {
    println("Hello")
    0
}

# === 问题 1：无参函数的不统一 ===

# 旧语法可以省略：
main() = {                    # ✓ 可以省略返回类型和参数
    println("Hello")
    0
}

# 新语法必须完整：
main:() -> Int = () => {      # ✗ 必须写 ()、->、类型
    println("Hello")
    0
}

# === 问题 2：语法风格分裂 ===

# 混合使用时会显得混乱
calculate:(Int) -> Int = (x) => { x * 2 }  # 新语法
calculate(Int) -> Int = (x) => { x * 2 }    # 旧语法

# 应该统一成一种风格！
```

#### MoonBit 的参考

MoonBit（类似定位的语言）采用的设计：
```moonbit
// 所有函数必须有签名
fn add(a: Int, b: Int) -> Int {
    a + b
}

fn main() {
    // ...
}
```

### 现有语法对比

| 场景 | 旧语法 | 新语法 | 问题 |
|------|--------|--------|------|
| 有参有返回 | `f(Int)->Int = ...` | `f:(Int)->Int = ...` | 冒号差异 |
| 无参有返回 | `f()->Int = ...` | `f:()->Int = ...` | 冒号差异 |
| 无参无返回 | `f() = ...` | `f:()→() = ...` | **新语法必须写签名** |
| 推断类型 | 不支持 | 不支持 | 两边都不支持 |

## 提案

以下提供 **4 种备选方案**，请社区讨论选择。

### 方案 A：保留旧语法兼容（保守方案）

**核心思想**：保留所有旧语法，新语法作为可选项。

```yaoxiang
# === 方案 A：全部支持 ===

# 旧语法（完全保留）
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = {
    println("Hello")
    0
}

# 简化版（无参无返回）
main() = {
    println("Hello")
    0
}

# 新语法（可选）
add:(Int, Int) -> Int = (a, b) => { a + b }
main:() -> Int = () => { println("Hello"); 0 }

# 使用规则：
# 1. 有参数函数：旧语法 `name(params) -> type`
# 2. 无参无返回：旧简化语法 `name() = body`
# 3. 新语法 `name:(params) -> type` 作为等价变体
```

**优点**：
- 完全向后兼容
- 无需修改现有代码
- 降低迁移成本

**缺点**：
- 语法分裂加剧
- 新用户困惑
- 编译器实现更复杂

### 方案 B：统一为新语法（激进方案）

**核心思想**：强制使用新语法 `name:(params) -> type`，旧语法标记为弃用。

```yaoxiang
# === 方案 B：新语法强制 ===

# 标准新语法
add:(Int, Int) -> Int = (a, b) => { a + b }
main:() -> Int = () => {
    println("Hello")
    0
}

# 特殊情况：无参无返回可以简化为
main:() = () => {
    println("Hello")
    0
}

# 或者进一步简化（待讨论）
main = () => {  # 推断为 () -> Int
    println("Hello")
    0
}
```

**迁移策略**：
```bash
# 提供自动化迁移工具
cargo yaoxiang-migrate --old-syntax-to-new
```

**优点**：
- 语法高度统一
- 与函数式语言风格一致
- 编译器实现简单

**缺点**：
- 需要迁移现有代码
- `main() = body` 的简洁性丧失
- 社区可能反对

### 方案 C：智能自动推断（推荐方案）

**核心思想**：根据赋值右侧的 Lambda 自动推断函数签名，左侧可简化。

```yaoxiang
# === 方案 C：自动推断 ===

# 基本规则：根据右侧推断左侧签名

# 1. 无参函数
main = () => {              # 推断为 main:() -> Int
    println("Hello")
    0
}

# 2. 有参函数（参数类型可省略）
add = (a: Int, b: Int) => {  # 推断为 add:(Int, Int) -> Int
    a + b
}

# 3. 完整签名（可省略部分）
add:(Int, Int) -> Int = (a, b) => { a + b }  # 可省略参数类型
add:(Int, _) -> Int = (a, b) => { a + b }    # 部分省略

# 4. 纯类型签名（左侧省略）
# 两种写法等价：
add = (a: Int, b: Int) => { a + b }
add:(Int, Int) -> Int = (a, b) => { a + b }
```

**推断规则**：

| 左侧形式 | 右侧 Lambda | 推断结果 |
|---------|------------|---------|
| `name = lambda` | `() => body` | `name:() -> InferredType` |
| `name = lambda` | `(a) => body` | `name:(InferredType) -> InferredType` |
| `name = lambda` | `(a, b) => body` | `name:(InferredType, InferredType) -> InferredType` |
| `name:Type = lambda` | `(a) => body` | `name:(Type) -> InferredType` |
| `name:()->Type = lambda` | `() => body` | `name:()->Type` |

**优点**：
- 语法最简洁
- 类型安全不打折
- 学习成本低

**缺点**：
- 语法解析器更复杂
- 需要类型推断引擎支持
- 错误信息需要清晰

### 方案 D：混合语法糖（MoonBit 风格）

**核心思想**：引入 `fn` 关键字，区分函数定义和普通赋值。

```yaoxiang
# === 方案 D：fn 关键字 ===

# 函数定义（必须完整）
fn add(a: Int, b: Int): Int {       # 类似 MoonBit 风格
    a + b
}

fn main(): Int {
    println("Hello")
    0
}

# 简化版（有类型推断）
fn add(a, b) = a + b                 # 参数类型可省略
fn main() = { println("Hello"); 0 }  # 返回类型可省略

# 普通赋值（非函数）
x = 42
y = (a, b) => a + b                  # 这是一个 lambda 变量，不是函数
```

**语法定义**：

```bnf
function_def ::= 'fn' identifier parameters [':' type] ['='] expression
               | identifier '=' expression              // 函数推断
               | identifier parameters '->' type '=' expression  // 旧语法（兼容）

parameters ::= '(' [parameter_list] ')'
parameter_list ::= parameter (',' parameter)*
parameter ::= identifier [':' type]
```

**优点**：
- 语义清晰（函数 vs 变量）
- 符合直觉
- 可渐进迁移

**缺点**：
- 引入新关键字
- 与当前语法差异大
- 需要大改解析器

## 详细设计

### 语法糖展开

无论采用哪种方案，最终都需要规范化到中间表示：

```rust
// 方案 C 的展开示例

// 源码
add = (a: Int, b: Int) => { a + b }

// 展开后 IR
let add:(Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### 类型推断算法

```rust
fn infer_function_signature(lambda: &Lambda) -> Type {
    match lambda {
        Lambda { params, body } => {
            let param_types: Vec<Type> = params
                .iter()
                .map(|p| {
                    p.type_
                        .clone()
                        .unwrap_or_else(|| infer_from_body(p.name, body))
                })
                .collect();

            let return_type = infer_type(body);

            Type::Function(param_types, Box::new(return_type))
        }
    }
}
```

### 错误处理

```yaoxiang
# === 推断失败示例 ===

# 无法推断参数类型（必须显式指定）
add = (a, b) => { a + b }
// 错误：无法推断参数类型，请显式指定：
// add:(Int, Int) -> Int = ...

# 返回类型无法推断
main = () => { println("Hello") }
// 警告：返回类型推断为 ()，是否需要指定为 main:() -> ()？
```

## 权衡

### 方案对比

| 维度 | A（兼容） | B（强制新语法） | C（自动推断） | D（fn 关键字） |
|------|----------|----------------|---------------|---------------|
| 兼容性 | ★★★★★ | ★★☆☆☆ | ★★★☆☆ | ★★☆☆☆ |
| 简洁性 | ★★☆☆☆ | ★★★☆☆ | ★★★★★ | ★★★★☆ |
| 实现难度 | ★★☆☆☆ | ★★☆☆☆ | ★★★★☆ | ★★★☆☆ |
| 学习成本 | ★★☆☆☆ | ★★★☆☆ | ★★★★★ | ★★★☆☆ |
| 统一性 | ★☆☆☆☆ | ★★★★★ | ★★★★★ | ★★★★★ |

### 推荐度（沫郁酱的个人建议）

```
方案 C（自动推断）> 方案 D（fn 关键字）> 方案 B（强制新语法）> 方案 A（兼容）
```

**理由**：
1. 方案 C 语法最简洁，类型安全不打折
2. 符合 "让编译器做更多工作" 的现代语言设计理念
3. 渐进式学习：先学简单写法，再学完整签名

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| 保持现状 | 不做统一 | 长期技术债务 |
| 仅保留新语法 | 删除旧语法 | 破坏性变更 |
| 引入新关键字 | 类似 Rust `fn` | 增加语法复杂度 |
| 运行时推断 | 编译时不检查 | 违反类型安全 |

## 实现策略

### 阶段划分

1. **Phase 1: 语法解析增强**（v0.3）
   - 扩展语法解析器支持多种形式
   - 添加类型推断基础设施

2. **Phase 2: 自动推断实现**（v0.4）
   - 实现完整的类型推断算法
   - 添加推断失败的用户提示

3. **Phase 3: 工具支持**（v0.5）
   - 开发迁移工具（可选）
   - IDE 智能提示集成
   - 文档更新

### 依赖关系

- 依赖 RFC-004（柯里化绑定）的类型系统
- 可能需要扩展类型推断引擎

### 风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 推断不准确 | 类型错误 | 提供手动覆盖语法 |
| 性能开销 | 编译变慢 | 缓存推断结果 |
| 用户困惑 | 学习曲线 | 完善文档和示例 |

## 开放问题

> **重要**：请社区讨论以下问题，在 RFC 审核阶段确定最终方案。

- [ ] **Q1**: 是否应该保留 `main() = body` 这种极简写法？
- [ ] **Q2**: 函数名后的 `:` 是否保留？（方案 C 是否需要 `name:Type` 前缀）
- [ ] **Q3**: 自动推断是否应该支持省略参数类型？
- [ ] **Q4**: 是否引入 `fn` 关键字？（方案 D）
- [ ] **Q5**: 旧代码的迁移策略是什么？
- [ ] **Q6**: 推断失败时应该报错还是警告？

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

### 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 语法统一 | 待定 | - | - |
| 推断范围 | 待定 | - | - |
| 关键字 | 待定 | - | - |

### 附录C：术语表

| 术语 | 定义 |
|------|------|
| 类型推断 | 编译器自动推导变量或表达式类型的能力 |
| 函数签名 | 函数的参数类型和返回类型完整描述 |
| 语法糖 | 使代码更易读的语法简化写法 |
| 规范化 | 将多种语法形式转换为统一内部表示 |

---

## 参考文献

- [MoonBit 语言设计](https://moonbitlang.com/)
- [Rust 函数语法](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 类型推断](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 类型推断](https://v2.ocaml.org/manual inference.html)
