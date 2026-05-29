# YaoXiang Design Manifesto

> **Version**: v1.2.0
> **Status**: Official Release
> **Authors**: MorningXU + YaoXiang Community
> **Date**: 2025-01-17

---

> "The Tao gives birth to the One, the One gives birth to the Two, the Two gives birth to the Three, and the Three gives birth to all things."
> — *Tao Te Ching*
>
> Types are like the Tao; all things are born from them.

---

## I. Why Create YaoXiang?

### 1.1 The Language Gap We Fill

Throughout the history of programming languages, we have witnessed the birth and evolution of many excellent languages: C brought efficiency revolution to systems programming, Python created a programming experience accessible to everyone, Rust proved that memory safety and performance can coexist, and TypeScript made large-scale frontend projects maintainable. However, when we survey today's language ecosystem, we still find a clear gap—no single language can simultaneously satisfy these three core requirements:

| Requirement | Problems with Existing Solutions |
|-------------|----------------------------------|
| **Type Safety** | Rust is too strict with a steep learning curve; TypeScript has optional types, unable to provide compile-time guarantees |
| **Natural Syntax** | Rust syntax is complex and obscure; Haskell's functional paradigm has a high barrier to entry; traditional static languages are verbose |
| **AI Friendly** | Existing languages have syntactic ambiguity, complex ASTs, and unpredictable hidden behaviors, limiting AI's accuracy in generating and modifying code |

The birth of YaoXiang is precisely to fill this gap. We believe: **Programming languages should be both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Practical Problems We Solve

**Problem One: Type System Fragmentation**

Today's programming languages suffer from severe type system fragmentation. Statically typed languages pursue absolute correctness at compile time, but often at the cost of development efficiency; dynamically typed languages offer flexibility but reveal maintainability defects in large projects. YaoXiang proposes a unified abstraction framework of "everything is a type," making types the thread running through the language's design, rather than patches added after the fact.

**Problem Two: The Binary Choice Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime performance. GC (Garbage Collection) frees developers but brings latency jitter and memory overhead; manual memory management is efficient but as dangerous as walking a tightrope. YaoXiang adopts Rust's ownership model, eliminating data races and memory leaks at compile time while maintaining zero-cost abstractions, achieving high performance without GC.

**Problem Three: The Cognitive Burden of Asynchronous Programming**

Modern applications are inseparable from networking and concurrency, yet asynchronous programming has always been a nightmare for programmers. Nested callback functions, Promise chain calls, async/await syntax—each solution adds complexity to the code. YaoXiang has redesigned the asynchronous model: simply add a `spawn` marker after the function signature, and the compiler automatically handles all asynchronous details, making concurrent programming as natural as writing synchronous code.

**Problem Four: Bottlenecks in AI-Assisted Programming**

When AI begins to assist developers in writing code, the choices made in language design become crucial. Fuzzy syntax rules, implicit type conversions, complex syntactic sugar—these are characteristics that human programmers have grown accustomed to, yet they become obstacles for AI to understand and generate. YaoXiang has made "AI friendly" a core goal from the very beginning of its design: strict indentation rules, clear code block boundaries, unambiguous syntactic structures, enabling AI to accurately understand, generate, and modify code.

### 1.3 The Philosophical Roots of the Language

YaoXiang's name derives from the "Yao" (爻) and "Xiang" (象) in the *I Ching* (Book of Changes). "Yao" are the fundamental symbols that compose hexagrams, symbolizing the interplay of yin and yang, motion and stillness; "Xiang" is the external manifestation of the essence of things, representing all phenomena and encompassing the myriad things.

This philosophical thinking is reflected in every detail of the language design:

