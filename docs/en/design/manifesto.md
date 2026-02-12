# YaoXiang Design Manifesto

> **Version**: v1.2.0
> **Status**: Released
> **Authors**: ChenXu + YaoXiang Community
> **Date**: 2025-01-17

---

> "One generates two, two generate three, three generate all things."
> — Dao De Jing
>
> Types are like the Dao, from which all things emerge.

---

## 1. Why Create YaoXiang?

### 1.1 Bridging the Language Gap

Throughout the history of programming languages, we have witnessed the birth and evolution of many excellent languages: C brought efficiency revolution to systems programming, Python created a programming experience accessible to everyone, TypeScript made large-scale frontend projects maintainable. However, when we examine today's language landscape, we still find a clear gap—no single language can simultaneously satisfy these three core needs:

| Need | Problems with Existing Solutions |
|------|----------------------------------|
| **Type Safety** | Rust is too strict with a steep learning curve; TypeScript has optional types and cannot provide compile-time guarantees |
| **Natural Syntax** | Rust syntax is complex and obscure; Haskell has a high barrier for functional programming; traditional static languages are verbose |
| **AI-Friendly** | Existing languages have many syntactic ambiguities, complex ASTs, and unpredictable hidden behaviors, limiting AI's accuracy in generating and modifying code |

YaoXiang was created to fill this gap. We believe: **programming languages should be both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Real Problems We Solve

**Problem One: Type System Fragmentation**

Today's programming languages exhibit severe fragmentation in type systems. Static typed languages pursue absolute correctness at compile time, but often at the cost of development efficiency; dynamic typed languages provide flexibility, but reveal maintainability defects in large projects. YaoXiang proposes a unified abstraction framework of "Everything is a Type," making types the main thread running through language design, not patches added after the fact.

**Problem Two: The Choice Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime performance. GC (Garbage Collection) frees developers but introduces latency jitter and memory overhead; manual memory management is efficient but as dangerous as walking a tightrope. YaoXiang adopts a Rust-style ownership model that eliminates data races and memory leaks at compile time while maintaining zero-cost abstractions, enabling high performance without GC.

**Problem Three: Cognitive Burden of Async Programming**

Modern applications are inseparable from networks and concurrency, yet async programming has always been a nightmare for programmers. Callback function nesting, Promise chaining, async/await syntax—each solution adds complexity to code. YaoXiang redesigned the async model: simply add a `spawn` marker after the function signature, and the compiler automatically handles all async details, making concurrent programming as natural as synchronous code.

**Problem Four: Bottlenecks in AI-Assisted Programming**

When AI begins to assist developers in writing code, the choices in language design become crucial. Fuzzy syntax rules, implicit type conversions, complex syntactic sugar—these features that human programmers have gotten used to become obstacles for AI to understand and generate. YaoXiang has "AI-Friendly" as a core goal from the start: strict indentation rules, clear code block boundaries, unambiguous syntax structures, enabling AI to accurately understand, generate, and modify code.

### 1.3 Philosophical Foundation of the Language

YaoXiang's name comes from "Yao" and "Xiang" in the Book of Changes. "Yao" are the basic symbols that make up hexagrams, symbolizing the changes of yin and yang, movement and stillness; "Xiang" is the external manifestation of the essence of things, representing all things and phenomena.

This philosophical thinking is reflected in every detail of language design:

- **Unity**: Just as simple Yao symbols compose complex hexagrams, YaoXiang uses a few core concepts (types, functions, constructors) to build a complete programming model
- **Hierarchy**: Just as Xiang has pre-heaven and post-heaven distinctions, YaoXiang's type system has a clear hierarchical structure, from primitive types to generics, from values to meta-types
- **Variability**: Just as yin and yang flow and change infinitely, YaoXiang supports dependent types, allowing types to evolve as values change
- **Recognizability**: Just as hexagrams can be interpreted and all things can be represented, YaoXiang provides complete type reflection capabilities, with runtime type information fully available

---

