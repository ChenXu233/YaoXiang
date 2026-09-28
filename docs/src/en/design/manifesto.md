# YaoXiang (爻象) Design Manifesto

> **Version**: v2.0.0 **Status**: Officially Released **Author**: Chenxu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Dao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to the myriad things." — _Tao Te Ching_
>
> Types are like the Dao; all things are born from them.

---

## 1. Why Create YaoXiang?

### 1.1 Filling a Gap in the Language Landscape

Throughout the long history of programming languages, we have witnessed the birth and evolution of
countless excellent languages: C brought about an efficiency revolution in system programming,
Python created a programming experience accessible to everyone, Rust proved that memory safety and
performance can coexist, and TypeScript made large frontend projects maintainable. Yet when we
examine today's language ecosystem, we still find a clear gap — **no single language can
simultaneously meet the following three core needs**:

| Need               | Problems with Existing Solutions                                                                                                                      |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Type Safety**    | Rust is too strict with a steep learning curve; TypeScript uses optional typing and provides no compile-time guarantees                               |
| **Natural Syntax** | Rust's syntax is complex and obscure; Haskell's functional paradigm has too high a barrier; traditional static languages are verbose and cumbersome   |
| **AI-Friendly**    | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behavior, limiting the accuracy of AI-generated and AI-modified code |

YaoXiang was born precisely to fill this gap. We believe: **a programming language should be both
powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Real Problems We Solve

**Problem One: Fragmentation of Type Systems**

Today's programming languages exhibit serious fragmentation in their type systems. Statically typed
languages pursue absolute correctness at compile time, but often at the cost of development
efficiency; dynamically typed languages offer flexibility but expose maintainability issues in large
projects. YaoXiang proposes the unified abstraction of "Everything is a Type," making types the
central thread running through language design rather than an after-the-fact patch.

**Problem Two: The Either/Or Between Memory Safety and Performance**

For a long time, developers have had to make a difficult choice between memory safety and runtime
performance. While GC (garbage collection) liberates developers, it brings latency fluctuations and
memory overhead; manual memory management is efficient but dangerous like walking a tightrope.
YaoXiang adopts a Rust-style ownership model to eliminate data races and memory leaks at compile
time while maintaining zero-cost abstractions — achieving high performance without GC.

**Problem Three: The Cognitive Burden of Asynchronous Programming**

Modern applications are inseparable from networking and concurrency, yet asynchronous programming
has always been a programmer's nightmare. Callback nesting, Promise chains, async/await syntax —
each approach adds complexity to code. YaoXiang has redesigned the asynchronous model: just add the
`spawn` marker after a function signature, and the compiler automatically handles all asynchronous
details, making concurrent programming as natural as synchronous code.

**Problem Four: The Bottleneck of AI-Assisted Programming**

When AI began assisting developers in writing code, language design choices became critically
important. Ambiguous syntax rules, implicit type conversions, complex syntactic sugar — these
features that human programmers have grown accustomed to become obstacles for AI understanding and
generation. From the very beginning, YaoXiang has made "AI-friendliness" a core design goal: strict
indentation rules, clear code block boundaries, and unambiguous syntactic structure let AI
accurately understand, generate, and modify code.

### 1.3 The Philosophical Roots of the Language

The name YaoXiang (爻象) is derived from the "Yao" and "Xiang" in the _I Ching_. "Yao" refers to the
basic symbols that compose hexagrams, symbolizing the interplay of yin and yang and the generation
of movement and stillness; "Xiang" refers to the external manifestations of the essence of things,
representing the myriad phenomena that encompass all things.

This philosophy is reflected in every detail of the language design:

- **Unity**: Just as a few simple symbols of Yao form complex hexagrams, YaoXiang builds a complete
  programming model from a handful of core concepts (types, functions, constructors)
- **Hierarchy**: Just as there are innate and acquired aspects of Xiang, YaoXiang's type system has
  a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Variability**: Just as yin and yang flow and transform endlessly, YaoXiang supports dependent
  types, allowing types to evolve as values change
