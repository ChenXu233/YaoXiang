# YaoXiang (爻象) Design Manifesto

> **Version**: v2.0.0 **Status**: Officially Released **Author**: Chenxu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Tao gives birth to one, one to two, two to three, and three to the myriad things." — _Tao Te
> Ching_
>
> Types are the Tao; all things are born from them.

---

## I. Why Create YaoXiang?

### 1.1 Filling the Language Gap

Throughout the long history of programming languages, we have witnessed the birth and evolution of
countless excellent languages: C brought the efficiency revolution to systems programming, Python
created a programming experience accessible to everyone, Rust proved that memory safety and
performance can coexist, and TypeScript made large-scale frontend projects maintainable. Yet, when
we survey today's language ecosystem, we still find an obvious fault line—**no single language can
simultaneously satisfy the following three core needs**:

| Need               | Problems with Existing Solutions                                                                                                                    |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Type Safety**    | Rust is too strict with a steep learning curve; TypeScript has optional types with no compile-time guarantees                                       |
| **Natural Syntax** | Rust's syntax is complex and obscure; Haskell's functional threshold is too high; traditional static languages are verbose and cumbersome           |
| **AI Friendly**    | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behaviors, limiting AI's accuracy in generating and modifying code |

YaoXiang was born precisely to fill this gap. We believe: **a programming language should be both
powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Practical Problems Solved

**Problem One: Fragmentation of Type Systems**

Today's programming languages exhibit severe fragmentation in their type systems. Statically-typed
languages pursue absolute correctness at compile time, but often at the cost of development
efficiency; dynamically-typed languages offer flexibility, but expose hard-to-maintain defects in
large projects. YaoXiang proposes a unified abstraction framework of "everything is a type," making
types the main thread running through language design rather than an afterthought patch.

**Problem Two: The Binary Choice Between Memory Safety and Performance**

For a long time, developers have had to make difficult trade-offs between memory safety and runtime
performance. While GC (garbage collection) liberates developers, it brings latency spikes and memory
overhead; while manual memory management is efficient, it is as dangerous as walking a tightrope.
YaoXiang adopts Rust-style ownership, eliminating data races and memory leaks at compile time, while
maintaining zero-cost abstractions and achieving high performance without GC.

**Problem Three: The Cognitive Burden of Asynchronous Programming**

Modern applications rely on networking and concurrency, yet asynchronous programming has always been
a programmer's nightmare. Nested callbacks, Promise chains, async/await syntax—each solution adds
complexity to code. YaoXiang redesigns the asynchronous model: use `spawn` to explicitly mark
parallel points; the compiler builds a dependency graph within that expression to execute in
parallel, and the caller synchronously blocks waiting for results (RFC-024). No callbacks, no
`await`, no function coloring.

**Problem Four: Bottlenecks in AI-Assisted Programming**

When AI began assisting developers in writing code, language design choices became critical.
Ambiguous syntax rules, implicit type conversions, complex syntactic sugar—these characteristics
that human programmers have grown accustomed to become obstacles for AI understanding and
generation. From day one, YaoXiang has taken "AI friendly" as a core design goal: strict indentation
rules, explicit code block boundaries, unambiguous syntactic structures, allowing AI to accurately
understand, generate, and modify code.

### 1.3 The Philosophical Roots of the Language

The name YaoXiang comes from "yao" (爻) and "xiang" (象) in the _I Ching_. "Yao" is the basic symbol
that composes hexagrams, symbolizing the changes of yin and yang, the interplay of motion and
stillness; "xiang" is the external manifestation of the essence of things, representing all
phenomena and encompassing everything.

This philosophical thought is reflected in every detail of the language design:

- **Unity**: Just as the simple symbols of yao and gua compose complex hexagrams, YaoXiang uses a
  few core concepts (types, functions, constructors) to build a complete programming model
- **Hierarchy**: Just as xiang has distinctions between innate and acquired, YaoXiang's type system
  has a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Variability**: Just as yin and yang flow and transform endlessly, YaoXiang supports dependent
  types, allowing types to evolve as values change
