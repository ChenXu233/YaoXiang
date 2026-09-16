# YaoXiang Design Manifesto

> **Version**: v2.0.0 **Status**: Official Release **Author**: Chenxu + YaoXiang Community **Date**:
> 2026-05-31

---

> "The Dao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to the ten thousand things." — _Tao Te Ching_
>
> Types are like the Dao; all things are born from them.

---

## 1. Why Create YaoXiang?

### 1.1 Filling the Language Gap

Throughout the long history of programming languages, we have witnessed the birth and evolution of
countless excellent languages: C brought the efficiency revolution to system programming, Python
created a programming experience accessible to everyone, Rust proved that memory safety and
performance can coexist, and TypeScript made large frontend projects maintainable. However, when we
examine today's language ecosystem, we still find an obvious fault line—**no single language can
simultaneously satisfy the following three core requirements**:

| Requirement        | Problems with Existing Solutions                                                                                                                         |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Type Safety**    | Rust is too strict with a steep learning curve; TypeScript has optional types and cannot provide compile-time guarantees                                 |
| **Natural Syntax** | Rust's syntax is complex and obscure; Haskell's functional approach has a high barrier to entry; traditional static languages are verbose and cumbersome |
| **AI-Friendly**    | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behaviors, limiting the accuracy of AI code generation and modification |

YaoXiang was born precisely to fill this gap. We believe: **a programming language should be both
powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 The Real Problems We Solve

**Problem One: Fragmentation of the Type System**

Today's programming languages show severe fragmentation in their type systems. Statically typed
languages pursue absolute correctness at compile time, but often at the cost of development
efficiency; dynamically typed languages offer flexibility but expose maintenance difficulties in
large projects. YaoXiang proposes a unified abstraction framework of "everything is a type," making
types the main thread running through language design rather than an afterthought patch.

**Problem Two: The Either/Or Choice Between Memory Safety and Performance**

For a long time, developers have had to make a difficult choice between memory safety and runtime
performance. While GC (garbage collection) liberates developers, it introduces latency fluctuations
and memory overhead; while manual memory management is efficient, it's as dangerous as walking a
tightrope. YaoXiang adopts Rust-style ownership semantics, eliminating data races and memory leaks
at compile time while maintaining zero-cost abstractions—achieving high performance without GC.

**Problem Three: The Cognitive Burden of Asynchronous Programming**

Modern applications rely on networking and concurrency, and asynchronous programming has long been a
programmer's nightmare. Callback nesting, Promise chains, async/await syntax—each approach adds to
code complexity. YaoXiang has redesigned the async model: simply add the `spawn` marker after a
function signature, and the compiler automatically handles all asynchronous details, making
concurrent programming as natural as synchronous code.

**Problem Four: The Bottleneck of AI-Assisted Programming**

When AI begins assisting developers in writing code, the choice of language design becomes crucial.
Ambiguous syntax rules, implicit type conversions, complex syntactic sugar—these features that human
programmers have grown accustomed to become obstacles for AI understanding and generation. From its
inception, YaoXiang has made "AI-friendly" a core design goal: strict indentation rules, clear code
block boundaries, unambiguous syntactic structures—enabling AI to accurately understand, generate,
and modify code.

### 1.3 The Philosophical Foundation of the Language

The name YaoXiang (爻象) comes from the "Yao" (爻) and "Xiang" (象) in the _I Ching_. "Yao" are the
basic symbols that compose hexagrams, symbolizing the interplay of yin and yang, the coexistence of
motion and stillness; "Xiang" represents the external manifestation of the essence of things,
embodying the vastness of all phenomena.

This philosophical thinking is reflected in every detail of the language design:

- **Unity**: Just as the simple symbols of yao compose complex hexagrams, YaoXiang uses a few core
  concepts (types, functions, constructors) to build a complete programming model
- **Hierarchy**: Just as xiang has distinctions of innate and acquired, YaoXiang's type system has a
  clear hierarchical structure, from primitive types to generics, from values to meta types
