# YaoXiang Design Manifesto

> **Version**: v1.2.0
> **Status**: Official Release
> **Author**: Chen Xu + YaoXiang Community
> **Date**: 2025-01-17

---

> "The Tao gives birth to the One, the One gives birth to Two, Two gives birth to Three, and Three gives birth to all things."
> — *Tao Te Ching*
>
> Types are like the Tao, from which all things are born.

---

## I. Why YaoXiang Was Created

### 1.1 A Gap in the Language Landscape

In the long history of programming languages, we have witnessed the birth and evolution of countless excellent languages: C brought efficiency revolution to systems programming, Python created a programming experience accessible to everyone, Rust proved that memory safety and performance can coexist, and TypeScript made large-scale frontend projects maintainable. However, when we survey today's language ecosystem, we still find a clear gap—**no single language can simultaneously satisfy these three core needs**:

| Need | Problems with Existing Solutions |
|------|----------------------------------|
| **Type Safety** | Rust is overly strict with a steep learning curve; TypeScript has optional types and cannot provide compile-time guarantees |
| **Natural Syntax** | Rust's syntax is complex and obscure; Haskell's functional paradigm has too high a barrier; traditional static languages are verbose and cumbersome |
| **AI-Friendly** | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behaviors, limiting AI accuracy in code generation and modification |

YaoXiang's creation is precisely to fill this gap. We believe: **Programming languages should be both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Practical Problems We Solve

**Problem 1: Type System Fragmentation**

Today's programming languages suffer from severe type system fragmentation. Static typing pursues absolute compile-time correctness but often at the cost of developer productivity; dynamic typing provides flexibility yet reveals maintainability flaws in large projects. YaoXiang proposes a unified abstraction framework of "Everything is a type," making types the thread that runs through the language design, not an afterthought patch.

**Problem 2: The False Choice Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime performance. Garbage collection (GC) frees developers but introduces latency jitter and memory overhead; manual memory management is efficient but as dangerous as walking a tightrope. YaoXiang adopts Rust's ownership model, eliminating data races and memory leaks at compile time while maintaining zero-cost abstractions, achieving high performance without a GC.

**Problem 3: Cognitive Overhead in Asynchronous Programming**

Modern applications are inseparable from networking and concurrency, yet asynchronous programming has always been a nightmare for programmers. Nested callback functions, Promise chains, async/await syntax—each approach increases code complexity. YaoXiang has redesigned the async model: simply add a `spawn` marker after the function signature, and the compiler automatically handles all async details, making concurrent programming as natural as writing synchronous code.

**Problem 4: Bottlenecks in AI-Assisted Programming**

When AI begins to assist developers in writing code, language design choices become crucial. Ambiguous syntax rules, implicit type conversions, and complex syntactic sugar—features that human programmers have grown accustomed to—become obstacles for AI to understand and generate. YaoXiang set "AI-friendly" as a core goal from the very beginning: strict indentation rules, clear code block boundaries, and unambiguous syntax structures enable AI to accurately understand, generate, and modify code.

### 1.3 The Philosophical Roots of the Language

The name YaoXiang comes from "Yao" (爻) and "Xiang" (象) in the *I Ching* (Book of Changes). "Yao" are the fundamental symbols that compose hexagrams, symbolizing the interplay of yin and yang, motion and stillness; "Xiang" is the external manifestation of a thing's essence, representing the myriad phenomena and all-encompassing scope.

This philosophical thinking is reflected in every detail of the language design:

- **Unity**: Just as simple Yao symbols form complex hexagrams, YaoXiang constructs a complete programming model from a few core concepts (types, functions, constructors)
- **Hierarchy**: Just as Xiang has distinctions between prior and later heavens, YaoXiang's type system has a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Changeability**: Just as yin and yang flow and transform endlessly, YaoXiang supports dependent types, allowing types to evolve with values
- **Identifiability**: Just as hexagrams can be interpreted and all things can be represented, YaoXiang provides complete type reflection capabilities, with full runtime type information available
- **Provability**: Just as hexagrams reveal the laws of things, YaoXiang's type system follows the Curry-Howard isomorphism (types as propositions, programs as proofs), where type checking is the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Every feature proposal must pass检验 through these principles.