## 2. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any feature proposal must pass the test of these principles.

### 2.1 Principle One: Everything is a Type

In YaoXiang's worldview, type is the highest-level abstract unit, the core concept running through the language.

**Concrete manifestations**:

- **Values are instances of types**: `42` is an instance of `Int` type, `"hello"` is an instance of `String` type
- **Types themselves are also types**: Meta-type (`type`) is the type of all types, `Int` is an instance of `type`
- **Functions are type mappings**: `fn add(Int, Int) -> Int` describes a type mapping from `Int × Int` to `Int`
- **Modules are type combinations**: Modules are namespace combinations containing functions and types

**Non-negotiable reason**: Unifying type abstraction simplifies language semantics, eliminates the binary opposition between values and types, and makes the type system a guardian of code correctness, not a stumbling block.

### 2.2 Principle Two: Strictly Structured

YaoXiang's syntax design pursues "unambiguous, predictable, easy to parse."

**Specific rules**:

- **Mandatory 4-space indentation**: Tab characters are prohibited, code block boundaries are clear
- **Brackets cannot be omitted**: Function parameters must have parentheses, list elements must have commas
- **Code blocks must have curly braces**: Control flow like `if`, `while`, `for` must use `{ }` to wrap
- **Streamlined keyword count**: Only 17 core keywords, no syntactic sugar proliferation

**Non-negotiable reason**: Strict structuring brings three key advantages—(1) IDE syntax highlighting and code folding are more accurate; (2) AI code generation and modification accuracy greatly improves; (3) New learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstraction

High-level abstractions should not incur runtime performance costs.

**Specific guarantees**:

- **Monomorphization**: Generic functions are expanded into concrete versions at compile time, no vtable lookup overhead
- **Inlining optimization**: Simple functions are automatically inlined, eliminating function call overhead
- **Stack allocation preferred**: Small objects are stack-allocated by default, heap allocation only when necessary
- **No GC**: Ownership model guarantees memory safety, no garbage collector runtime overhead

**Non-negotiable reason**: Performance is the lifeline of programming languages. Any design that sacrifices performance for convenience is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity go hand in hand. YaoXiang chooses immutable by default, making code easier to reason about and understand.

**Specific rules**:

- Variables are immutable by default, cannot be modified after assignment
- Mutability must be explicitly declared with `mut` when needed
- References are immutable by default, mutable references require `mut` marker
- Ownership transfer means the original binding becomes invalid

**Non-negotiable reason**: Immutability is the foundation of concurrency safety, a guarantee of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Type as Data

Type information should not only exist at compile time but should be fully available at runtime.

**Specific capabilities**:

- Runtime type query: Any value can obtain its type information
- Type reflection: Can construct and manipulate types themselves
- Pattern matching destructuring: Type constructors can be directly used in pattern matching
- Generic specialization: Runtime can obtain the concrete types of generic parameters

**Non-negotiable reason**: Complete type reflection capability is the foundation of metaprogramming and the cornerstone of high-performance frameworks and tools.

---

## 3. Key Innovations and Features

While absorbing excellent features from existing languages, YaoXiang proposes the following innovative designs.

### 3.1 Innovation One: Unified Type Syntax

**Traditional language type definitions** often require multiple keywords:

```rust
// Rust
struct Point { x: f64, y: f64 }
enum Result<T, E> { Ok(T), Err(E) }
enum Color { Red, Green, Blue }
union IntOrFloat { i: i32, f: f32 }
```

**YaoXiang's unified syntax**: Only the `type` keyword, `{}` for data structure definitions, `[]` for interface type definitions.

```yaoxiang
# === Data Types (Curly Braces) ===

# Struct
type Point = { x: Float, y: Float }

# Enum (Multiple Constructors)
type Result[T, E] = { ok(T) | err(E) }

# Zero-Parameter Constructors
type Color = { red | green | blue }

# Mixed Types
type Shape = { circle(Float) | rect(Float, Float) }

# === Interface Types (Square Brackets) ===

# Interface Definition: Method Signature Collection
type Serializable = [
    serialize() -> String
]

type Drawable = [
    draw(Surface) -> Void,
    bounding_box() -> Rect
]

# === Generics ===

type Option[T] = { some(T) | none }
type Result[T, E] = { ok(T) | err(E) }
```

