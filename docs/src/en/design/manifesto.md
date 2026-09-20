# YaoXiang Design Manifesto

> **Version**: v2.0.0 **Status**: Officially Released **Author**: Chenxu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Dao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to the myriad things." — Tao Te Ching
>
> Types are like the Dao, from which all things are born.

---

## I. Why Create YaoXiang?

### 1.1 Filling the Language Gap

In the long history of programming languages, we have witnessed the birth and evolution of countless
excellent languages: C brought the efficiency revolution in system programming, Python created a
programming experience accessible to everyone, Rust proved that memory safety and performance can
coexist, TypeScript made large frontend projects maintainable. However, when we examine today's
language ecosystem, we still find an obvious gap—**no language can simultaneously satisfy the
following three core requirements**:

| Requirement        | Problems with Existing Solutions                                                                                                                    |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Type Safety**    | Rust is too strict with a steep learning curve; TypeScript has optional types and cannot provide compile-time guarantees                            |
| **Natural Syntax** | Rust's syntax is complex and obscure; Haskell's functional paradigm has too high a barrier; traditional static languages are verbose and cumbersome |
| **AI-Friendly**    | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behaviors, limiting AI's accuracy in generating and modifying code |

YaoXiang was born precisely to fill this gap. We believe: **programming languages should be both
powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Real Problems Solved

**Problem One: Fragmentation of Type Systems**

Today's programming languages show severe fragmentation in their type systems. Static-typed
languages pursue absolute correctness at compile time, but often at the cost of development
efficiency; dynamically typed languages provide flexibility, but expose maintenance difficulties in
large projects. YaoXiang proposes a unified abstraction framework of "everything is a type," making
types the main thread throughout language design, rather than an afterthought patch.

**Problem Two: The Dilemma Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime
performance. GC (garbage collection), while liberating developers, brings latency fluctuations and
memory overhead; manual memory management, though efficient, is as dangerous as walking a tightrope.
YaoXiang adopts a Rust-style ownership model, eliminating data races and memory leaks at compile
time while maintaining zero-cost abstractions, achieving high performance without GC.

**Problem Three: Cognitive Burden of Asynchronous Programming**

Modern applications cannot do without networking and concurrency, and asynchronous programming has
always been a programmer's nightmare. Callback function nesting, Promise chain calls, async/await
syntax—each solution adds complexity to the code. YaoXiang redesigned the asynchronous model: just
add the `spawn` marker after the function signature, and the compiler automatically handles all
asynchronous details, making concurrent programming as natural as synchronous code.

**Problem Four: The Bottleneck of AI-Assisted Programming**

When AI begins to assist developers in writing code, the choice of language design becomes crucial.
Ambiguous syntax rules, implicit type conversions, complex syntactic sugar—these features that human
programmers have grown accustomed to become obstacles for AI to understand and generate. From the
very beginning of its design, YaoXiang has made "AI-friendly" a core goal: strict indentation rules,
explicit code block boundaries, unambiguous syntactic structures, allowing AI to accurately
understand, generate, and modify code.

### 1.3 The Philosophical Foundation of the Language

The name YaoXiang originates from the "Yao" and "Xiang" in the Book of Changes (I Ching). "Yao" is
the basic symbol that constitutes hexagrams, symbolizing the changes of yin and yang, the interplay
of motion and stillness; "Xiang" is the external manifestation of the essence of things,
representing all phenomena in the world.

This philosophical thought is reflected in every detail of the language design:

- **Unity**: Just as the simple symbols of trigrams form complex hexagrams, YaoXiang uses a few core
  concepts (types, functions, constructors) to build a complete programming model
- **Hierarchy**: Just as there are innate and acquired distinctions in phenomena, YaoXiang's type
  system has a clear hierarchical structure, from primitive type to generics, from values to meta
  type
- **Variability**: Just as yin and yang flow endlessly in transformation, YaoXiang supports
  dependent types, allowing types to evolve as values change
- **Recognizability**: Just as hexagrams can be interpreted and all things can be represented,
  YaoXiang provides complete type reflection capabilities, with full runtime type information
  available
- **Provability**: Just as hexagrams reveal the patterns of things, YaoXiang's type system follows
  the Curry-Howard isomorphism (types as propositions, programs as proofs); the process of type
  checking is the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any
feature proposal must be tested against these principles.

### 2.1 Principle One: Everything Is a Type