### 2.1 Principle 1: Everything is a Type

In YaoXiang's worldview, type is the highest-level abstraction unit, the core concept that runs through the entire language.

**Concrete manifestations**:

- **Values are instances of types**: `42` is an instance of type `Int`, `"hello"` is an instance of type `String`
- **Types themselves are also types**: The meta type (`type`) is the type of all types, `Int` is an instance of `type`
- **Functions are type mappings**: `fn add(Int, Int) -> Int` describes a type mapping from `Int × Int` to `Int`
- **Modules are type compositions**: Modules are named combinations of namespaces containing functions and types

**Non-negotiable rationale**: Unified type abstraction simplifies language semantics, eliminates the dualism between values and types, and makes the type system the guardian of code correctness rather than a stumbling block.

### 2.2 Principle 2: Strictly Structured

YaoXiang's syntax design pursues "unambiguous, predictable, and easy to parse."

**Specific rules**:

- **Mandatory 4-space indentation**: Tab characters are prohibited; code block boundaries are crystal clear
- **Parentheses cannot be omitted**: Function arguments must have parentheses, list elements must have commas
- **Code blocks must have curly braces**: Control flow like `if`, `while`, `for` must use `{ }` to enclose blocks
- **Minimal keyword count**: Only 17 core keywords, rejecting syntactic sugar proliferation

**Non-negotiable rationale**: Strict structure brings three key advantages—(1) more accurate IDE syntax highlighting and code folding; (2) significantly improved AI code generation and modification accuracy; (3) new learners can quickly understand code structure.

### 2.3 Principle 3: Zero-Cost Abstraction

High-level abstractions should not incur runtime performance overhead.

**Concrete guarantees**:

- **Monomorphization**: Generic functions are expanded into concrete versions at compile time, with no virtual table lookup overhead
- **Inlining optimization**: Simple functions are automatically inlined, eliminating function call overhead
- **Stack allocation by default**: Small objects are allocated on the stack by default; heap allocation is used only when necessary
- **No GC**: The ownership model ensures memory safety without garbage collector runtime overhead

**Non-negotiable rationale**: Performance is the survival bottom line of programming languages. Any design that sacrifices performance for convenience is a betrayal of programmers.

### 2.4 Principle 4: Immutable by Default

Mutability and complexity are inseparable companions. YaoXiang chooses immutable by default, making code easier to reason about and understand.

**Specific rules**:

- Variables are immutable by default and cannot be modified after assignment
- Mutability must be explicitly declared with `mut` when needed
- References are immutable by default; mutable references require `mut` marking
- Transfer of ownership means the original binding becomes invalid

**Non-negotiable rationale**: Immutability is the foundation of concurrency safety, the guarantee of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle 5: Types as Data

Type information should not exist only at compile time; it should be fully available at runtime.

**Specific capabilities**:

- Runtime type querying: any value can obtain its type information
- Type reflection: can construct and manipulate types themselves
- Pattern matching destructuring: type constructors can be directly used in pattern matching
- Generic specialization: can obtain instantiated types of generic parameters at runtime

**Non-negotiable rationale**: Complete type reflection capability is the foundation of metaprogramming, the cornerstone of high-performance frameworks and tools.

---

## III. Key Innovations and Features

While absorbing excellent features from existing languages, YaoXiang proposes the following innovative designs.

### 3.1 Innovation 1: Unified Type Syntax

**Traditional language type definitions** often require multiple keywords:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
union IntOrFloat { i: i32, f: f32 }
```

**YaoXiang's unified syntax**: only the `type` keyword, `{}` for data structures, `[]` for interface types.

```yaoxiang
# === Data Types (curly braces) ===

# Struct
type Point = { x: Float, y: Float }