- **Mutability**: Just as yin and yang flow and transform without end, YaoXiang supports dependent
  types, allowing types to evolve as values change
- **Recognizability**: Just as hexagrams can be interpreted and all things can be symbolized,
  YaoXiang provides complete type reflection capabilities, with runtime type information fully
  available
- **Provability**: Just as hexagrams reveal the patterns of things, YaoXiang's type system follows
  the Curry-Howard correspondence (types as propositions, programs as proofs); type checking is the
  verification of logical proofs

---

## 2. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any
feature proposal must be tested against these principles.

### 2.1 Principle One: Everything Is a Type

In YaoXiang's worldview, types are the highest-level abstraction units and the core concept running
through the language.

**Concrete Manifestations**:

- **Values are instances of types**: `42` is an instance of type `Int`, `"hello"` is an instance of
  type `String`
- **Types themselves are also types**: `Type` is the language's only meta type keyword; the type of
  `Int` is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from
  `Int × Int` to `Int`
- **Modules are type compositions**: modules are namespace compositions containing functions and
  types

**Why It's Non-Negotiable**: Unified type abstraction simplifies language semantics, eliminates the
dualism between values and types, and makes the type system the guardian of code correctness rather
than an obstacle.

### 2.2 Principle Two: Strict Structuring

YaoXiang's syntax design pursues "unambiguity, predictability, and easy parsing."

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are forbidden; code block boundaries are clear
  at a glance
- **Brackets cannot be omitted**: function parameters must have parentheses, list elements must have
  commas
- **Code blocks must use curly braces**: control flow constructs like `if`, `while`, `for` must be
  wrapped with `{ }`
- **Minimal set of keywords**: only 17 core keywords are retained; syntactic sugar proliferation is
  rejected

**Why It's Non-Negotiable**: Strict structuring brings three key advantages—(1) more accurate IDE
syntax highlighting and code folding; (2) significantly improved accuracy of AI code generation and
modification; (3) newcomers can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: generic functions are expanded into concrete versions at compile time, with
  no vtable lookup overhead
- **Inlining optimization**: simple functions are automatically inlined, eliminating function call
  overhead
- **Stack allocation priority**: small objects default to stack allocation; heap allocation is used
  only when necessary
- **No GC**: the ownership model guarantees memory safety without the runtime overhead of a garbage
  collector

**Why It's Non-Negotiable**: Performance is the survival baseline of a programming language. Any
design that trades performance for convenience is a betrayal of the programmer.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity go hand in hand. YaoXiang chooses immutability by default, making code
easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default and cannot be modified after assignment
- Mutability must be explicitly declared with `mut` when needed
- References are immutable by default; mutable references require the `mut` marker
- The transfer of ownership means the original binding is invalidated

**Why It's Non-Negotiable**: Immutability is the foundation of concurrency safety, the guarantee of
code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types Are Data

Type information should not exist only at compile time but should be fully available at runtime.

**Specific Capabilities**:

- Runtime type queries: any value can retrieve its type information
- Type reflection: types themselves can be constructed and manipulated
- Pattern matching destructuring: type constructors can be used directly in pattern matching
- Generic specialization: the concrete types of generic parameters can be obtained at runtime

**Why It's Non-Negotiable**: Complete type reflection capability is the foundation of
metaprogramming and the cornerstone of high-performance frameworks and tools.

---

## 3. Key Innovations and Features

While absorbing the excellent features of existing languages, YaoXiang proposes the following
innovative designs.

### 3.1 Innovation One: Unified Type Syntax

**Traditional languages' type definitions** often require multiple keywords:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
trait Drawable { fn draw(&self, s: &Surface); }
```

**YaoXiang's unified syntax**: everything follows `name: type = value`; `Type` is the only meta type
keyword.

```yaoxiang
# === Record Type ===