- **Identifiability**: Just as hexagrams can be interpreted and all things can be symbolized,
  YaoXiang provides full type reflection, with complete runtime type information
- **Provability**: Just as hexagrams reveal the patterns of things, YaoXiang's type system follows
  the Curry-Howard correspondence (types as propositions, programs as proofs); type checking is the
  verification of a logical proof

---

## 2. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang and are **non-negotiable and
inviolable**. Every feature proposal must pass the test of these principles.

### 2.1 Principle One: Everything is a Type

In YaoXiang's worldview, types are the highest-level abstraction units and the central concept
running through the language.

**Concrete Manifestations**:

- **Values are instances of types**: `42` is an instance of `Int`, `"hello"` is an instance of
  `String`
- **Types themselves are types**: `Type` is the language's only meta type keyword; the type of `Int`
  is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from
  `Int × Int` to `Int`
- **Modules are type compositions**: a module is a composition of namespaces containing functions
  and types

**Why It's Non-Negotiable**: Unified type abstraction simplifies language semantics, eliminates the
dichotomy between values and types, and allows the type system to be the guardian of code
correctness rather than an obstacle.

### 2.2 Principle Two: Strict Structure

YaoXiang's syntax design pursues "unambiguity, predictability, and easy parsing."

**Concrete Rules**:

- **Mandatory 4-space indentation**: Tab characters are forbidden; code block boundaries are clear
  at a glance
- **Brackets cannot be omitted**: function parameters must be parenthesized, and list elements must
  be separated by commas
- **Code blocks must use curly braces**: control flow constructs like `if`, `while`, `for` must be
  wrapped in `{ }`
- **Minimal set of keywords**: only 17 core keywords are retained; syntactic sugar proliferation is
  rejected

**Why It's Non-Negotiable**: Strict structure brings three key advantages — (1) more accurate IDE
syntax highlighting and code folding; (2) dramatically improved accuracy of AI code generation and
modification; (3) new learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Concrete Guarantees**:

- **Monomorphization**: generic functions are expanded into specific versions at compile time, with
  no vtable lookup overhead
- **Inlining optimization**: simple functions are automatically inlined, eliminating function call
  overhead
- **Stack allocation by default**: small objects are stack-allocated by default; heap allocation is
  used only when necessary
- **No GC**: the ownership model guarantees memory safety, with no runtime overhead from a garbage
  collector

**Why It's Non-Negotiable**: Performance is the survival baseline of a programming language. Any
design that trades performance for convenience is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability is shadowed by complexity. YaoXiang chooses immutability by default, making code easier
to reason about and understand.

**Concrete Rules**:

- Variables are immutable by default and cannot be modified after assignment
- When mutability is needed, it must be explicitly declared with `mut`
- References are immutable by default; mutable references require the `mut` marker
- Transfer of ownership means the original binding is invalidated

**Why It's Non-Negotiable**: Immutability is the foundation of concurrent safety, the guarantor of
code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types are Data

Type information should not only exist at compile time but should be fully available at runtime.

**Concrete Capabilities**:

- Runtime type queries: any value can obtain its type information
- Type reflection: types themselves can be constructed and manipulated
- Destructuring via pattern matching: type constructors can be used directly in pattern matching
- Generic specialization: the realized type of generic parameters can be obtained at runtime

**Why It's Non-Negotiable**: Complete type reflection is the foundation of metaprogramming and the
cornerstone of high-performance frameworks and tools.

---

## 3. Key Innovations and Features

While absorbing the excellent features of existing languages, YaoXiang proposes the following
innovative designs.

### 3.1 Innovation One: Unified Type Syntax