# Enum (multiple constructors)
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Zero-argument constructors
type Color = { red | green | blue }

# Hybrid types
type Shape = { circle(Float) | rect(Float, Float) }

# === Interface Types (square brackets) ===

# Interface definition: set of method signatures
type Serializable = [
    serialize() -> String
]

type Drawable = [
    draw(Surface) -> Void,
    bounding_box() -> Rect
]

# === Generics ===

Option: (T: Type) -> Type = { some(T) | none }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }
```

**Innovation value**: Unified type syntax, eliminating `enum`/`struct`/`union`/`trait` keyword fragmentation.

### 3.2 Innovation 2: Constructors are Types

**Value construction and pattern matching are exactly the same**:

```yaoxiang
# Type definition
type Point = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Value construction: same as function calls
p: Point = Point(3.0, 4.0)
r: Result[Int, String] = ok(42)
err_msg: Result[Int, String] = err("not found")
c: Color = green

# Pattern matching: direct destructuring
match point {
    Point(0.0, 0.0) -> "origin"
    Point(x, y) -> "point at ({x}, {y})"
}
```

### 3.3 Innovation 3: Curried Method Binding

YaoXiang adopts a pure functional design, implementing object-method-call-like syntax through currying, without introducing `class` and `method` keywords.

```yaoxiang
# === Point.yx ===

# Type definition
type Point = { x: Float, y: Float }

# Core function: Euclidean distance
distance(Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# Method syntax sugar binding
Point.distance = distance  # Default binding to position 0
```

```yaoxiang
# === main.yx ===

use Point

main() -> Void = () => {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # Two calling styles are completely equivalent
    d1 = distance(p1, p2)      # Direct core function call
    d2 = p1.distance(p2)       # Method syntax sugar

    # Functional usage: pre-binding the first argument
    dist_from_origin = Point.distance(Point(0.0, 0.0))
    result = dist_from_origin(p1)   # 5.0

    # Currying usage: delayed evaluation
    get_dist_to_p2 = p2.distance(_)
    d3 = get_dist_to_p2(p1)         # 2.828
}
```

**Innovation value**: Pure functional design, no hidden `self` parameter, functions as values can be freely passed and composed.

### 3.4 Innovation 4: Seam (spawn) Model

> "All things arise together, and I observe their return." — *I Ching, Hexagram 24 (Fu/Return)*
>
> The Seam model draws its meaning from this, describing a programming paradigm: the developer describes logic in a synchronous, sequential manner, while the language runtime causes its computational units to automatically and efficiently execute concurrently like all things arising together, ultimately converging in unified coordination.

**Three Core Principles**:

| Principle | Description |
|-----------|-------------|
| **Synchronous Syntax** | What you see is sequential code |
| **Concurrent Essence** | Runtime automatically extracts parallelism |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology System**:

| Official Term | Corresponding Syntax | Explanation |
|---------------|---------------------|-------------|
| **spawn function** | `spawn (params) => body` | Defines a computation unit that can participate in seam execution |
| **spawn block** | `spawn { a(), b() }` | Explicitly declared concurrency domain, tasks within the block execute concurrently |
| **spawn loop** | `spawn for x in xs { ... }` | Data parallelism, loop body executes concurrently on all elements |
| **spawn value** | `Async(T)` | A future value currently in seam execution, automatically awaited on use |
| **spawn graph** | Lazy evaluation graph (DAG) | The stage where seam occurs, describing dependencies and parallelism |
| **spawn scheduler** | Runtime task scheduler | The intelligent coordinator that orchestrates all things, letting them seam at the right moments |

> **See also**: [RFC-001 Seam Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === spawn function ===
# Function marked with spawn (RFC-003 syntax)
fetch_data: String -> JSON = spawn (url) => {
    HTTP.get(url).json()
}

# === spawn block ===
# Expressions inside spawn { } execute mandatorily in parallel
compute_all: () -> (Int, Int, Int) = spawn () => {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    (a, b, c)
}

# === spawn loop ===
# Loops marked with spawn for auto-parallelize
parallel_sum: Int -> Int = spawn (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)  # Each iteration executes in parallel
    }
    total
}

# === Automatic await ===
main() -> Void = () => {
    # Two independent requests execute in parallel automatically
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # Await points are automatically inserted where results are needed
    print(users.length + posts.length)  # Auto-await users and posts
}
```

**Thread Safety**:

```yaoxiang
# Send/Sync constraints ensure compile-time thread safety
type SafeCounter = SafeCounter(mutex: Mutex(Int))

main: () -> Void = () => {
    counter: Arc[SafeCounter] = Arc.new(SafeCounter(Mutex.new(0)))

    # spawn automatically checks Send constraints
    spawn () => {
        guard = counter.mutex.lock()
        guard.value = guard.value + 1
    }
}
```

**Technical Documentation**:
- See [RFC-001 Seam Model](./rfc/001-concurrent-model-error-handling.md)

**Innovation value**: The cognitive overhead of asynchronous programming drops to zero, code readability is completely identical to synchronous code, while achieving high-performance parallel execution efficiency.

### 3.5 Innovation 5: Dependent Type Support (Future Feature)

> **Status**: Consider implementing after v1.0

Types can depend on values, enabling true type-driven development.

```yaoxiang
# Fixed-length vector (future syntax)
Vector: (T: Type, n: Int) -> Type = {
    data: [T; n]
    length: n
}

# Usage
vec: Vector[Int, 3] = Vector([1, 2, 3], 3)

# Type checking
# vec: Vector[Int, 3] = Vector([1, 2], 2)  # Compile error! Length mismatch
```

**Innovation value**: Capture more errors at compile time, enabling more precise type guarantees.

### 3.6 Innovation 6: Minimalist Keyword Design

YaoXiang defines only 18 core keywords, far fewer than mainstream languages:

```
type   pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

| Language Compared | Keyword Count |
|--------------------|---------------|
| YaoXiang | **18** |
| Rust | 51+ |
| Python | 35 |
| TypeScript | 64+ |
| Go | 25 |

**Innovation value**: Lower memory burden, more consistent syntax style, easier-to-parse syntax structure.

---

## IV. Preliminary Syntax Preview

The following code examples showcase YaoXiang's language style, helping you quickly experience its design aesthetics.

### 4.1 Hello World

```yaoxiang
# hello.yx
use std.io

main() -> Void = () => {
    println("Hello, YaoXiang!")
}
```

### 4.2 Type Definitions and Functions

```yaoxiang
# Unified type syntax
type Point = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }
type Color = { red | green | blue }