Point: Type = {
    x: Float,
    y: Float,
}

# Field with default value
Point3D: Type = {
    x: Float = 0,
    y: Float = 0,
    z: Float = 0,
}

# === Generic Type ===

Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

# === Interface (record whose fields are all function types) ===

Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

Serializable: Type = {
    serialize: () -> String,
}

# === Interface Implementation (interface name written in type body) ===

Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable,
}

# === Method (Type.method syntax) ===

Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

**Innovative Value**: No `fn`, `struct`, `enum`, `trait`, `impl` keyword fragmentation—one unified
syntax covers all declarations.

### 3.2 Innovation Two: Constructors Are Types

**Value construction is exactly the same as function calls**:

```yaoxiang
# Type definition
Point: Type = { x: Float, y: Float }
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

# Value construction: same as function call
p: Point = Point(3.0, 4.0)
opt: Option(Int) = Option.some(42)
none: Option(Int) = Option.none()

# Pattern matching: direct destructuring
match opt {
    Option.some(value) -> print(value)
    Option.none -> print("nothing")
}
```

### 3.3 Innovation Three: Curried Method Binding

YaoXiang adopts a purely functional design, achieving object-method-like syntactic sugar through
currying, without introducing the `class` and `method` keywords.

```yaoxiang
# === Type Definition ===

Point: Type = {
    x: Float,
    y: Float,
}

# Core function: Euclidean distance
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

# Method syntactic sugar binding ([0] indicates binding to the 0th parameter position)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Both calling forms are completely equivalent
d1 = distance(p1, p2)     # Direct call to core function
d2 = p1.distance(p2)      # Method syntactic sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, awaiting the second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovative Value**: Purely functional design, no hidden `self` parameter; functions are values
that can be freely passed and composed.

### 3.4 Innovation Four: The Spawn Model

> "The ten thousand things arise together; we observe their return." — _I Ching · Fu Hexagram_
>
> The spawn model draws its meaning from this, describing a programming paradigm: developers
> describe logic with synchronous, sequential thinking, while the language runtime lets the
> computational units within automatically and efficiently execute concurrently like the ten
> thousand things arising together, ultimately unifying and coordinating.

**Three Core Principles**:

| Principle                | Description                                                              |
| ------------------------ | ------------------------------------------------------------------------ |
| **Synchronous Syntax**   | What you see is what you get, sequential code                            |
| **Concurrent Essence**   | The runtime automatically extracts parallelism                           |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology**:

| Official Term       | Corresponding Syntax        | Explanation                                                                                          |
| ------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------- |
| **Spawn Function**  | `spawn (params) => body`    | Defines a computational unit that can participate in spawn execution                                 |
| **Spawn Block**     | `spawn { a(), b() }`        | An explicitly declared concurrent region; tasks within the block execute as spawns                   |
| **Spawn Loop**      | `spawn for x in xs { ... }` | Data parallelism; the loop body spawns across all elements                                           |
| **Spawn Value**     | `Async(T)`                  | A future value currently spawning; automatically awaited when used                                   |
| **Spawn Graph**     | Lazy evaluation DAG         | The stage where spawning happens, describing dependencies and parallelism                            |
| **Spawn Scheduler** | Runtime task scheduler      | The intelligent core that coordinates the ten thousand things, making them spawn at the right moment |

> **See**: [RFC-001 Spawn Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === Spawn Function ===
# A function marked with spawn
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === Spawn Block ===
# Expressions inside spawn { } are forced to execute in parallel
compute_all: () -> (Int, Int, Int) spawn = {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    return (a, b, c)
}

# === Automatic Waiting ===
main: () -> Void = {
    # Two independent requests automatically execute in parallel
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # Waiting points are automatically inserted when results are needed
    print(users.length + posts.length)  # Automatically waits for users and posts
}
```

**Thread Safety**:

```yaoxiang
# The ref keyword automatically handles thread safety (compiler automatically picks Rc/Arc)
main: () -> Void = {
    counter = ref SafeCounter(0)

    # Cross-task sharing: compiler automatically picks Arc
    spawn {
        counter.increment()
    }
    spawn {
        counter.increment()
    }
}
```

**Technical Documentation**:

- See [RFC-001 Spawn Model](./rfc/accepted/001-concurrent-model-error-handling.md)

**Innovative Value**: The cognitive burden of async programming is reduced to zero; code readability
is identical to synchronous code while achieving high-performance parallel execution.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, achieving true type-driven development.

```yaoxiang
# Matrix type: dimensions determined at compile time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time computation: factorial(3) = 6
vec: Vec(factorial(3)) = Vec(6)()

# Compile-time dimension verification
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimension mismatch
```

**Innovative Value**: Catch more errors at compile time; achieve more precise type guarantees.

### 3.6 Innovation Six: Minimal Keyword Design

YaoXiang defines only 17 core keywords—far fewer than mainstream languages:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

| Comparison Language | Number of Keywords |
| ------------------- | ------------------ |
| YaoXiang            | **17**             |
| Rust                | 51+                |
| Python              | 35                 |
| TypeScript          | 64+                |
| Go                  | 25                 |

**Innovative Value**: Lower memory burden, more consistent syntactic style, easier-to-parse
syntactic structure.

---

## 4. Preliminary Syntax Preview

The following code examples showcase the language aesthetics of YaoXiang, helping you quickly get a
feel for its design.

### 4.1 Hello World

```yaoxiang
# hello.yx

main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

### 4.2 Type Definitions and Functions

```yaoxiang
# Unified type syntax: name: type = value

# Record type
Point: Type = { x: Float, y: Float }

# Generic type
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

# Interface type (record whose fields are all functions)
Serializable: Type = {
    serialize: () -> String,
}

# Function definition
add: (a: Int, b: Int) -> Int = a + b

# Generic function
identity: (T: Type) -> ((x: T) -> T) = x

# Multi-line function
fact: (n: Int) -> Int = {
    if n == 0 { return 1 }
    return n * fact(n - 1)
}
```

### 4.3 Pattern Matching

```yaoxiang
# Pattern matching
classify: (n: Int) -> String = {
    return match n {
        0 -> "zero",
        1 -> "one",
        _ if n < 0 -> "negative",
        _ -> "positive",
    }
}

# Destructuring pattern
Point: Type = { x: Float, y: Float }
match point {
    Point(0.0, 0.0) -> "origin",
    Point(x, y) -> "point at (${x}, ${y})",
}
```

### 4.4 Ownership Model (RFC-009 v9)

```yaoxiang
Point: Type = { x: Float, y: Float }

# Default Move (zero-copy)
p1 = Point(1.0, 2.0)
p2 = p1              # Move, p1 can no longer be read

# &T / &mut T tokens (zero compile-time overhead)
p2.print()           # Compiler automatically creates a &Point token
p2.shift(1.0, 1.0)   # Compiler automatically creates a &mut Point token

# ref: shared ownership (compiler automatically picks Rc/Arc)
shared = ref p2      # Share across scopes

# clone(): explicit deep copy
backup = p2.clone()

# unsafe + raw pointer: system-level
unsafe {
    ptr: *Point = &p2
    (*ptr).x = 0.0
}
```

**Ownership Gradient**:

```
&T / &mut T    Move       ref        clone()    unsafe
    |             |          |           |          |
Borrow Token   Default   Shared      Deep Copy   Raw Pointer
Zero-Cost     Zero-Copy Auto Rc/Arc  Explicit   System-Level
```

### 4.5 Error Handling

```yaoxiang
# Result type
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

# Use match to handle
result = divide(10.0, 2.0)
match result {
    Result.ok(value) -> print(value),
    Result.err(msg) -> print("Error: ${msg}"),
}
```

### 4.6 Concurrent Programming (Spawn Model)

```yaoxiang
# spawn marks an async function
fetch_api: (url: String) -> JSON spawn = {
    response = HTTP.get(url)
    return JSON.parse(response.body)
}

