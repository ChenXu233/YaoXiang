# YaoXiang（爻象）设计宣言

> **版本**：v2.0.0
> **状态**：正式发布
> **作者**：晨煦 + YaoXiang 社区
> **日期**：2026-05-31

---

> 「道生一，一生二，二生三，三生万物。」
> —— 《道德经》
>
> 类型如道，万物皆由此生。

---

## 一、为什么创造 YaoXiang？

### 1.1 填补的语言空白

在编程语言的历史长河中，我们见证了无数优秀语言的诞生与演进：C 语言带来了系统编程的效率革命，Python 创造了人人可学的编程体验，Rust 证明了内存安全与性能可以兼得，TypeScript 让大型前端项目变得可维护。然而，当我们审视当今的语言生态时，仍然发现一个明显的断层地带——**没有任何一门语言能够同时满足以下三个核心需求**：

| 需求 | 现有解决方案的问题 |
|------|-------------------|
| **类型安全** | Rust 过于严苛，学习曲线陡峭；TypeScript 是可选类型，无法提供编译时保证 |
| **自然语法** | Rust 语法复杂晦涩；Haskell 函数式门槛过高；传统静态语言冗长繁琐 |
| **AI 友好** | 现有语言语法歧义多、AST 复杂、隐藏行为难以预测，AI 生成和修改代码的准确率受限 |

YaoXiang 的诞生，正是为了填补这一空白。我们相信：**编程语言应该既强大又亲和，既安全又高效，既严谨又优雅**。

### 1.2 解决的实际问题

**问题一：类型系统的碎片化**

当今的编程语言在类型系统上呈现严重的碎片化。静态类型语言追求编译时的绝对正确，但往往以牺牲开发效率为代价；动态类型语言提供了灵活性，却在大型项目中暴露出难以维护的缺陷。YaoXiang 提出「一切皆类型」的统一抽象框架，让类型成为贯穿语言设计的主线，而非事后添加的补丁。

**问题二：内存安全与性能的二选一**

长久以来，开发者不得不在内存安全与运行性能之间做出艰难抉择。GC（垃圾回收）虽然解放了开发者，却带来了延迟波动和内存开销；手动内存管理虽然高效，却如同走钢丝般危险。YaoXiang 采用 Rust 风格的所有权模型，在编译期消除数据竞争和内存泄漏，同时保持零成本抽象，无需 GC 即可实现高性能。

**问题三：异步编程的认知负担**

现代应用离不开网络和并发，而异步编程一直是程序员的噩梦。回调函数嵌套、Promise 链式调用、async/await 语法——每一种方案都增加了代码的复杂性。YaoXiang 重新设计了异步模型：只需在函数签名后添加 `spawn` 标记，编译器自动处理所有异步细节，让并发编程如同同步代码一样自然。

**问题四：AI 辅助编程的瓶颈**

当 AI 开始辅助开发者编写代码时，语言设计的选择变得至关重要。模糊的语法规则、隐式的类型转换、复杂的语法糖——这些人类程序员已经习惯的特性，却成为 AI 理解和生成的障碍。YaoXiang 从设计之初就将「AI 友好」作为核心目标：严格的缩进规则、明确的代码块边界、无歧义的语法结构，让 AI 能够准确理解、生成和修改代码。

### 1.3 语言的哲学根基

YaoXiang 的名字源于《易经》中的「爻」与「象」。「爻」是组成卦象的基本符号，象征着阴阳变化、动静相生；「象」是事物本质的外在表现，代表万象万物、包罗万象。

这一哲学思想体现在语言设计的每一个细节中：

- **统一性**：如同爻卦的简单符号构成复杂卦象，YaoXiang 用少数几个核心概念（类型、函数、构造器）构建完整的编程模型
- **层次性**：如同象有先后天之分，YaoXiang 的类型系统具有清晰的层次结构，从原类型到泛型，从值到元类型
- **变化性**：如同阴阳流转、变化无穷，YaoXiang 支持依赖类型，允许类型随值变化而演化
- **可识别性**：如同卦象可解、万物可象，YaoXiang 提供完整的类型反射能力，运行时类型信息完全可用
- **可证明性**：如同卦象揭示事物规律，YaoXiang 的类型系统遵循 Curry-Howard 同构（类型即命题，程序即证明），类型检查的过程就是逻辑证明的验证