**Innovation value**: Unified type syntax, eliminating `enum`/`struct`/`union`/`trait` keyword fragmentation.

### 3.2 Innovation Two: Constructors as Types

**Value construction and pattern matching are identical**:

```yaoxiang
# Type Definition
type Point = { x: Float, y: Float }
type Result[T, E] = { ok(T) | err(E) }

# Value Construction: Same as Function Calls
p: Point = Point(3.0, 4.0)
r: Result[Int, String] = ok(42)
err_msg: Result[Int, String] = err("not found")
c: Color = green

# Pattern Matching: Direct Destructuring
match point {
    Point(0.0, 0.0) -> "origin"
    Point(x, y) -> "point at ({x}, {y})"
}
```

### 3.3 Innovation Three: Curried Method Binding

YaoXiang adopts pure functional design, achieving object method call-like syntax through currying, without introducing `class` and `method` keywords.

```yaoxiang
# === Point.yx ===

# Type Definition
type Point = { x: Float, y: Float }

# Core Function: Euclidean Distance
distance(Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# Method Syntax Sugar Binding
Point.distance = distance  # Binds to position 0 by default
```

```yaoxiang
# === main.yx ===

use Point

main() -> Void = () => {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # Two calling methods are completely equivalent
    d1 = distance(p1, p2)      # Direct core function call
    d2 = p1.distance(p2)        # Method syntax sugar

    # Functional Usage: Pre-bind first argument
    dist_from_origin = Point.distance(Point(0.0, 0.0))
    result = dist_from_origin(p1)   # 5.0

    # Curried Usage: Delayed Evaluation
    get_dist_to_p2 = p2.distance(_)
    d3 = get_dist_to_p2(p1)         # 2.828
}
```

**Innovation value**: Pure functional design, no hidden `self` parameter, functions as values can be freely passed and combined.

### 3.4 Innovation Four: Concurrency Model

> "All things grow together, and I observe their return." — I Ching, Hexagram Fu
>
> The concurrency model takes its meaning from this, describing a programming paradigm: developers describe logic in synchronous, sequential thinking, while the language runtime makes computational units automatically and efficiently execute concurrently like all things growing together, ultimately coordinating and unifying.

**Core Three Principles**:

| Principle | Description |
|-----------|-------------|
| **Synchronous Syntax** | What you see is what you get, sequential code |
| **Concurrent Essence** | Runtime automatically extracts parallelism |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology System**:

| Official Term | Corresponding Syntax | Explanation |
|---------------|---------------------|-------------|
| **Concurrent Function** | `spawn (params) => body` | Defines computational units that can participate in concurrent execution |
| **Concurrent Block** | `spawn { a(), b() }` | Explicitly declared concurrent domain, tasks within the block execute concurrently |
| **Concurrent Loop** | `spawn for x in xs { ... }` | Data parallelism, loop body executes concurrently on all elements |
| **Concurrent Value** | `Async[T]` | Future value currently in concurrent execution, automatically awaited when used |
| **Concurrent Graph** | Lazy Evaluation Graph (DAG) | The stage where concurrency happens, describing dependencies and parallelism relationships |
| **Concurrent Scheduler** | Runtime Task Scheduler | Coordinates all things, an intelligent center that makes them execute concurrently at the right time |

> **See also**: [RFC-001 Concurrency Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === Concurrent Function ===
# spawn-marked function (RFC-003 syntax)
fetch_data: String -> JSON = spawn (url) => {
    HTTP.get(url).json()
}

# === Concurrent Block ===
# Expressions in spawn { } are forced to execute in parallel
compute_all: () -> (Int, Int, Int) = spawn () => {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    (a, b, c)
}

# === Concurrent Loop ===
# spawn for-marked loops are automatically parallelized
parallel_sum: Int -> Int = spawn (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)  # Each iteration executes in parallel
    }
    total
}