# Interface type
type Serializable = [ serialize() -> String ]

# Function definition
add: (Int, Int) -> Int = (a, b) => a + b

# Generic function
identity: (T: Type) -> ((x: T) -> T) = (x) => x

# Multi-line function
fact: Int -> Int = (n) => {
    if n == 0 { 1 } else { n * fact(n - 1) }
}
```

### 4.3 Pattern Matching

```yaoxiang
# Powerful pattern matching
classify(Int) -> String = (n) => {
    match n {
        0 -> "zero"
        1 -> "one"
        _ if n < 0 -> "negative"
        _ -> "positive"
    }
}

# Destructuring patterns
type Point = { x: Float, y: Float }
match point {
    Point(0.0, 0.0) -> "origin"
    Point(x, y) -> "point at ({x}, {y})"
}
```

### 4.4 List Comprehensions

```yaoxiang
# Python-style list comprehensions
evens = [x * 2 for x in 0..10]          # [0, 4, 8, 12, 16]
squares = [x * x for x in 1..10 if x % 2 == 1]  # [1, 9, 25, 49, 81]

# Nested comprehensions
matrix = [[i * j for j in 1..4] for i in 1..3]
# [[1, 2, 3], [2, 4, 6], [3, 6, 9]]
```

### 4.5 Ownership Model

```yaoxiang
type Point = { x: Float, y: Float }

# Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p              # Move, ownership transfer, p invalidated

# Explicit ref = Arc (safe sharing)
shared = ref p      # Arc, thread-safe

spawn(() => print(shared.x))   # ✅ Safe

# Explicit clone() = copy
p3 = p.clone()      # p and p3 are independent
```

**Core rules**:

- Default Move (zero-copy)
- Sharing uses `ref` (Arc)
- Copies use `clone()`
- System-level uses `unsafe` + `*T`

### 4.6 Error Handling

```yaoxiang
# Result type
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

divide: (Float, Float) -> Result[Float, String] = (a, b) => {
    if b == 0.0 {
        err("Division by zero")
    } else {
        ok(a / b)
    }
}

# Using the ? operator
process: () -> Result[Int, String] = () => {
    a = read_number()?
    b = read_number()?
    c = divide(a, b)?
    ok(c * 2)
}
```

### 4.7 Concurrent Programming (SeamlessAsync)

```yaoxiang
# spawn marks async functions
fetch_api: String -> JSON spawn = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

# Concurrent construction block: explicit parallelism
process_all: () -> (JSON, JSON, JSON) spawn = () => {
    (a, b, c) = spawn {
        fetch_api("https://api1.com/data"),
        fetch_api("https://api2.com/data"),
        fetch_api("https://api3.com/data")
    }
    (a, b, c)
}

# Data-parallel loop
parallel_process: Int -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        compute(i)
    }
    total
}

# Thread safety example
type ThreadSafeCounter = { value: Mutex(Int) }