- **Recognizability**: Just as hexagrams can be interpreted and all things can be signified,
  YaoXiang provides complete type reflection capabilities, with full runtime type information
  available
- **Provability**: Just as hexagrams reveal the laws of things, YaoXiang's type system follows the
  Curry-Howard Isomorphism (types as propositions, programs as proofs); the process of type checking
  is the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any
feature proposal must be examined against these principles.

### 2.1 Principle One: Everything Is a Type

In YaoXiang's worldview, types are the highest-level abstraction units and the core concept running
through the language.

**Specific Manifestations**:

- **Values are instances of types**: `42` is an instance of type `Int`; `"hello"` is an instance of
  type `String`
- **Types themselves are also types**: `Type` is the language's only meta type keyword; the type of
  `Int` is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from
  `Int × Int` to `Int`
- **Modules are type compositions**: modules are namespace compositions containing functions and
  types

**Why Non-Negotiable**: A unified type abstraction simplifies language semantics, eliminates the
binary opposition between values and types, and allows the type system to become the guardian of
code correctness rather than a stumbling block.

### 2.2 Principle Two: Strict Structuring

YaoXiang's syntax design pursues "unambiguous, predictable, and easy to parse."

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are forbidden; code block boundaries are clear
  at a glance
- **Brackets cannot be omitted**: Function parameters must have parentheses, list elements must have
  commas
- **Code blocks must use curly braces**: Control flow like `if`, `while`, `for` must be wrapped with
  `{ }`
- **Minimal keyword set**: Only 18 core keywords are retained, rejecting syntactic sugar
  proliferation

**Why Non-Negotiable**: Strict structuring brings three key advantages—(1) IDE syntax highlighting
and code folding are more accurate; (2) AI code generation and modification accuracy improves
dramatically; (3) new learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: Generic functions are expanded into specific versions at compile time, with
  no vtable lookup overhead
- **Inlining optimization**: Simple functions are automatically inlined, eliminating function call
  overhead
- **Stack allocation priority**: Small objects are stack-allocated by default; heap allocation is
  used only when necessary
- **No GC**: The ownership model guarantees memory safety, with no runtime overhead from a garbage
  collector

**Why Non-Negotiable**: Performance is the lifeline of a programming language. Any design that
trades performance for convenience is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity go hand in hand. YaoXiang chooses immutability by default, making code
easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default; they cannot be modified after assignment
- Mutability must be explicitly declared with `mut` when needed
- References are immutable by default; mutable references require the `mut` marker
- Transfer of ownership means the original binding becomes invalid

**Why Non-Negotiable**: Immutability is the foundation of concurrency safety, the guarantee of code
readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types Are Data

Type information should not only exist at compile time, but should be fully available at runtime.

**Specific Capabilities**:

- Runtime type queries: any value can obtain its type information
- Type reflection: types themselves can be constructed and manipulated
- Pattern matching destructuring: type constructors can be used directly in pattern matching
- Generic specialization: the realized type of generic parameters can be obtained at runtime

**Why Non-Negotiable**: Complete type reflection capability is the foundation of metaprogramming,
and the cornerstone of high-performance frameworks and tools.

---

## III. Key Innovations and Features

While absorbing excellent features from existing languages, YaoXiang proposes the following
innovative designs.

### 3.1 Innovation One: Unified Type Syntax

**Type definitions in traditional languages** often require multiple keywords:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
trait Drawable { fn draw(&self, s: &Surface); }
```

**YaoXiang's unified syntax**: everything is `name: type = value`; `Type` is the only meta type
keyword.

```yaoxiang
# === Record Type ===

Point: Type = {
    x: Float,
    y: Float,
}

# Fields with default values
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

# === Interface (a record whose fields are all function types) ===

Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

Serializable: Type = {
    serialize: () -> String,
}

# === Interface implementation (interface name written inside the type body) ===

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

**Innovation Value**: No fragmentation of `fn`, `struct`, `enum`, `trait`, `impl` keywords—one
unified syntax covers all declarations.

### 3.2 Innovation Two: Constructors Are Types

