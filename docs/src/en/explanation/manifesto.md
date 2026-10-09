---
frontmatter: (none)
---

# YaoXiang Design Manifesto

> **Version**: v2.0.0 **Status**: Officially Released **Author**: Chenxu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Dao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to the myriad things." — _Tao Te Ching_
>
> Types are like the Dao; all things are born from them.

---

## I. Why Create YaoXiang?

### 1.1 Filling a Language Gap

Throughout the long history of programming languages, we have witnessed the birth and evolution of
countless excellent languages: C brought the efficiency revolution to systems programming, Python
created a programming experience accessible to everyone, Rust proved that memory safety and
performance can coexist, and TypeScript made large-scale frontend projects maintainable. However,
when we examine today's language ecosystem, we still find an obvious gap — **no single language can
simultaneously satisfy the following three core needs**:

| Need               | Problems with Existing Solutions                                                                                                                         |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Type Safety**    | Rust is overly strict with a steep learning curve; TypeScript has optional types and cannot provide compile-time guarantees                              |
| **Natural Syntax** | Rust's syntax is complex and obscure; Haskell's functional threshold is too high; traditional static languages are verbose and cumbersome                |
| **AI-Friendly**    | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behaviors, limiting the accuracy of AI code generation and modification |

YaoXiang was born precisely to fill this gap. We believe: **a programming language should be both
powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Problems to Solve

**Problem One: Fragmentation of Type Systems**

Today's programming languages exhibit serious fragmentation in their type systems. Statically-typed
languages pursue absolute correctness at compile time, but often at the expense of development
efficiency; dynamically-typed languages provide flexibility, but expose hard-to-maintain flaws in
large projects. YaoXiang proposes a unified abstraction framework of "everything is a type", making
types the main thread running through language design, rather than patches added after the fact.

**Problem Two: The Binary Choice Between Memory Safety and Performance**

For a long time, developers have had to make a difficult choice between memory safety and runtime
performance. GC (garbage collection) frees developers but brings latency fluctuations and memory
overhead; manual memory management is efficient but as dangerous as walking a tightrope. YaoXiang
adopts Rust-style ownership model, eliminating data races and memory leaks at compile-time, while
maintaining zero-cost abstractions and achieving high performance without GC.

**Problem Three: The Cognitive Burden of Asynchronous Programming**

Modern applications are inseparable from networking and concurrency, and asynchronous programming
has always been a programmer's nightmare. Nested callback functions, Promise chains, async/await
syntax — each solution adds complexity to the code. YaoXiang redesigns the asynchronous model: use
`spawn` to explicitly mark parallel points, the compiler builds a dependency graph within that
expression for parallel execution, and the caller synchronously blocks waiting for results
(RFC-024). No callbacks, no `await`, no function coloring.

**Problem Four: The Bottleneck of AI-Assisted Programming**

As AI begins to assist developers in writing code, language design choices become crucial. Ambiguous
syntax rules, implicit type conversions, complex syntactic sugar — these features that human
programmers have grown accustomed to become obstacles for AI to understand and generate. From the
very beginning, YaoXiang has taken "AI-friendly" as a core goal: strict indentation rules, clear
code block boundaries, unambiguous syntax structures, enabling AI to accurately understand,
generate, and modify code.

### 1.3 The Philosophical Foundations of the Language

The name YaoXiang comes from "Yao" (爻) and "Xiang" (象) in the _Book of Changes_ (I Ching). "Yao"
is the basic symbol that composes hexagrams, symbolizing the interplay of yin and yang, the arising
of motion and stillness; "Xiang" is the external manifestation of the essence of things,
representing all phenomena and embracing everything.

This philosophical thought is embodied in every detail of the language design:

- **Unity**: Just as the simple symbols of yao compose complex hexagrams, YaoXiang uses a few core
  concepts (types, functions, constructors) to build a complete programming model
- **Hierarchy**: Just as xiang has the distinction of innate and acquired, YaoXiang's type system
  has a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Changeability**: Just as yin and yang flow with endless transformation, YaoXiang supports
  dependent types, allowing types to evolve as values change
