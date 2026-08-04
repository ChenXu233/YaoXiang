# YaoXiang Design Manifesto

> **Version**: v2.0.0 **Status**: Officially Released **Author**: Chenxu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Dao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to the myriad things." — _Dao De Jing_
>
> Types are like the Dao; all things are born from them.

---

## 1. Why Create YaoXiang?

### 1.1 Filling the Language Gap

Throughout the long history of programming languages, we have witnessed the birth and evolution of
countless excellent languages: C brought the efficiency revolution to system programming, Python
created a learning experience accessible to everyone, Rust proved that memory safety and performance
can coexist, and TypeScript made large front-end projects maintainable. However, when we examine
today's language ecosystem, we still find an obvious gap—**no language can simultaneously satisfy
the following three core needs**:

| Need               | Problems with Existing Solutions                                                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Type Safety**    | Rust is too strict with a steep learning curve; TypeScript has optional types, offering no compile-time guarantee                                  |
| **Natural Syntax** | Rust syntax is complex and obscure; Haskell's functional paradigm has too high a barrier; traditional static languages are verbose and cumbersome  |
| **AI-Friendly**    | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behavior, limiting AI's accuracy in generating and modifying code |

YaoXiang was born precisely to fill this gap. We believe: **a programming language should be both
powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Real Problems Solved

**Problem One: Fragmentation of Type Systems**

Today's programming languages show severe fragmentation in their type systems. Static-typed
languages pursue absolute correctness at compile time, but often at the cost of development
efficiency; dynamically-typed languages offer flexibility, yet expose maintenance difficulties in
large projects. YaoXiang proposes a unified abstraction framework of "Everything is a Type," making
types the main thread running through language design rather than a patch added afterward.

**Problem Two: The Dilemma Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime
performance. While GC (garbage collection) liberates developers, it brings latency fluctuations and
memory overhead; while manual memory management is efficient, it is as dangerous as walking a
tightrope. YaoXiang adopts Rust-style ownership model, eliminating data races and memory leaks at
compile time, while maintaining zero-cost abstractions, achieving high performance without GC.

**Problem Three: The Cognitive Burden of Asynchronous Programming**

Modern applications cannot do without networking and concurrency, and asynchronous programming has
always been a programmer's nightmare. Nested callbacks, Promise chains, async/await syntax—each
solution adds complexity to the code. YaoXiang has redesigned the asynchronous model: just add a
`spawn` marker after the function signature, and the compiler automatically handles all asynchronous
details, making concurrent programming as natural as synchronous code.

**Problem Four: The Bottleneck of AI-Assisted Programming**

When AI begins to assist developers in writing code, language design choices become crucial.
Ambiguous grammar rules, implicit type conversions, complex syntactic sugar—these features that
human programmers have grown accustomed to become obstacles for AI understanding and generation.
From the very beginning of its design, YaoXiang has taken "AI-friendliness" as a core goal: strict
indentation rules, clear code block boundaries, unambiguous syntax structures, allowing AI to
accurately understand, generate, and modify code.

### 1.3 The Philosophical Foundations of the Language

The name YaoXiang comes from "爻" (Yao) and "象" (Xiang) in the _I Ching_. "Yao" is the fundamental
symbol composing hexagrams, symbolizing the changes of yin and yang, the interplay of motion and
stillness; "Xiang" is the external manifestation of the essence of things, representing all
phenomena and encompassing everything.

This philosophical thought is reflected in every detail of the language's design:

- **Unity**: Just as simple symbols of Yao form complex hexagrams, YaoXiang builds a complete
  programming model with a few core concepts (types, functions, constructors)
- **Hierarchy**: Just as there are distinctions between innate and acquired Xiang, YaoXiang's type
  system has a clear hierarchical structure, from primitive types to generics, from values to meta
  types
- **Variability**: Just as yin and yang flow with endless change, YaoXiang supports dependent types,
  allowing types to evolve with values
- **Recognizability**: Just as hexagrams can be interpreted and all things can be represented,
  YaoXiang provides complete type reflection capabilities, with runtime type information fully
  available
- **Provability**: Just as hexagrams reveal the patterns of things, YaoXiang's type system follows
  the Curry-Howard isomorphism (types as propositions, programs as proofs), and the process of type
  checking is the verification of logical proofs

---

## 2. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any
feature proposal must be tested against these principles.