**Value construction is completely identical to function calls**:

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

YaoXiang adopts a pure functional design, achieving object-method-call-like syntactic sugar through
currying, without introducing the `class` and `method` keywords.

```yaoxiang
# === Type definition ===

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

# Method syntactic sugar binding ([0] means bind to argument position 0)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# The two calling forms are completely equivalent
d1 = distance(p1, p2)     # Direct call to core function
d2 = p1.distance(p2)      # Method syntactic sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, waiting for the second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovation Value**: Pure functional design, no hidden `self` parameter, functions are values that
can be freely passed and composed.

### 3.4 Innovation Four: Spawn Model

> "All things arise together; I observe their return." — _I Ching, Fu Gua_
>
> The spawn model takes its inspiration from this, describing a programming paradigm: developers
> describe logic with synchronous, sequential thinking, while the language runtime causes the
> computational units within to execute concurrently, automatically and efficiently, like all things
> arising together, ultimately unifying and coordinating them.

**Three Core Principles**:

| Principle                | Description                                                              |
| ------------------------ | ------------------------------------------------------------------------ |
| **Synchronous Syntax**   | What you see is what you get, sequential code                            |
| **Concurrent Essence**   | The runtime automatically extracts parallelism                           |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology**:

| Official Term       | Corresponding Syntax            | Explanation                                                                                       |
| ------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------- |
| **Spawn function**  | `spawn (params) => body`        | Defines a computational unit that can participate in spawn execution                              |
| **Spawn block**     | `spawn { a(), b() }`            | Explicitly declared concurrent domain; tasks within the block execute as spawn                    |
| **Spawn loop**      | `spawn for x in xs { ... }`     | Data parallelism; the loop body executes as spawn over all elements                               |
| **Spawn value**     | No independent handle (RFC-024) | No `Async(T)`/future handle; the caller synchronously blocks waiting for the result               |
| **Spawn graph**     | DAG within expression (RFC-024) | The stage where spawn happens; **only built within spawn expressions**, no whole-program analysis |
| **Spawn scheduler** | Runtime task scheduler          | The intelligent center that coordinates all things to spawn at the right moment                   |

> **See also**:
> [RFC-024 Spawn-based Concurrency Runtime Semantics](../rfc/accepted/024-concurrency-model.md)
> (orthogonal syntax parts see [RFC-032](../rfc/review/032-spawn-unified-expression.md))

```yaoxiang
# === Spawn function ===
# A function marked with spawn
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === Spawn block ===
# Expressions within spawn { } are forced to execute in parallel
compute_all: () -> (Int, Int, Int) spawn = {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    return (a, b, c)
}