- **Unity**: Just as simple Yao symbols form complex hexagrams, YaoXiang constructs a complete programming model from a few core concepts (types, functions, constructors)
- **Hierarchy**: Just as Xiang has distinctions between prior and later heaven, YaoXiang's type system has a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Changeability**: Just as yin and yang flow and change endlessly, YaoXiang supports dependent types, allowing types to evolve as values change
- **Identifiability**: Just as hexagrams can be interpreted and all things have their Xiang, YaoXiang provides complete type reflection capabilities, with runtime type information fully available
- **Provability**: Just as hexagrams reveal the laws of things, YaoXiang's type system follows the Curry-Howard isomorphism (types are propositions, programs are proofs), making type checking the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any feature proposal must pass examination by these principles.

### 2.1 Principle One: Everything is a Type

In YaoXiang's worldview, types are the highest-level abstraction unit, the core concept running through the language.

**Concrete Manifestations**:

- **Values are instances of types**: `42` is an instance of type `Int`, `"hello"` is an instance of type `String`
- **Types themselves are also types**: The meta type (`type`) is the type of all types, `Int` is an instance of `type`
- **Functions are type mappings**: `fn add(Int, Int) -> Int` describes a type mapping from `Int × Int` to `Int`
- **Modules are type compositions**: Modules are named combinations of namespaces containing functions and types

**Non-Negotiable Reason**: Unified type abstraction simplifies language semantics, eliminates the dualism between values and types, and makes the type system the guardian of code correctness, not a stumbling block.

### 2.2 Principle Two: Strictly Structured

YaoXiang's syntax design pursues "unambiguous, predictable, and easy to parse."

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are prohibited; code block boundaries are clear at a glance
- **Parentheses cannot be omitted**: Function parameters must have parentheses; list elements must have commas
- **Code blocks must use curly braces**: Control flow constructs like `if`, `while`, `for` must be wrapped in `{ }`
- **Streamlined keyword count**: Only 17 core keywords are retained, refusing syntactic sugar proliferation

**Non-Negotiable Reason**: Strict structuring brings three key advantages—(1) more accurate IDE syntax highlighting and code folding; (2) significantly improved AI code generation and modification accuracy; (3) new learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: Generic functions are expanded into concrete versions at compile time, with no virtual table lookup overhead
- **Inline optimization**: Simple functions are automatically inlined, eliminating function call overhead
- **Stack allocation priority**: Small objects are allocated on the stack by default; heap allocation is only used when necessary
- **No GC**: The ownership model guarantees memory safety without garbage collector runtime overhead

**Non-Negotiable Reason**: Performance is the survival bottom line of programming languages. Any design that sacrifices performance for convenience is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity are inseparable companions. YaoXiang chooses immutable by default, making code easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default; once assigned, they cannot be modified
- When mutability is needed, it must be explicitly declared with `mut`
- References are immutable by default; mutable references require `mut` marking
- Ownership transfer means the original binding becomes invalid

**Non-Negotiable Reason**: Immutability is the foundation of concurrency safety, the guarantee of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types as Data

Type information should not only exist at compile time but should be fully available at runtime.

**Specific Capabilities**:

- Runtime type queries: Any value can obtain its type information
- Type reflection: Can construct and manipulate types themselves
- Pattern matching deconstruction: Type constructors can be directly used in pattern matching
- Generic specialization: Runtime can obtain instantiated types of generic parameters

**Non-Negotiable Reason**: Complete type reflection capability is the foundation of metaprogramming and the cornerstone of high-performance frameworks and tools.

---

## III. Key Innovations and Features

While absorbing the excellent features of existing languages, YaoXiang proposes the following innovative designs.

### 3.1 Innovation One: Unified Type Syntax

**Traditional language type definitions** often require multiple keywords:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
union IntOrFloat { i: i32, f: f32 }
```

**YaoXiang's unified syntax**: Only the `type` keyword, `{}` for data structures, `[]` for interface types.

```yaoxiang
# === Data Types (curly braces) ===

# Struct
type Point = { x: Float, y: Float }

# Enum (multiple constructors)
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Zero-argument constructors
type Color = { red | green | blue }