---

## 二、核心哲学与原则

以下设计信条是 YaoXiang 的基石，**不可妥协、不可违背**。任何特性提案都必须经过这些原则的检验。

### 2.1 原则一：一切皆类型

在 YaoXiang 的世界观中，类型是最高层的抽象单元，是贯穿语言的核心概念。

**具体体现**：

- **值是类型的实例**：`42` 是 `Int` 类型的实例，`"hello"` 是 `String` 类型的实例
- **类型本身也是类型**：`Type` 是语言唯一的元类型关键字，`Int` 的類型是 `Type`
- **函数是类型映射**：`add: (a: Int, b: Int) -> Int` 描述了一个从 `Int × Int` 到 `Int` 的类型映射
- **模块是类型组合**：模块是包含函数和类型的命名空间组合

**不可妥协的理由**：统一类型抽象能够简化语言语义，消除值与类型的二元对立，让类型系统成为代码正确性的守护者，而非绊脚石。

### 2.2 原则二：严格结构化

YaoXiang 的语法设计追求「无歧义、可预测、易解析」。

**具体规则**：

- **强制 4 空格缩进**：禁止使用 Tab 字符，代码块边界一目了然
- **括号不可省略**：函数参数必须有括号，列表元素必须有逗号
- **代码块必须有花括号**：`if`、`while`、`for` 等控制流必须使用 `{ }` 包裹
- **关键字数量精简**：仅保留 17 个核心关键字，拒绝语法糖泛滥

**不可妥协的理由**：严格结构化带来三个关键优势——（1）IDE 语法高亮和代码折叠更准确；（2）AI 代码生成和修改的准确率大幅提升；（3）新学习者能够快速理解代码结构。

### 2.3 原则三：零成本抽象

高层次的抽象不应该带来运行时的性能开销。

**具体保证**：

- **单态化**：泛型函数在编译时展开为具体版本，无虚表查找开销
- **内联优化**：简单函数自动内联，消除函数调用开销
- **栈分配优先**：小对象默认栈分配，堆分配仅在必要时使用
- **无 GC**：所有权模型保证内存安全，无需垃圾回收器的运行时开销

**不可妥协的理由**：性能是编程语言的生存底线。任何以性能为代价换取便利性的设计都是对程序员的背叛。

### 2.4 原则四：默认不可变

可变性与复杂性如影随形。YaoXiang 选择默认不可变，让代码更易于推理和理解。

**具体规则**：

- 变量默认不可变，赋值后不能再修改
- 需要可变时必须显式声明 `mut`
- 引用默认不可变，可变引用需要 `mut` 标记
- 所有权的转移意味着原绑定失效

**不可妥协的理由**：不可变性是并发安全的基础，是代码可读性的保障，是函数式编程智慧的结晶。

### 2.5 原则五：类型即数据

类型信息不应仅存在于编译期，而应在运行时完全可用。

**具体能力**：

- 运行时类型查询：任何值都可以获取其类型信息
- 类型反射：可以构造和操作类型本身
- 模式匹配解构：类型构造器可直接用于模式匹配
- 泛型特化：运行时可以获取泛型参数的具现化类型

**不可妥协的理由**：完整的类型反射能力是元编程的基础，是高性能框架和工具的基石。

---

## 三、关键创新与特性

YaoXiang 在吸收现有语言优秀特性的同时，提出了以下创新性设计。

### 3.1 创新一：统一类型语法

**传统语言的类型定义**往往需要多个关键字：

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
trait Drawable { fn draw(&self, s: &Surface); }
```

**YaoXiang 的统一语法**：一切皆 `name: type = value`，`Type` 是唯一的元类型关键字。

```yaoxiang
# === 记录类型 ===

Point: Type = {
    x: Float,
    y: Float,
}

# 带默认值的字段
Point3D: Type = {
    x: Float = 0,
    y: Float = 0,
    z: Float = 0,
}