- **Recognizability**: Just as hexagrams can be interpreted and all things can be represented,
  YaoXiang provides complete type reflection capabilities with fully available runtime type
  information
- **Provability**: Just as hexagrams reveal the laws of things, YaoXiang's type system follows the
  Curry-Howard isomorphism (types as propositions, programs as proofs), and the type-checking
  process is the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any
feature proposal must pass the test of these principles.

### 2.1 Principle One: Everything Is a Type

In YaoXiang's worldview, types are the highest-level abstraction unit and the core concept running
through the language.

**Specific Embodiments**:

- **Values are instances of types**: `42` is an instance of `Int`, `"hello"` is an instance of
  `String`
- **Types themselves are also types**: `Type` is the language's sole meta type keyword; the type of
  `Int` is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from
  `Int × Int` to `Int`
- **Modules are combinations of types**: modules are namespace combinations containing functions and
  types

**Reason for Non-Negotiability**: Unified type abstraction can simplify language semantics,
eliminate the binary opposition between values and types, and make the type system the guardian of
code correctness rather than an obstacle.

### 2.2 Principle Two: Strict Structuring

YaoXiang's syntax design pursues "unambiguous, predictable, easy to parse".

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are forbidden; code block boundaries are clear
  at a glance
- **Brackets cannot be omitted**: function parameters must have parentheses, list elements must have
  commas
- **Code blocks must use braces**: control flows like `if`, `while`, `for` must be wrapped in `{ }`
- **Minimal number of keywords**: only 18 core keywords are retained, rejecting syntactic sugar
  proliferation

**Reason for Non-Negotiability**: Strict structuring brings three key advantages — (1) IDE syntax
highlighting and code folding are more accurate; (2) the accuracy of AI code generation and
modification is greatly improved; (3) new learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: generic functions are expanded into concrete versions at compile-time, with
  no virtual table lookup overhead
- **Inline optimization**: simple functions are automatically inlined, eliminating function call
  overhead
- **Stack allocation priority**: small objects are stack-allocated by default; heap allocation is
  used only when necessary
- **No GC**: the ownership model guarantees memory safety without the runtime overhead of a garbage
  collector

**Reason for Non-Negotiability**: Performance is the survival baseline of a programming language.
Any design that trades performance for convenience is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity go hand in hand. YaoXiang chooses immutability by default to make code
easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default and cannot be modified after assignment
- When mutability is needed, it must be explicitly declared with `mut`
- References are immutable by default; mutable references need a `mut` marker
- The transfer of ownership means the original binding becomes invalid

**Reason for Non-Negotiability**: Immutability is the foundation of concurrency safety, the
guarantee of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types Are Data

Type information should not only exist at compile-time but should be fully available at runtime.

**Specific Capabilities**:

- Runtime type queries: any value can obtain its type information
- Type reflection: types themselves can be constructed and manipulated
- Pattern matching destructuring: type constructors can be used directly in pattern matching
- Generic specialization: the concrete types of generic parameters can be obtained at runtime

**Reason for Non-Negotiability**: Complete type reflection capability is the foundation of
metaprogramming and the cornerstone of high-performance frameworks and tools.

---

## III. Key Innovations and Features

While absorbing the excellent features of existing languages, YaoXiang proposes the following
innovative designs.

### 3.1 Innovation One: Unified Type Syntax

**Traditional language type definitions** often require multiple keywords:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
trait Drawable { fn draw(&self, s: &Surface); }
```

**YaoXiang's unified syntax**: everything is `name: type = value`, with `Type` as the sole meta type
keyword.

```yaoxiang
# === Record type ===

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

# === Generic type ===

Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

# === Interface (record where all fields are function types) ===

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

**Innovation Value**: No fragmentation of `fn`, `struct`, `enum`, `trait`, `impl` keywords — a
single unified syntax covers all declarations.

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
currying, without introducing `class` and `method` keywords.

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