# === Automatic Awaiting ===
main() -> Void = () => {
    # Two independent requests execute automatically in parallel
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # Wait points are automatically inserted when results are needed
    print(users.length + posts.length)  # Automatically awaits users and posts
}
```

**Thread Safety**:

```yaoxiang
# Send/Sync Constraints Guarantee Compile-Time Thread Safety
type SafeCounter = SafeCounter(mutex: Mutex[Int])

main: () -> Void = () => {
    counter: Arc[SafeCounter] = Arc.new(SafeCounter(Mutex.new(0)))

    # spawn Automatically Checks Send Constraints
    spawn () => {
        guard = counter.mutex.lock()
        guard.value = guard.value + 1
    }
}
```

**Technical Documentation**:
- See [RFC-001 Concurrency Model](./rfc/001-concurrent-model-error-handling.md)

**Innovation value**: The cognitive burden of async programming is reduced to zero, code readability is exactly the same as synchronous code, while achieving high-performance parallel execution efficiency.

### 3.5 Innovation Five: Dependent Type Support (Future Feature)

> **Status**: Consider implementing after v1.0

Types can depend on values, enabling true type-driven development.

```yaoxiang
# Fixed-Length Vector (Future Syntax)
type Vector[T, n: Nat] = {
    data: [T; n]
    length: n
}

# Usage
vec: Vector[Int, 3] = Vector([1, 2, 3], 3)

# Type Checking
# vec: Vector[Int, 3] = Vector([1, 2], 2)  # Compile error! Length mismatch
```

**Innovation value**: Capture more errors at compile time, achieve more precise type guarantees.

### 3.6 Innovation Six: Minimalist Keyword Design

YaoXiang defines only 18 core keywords, far fewer than mainstream languages:

```
type   pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

| Language | Keyword Count |
|----------|---------------|
| YaoXiang | **18** |
| Rust | 51+ |
| Python | 35 |
| TypeScript | 64+ |
| Go | 25 |

**Innovation value**: Lower memory burden, more consistent syntax style, easier to parse syntax structure.

---

## 4. Preliminary Syntax Preview

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
# Unified Type Syntax
type Point = { x: Float, y: Float }
type Result[T, E] = { ok(T) | err(E) }
type Color = { red | green | blue }

# Interface Types
type Serializable = [ serialize() -> String ]

# Function Definitions
add: (Int, Int) -> Int = (a, b) => a + b

# Generic Functions
identity: [T](T) -> T = (x) => x

# Multi-line Functions
fact: Int -> Int = (n) => {
    if n == 0 { 1 } else { n * fact(n - 1) }
}
```

### 4.3 Pattern Matching

```yaoxiang
# Powerful Pattern Matching
classify(Int) -> String = (n) => {
    match n {
        0 -> "zero"
        1 -> "one"
        _ if n < 0 -> "negative"
        _ -> "positive"
    }
}

# Destructuring Patterns
type Point = { x: Float, y: Float }
match point {
    Point(0.0, 0.0) -> "origin"
    Point(x, y) -> "point at ({x}, {y})"
}
```

### 4.4 List Comprehensions

```yaoxiang
# Python-style List Comprehensions
evens = [x * 2 for x in 0..10]          # [0, 4, 8, 12, 16]
squares = [x * x for x in 1..10 if x % 2 == 1]  # [1, 9, 25, 49, 81]

# Nested Comprehensions
matrix = [[i * j for j in 1..4] for i in 1..3]
# [[1, 2, 3], [2, 4, 6], [3, 6, 9]]
```

### 4.5 Ownership Model

```yaoxiang
type Point = { x: Float, y: Float }

# Default Move (Zero-Copy)
p = Point(1.0, 2.0)
p2 = p              # Move, ownership transfer, p becomes invalid