main: () -> Void = () => {
    counter = ThreadSafeCounter(Mutex.new(0))

    # spawn automatically checks Send constraints
    spawn () => {
        guard = counter.value.lock()
        guard.value = guard.value + 1
    }
}
```

---

## V. Roadmap and Open Items

### 5.1 Decided Design Decisions

The following decisions have been thoroughly discussed and reviewed, **and will not accept changes**:

| Module | Decision | Description |
|--------|----------|-------------|
| **Type System** | Everything is a Type | Values, functions, modules, and generics are all types |
| **Type Syntax** | Unified curly brace syntax | `{}` for data types, `[]` for interface types |
| **Indentation Rules** | 4-space indentation | Mandatory requirement, tabs prohibited |
| **Keywords** | 17 core keywords | As listed in the table above |
| **Function Syntax** | Arrow function syntax | `name: (Type1, Type2) -> Type = (params) => body` |
| **Method Binding** | RFC-004 Curried Binding | `Type.method = function` (default binding to position 0) |
| **Async Model** | RFC-003 Seam Model | `spawn (params) => body`, lazy evaluation, auto-parallelism |
| **Memory Management** | Ownership Model | No GC, compile-time safety guarantees |
| **File as Module** | Module System | Every `.yx` file is a module |
| **Main Function** | `main: () -> Void` | Program entry point |
| **Thread Safety** | Send/Sync Constraints | Compile-time data race elimination |

### 5.2 Design Topics Under Discussion

The following topics are still under discussion, **community contributions are welcome**:

| Topic | Current Status | Open Questions |
|-------|----------------|----------------|
| **Literal Syntax** | Float support | Support `3.14e-10` scientific notation? |
| **Generic Inference** | Basic support | Return type generic inference? |
| **Macro System** | Not yet designed | Need hygiene macros? Syntax design direction? |
| **Package Manager** | Not yet designed | Need centralized package registry? Dependency resolution strategy? |
| **FFI** | Not yet designed | Specific plan for C interop? |
| **Generic Constraints** | Basic support | Support trait/bounds mechanism? |
| **Reflection Depth** | Basic support | Support accessing private members? |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           YaoXiang Implementation Roadmap                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: AOT  │
│        ✅ Completed                   │ (current stage)            Compiler   │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←────── v1.0: YaoXiang JIT ←──────── v2.0:     │
│        (bootstrap)                   (bootstrap)                  YaoXiang   │
│                                                                              AOT     │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Milestone Details**:

| Version | Status | Goal | Deliverables |
|---------|--------|------|--------------|
| **v0.1** | ✅ Complete | Interpreter prototype | Basic interpreter, lexer, parser, primitive types |
| **v0.2** | ✅ Complete | Full interpreter | Type checking, pattern matching, module system |
| **v0.3** | 🔄 In Progress | Bytecode generation | IR intermediate representation, bytecode generation, closure optimization, monomorphization |
| **v0.4** | 🔄 In Progress | Bytecode VM | VM core, instruction execution, call frame management, inline caching |
| **v0.5** | ⏳ Not Started | Runtime system | GC, scheduler, stdlib IO |
| **v1.0** | ⏳ Not Started | AOT Compiler | Full optimization, native code generation |
| **v2.0** | ⏳ Not Started | Bootstrap compiler | New compiler written in YaoXiang |

### 5.4 Current Implementation Status

| Module | Status | Completion | Description |
|--------|--------|------------|-------------|
| **Lexer** | ✅ Complete | 100% | Token definition, keyword recognition, test cases |
| **Parser** | ✅ Complete | 100% | AST definition, expression/statement parsing, boundary tests |
| **Type Checker** | ✅ Complete | 95% | Type inference, monomorphization, generic specialization, error handling |
| **IR Intermediate Representation** | ✅ Complete | 90% | IR instruction definition, type representation, control flow graph |
| **Bytecode Generation** | ✅ Complete | 85% | Expression/statement/control flow bytecode, closure conversion |
| **Ownership System** | ✅ Complete | 100% | Move semantics, Clone/Drop semantics, mutability checking, Send/Sync constraints |
| **Monomorphization** | ✅ Complete | 100% | Generic instantiation, specialization implementation |
| **Escape Analysis** | 🔄 In Progress | 40% | Basic framework, variable escape determination |
| **Bytecode VM** | 🔄 In Progress | 70% | VM core, instruction execution, call frames, inline caching |
| **Runtime Scheduler** | 🔄 In Progress | 60% | Task descriptors, work-stealing queues, wait queues |
| **Runtime Memory** | 🔄 In Progress | 50% | Memory allocator, GC framework |
| **Standard Library** | 🔄 In Progress | 30% | IO, String, List, Dict, Math, Concurrent |
| **JIT Compiler** | ⏳ Not Started | 0% | To integrate Cranelift/LLVM |
| **AOT Compiler** | ⏳ Not Started | 0% | To be implemented |

**Code Generation Module Details**:

| Sub-module | Status | Key Features |
|------------|--------|--------------|
| Expression Generation | ✅ Complete | Arithmetic, comparison, logic, function calls |
| Statement Generation | ✅ Complete | Assignment, return, conditionals, loops |
| Control Flow Generation | ✅ Complete | Switch pattern matching, loop unrolling |
| Closure Handling | ✅ Complete | Environment capture, closure conversion |
| Bytecode Serialization | ✅ Complete | Bytecode read/write, test cases |
| Generator Code Generation | ✅ Complete | yield syntax support, state machine conversion |
| Integration Tests | ✅ Complete | End-to-end compilation execution tests |

**Async Implementation Status (Seam Model)**:

| Sub-module | Status | Description |
|------------|--------|-------------|
| spawn keyword parsing | ✅ Complete | Lexer/syntax analysis support |
| is_async flag | ✅ Complete | AST/type system support |
| Async(T) type design | ✅ Complete | Design documentation complete |
| Scheduler framework | ✅ Complete | Basic work-stealing implementation |
| Send/Sync constraints | ✅ Complete | Type constraint design documentation |
| IR extension | 🔄 In Progress | CallAsync instruction defined |
| VM async instructions | 🔄 In Progress | Instruction framework defined |
| Full implementation | ⏳ Not Started | v0.5 milestone |

---

## VI. How to Contribute

YaoXiang is a language born from the community, growing in the community, and serving the community. We sincerely invite every developer who loves programming language design to join this journey of exploration.

### 6.1 Design Discussion

**Suitable for**: Programming language theory researchers, type system enthusiasts, language design fanatics

**How to participate**:

- **GitHub Discussions**: Participate in discussions under the "Language Design" category
- **Design Proposals (RFC)**: Propose design documents for new features, following the template under `rfcs/`
- **Syntax Review**: Propose improvements or discover potential issues in existing syntax design

| **Current Hot Topics**: |
| |
| - Macro system design and implementation |
| - Interface type mechanism |
| - Error handling syntax optimization |
| - Standard library API design |

**Submitting a design proposal**:

1. Create a new file under `rfcs/`
2. Fill in the RFC template (motivation, detailed design, pros/cons analysis, alternatives)
3. Open a Pull Request for community review
4. Merged or rejected after core team deliberation

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, systems programmers, performance optimization experts

**Current implementation priorities** (in order of priority):

| Priority | Module | Description | Difficulty |
|----------|--------|-------------|------------|
| P0 | **Bytecode VM** | VM instruction completion, performance optimization | Medium |
| P0 | **Runtime Memory** | GC implementation, memory allocator | High |
| P0 | **Async Runtime** | Complete seam model implementation | High |
| P1 | Standard Library | IO, String, List, Concurrent | Medium |
| P1 | JIT Compiler | Cranelift integration | High |
| P2 | AOT Compiler | LLVM/Cranelift backend | High |
| P3 | Bootstrap Compiler | Rewrite in YaoXiang | Extremely High |

**Tech Stack**:

- **Implementation Language**: Rust (current stage)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Testing Framework**: Rust `#[test]` + `cargo nextest`