# Method syntactic sugar binding ([0] means bind to the 0th parameter position)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Two calling styles are completely equivalent
d1 = distance(p1, p2)     # Direct call to core function
d2 = p1.distance(p2)      # Method syntactic sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, awaiting second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovation Value**: Pure functional design, no hidden `self` parameter, functions are values that
can be freely passed and composed.

### 3.4 Innovation Four: The Spawn Model

> "The myriad things arise together; thereby we observe their cycles." — _I Ching · Fu Gua_
>
> The spawn model takes its inspiration from this, describing a programming paradigm: developers
> describe logic with synchronous, sequential thinking, while the language runtime makes the
> computational units within it, like all things arising together, automatically and efficiently
> execute concurrently, and unify in cooperation at the end.

**Three Core Principles**:

| Principle               | Description                                                               |
| ----------------------- | ------------------------------------------------------------------------- |
| **Synchronous Syntax**  | What you see is what you get — sequential code                            |
| **Concurrent Essence**  | Runtime automatically extracts parallelism                                |
| **Unified Cooperation** | Results automatically aggregate when needed, ensuring logical correctness |

**Terminology System**:

| Official Term       | Corresponding Syntax            | Explanation                                                                                      |
| ------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------ |
| **spawn function**  | `spawn (params) => body`        | Defines a computational unit that can participate in spawn execution                             |
| **spawn block**     | `spawn { a(), b() }`            | Explicitly declared concurrent realm, tasks within the block execute in spawn                    |
| **spawn loop**      | `spawn for x in xs { ... }`     | Data parallelism, loop body executes in spawn on all elements                                    |
| **spawn value**     | No independent handle (RFC-024) | No `Async(T)`/future handle; caller synchronously blocks waiting for result                      |
| **spawn graph**     | DAG within expression (RFC-024) | The stage where spawn occurs; **built only within spawn expressions**, no whole-program analysis |
| **spawn scheduler** | Runtime task scheduler          | The intelligent hub that coordinates all things, making them spawn at the right time             |

> **See details**:
> [RFC-024 spawn-based concurrent runtime semantics](../rfc/accepted/024-concurrency-model.md)
> (syntax orthogonal part see [RFC-032](../rfc/review/032-spawn-unified-expression.md))

```yaoxiang
# === spawn function ===
# Function marked with spawn
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === spawn block ===
# Expressions inside spawn { } are forced to execute in parallel
compute_all: () -> (Int, Int, Int) spawn = {
    (a, b, c) = spawn {
        heavy_calc(1),    # task 1
        heavy_calc(2),    # task 2
        another_calc(3)   # task 3
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

    # spawn expression synchronously blocks when evaluated, directly returning the result (no future handle)
    print(users.length + posts.length)
}
```

**Thread Safety**:

```yaoxiang
# ref keyword automatically handles thread safety (compiler automatically chooses Rc/Arc)
main: () -> Void = {
    counter = ref SafeCounter(0)

    # Cross-task sharing: compiler automatically chooses Arc
    spawn {
        counter.increment()
    }
    spawn {
        counter.increment()
    }
}
```

**Technical Documentation**:

- See details
  [RFC-024 spawn-based concurrent runtime semantics](../rfc/accepted/024-concurrency-model.md)

**Innovation Value**: The cognitive burden of asynchronous programming is reduced to zero, code
readability is identical to synchronous code, while achieving high-performance parallel execution
efficiency.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, achieving truly type-driven development.

```yaoxiang
# Matrix type: dimensions determined at compile-time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time calculation: factorial(3) = 6
arr: Array(Int, factorial(3)) = Array(Int, 6)()

# Compile-time dimension verification
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimension mismatch
```

**Innovation Value**: Catch more errors at compile-time, achieving more precise type guarantees.

### 3.6 Innovation Six: Minimal Keyword Design

YaoXiang defines only 17 core keywords, far fewer than mainstream languages:

```
use    spawn  ref
mut    if     else   match
while  for    return break
continue as    in    unsafe
and    or
```