# Explicit ref = Arc (Safe Sharing)
shared = ref p      # Arc, thread-safe

spawn(() => print(shared.x))   # ✅ Safe

# Explicit clone() = Copy
p3 = p.clone()      # p and p3 are independent
```

**Core Rules**:
- Default Move (Zero-Copy)
- Sharing with `ref` (Arc)
- Copies with `clone()`
- System-level with `unsafe` + `*T`

### 4.6 Error Handling

```yaoxiang
# Result Type
type Result[T, E] = { ok(T) | err(E) }

divide: (Float, Float) -> Result[Float, String] = (a, b) => {
    if b == 0.0 {
        err("Division by zero")
    } else {
        ok(a / b)
    }
}

# Using ? Operator
process: () -> Result[Int, String] = () => {
    a = read_number()?
    b = read_number()?
    c = divide(a, b)?
    ok(c * 2)
}
```

### 4.7 Concurrent Programming (SeamlessAsync)

```yaoxiang
# spawn-marked async function
fetch_api: String -> JSON spawn = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

# Concurrent Construction Block: Explicit Parallelism
process_all: () -> (JSON, JSON, JSON) spawn = () => {
    (a, b, c) = spawn {
        fetch_api("https://api1.com/data"),
        fetch_api("https://api2.com/data"),
        fetch_api("https://api3.com/data")
    }
    (a, b, c)
}

# Data-Parallel Loop
parallel_process: Int -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        compute(i)
    }
    total
}

# Thread Safety Example
type ThreadSafeCounter = { value: Mutex[Int] }