# === 泛型类型 ===

Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

# === 接口（字段全为函数类型的记录） ===

Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

Serializable: Type = {
    serialize: () -> String,
}

# === 接口实现（接口名写在类型体内） ===

Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable,
}

# === 方法（Type.method 语法） ===

Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

**创新价值**：没有 `fn`、`struct`、`enum`、`trait`、`impl` 关键字碎片——一个统一语法覆盖所有声明。

### 3.2 创新二：构造器即类型

**值构造与函数调用完全相同**：

```yaoxiang
# 类型定义
Point: Type = { x: Float, y: Float }
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

# 值构造：与函数调用相同
p: Point = Point(3.0, 4.0)
opt: Option(Int) = Option.some(42)
none: Option(Int) = Option.none()

# 模式匹配：直接解构
match opt {
    Option.some(value) -> print(value)
    Option.none -> print("nothing")
}
```

### 3.3 创新三：柯里化方法绑定

YaoXiang 采用纯函数式设计，通过柯里化实现类似对象方法调用的语法糖，无需引入 `class` 和 `method` 关键字。

```yaoxiang
# === 类型定义 ===

Point: Type = {
    x: Float,
    y: Float,
}

# 核心函数：欧几里得距离
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

# 方法语法糖绑定（[0] 表示绑定到第 0 个参数位置）
Point.distance = distance[0]

# === 使用 ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 两种调用方式完全等价
d1 = distance(p1, p2)     # 直接调用核心函数
d2 = p1.distance(p2)      # 方法语法糖

# 柯里化用法
dist_from_p1 = p1.distance  # 部分应用，等待第二个参数
d3 = dist_from_p1(p2)       # 2.828
```

**创新价值**：纯函数式设计，无隐藏的 `self` 参数，函数即值可以自由传递和组合。

### 3.4 创新四：并作模型

> 「万物并作，吾以观复。」——《易·复卦》
>
> 并作模型取意于此，描述了一种编程范式：开发者以同步、顺序的思维描述逻辑，而语言运行时令其中的计算单元如万物并作般自动、高效地并发执行，并在最终统一协同。

**核心三原则**：

| 原则 | 说明 |
|------|------|
| **同步语法** | 所见即所得的顺序代码 |
| **并发本质** | 运行时自动提取并行性 |
| **统一协同** | 结果在需要时自动汇聚，保证逻辑正确 |

**术语体系**：

| 官方术语 | 对应语法 | 阐释 |
|----------|----------|------|
| **并作函数** | `spawn (params) => body` | 定义可参与并作执行的计算单元 |
| **并作块** | `spawn { a(), b() }` | 显式声明的并发疆域，块内任务并作执行 |
| **并作循环** | `spawn for x in xs { ... }` | 数据并行，循环体在所有元素上并作执行 |
| **并作值** | `Async(T)` | 正在并作中的未来值，使用时自动等待 |
| **并作图** | 惰性计算图(DAG) | 并作发生的舞台，描述依赖与并行关系 |
| **并作调度器** | 运行时任务调度器 | 协调万物，让它们在正确时机并作的智能中枢 |