| Comparison Language | Number of Keywords |
| ------------------- | ------------------ |
| YaoXiang            | **17**             |
| Rust                | 51+                |
| Python              | 35                 |
| TypeScript          | 64+                |
| Go                  | 25                 |

> **Regarding `pub`**: According to
> [RFC-029 module semantics](../rfc/accepted/029-module-semantics.md), the language **introduces no
> visibility mechanism** — no `pub`, no `private`, no `export`. RFC-029g has removed this keyword
> entirely: `pub` is now just an ordinary identifier (the old syntax `pub x = 1` reports E1001), and
> the keyword count has changed from 18 to 17.

**Innovation Value**: Lower memory burden, more consistent syntax style, easier-to-parse syntax
structure.

---

## IV. Preliminary Syntax Preview

The following code examples showcase the style of YaoXiang to help you quickly appreciate its design
aesthetics.

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

# Interface type (record where all fields are functions)
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

# &T / &mut T tokens (zero overhead at compile-time)
p2.print()           # compiler automatically creates &Point token
p2.shift(1.0, 1.0)  # compiler automatically creates &mut Point token

# ref: shared ownership (compiler automatically chooses Rc/Arc)
shared = ref p2      # share across scopes

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
Borrow token    Default    Shared hold  Deep copy  Raw pointer
Zero cost       Zero copy  Auto Rc/Arc  Explicit  System-level
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

# Handle with match
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

# Concurrent block: explicit parallelism
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

The following decisions have been fully discussed and reviewed, and **are no longer subject to
change**:

| Module                 | Decision                         | Description                                                                                                                        |
| ---------------------- | -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **Type System**        | Everything is a type             | Values, functions, modules, generics are all types                                                                                 |
| **Type Syntax**        | Unified `name: type = value`     | One declaration form covers all cases, `Type` is the sole meta type keyword                                                        |
| **Keywords**           | 18 core keywords                 | Excluding `type`/`fn`/`struct`/`enum`/`trait`/`impl`                                                                               |
| **Function Syntax**    | Signature + expression           | `name: (params) -> ReturnType = body`                                                                                              |
| **Method Binding**     | RFC-004 curried binding          | `Type.method = function[position]`                                                                                                 |
| **Asynchronous Model** | Spawn model                      | `spawn` explicitly marks parallel points; regular code executes sequentially, DAG analysis only within spawn expressions (RFC-024) |
| **Memory Management**  | Ownership model (RFC-009 v9)     | Move + &T/&mut T tokens + ref + clone + unsafe, no GC                                                                              |
| **File as Module**     | Module system                    | Each `.yx` file is a module                                                                                                        |
| **Main Function**      | `main: () -> Void`               | Program entry point                                                                                                                |
| **Thread Safety**      | ref automatically chooses Rc/Arc | Compiler escape analysis, transparent to users                                                                                     |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang Implementation Roadmap (Example)              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: Rust  │
│        ✅ Completed                    │ (current stage)           AOT       │
│                                      │                           Compiler   │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←─────── v1.0: YaoXiang JIT Compiler ←──── v2.0:│
│        (Self-hosting)                     (Self-hosting)              YaoXiang│
│                                                                         AOT  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## VI. How to Contribute

YaoXiang is a language born in the community, grown in the community, and serving the community. We
sincerely invite every developer who loves programming language design to join this exploration
journey.

### 6.1 Design Discussions

**Suitable for**: Programming language theory researchers, type system enthusiasts, language design
fanatics

**How to participate**:

- **GitHub Discussions**: Participate in discussions in the "Language Design" category
- **Design Proposals (RFCs)**: Propose design documents for new features, following the template in
  the `rfcs/` directory
- **Syntax Review**: Suggest improvements or identify potential issues in existing syntax designs

| **Current hot topics**: | | | | - Design and implementation of macro systems | | - Interface type
mechanism | | - Error handling syntax optimization | | - Standard library API design |

**Submitting Design Proposals**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros and cons analysis, alternatives)
3. Initiate a Pull Request for community review
4. Merge or reject after deliberation by the core team

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, systems programmers, performance optimization experts

