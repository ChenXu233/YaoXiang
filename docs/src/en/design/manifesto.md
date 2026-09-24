# YaoXiang Design Manifesto

> **Version**: v2.0.0 **Status**: Official Release **Author**: Chen Xu + YaoXiang Community **Date**: 2026-05-31

---

> 「The Tao gave birth to the One, the One gave birth to the Two, the Two gave birth to the Three, the Three gave birth to all things.」—— *Dao De Jing*
>
> Types are like the Tao, from which all things are born.

---

## I. Why Create YaoXiang?

### 1.1 The Language Gap

In the long history of programming languages, we have witnessed the birth and evolution of countless excellent languages: C brought efficiency revolution to systems programming, Python created a programming experience accessible to everyone, Rust proved that memory safety and performance can coexist, and TypeScript made large frontend projects maintainable. However, when we examine today's language ecosystem, we still find a clear gap—**no single language can simultaneously satisfy these three core requirements**:

| Requirement     | Problems with Existing Solutions                                                                                       |
| --------------- | ---------------------------------------------------------------------------------------------------------------------- |
| **Type Safety** | Rust is overly strict with a steep learning curve; TypeScript has optional types, unable to provide compile-time guarantees |
| **Natural Syntax** | Rust syntax is complex and obscure; Haskell's functional barrier is too high; traditional static languages are verbose |
| **AI-Friendly** | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behaviors that limit AI code generation and modification accuracy |

The birth of YaoXiang is precisely to fill this gap. We believe: **A programming language should be both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Practical Problems Solved

**Problem One: Type System Fragmentation**

Today's programming languages exhibit severe fragmentation in their type systems. Statically typed languages pursue absolute compile-time correctness, but often at the cost of development efficiency; dynamically typed languages provide flexibility but reveal maintenance defects in large projects. YaoXiang proposes a unified abstraction framework of "everything is a type," making types the through-line of language design, not patches added afterward.

**Problem Two: The Binary Choice Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime performance. Garbage Collection (GC) frees developers but brings latency jitter and memory overhead; manual memory management is efficient but as dangerous as walking a tightrope. YaoXiang adopts Rust's ownership model, eliminating data races and memory leaks at compile-time while maintaining zero-cost abstractions, achieving high performance without GC.

**Problem Three: Cognitive Burden of Async Programming**

Modern applications are inseparable from networking and concurrency, and async programming has always been a nightmare for programmers. Nested callback functions, Promise chaining, async/await syntax—each approach adds complexity to the code. YaoXiang has redesigned the async model: simply add a `spawn` marker after the function signature, and the compiler automatically handles all async details, making concurrent programming as natural as synchronous code.

**Problem Four: Bottleneck in AI-Assisted Programming**

When AI begins to assist developers in writing code, the choices in language design become crucial. Fuzzy syntax rules, implicit type conversions, complex syntactic sugar—these features that human programmers have grown accustomed to become obstacles for AI to understand and generate. YaoXiang has made "AI-friendly" a core goal from design inception: strict indentation rules, clear code block boundaries, unambiguous syntax structures, enabling AI to accurately understand, generate, and modify code.

### 1.3 Philosophical Roots of the Language

The name YaoXiang comes from "Yao" (爻) and "Xiang" (象) in the I Ching (Book of Changes). "Yao" are the basic symbols that compose hexagrams, symbolizing the interplay of yin and yang, motion and stillness; "Xiang" is the external manifestation of the essence of things, representing all phenomena and the vast array of existence.

This philosophical thinking is reflected in every detail of the language design:

- **Unity**: Just as simple yao symbols compose complex hexagrams, YaoXiang builds a complete programming model from a few core concepts (types, functions, constructors)
- **Hierarchy**: Just as Xiang has distinctions between prior and later heaven, YaoXiang's type system has a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Variability**: Just as yin and yang flow and change infinitely, YaoXiang supports dependent types, allowing types to evolve with values
- **Identifiability**: Just as hexagrams can be interpreted and all things can be manifested, YaoXiang provides complete type reflection capabilities, with runtime type information fully available
- **Provability**: Just as hexagrams reveal the laws of things, YaoXiang's type system follows the Curry-Howard isomorphism (types are propositions, programs are proofs), where type checking is the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and unbreakable**. Every feature proposal must pass scrutiny by these principles.

