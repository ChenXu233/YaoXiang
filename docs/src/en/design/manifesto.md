# YaoXiang (爻象) Design Manifesto

> **Version**: v2.0.0
> **Status**: Official Release
> **Author**: Chenxu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Tao gives birth to one, one gives birth to two, two gives birth to three, three gives birth to the myriad things."
> — *Tao Te Ching*
>
> Types are like the Tao; the myriad things are born from them.

---

## 1. Why Create YaoXiang?

### 1.1 Filling the Language Gap

In the long history of programming languages, we have witnessed the birth and evolution of countless excellent languages: C brought the efficiency revolution in systems programming, Python created a programming experience accessible to everyone, Rust proved that memory safety and performance can coexist, and TypeScript made large-scale frontend projects maintainable. However, when we look at today's language ecosystem, we still see a clear fault line — **no language can simultaneously satisfy the following three core needs**:

| Need | Problems with Existing Solutions |
|------|----------------------------------|
| **Type safety** | Rust is too strict, with a steep learning curve; TypeScript has optional types and cannot provide compile-time guarantees |
| **Natural syntax** | Rust's syntax is complex and obscure; Haskell's functional paradigm has too high a barrier; traditional static languages are verbose and cumbersome |
| **AI-friendly** | Existing languages have many syntactic ambiguities, complex ASTs, and unpredictable hidden behaviors, limiting the accuracy of AI-generated and AI-modified code |

The birth of YaoXiang is precisely to fill this gap. We believe: **a programming language should be both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Real Problems Solved

**Problem 1: Fragmentation of Type Systems**

Today's programming languages exhibit severe fragmentation in their type systems. Statically-typed languages pursue absolute correctness at compile-time, but often at the cost of development efficiency; dynamically-typed languages provide flexibility, but expose hard-to-maintain defects in large projects. YaoXiang proposes the unified abstraction framework of "everything is a type," making types the main thread that runs through language design, rather than a patch added afterwards.

**Problem 2: The Binary Choice Between Memory Safety and Performance**

For a long time, developers have had to make a difficult choice between memory safety and runtime performance. GC (garbage collection) liberates developers but brings latency fluctuations and memory overhead; manual memory management is efficient but dangerous, like walking a tightrope. YaoXiang adopts a Rust-style ownership model to eliminate data races and memory leaks at compile-time, while maintaining zero-cost abstractions and achieving high performance without a GC.

**Problem 3: The Cognitive Load of Asynchronous Programming**

Modern applications cannot do without networking and concurrency, and asynchronous programming has long been a programmer's nightmare. Callback nesting, Promise chains, async/await syntax — every solution increases code complexity. YaoXiang redesigns the asynchronous model: simply add the `spawn` keyword after a function signature, and the compiler automatically handles all asynchronous details, making concurrent programming as natural as synchronous code.

**Problem 4: The Bottleneck of AI-Assisted Programming**

As AI begins to assist developers in writing code, language design choices become crucial. Ambiguous syntactic rules, implicit type conversions, complex syntactic sugar — characteristics that human programmers have long grown used to — become obstacles for AI to understand and generate. From the very beginning of its design, YaoXiang has placed "AI-friendly" as a core goal: strict indentation rules, clear code block boundaries, unambiguous syntactic structures, allowing AI to accurately understand, generate, and modify code.

### 1.3 The Philosophical Roots of the Language

The name YaoXiang originates from "爻" (*yáo*) and "象" (*xiàng*) in the *I Ching*. "爻" is the basic symbol that composes the trigrams and hexagrams, symbolizing the alternation of yin and yang and the interplay of motion and stillness; "象" is the external manifestation of the essence of things, representing the myriad phenomena of the world.

This philosophical thought is reflected in every detail of the language's design:

- **Unity**: Just as the simple symbols of the yaos compose the complex trigrams, YaoXiang uses a few core concepts (types, functions, constructors) to build a complete programming model.
- **Hierarchy**: Just as xiangs have innate and acquired forms, YaoXiang's type system has a clear hierarchical structure, from primitive types to generics, from values to meta types.
- **Changeability**: Just as yin and yang flow and transform endlessly, YaoXiang supports dependent types, allowing types to evolve as values change.
- **Identifiability**: Just as trigrams can be interpreted and all things can be symbolized, YaoXiang provides complete type reflection capabilities, with full runtime type information available.
- **Provability**: Just as trigrams reveal the laws of things, YaoXiang's type system follows the Curry-Howard correspondence (types as propositions, programs as proofs); the type-checking process is the verification of a logical proof.

---

## 2. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any feature proposal must be tested against these principles.

### 2.1 Principle 1: Everything Is a Type

In YaoXiang's worldview, types are the highest-level abstraction units and the core concept that runs through the language.

**Concretely**:

- **Values are instances of types**: `42` is an instance of type `Int`; `"hello"` is an instance of type `String`.
- **Types themselves are also types**: `Type` is the language's only meta type keyword; the type of `Int` is `Type`.
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from `Int × Int` to `Int`.
- **Modules are type compositions**: modules are namespace compositions that contain functions and types.

**Why non-negotiable**: A unified type abstraction simplifies language semantics, eliminates the duality between values and types, and lets the type system serve as the guardian of code correctness rather than a stumbling block.

### 2.2 Principle 2: Strict Structuring

YaoXiang's syntax design pursues the goals of "unambiguous, predictable, and easy to parse."

**Specific rules**:

- **Mandatory 4-space indentation**: Tab characters are forbidden; code block boundaries are immediately visible.
- **Brackets cannot be omitted**: function parameters must be enclosed in parentheses, list elements must be separated by commas.
- **Code blocks must use curly braces**: control-flow constructs like `if`, `while`, `for` must be wrapped in `{ }`.
- **Streamlined keyword set**: only 17 core keywords are retained; the proliferation of syntactic sugar is rejected.

**Why non-negotiable**: Strict structuring brings three key advantages — (1) IDE syntax highlighting and code folding become more accurate; (2) the accuracy of AI code generation and modification is significantly improved; (3) new learners can quickly understand code structure.

### 2.3 Principle 3: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific guarantees**:

- **Monomorphization**: generic functions are expanded into concrete versions at compile-time, with no virtual-table lookup overhead.
- **Inline optimization**: simple functions are automatically inlined, eliminating function-call overhead.
- **Stack allocation preferred**: small objects are stack-allocated by default; heap allocation is used only when necessary.
- **No GC**: the ownership model guarantees memory safety, without the runtime overhead of a garbage collector.

**Why non-negotiable**: Performance is the lifeline of a programming language. Any design that trades performance for convenience is a betrayal of the programmer.

### 2.4 Principle 4: Immutable by Default

Mutability goes hand in hand with complexity. YaoXiang chooses immutability by default, making code easier to reason about and understand.

**Specific rules**:

- Variables are immutable by default and cannot be modified after assignment.
- When mutability is needed, it must be declared explicitly with `mut`.
- References are immutable by default; mutable references need the `mut` marker.
- Transfer of ownership invalidates the original binding.

**Why non-negotiable**: Immutability is the foundation of concurrency safety, the guarantee of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle 5: Types Are Data

Type information should not exist only at compile-time, but should be fully available at runtime.

**Specific capabilities**:

- Runtime type query: any value can retrieve its type information.
- Type reflection: types themselves can be constructed and manipulated.
- Pattern-matching destructuring: type constructors can be used directly in pattern matching.
- Generic specialization: the concretized types of generic parameters can be obtained at runtime.

**Why non-negotiable**: Complete type reflection is the foundation of metaprogramming and the cornerstone of high-performance frameworks and tools.

---

## 3. Key Innovations and Features

While absorbing the excellent features of existing languages, YaoXiang proposes the following innovative designs.

### 3.1 Innovation 1: Unified Type Syntax

**Traditional languages** often require multiple keywords for type definitions:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
trait Drawable { fn draw(&self, s: &Surface); }
```

**YaoXiang's unified syntax**: everything is `name: type = value`; `Type` is the only meta type keyword.

```yaoxiang
# === Record type ===

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

# === Generic type ===

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

# === Interface implementation (the interface name is written in the type body) ===

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