# Hybrid type
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

**Innovation Value**: Unified type syntax, eliminating keyword fragmentation between `enum`/`struct`/`union`/`trait`.

### 3.2 Innovation Two: Constructors as Types

**Value construction and pattern matching are completely identical**:

```yaoxiang
# Type definition
type Point = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Value construction: identical to function calls
p: Point = Point(3.0, 4.0)
r: Result[Int, String] = ok(42)
err_msg: Result[Int, String] = err("not found")
c: Color = green

# Pattern matching: direct deconstruction
match point {
    Point(0.0, 0.0) -> "origin"
    Point(x, y) -> "point at ({x}, {y})"
}
```

### 3.3 Innovation Three: Curried Method Binding

YaoXiang uses pure functional design, achieving object method call syntax through currying, without introducing `class` and `method` keywords.

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
Point.distance = distance  # Bound to position 0 by default
```

```yaoxiang
# === main.yx ===

use Point

main() -> Void = () => {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # Two calling methods are completely equivalent
    d1 = distance(p1, p2)      # Direct core function call
    d2 = p1.distance(p2)       # Method syntax sugar

    # Functional usage: pre-bind first argument
    dist_from_origin = Point.distance(Point(0.0, 0.0))
    result = dist_from_origin(p1)   # 5.0

    # Currying usage: lazy evaluation
    get_dist_to_p2 = p2.distance(_)
    d3 = get_dist_to_p2(p1)         # 2.828
}
```

**Innovation Value**: Pure functional design, no hidden `self` parameter, functions as values can be freely passed and combined.

### 3.4 Innovation Four: Spawn Model

> "All things arise together, and I observe their return." — *I Ching, Hexagram 24 (Fu)*

The spawn model draws its meaning from this, describing a programming paradigm: developers describe logic with synchronous, sequential thinking, while the language runtime causes computational units within it to automatically and efficiently execute concurrently like all things arising together, and unify and coordinate at the end.

**Three Core Principles**:

| Principle | Description |
|-----------|-------------|
| **Synchronous Syntax** | What you see is sequential code |
| **Concurrent Nature** | Runtime automatically extracts parallelism |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology System**:

| Official Term | Corresponding Syntax | Explanation |
|---------------|---------------------|-------------|
| **Spawn Function** | `spawn (params) => body` | Defines a computational unit that can participate in spawn execution |
| **Spawn Block** | `spawn { a(), b() }` | Explicitly declared concurrency domain; tasks within the block execute concurrently |
| **Spawn Loop** | `spawn for x in xs { ... }` | Data parallelism; loop body executes concurrently on all elements |
| **Spawn Value** | `Async(T)` | A future value currently in spawn; automatically awaited when used |
| **Spawn Graph** | Lazy evaluation graph (DAG) | The stage where spawn occurs; describes dependencies and parallelism |
| **Spawn Scheduler** | Runtime task scheduler | Coordinates all things, letting them spawn at the right moments |

> **See also**: [RFC-001 Spawn Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === Spawn Function ===
# Function marked with spawn (RFC-003 syntax)
fetch_data: String -> JSON = spawn (url) => {
    HTTP.get(url).json()
}

# === Spawn Block ===
# Expressions within spawn { } execute in parallel by force
compute_all: () -> (Int, Int, Int) = spawn () => {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    (a, b, c)
}

# === Spawn Loop ===
# Loop marked with spawn for automatically parallelizes
parallel_sum: Int -> Int = spawn (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)  # Each iteration executes in parallel
    }
    total
}

# === Automatic Await ===
main() -> Void = () => {
    # Two independent requests execute in parallel automatically
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # Await points are automatically inserted when results are needed
    print(users.length + posts.length)  # Automatically awaits users and posts
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
- See [RFC-001 Spawn Model](./rfc/001-concurrent-model-error-handling.md)

**Innovation Value**: The cognitive burden of asynchronous programming drops to zero; code readability is completely identical to synchronous code, while gaining high-performance parallel execution efficiency.

### 3.5 Innovation Five: Dependent Type Support (Future Feature)

> **Status**: Consider implementation after v1.0

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

**Innovation Value**: Capture more errors at compile time, achieving more precise type guarantees.

### 3.6 Innovation Six: Minimalist Keyword Design

YaoXiang defines only 18 core keywords, far fewer than mainstream languages:

```
type   pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

