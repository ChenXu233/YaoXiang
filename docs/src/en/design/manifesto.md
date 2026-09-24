# YaoXiang Design Manifesto

> **Version**: v2.0.0 **Status**: Official Release **Authors**: Chenxu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Dao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to the myriad things." — _Tao Te Ching_
>
> Types are like the Dao, from which all things are born.

---

## 1. Why Create YaoXiang?

### 1.1 Filling a Language Gap

Throughout the long history of programming languages, we have witnessed the birth and evolution of
countless excellent languages: C brought the efficiency revolution in system programming, Python
created a programming experience accessible to everyone, Rust proved that memory safety and
performance can coexist, and TypeScript made large frontend projects maintainable. However, when we
examine today's language ecosystem, we still find an obvious gap—**no single language can
simultaneously satisfy the following three core needs**:

| Need               | Problems with Existing Solutions                                                                                                                         |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Type Safety**    | Rust is overly strict with a steep learning curve; TypeScript uses optional types and cannot provide compile-time guarantees                             |
| **Natural Syntax** | Rust's syntax is complex and obscure; Haskell's functional threshold is too high; traditional static languages are verbose and cumbersome                |
| **AI Friendly**    | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behaviors, limiting the accuracy of AI code generation and modification |

The birth of YaoXiang is precisely to fill this gap. We believe: **a programming language should be
both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Real Problems Being Solved

**Problem One: Fragmentation of Type Systems**

Today's programming languages show severe fragmentation in their type systems. Static type languages
pursue absolute correctness at compile time but often at the expense of development efficiency;
dynamic type languages offer flexibility but expose difficult-to-maintain defects in large projects.
YaoXiang proposes a unified abstract framework of "everything is a type," making types the main
thread throughout language design rather than a post-hoc patch.

**Problem Two: The Dilemma Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime
performance. GC (garbage collection) liberates developers but introduces latency fluctuations and
memory overhead; manual memory management is efficient but as dangerous as walking a tightrope.
YaoXiang adopts a Rust-style ownership model, eliminating data races and memory leaks at compile
time while maintaining zero-cost abstractions and achieving high performance without a GC.

**Problem Three: The Cognitive Burden of Asynchronous Programming**

Modern applications cannot do without networking and concurrency, yet asynchronous programming has
long been a programmer's nightmare. Nested callback functions, Promise chains, async/await
syntax—each solution adds to the complexity of code. YaoXiang redesigns the asynchronous model:
simply add a `spawn` marker after a function signature, and the compiler automatically handles all
asynchronous details, making concurrent programming as natural as synchronous code.

**Problem Four: The Bottleneck of AI-Assisted Programming**

When AI began to assist developers in writing code, the choice of language design became crucial.
Ambiguous syntax rules, implicit type conversions, and complex syntactic sugar—these characteristics
that human programmers have grown accustomed to have become obstacles for AI to understand and
generate. From the very beginning, YaoXiang has made "AI friendly" a core design goal: strict
indentation rules, clear code block boundaries, and unambiguous syntactic structures allow AI to
accurately understand, generate, and modify code.

### 1.3 The Philosophical Foundation of the Language

The name YaoXiang comes from "Yao" (爻) and "Xiang" (象) in the _Book of Changes_. "Yao" represents
the fundamental symbols composing hexagrams, symbolizing the interplay of yin and yang, motion and
stillness; "Xiang" represents the external manifestations of the essence of things, embodying the
myriad phenomena of the universe.

This philosophical thought is reflected in every detail of the language's design:

- **Unity**: Just as the simple symbols of yao lines compose complex hexagrams, YaoXiang uses a few
  core concepts (types, functions, constructors) to build a complete programming model
- **Hierarchy**: Just as xiang has distinctions of pre-heaven and post-heaven, YaoXiang's type
  system has a clear hierarchical structure, from primitive types to generics, from values to meta
  types
- **Variability**: Just as yin and yang flow and transform endlessly, YaoXiang supports dependent
  types, allowing types to evolve as values change
- **Recognizability**: Just as hexagrams can be interpreted and all things can be symbolized,
  YaoXiang provides complete type reflection capabilities, with full runtime type information
  available