**Current Implementation Focus** (sorted by priority):

| Priority | Module                | Description                                         | Difficulty     |
| -------- | --------------------- | --------------------------------------------------- | -------------- |
| P0       | **Bytecode VM**       | VM instruction refinement, performance optimization | Medium         |
| P0       | **Runtime Memory**    | GC implementation, memory allocator                 | High           |
| P0       | **Async Runtime**     | Complete implementation of the spawn model          | High           |
| P1       | Standard Library      | IO, String, List, Concurrent                        | Medium         |
| P1       | JIT Compiler          | Cranelift integration                               | High           |
| P2       | AOT Compiler          | LLVM/Cranelift backend                              | High           |
| P3       | Self-hosting Compiler | Rewrite using YaoXiang                              | Extremely high |

**Tech Stack**:

- **Implementation language**: Rust (current stage)
- **Code generation**: Cranelift or LLVM
- **Build tool**: Cargo
- **Testing framework**: Rust `#[test]` + `cargo nextest`

**Start Contributing**:

1. Check `docs/YaoXiang-implementation-plan.md` to understand the architecture design
2. Choose a module of interest under the `src/` directory
3. Check `tests/unit/` to understand testing requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, efficiency tool seekers

**Tools to Develop**:

| Tool                     | Status         | Description                                |
| ------------------------ | -------------- | ------------------------------------------ |
| **LSP Server**           | ⏳ Not started | Language Server Protocol support           |
| **Debugger Integration** | ⏳ Not started | GDB/LLDB integration                       |
| **Formatter**            | ⏳ Not started | `yx format`                                |
| **Package Manager**      | ⏳ Not started | Dependency management, version resolution  |
| **Package Registry**     | ⏳ Not started | Central registry or decentralized          |
| **REPL**                 | ⏳ Not started | Interactive interpreter                    |
| **Benchmark Tool**       | ⏳ Not started | Performance analysis                       |
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
│   │   └── benchmark/            # Benchmark
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
| `std.math`       | P0       | Mathematical functions, constants |
| `std.time`       | P1       | Time and date operations          |
| `std.net`        | P1       | Network programming, HTTP         |
| `std.concurrent` | P1       | Concurrency primitives, channels  |
| `std.crypto`     | P2       | Encryption hash, signatures       |
| `std.json`       | P1       | JSON parsing/generation           |
| `std.regex`      | P2       | Regular expressions               |
| `std.database`   | P3       | Database connection               |
| `std.gui`        | P3       | Graphical interface (long-term)   |

**Design Principles**:

- Consistency: Functions with the same functionality should have consistent names and behavior
- Simplicity: APIs should be intuitive and easy to use, avoiding over-engineering
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documentation Needed**:

| Documentation          | Status         | Description                               |
| ---------------------- | -------------- | ----------------------------------------- |
| Quick Start            | ✅ Complete    | 5-minute getting started guide            |
| Language Guide         | ✅ Complete    | Systematic learning of core concepts      |
| Language Specification | ✅ Complete    | Complete syntax and semantics definition  |
| Implementation Plan    | ✅ Complete    | Compiler implementation technical details |
| API Documentation      | ⏳ Not started | Standard library API reference            |
| Tutorials              | ⏳ Not started | Advanced tutorials and best practices     |
| Blog                   | ⏳ Not started | Technical articles and design stories     |
| Translation            | ⏳ Not started | Multi-language support                    |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

**Community Activities**:

- Regular online meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution sprints (quarterly)
- Offline gatherings and conference talks

**Communication Channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Issue reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contributing Guidelines

**How to Start Contributing**:

1. **Understand the project**: Read the README and design documents
2. **Choose a direction**: Select a contribution area based on your interests
3. **Set up the environment**: Rust 1.75+, cargo, git
4. **Find tasks**: Check the `good first issue` label on GitHub Issues
5. **Submit PR**: Follow commit conventions, write tests
6. **Participate in review**: Review others' code, participate in discussions