**Traditional languages** often require multiple keywords to define types:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
trait Drawable { fn draw(&self, s: &Surface); }
```

**YaoXiang's unified syntax**: everything follows `name: type = value`, and `Type` is the only meta
type keyword.

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

**Innovation Value**: No fragmentation of `fn`, `struct`, `enum`, `trait`, `impl` keywords — one
unified syntax covers all declarations.

### 3.2 Innovation Two: Constructors are Types

**Value construction is completely identical to function calls**:

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

### 3.3 Innovation Three: Curried Method Binding

YaoXiang adopts a purely functional design, using currying to implement object-method-call-like
syntactic sugar, without introducing the `class` and `method` keywords.

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

**Innovation Value**: Purely functional design, no hidden `self` parameter, functions are values
that can be freely passed and composed.

### 3.4 Innovation Four: The Spawn Model

> "The myriad things arise together; by this I observe the cycles of return." — _I Ching · Return
> Hexagram_
>
> The Spawn Model is inspired by this, describing a programming paradigm: developers describe logic
> with synchronous, sequential thinking, while the language runtime makes the computational units
> within automatically and efficiently execute concurrently, like the myriad things arising
> together, and ultimately collaborate in unity.

**Three Core Principles**:

| Principle                 | Description                                                              |
| ------------------------- | ------------------------------------------------------------------------ |
| **Synchronous Syntax**    | Sequential code that is exactly what it seems                            |
| **Concurrent Essence**    | The runtime automatically extracts parallelism                           |
| **Unified Collaboration** | Results automatically converge when needed, ensuring logical correctness |

**Terminology System**:

| Official Term       | Corresponding Syntax        | Explanation                                                                                   |
| ------------------- | --------------------------- | --------------------------------------------------------------------------------------------- |
| **Spawn Function**  | `spawn (params) => body`    | Defines a computational unit that can participate in spawn execution                          |
| **Spawn Block**     | `spawn { a(), b() }`        | An explicitly declared concurrent region; tasks within execute as spawn                       |
| **Spawn Loop**      | `spawn for x in xs { ... }` | Data parallelism; the loop body executes as spawn over all elements                           |
| **Spawn Value**     | `Async(T)`                  | A future value that is currently spawning; automatically awaited on use                       |
| **Spawn Graph**     | Lazy computation DAG        | The stage on which spawning occurs; describes dependencies and parallelism                    |
| **Spawn Scheduler** | Runtime task scheduler      | The intelligent hub that coordinates the myriad things, making them spawn at the right moment |

> **See**: [RFC-001 Spawn Model](./rfc/deprecated/001-concurrent-model-error-handling.md)

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

**Thread Safety**:

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

**Technical Documents**:

- See [RFC-001 Spawn Model](./rfc/deprecated/001-concurrent-model-error-handling.md)

**Innovation Value**: The cognitive burden of asynchronous programming is reduced to zero, code
readability is identical to synchronous code, and high-performance parallel execution efficiency is
simultaneously achieved.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, enabling true type-driven development.

```yaoxiang
# 矩阵类型：维度在编译期确定
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# 编译期计算：factorial(3) = 6
arr: Array(Int, factorial(3)) = Array(Int, 6)()

# 编译期维度验证
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # 编译错误：维度不匹配
```

**Innovation Value**: Catch more errors at compile time and achieve more precise type guarantees.

### 3.6 Innovation Six: Minimal Keyword Design

YaoXiang defines only 17 core keywords, far fewer than mainstream languages:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

| Language   | Number of Keywords |
| ---------- | ------------------ |
| YaoXiang   | **17**             |
| Rust       | 51+                |
| Python     | 35                 |
| TypeScript | 64+                |
| Go         | 25                 |

**Innovation Value**: Lower memory burden, more consistent syntactic style, and an easier-to-parse
grammar.

---

## 4. Preliminary Syntax Preview

The following code examples showcase the style of YaoXiang, helping you quickly sense its design
aesthetic.

### 4.1 Hello World

```yaoxiang
# hello.yx

main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

### 4.2 Type Definitions and Functions

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

### 4.3 Pattern Matching

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

### 4.4 Ownership Model (RFC-009 v9)

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

**Ownership Gradient**:

```
&T / &mut T    Move       ref        clone()    unsafe
    |             |          |           |          |
借用令牌       默认      共享持有     深拷贝     裸指针
零成本         零拷贝    自动Rc/Arc   显式      系统级
```