### 2.1 Principle One: Everything is a Type

In YaoXiang's worldview, types are the highest-level abstraction units, the core concept running
through the language.

**Specific Manifestations**:

- **Values are instances of types**: `42` is an instance of `Int`, `"hello"` is an instance of
  `String`
- **Types themselves are types**: `Type` is the language's only meta type keyword; the type of `Int`
  is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from
  `Int × Int` to `Int`
- **Modules are type compositions**: modules are combinations of namespaces containing functions and
  types

**Why It's Non-Negotiable**: Unified type abstraction can simplify language semantics, eliminate the
dualism between values and types, and let the type system become the guardian of code correctness
rather than an obstacle.

### 2.2 Principle Two: Strict Structuring

YaoXiang's syntax design pursues "unambiguous, predictable, easy to parse."

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are prohibited; code block boundaries are clear
  at a glance
- **Brackets cannot be omitted**: function parameters must have parentheses, list elements must have
  commas
- **Code blocks must have braces**: control flow like `if`, `while`, `for` must be wrapped in `{ }`
- **Minimal keywords**: only 17 core keywords are retained, rejecting the proliferation of syntactic
  sugar

**Why It's Non-Negotiable**: Strict structuring brings three key advantages—(1) IDE syntax
highlighting and code folding become more accurate; (2) AI code generation and modification accuracy
greatly improves; (3) new learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: generic functions are expanded into specific versions at compile time, with
  no virtual table lookup overhead
- **Inline optimization**: simple functions are automatically inlined, eliminating function call
  overhead
- **Stack allocation priority**: small objects are stack-allocated by default, heap allocation only
  when necessary
- **No GC**: ownership model ensures memory safety, with no runtime overhead from garbage collectors

**Why It's Non-Negotiable**: Performance is the lifeline of a programming language. Any design that
trades performance for convenience is a betrayal of the programmer.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity are inseparable companions. YaoXiang chooses immutability by default,
making code easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default and cannot be modified after assignment
- Mutability must be explicitly declared with `mut` when needed
- References are immutable by default, mutable references require the `mut` marker
- Ownership transfer means the original binding is invalidated

**Why It's Non-Negotiable**: Immutability is the foundation of concurrency safety, the guarantee of
code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types are Data

Type information should not only exist at compile time, but be fully available at runtime.

**Specific Capabilities**:

- Runtime type queries: any value can obtain its type information
- Type reflection: types themselves can be constructed and manipulated
- Pattern matching destructuring: type constructors can be directly used for pattern matching
- Generic specialization: specialized types of generic parameters can be obtained at runtime

**Why It's Non-Negotiable**: Complete type reflection capabilities are the foundation of
metaprogramming, the cornerstone of high-performance frameworks and tools.

---

## 3. Key Innovations and Features

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

**YaoXiang's unified syntax**: everything is `name: type = value`, with `Type` as the only meta type
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

# === Interface Implementation (interface names written inside the type body) ===

Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable,
}

# === Methods (Type.method syntax) ===

Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

**Innovation Value**: No fragmentation of `fn`, `struct`, `enum`, `trait`, `impl` keywords—one
unified syntax covers all declarations.

### 3.2 Innovation Two: Constructors are Types

**Value construction is identical to function calls**:

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

YaoXiang adopts a pure functional design, using currying to implement syntactic sugar similar to
object method calls, without introducing `class` and `method` keywords.

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

# Method syntactic sugar binding ([0] indicates binding to parameter position 0)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Both call methods are completely equivalent
d1 = distance(p1, p2)     # Direct call to core function
d2 = p1.distance(p2)      # Method syntactic sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, waiting for the second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovation Value**: Pure functional design, no hidden `self` parameter, functions are values that
can be freely passed and composed.

### 3.4 Innovation Four: The Spawn Model

> "All things flourish together, I observe their return." — _I Ching · Return Hexagram_
>
> The spawn model draws its meaning from this, describing a programming paradigm: developers
> describe logic with synchronous, sequential thinking, while the language runtime makes the
> computational units within automatically and efficiently execute concurrently like all things
> flourishing together, and finally unify and collaborate.

**Three Core Principles**:

| Principle                 | Description                                                                   |
| ------------------------- | ----------------------------------------------------------------------------- |
| **Synchronous Syntax**    | Sequential code, WYSIWYG                                                      |
| **Concurrent Essence**    | Runtime automatically extracts parallelism                                    |
| **Unified Collaboration** | Results automatically aggregate when needed, guaranteeing logical correctness |

**Terminology**:

| Official Term       | Corresponding Syntax         | Explanation                                                                            |
| ------------------- | ---------------------------- | -------------------------------------------------------------------------------------- |
| **Spawn Function**  | `spawn (params) => body`     | Defines computational units that can participate in spawn execution                    |
| **Spawn Block**     | `spawn { a(), b() }`         | Explicitly declared concurrent scope, tasks inside the block execute in spawn          |
| **Spawn Loop**      | `spawn for x in xs { ... }`  | Data parallelism, the loop body executes in spawn on all elements                      |
| **Spawn Value**     | `Async(T)`                   | A future value currently being spawned, automatically awaited when used                |
| **Spawn Graph**     | Lazy computation graph (DAG) | The stage where spawn happens, describing dependencies and parallel relationships      |
| **Spawn Scheduler** | Runtime task scheduler       | The intelligent hub that coordinates all things, making them spawn at the right moment |

> **See also**: [RFC-001 The Spawn Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === Spawn Function ===
# Function marked with spawn
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

# === Automatic Awaiting ===
main: () -> Void = {
    # Two independent requests automatically execute in parallel
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # Wait points are automatically inserted when results are needed
    print(users.length + posts.length)  # Automatically waits for users and posts
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

- See [RFC-001 The Spawn Model](./rfc/accepted/001-concurrent-model-error-handling.md) for details

**Innovation Value**: The cognitive burden of asynchronous programming is reduced to zero; code
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
vec: Vec(factorial(3)) = Vec(6)()

# Compile-time dimension validation
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimension mismatch
```

**Innovation Value**: Captures more errors at compile time, achieving more precise type guarantees.

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

**Innovation Value**: Lower memory burden, more consistent syntax style, easier to parse syntax
structure.

---

## 4. Preliminary Syntax Preview

The following code examples show the style of YaoXiang, helping you quickly experience its design
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

# Destructuring patterns
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
p2.print()           # Compiler automatically creates an &Point token
p2.shift(1.0, 1.0)  # Compiler automatically creates an &mut Point token

# ref: shared holding (compiler automatically chooses Rc/Arc)
shared = ref p2      # Cross-scope sharing

# clone(): explicit deep copy
backup = p2.clone()

# unsafe + raw pointers: system-level
unsafe {
    ptr: *Point = &p2
    (*ptr).x = 0.0
}
```

**Ownership Gradient**:

```
&T / &mut T    Move       ref        clone()    unsafe
    |             |          |           |          |
Borrow token   Default   Shared hold  Deep copy  Raw pointer
Zero cost     Zero-copy  Auto Rc/Arc  Explicit   System-level
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

# Using match for handling
result = divide(10.0, 2.0)
match result {
    Result.ok(value) -> print(value),
    Result.err(msg) -> print("Error: ${msg}"),
}
```

### 4.6 Concurrent Programming (The Spawn Model)

```yaoxiang
# spawn marks an asynchronous function
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

The following decisions have been fully discussed and reviewed, **no further changes will be
accepted**:

| Module                 | Decision                         | Description                                                                 |
| ---------------------- | -------------------------------- | --------------------------------------------------------------------------- |
| **Type System**        | Everything is a type             | Values, functions, modules, generics are all types                          |
| **Type Syntax**        | Unified `name: type = value`     | One declaration form covers all cases, `Type` is the only meta type keyword |
| **Keywords**           | 17 core keywords                 | Excludes `type`/`fn`/`struct`/`enum`/`trait`/`impl`                         |
| **Function Syntax**    | Signature + expression           | `name: (params) -> ReturnType = body`                                       |
| **Method Binding**     | RFC-004 Curried binding          | `Type.method = function[position]`                                          |
| **Asynchronous Model** | The spawn model                  | `spawn` marker, lazy evaluation, automatic parallelism                      |
| **Memory Management**  | Ownership model (RFC-009 v9)     | Move + &T/&mut T tokens + ref + clone + unsafe, no GC                       |
| **File is Module**     | Module system                    | Each `.yx` file is a module                                                 |
| **Main Function**      | `main: () -> Void`               | Program entry point                                                         |
| **Thread Safety**      | ref automatically chooses Rc/Arc | Compiler escape analysis, transparent to users                              |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang Implementation Roadmap (Example)        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: Rust AOT│
│        ✅ Completed                   │ (Current Stage)              Compiler  │
│                                       │                                      │
│                                       ▼                                      │
│  v0.6: YaoXiang Interpreter ←─────── v1.0: YaoXiang JIT Compiler ←──── v2.0:│
│        (Self-hosting)                  (Self-hosting)              YaoXiang AOT│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 6. How to Contribute

YaoXiang is a language born in the community, grown in the community, and serving the community. We
sincerely invite every developer who loves programming language design to join this exploration
journey.

### 6.1 Design Discussion

**Suitable for**: Programming language theory researchers, type system enthusiasts, language design
fanatics

**How to Participate**:

- **GitHub Discussions**: Participate in discussions in the "Language Design" category
- **Design Proposals (RFC)**: Submit design documents for new features, following the templates in
  the `rfcs/` directory
- **Syntax Review**: Suggest improvements to existing syntax designs or identify potential issues

| **Current Hot Topics**: | | | | - Macro system design and implementation | | - Interface type
mechanism | | - Error handling syntax optimization | | - Standard library API design |

**Submitting a Design Proposal**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros and cons analysis, alternatives)
3. Initiate a Pull Request for community review
4. After core team review, merge or reject

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, systems programmers, performance optimization experts

**Current Implementation Focus** (in priority order):

| Priority | Module                | Description                                         | Difficulty |
| -------- | --------------------- | --------------------------------------------------- | ---------- |
| P0       | **Bytecode VM**       | VM instruction refinement, performance optimization | Medium     |
| P0       | **Runtime Memory**    | GC implementation, memory allocator                 | High       |
| P0       | **Async Runtime**     | Complete implementation of the spawn model          | High       |
| P1       | Standard Library      | IO, String, List, Concurrent                        | Medium     |
| P1       | JIT Compiler          | Cranelift integration                               | High       |
| P2       | AOT Compiler          | LLVM/Cranelift backend                              | High       |
| P3       | Self-hosting Compiler | Rewrite in YaoXiang                                 | Extreme    |

**Technology Stack**:

- **Implementation Language**: Rust (current stage)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Testing Framework**: Rust `#[test]` + `cargo nextest`

**Starting to Contribute**:

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
| **Formatter**            | ⏳ Not started | `yaoxiang fmt`                             |
| **Package Manager**      | ⏳ Not started | Dependency management, version resolution  |
| **Package Registry**     | ⏳ Not started | Centralized or decentralized               |
| **REPL**                 | ⏳ Not started | Interactive interpreter                    |
| **Benchmark Tool**       | ⏳ Not started | Performance profiling                      |
| **VS Code Extension**    | ⏳ Not started | Syntax highlighting, completion, debugging |
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

**Standard Library Module Planning**:

| Module           | Priority | Description                       |
| ---------------- | -------- | --------------------------------- |
| `std.io`         | P0       | File IO, console input/output     |
| `std.string`     | P0       | String operations, formatting     |
| `std.list`       | P0       | List/array operations             |
| `std.dict`       | P0       | Dictionary/hash table             |
| `std.math`       | P0       | Math functions, constants         |
| `std.time`       | P1       | Date and time operations          |
| `std.net`        | P1       | Network programming, HTTP         |
| `std.concurrent` | P1       | Concurrency primitives, channels  |
| `std.crypto`     | P2       | Cryptographic hashing, signatures |
| `std.json`       | P1       | JSON parsing/generation           |
| `std.regex`      | P2       | Regular expressions               |
| `std.database`   | P3       | Database connections              |
| `std.gui`        | P3       | GUI (long-term)                   |

**Design Principles**:

- Consistency: same-function functions share consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoiding over-design
- Performance: standard library functions should be efficient, avoiding unnecessary copies
- Testability: every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documentation Needed**:

| Document            | Status         | Description                               |
| ------------------- | -------------- | ----------------------------------------- |
| Quick Start         | ✅ Completed   | 5-minute getting started guide            |
| Language Guide      | ✅ Completed   | Systematic learning of core concepts      |
| Language Spec       | ✅ Completed   | Complete syntax and semantic definition   |
| Implementation Plan | ✅ Completed   | Compiler implementation technical details |
| API Documentation   | ⏳ Not started | Standard library API reference            |
| Tutorials           | ⏳ Not started | Advanced tutorials and best practices     |
| Blog                | ⏳ Not started | Technical articles and design stories     |
| Translation         | ⏳ Not started | Multi-language support                    |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

**Community Activities**:

- Regular online Meetup (monthly)
- Design and implementation discussion (weekly)
- Code contribution Sprint (quarterly)
- Offline gatherings and conference talks

**Communication Channels**:

- GitHub Discussions: technical discussions
- GitHub Issues: bug reports and feature requests
- Discord/Slack: real-time communication
- Twitter/X: project updates
- Blog: in-depth articles

### 6.7 Contribution Guide

**How to Start Contributing**:

1. **Understand the Project**: Read the README and design documents
2. **Choose a Direction**: Choose a contribution area based on your interests
3. **Set Up the Environment**: Rust 1.75+, cargo, git
4. **Find Tasks**: Check the `good first issue` label on GitHub Issues
5. **Submit PR**: Follow commit conventions, write tests
6. **Participate in Review**: Review others' code, participate in discussions

**Commit Conventions**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: new feature
fix: bug fix
docs: documentation update
style: code formatting (no functional impact)
refactor: refactoring
perf: performance optimization
test: testing
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
- Update relevant documentation

---

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword                 | Function                                               |
| ----------------------- | ------------------------------------------------------ |
| `pub`                   | Public export                                          |
| `use`                   | Import module                                          |
| `spawn`                 | Spawn marker                                           |
| `ref`                   | Shared holding (compiler automatically chooses Rc/Arc) |
| `mut`                   | Mutable variable                                       |
| `if/else if/else`       | Conditional branch                                     |
| `match`                 | Pattern matching                                       |
| `while/for`             | Loop                                                   |
| `return/break/continue` | Control flow                                           |
| `as`                    | Type conversion                                        |
| `in`                    | Membership check / list comprehension                  |
| `unsafe`                | unsafe code block (raw pointers)                       |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type`
> keyword has been removed in RFC-010, replaced by the unified `name: Type = value` syntax.

### A.3 Primitive Types

| Type     | Description       | Default Size |
| -------- | ----------------- | ------------ |
| `Void`   | Void value        | 0 bytes      |
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

## Appendix B: Design Inspiration

YaoXiang's design draws on the excellent ideas from the following languages and projects:

| Source                       | Inspiration Points                                                                      |
| ---------------------------- | --------------------------------------------------------------------------------------- |
| **Rust**                     | Ownership model, zero-cost abstractions, type system                                    |
| **Python**                   | Syntax style, readability, list comprehension                                           |
| **Idris/Agda**               | Dependent types, type-driven development                                                |
| **Curry-Howard Isomorphism** | Types as propositions, programs as proofs, the unified theory of type systems and logic |
| **TypeScript**               | Type annotations, runtime types                                                         |
| **MoonBit**                  | AI-friendly design, concise syntax                                                      |
| **Haskell**                  | Pure functional, pattern matching                                                       |
| **OCaml**                    | Type inference, variant types                                                           |

---

## Appendix C: FAQ

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions, but uses simpler syntax and a
lower cognitive burden. **The spawn model** is more concise than Rust's `async/await`—just one
`spawn` marker, no need to manually manage Future and Pin. "All things flourish together, I observe
their return," making concurrent programming as intuitive as describing the laws of nature. **The
ownership model** (RFC-009 v9) replaces lifetime annotations with Move + &T/&mut T tokens, and
replaces the borrow checker with type attributes (Dup/Linear). Unified type syntax eliminates the
conceptual fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What kind of development is YaoXiang suitable for?**

A: System programming, application development, web services, scripting tools, AI-assisted
programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing confusion caused by nesting
depth. This is a well-considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production ready. Release date depends on implementation progress, see
[Version Planning RFC](./rfc/003-version-planning.md) for details.

**Q: How to contact the core team?**

A: Via GitHub Discussions or Discord community channels. Core team members respond regularly.

---

> **Last Updated**: 2026-05-31
>
> **Document Version**: v2.0.0
>
> **License**: [MIT](LICENSE)

---

> "Yao and Xiang change, all things are born; types evolve, programs are formed."
>
> May YaoXiang's design journey travel with you.