| Comparison Language | Keyword Count |
|---------------------|---------------|
| YaoXiang | **18** |
| Rust | 51+ |
| Python | 35 |
| TypeScript | 64+ |
| Go | 25 |

**Innovation Value**: Lower memory burden, more consistent syntax style, easier-to-parse syntactic structure.

---

## IV. Preliminary Syntax Preview

The following code examples showcase YaoXiang's language style, helping you quickly sense its design aesthetics.

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

# Destructuring pattern
type Point = { x: Float, y: Float }
match point {
    Point(0.0, 0.0) -> "origin"
    Point(x, y) -> "point at ({x}, {y})"
}
```

### 4.4 List Comprehensions

```yaoxiang
# Python-style list comprehension
evens = [x * 2 for x in 0..10]          # [0, 4, 8, 12, 16]
squares = [x * x for x in 1..10 if x % 2 == 1]  # [1, 9, 25, 49, 81]

# Nested comprehension
matrix = [[i * j for j in 1..4] for i in 1..3]
# [[1, 2, 3], [2, 4, 6], [3, 6, 9]]
```

### 4.5 Ownership Model

```yaoxiang
type Point = { x: Float, y: Float }

# Default Move (zero-copy)
p = Point(1.0, 2.0)
p2 = p              # Move, ownership transferred, p invalidated

# Explicit ref = Arc (safe sharing)
shared = ref p      # Arc, thread-safe

spawn(() => print(shared.x))   # ✅ Safe