- **Provability**: Just as hexagrams reveal the patterns of things, YaoXiang's type system follows
  the Curry-Howard isomorphism (types as propositions, programs as proofs); type checking is the
  verification of logical proofs

---

## 2. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any
feature proposal must be examined against these principles.

### 2.1 Principle One: Everything Is a Type

In YaoXiang's worldview, types are the highest-level abstraction units and the core concept running
through the language.

**Specific Manifestations**:

- **Values are instances of types**: `42` is an instance of type `Int`, `"hello"` is an instance of
  type `String`
- **Types themselves are types**: `Type` is the only meta type keyword in the language; the type of
  `Int` is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from
  `Int × Int` to `Int`
- **Modules are type compositions**: Modules are namespace compositions containing functions and
  types

**Reason for Non-Negotiability**: Unified type abstraction simplifies language semantics, eliminates
the dualism between values and types, and allows the type system to become a guardian of code
correctness rather than a stumbling block.

### 2.2 Principle Two: Strict Structuring

YaoXiang's syntax design pursues "unambiguous, predictable, and easy to parse."

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are forbidden; code block boundaries are
  immediately visible
- **Brackets cannot be omitted**: Function parameters must have parentheses, list elements must have
  commas
- **Code blocks must have curly braces**: `if`, `while`, `for` and other control flows must be
  wrapped in `{ }`
- **Streamlined keyword count**: Only 17 core keywords are retained, rejecting the proliferation of
  syntactic sugar

**Reason for Non-Negotiability**: Strict structuring brings three key advantages—(1) IDE syntax
highlighting and code folding become more accurate; (2) the accuracy of AI code generation and
modification is greatly improved; (3) new learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: Generic functions are expanded at compile time into specific versions, with
  no virtual table lookup overhead
- **Inline optimization**: Simple functions are automatically inlined, eliminating function call
  overhead
- **Stack allocation priority**: Small objects are allocated on the stack by default; heap
  allocation is used only when necessary
- **No GC**: The ownership model guarantees memory safety without the runtime overhead of a garbage
  collector

**Reason for Non-Negotiability**: Performance is the lifeline of a programming language. Any design
that trades performance for convenience is a betrayal of the programmer.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity go hand in hand. YaoXiang chooses immutability by default, making code
easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default and cannot be modified after assignment
- Mutability must be explicitly declared with `mut` when needed
- References are immutable by default; mutable references require the `mut` marker
- The transfer of ownership means the original binding becomes invalid

**Reason for Non-Negotiability**: Immutability is the foundation of concurrent safety, the guarantee
of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types Are Data

Type information should not only exist at compile time but should be fully available at runtime.

**Specific Capabilities**:

- Runtime type queries: any value can obtain its type information
- Type reflection: types themselves can be constructed and manipulated
- Pattern matching destructuring: type constructors can be used directly in pattern matching
- Generic specialization: runtime can obtain the concrete types of generic parameters

**Reason for Non-Negotiability**: Complete type reflection capabilities are the foundation of
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

**YaoXiang's unified syntax**: everything is `name: type = value`, and `Type` is the only meta type
keyword.

```yaoxiang
# === Record Types ===

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

# === Generic Types ===

Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

# === Interfaces (records whose fields are all function types) ===

Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

Serializable: Type = {
    serialize: () -> String,
}

# === Interface implementation (interface names are written within the type body) ===

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

### 3.2 Innovation Two: Constructors Are Types

**Value construction is completely identical to function calls**:

```yaoxiang
# Type definitions
Point: Type = { x: Float, y: Float }
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

# Value construction: same as function calls
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

YaoXiang adopts a purely functional design, using currying to implement syntax sugar similar to
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