**Innovative value**: No fragmentation of `fn`, `struct`, `enum`, `trait`, `impl` keywords — one unified syntax covers all declarations.

### 3.2 Innovation 2: Constructors Are Types

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

### 3.3 Innovation 3: Curried Method Binding

YaoXiang adopts a pure functional design and uses currying to provide syntactic sugar reminiscent of object method calls — without introducing `class` or `method` keywords.

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

# Method syntactic-sugar binding ([0] means bind to the 0th parameter position)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# The two call forms are completely equivalent
d1 = distance(p1, p2)     # Direct call to the core function
d2 = p1.distance(p2)      # Method syntactic sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, waiting for the second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovative value**: A pure functional design with no hidden `self` parameter; functions are values that can be freely passed and composed.

### 3.4 Innovation 4: The Spawn Model

> "The myriad things spawn together; I observe their return." — *I Ching*, Return Hexagram
>
> The spawn model takes its meaning from this, describing a programming paradigm: developers describe logic with synchronous, sequential thinking, while the language runtime makes the computational units within automatically and efficiently execute concurrently — like the myriad things spawning — and finally converge in unified coordination.

**The three core principles**:

| Principle | Explanation |
|------|------|
| **Synchronous syntax** | Sequential code: what you see is what you get |
| **Concurrent essence** | The runtime automatically extracts parallelism |
| **Unified coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology**:

| Official Term | Corresponding Syntax | Explanation |
|----------|----------|------|
| **spawn function** | `spawn (params) => body` | Defines a computational unit that can participate in spawn execution |
| **spawn block** | `spawn { a(), b() }` | An explicitly declared concurrent region; tasks inside the block execute via spawn |
| **spawn loop** | `spawn for x in xs { ... }` | Data-parallel: the loop body executes via spawn across all elements |
| **spawn value** | `Async(T)` | A future value that is spawning; automatically awaited when used |
| **spawn graph** | Lazy computation graph (DAG) | The stage on which spawning occurs, describing dependencies and parallelism |
| **spawn scheduler** | Runtime task scheduler | The intelligent hub that coordinates all things, making them spawn at the right time |