# Explicit clone() = copy
p3 = p.clone()      # p and p3 are independent
```

**Core Rules**:
- Default Move (zero-copy)
- Sharing with `ref` (Arc)
- Copies with `clone()`
- System-level with `unsafe` + `*T`

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
# Spawn-marked async function
fetch_api: String -> JSON spawn = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

# Concurrent construct block: explicit parallelism
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

## V. Roadmap and Pending Items

### 5.1 Decided Design Decisions

The following decisions have been fully discussed and reviewed, **no longer accepting changes**:

| Module | Decision | Description |
|--------|----------|-------------|
| **Type System** | Everything is a type | Values, functions, modules, generics are all types |
| **Type Syntax** | Unified curly brace syntax | `{}` for data types, `[]` for interface types |
| **Indentation Rule** | 4-space indentation | Mandatory, Tab prohibited |
| **Keywords** | 17 core keywords | As listed above |
| **Function Syntax** | Arrow function syntax | `name: (Type1, Type2) -> Type = (params) => body` |
| **Method Binding** | RFC-004 curried binding | `Type.method = function` (bound to position 0 by default) |
| **Async Model** | RFC-003 spawn model | `spawn (params) => body`, lazy evaluation, automatic parallelization |
| **Memory Management** | Ownership model | No GC, compile-time safety guarantees |
| **File as Module** | Module system | Each `.yx` file is a module |
| **Main Function** | `main: () -> Void` | Program entry point |
| **Thread Safety** | Send/Sync constraints | Compile-time elimination of data races |

### 5.2 Pending Design Topics Under Discussion

The following topics are still under discussion, **community contributions welcome**:

| Topic | Current Status | Open Questions |
|-------|----------------|----------------|
| **Literal Syntax** | Float support | Support `3.14e-10` scientific notation? |
| **Generic Inference** | Basic support | Support return type generic inference? |
| **Macro System** | Not yet designed | Need hygienic macros? Syntax design direction? |
| **Package Manager** | Not yet designed | Need centralized package registry? Dependency resolution strategy? |
| **FFI** | Not yet designed | Specific plan for C interop? |
| **Generic Constraints** | Basic support | Support trait/bounds mechanism? |
| **Reflection Depth** | Basic support | Support accessing private members? |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            YaoXiang Implementation Roadmap                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0:     │
│        ✅ Completed                   │ (current stage)           Rust AOT   │
│                                      │                            Compiler  │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←────── v1.0: YaoXiang JIT Compiler ←──── v2.0│
│        (self-hosted)                    (self-hosted)                       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Milestone Details**:

| Version | Status | Goal | Deliverables |
|---------|--------|------|--------------|
| **v0.1** | ✅ Completed | Interpreter prototype | Basic interpreter, lexical analysis, syntax analysis, basic types |
| **v0.2** | ✅ Completed | Complete interpreter | Type checking, pattern matching, module system |
| **v0.3** | 🔄 In Progress | Bytecode generation | IR intermediate representation, bytecode generation, closure optimization, monomorphization |
| **v0.4** | 🔄 In Progress | Bytecode VM | VM core, instruction execution, call frame management, inline caching |
| **v0.5** | ⏳ To Start | Runtime system | GC, scheduler, standard library IO |
| **v1.0** | ⏳ To Start | AOT compiler | Complete optimization, native code generation |
| **v2.0** | ⏳ To Start | Self-hosted compiler | New compiler written in YaoXiang |

### 5.4 Current Implementation Status

| Module | Status | Completion | Description |
|--------|--------|------------|-------------|
| **Lexer** | ✅ Completed | 100% | Token definition, keyword recognition, test cases |
| **Parser** | ✅ Completed | 100% | AST definition, expression/statement parsing, boundary tests |
| **Type Checker** | ✅ Completed | 95% | Type inference, monomorphization, generic specialization, error handling |
| **IR Intermediate Representation** | ✅ Completed | 90% | IR instruction definition, type representation, control flow graph |
| **Bytecode Generation** | ✅ Completed | 85% | Expression/statement/control flow bytecode, closure conversion |
| **Ownership System** | ✅ Completed | 100% | Move semantics, Clone/Drop semantics, mutability checking, Send/Sync constraints |
| **Monomorphization** | ✅ Completed | 100% | Generic instantiation, specialization implementation |
| **Escape Analysis** | 🔄 In Progress | 40% | Basic framework, variable escape judgment |
| **Bytecode VM** | 🔄 In Progress | 70% | VM core, instruction execution, call frames, inline caching |
| **Runtime Scheduler** | 🔄 In Progress | 60% | Task descriptors, work-stealing queues, wait queues |
| **Runtime Memory** | 🔄 In Progress | 50% | Memory allocator, GC framework |
| **Standard Library** | 🔄 In Progress | 30% | IO, String, List, Dict, Math, Concurrent |
| **JIT Compiler** | ⏳ To Start | 0% | Pending Cranelift/LLVM integration |
| **AOT Compiler** | ⏳ To Start | 0% | Pending implementation |

**Code Generation Module Details**:

| Sub-Module | Status | Key Features |
|------------|--------|--------------|
| Expression Generation | ✅ Completed | Arithmetic, comparison, logic, function calls |
| Statement Generation | ✅ Completed | Assignment, return, conditionals, loops |
| Control Flow Generation | ✅ Completed | Switch pattern matching, loop unrolling |
| Closure Handling | ✅ Completed | Environment capture, closure conversion |
| Bytecode Serialization | ✅ Completed | Bytecode read/write, test cases |
| Generator Code Generation | ✅ Completed | yield syntax support, state machine conversion |
| Integration Tests | ✅ Completed | End-to-end compilation execution tests |

**Async Implementation Status (Spawn Model)**:

| Sub-Module | Status | Description |
|------------|--------|-------------|
| spawn keyword parsing | ✅ Completed | Lexer/syntax analysis support |
| is_async flag | ✅ Completed | AST/type system support |
| Async(T) type design | ✅ Completed | Design documentation complete |
| Scheduler framework | ✅ Completed | Basic work-stealing implementation |
| Send/Sync constraints | ✅ Completed | Type constraint design documentation |
| IR extension | 🔄 In Progress | CallAsync instruction defined |
| VM async instructions | 🔄 In Progress | Instruction framework defined |
| Complete implementation | ⏳ To Start | v0.5 milestone |

---

## VI. How to Contribute

YaoXiang is a language born from the community, growing in the community, and serving the community. We sincerely invite every developer passionate about programming language design to join this journey of exploration.

### 6.1 Design Discussion

**Suitable for**: Programming language theory researchers, type system enthusiasts, language design enthusiasts

**Participation Methods**:

- **GitHub Discussions**: Participate in "Language Design" category discussions
- **Design Proposals (RFC)**: Propose design documents for new features, following templates under the `rfcs/` directory
- **Syntax Review**: Provide improvement suggestions or discover potential issues in existing syntax design

| **Current Hot Topics**: |
| |
| - Macro system design and implementation |
| - Interface type mechanism |
| - Error handling syntax optimization |
| - Standard library API design |

**Submitting Design Proposals**:

1. Create a new file in the `rfcs/` directory
2. Fill out the RFC template (motivation, detailed design, pros/cons analysis, alternatives)
3. Open a Pull Request for community review
4. After core team review, merge or reject

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, systems programmers, performance optimization experts

**Current Implementation Focus** (by priority):

| Priority | Module | Description | Difficulty |
|----------|--------|-------------|------------|
| P0 | **Bytecode VM** | VM instruction completion, performance optimization | Medium |
| P0 | **Runtime Memory** | GC implementation, memory allocator | High |
| P0 | **Async Runtime** | Complete spawn model implementation | High |
| P1 | Standard Library | IO, String, List, Concurrent | Medium |
| P1 | JIT Compiler | Cranelift integration | High |
| P2 | AOT Compiler | LLVM/Cranelift backend | High |
| P3 | Self-hosted Compiler | Rewrite in YaoXiang | Extremely High |

**Tech Stack**:

- **Implementation Language**: Rust (current stage)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Test Framework**: Rust `#[test]` + `cargo nextest`