# Method syntax sugar binding ([0] indicates binding to the 0th parameter position)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# The two calling methods are completely equivalent
d1 = distance(p1, p2)     # Direct call to the core function
d2 = p1.distance(p2)      # Method syntax sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, waiting for the second parameter
d3 = dist_from_p1(p2)       # 2.828
```

**Innovation Value**: Purely functional design, no hidden `self` parameter, functions are values
that can be freely passed and composed.

### 3.4 Innovation Four: The Spawn Model

> "The myriad things arise together; I observe their return." — _Yi·Hexagram of Return (Fu)_
>
> The spawn model takes its meaning from this, describing a programming paradigm: developers
> describe logic with synchronous, sequential thinking, while the language runtime automatically and
> efficiently executes the computational units therein concurrently like the myriad things arising,
> and ultimately unifies and coordinates them.

**Three Core Principles**:

| Principle                | Description                                                                  |
| ------------------------ | ---------------------------------------------------------------------------- |
| **Synchronous Syntax**   | Sequential code where what you see is what you get                           |
| **Concurrent Essence**   | The runtime automatically extracts parallelism                               |
| **Unified Coordination** | Results automatically converge when needed, guaranteeing logical correctness |

**Terminology**:

| Official Term       | Corresponding Syntax         | Explanation                                                                                              |
| ------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------- |
| **spawn function**  | `spawn (params) => body`     | Defines a computational unit that can participate in spawn execution                                     |
| **spawn block**     | `spawn { a(), b() }`         | An explicitly declared concurrent region, with tasks inside the block executing concurrently             |
| **spawn loop**      | `spawn for x in xs { ... }`  | Data parallelism, with the loop body executing concurrently over all elements                            |
| **spawn value**     | `Async(T)`                   | A future value currently being spawned, automatically awaited when used                                  |
| **spawn graph**     | Lazy computation graph (DAG) | The stage where spawning occurs, describing dependencies and parallelism                                 |
| **spawn scheduler** | Runtime task scheduler       | The intelligent coordinator that harmonizes the myriad things, causing them to spawn at the right moment |

> **See details**: [RFC-001 Spawn Model](./rfc/deprecated/001-concurrent-model-error-handling.md)

```yaoxiang
# === spawn Function ===
# A function marked with spawn
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === spawn Block ===
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
# The ref keyword automatically handles thread safety (compiler automatically chooses Rc/Arc)
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

- See [RFC-001 Spawn Model](./rfc/deprecated/001-concurrent-model-error-handling.md)

**Innovation Value**: The cognitive burden of asynchronous programming is reduced to zero, code
readability is identical to synchronous code, and high-performance parallel execution efficiency is
obtained.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, enabling true type-driven development.

```yaoxiang
# Matrix type: dimensions determined at compile time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time computation: factorial(3) = 6
arr: Array(Int, factorial(3)) = Array(Int, 6)()

# Compile-time dimension verification
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimensions mismatch
```

**Innovation Value**: Catches more errors at compile time, enabling more precise type guarantees.

### 3.6 Innovation Six: Minimalist Keyword Design

YaoXiang defines only 17 core keywords, far fewer than mainstream languages:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

| Compared Language | Number of Keywords |
| ----------------- | ------------------ |
| YaoXiang          | **17**             |
| Rust              | 51+                |
| Python            | 35                 |
| TypeScript        | 64+                |
| Go                | 25                 |

**Innovation Value**: Lower memory burden, more consistent syntactic style, and easier-to-parse
grammatical structure.

---

## 4. Initial Syntax Preview

The following code examples showcase the style of the YaoXiang language and help you quickly
experience its design aesthetics.

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

# &T / &mut T tokens (zero compile-time cost)
p2.print()           # Compiler automatically creates a &Point token
p2.shift(1.0, 1.0)  # Compiler automatically creates a &mut Point token

# ref: shared ownership (compiler automatically chooses Rc/Arc)
shared = ref p2      # Share across scopes

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
Borrow token  Default  Shared own.  Deep copy  Raw pointer
Zero cost    Zero-copy Auto Rc/Arc  Explicit  System-level
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

### 4.6 Concurrent Programming (Spawn Model)

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

## 5. Roadmap and Pending Items

### 5.1 Decided Design Decisions

The following decisions have been thoroughly discussed and reviewed, and **are no longer subject to
change**:

| Module                 | Decision                         | Description                                                                 |
| ---------------------- | -------------------------------- | --------------------------------------------------------------------------- |
| **Type System**        | Everything is a type             | Values, functions, modules, and generics are all types                      |
| **Type Syntax**        | Unified `name: type = value`     | One declaration form covers all cases; `Type` is the only meta type keyword |
| **Keywords**           | 17 core keywords                 | Does not include `type`/`fn`/`struct`/`enum`/`trait`/`impl`                 |
| **Function Syntax**    | Signature + expression           | `name: (params) -> ReturnType = body`                                       |
| **Method Binding**     | RFC-004 Curried binding          | `Type.method = function[position]`                                          |
| **Asynchronous Model** | Spawn model                      | `spawn` marker, lazy evaluation, automatic parallelism                      |
| **Memory Management**  | Ownership model (RFC-009 v9)     | Move + &T/&mut T tokens + ref + clone + unsafe, no GC                       |
| **File as Module**     | Module system                    | Each `.yx` file is a module                                                 |
| **Main Function**      | `main: () -> Void`               | Program entry point                                                         |
| **Thread Safety**      | ref automatically selects Rc/Arc | Compiler escape analysis, transparent to users                              |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang Implementation Roadmap (Example)            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: Rust AOT│
│        ✅ Completed                    │ (Current Phase)         Compiler    │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←─────── v1.0: YaoXiang JIT Compiler ←──── v2.0:│
│        (Self-hosting)                  (Self-hosting)              YaoXiang AOT│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 6. How to Participate in Contributing

YaoXiang is a language born of the community, growing in the community, and serving the community.
We sincerely invite every developer who loves programming language design to join this exploration
journey.

### 6.1 Design Discussions

**Suitable for**: Programming language theory researchers, type system enthusiasts, language design
fanatics

**How to Participate**:

- **GitHub Discussions**: Participate in discussions under the "Language Design" category
- **Design Proposals (RFC)**: Propose design documents for new features, following the template in
  the `rfcs/` directory
- **Syntax Review**: Suggest improvements to existing syntax designs or identify potential issues

| **Current Hot Topics**: | | | | - Macro system design and implementation | | - Interface type
mechanism | | - Error handling syntax optimization | | - Standard library API design |

**Submitting a Design Proposal**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros/cons analysis, alternatives)
3. Initiate a Pull Request for community review
4. Merge or reject after deliberation by the core team

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, system programmers, performance optimization experts

**Current Implementation Priorities** (sorted by priority):

| Priority | Module                       | Description                                         | Difficulty |
| -------- | ---------------------------- | --------------------------------------------------- | ---------- |
| P0       | **Bytecode Virtual Machine** | VM instruction completion, performance optimization | Medium     |
| P0       | **Runtime Memory**           | GC implementation, memory allocator                 | High       |
| P0       | **Asynchronous Runtime**     | Complete implementation of the spawn model          | High       |
| P1       | Standard Library             | IO, String, List, Concurrent                        | Medium     |
| P1       | JIT Compiler                 | Cranelift integration                               | High       |
| P2       | AOT Compiler                 | LLVM/Cranelift backend                              | High       |
| P3       | Self-Hosting Compiler        | Rewrite in YaoXiang                                 | Very High  |

**Technology Stack**:

- **Implementation Language**: Rust (current stage)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Test Framework**: Rust `#[test]` + `cargo nextest`

**Start Contributing**:

1. Check `docs/YaoXiang-implementation-plan.md` to understand the architecture design
2. Choose an interesting module under the `src/` directory
3. Check `tests/unit/` to understand testing requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, efficiency tool seekers

**Tools to Develop**:

| Tool                     | Status           | Description                                |
| ------------------------ | ---------------- | ------------------------------------------ |
| **LSP Server**           | ⏳ To be started | Language Server Protocol support           |
| **Debugger Integration** | ⏳ To be started | GDB/LLDB integration                       |
| **Formatter**            | ⏳ To be started | `yx format`                                |
| **Package Manager**      | ⏳ To be started | Dependency management, version resolution  |
| **Package Registry**     | ⏳ To be started | Centralized or decentralized               |
| **REPL**                 | ⏳ To be started | Interactive interpreter                    |
| **Benchmarking Tool**    | ⏳ To be started | Performance analysis                       |
| **VS Code Plugin**       | ⏳ To be started | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin**    | ⏳ To be started | Syntax highlighting, LSP client            |

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