**Commit Conventions**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: New feature
fix: Bug fix
docs: Documentation update
style: Code formatting (no functional impact)
refactor: Refactor
perf: Performance optimization
test: Test
chore: Build tool or auxiliary tool

# Examples
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**Code Style**:

- Follow the `rustfmt.toml` conventions
- Ensure `cargo clippy` has no warnings
- Write necessary unit tests
- Update relevant documentation

---

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword                 | Purpose                                                                   |
| ----------------------- | ------------------------------------------------------------------------- |
| ~~`pub`~~               | Removed (RFC-029g) — no longer a keyword, reverted to ordinary identifier |
| `use`                   | Import module                                                             |
| `spawn`                 | Spawn marker                                                              |
| `ref`                   | Shared ownership (compiler automatically chooses Rc/Arc)                  |
| `mut`                   | Mutable variable                                                          |
| `if/else if/else`       | Conditional branching                                                     |
| `match`                 | Pattern matching                                                          |
| `while/for`             | Loop                                                                      |
| `return/break/continue` | Control flow                                                              |
| `as`                    | Type conversion                                                           |
| `in`                    | Membership check / list comprehension                                     |
| `unsafe`                | unsafe code block (raw pointer)                                           |
| `and`                   | Logical AND                                                               |
| `or`                    | Logical OR                                                                |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type`
> keyword was removed in RFC-010, unified into the `name: Type = value` syntax.

### A.3 Primitive Types

| Type     | Description           | Default Size |
| -------- | --------------------- | ------------ |
| `Void`   | Empty value           | 0 bytes      |
| `Bool`   | Boolean value         | 1 byte       |
| `Int`    | Signed integer        | 8 bytes      |
| `Uint`   | Unsigned integer      | 8 bytes      |
| `Float`  | Floating-point number | 8 bytes      |
| `String` | UTF-8 string          | Variable     |
| `Char`   | Unicode character     | 4 bytes      |
| `Bytes`  | Raw bytes             | Variable     |

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

## Appendix B: Design Inspiration

YaoXiang's design draws excellent ideas from the following languages and projects:

| Source                       | Inspiration                                                                         |
| ---------------------------- | ----------------------------------------------------------------------------------- |
| **Rust**                     | Ownership model, zero-cost abstractions, type system                                |
| **Python**                   | Syntax style, readability, list comprehensions                                      |
| **Idris/Agda**               | Dependent types, type-driven development                                            |
| **Curry-Howard Isomorphism** | Types as propositions, programs as proofs, unified theory of type systems and logic |
| **TypeScript**               | Type annotations, runtime types                                                     |
| **MoonBit**                  | AI-friendly design, concise syntax                                                  |
| **Haskell**                  | Pure functional, pattern matching                                                   |
| **OCaml**                    | Type inference, variant types                                                       |

---

## Appendix C: FAQ

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions but adopts simpler syntax and a
lower cognitive burden. The **spawn model** is more concise than Rust's `async/await` — only one
`spawn` marker is needed, no need to manually manage Future and Pin. "The myriad things arise
together; thereby we observe their cycles", making concurrent programming as intuitive as describing
natural laws. The **ownership model** (RFC-009 v9) replaces lifetime annotations with Move + &T/&mut
T tokens, and replaces the borrow checker with type attributes (Dup/Linear). The unified type syntax
eliminates the concept fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What types of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted
programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4-space indentation provides clear visual separation of code blocks, reducing the confusion
caused by nesting depth. This is a well-considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. The release time depends on implementation progress; the current
version number is subject to the `Cargo.toml` in the repository root.

**Q: How do I contact the core team?**

A: Through GitHub Discussions or the Discord community channel. Core team members reply regularly.

---

> **Last Updated**: 2026-10-02
>
> **Document Version**: v2.0.0
>
> **License**: MIT

---

> "The changes of Yao and Xiang give birth to the myriad things. The evolution of types completes
> the program."
>
> May YaoXiang's design journey walk with you.