In YaoXiang's worldview, types are the highest-level abstraction units and the core concept that
runs through the language.

**Specific Manifestations**:

- **Values are instances of types**: `42` is an instance of the `Int` type, `"hello"` is an instance
  of the `String` type
- **Types themselves are types**: `Type` is the language's only meta type keyword; the type of `Int`
  is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from
  `Int × Int` to `Int`
- **Modules are type combinations**: modules are namespace combinations containing functions and
  types

**Why Non-Negotiable**: Unified type abstraction simplifies language semantics, eliminates the
binary opposition between values and types, and allows the type system to become the guardian of
code correctness rather than a stumbling block.

### 2.2 Principle Two: Strict Structuring

YaoXiang's syntax design pursues "unambiguous, predictable, easy to parse."

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are prohibited; code block boundaries are clear
  at a glance
- **Brackets cannot be omitted**: Function parameters must have parentheses, list elements must have
  commas
- **Code blocks must have curly braces**: `if`, `while`, `for` and other control flows must use
  `{ }` to wrap
- **Minimal number of keywords**: Only 17 core keywords are retained, rejecting syntactic sugar
  proliferation

**Why Non-Negotiable**: Strict structuring brings three key advantages—(1) more accurate IDE syntax
highlighting and code folding; (2) significantly improved accuracy of AI code generation and
modification; (3) new learners can quickly understand the code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: Generic functions are expanded into specific versions at compile time, with
  no vtable lookup overhead
- **Inline optimization**: Simple functions are automatically inlined, eliminating function call
  overhead
- **Stack allocation priority**: Small objects are allocated on the stack by default, heap
  allocation is only used when necessary
- **No GC**: The ownership model guarantees memory safety without the runtime overhead of a garbage
  collector

**Why Non-Negotiable**: Performance is the lifeline of programming languages. Any design that
exchanges performance for convenience is a betrayal of the programmer.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity go hand in hand. YaoXiang chooses immutability by default, making code
easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default and cannot be modified after assignment
- Mutability must be explicitly declared with `mut` when needed
- References are immutable by default; mutable references require the `mut` marker
- Transfer of ownership means the original binding is invalidated

**Why Non-Negotiable**: Immutability is the foundation of concurrency safety, the guarantee of code
readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types Are Data

Type information should not only exist at compile-time, but should be fully available at runtime.

**Specific Capabilities**:

- Runtime type query: any value can obtain its type information
- Type reflection: types themselves can be constructed and manipulated
- Pattern matching destructuring: type constructors can be used directly in pattern matching
- Generic specialization: runtime can obtain the realized types of generic parameters

**Why Non-Negotiable**: Complete type reflection capabilities are the foundation of metaprogramming
and the cornerstone of high-performance frameworks and tools.

---

## III. Key Innovations and Features

While absorbing the excellent features of existing languages, YaoXiang proposes the following
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

# === Interface Implementation (interface names written in the type body) ===

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

**Innovation Value**: No `fn`, `struct`, `enum`, `trait`, `impl` keyword fragments—one unified
syntax covers all declarations.

### 3.2 Innovation Two: Constructors Are Types

**Value construction is completely identical to function calls**:

```yaoxiang
# Type definition
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

YaoXiang adopts a pure functional design, achieving object-method-call-like syntactic sugar through
currying, without introducing `class` and `method` keywords.

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

# Method syntax sugar binding ([0] means bound to the 0th parameter position)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Two calling methods are completely equivalent
d1 = distance(p1, p2)     # Direct call to the core function
d2 = p1.distance(p2)      # Method syntactic sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, waiting for the second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovation Value**: Pure functional design, no hidden `self` parameter, functions are values that
can be freely passed and composed.

### 3.4 Innovation Four: Spawn Model

> "The myriad things arise together, I observe their returns." — I Ching, Return Hexagram
>
> The spawn model takes its meaning from this, describing a programming paradigm: developers
> describe logic with synchronous, sequential thinking, while the language runtime automatically and
> efficiently executes the computation units in parallel like myriad things arising together,
> finally coordinating and unifying them.

**Three Core Principles**:

| Principle                | Description                                                              |
| ------------------------ | ------------------------------------------------------------------------ |
| **Synchronous Syntax**   | What you see is what you get, sequential code                            |
| **Concurrent Essence**   | Runtime automatically extracts parallelism                               |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology System**:

| Official Term       | Corresponding Syntax         | Explanation                                                                                |
| ------------------- | ---------------------------- | ------------------------------------------------------------------------------------------ |
| **spawn function**  | `spawn (params) => body`     | Defines a computation unit that can participate in spawn execution                         |
| **spawn block**     | `spawn { a(), b() }`         | Explicitly declared concurrent territory, tasks within the block execute concurrently      |
| **spawn loop**      | `spawn for x in xs { ... }`  | Data parallelism, loop body executes concurrently on all elements                          |
| **spawn value**     | `Async(T)`                   | A future value being spawned, automatically awaited when used                              |
| **spawn graph**     | Lazy computation graph (DAG) | The stage where spawning happens, describing dependencies and parallelism                  |
| **spawn scheduler** | Runtime task scheduler       | The intelligent hub that coordinates myriad things, letting them spawn at the right moment |

> **See**: [RFC-001 Spawn Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === spawn Function ===
# spawn-marked function
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

- See [RFC-001 Spawn Model](./rfc/accepted/001-concurrent-model-error-handling.md)

**Innovation Value**: The cognitive burden of asynchronous programming is reduced to zero; code
readability is exactly the same as synchronous code, while achieving high-performance parallel
execution efficiency.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In Design, Partially Implemented

Types can depend on values, enabling truly type-driven development.

```yaoxiang
# Matrix type: dimensions determined at compile time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time calculation: factorial(3) = 6
arr: Array(Int, factorial(3)) = Array(Int, 6)()

# Compile-time dimension validation
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimension mismatch
```

**Innovation Value**: Captures more errors at compile-time, providing more precise type guarantees.

### 3.6 Innovation Six: Minimalist Keyword Design

YaoXiang defines only 17 core keywords, far fewer than mainstream languages:

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

**Innovation Value**: Lower memory burden, more consistent syntactic style, easier-to-parse
syntactic structure.

---

## IV. Preliminary Syntax Preview

The following code examples showcase the language style of YaoXiang, helping you quickly appreciate
its design aesthetics.

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

# &T / &mut T tokens (zero overhead at compile time)
p2.print()           # Compiler automatically creates &Point token
p2.shift(1.0, 1.0)  # Compiler automatically creates &mut Point token

# ref: shared ownership (compiler automatically chooses Rc/Arc)
shared = ref p2      # Sharing across scopes

# clone(): explicit deep copy
backup = p2.clone()

# unsafe + raw pointer: system level
unsafe {
    ptr: *Point = &p2
    (*ptr).x = 0.0
}
```

**Ownership Gradient**:

```
&T / &mut T    Move       ref        clone()    unsafe
    |             |          |           |          |
Borrow token    Default    Shared     Deep copy  Raw pointer
Zero cost       Zero copy  Auto Rc/Arc  Explicit  System level
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
# spawn marks async functions
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

The following decisions have been fully discussed and reviewed, **and are no longer subject to
change**:

| Module                | Decision                         | Description                                                                 |
| --------------------- | -------------------------------- | --------------------------------------------------------------------------- |
| **Type System**       | Everything is a type             | Values, functions, modules, generics are all types                          |
| **Type Syntax**       | Unified `name: type = value`     | One declaration form covers all cases; `Type` is the only meta type keyword |
| **Keywords**          | 17 core keywords                 | Does not include `type`/`fn`/`struct`/`enum`/`trait`/`impl`                 |
| **Function Syntax**   | Signature + expression           | `name: (params) -> ReturnType = body`                                       |
| **Method Binding**    | RFC-004 curried binding          | `Type.method = function[position]`                                          |
| **Async Model**       | Spawn model                      | `spawn` marker, lazy evaluation, automatic parallelism                      |
| **Memory Management** | Ownership model (RFC-009 v9)     | Move + &T/&mut T tokens + ref + clone + unsafe, no GC                       |
| **File as Module**    | Module system                    | Each `.yx` file is a module                                                 |
| **Main Function**     | `main: () -> Void`               | Program entry point                                                         |
| **Thread Safety**     | ref automatically chooses Rc/Arc | Compiler escape analysis, transparent to users                              |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang Implementation Roadmap (Example)        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: Rust AOT│
│        ✅ Completed                  │ (Current Phase)             Compiler │
│                                       │                                      │
│                                       ▼                                      │
│  v0.6: YaoXiang Interpreter ←─────── v1.0: YaoXiang JIT Compiler ←──── v2.0:│
│        (Bootstrap)                       (Bootstrap)                  YaoXiang│
│                                                                          AOT   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## VI. How to Contribute

YaoXiang is a language born in the community, grown in the community, and serving the community. We
sincerely invite every developer who loves programming language design to join this exploratory
journey.

### 6.1 Design Discussion

**Suitable for**: Programming language theorists, type system enthusiasts, language design fanatics

**Ways to Participate**:

- **GitHub Discussions**: Participate in discussions in the "Language Design" category
- **Design Proposals (RFC)**: Propose design documents for new features, following the template in
  the `rfcs/` directory
- **Syntax Review**: Propose improvements or identify potential issues in existing syntax design

| **Current Hot Topics**: | | | | - Macro system design and implementation | | - Interface type
mechanism | | - Error handling syntax optimization | | - Standard library API design |

**Submitting Design Proposals**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros and cons analysis, alternatives)
3. Initiate a Pull Request for community review
4. Merge or reject after core team review

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, system programmers, performance optimization experts

**Current Implementation Focus** (sorted by priority):

| Priority | Module                       | Description                                         | Difficulty |
| -------- | ---------------------------- | --------------------------------------------------- | ---------- |
| P0       | **Bytecode Virtual Machine** | VM instruction completion, performance optimization | Medium     |
| P0       | **Runtime Memory**           | GC implementation, memory allocator                 | High       |
| P0       | **Async Runtime**            | Complete implementation of the spawn model          | High       |
| P1       | Standard Library             | IO, String, List, Concurrent                        | Medium     |
| P1       | JIT Compiler                 | Cranelift integration                               | High       |
| P2       | AOT Compiler                 | LLVM/Cranelift backend                              | High       |
| P3       | Bootstrap Compiler           | Rewrite in YaoXiang                                 | Extreme    |

**Technology Stack**:

- **Implementation Language**: Rust (current phase)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Test Framework**: Rust `#[test]` + `cargo nextest`

**Start Contributing**:

1. Check `docs/YaoXiang-implementation-plan.md` to understand the architecture design
2. Select a module of interest under the `src/` directory
3. Check `tests/unit/` for test requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, efficiency tool seekers

**Tools that need to be developed**:

| Tool                     | Status           | Description                                |
| ------------------------ | ---------------- | ------------------------------------------ |
| **LSP Server**           | ⏳ To Be Started | Language Server Protocol support           |
| **Debugger Integration** | ⏳ To Be Started | GDB/LLDB integration                       |
| **Formatter**            | ⏳ To Be Started | `yx format`                                |
| **Package Manager**      | ⏳ To Be Started | Dependency management, version resolution  |
| **Package Repository**   | ⏳ To Be Started | Central repository or decentralized        |
| **REPL**                 | ⏳ To Be Started | Interactive interpreter                    |
| **Benchmarking Tool**    | ⏳ To Be Started | Performance analysis                       |
| **VS Code Plugin**       | ⏳ To Be Started | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin**    | ⏳ To Be Started | Syntax highlighting, LSP client            |

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

### 6.4 Standard Library Construction

**Suitable for**: Library developers, API designers, domain experts

**Standard Library Module Planning**:

| Module           | Priority | Description                       |
| ---------------- | -------- | --------------------------------- |
| `std.io`         | P0       | File IO, console input/output     |
| `std.string`     | P0       | String operations, formatting     |
| `std.list`       | P0       | List/array operations             |
| `std.dict`       | P0       | Dictionary/Hash table             |
| `std.math`       | P0       | Mathematical functions, constants |
| `std.time`       | P1       | Date and time operations          |
| `std.net`        | P1       | Network programming, HTTP         |
| `std.concurrent` | P1       | Concurrency primitives, channels  |
| `std.crypto`     | P2       | Cryptographic hashing, signatures |
| `std.json`       | P1       | JSON parsing/generation           |
| `std.regex`      | P2       | Regular expressions               |
| `std.database`   | P3       | Database connections              |
| `std.gui`        | P3       | Graphical interface (long-term)   |

**Design Principles**:

- Consistency: Functions with the same functionality have consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoiding over-engineering
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documents that need contribution**:

| Document               | Status           | Description                               |
| ---------------------- | ---------------- | ----------------------------------------- |
| Quick Start            | ✅ Completed     | 5-minute getting started guide            |
| Language Guide         | ✅ Completed     | Systematic learning of core concepts      |
| Language Specification | ✅ Completed     | Complete syntax and semantic definitions  |
| Implementation Plan    | ✅ Completed     | Compiler implementation technical details |
| API Documentation      | ⏳ To Be Started | Standard library API reference            |
| Tutorials              | ⏳ To Be Started | Advanced tutorials and best practices     |
| Blog                   | ⏳ To Be Started | Technical articles and design stories     |
| Translation            | ⏳ To Be Started | Multi-language support                    |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

**Community Activities**:

- Regular online Meetup (monthly)
- Design and implementation discussions (weekly)
- Code contribution Sprints (quarterly)
- Offline gatherings and conference talks

**Communication Channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Bug reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guidelines

**How to Start Contributing**:

1. **Understand the project**: Read the README and design documents
2. **Choose a direction**: Select a contribution area based on your interests
3. **Set up the environment**: Rust 1.75+, cargo, git
4. **Find tasks**: Check the `good first issue` label in GitHub Issues
5. **Submit a PR**: Follow the commit conventions, write tests
6. **Participate in reviews**: Review others' code, participate in discussions

**Commit Conventions**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: New feature
fix: Bug fix
docs: Documentation update
style: Code formatting (does not affect functionality)
refactor: Refactoring
perf: Performance optimization
test: Tests
chore: Build tools or auxiliary tools

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

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword                 | Function                                                 |
| ----------------------- | -------------------------------------------------------- |
| `pub`                   | Public export                                            |
| `use`                   | Import module                                            |
| `spawn`                 | Spawn marker                                             |
| `ref`                   | Shared ownership (compiler automatically chooses Rc/Arc) |
| `mut`                   | Mutable variable                                         |
| `if/else if/else`       | Conditional branch                                       |
| `match`                 | Pattern matching                                         |
| `while/for`             | Loop                                                     |
| `return/break/continue` | Control flow                                             |
| `as`                    | Type conversion                                          |
| `in`                    | Member detection / list comprehension                    |
| `unsafe`                | unsafe code block (raw pointers)                         |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type`
> keyword has been removed in RFC-010, with the unified `name: Type = value` syntax.

### A.3 Primitive Types

| Type     | Description       | Default Size |
| -------- | ----------------- | ------------ |
| `Void`   | Void value        | 0 bytes      |
| `Bool`   | Boolean value     | 1 byte       |
| `Int`    | Signed integer    | 8 bytes      |
| `Uint`   | Unsigned integer  | 8 bytes      |
| `Float`  | Float             | 8 bytes      |
| `String` | UTF-8 string      | Variable     |
| `Char`   | Unicode character | 4 bytes      |
| `Bytes`  | Raw bytes         | Variable     |

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

> Unary prefix operators (`!` `-` `+`) are tightly bound, higher than all binary operators
> (Zig-style, see SPEC §2.2).

---

## Appendix B: Design Inspirations

YaoXiang's design draws on the excellent ideas of the following languages and projects:

| Source                       | Reference Point                                                                     |
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

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions, but adopts simpler syntax and a
lower cognitive burden. The **Spawn Model** is more concise than Rust's `async/await`—just one
`spawn` marker, no need to manually manage Future and Pin. "The myriad things arise together, I
observe their returns," making concurrent programming as intuitive as describing natural laws. The
**Ownership Model** (RFC-009 v9) uses Move + &T/&mut T tokens to replace lifetime annotations, and
uses type attributes (Dup/Linear) to replace the borrow checker. The unified type syntax eliminates
the concept fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What types of development is YaoXiang suitable for?**

A: System programming, application development, web services, scripting tools, AI-assisted
programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing confusion caused by nesting
depth. This is a well-thought-out "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. The release time depends on the implementation progress, see
[Version Planning RFC](./rfc/003-version-planning.md).

**Q: How to contact the core team?**

A: Through GitHub Discussions or the Discord community channel. Core team members will respond
regularly.

---

> **Last Updated**: 2026-05-31
>
> **Document Version**: v2.0.0
>
> **License**: [MIT](LICENSE)

---

> "YaoXiang changes, the myriad things are born. Types evolve, programs are completed."
>
> May YaoXiang's design journey walk with you.