**Standard Library Module Planning**:

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
| `std.database`   | P3       | Database connections             |
| `std.gui`        | P3       | Graphical interface (long-term)  |

**Design Principles**:

- Consistency: Function names and behaviors with the same functionality should remain consistent
- Simplicity: APIs should be intuitive and easy to use, avoiding over-engineering
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Each function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documentation to Contribute**:

| Documentation          | Status           | Description                               |
| ---------------------- | ---------------- | ----------------------------------------- |
| Quick Start            | ✅ Completed     | 5-minute getting started guide            |
| Language Guide         | ✅ Completed     | Systematic study of core concepts         |
| Language Specification | ✅ Completed     | Complete syntax and semantic definition   |
| Implementation Plan    | ✅ Completed     | Compiler implementation technical details |
| API Documentation      | ⏳ To be started | Standard library API reference            |
| Tutorials              | ⏳ To be started | Advanced tutorials and best practices     |
| Blog                   | ⏳ To be started | Technical articles and design stories     |
| Translation            | ⏳ To be started | Multi-language support                    |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

**Community Activities**:

- Regular online meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution sprints (quarterly)
- Offline gatherings and conference talks

**Communication Channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Bug reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guide

**How to Start Contributing**:

1. **Understand the Project**: Read the README and design documents
2. **Choose a Direction**: Select a contribution area based on your interests
3. **Set Up the Environment**: Rust 1.75+, cargo, git
4. **Find a Task**: Check GitHub Issues for the `good first issue` label
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

- Follow the `rustfmt.toml` specification
- Ensure `cargo clippy` produces no warnings
- Write necessary unit tests
- Update relevant documentation

---

## Appendix A: Language Cheat Sheet

### A.1 Keywords

| Keyword                 | Purpose                                                  |
| ----------------------- | -------------------------------------------------------- |
| `pub`                   | Public export                                            |
| `use`                   | Import module                                            |
| `spawn`                 | Spawn marker                                             |
| `ref`                   | Shared ownership (compiler automatically selects Rc/Arc) |
| `mut`                   | Mutable variable                                         |
| `if/else if/else`       | Conditional branches                                     |
| `match`                 | Pattern matching                                         |
| `while/for`             | Loops                                                    |
| `return/break/continue` | Control flow                                             |
| `as`                    | Type conversion                                          |
| `in`                    | Membership test/list comprehension                       |
| `unsafe`                | unsafe code block (raw pointers)                         |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type`
> keyword has been removed in RFC-010, unifying with the `name: Type = value` syntax.

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

## Appendix B: Design Inspirations

YaoXiang's design draws on the excellent ideas from the following languages and projects:

| Source                       | Inspiration                                                                             |
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

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions, but adopts a simpler syntax and
lower cognitive burden. The **spawn model** is more concise than Rust's `async/await`—just one
`spawn` marker, with no need to manually manage Future and Pin. "The myriad things arise together; I
observe their return," making concurrent programming as intuitive as describing natural laws. The
**ownership model** (RFC-009 v9) uses Move + &T/&mut T tokens to replace lifetime annotations, and
type attributes (Dup/Linear) to replace the borrow checker. The unified type syntax eliminates the
conceptual fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What types of development is YaoXiang suitable for?**

A: System programming, application development, web services, scripting tools, AI-assisted
programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing the confusion brought by
nesting depth. This is a well-considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. The release date depends on implementation progress; see the
[Version Planning RFC](./rfc/rejected/003-version-planning.md).

**Q: How do I contact the core team?**

A: Through GitHub Discussions or the Discord community channel. Core team members respond regularly.

---

> **Last Updated**: 2026-05-31
>
> **Document Version**: v2.0.0
>
> **License**: [MIT](LICENSE)

---

> "The Yaos and Xiangs transform, and the myriad things are born. Types evolve, and programs are
> made."
>
> May the design journey of YaoXiang be accompanied by you.