**Getting started**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand the architecture
2. Choose a module under `src/` that interests you
3. Read `tests/unit/` to understand testing requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, productivity tool pursuers

**Tools to develop**:

| Tool | Status | Description |
|------|--------|-------------|
| **LSP Server** | ⏳ Not Started | Language Server Protocol support |
| **Debugger Integration** | ⏳ Not Started | GDB/LLDB integration |
| **Formatter** | ⏳ Not Started | `yaoxiang fmt` |
| **Package Manager** | ⏳ Not Started | Dependency management, version resolution |
| **Package Registry** | ⏳ Not Started | Centralized or decentralized |
| **REPL** | ⏳ Not Started | Interactive interpreter |
| **Benchmarking Tool** | ⏳ Not Started | Performance analysis |
| **VS Code Extension** | ⏳ Not Started | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin** | ⏳ Not Started | Syntax highlighting, LSP client |

**Project structure reference**:

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

**Standard library module planning**:

| Module | Priority | Description |
|--------|----------|-------------|
| `std.io` | P0 | File IO, console input/output |
| `std.string` | P0 | String operations, formatting |
| `std.list` | P0 | List/array operations |
| `std.dict` | P0 | Dictionary/hashmap |
| `std.math` | P0 | Math functions, constants |
| `std.time` | P1 | Date/time operations |
| `std.net` | P1 | Networking, HTTP |
| `std.concurrent` | P1 | Concurrency primitives, channels |
| `std.crypto` | P2 | Cryptographic hashing, signatures |
| `std.json` | P1 | JSON parsing/generation |
| `std.regex` | P2 | Regular expressions |
| `std.database` | P3 | Database connectivity |
| `std.gui` | P3 | Graphical UI (long-term) |

**Design principles**:

- Consistency: Functions with the same behavior maintain consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoiding over-engineering
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documentation to contribute**:

| Document | Status | Description |
|----------|--------|-------------|
| Quick Start | ✅ Complete | 5-minute getting started guide |
| Language Guide | ✅ Complete | Systematic learning of core concepts |
| Language Specification | ✅ Complete | Complete syntax and semantics definition |
| Implementation Plan | ✅ Complete | Compiler implementation technical details |
| API Documentation | ⏳ Not Started | Standard library API reference |
| Tutorials | ⏳ Not Started | Advanced tutorials and best practices |
| Blog | ⏳ Not Started | Technical articles and design stories |
| Translations | ⏳ Not Started | Multi-language support |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