# Concurrent construct block: explicit parallelism
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

## 5. Roadmap and Open Items

### 5.1 Decided Design Decisions

The following decisions have been thoroughly discussed and reviewed, and **are no longer subject to
change**:

| Module                | Decision                       | Description                                                                 |
| --------------------- | ------------------------------ | --------------------------------------------------------------------------- |
| **Type System**       | Everything is a type           | Values, functions, modules, and generics are all types                      |
| **Type Syntax**       | Unified `name: type = value`   | One declaration form covers all cases; `Type` is the only meta type keyword |
| **Keywords**          | 17 core keywords               | Does not include `type`/`fn`/`struct`/`enum`/`trait`/`impl`                 |
| **Function Syntax**   | Signature + expression         | `name: (params) -> ReturnType = body`                                       |
| **Method Binding**    | RFC-004 Curried Binding        | `Type.method = function[position]`                                          |
| **Async Model**       | Spawn model                    | `spawn` marker, lazy evaluation, automatic parallelism                      |
| **Memory Management** | Ownership model (RFC-009 v9)   | Move + &T/&mut T tokens + ref + clone + unsafe, no GC                       |
| **File as Module**    | Module system                  | Each `.yx` file is a module                                                 |
| **Main Function**     | `main: () -> Void`             | Program entry point                                                         |
| **Thread Safety**     | ref automatically picks Rc/Arc | Compiler escape analysis; users are unaware                                 |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang Implementation Roadmap (Example)            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: Rust AOT│
│        ✅ Completed                   │ (Current Phase)             Compiler    │
│                                        │                                      │
│                                        ▼                                      │
│  v0.6: YaoXiang Interpreter  ←─────── v1.0: YaoXiang JIT Compiler ←──── v2.0:│
│        (Self-hosting)                  (Self-hosting)                YaoXiang AOT│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 6. How to Contribute

YaoXiang is a language born in the community, growing in the community, and serving the community.
We sincerely invite every developer who loves programming language design to join this journey of
exploration.

### 6.1 Design Discussion

**Suitable For**: Programming language theory researchers, type system enthusiasts, language design
fanatics

**How to Participate**:

- **GitHub Discussions**: participate in discussions under the "Language Design" category
- **Design Proposals (RFC)**: submit design documents for new features, following the template in
  the `rfcs/` directory
- **Syntax Review**: propose improvements or identify potential issues with existing syntax designs

| **Current Hot Topics**: | | - Macro system design and implementation | | - Interface type
mechanism | | - Error handling syntax optimization | | - Standard library API design |

**Submitting a Design Proposal**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros/cons analysis, alternatives)
3. Open a Pull Request for community review
4. After core team review, merge or reject

### 6.2 Compiler Implementation

**Suitable For**: Compiler developers, system programmers, performance optimization experts

**Current Implementation Priorities** (sorted by priority):

| Priority | Module                | Description                                         | Difficulty |
| -------- | --------------------- | --------------------------------------------------- | ---------- |
| P0       | **Bytecode VM**       | VM instruction completion, performance optimization | Medium     |
| P0       | **Runtime Memory**    | GC implementation, memory allocator                 | High       |
| P0       | **Async Runtime**     | Complete implementation of spawn model              | High       |
| P1       | Standard Library      | IO, String, List, Concurrent                        | Medium     |
| P1       | JIT Compiler          | Cranelift integration                               | High       |
| P2       | AOT Compiler          | LLVM / Cranelift backend                            | High       |
| P3       | Self-Hosting Compiler | Rewrite in YaoXiang                                 | Very High  |

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

**Suitable For**: IDE plugin developers, toolchain enthusiasts, efficiency tool pursuers

**Tools to Develop**:

| Tool                     | Status         | Description                                |
| ------------------------ | -------------- | ------------------------------------------ |
| **LSP Server**           | ⏳ Not Started | Language Server Protocol support           |
| **Debugger Integration** | ⏳ Not Started | GDB/LLDB integration                       |
| **Formatter**            | ⏳ Not Started | `yx format`                                |
| **Package Manager**      | ⏳ Not Started | Dependency management, version resolution  |
| **Package Registry**     | ⏳ Not Started | Centralized or decentralized registry      |
| **REPL**                 | ⏳ Not Started | Interactive interpreter                    |
| **Benchmarking Tool**    | ⏳ Not Started | Performance analysis                       |
| **VS Code Plugin**       | ⏳ Not Started | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin**    | ⏳ Not Started | Syntax highlighting, LSP client            |

**Project Structure Reference**:

```
yaoxiang/
├── src/
│   ├── tools/                    # Toolchain
│   │   ├── lsp/                  # LSP server
│   │   ├── fmt/                  # Formatter
│   │   ├── repl/                 # REPL
│   │   └── benchmark/            # Benchmarking
│   └── ...
├── extensions/                   # Editor extensions
│   ├── vscode/                   # VS Code
│   └── vim/                      # Vim/Neovim
```

### 6.4 Standard Library Development

**Suitable For**: Library developers, API designers, domain experts

**Standard Library Module Plan**:

| Module           | Priority | Description                      |
| ---------------- | -------- | -------------------------------- |
| `std.io`         | P0       | File IO, console input/output    |
| `std.string`     | P0       | String operations, formatting    |
| `std.list`       | P0       | List/array operations            |
| `std.dict`       | P0       | Dictionary/hash table            |
| `std.math`       | P0       | Math functions, constants        |
| `std.time`       | P1       | Time and date operations         |
| `std.net`        | P1       | Network programming, HTTP        |
| `std.concurrent` | P1       | Concurrency primitives, channels |
| `std.crypto`     | P2       | Cryptographic hashes, signatures |
| `std.json`       | P1       | JSON parsing/generation          |
| `std.regex`      | P2       | Regular expressions              |
| `std.database`   | P3       | Database connection              |
| `std.gui`        | P3       | Graphical interface (long-term)  |

**Design Principles**:

- Consistency: function names and behaviors for the same functionality remain consistent
- Simplicity: APIs should be intuitive and easy to use, avoiding over-engineering
- Performance: standard library functions should be efficient and avoid unnecessary copies
- Testability: every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable For**: Technical writers, educators, community managers

**Documentation Needed**:

| Document               | Status         | Description                               |
| ---------------------- | -------------- | ----------------------------------------- |
| Quick Start            | ✅ Completed   | 5-minute getting started guide            |
| Language Guide         | ✅ Completed   | Systematic learning of core concepts      |
| Language Specification | ✅ Completed   | Complete syntax and semantic definition   |
| Implementation Plan    | ✅ Completed   | Compiler implementation technical details |
| API Documentation      | ⏳ Not Started | Standard library API reference            |
| Tutorials              | ⏳ Not Started | Advanced tutorials and best practices     |
| Blog                   | ⏳ Not Started | Technical articles and design stories     |
| Translation            | ⏳ Not Started | Multi-language support                    |

### 6.6 Community Building

**Suitable For**: Community managers, event organizers, evangelists

**Community Activities**:

- Regular online meetups (monthly)
- Design and implementation discussion sessions (weekly)
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

1. **Understand the Project**: read the README and design documents
2. **Choose a Direction**: pick a contribution area based on your interests
3. **Set Up the Environment**: Rust 1.75+, cargo, git
4. **Find Tasks**: check the `good first issue` label on GitHub Issues
5. **Submit PR**: follow commit conventions, write tests
6. **Participate in Review**: review others' code and participate in discussions