**Getting Started with Contributions**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand architecture design
2. Choose a module of interest under `src/`
3. Check `tests/unit/` to understand test requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, productivity tool pursuers

**Tools Needing Development**:

| Tool | Status | Description |
|------|--------|-------------|
| **LSP Server** | ⏳ To Start | Language Server Protocol support |
| **Debugger Integration** | ⏳ To Start | GDB/LLDB integration |
| **Formatter** | ⏳ To Start | `yaoxiang fmt` |
| **Package Manager** | ⏳ To Start | Dependency management, version resolution |
| **Package Registry** | ⏳ To Start | Centralized or decentralized |
| **REPL** | ⏳ To Start | Interactive interpreter |
| **Benchmark Tool** | ⏳ To Start | Performance analysis |
| **VS Code Extension** | ⏳ To Start | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin** | ⏳ To Start | Syntax highlighting, LSP client |

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

| Module | Priority | Description |
|--------|----------|-------------|
| `std.io` | P0 | File IO, console input/output |
| `std.string` | P0 | String operations, formatting |
| `std.list` | P0 | List/array operations |
| `std.dict` | P0 | Dictionary/hashmap |
| `std.math` | P0 | Mathematical functions, constants |
| `std.time` | P1 | Date/time operations |
| `std.net` | P1 | Network programming, HTTP |
| `std.concurrent` | P1 | Concurrency primitives, channels |
| `std.crypto` | P2 | Cryptographic hashing, signatures |
| `std.json` | P1 | JSON parsing/generation |
| `std.regex` | P2 | Regular expressions |
| `std.database` | P3 | Database connectivity |
| `std.gui` | P3 | Graphical interface (long-term) |