**Community activities**:

- Regular online Meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution Sprints (quarterly)
- In-person gatherings and conference talks

**Channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Bug reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guidelines

**How to start contributing**:

1. **Understand the project**: Read the README and design documents
2. **Choose your direction**: Select a contribution area based on your interests
3. **Set up your environment**: Rust 1.75+, cargo, git
4. **Find tasks**: Look at GitHub Issues for `good first issue` labels
5. **Submit PRs**: Follow commit conventions, write tests
6. **Participate in reviews**: Review others' code, join discussions

**Commit conventions**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: New feature
fix: Bug fix
docs: Documentation update
style: Code formatting (no functional impact)
refactor: Refactoring
perf: Performance improvement
test: Testing
chore: Build tools or auxiliary tools

# Examples
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**Code style**:

- Follow `rustfmt.toml` conventions
- Ensure `cargo clippy` passes with no warnings
- Write necessary unit tests
- Update relevant documentation

---

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword | Purpose |
|---------|---------|
| `type` | Type definition |
| `pub` | Public export |
| `use` | Import module |
| `spawn` | spawn marker (RFC-003: `spawn (params) => body`) |
| `ref` | Shared pointer (Arc) |
| `mut` | Mutable variable |
| `if/elif/else` | Conditional branching |
| `match` | Pattern matching |
| `while/for` | Loops |
| `return/break/continue` | Control flow |
| `as` | Type casting |
| `in` | Membership test/list comprehension |
| `unsafe` | unsafe code block (raw pointers) |

### A.2 Attributes

| Attribute | Purpose |
|-----------|---------|
| `@block` | Marks the following function to execute synchronously |
| `@eager` | Marks expressions that need eager evaluation |
| `@Send` | Explicitly declares satisfying Send constraint |
| `@Sync` | Explicitly declares satisfying Sync constraint |

### A.3 Primitive Types

| Type | Description | Default Size |
|------|-------------|--------------|
| `Void` | void | 0 bytes |
| `Bool` | Boolean | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating-point | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | Raw bytes | Variable |

### A.4 Operator Precedence

| Precedence | Operators | Associativity |
|-------------|-----------|---------------|
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

YaoXiang's design draws from the excellent ideas of the following languages and projects:

| Source | Borrowed Aspects |
|--------|-----------------|
| **Rust** | Ownership model, zero-cost abstractions, type system |
| **Python** | Syntax style, readability, list comprehensions |
| **Idris/Agda** | Dependent types, type-driven development |
| **Curry-Howard Isomorphism** | Types as propositions, programs as proofs, unified theory of types and logic |
| **TypeScript** | Type annotations, runtime types |
| **MoonBit** | AI-friendly design, concise syntax |
| **Haskell** | Pure functional programming, pattern matching |
| **OCaml** | Type inference, variant types |

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions while adopting simpler syntax and lower cognitive overhead. The **seam model** is more concise than Rust's `async/await`—just one `spawn` marker, no manual Future and Pin management. "All things arise together, and I observe their return"—making concurrent programming as intuitive as describing natural laws. Send/Sync constraints provide equivalent thread safety guarantees. Unified type syntax eliminates `enum`/`struct`/`union` concept fragmentation.

**Q: What types of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted programming. The goal is to become a general-purpose programming language.

**Q: Why 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing confusion from deep nesting. This is a carefully considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. Release timing depends on implementation progress; see [Version Planning RFC](./rfc/003-version-planning.md) for details.

**Q: How do I contact the core team?**

A: Through GitHub Discussions or Discord community channels. Core team members respond regularly.

---

> **Last Updated**: 2025-01-17
>
> **Document Version**: v1.2.0
>
> **License**: [MIT](LICENSE)

---

> "YaoXiang transforms and all things are born. Types evolve and programs are formed."
>
> May YaoXiang's design journey walk alongside you.