# === Explicit spawn + synchronous wait ===
main: () -> Void = {
    # Parallel points must be explicitly marked: RFC-024 has no whole-program automatic parallelism
    (users, posts) = spawn {
        fetch_data("https://api.example.com/users"),
        fetch_data("https://api.example.com/posts")
    }

    # The spawn expression blocks synchronously during evaluation, directly yielding the result (no future handle)
    print(users.length + posts.length)
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

- See [RFC-024 Spawn-based Concurrency Runtime Semantics](../rfc/accepted/024-concurrency-model.md)

**Innovation Value**: The cognitive burden of asynchronous programming drops to zero; code
readability is identical to synchronous code, while achieving high-performance parallel execution
efficiency.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, achieving true type-driven development.

```yaoxiang
# Matrix type: dimensions determined at compile time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time computation: factorial(3) = 6
arr: Array(Int, factorial(3)) = Array(Int, 6)()

# Compile-time dimension validation
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimensions don't match
```

**Innovation Value**: Capture more errors at compile time, achieving more precise type guarantees.

### 3.6 Innovation Six: Minimal Keyword Design

YaoXiang defines only 18 core keywords, far fewer than mainstream languages:

```
pub    use    spawn
ref    mut    if     else
match  while  for    return
break  continue as     in     unsafe
and    or
```

| Compared Language | Keyword Count |
| ----------------- | ------------- |
| YaoXiang          | **18**        |
| Rust              | 51+           |
| Python            | 35            |
| TypeScript        | 64+           |
| Go                | 25            |

> **About `pub`**: The lexer still recognizes it as a keyword (`src/frontend/core/lexer/state.rs`),
> but per [RFC-029 Module Semantics](../rfc/accepted/029-module-semantics.md), the language
> **introduces no visibility mechanism**—no `pub`, no `private`, no `export`. `pub` produces no
> visibility effect.

**Innovation Value**: Lower memory burden, more consistent syntactic style, easier-to-parse
syntactic structures.

---

## IV. Initial Syntax Preview

The following code examples showcase the style of YaoXiang, helping you quickly appreciate its
design aesthetics.

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

# Interface type (a record whose fields are all functions)
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

# &T / &mut T borrow token (zero compile-time cost)
p2.print()           # Compiler automatically creates a &Point borrow token
p2.shift(1.0, 1.0)   # Compiler automatically creates a &mut Point borrow token

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
borrow token   default   shared      deep copy  raw pointer
zero cost     zero copy  auto Rc/Arc  explicit  system-level
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
# spawn marks an asynchronous function
fetch_api: (url: String) -> JSON spawn = {
    response = HTTP.get(url)
    return JSON.parse(response.body)
}

# Concurrent construction block: explicit parallelism
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

## V. Roadmap and Pending Items

### 5.1 Decided Design Decisions

The following decisions have been thoroughly discussed and reviewed, and **are no longer subject to
change**:

| Module                 | Decision                       | Description                                                                                                                             |
| ---------------------- | ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| **Type System**        | Everything is a type           | Values, functions, modules, and generics are all types                                                                                  |
| **Type Syntax**        | Unified `name: type = value`   | One declaration form covers all cases; `Type` is the only meta type keyword                                                             |
| **Keywords**           | 18 core keywords               | Excludes `type`/`fn`/`struct`/`enum`/`trait`/`impl`                                                                                     |
| **Function Syntax**    | Signature + expression         | `name: (params) -> ReturnType = body`                                                                                                   |
| **Method Binding**     | RFC-004 curried binding        | `Type.method = function[position]`                                                                                                      |
| **Asynchronous Model** | Spawn model                    | `spawn` explicitly marks parallel points; regular code runs sequentially; DAG analysis is limited to within spawn expressions (RFC-024) |
| **Memory Management**  | Ownership model (RFC-009 v9)   | Move + &T/&mut T borrow token + ref + clone + unsafe, no GC                                                                             |
| **File as Module**     | Module system                  | Each `.yx` file is a module                                                                                                             |
| **Main Function**      | `main: () -> Void`             | Program entry point                                                                                                                     |
| **Thread Safety**      | ref automatically picks Rc/Arc | Compiler escape analysis, transparent to users                                                                                          |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang Implementation Roadmap (Example)             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: Rust AOT│
│        ✅ Completed                  │ (current stage)               Compiler  │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←─────── v1.0: YaoXiang JIT Compiler ←──── v2.0:│
│        (self-hosted)                   (self-hosted)                   YaoXiang│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## VI. How to Contribute

YaoXiang is a language born in the community, grown by the community, and serving the community. We
sincerely invite every developer passionate about programming language design to join this journey
of exploration.

### 6.1 Design Discussions

**Suitable for**: Programming language theory researchers, type system enthusiasts, language design
fanatics

**How to Participate**:

- **GitHub Discussions**: Participate in discussions under the "Language Design" category
- **Design Proposals (RFCs)**: Propose design documents for new features, following the template in
  the `rfcs/` directory
- **Syntax Review**: Suggest improvements to existing syntax designs or identify potential issues

| **Current Hot Topics**: | | | | - Design and implementation of the macro system | | - Interface
type mechanism | | - Error handling syntax optimization | | - Standard library API design |

**Submitting a Design Proposal**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros and cons analysis, alternatives)
3. Open a Pull Request for community review
4. Merged or rejected after review by the core team

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, systems programmers, performance optimization experts

**Current Implementation Priorities** (sorted by priority):