> **详见**：[RFC-001 并作模型](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === 并作函数 ===
# spawn 标记的函数
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === 并作块 ===
# spawn { } 内的表达式强制并行执行
compute_all: () -> (Int, Int, Int) spawn = {
    (a, b, c) = spawn {
        heavy_calc(1),    # 任务 1
        heavy_calc(2),    # 任务 2
        another_calc(3)   # 任务 3
    }
    return (a, b, c)
}

# === 自动等待 ===
main: () -> Void = {
    # 两个独立请求自动并行执行
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # 等待点在需要结果时自动插入
    print(users.length + posts.length)  # 自动等待 users 和 posts
}
```

**线程安全**：

```yaoxiang
# ref 关键字自动处理线程安全（编译器自动选 Rc/Arc）
main: () -> Void = {
    counter = ref SafeCounter(0)

    # 跨任务共享：编译器自动选 Arc
    spawn {
        counter.increment()
    }
    spawn {
        counter.increment()
    }
}
```

**技术文档**：
- 详见 [RFC-001 并作模型](./rfc/accepted/001-concurrent-model-error-handling.md)

**创新价值**：异步编程的认知负担降为零，代码可读性与同步代码完全相同，同时获得高性能并行的执行效率。

### 3.5 创新五：值依赖类型（RFC-011）

> **状态**：设计中，部分实现

类型可以依赖于值，实现真正的类型驱动开发。

```yaoxiang
# 矩阵类型：维度在编译期确定
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# 编译期计算：factorial(3) = 6
vec: Vec(factorial(3)) = Vec(6)()

# 编译期维度验证
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # 编译错误：维度不匹配
```

**创新价值**：在编译期捕获更多错误，实现更精确的类型保证。

### 3.6 创新六：极简关键字设计

YaoXiang 仅定义 17 个核心关键字，数量远少于主流语言：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

| 对比语言 | 关键字数量 |
|----------|-----------|
| YaoXiang | **17** |
| Rust | 51+ |
| Python | 35 |
| TypeScript | 64+ |
| Go | 25 |

**创新价值**：更低的记忆负担，更一致的语法风格，更易解析的语法结构。

---

## 四、初步语法预览

以下代码示例展示 YaoXiang 的语言风貌，帮助您快速感受其设计美学。

### 4.1 Hello World

```yaoxiang
# hello.yx

main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

### 4.2 类型定义与函数

```yaoxiang
# 统一类型语法：name: type = value

# 记录类型
Point: Type = { x: Float, y: Float }

# 泛型类型
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

# 接口类型（字段全为函数的记录）
Serializable: Type = {
    serialize: () -> String,
}

# 函数定义
add: (a: Int, b: Int) -> Int = a + b

# 泛型函数
identity: (T: Type) -> ((x: T) -> T) = x

# 多行函数
fact: (n: Int) -> Int = {
    if n == 0 { return 1 }
    return n * fact(n - 1)
}
```

### 4.3 模式匹配

```yaoxiang
# 模式匹配
classify: (n: Int) -> String = {
    return match n {
        0 -> "zero",
        1 -> "one",
        _ if n < 0 -> "negative",
        _ -> "positive",
    }
}

# 解构模式
Point: Type = { x: Float, y: Float }
match point {
    Point(0.0, 0.0) -> "origin",
    Point(x, y) -> "point at (${x}, ${y})",
}
```

### 4.4 所有权模型（RFC-009 v9）

```yaoxiang
Point: Type = { x: Float, y: Float }

# 默认 Move（零拷贝）
p1 = Point(1.0, 2.0)
p2 = p1              # Move，p1 不可再读

# &T / &mut T 令牌（编译期零开销）
p2.print()           # 编译器自动创建 &Point 令牌
p2.shift(1.0, 1.0)  # 编译器自动创建 &mut Point 令牌

# ref：共享持有（编译器自动选 Rc/Arc）
shared = ref p2      # 跨作用域共享

# clone()：显式深拷贝
backup = p2.clone()

# unsafe + 裸指针：系统级
unsafe {
    ptr: *Point = &p2
    (*ptr).x = 0.0
}
```

**所有权梯度**：
```
&T / &mut T    Move       ref        clone()    unsafe
    |             |          |           |          |
借用令牌       默认      共享持有     深拷贝     裸指针
零成本         零拷贝    自动Rc/Arc   显式      系统级
```

### 4.5 错误处理

```yaoxiang
# Result 类型
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

divide: (a: Float, b: Float) -> Result(Float, String) = {
    if b == 0.0 {
        return Result.err("Division by zero")
    }
    return Result.ok(a / b)
}

# 使用 match 处理
result = divide(10.0, 2.0)
match result {
    Result.ok(value) -> print(value),
    Result.err(msg) -> print("Error: ${msg}"),
}
```

### 4.6 并发编程（并作模型）

```yaoxiang
# spawn 标记异步函数
fetch_api: (url: String) -> JSON spawn = {
    response = HTTP.get(url)
    return JSON.parse(response.body)
}

# 并发构造块：显式并行
process_all: () -> (JSON, JSON, JSON) spawn = {
    (a, b, c) = spawn {
        fetch_api("https://api1.com/data"),
        fetch_api("https://api2.com/data"),
        fetch_api("https://api3.com/data")
    }
    return (a, b, c)
}
```

---

## 五、路线图与待定项

### 5.1 已决定的设计决策

以下决策已经过充分讨论和审阅，**不再接受更改**：

| 模块 | 决策 | 说明 |
|------|------|------|
| **类型系统** | 一切皆类型 | 值、函数、模块、泛型都是类型 |
| **类型语法** | 统一 `name: type = value` | 一种声明形式覆盖所有情况，`Type` 是唯一元类型关键字 |
| **关键字** | 17个核心关键字 | 不含 `type`/`fn`/`struct`/`enum`/`trait`/`impl` |
| **函数语法** | 签名 + 表达式 | `name: (params) -> ReturnType = body` |
| **方法绑定** | RFC-004 柯里化绑定 | `Type.method = function[position]` |
| **异步模型** | 并作模型 | `spawn` 标记，惰性求值，自动并行 |
| **内存管理** | 所有权模型（RFC-009 v9） | Move + &T/&mut T 令牌 + ref + clone + unsafe，无 GC |
| **文件即模块** | 模块系统 | 每个 `.yx` 文件是一个模块 |
| **主函数** | `main: () -> Void` | 程序入口点 |
| **线程安全** | ref 自动选 Rc/Arc | 编译器逃逸分析，用户无感 |

### 5.3 实现路线图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang 实现路线图（示例）                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust 解释器 ────────→ v0.5: Rust 编译器 ────────→ v1.0: Rust AOT    │
│        ✅ 已完成                    │ (当前阶段)               编译器          │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang 解释器 ←─────── v1.0: YaoXiang JIT 编译器 ←──── v2.0:      │
│        （自举）                     （自举）                      YaoXiang AOT │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 六、如何参与贡献

YaoXiang 是一门诞生于社区、成长于社区、服务于社区的语言。我们诚挚邀请每一位热爱编程语言设计的开发者加入这场探索之旅。

### 6.1 设计讨论

**适合人群**：编程语言理论研究者、类型系统爱好者、语言设计狂热者

**参与方式**：

- **GitHub Discussions**：参与「Language Design」分类的讨论
- **设计提案（RFC）**：提出新特性的设计文档，遵循 `rfcs/` 目录下的模板
- **语法评审**：对现有语法设计提出改进建议或发现潜在问题

| **当前热门议题**： |
| |
| - 宏系统的设计与实现 |
| - 接口类型机制 |
| - 错误处理语法优化 |
| - 标准库 API 设计 |

**提交设计提案**：

1. 在 `rfcs/` 目录创建新文件
2. 填写 RFC 模板（动机、详细设计、优缺点分析、替代方案）
3. 发起 Pull Request 进行社区评审
4. 经过核心团队审议后合并或拒绝

### 6.2 编译器实现

**适合人群**：编译器开发者、系统程序员、性能优化专家

**当前实现重点**（按优先级排序）：

| 优先级 | 模块 | 说明 | 难度 |
|--------|------|------|------|
| P0 | **字节码虚拟机** | VM 指令完善、性能优化 | 中 |
| P0 | **运行时内存** | GC 实现、内存分配器 | 高 |
| P0 | **异步运行时** | 并作模型完整实现 | 高 |
| P1 | 标准库 | IO、String、List、Concurrent | 中 |
| P1 | JIT 编译器 | Cranelift 集成 | 高 |
| P2 | AOT 编译器 | LLVM/ Cranelift 后端 | 高 |
| P3 | 自举编译器 | 用 YaoXiang 重写 | 极高 |

**技术栈**：

- **实现语言**：Rust（当前阶段）
- **代码生成**：Cranelift 或 LLVM
- **构建工具**：Cargo
- **测试框架**：Rust `#[test]` + `cargo nextest`

**开始贡献**：

1. 查看 `docs/YaoXiang-implementation-plan.md` 了解架构设计
2. 选择 `src/` 目录下感兴趣的模块
3. 查看 `tests/unit/` 了解测试要求
4. 提交代码前确保 `cargo fmt` 和 `cargo clippy` 通过

### 6.3 工具链开发

**适合人群**：IDE 插件开发者、工具链爱好者、效率工具追求者

**需要开发的工具**：

| 工具 | 状态 | 说明 |
|------|------|------|
| **LSP 服务器** | ⏳ 待开始 | 语言服务器协议支持 |
| **调试器集成** | ⏳ 待开始 | GDB/LLDB 集成 |
| **格式化工具** | ⏳ 待开始 | `yaoxiang fmt` |
| **包管理器** | ⏳ 待开始 | 依赖管理、版本解析 |
| **包仓库** | ⏳ 待开始 | 中央仓库或去中心化 |
| **REPL** | ⏳ 待开始 | 交互式解释器 |
| **基准测试工具** | ⏳ 待开始 | 性能分析 |
| **VS Code 插件** | ⏳ 待开始 | 语法高亮、补全、调试 |
| **Vim/Neovim 插件** | ⏳ 待开始 | 语法高亮、LSP 客户端 |

**项目结构参考**：

```
yaoxiang/
├── src/
│   ├── tools/                    # 工具链
│   │   ├── lsp/                  # LSP 服务器
│   │   ├── fmt/                  # 格式化工具
│   │   ├── repl/                 # REPL
│   │   └── benchmark/            # 基准测试
│   └── ...
├── extensions/                   # 编辑器扩展
│   ├── vscode/                   # VS Code
│   └── vim/                      # Vim/Neovim
```

### 6.4 标准库建设

**适合人群**：库开发者、API 设计者、领域专家

**标准库模块规划**：

| 模块 | 优先级 | 说明 |
|------|--------|------|
| `std.io` | P0 | 文件 IO、控制台输入输出 |
| `std.string` | P0 | 字符串操作、格式化 |
| `std.list` | P0 | 列表/数组操作 |
| `std.dict` | P0 | 字典/哈希表 |
| `std.math` | P0 | 数学函数、常量 |
| `std.time` | P1 | 时间日期操作 |
| `std.net` | P1 | 网络编程、HTTP |
| `std.concurrent` | P1 | 并发原语、通道 |
| `std.crypto` | P2 | 加密哈希、签名 |
| `std.json` | P1 | JSON 解析/生成 |
| `std.regex` | P2 | 正则表达式 |
| `std.database` | P3 | 数据库连接 |
| `std.gui` | P3 | 图形界面（长期） |

**设计原则**：

- 一致性：相同功能的函数命名和行为保持一致
- 简洁性：API 应当直观易用，避免过度设计
- 性能：标准库函数应当高效，避免不必要的拷贝
- 可测试：每个函数都应有对应的单元测试

### 6.5 文档与教程

**适合人群**：技术写作者、教育工作者、社区经理

**需要贡献的文档**：

| 文档 | 状态 | 说明 |
|------|------|------|
| 快速入门 | ✅ 完成 | 5 分钟上手指南 |
| 语言指南 | ✅ 完成 | 系统学习核心概念 |
| 语言规范 | ✅ 完成 | 完整的语法和语义定义 |
| 实现计划 | ✅ 完成 | 编译器实现技术细节 |
| API 文档 | ⏳ 待开始 | 标准库 API 参考 |
| 教程 | ⏳ 待开始 | 进阶教程和最佳实践 |
| 博客 | ⏳ 待开始 | 技术文章和设计故事 |
| 翻译 | ⏳ 待开始 | 多语言支持 |

### 6.6 社区建设

**适合人群**：社区经理、活动组织者、布道师

**社区活动**：

- 定期线上 Meetup（每月一次）
- 设计与实现讨论会（每周一次）
- 代码贡献 Sprint（每季度一次）
- 线下聚会和 conference 演讲

**传播渠道**：

- GitHub Discussions：技术讨论
- GitHub Issues：问题报告和功能请求
- Discord/Slack：实时交流
- Twitter/X：项目动态
- 博客：深度文章

### 6.7 贡献指南

**如何开始贡献**：

1. **了解项目**：阅读 README 和设计文档
2. **选择方向**：根据兴趣选择贡献领域
3. **搭建环境**：Rust 1.75+、cargo、git
4. **找任务**：查看 GitHub Issues 的 `good first issue` 标签
5. **提交 PR**：遵循提交规范，编写测试
6. **参与评审**：review 他人的代码，参与讨论

**提交规范**：

```bash
# 提交信息格式
<type>(<scope>): <subject>

# 类型
feat: 新功能
fix: Bug 修复
docs: 文档更新
style: 代码格式（不影响功能）
refactor: 重构
perf: 性能优化
test: 测试
chore: 构建工具或辅助工具

# 示例
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**代码风格**：

- 遵循 `rustfmt.toml` 规范
- 确保 `cargo clippy` 无警告
- 编写必要的单元测试
- 更新相关文档

---

## 附录A：语言速查

### A.1 关键字

| 关键字 | 作用 |
|--------|------|
| `pub` | 公共导出 |
| `use` | 导入模块 |
| `spawn` | 并作标记 |
| `ref` | 共享持有（编译器自动选 Rc/Arc） |
| `mut` | 可变变量 |
| `if/elif/else` | 条件分支 |
| `match` | 模式匹配 |
| `while/for` | 循环 |
| `return/break/continue` | 控制流 |
| `as` | 类型转换 |
| `in` | 成员检测/列表推导 |
| `unsafe` | unsafe 代码块（裸指针） |

> **注意**：`Type`、`true`、`false`、`void` 等是保留字，不是关键字。`type` 关键字已在 RFC-010 中移除，统一使用 `name: Type = value` 语法。

### A.3 原类型

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

### A.4 运算符优先级

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

---

## 附录B：设计灵感

YaoXiang 的设计借鉴了以下语言和项目的优秀思想：

| 来源 | 借鉴点 |
|------|--------|
| **Rust** | 所有权模型、零成本抽象、类型系统 |
| **Python** | 语法风格、可读性、列表推导 |
| **Idris/Agda** | 依赖类型、类型驱动开发 |
| **Curry-Howard 同构** | 类型即命题，程序即证明，类型系统与逻辑的统一理论 |
| **TypeScript** | 类型注解、运行时类型 |
| **MoonBit** | AI 友好设计、简洁语法 |
| **Haskell** | 纯函数式、模式匹配 |
| **OCaml** | 类型推断、变体类型 |

---

## 附录C：常见问题

**Q: YaoXiang 与 Rust 相比有什么优势？**

A: YaoXiang 保留了 Rust 的内存安全和零成本抽象，但采用更简单的语法和更低的认知负担。**并作模型**比 Rust 的 `async/await` 更简洁——只需一个 `spawn` 标记，无需手动管理 Future 和 Pin。「万物并作，吾以观复」，让并发编程如同描述自然规律般直观。**所有权模型**（RFC-009 v9）用 Move + &T/&mut T 令牌替代生命周期标注，用类型属性（Dup/Linear）替代借用检查器。统一类型语法消除了 `enum`/`struct`/`trait`/`impl` 的概念碎片。

**Q: YaoXiang 适合做什么类型的开发？**

A: 系统编程、应用开发、Web 服务、脚本工具、AI 辅助编程。目标是成为一门通用编程语言。

**Q: 为什么选择 4 空格缩进？**

A: 4 空格提供了清晰的代码块视觉分隔，减少了嵌套深度带来的混淆。这是经过深思熟虑的「AI 友好」设计决策。

**Q: 什么时候会发布 1.0 版本？**

A: v1.0 目标：生产可用。发布时间取决于实现进度，详见 [版本规划 RFC](./rfc/003-version-planning.md)。

**Q: 如何联系核心团队？**

A: 通过 GitHub Discussions 或 Discord 社区频道。核心团队成员会定期回复。

---

> **最后更新**：2026-05-31
>
> **文档版本**：v2.0.0
>
> **许可证**：[MIT](LICENSE)

---

> 「爻象变化，万物生焉。类型演化，程序成焉。」
>
> 愿 YaoXiang 的设计之旅，与您同行。