### 2.1 Principle One: Everything is a Type

In YaoXiang's worldview, types are the highest-level abstraction unit, the core concept running through the language.

**Specific Manifestations**:

- **Values are instances of types**: `42` is an instance of type `Int`, `"hello"` is an instance of type `String`
- **Types themselves are also types**: `Type` is the language's only meta type keyword, and the type of `Int` is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from `Int × Int` to `Int`
- **Modules are type compositions**: Modules are named combinations of namespaces containing functions and types

**Non-negotiable Reason**: Unified type abstraction simplifies language semantics, eliminating the duality between values and types, making the type system the guardian of code correctness, not an obstacle.

### 2.2 Principle Two: Strictly Structured

YaoXiang's syntax design pursues "unambiguous, predictable, and easy to parse."

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are prohibited, code block boundaries are clear at a glance
- **Parentheses cannot be omitted**: Function parameters must have parentheses, list elements must have commas
- **Code blocks must have curly braces**: Control flow like `if`, `while`, `for` must use `{ }` to wrap
- **Streamlined keyword count**: Only 17 core keywords are retained, syntactic sugar is rejected

**Non-negotiable Reason**: Strict structure brings three key advantages—(1) more accurate IDE syntax highlighting and code folding; (2) significantly improved AI code generation and modification accuracy; (3) newcomers can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstraction

High-level abstractions should not bring runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: Generic functions are expanded into concrete versions at compile-time, with no virtual table lookup overhead
- **Inlining optimization**: Simple functions are automatically inlined, eliminating function call overhead
- **Stack allocation priority**: Small objects are stack-allocated by default, heap allocation is used only when necessary
- **No GC**: The ownership model ensures memory safety, no runtime overhead from garbage collectors

**Non-negotiable Reason**: Performance is the survival bottom line of programming languages. Any design that trades convenience for performance is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability comes with complexity like a shadow. YaoXiang chooses immutable by default, making code easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default and cannot be modified after assignment
- When mutability is needed, it must be explicitly declared with `mut`
- References are immutable by default, mutable references need the `mut` marker
- Ownership transfer means the original binding becomes invalid

**Non-negotiable Reason**: Immutability is the foundation of concurrency safety, the guarantee of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types as Data

Type information should not exist only at compile-time but should be fully available at runtime.

**Specific Capabilities**:

- Runtime type querying: Any value can obtain its type information
- Type reflection: Types themselves can be constructed and manipulated
- Pattern matching destructuring: Type constructors can be directly used in pattern matching
- Generic specialization: Runtime can obtain the materialized type of generic parameters

**Non-negotiable Reason**: Complete type reflection capability is the foundation of metaprogramming and the cornerstone of high-performance frameworks and tools.

---

## III. Key Innovations and Features

YaoXiang, while absorbing the excellent features of existing languages, proposes the following innovative designs.

### 3.1 Innovation One: Unified Type Syntax