### 4.5 Error Handling

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

### 4.6 Concurrent Programming (Spawn Model)

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

## 5. Roadmap and Pending Items

### 5.1 Decided Design Decisions

The following decisions have been thoroughly discussed and reviewed and **no longer accept
changes**:

| Module                | Decision                     | Description                                                                 |
| --------------------- | ---------------------------- | --------------------------------------------------------------------------- |
| **Type System**       | Everything is a Type         | Values, functions, modules, and generics are all types                      |
| **Type Syntax**       | Unified `name: type = value` | One declaration form covers all cases; `Type` is the only meta type keyword |
| **Keywords**          | 17 core keywords             | Excludes `type`/`fn`/`struct`/`enum`/`trait`/`impl`                         |
| **Function Syntax**   | Signature + expression       | `name: (params) -> ReturnType = body`                                       |
| **Method Binding**    | RFC-004 Curried Binding      | `Type.method = function[position]`                                          |
| **Async Model**       | Spawn Model                  | `spawn` marker, lazy evaluation, automatic parallelism                      |
| **Memory Management** | Ownership Model (RFC-009 v9) | Move + &T/&mut T tokens + ref + clone + unsafe, no GC                       |
| **File as Module**    | Module system                | Each `.yx` file is a module                                                 |
| **Main Function**     | `main: () -> Void`           | Program entry point                                                         |
| **Thread Safety**     | ref auto-selects Rc/Arc      | Compiler escape analysis; transparent to users                              |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang Implementation Roadmap (Example)             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ──────→ v0.5: Rust Compiler ──────→ v1.0: Rust AOT  │
│        ✅ Completed                │ (Current Phase)              Compiler  │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←─── v1.0: YaoXiang JIT Compiler ←── v2.0:      │
│        (Self-hosting)              (Self-hosting)              YaoXiang AOT │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 6. How to Contribute

YaoXiang is a language born from the community, grown in the community, and serving the community.
We sincerely invite every developer who loves programming language design to join this journey of
exploration.

### 6.1 Design Discussions

**Suitable for**: programming language theory researchers, type system enthusiasts, language design
fanatics

**How to participate**:

- **GitHub Discussions**: participate in discussions under the "Language Design" category
- **Design Proposals (RFCs)**: submit design documents for new features, following the template in
  the `rfcs/` directory
- **Syntax Review**: suggest improvements or identify potential issues with existing syntax designs

| **Current Hot Topics**: | | | | - Macro system design and implementation | | - Interface type
mechanism | | - Error handling syntax optimization | | - Standard library API design |

**Submitting a Design Proposal**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros and cons analysis, alternatives)
3. Open a Pull Request for community review
4. Merge or reject after review by the core team

### 6.2 Compiler Implementation

**Suitable for**: compiler developers, systems programmers, performance optimization experts

**Current Implementation Priorities** (sorted by priority):

| Priority | Module                | Description                                          | Difficulty |
| -------- | --------------------- | ---------------------------------------------------- | ---------- |
| P0       | **Bytecode VM**       | VM instruction improvement, performance optimization | Medium     |
| P0       | **Runtime Memory**    | GC implementation, memory allocator                  | High       |
| P0       | **Async Runtime**     | Complete implementation of Spawn Model               | High       |
| P1       | Standard Library      | IO, String, List, Concurrent                         | Medium     |
| P1       | JIT Compiler          | Cranelift integration                                | High       |
| P2       | AOT Compiler          | LLVM/Cranelift backend                               | High       |
| P3       | Self-hosting Compiler | Rewrite in YaoXiang                                  | Extreme    |

**Technology Stack**:

- **Implementation Language**: Rust (current phase)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Test Framework**: Rust `#[test]` + `cargo nextest`

**Start Contributing**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand the architecture design
2. Choose a module of interest under the `src/` directory
3. Check `tests/unit/` to understand test requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, productivity tool seekers

**Tools Needed**:

| Tool                     | Status         | Description                                |
| ------------------------ | -------------- | ------------------------------------------ |
| **LSP Server**           | ⏳ Not started | Language Server Protocol support           |
| **Debugger Integration** | ⏳ Not started | GDB/LLDB integration                       |
| **Formatter**            | ⏳ Not started | `yx format`                                |
| **Package Manager**      | ⏳ Not started | Dependency management, version resolution  |
| **Package Registry**     | ⏳ Not started | Centralized or decentralized               |
| **REPL**                 | ⏳ Not started | Interactive interpreter                    |
| **Benchmark Tool**       | ⏳ Not started | Performance analysis                       |
| **VS Code Plugin**       | ⏳ Not started | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin**    | ⏳ Not started | Syntax highlighting, LSP client            |

**Project Structure Reference**:

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

### 6.4 Standard Library Development

**Suitable for**: library developers, API designers, domain experts

**Standard Library Module Plan**:

| Module           | Priority | Description                      |
| ---------------- | -------- | -------------------------------- |
| `std.io`         | P0       | File IO, console I/O             |
| `std.string`     | P0       | String operations, formatting    |
| `std.list`       | P0       | List/array operations            |
| `std.dict`       | P0       | Dictionary/hash table            |
| `std.math`       | P0       | Math functions, constants        |
| `std.time`       | P1       | Date and time operations         |
| `std.net`        | P1       | Network programming, HTTP        |
| `std.concurrent` | P1       | Concurrency primitives, channels |
| `std.crypto`     | P2       | Encryption, hashing, signing     |
| `std.json`       | P1       | JSON parsing/generation          |
| `std.regex`      | P2       | Regular expressions              |
| `std.database`   | P3       | Database connectivity            |
| `std.gui`        | P3       | GUI (long term)                  |

**Design Principles**:

- Consistency: functions with similar purposes should share naming and behavior
- Simplicity: APIs should be intuitive and easy to use; avoid over-engineering
- Performance: standard library functions should be efficient and avoid unnecessary copies
- Testability: every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: technical writers, educators, community managers

**Documentation Needed**:

| Document            | Status         | Description                               |
| ------------------- | -------------- | ----------------------------------------- |
| Quick Start         | ✅ Completed   | 5-minute getting-started guide            |
| Language Guide      | ✅ Completed   | Systematic learning of core concepts      |
| Language Spec       | ✅ Completed   | Complete syntax and semantic definition   |
| Implementation Plan | ✅ Completed   | Compiler implementation technical details |
| API Docs            | ⏳ Not started | Standard library API reference            |
| Tutorials           | ⏳ Not started | Advanced tutorials and best practices     |
| Blog                | ⏳ Not started | Technical articles and design stories     |
| Translation         | ⏳ Not started | Multilingual support                      |

### 6.6 Community Building

**Suitable for**: community managers, event organizers, evangelists

**Community Activities**:

- Regular online meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution sprints (quarterly)
- Offline gatherings and conference talks

**Communication Channels**:

- GitHub Discussions: technical discussions
- GitHub Issues: bug reports and feature requests
- Discord/Slack: real-time communication
- Twitter/X: project updates
- Blog: in-depth articles

### 6.7 Contribution Guide

**How to Start Contributing**:

1. **Understand the project**: read the README and design documents
2. **Choose a direction**: select a contribution area based on your interests
3. **Set up the environment**: Rust 1.75+, cargo, git
4. **Find a task**: check GitHub Issues for the `good first issue` label
5. **Submit a PR**: follow the commit conventions and write tests
6. **Participate in review**: review others' code and participate in discussions

**Commit Conventions**:

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

**Code Style**:

- Follow the `rustfmt.toml` configuration
- Ensure `cargo clippy` is warning-free
- Write necessary unit tests
- Update relevant documentation