**Design Principles**:

- Consistency: Functions with the same functionality maintain consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoiding over-design
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Each function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documentation Needing Contributions**:

| Document | Status | Description |
|----------|--------|-------------|
| Quick Start | ✅ Completed | 5-minute getting started guide |
| Language Guide | ✅ Completed | Systematic learning of core concepts |
| Language Specification | ✅ Completed | Complete syntax and semantics definition |
| Implementation Plan | ✅ Completed | Compiler implementation technical details |
| API Documentation | ⏳ To Start | Standard library API reference |
| Tutorials | ⏳ To Start | Advanced tutorials and best practices |
| Blog | ⏳ To Start | Technical articles and design stories |
| Translations | ⏳ To Start | Multi-language support |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

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
2. **Choose a direction**: Select a contribution area based on your interests
3. **Set up environment**: Rust 1.75+, cargo, git
4. **Find tasks**: Check `good first issue` labels in GitHub Issues
5. **Submit PR**: Follow submission guidelines, write tests
6. **Participate in review**: Review others' code, join discussions

**Submission Format**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: New feature
fix: Bug fix
docs: Documentation update
style: Code formatting (no functional impact)
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

- Follow `rustfmt.toml` specifications
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
| `spawn` | Spawn marker (RFC-003: `spawn (params) => body`) |
| `ref` | Shared pointer (Arc) |
| `mut` | Mutable variable |
| `if/elif/else` | Conditional branches |
| `match` | Pattern matching |
| `while/for` | Loops |
| `return/break/continue` | Control flow |
| `as` | Type casting |
| `in` | Membership test/list comprehension |
| `unsafe` | unsafe code block (raw pointers) |

### A.2 Annotations

| Annotation | Purpose |
|------------|---------|
| `@block` | Marks a function to be executed synchronously |
| `@eager` | Marks an expression to be eagerly evaluated |
| `@Send` | Explicitly declares satisfying Send constraint |
| `@Sync` | Explicitly declares satisfying Sync constraint |

### A.3 Primitive Types

| Type | Description | Default Size |
|------|-------------|--------------|
| `Void` | Empty value | 0 bytes |
| `Bool` | Boolean | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating point | 8 bytes |
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
|--------|------------------|
| **Rust** | Ownership model, zero-cost abstractions, type system |
| **Python** | Syntax style, readability, list comprehensions |
| **Idris/Agda** | Dependent types, type-driven development |
| **Curry-Howard Isomorphism** | Types are propositions, programs are proofs, unified theory of types and logic |
| **TypeScript** | Type annotations, runtime types |
| **MoonBit** | AI-friendly design, concise syntax |
| **Haskell** | Pure functional, pattern matching |
| **OCaml** | Type inference, variant types |

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions but uses simpler syntax and lower cognitive burden. The **spawn model** is more concise than Rust's `async/await`—just one `spawn` marker, no manual Future and Pin management. "All things arise together, and I observe their return," making concurrent programming as intuitive as describing natural laws. Send/Sync constraints provide equivalent thread safety guarantees. Unified type syntax eliminates conceptual fragmentation between `enum`/`struct`/`union`.

**Q: What types of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing confusion from nesting depth. This is a carefully considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. Release timing depends on implementation progress; see [Version Planning RFC](./rfc/003-version-planning.md) for details.

**Q: How can I contact the core team?**

A: Through GitHub Discussions or Discord community channel. Core team members respond regularly.

---

> **Last Updated**: 2025-01-17
>
> **Document Version**: v1.2.0
>
> **License**: [MIT](LICENSE)

---

> "The changes of YaoXiang give birth to all things. The evolution of types completes the program."

> May YaoXiang's design journey walk alongside you.