| Priority | Module                       | Description                                         | Difficulty |
| -------- | ---------------------------- | --------------------------------------------------- | ---------- |
| P0       | **Bytecode Virtual Machine** | VM instruction refinement, performance optimization | Medium     |
| P0       | **Runtime Memory**           | GC implementation, memory allocator                 | High       |
| P0       | **Async Runtime**            | Complete implementation of the spawn model          | High       |
| P1       | Standard Library             | IO, String, List, Concurrent                        | Medium     |
| P1       | JIT Compiler                 | Cranelift integration                               | High       |
| P2       | AOT Compiler                 | LLVM/Cranelift backend                              | High       |
| P3       | Self-hosted Compiler         | Rewrite in YaoXiang                                 | Very High  |

**Tech Stack**:

- **Implementation Language**: Rust (current stage)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Test Framework**: Rust `#[test]` + `cargo nextest`

**Getting Started Contributing**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand the architecture design
2. Choose a module of interest under the `src/` directory
3. Check `tests/unit/` to understand test requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, productivity tool seekers

**Tools That Need Development**:

| Tool                     | Status         | Description                                |
| ------------------------ | -------------- | ------------------------------------------ |
| **LSP Server**           | ⏳ Not started | Language Server Protocol support           |
| **Debugger Integration** | ⏳ Not started | GDB/LLDB integration                       |
| **Formatter**            | ⏳ Not started | `yx format`                                |
| **Package Manager**      | ⏳ Not started | Dependency management, version resolution  |
| **Package Registry**     | ⏳ Not started | Central registry or decentralized          |
| **REPL**                 | ⏳ Not started | Interactive interpreter                    |
| **Benchmarking Tool**    | ⏳ Not started | Performance analysis                       |
| **VS Code Plugin**       | ⏳ Not started | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin**    | ⏳ Not started | Syntax highlighting, LSP client            |

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

**Suitable for**: Library developers, API designers, domain experts

**Standard Library Module Plan**:

| Module           | Priority | Description                       |
| ---------------- | -------- | --------------------------------- |
| `std.io`         | P0       | File IO, console input/output     |
| `std.string`     | P0       | String operations, formatting     |
| `std.list`       | P0       | List/array operations             |
| `std.dict`       | P0       | Dictionary/hash table             |
| `std.math`       | P0       | Math functions, constants         |
| `std.time`       | P1       | Time and date operations          |
| `std.net`        | P1       | Network programming, HTTP         |
| `std.concurrent` | P1       | Concurrency primitives, channels  |
| `std.crypto`     | P2       | Cryptographic hashing, signatures |
| `std.json`       | P1       | JSON parsing/generation           |
| `std.regex`      | P2       | Regular expressions               |
| `std.database`   | P3       | Database connections              |
| `std.gui`        | P3       | Graphical interface (long-term)   |

**Design Principles**:

- Consistency: Functions with the same purpose have consistent names and behaviors
- Simplicity: APIs should be intuitive and easy to use, avoiding over-engineering
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documentation That Needs Contribution**:

| Document               | Status         | Description                               |
| ---------------------- | -------------- | ----------------------------------------- |
| Quick Start            | ✅ Complete    | 5-minute getting started guide            |
| Language Guide         | ✅ Complete    | Systematic learning of core concepts      |
| Language Specification | ✅ Complete    | Complete syntax and semantic definition   |
| Implementation Plan    | ✅ Complete    | Compiler implementation technical details |
| API Documentation      | ⏳ Not started | Standard library API reference            |
| Tutorials              | ⏳ Not started | Advanced tutorials and best practices     |
| Blog                   | ⏳ Not started | Technical articles and design stories     |
| Translation            | ⏳ Not started | Multi-language support                    |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

**Community Activities**:

- Regular online Meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution Sprints (quarterly)
- Offline gatherings and conference talks

**Communication Channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Issue reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guide

**How to Start Contributing**:

1. **Understand the Project**: Read the README and design documents
2. **Choose a Direction**: Pick a contribution area based on your interest
3. **Set Up Environment**: Rust 1.75+, cargo, git
4. **Find a Task**: Check the `good first issue` label in GitHub Issues
5. **Submit a PR**: Follow commit conventions, write tests
6. **Participate in Review**: Review others' code, participate in discussions

**Commit Convention**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: new feature
fix: bug fix
docs: documentation update
style: code formatting (does not affect functionality)
refactor: refactor
perf: performance optimization
test: test
chore: build tools or auxiliary tools

# Examples
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**Code Style**:

- Follow `rustfmt.toml` specifications
- Ensure `cargo clippy` has no warnings
- Write necessary unit tests
- Update related documentation

---

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword                 | Role                                                      |
| ----------------------- | --------------------------------------------------------- |
| `pub`                   | Reserved keyword, produces no visibility effect (RFC-029) |
| `use`                   | Import module                                             |
| `spawn`                 | Spawn marker                                              |
| `ref`                   | Shared ownership (compiler automatically picks Rc/Arc)    |
| `mut`                   | Mutable variable                                          |
| `if/else if/else`       | Conditional branch                                        |
| `match`                 | Pattern matching                                          |
| `while/for`             | Loop                                                      |
| `return/break/continue` | Control flow                                              |
| `as`                    | Type conversion                                           |
| `in`                    | Membership test/list comprehension                        |
| `unsafe`                | unsafe code block (raw pointer)                           |
| `and`                   | Logical AND                                               |
| `or`                    | Logical OR                                                |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type`
> keyword was removed in RFC-010, unifying with the `name: Type = value` syntax.

### A.3 Primitive Types

| Type     | Description        | Default Size |
| -------- | ------------------ | ------------ |
| `Void`   | Empty value        | 0 bytes      |
| `Bool`   | Boolean value      | 1 byte       |
| `Int`    | Signed integer     | 8 bytes      |
| `Uint`   | Unsigned integer   | 8 bytes      |
| `Float`  | Float point number | 8 bytes      |
| `String` | UTF-8 string       | Variable     |
| `Char`   | Unicode character  | 4 bytes      |
| `Bytes`  | Raw bytes          | Variable     |

### A.4 Operator Precedence

| Precedence | Operator                    | Associativity |
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

| Source                       | Inspiration Point                                                                       |
| ---------------------------- | --------------------------------------------------------------------------------------- |
| **Rust**                     | Ownership model, zero-cost abstractions, type system                                    |
| **Python**                   | Syntax style, readability, list comprehension                                           |
| **Idris/Agda**               | Dependent types, type-driven development                                                |
| **Curry-Howard Isomorphism** | Types as propositions, programs as proofs; the unified theory of type systems and logic |
| **TypeScript**               | Type annotations, runtime types                                                         |
| **MoonBit**                  | AI-friendly design, concise syntax                                                      |
| **Haskell**                  | Pure functional, pattern matching                                                       |
| **OCaml**                    | Type inference, variant types                                                           |

---

## Appendix C: FAQ

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions, but adopts a simpler syntax and
lower cognitive burden. The **Spawn Model** is more concise than Rust's `async/await`—just a single
`spawn` marker, with no need to manually manage Future and Pin. "All things arise together; I
observe their return," making concurrent programming as intuitive as describing the laws of nature.
The **Ownership Model** (RFC-009 v9) uses Move + &T/&mut T borrow tokens instead of lifetime
annotations, and type attributes (Dup/Linear) instead of the borrow checker. The unified type syntax
eliminates the conceptual fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What kind of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted
programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing the confusion caused by deep
nesting. This is a well-considered "AI friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. The release date depends on implementation progress; the current
version number is determined by the `Cargo.toml` at the root of the repository.

**Q: How to contact the core team?**

A: Through GitHub Discussions or the Discord community channel. Core team members will respond
regularly.

---

> **Last Updated**: 2026-10-02
>
> **Document Version**: v2.0.0
>
> **License**: MIT

---

> "Yao and xiang transform; all things are born. Types evolve; programs are made."
>
> May the design journey of YaoXiang walk alongside you.