> **See**: [RFC-001 The Spawn Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === spawn function ===
# A function marked with spawn
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === spawn block ===
# Expressions inside spawn { } are forced to execute in parallel
compute_all: () -> (Int, Int, Int) spawn = {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    return (a, b, c)
}

# === Automatic waiting ===
main: () -> Void = {
    # Two independent requests automatically execute in parallel
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # The waiting point is automatically inserted when the result is needed
    print(users.length + posts.length)  # Automatically waits for users and posts
}
```

**Thread safety**:

```yaoxiang
# The ref keyword automatically handles thread safety (the compiler automatically chooses Rc/Arc)
main: () -> Void = {
    counter = ref SafeCounter(0)

    # Cross-task sharing: the compiler automatically chooses Arc
    spawn {
        counter.increment()
    }
    spawn {
        counter.increment()
    }
}
```

**Technical documentation**:
- See [RFC-001 The Spawn Model](./rfc/accepted/001-concurrent-model-error-handling.md)

**Innovative value**: The cognitive load of asynchronous programming is reduced to zero; code readability is exactly the same as synchronous code, while gaining high-performance parallel execution efficiency.

### 3.5 Innovation 5: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, enabling true type-driven development.

```yaoxiang
# Matrix type: dimensions are determined at compile-time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time computation: factorial(3) = 6
vec: Vec(factorial(3)) = Vec(6)()

# Compile-time dimension verification
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimension mismatch
```

**Innovative value**: More errors are caught at compile-time, achieving more precise type guarantees.

### 3.6 Innovation 6: Minimal Keyword Design

YaoXiang defines only 17 core keywords — far fewer than mainstream languages:

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

| Compared Language | Number of Keywords |
|----------|-----------|
| YaoXiang | **17** |
| Rust | 51+ |
| Python | 35 |
| TypeScript | 64+ |
| Go | 25 |

**Innovative value**: Lower memorization burden, more consistent syntactic style, and an easier-to-parse syntactic structure.

---

## 4. Initial Syntax Preview

The following code examples showcase the style of YaoXiang and help you quickly appreciate its design aesthetics.

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

# &T / &mut T tokens (zero overhead at compile-time)
p2.print()           # Compiler automatically creates a &Point token
p2.shift(1.0, 1.0)   # Compiler automatically creates a &mut Point token

# ref: shared ownership (compiler automatically chooses Rc/Arc)
shared = ref p2      # Share across scopes

# clone(): explicit deep copy
backup = p2.clone()

# unsafe + raw pointer: system-level
unsafe {
    ptr: *Point = &p2
    (*ptr).x = 0.0
}
```

**Ownership gradient**:
```
&T / &mut T    Move       ref        clone()    unsafe
    |             |          |           |          |
Borrow token   Default  Shared ownership  Deep copy  Raw pointer
Zero cost     Zero-copy  Auto Rc/Arc     Explicit   System-level
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

# Using match to handle the result
result = divide(10.0, 2.0)
match result {
    Result.ok(value) -> print(value),
    Result.err(msg) -> print("Error: ${msg}"),
}
```

### 4.6 Concurrent Programming (The Spawn Model)

```yaoxiang
# Mark an async function with spawn
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

## 5. Roadmap and Open Items

### 5.1 Decided Design Decisions

The following decisions have been fully discussed and reviewed and **are no longer open to change**:

| Module | Decision | Explanation |
|------|------|------|
| **Type system** | Everything is a type | Values, functions, modules, and generics are all types |
| **Type syntax** | Unified `name: type = value` | One declaration form covers all cases; `Type` is the only meta type keyword |
| **Keywords** | 17 core keywords | Excludes `type` / `fn` / `struct` / `enum` / `trait` / `impl` |
| **Function syntax** | Signature + expression | `name: (params) -> ReturnType = body` |
| **Method binding** | RFC-004 curried binding | `Type.method = function[position]` |
| **Async model** | The spawn model | Marked with `spawn`, lazy evaluation, automatic parallelism |
| **Memory management** | Ownership model (RFC-009 v9) | Move + `&T`/`&mut T` tokens + `ref` + `clone` + `unsafe`; no GC |
| **File as module** | Module system | Each `.yx` file is a module |
| **Main function** | `main: () -> Void` | Program entry point |
| **Thread safety** | `ref` automatically chooses Rc/Arc | Compiler-driven escape analysis, transparent to the user |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        YaoXiang Implementation Roadmap (Example)             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: Rust AOT │
│        ✅ Done                          │ (current stage)        Compiler      │
│                                          │                                      │
│                                          ▼                                      │
│  v0.6: YaoXiang Interpreter ←─────── v1.0: YaoXiang JIT Compiler ←──── v2.0: │
│        (self-hosted)                     (self-hosted)              YaoXiang AOT │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 6. How to Contribute

YaoXiang is a language born in the community, growing in the community, and serving the community. We sincerely invite every developer who loves programming language design to join this exploration.

### 6.1 Design Discussion

**Suitable for**: programming-language theory researchers, type-system enthusiasts, and language-design fanatics

**How to participate**:

- **GitHub Discussions**: join the discussions under the "Language Design" category
- **Design Proposals (RFCs)**: submit design documents for new features, following the template in the `rfcs/` directory
- **Syntax review**: propose improvements to existing syntax designs or identify potential issues

| **Current hot topics**: |
| |
| - Macro system design and implementation |
| - Interface type mechanism |
| - Error-handling syntax optimization |
| - Standard library API design |

**Submitting a design proposal**:

1. Create a new file under the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros and cons analysis, alternatives)
3. Open a Pull Request for community review
4. After review by the core team, the proposal is either merged or rejected

### 6.2 Compiler Implementation

**Suitable for**: compiler developers, systems programmers, and performance-optimization experts

**Current implementation focus** (sorted by priority):

| Priority | Module | Description | Difficulty |
|--------|------|------|------|
| P0 | **Bytecode virtual machine** | VM instruction refinement, performance optimization | Medium |
| P0 | **Runtime memory** | GC implementation, memory allocator | High |
| P0 | **Async runtime** | Complete implementation of the spawn model | High |
| P1 | Standard library | IO, String, List, Concurrent | Medium |
| P1 | JIT compiler | Cranelift integration | High |
| P2 | AOT compiler | LLVM / Cranelift backend | High |
| P3 | Self-hosting compiler | Rewrite in YaoXiang | Very high |

**Tech stack**:

- **Implementation language**: Rust (current stage)
- **Code generation**: Cranelift or LLVM
- **Build tool**: Cargo
- **Test framework**: Rust `#[test]` + `cargo nextest`

**Start contributing**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand the architecture
2. Choose a module of interest under the `src/` directory
3. Browse `tests/unit/` to understand the test requirements
4. Make sure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, and efficiency-tool seekers

**Tools to be developed**:

| Tool | Status | Description |
|------|------|------|
| **LSP server** | ⏳ To be started | Language Server Protocol support |
| **Debugger integration** | ⏳ To be started | GDB / LLDB integration |
| **Formatter** | ⏳ To be started | `yaoxiang fmt` |
| **Package manager** | ⏳ To be started | Dependency management, version resolution |
| **Package registry** | ⏳ To be started | Centralized or decentralized registry |
| **REPL** | ⏳ To be started | Interactive interpreter |
| **Benchmarking tool** | ⏳ To be started | Performance analysis |
| **VS Code plugin** | ⏳ To be started | Syntax highlighting, completion, debugging |
| **Vim / Neovim plugin** | ⏳ To be started | Syntax highlighting, LSP client |

**Project structure reference**:

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
│   └── vim/                      # Vim / Neovim
```

### 6.4 Standard Library Development

**Suitable for**: library developers, API designers, and domain experts

**Standard library module plan**:

| Module | Priority | Description |
|------|--------|------|
| `std.io` | P0 | File IO, console input/output |
| `std.string` | P0 | String manipulation, formatting |
| `std.list` | P0 | List / array operations |
| `std.dict` | P0 | Dictionary / hash table |
| `std.math` | P0 | Math functions, constants |
| `std.time` | P1 | Time and date operations |
| `std.net` | P1 | Network programming, HTTP |
| `std.concurrent` | P1 | Concurrency primitives, channels |
| `std.crypto` | P2 | Cryptographic hashing, signatures |
| `std.json` | P1 | JSON parsing / generation |
| `std.regex` | P2 | Regular expressions |
| `std.database` | P3 | Database connections |
| `std.gui` | P3 | Graphical interface (long-term) |

**Design principles**:

- **Consistency**: function names and behaviors for the same functionality remain consistent
- **Simplicity**: APIs should be intuitive and easy to use, avoiding over-design
- **Performance**: standard-library functions should be efficient, avoiding unnecessary copies
- **Testability**: every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: technical writers, educators, and community managers

**Documentation to be contributed**:

| Documentation | Status | Description |
|------|------|------|
| Quick start | ✅ Done | A 5-minute getting-started guide |
| Language guide | ✅ Done | Systematic study of core concepts |
| Language specification | ✅ Done | Complete syntax and semantic definition |
| Implementation plan | ✅ Done | Compiler-implementation technical details |
| API documentation | ⏳ To be started | Standard library API reference |
| Tutorials | ⏳ To be started | Advanced tutorials and best practices |
| Blog | ⏳ To be started | Technical articles and design stories |
| Translations | ⏳ To be started | Multilingual support |

### 6.6 Community Building

**Suitable for**: community managers, event organizers, and evangelists

**Community activities**:

- Regular online Meetups (once a month)
- Design and implementation discussions (once a week)
- Code-contribution Sprints (once a quarter)
- Offline gatherings and conference talks

**Communication channels**:

- GitHub Discussions: technical discussions
- GitHub Issues: bug reports and feature requests
- Discord / Slack: real-time communication
- Twitter / X: project updates
- Blog: in-depth articles

### 6.7 Contribution Guidelines

**How to start contributing**:

1. **Understand the project**: read the README and design documents
2. **Choose a direction**: pick a contribution area based on your interests
3. **Set up the environment**: Rust 1.75+, cargo, git
4. **Find a task**: look at GitHub Issues tagged `good first issue`
5. **Submit a PR**: follow the commit conventions and write tests
6. **Participate in review**: review others' code and join the discussions

**Commit conventions**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: new feature
fix:  bug fix
docs: documentation update
style: code formatting (no functional change)
refactor: refactor
perf: performance optimization
test: test
chore: build tools or auxiliary tools

# Examples
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**Code style**:

- Follow the `rustfmt.toml` specification
- Ensure `cargo clippy` produces no warnings
- Write necessary unit tests
- Update related documentation

---

## Appendix A: Language Cheat Sheet

### A.1 Keywords

| Keyword | Purpose |
|--------|------|
| `pub` | Public export |
| `use` | Import module |
| `spawn` | Spawn marker |
| `ref` | Shared ownership (compiler automatically chooses Rc/Arc) |
| `mut` | Mutable variable |
| `if` / `elif` / `else` | Conditional branch |
| `match` | Pattern matching |
| `while` / `for` | Loops |
| `return` / `break` / `continue` | Control flow |
| `as` | Type conversion |
| `in` | Membership test / list comprehension |
| `unsafe` | `unsafe` code block (raw pointer) |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type` keyword was removed in RFC-010, unifying the `name: Type = value` syntax.

### A.3 Primitive Types

| Type | Description | Default Size |
|------|------|----------|
| `Void` | Void value | 0 bytes |
| `Bool` | Boolean value | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating-point number | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | Raw bytes | Variable |

### A.4 Operator Precedence

| Precedence | Operators | Associativity |
|--------|--------|--------|
| 1 | `()` `[]` `.` | Left to right |
| 2 | `as` | Left to right |
| 3 | `*` `/` `%` | Left to right |
| 4 | `+` `-` | Left to right |
| 5 | `<<` `>>` | Left to right |
| 6 | `&` `\|` `^` | Left to right |
| 7 | `==` `!=` `<` `>` `<=` `>=` | Left to right |
| 8 | `not` | Right to left |
| 9 | `and` `or` | Left to right |
| 10 | `if...else` | Right to left |
| 11 | `=` `+=` `-=` `*=` `/=` | Right to left |

---

## Appendix B: Design Inspirations

YaoXiang's design draws on the excellent ideas of the following languages and projects:

| Source | Inspiration |
|------|--------|
| **Rust** | Ownership model, zero-cost abstractions, type system |
| **Python** | Syntax style, readability, list comprehensions |
| **Idris / Agda** | Dependent types, type-driven development |
| **Curry-Howard correspondence** | Types as propositions, programs as proofs; the unified theory of type systems and logic |
| **TypeScript** | Type annotations, runtime types |
| **MoonBit** | AI-friendly design, concise syntax |
| **Haskell** | Pure functional programming, pattern matching |
| **OCaml** | Type inference, variant types |

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions, but adopts simpler syntax and a lower cognitive load. **The spawn model** is more concise than Rust's `async/await` — only one `spawn` keyword is needed, with no need to manually manage `Future` and `Pin`. "The myriad things spawn together; I observe their return," making concurrent programming as intuitive as describing a natural law. **The ownership model** (RFC-009 v9) uses Move + `&T` / `&mut T` tokens to replace lifetime annotations, and uses type attributes (Dup / Linear) to replace the borrow checker. The unified type syntax eliminates the conceptual fragmentation of `enum` / `struct` / `trait` / `impl`.

**Q: What kinds of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, and AI-assisted programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks and reduce confusion caused by deep nesting. This is a deliberately chosen "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: The v1.0 goal is production readiness. The release date depends on implementation progress; see the [Version Planning RFC](./rfc/003-version-planning.md) for details.

**Q: How do I contact the core team?**

A: Through GitHub Discussions or the Discord community channel. Core team members reply regularly.

---

> **Last updated**: 2026-05-31
>
> **Document version**: v2.0.0
>
> **License**: [MIT](LICENSE)

---

> "Yao-Xiang transform; the myriad things arise. Types evolve; programs come to be."
>
> May the design journey of YaoXiang walk alongside you.