**Traditional language type definitions** often require multiple keywords:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
trait Drawable { fn draw(&self, s: &Surface); }
```

**YaoXiang's unified syntax**: Everything is `name: type = value`, `Type` is the only meta type keyword.

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

# === Interfaces (records with all function fields) ===

Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

Serializable: Type = {
    serialize: () -> String,
}

# === Interface Implementation (interface name inside the type body) ===

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

**Innovation Value**: No fragmentation of `fn`, `struct`, `enum`, `trait`, `impl` keywords—one unified syntax covers all declarations.

### 3.2 Innovation Two: Constructors are Types

**Value construction is exactly the same as function calls**:

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

YaoXiang adopts a pure functional design, achieving object method call syntax sugar through currying, without introducing `class` and `method` keywords.

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

# Method syntax sugar binding ([0] means binding to the 0th parameter position)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Both calling styles are completely equivalent
d1 = distance(p1, p2)     # Direct core function call
d2 = p1.distance(p2)      # Method syntax sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, waiting for second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovation Value**: Pure functional design, no hidden `self` parameter, functions are values that can be freely passed and composed.

### 3.4 Innovation Four: Spawn Model

> 「All things arise together, I observe their return.」—— *I Ching, Hexagram Fu*
>
> The spawn model draws its meaning from this, describing a programming paradigm: developers describe logic with synchronous, sequential thinking, while the language runtime causes its computational units to automatically and efficiently execute concurrently like all things arising together, and ultimately unify and coordinate at the end.

**Core Three Principles**:

| Principle        | Description                                             |
| ---------------- | ------------------------------------------------------- |
| **Sync Syntax**  | What you see is sequential code                        |
| **Concurrent Essence** | Runtime automatically extracts parallelism          |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology System**:

| Official Term   | Corresponding Syntax            | Explanation                                                                       |
| --------------- | ------------------------------- | --------------------------------------------------------------------------------- |
| **Spawn Function** | `spawn (params) => body`     | Defines a computational unit that can participate in concurrent execution           |
| **Spawn Block**     | `spawn { a(), b() }`        | Explicitly declared concurrent scope, tasks inside spawn block execute concurrently |
| **Spawn Loop**      | `spawn for x in xs { ... }` | Data parallelism, loop body spawns on all elements                               |
| **Spawn Value**      | `Async(T)`                  | A future value currently spawning, automatically awaited when used               |
| **Spawn Graph**      | Lazy evaluation DAG         | The stage where spawning occurs, describing dependencies and parallelism           |
| **Spawn Scheduler**  | Runtime task scheduler      | The intelligent hub that coordinates all things, making them spawn at the right time |

> **See also**: [RFC-001 Spawn Model](./rfc/deprecated/001-concurrent-model-error-handling.md)

```yaoxiang
# === Spawn Functions ===
# Functions marked with spawn
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === Spawn Blocks ===
# Expressions inside spawn { } force parallel execution
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

    # Await points are automatically inserted when results are needed
    print(users.length + posts.length)  # Automatically awaits users and posts
}
```

**Thread Safety**:

```yaoxiang
# ref keyword automatically handles thread safety (compiler automatically selects Rc/Arc)
main: () -> Void = {
    counter = ref SafeCounter(0)

    # Cross-task sharing: compiler automatically selects Arc
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

**Innovation Value**: The cognitive burden of async programming drops to zero, code readability is completely identical to synchronous code, while achieving high-performance parallel execution efficiency.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, enabling true type-driven development.

```yaoxiang
# Matrix type: dimensions determined at compile-time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time computation: factorial(3) = 6
arr: Array(Int, factorial(3)) = Array(Int, 6)()

# Compile-time dimension verification
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimension mismatch
```

**Innovation Value**: Capture more errors at compile-time, achieving more precise type guarantees.

### 3.6 Innovation Six: Minimalist Keyword Design

YaoXiang defines only 17 core keywords, far fewer than mainstream languages:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

| Comparison Language | Keyword Count |
| ------------------- | ------------- |
| YaoXiang            | **17**        |
| Rust                 | 51+           |
| Python               | 35            |
| TypeScript           | 64+           |
| Go                   | 25            |

**Innovation Value**: Lower memory burden, more consistent syntax style, easier-to-parse syntax structure.

---

## IV. Preliminary Syntax Preview

The following code examples showcase YaoXiang's language style, helping you quickly feel its design aesthetics.

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

# Interface type (records with all function fields)
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
p2 = p1              # Move, p1 cannot be read again

# &T / &mut T tokens (compile-time zero overhead)
p2.print()           # Compiler automatically creates &Point token
p2.shift(1.0, 1.0)  # Compiler automatically creates &mut Point token

# ref: shared ownership (compiler automatically selects Rc/Arc)
shared = ref p2      # Cross-scope sharing

# clone(): explicit deep copy
backup = p2.clone()

# unsafe + raw pointers: systems-level
unsafe {
    ptr: *Point = &p2
    (*ptr).x = 0.0
}
```

**Ownership Gradient**:

```
&T / &mut T    Move       ref        clone()    unsafe
    |             |          |           |          |
Borrow Token  Default   Shared Own   Deep Copy  Raw Ptr
Zero-Cost     Zero-Copy Auto Rc/Arc Explicit    Systems
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
# spawn-marked async function
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

The following decisions have been fully discussed and reviewed, **no longer accepting changes**:

| Module           | Decision                              | Description                                                                          |
| ---------------- | ------------------------------------- | ------------------------------------------------------------------------------------ |
| **Type System**  | Everything is a type                  | Values, functions, modules, generics are all types                                   |
| **Type Syntax**  | Unified `name: type = value`          | One declaration form covers all cases, `Type` is the only meta type keyword         |
| **Keywords**     | 17 core keywords                      | Excludes `type`/`fn`/`struct`/`enum`/`trait`/`impl`                                  |
| **Function Syntax** | Signature + expression             | `name: (params) -> ReturnType = body`                                                |
| **Method Binding** | RFC-004 Curried Binding            | `Type.method = function[position]`                                                   |
| **Async Model**  | Spawn model                           | `spawn` marker, lazy evaluation, automatic parallelism                                |
| **Memory Management** | Ownership model (RFC-009 v9)    | Move + &T/&mut T tokens + ref + clone + unsafe, no GC                                 |
| **File as Module** | Module system                     | Every `.yx` file is a module                                                          |
| **Main Function** | `main: () -> Void`                 | Program entry point                                                                  |
| **Thread Safety** | ref auto-select Rc/Arc             | Compiler escape analysis, invisible to users                                         |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           YaoXiang Implementation Roadmap (Example)             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: AOT  │
│        ✅ Completed                   │ (Current Stage)            Compiler    │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←─────── v1.0: YaoXiang JIT Compiler ←──── v2.0 │
│        (Self-hosted)                 (Self-hosted)                    YaoXiang AOT │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## VI. How to Contribute

YaoXiang is a language born from the community, growing in the community, and serving the community. We sincerely invite every developer passionate about programming language design to join this exploration journey.

### 6.1 Design Discussion

**Target Audience**: Programming language theory researchers, type system enthusiasts, language design fanatics

**Participation Methods**:

- **GitHub Discussions**: Participate in discussions in the "Language Design" category
- **Design Proposals (RFC)**: Propose design documents for new features, following templates under the `rfcs/` directory
- **Syntax Review**: Provide improvement suggestions or discover potential issues for existing syntax design

| **Current Hot Topics**: | |
| | - Macro system design and implementation |
| | - Interface type mechanism |
| | - Error handling syntax optimization |
| | - Standard library API design |

**Submitting Design Proposals**:

1. Create a new file in the `rfcs/` directory
2. Fill out the RFC template (motivation, detailed design, pros/cons analysis, alternatives)
3. Submit a Pull Request for community review
4. Merged or rejected after core team deliberation

### 6.2 Compiler Implementation

**Target Audience**: Compiler developers, systems programmers, performance optimization experts

**Current Implementation Priorities** (in order of priority):

| Priority | Module              | Description                          | Difficulty |
| -------- | ------------------- | ------------------------------------ | ---------- |
| P0       | **Bytecode VM**     | VM instruction perfection, optimization | Medium  |
| P0       | **Runtime Memory** | GC implementation, memory allocators | High      |
| P0       | **Async Runtime**   | Complete spawn model implementation  | High      |
| P1       | Standard Library   | IO, String, List, Concurrent         | Medium    |
| P1       | JIT Compiler       | Cranelift integration                | High      |
| P2       | AOT Compiler       | LLVM/Cranelift backend               | High      |
| P3       | Self-Host Compiler | Rewrite in YaoXiang                  | Extreme   |

**Tech Stack**:

- **Implementation Language**: Rust (current stage)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Testing Framework**: Rust `#[test]` + `cargo nextest`

**Starting Contributions**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand architecture design
2. Choose a module of interest under the `src/` directory
3. Check `tests/unit/` to understand testing requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Target Audience**: IDE plugin developers, toolchain enthusiasts, productivity tool seekers

**Tools to Develop**:

| Tool                | Status        | Description              |
| ------------------- | ------------- | ------------------------ |
| **LSP Server**      | ⏳ Not started | Language Server Protocol support |
| **Debugger Integration** | ⏳ Not started | GDB/LLDB integration     |
| **Formatter**       | ⏳ Not started | `yx format`              |
| **Package Manager** | ⏳ Not started | Dependency management, version resolution |
| **Package Registry**| ⏳ Not started | Centralized or decentralized |
| **REPL**            | ⏳ Not started | Interactive interpreter  |
| **Benchmark Tool**  | ⏳ Not started | Performance analysis     |
| **VS Code Plugin**  | ⏳ Not started | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin** | ⏳ Not started | Syntax highlighting, LSP client |

**Project Structure Reference**:

```
yaoxiang/
├── src/
│   ├── tools/                    # Toolchain
│   │   ├── lsp/                  # LSP Server
│   │   ├── fmt/                  # Formatter
│   │   ├── repl/                 # REPL
│   │   └── benchmark/            # Benchmark
│   └── ...
├── extensions/                   # Editor Extensions
│   ├── vscode/                   # VS Code
│   └── vim/                      # Vim/Neovim
```

### 6.4 Standard Library Construction

**Target Audience**: Library developers, API designers, domain experts

**Standard Library Module Planning**:

| Module             | Priority | Description                      |
| ------------------ | -------- | -------------------------------- |
| `std.io`           | P0       | File IO, console input/output    |
| `std.string`       | P0       | String operations, formatting     |
| `std.list`         | P0       | List/array operations            |
| `std.dict`         | P0       | Dictionary/hash table            |
| `std.math`         | P0       | Math functions, constants        |
| `std.time`         | P1       | Time and date operations          |
| `std.net`          | P1       | Network programming, HTTP         |
| `std.concurrent`   | P1       | Concurrency primitives, channels |
| `std.crypto`       | P2       | Cryptographic hashing, signatures |
| `std.json`         | P1       | JSON parsing/generation           |
| `std.regex`        | P2       | Regular expressions               |
| `std.database`     | P3       | Database connections              |
| `std.gui`          | P3       | GUI (long-term)                  |

**Design Principles**:

- Consistency: Functions with the same functionality maintain consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoid over-design
- Performance: Standard library functions should be efficient, avoid unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Target Audience**: Technical writers, educators, community managers

**Documentation Needed**:

| Document      | Status        | Description                |
| ------------- | ------------- | -------------------------- |
| Quick Start   | ✅ Completed  | 5-minute getting started   |
| Language Guide | ✅ Completed | Systematic learning of core concepts |
| Language Spec | ✅ Completed | Complete syntax and semantics definition |
| Impl Plan     | ✅ Completed | Compiler implementation technical details |
| API Docs      | ⏳ Not started | Standard library API reference |
| Tutorials     | ⏳ Not started | Advanced tutorials and best practices |
| Blog          | ⏳ Not started | Technical articles and design stories |
| Translation   | ⏳ Not started | Multi-language support     |

### 6.6 Community Building

**Target Audience**: Community managers, event organizers, evangelists

**Community Activities**:

- Regular online Meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution Sprints (quarterly)
- In-person gatherings and conference talks

**Communication Channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Bug reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guidelines

**How to Start Contributing**:

1. **Understand the project**: Read README and design documents
2. **Choose a direction**: Select a contribution area based on interest
3. **Set up environment**: Rust 1.75+, cargo, git
4. **Find tasks**: Check GitHub Issues for `good first issue` tags
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
style: Code formatting (no functional changes)
refactor: Refactoring
perf: Performance optimization
test: Testing
chore: Build tools or auxiliary tools

# Examples
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**Code Style**:

- Follow `rustfmt.toml` conventions
- Ensure `cargo clippy` has no warnings
- Write necessary unit tests
- Update relevant documentation

---

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword                | Purpose                            |
| ---------------------- | ---------------------------------- |
| `pub`                  | Public export                      |
| `use`                  | Import module                      |
| `spawn`                | Spawn marker                       |
| `ref`                  | Shared ownership (compiler auto-selects Rc/Arc) |
| `mut`                  | Mutable variable                   |
| `if/else if/else`      | Conditional branches               |
| `match`                | Pattern matching                   |
| `while/for`            | Loops                              |
| `return/break/continue`| Control flow                      |
| `as`                   | Type conversion                    |
| `in`                   | Membership test/list comprehension |
| `unsafe`               | Unsafe code block (raw pointers)   |

> **Note**: `Type`, `true`, `false`, `void` etc. are reserved words, not keywords. The `type` keyword has been removed in RFC-010, using the unified `name: Type = value` syntax instead.

### A.3 Primitive Types

| Type     | Description         | Default Size |
| -------- | ------------------- | ------------ |
| `Void`   | Empty value         | 0 bytes      |
| `Bool`   | Boolean value        | 1 byte       |
| `Int`    | Signed integer      | 8 bytes      |
| `Uint`   | Unsigned integer    | 8 bytes      |
| `Float`  | Floating-point      | 8 bytes      |
| `String` | UTF-8 string        | Variable     |
| `Char`   | Unicode character   | 4 bytes      |
| `Bytes`  | Raw bytes           | Variable     |

### A.4 Operator Precedence

| Precedence | Operators                      | Associativity |
| ---------- | ------------------------------ | ------------- |
| 1          | `()` `[]` `.` `?`              | Left to right |
| 2          | `as`                           | Left to right |
| 3          | Unary prefix `!` `-` `+`        | Right to left |
| 4          | `*` `/` `%`                    | Left to right |
| 5          | `+` `-`                        | Left to right |
| 6          | `..`                           | Left to right |
| 7          | `<<` `>>`                      | Left to right |
| 8          | `&` `\|` `^`                   | Left to right |
| 9          | `==` `!=` `<` `>` `<=` `>=`    | Left to right |
| 10         | `and` `or`                     | Left to right |
| 11         | `if...else`                    | Right to left |
| 12         | `=` `+=` `-=` `*=` `/=`        | Right to left |

> Unary prefix operators (`!` `-` `+`) bind tightly, higher than all binary operators (Zig style, see SPEC §2.2).

---

## Appendix B: Design Inspirations

YaoXiang's design draws from the excellent ideas of the following languages and projects:

| Source                 | Borrowed Points                                           |
| ---------------------- | --------------------------------------------------------- |
| **Rust**               | Ownership model, zero-cost abstractions, type system      |
| **Python**             | Syntax style, readability, list comprehensions           |
| **Idris/Agda**         | Dependent types, type-driven development                 |
| **Curry-Howard Isomorphism** | Types are propositions, programs are proofs, unified theory of type systems and logic |
| **TypeScript**         | Type annotations, runtime types                           |
| **MoonBit**            | AI-friendly design, concise syntax                        |
| **Haskell**            | Pure functional, pattern matching                         |
| **OCaml**              | Type inference, variant types                             |

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have compared to Rust?**

A:
YaoXiang retains Rust's memory safety and zero-cost abstractions but uses simpler syntax and lower cognitive burden. The **spawn model** is more concise than Rust's `async/await`—just one `spawn` marker, no manual Future and Pin management. "All things arise together, I observe their return," making concurrent programming as intuitive as describing natural laws. The **ownership model** (RFC-009 v9) uses Move + &T/&mut T tokens instead of lifetime annotations, and type attributes (Dup/Linear) instead of the borrow checker. Unified type syntax eliminates the conceptual fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What types of development is YaoXiang suitable for?**

A: Systems programming, application development, Web services, scripting tools, AI-assisted programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A:
4 spaces provide clear visual separation of code blocks, reducing confusion from nesting depth. This is a carefully considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. Release timing depends on implementation progress; see [Version Planning RFC](./rfc/rejected/003-version-planning.md).

**Q: How to contact the core team?**

A: Through GitHub Discussions or Discord community channels. Core team members respond regularly.

---

> **Last Updated**: 2026-05-31
>
> **Document Version**: v2.0.0
>
> **License**: MIT

---

> 「The changes of YaoXiang give birth to all things. Type evolution creates programs.」
>
> May the journey of YaoXiang's design walk alongside you.