---

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword                 | Purpose                                         |
| ----------------------- | ----------------------------------------------- |
| `pub`                   | Public export                                   |
| `use`                   | Import module                                   |
| `spawn`                 | Spawn marker                                    |
| `ref`                   | Shared ownership (compiler auto-selects Rc/Arc) |
| `mut`                   | Mutable variable                                |
| `if/else if/else`       | Conditional branches                            |
| `match`                 | Pattern matching                                |
| `while/for`             | Loops                                           |
| `return/break/continue` | Control flow                                    |
| `as`                    | Type conversion                                 |
| `in`                    | Membership test / list comprehension            |
| `unsafe`                | unsafe block (raw pointers)                     |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type`
> keyword was removed in RFC-010; the unified `name: Type = value` syntax is used instead.

### A.3 Primitive Types

| Type     | Description       | Default Size |
| -------- | ----------------- | ------------ |
| `Void`   | Void              | 0 bytes      |
| `Bool`   | Boolean           | 1 byte       |
| `Int`    | Signed integer    | 8 bytes      |
| `Uint`   | Unsigned integer  | 8 bytes      |
| `Float`  | Float             | 8 bytes      |
| `String` | UTF-8 string      | Variable     |
| `Char`   | Unicode character | 4 bytes      |
| `Bytes`  | Raw bytes         | Variable     |

### A.4 Operator Precedence

| Precedence | Operators                   | Associativity |
| ---------- | --------------------------- | ------------- |
| 1          | `()` `[]` `.` `?`           | Left to right |
| 2          | `as`                        | Left to right |
| 3          | Unary prefix `!` `-` `+`    | Right to left |
| 4          | `*` `/` `%`                 | Left to right |
| 5          | `+` `-`                     | Left to right |
| 6          | `..`                        | Left to right |
| 7          | `<<` `>>`                   | Left to right |
| 8          | `&` `\|` `^`                | Left to right |
| 9          | `==` `!=` `<` `>` `<=` `>=` | Left to right |
| 10         | `and` `or`                  | Left to right |
| 11         | `if...else`                 | Right to left |
| 12         | `=` `+=` `-=` `*=` `/=`     | Right to left |

> Unary prefix operators (`!` `-` `+`) bind tightly, above all binary operators (Zig-style, see SPEC
> §2.2).

---

## Appendix B: Design Inspirations

YaoXiang's design draws on the excellent ideas from the following languages and projects:

| Source                          | Inspiration                                                                         |
| ------------------------------- | ----------------------------------------------------------------------------------- |
| **Rust**                        | Ownership model, zero-cost abstractions, type system                                |
| **Python**                      | Syntax style, readability, list comprehensions                                      |
| **Idris/Agda**                  | Dependent types, type-driven development                                            |
| **Curry-Howard Correspondence** | Types as propositions, programs as proofs, unified theory of type systems and logic |
| **TypeScript**                  | Type annotations, runtime types                                                     |
| **MoonBit**                     | AI-friendly design, concise syntax                                                  |
| **Haskell**                     | Pure functional, pattern matching                                                   |
| **OCaml**                       | Type inference, variant types                                                       |

---

## Appendix C: FAQ

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions, but uses simpler syntax and has
a lower cognitive burden. The **Spawn Model** is more concise than Rust's `async/await` — just one
`spawn` marker is needed, with no manual management of Future and Pin. "The myriad things arise
together; by this I observe the cycles of return," making concurrent programming as intuitive as
describing natural laws. The **Ownership Model** (RFC-009 v9) replaces lifetime annotations with
Move + &T/&mut T tokens, and uses type attributes (Dup/Linear) in place of the borrow checker. The
unified type syntax eliminates the conceptual fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What kinds of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted
programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks and reduce confusion from deep nesting.
This is a deliberate "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. The release date depends on implementation progress; see the
[Version Planning RFC](./rfc/rejected/003-version-planning.md).

**Q: How do I contact the core team?**

A: Via GitHub Discussions or the Discord community channel. Core team members reply regularly.

---

> **Last Updated**: 2026-05-31
>
> **Document Version**: v2.0.0
>
> **License**: MIT

---

> "Yao and Xiang transform, the myriad things are born. Types evolve, programs come into being."
>
> May YaoXiang's design journey walk alongside you.