**Commit Conventions**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: new feature
fix: bug fix
docs: documentation update
style: code formatting (no functional change)
refactor: refactoring
perf: performance optimization
test: tests
chore: build tools or auxiliary tools

# Examples
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**Code Style**:

- Follow the `rustfmt.toml` specification
- Ensure `cargo clippy` has no warnings
- Write necessary unit tests
- Update related documentation

---

## Appendix A: Language Cheat Sheet

### A.1 Keywords

| Keyword                 | Function                                               |
| ----------------------- | ------------------------------------------------------ |
| `pub`                   | Public export                                          |
| `use`                   | Import module                                          |
| `spawn`                 | Spawn marker                                           |
| `ref`                   | Shared ownership (compiler automatically picks Rc/Arc) |
| `mut`                   | Mutable variable                                       |
| `if/else if/else`       | Conditional branching                                  |
| `match`                 | Pattern matching                                       |
| `while/for`             | Loops                                                  |
| `return/break/continue` | Control flow                                           |
| `as`                    | Type conversion                                        |
| `in`                    | Membership test / list comprehension                   |
| `unsafe`                | unsafe code block (raw pointer)                        |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type`
> keyword was removed in RFC-010, unified into the `name: Type = value` syntax.

### A.3 Primitive Types

| Type     | Description       | Default Size |
| -------- | ----------------- | ------------ |
| `Void`   | Empty value       | 0 bytes      |
| `Bool`   | Boolean value     | 1 byte       |
| `Int`    | Signed integer    | 8 bytes      |
| `Uint`   | Unsigned integer  | 8 bytes      |
| `Float`  | Floating point    | 8 bytes      |
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

> Unary prefix operators (`!` `-` `+`) bind tightly, higher than all binary operators (Zig-style,
> see SPEC §2.2).

---

## Appendix B: Design Inspirations

YaoXiang's design draws on the excellent ideas from the following languages and projects:

| Source                          | Inspiration Point                                                                   |
| ------------------------------- | ----------------------------------------------------------------------------------- |
| **Rust**                        | Ownership model, zero-cost abstractions, type system                                |
| **Python**                      | Syntax style, readability, list comprehension                                       |
| **Idris/Agda**                  | Dependent types, type-driven development                                            |
| **Curry-Howard Correspondence** | Types as propositions, programs as proofs; unified theory of type systems and logic |
| **TypeScript**                  | Type annotations, runtime types                                                     |
| **MoonBit**                     | AI-friendly design, concise syntax                                                  |
| **Haskell**                     | Pure functional, pattern matching                                                   |
| **OCaml**                       | Type inference, variant types                                                       |

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang preserves Rust's memory safety and zero-cost abstractions, but uses simpler syntax and a
lower cognitive burden. The **spawn model** is more concise than Rust's `async/await`—just a single
`spawn` marker, with no need to manually manage Future and Pin. "The ten thousand things arise
together; we observe their return," making concurrent programming as intuitive as describing the
laws of nature. The **ownership model** (RFC-009 v9) uses Move + &T/&mut T tokens to replace
lifetime annotations, and uses type attributes (Dup/Linear) to replace the borrow checker. The
unified type syntax eliminates the conceptual fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What kind of development is YaoXiang suitable for?**

A: System programming, application development, web services, scripting tools, AI-assisted
programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing confusion caused by nesting
depth. This is a well-considered "AI-friendly" design decision.

**Q: When will the 1.0 version be released?**

A: v1.0 goal: production-ready. The release date depends on implementation progress; see
[Version Planning RFC](./rfc/003-version-planning.md) for details.

**Q: How do I contact the core team?**

A: Through GitHub Discussions or the Discord community channel. Core team members will respond
regularly.

---

> **Last Updated**: 2026-05-31
>
> **Document Version**: v2.0.0
>
> **License**: [MIT](LICENSE)

---

> "Yao and Xiang transform; the ten thousand things are born. Types evolve; programs are formed."
>
> May the design journey of YaoXiang walk alongside you.