main: () -> Void = () => {
    counter = ThreadSafeCounter(Mutex.new(0))

    # spawn Automatically Checks Send Constraints
    spawn () => {
        guard = counter.value.lock()
        guard.value = guard.value + 1
    }
}
```

---

## 5. Roadmap and Pending Items

### 5.1 Decided Design Decisions

The following decisions have been fully discussed and reviewed, **no longer accepting changes**:

| Module | Decision | Description |
|--------|----------|-------------|
| **Type System** | Everything is a Type | Values, functions, modules, generics are all types |
| **Type Syntax** | Unified Curly Brace Syntax | `{}` data types, `[]` interface types |
| **Indentation Rules** | 4-space indentation | Mandatory, Tab prohibited |
| **Keywords** | 17 core keywords | As listed above |
| **Function Syntax** | Arrow function syntax | `name: (Type1, Type2) -> Type = (params) => body` |
| **Method Binding** | RFC-004 Curried Binding | `Type.method = function` (binds to position 0 by default) |
| **Async Model** | RFC-003 Concurrent Model | `spawn (params) => body`, lazy evaluation, automatic parallelism |
| **Memory Management** | Ownership Model | No GC, compile-time safety guarantees |
| **File as Module** | Module System | Each `.yx` file is a module |
| **Main Function** | `main: () -> Void` | Program entry point |
| **Thread Safety** | Send/Sync Constraints | Compile-time elimination of data races |

### 5.2 Design Topics Under Discussion

The following topics are still under discussion, **community contributions welcome**:

| Topic | Current Status | Open Questions |
|-------|----------------|----------------|
| **Literal Syntax** | Float support | Should `3.14e-10` scientific notation be supported? |
| **Generic Inference** | Basic support | Should return type generic inference be supported? |
| **Macro System** | Not yet designed | Are hygienic macros needed? Syntax design direction? |
| **Package Manager** | Not yet designed | Is a centralized package registry needed? Dependency resolution strategy? |
| **FFI** | Not yet designed | What is the specific plan for C interop? |
| **Generic Constraints** | Basic support | Should trait/bounds mechanism be supported? |
| **Reflection Depth** | Basic support | Should private member access be supported? |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          YaoXiang Implementation Roadmap                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0:     │
│        ✅ Completed                    │ (Current Phase)         Rust AOT    │
│                                      │                         Compiler    │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←─────── v1.0: YaoXiang JIT Compiler ←─── v2.0│
│        (Self-hosted)                  (Self-hosted)                     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Milestone Details**:

| Version | Status | Goals | Deliverables |
|---------|--------|-------|--------------|
| **v0.1** | ✅ Completed | Interpreter Prototype | Basic interpreter, lexer, parser, basic types |
| **v0.2** | ✅ Completed | Complete Interpreter | Type checking, pattern matching, module system |
| **v0.3** | 🔄 In Progress | Bytecode Generation | IR intermediate representation, bytecode generation, closure optimization, monomorphization |
| **v0.4** | 🔄 In Progress | Bytecode VM | VM core, instruction execution, call frame management, inline cache |
| **v0.5** | ⏳ To Start | Runtime System | GC, scheduler, standard library IO |
| **v1.0** | ⏳ To Start | AOT Compiler | Complete optimization, native code generation |
| **v2.0** | ⏳ To Start | Self-hosted Compiler | New compiler written in YaoXiang |

### 5.4 Current Implementation Status

| Module | Status | Completeness | Description |
|--------|--------|--------------|-------------|
| **Lexer** | ✅ Completed | 100% | Token definition, keyword recognition, test cases |
| **Parser** | ✅ Completed | 100% | AST definition, expression/statement parsing, boundary tests |
| **Type Checker** | ✅ Completed | 95% | Type inference, monomorphization, generic specialization, error handling |
| **IR Intermediate Representation** | ✅ Completed | 90% | IR instruction definition, type representation, control flow graph |
| **Bytecode Generation** | ✅ Completed | 85% | Expression/statement/control flow bytecode, closure conversion |
| **Ownership System** | ✅ Completed | 100% | Move semantics, Clone/Drop semantics, mutability checking, Send/Sync constraints |
| **Monomorphization** | ✅ Completed | 100% | Generic instantiation, specialization implementation |
| **Escape Analysis** | 🔄 In Progress | 40% | Basic framework, variable escape determination |
| **Bytecode VM** | 🔄 In Progress | 70% | VM core, instruction execution, call frames, inline cache |
| **Runtime Scheduler** | 🔄 In Progress | 60% | Task descriptors, work-stealing queues, wait queues |
| **Runtime Memory** | 🔄 In Progress | 50% | Memory allocator, GC framework |
| **Standard Library** | 🔄 In Progress | 30% | IO, String, List, Dict, Math, Concurrent |
| **JIT Compiler** | ⏳ To Start | 0% | Pending Cranelift/LLVM integration |
| **AOT Compiler** | ⏳ To Start | 0% | Pending implementation |

**Code Generation Module Details**:

| Submodule | Status | Key Features |
|-----------|--------|--------------|
| Expression Generation | ✅ Completed | Arithmetic, comparison, logic, function calls |
| Statement Generation | ✅ Completed | Assignment, return, condition, loop |
| Control Flow Generation | ✅ Completed | Switch pattern matching, loop unrolling |
| Closure Handling | ✅ Completed | Environment capture, closure conversion |
| Bytecode Serialization | ✅ Completed | Bytecode read/write, test cases |
| Generator Code Generation | ✅ Completed | Yield syntax support, state machine conversion |
| Integration Testing | ✅ Completed | End-to-end compilation execution testing |

**Async Implementation Status (Concurrency Model)**:

| Submodule | Status | Description |
|-----------|--------|-------------|
| spawn Keyword Parsing | ✅ Completed | Lexer/syntax analysis support |
| is_async Flag | ✅ Completed | AST/type system support |
| Async[T] Type Design | ✅ Completed | Design document completed |
| Scheduler Framework | ✅ Completed | Basic work-stealing implementation |
| Send/Sync Constraints | ✅ Completed | Type constraint design document |
| IR Extension | 🔄 In Progress | CallAsync instruction defined |
| VM Async Instructions | 🔄 In Progress | Instruction framework defined |
| Complete Implementation | ⏳ To Start | v0.5 milestone |

---

## 6. How to Contribute

YaoXiang is a language born from the community, growing in the community, and serving the community. We sincerely invite every developer passionate about programming language design to join this exploration journey.

### 6.1 Design Discussions

**Target Audience**: Programming language theory researchers, type system enthusiasts, language design enthusiasts

**Participation Methods**:

- **GitHub Discussions**: Participate in discussions in the "Language Design" category
- **Design Proposals (RFC)**: Propose design documents for new features, following templates in the `rfcs/` directory
- **Syntax Reviews**: Propose improvements or discover potential issues in existing syntax design

| **Current Hot Topics**: |
| |
| - Design and implementation of macro systems |
| - Interface type mechanism |
| - Error handling syntax optimization |
| - Standard library API design |

**Submitting Design Proposals**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros/cons analysis, alternatives)
3. Create a Pull Request for community review
4. After core team review, merge or reject

### 6.2 Compiler Implementation

**Target Audience**: Compiler developers, system programmers, performance optimization experts

**Current Implementation Priorities** (by priority):

| Priority | Module | Description | Difficulty |
|----------|--------|-------------|------------|
| P0 | **Bytecode VM** | VM instruction perfection, performance optimization | Medium |
| P0 | **Runtime Memory** | GC implementation, memory allocator | High |
| P0 | **Async Runtime** | Complete concurrency model implementation | High |
| P1 | Standard Library | IO, String, List, Concurrent | Medium |
| P1 | JIT Compiler | Cranelift Integration | High |
| P2 | AOT Compiler | LLVM/Cranelift backend | High |
| P3 | Self-hosted Compiler | Rewrite in YaoXiang | Very High |

**Technology Stack**:

- **Implementation Language**: Rust (current phase)
- **Code Generation**: Cranelift or LLVM
- **Build Tool**: Cargo
- **Test Framework**: Rust `#[test]` + `cargo nextest`

**Getting Started**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand architecture design
2. Choose modules of interest under `src/`
3. Check `tests/unit/` to understand testing requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Target Audience**: IDE plugin developers, toolchain enthusiasts, efficiency tool seekers

**Tools to Develop**:

| Tool | Status | Description |
|------|--------|-------------|
| **LSP Server** | ⏳ To Start | Language Server Protocol support |
| **Debugger Integration** | ⏳ To Start | GDB/LLDB integration |
| **Formatter** | ⏳ To Start | `yaoxiang fmt` |
| **Package Manager** | ⏳ To Start | Dependency management, version resolution |
| **Package Registry** | ⏳ To Start | Centralized or decentralized |
| **REPL** | ⏳ To Start | Interactive interpreter |
| **Benchmark Tool** | ⏳ To Start | Performance analysis |
| **VS Code Plugin** | ⏳ To Start | Syntax highlighting, completion, debugging |
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

**Target Audience**: Library developers, API designers, domain experts

**Standard Library Module Planning**:

| Module | Priority | Description |
|--------|----------|-------------|
| `std.io` | P0 | File IO, console input/output |
| `std.string` | P0 | String operations, formatting |
| `std.list` | P0 | List/array operations |
| `std.dict` | P0 | Dictionary/hash table |
| `std.math` | P0 | Math functions, constants |
| `std.time` | P1 | Time/date operations |
| `std.net` | P1 | Network programming, HTTP |
| `std.concurrent` | P1 | Concurrency primitives, channels |
| `std.crypto` | P2 | Cryptographic hash, signatures |
| `std.json` | P1 | JSON parsing/generation |
| `std.regex` | P2 | Regular expressions |
| `std.database` | P3 | Database connection |
| `std.gui` | P3 | Graphical interface (long-term) |

**Design Principles**:

- Consistency: Functions with the same functionality have consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoid over-engineering
- Performance: Standard library functions should be efficient, avoid unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Target Audience**: Technical writers, educators, community managers

**Documentation to Contribute**:

| Document | Status | Description |
|----------|--------|-------------|
| Quick Start | ✅ Completed | 5-minute getting started guide |
| Language Guide | ✅ Completed | Systematically learn core concepts |
| Language Specification | ✅ Completed | Complete syntax and semantics definition |
| Implementation Plan | ✅ Completed | Compiler implementation technical details |
| API Documentation | ⏳ To Start | Standard library API reference |
| Tutorials | ⏳ To Start | Advanced tutorials and best practices |
| Blog | ⏳ To Start | Technical articles and design stories |
| Translation | ⏳ To Start | Multi-language support |

### 6.6 Community Building

**Target Audience**: Community managers, event organizers, evangelists

**Community Events**:

- Regular online Meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution Sprints (quarterly)
- Offline gatherings and conference talks

**Communication Channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Bug reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guide

**How to Start Contributing**:

1. **Understand the Project**: Read README and design documents
2. **Choose a Direction**: Select contribution area based on interest
3. **Set Up Environment**: Rust 1.75+, cargo, git
4. **Find Tasks**: Check GitHub Issues with `good first issue` label
5. **Submit PR**: Follow submission guidelines, write tests
6. **Participate in Reviews**: Review others' code, participate in discussions

**Submission Guidelines**:

```bash
# Commit Message Format
<type>(<scope>): <subject>

# Types
feat: New feature
fix: Bug fix
docs: Documentation updates
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
- Ensure `cargo clippy` has no warnings
- Write necessary unit tests
- Update relevant documentation

---

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword | Function |
|---------|----------|
| `type` | Type definition |
| `pub` | Public export |
| `use` | Import module |
| `spawn` | Concurrent marker (RFC-003: `spawn (params) => body`) |
| `ref` | Shared pointer (Arc) |
| `mut` | Mutable variable |
| `if/elif/else` | Conditional branches |
| `match` | Pattern matching |
| `while/for` | Loops |
| `return/break/continue` | Control flow |
| `as` | Type conversion |
| `in` | Membership test/list comprehension |
| `unsafe` | Unsafe code block (raw pointer) |

### A.2 Annotations

| Annotation | Function |
|------------|----------|
| `@block` | Mark function to execute synchronously |
| `@eager` | Mark expression to be eagerly evaluated |
| `@Send` | Explicitly declare satisfaction of Send constraint |
| `@Sync` | Explicitly declare satisfaction of Sync constraint |

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

| Precedence | Operator | Associativity |
|------------|----------|---------------|
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

## Appendix B: Design Inspiration

YaoXiang's design draws on excellent ideas from the following languages and projects:

| Source | Inspiration Points |
|--------|-------------------|
| **Rust** | Ownership model, zero-cost abstraction, type system |
| **Python** | Syntax style, readability, list comprehensions |
| **Idris/Agda** | Dependent types, type-driven development |
| **TypeScript** | Type annotations, runtime types |
| **MoonBit** | AI-friendly design, concise syntax |
| **Haskell** | Pure functional, pattern matching |
| **OCaml** | Type inference, variant types |

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have compared to Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstraction but adopts simpler syntax and lower cognitive burden. The **concurrency model** is more concise than Rust's `async/await`—only one `spawn` marker is needed, no manual management of Future and Pin. "All things grow together, and I observe their return," making concurrent programming as intuitive as describing natural laws. Send/Sync constraints provide equal thread safety guarantees. Unified type syntax eliminates `enum`/`struct`/`union` concept fragmentation.

**Q: What types of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing confusion from nesting depth. This is a thoughtfully considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: Production-ready. Release time depends on implementation progress, see [Version Planning RFC](./rfc/003-version-planning.md).

**Q: How to contact the core team?**

A: Through GitHub Discussions or Discord community channel. Core team members respond regularly.

---

> **Last Updated**: 2025-01-17
>
> **Document Version**: v1.2.0
>
> **License**: [MIT](LICENSE)

---

> "YaoXiang changes, all things are born. Types evolve, programs are formed."
>
> May the journey of YaoXiang's design be with you.
