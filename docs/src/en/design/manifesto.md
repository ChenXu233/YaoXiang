# YaoXiang Design Manifesto

> **Version**: v1.2.0
> **Status**: Official Release
> **Authors**: ChenXu + YaoXiang Community
> **Date**: 2025-01-17

---

> 「The Tao gave birth to the One, the One gave birth to the Two, the Two gave birth to the Three, and the Three gave birth to all things.」
> —— *Tao Te Ching*
>
> Types are like the Tao, from which all things are born.

---

## I. Why Create YaoXiang?

### 1.1 The Language Gap

In the long history of programming languages, we have witnessed the birth and evolution of many excellent languages: C brought an efficiency revolution to systems programming, Python created a programming experience accessible to everyone, Rust proved that memory safety and performance can coexist, and TypeScript made large frontend projects maintainable. However, when we examine today's language ecosystem, we still find a clear gap——**no single language can simultaneously satisfy these three core requirements**:

| Requirement | Problems with Existing Solutions |
|-------------|----------------------------------|
| **Type Safety** | Rust is too strict with a steep learning curve; TypeScript uses optional types and cannot provide compile-time guarantees |
| **Natural Syntax** | Rust syntax is complex and obscure; Haskell has a high barrier to functional programming; traditional static languages are verbose |
| **AI-Friendly** | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behaviors that limit AI accuracy in code generation and modification |

The creation of YaoXiang is precisely to fill this gap. We believe: **Programming languages should be both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Practical Problems Solved

**Problem One: Fragmentation of Type Systems**

Today's programming languages exhibit severe fragmentation in their type systems. Statically typed languages pursue absolute correctness at compile time but often at the cost of development efficiency; dynamically typed languages provide flexibility but reveal maintainability defects in large projects. YaoXiang proposes a unified abstract framework of "Everything is a Type," making types the thread running through the language design, not patches added after the fact.

**Problem Two: The Binary Choice Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime performance. GC (garbage collection) frees developers but introduces latency spikes and memory overhead; manual memory management is efficient but as dangerous as walking a tightrope. YaoXiang adopts Rust's ownership model, eliminating data races and memory leaks at compile time while maintaining zero-cost abstraction and achieving high performance without GC.

**Problem Three: Cognitive Burden of Asynchronous Programming**

Modern applications are inseparable from networking and concurrency, and asynchronous programming has always been a nightmare for programmers. Callback nesting, Promise chaining, async/await syntax——each approach increases code complexity. YaoXiang redesigns the async model: simply add a `spawn` marker after the function signature, and the compiler automatically handles all async details, making concurrent programming as natural as synchronous code.

**Problem Four: Bottleneck in AI-Assisted Programming**

When AI begins to assist developers in writing code, the choices in language design become crucial. Fuzzy syntax rules, implicit type conversions, complex syntactic sugar——these are features that human programmers have become accustomed to, but they become obstacles for AI to understand and generate. YaoXiang set "AI-friendly" as a core goal from the very beginning: strict indentation rules, clear code block boundaries, and unambiguous syntax structures enable AI to accurately understand, generate, and modify code.

### 1.3 Philosophical Roots of the Language

The name YaoXiang comes from the "Yao" (爻) and "Xiang" (象) in the I Ching (Book of Changes). "Yao" is the basic symbol that composes hexagrams, symbolizing the transformation of yin and yang, the interaction of motion and stillness; "Xiang" is the external manifestation of the essence of things, representing all phenomena and the totality of existence.

This philosophical thinking is reflected in every detail of the language design:

- **Unity**: Just as simple symbols of hexagrams compose complex patterns, YaoXiang builds a complete programming model with a few core concepts (types, functions, constructors)
- **Hierarchy**: Just as Xiang has the distinction between prior and later heaven, YaoXiang's type system has a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Variability**: Just as yin and yang flow and transform endlessly, YaoXiang supports dependent types, allowing types to evolve as values change
- **Identifiability**: Just as hexagrams can be interpreted and all things can be manifested, YaoXiang provides complete type reflection capabilities, with runtime type information fully available
- **Provability**: Just as hexagrams reveal the laws of things, YaoXiang's type system follows the Curry-Howard isomorphism (types are propositions, programs are proofs), making type checking the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Every feature proposal must pass examination by these principles.

### 2.1 Principle One: Everything is a Type

In YaoXiang's worldview, types are the highest-level abstract units, the core concept running through the language.

**Specific Manifestations**:

- **Values are instances of types**: `42` is an instance of type `Int`, `"hello"` is an instance of type `String`
- **Types themselves are also types**: Meta types (`type`) are the types of all types, `Int` is an instance of `type`
- **Functions are type mappings**: `fn add(Int, Int) -> Int` describes a type mapping from `Int × Int` to `Int`
- **Modules are type compositions**: Modules are named namespace compositions containing functions and types

**Non-negotiable reason**: Unified type abstraction simplifies language semantics, eliminates the binary opposition between values and types, and makes the type system the guardian of code correctness rather than a stumbling block.

### 2.2 Principle Two: Strictly Structured

YaoXiang's syntax design pursues "unambiguous, predictable, and easy to parse."

**Specific Rules**:

- **Mandatory 4-space indentation**: Tab characters are prohibited, code block boundaries are clear at a glance
- **Parentheses cannot be omitted**: Function parameters must have parentheses, list elements must have commas
- **Code blocks must use curly braces**: Control flow like `if`, `while`, `for` must use `{ }` to wrap
- **Streamlined keyword count**: Only 17 core keywords are retained, syntactic sugar proliferation is refused

**Non-negotiable reason**: Strict structure brings three key advantages——(1) more accurate IDE syntax highlighting and code folding; (2) significantly improved AI code generation and modification accuracy; (3) newcomers can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstraction

High-level abstractions should not incur runtime performance overhead.

**Specific Guarantees**:

- **Monomorphization**: Generic functions are expanded into concrete versions at compile time, no vtable lookup overhead
- **Inline optimization**: Simple functions are automatically inlined, eliminating function call overhead
- **Stack allocation by default**: Small objects are stack-allocated by default, heap allocation is only used when necessary
- **No GC**: The ownership model guarantees memory safety without garbage collector runtime overhead

**Non-negotiable reason**: Performance is the survival底线 of programming languages. Any design that sacrifices performance for convenience is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity are inseparable companions. YaoXiang chooses immutable by default, making code easier to reason about and understand.

**Specific Rules**:

- Variables are immutable by default and cannot be modified after assignment
- `mut` must be explicitly declared when mutability is needed
- References are immutable by default, mutable references require the `mut` marker
- Transfer of ownership means the original binding becomes invalid

**Non-negotiable reason**: Immutability is the foundation of concurrency safety, the guarantee of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types as Data

Type information should not only exist at compile time but should be fully available at runtime.

**Specific Capabilities**:

- Runtime type querying: Any value can obtain its type information
- Type reflection: Types themselves can be constructed and manipulated
- Pattern matching and destructuring: Type constructors can be directly used in pattern matching
- Generic specialization: Runtime can obtain the instantiated types of generic parameters

**Non-negotiable reason**: Complete type reflection capability is the foundation of metaprogramming, the cornerstone of high-performance frameworks and tools.

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
union IntOrFloat { i: i32, f: f32 }
```

**YaoXiang's unified syntax**: Only the `type` keyword, `{}` for data structures, `[]` for interface types.

```yaoxiang
# === Data Types (curly braces) ===

# Struct
type Point = { x: Float, y: Float }

# Enum (multiple constructors)
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Zero-argument constructor
type Color = { red | green | blue }

# Sum type
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

**Innovation Value**: Unified type syntax, eliminating the fragmentation of `enum`/`struct`/`union`/`trait` keywords.

### 3.2 Innovation Two: Constructors as Types

**Value construction and pattern matching are completely identical**:

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

### 3.3 Innovation Three: Curried Method Binding

YaoXiang adopts a pure functional design, implementing method-call-like syntax sugar through currying, without introducing `class` and `method` keywords.

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

    # Both calling styles are completely equivalent
    d1 = distance(p1, p2)      # Direct core function call
    d2 = p1.distance(p2)       # Method syntax sugar

    # Functional usage: pre-bind the first argument
    dist_from_origin = Point.distance(Point(0.0, 0.0))
    result = dist_from_origin(p1)   # 5.0

    # Currying usage: delayed evaluation
    get_dist_to_p2 = p2.distance(_)
    d3 = get_dist_to_p2(p1)         # 2.828
}
```

**Innovation Value**: Pure functional design, no hidden `self` parameter, functions as values that can be freely passed and composed.

### 3.4 Innovation Four: Spawn Model

> 「All things rise and act together; I observe their return.」——《I Ching: Fu (Return)》
>
> The spawn model derives its meaning from this, describing a programming paradigm: developers describe logic with synchronous, sequential thinking, while the language runtime makes computational units automatically and efficiently execute concurrently like all things rising together, ultimately unifying and coordinating at the end.

**Three Core Principles**:

| Principle | Description |
|-----------|-------------|
| **Synchronous Syntax** | What you see is what you get sequential code |
| **Concurrent Essence** | Automatic parallelism extraction at runtime |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology**:

| Official Term | Corresponding Syntax | Explanation |
|--------------|---------------------|-------------|
| **spawn function** | `spawn (params) => body` | Defines a computational unit that can participate in concurrent execution |
| **spawn block** | `spawn { a(), b() }` | Explicitly declared concurrent scope, tasks within the block execute concurrently |
| **spawn loop** | `spawn for x in xs { ... }` | Data parallelism, loop body executes concurrently on all elements |
| **spawn value** | `Async(T)` | A future value currently in concurrent execution, automatically awaited when used |
| **spawn graph** | Lazy evaluation graph (DAG) | The stage where concurrency happens, describing dependencies and parallelism |
| **spawn scheduler** | Runtime task scheduler | The intelligent coordinator that coordinates all things, letting them act together at the right moment |

> **See also**: [RFC-001 Spawn Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === Spawn Function ===
# Function marked with spawn (RFC-003 syntax)
fetch_data: String -> JSON = spawn (url) => {
    HTTP.get(url).json()
}

# === Spawn Block ===
# Expressions inside spawn { } are forced to execute in parallel
compute_all: () -> (Int, Int, Int) = spawn () => {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    (a, b, c)
}

# === Spawn Loop ===
# Loops marked with spawn for are automatically parallelized
parallel_sum: Int -> Int = spawn (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)  # Each iteration executes in parallel
    }
    total
}

# === Automatic Waiting ===
main() -> Void = () => {
    # Two independent requests execute in parallel automatically
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # Wait points are automatically inserted when results are needed
    print(users.length + posts.length)  # Automatically await users and posts
}
```

**Thread Safety**:

```yaoxiang
# Send/Sync constraints guarantee compile-time thread safety
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

**Innovation Value**: The cognitive burden of asynchronous programming is reduced to zero, code readability is completely identical to synchronous code, while achieving high-performance parallel execution efficiency.

### 3.5 Innovation Five: Dependent Type Support (Future Feature)

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

**Innovation Value**: Lower memory burden, more consistent syntax style, easier-to-parse syntax structure.

---

## IV. Preliminary Syntax Preview

The following code examples demonstrate YaoXiang's language style, helping you quickly experience its design aesthetics.

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
p2 = p              # Move, ownership transfer, p invalid

# Explicit ref = Arc (safe sharing)
shared = ref p      # Arc, thread-safe

spawn(() => print(shared.x))   # ✅ Safe

# Explicit clone() = copy
p3 = p.clone()      # p and p3 are independent
```

**Core Rules**:
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

# Data parallel loop
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

The following decisions have been thoroughly discussed and reviewed, **no longer accepting changes**:

| Module | Decision | Description |
|--------|----------|-------------|
| **Type System** | Everything is a Type | Values, functions, modules, and generics are all types |
| **Type Syntax** | Unified curly brace syntax | `{}` for data types, `[]` for interface types |
| **Indentation Rules** | 4-space indentation | Mandatory requirement, Tab prohibited |
| **Keywords** | 17 core keywords | As listed in the table above |
| **Function Syntax** | Arrow function syntax | `name: (Type1, Type2) -> Type = (params) => body` |
| **Method Binding** | RFC-004 curried binding | `Type.method = function` (default binding to position 0) |
| **Async Model** | RFC-003 spawn model | `spawn (params) => body`, lazy evaluation, automatic parallelism |
| **Memory Management** | Ownership model | No GC, compile-time safety guarantees |
| **File as Module** | Module system | Each `.yx` file is a module |
| **Main Function** | `main: () -> Void` | Program entry point |
| **Thread Safety** | Send/Sync constraints | Compile-time elimination of data races |

### 5.2 Pending Design Topics

The following topics are still under discussion, **community contributions welcome**:

| Topic | Current Status | Open Questions |
|-------|----------------|-----------------|
| **Literal Syntax** | Float support | Scientific notation support (`3.14e-10`)? |
| **Generic Inference** | Basic support | Return type generic inference? |
| **Macro System** | Not yet designed | Hygienic macros needed? Syntax design direction? |
| **Package Manager** | Not yet designed | Centralized package registry needed? Dependency resolution strategy? |
| **FFI** | Not yet designed | Specific plan for C interop? |
| **Generic Constraints** | Basic support | trait/bounds mechanism support? |
| **Reflection Depth** | Basic support | Private member access? |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          YaoXiang Implementation Roadmap                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: AOT │
│        ✅ Completed                   │ (current phase)              Compiler │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←────── v1.0: YaoXiang JIT Compiler ←──── v2.0:│
│        (bootstrapping)               (bootstrapping)               YaoXiang AOT │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Milestone Details**:

| Version | Status | Goal | Deliverables |
|---------|--------|------|--------------|
| **v0.1** | ✅ Complete | Interpreter prototype | Basic interpreter, lexical analysis, syntax analysis, primitive types |
| **v0.2** | ✅ Complete | Full interpreter | Type checking, pattern matching, module system |
| **v0.3** | 🔄 In Progress | Bytecode generation | IR intermediate representation, bytecode generation, closure optimization, monomorphization |
| **v0.4** | 🔄 In Progress | Bytecode VM | VM core, instruction execution, call frame management, inline caches |
| **v0.5** | ⏳ Pending | Runtime system | GC, scheduler, standard library IO |
| **v1.0** | ⏳ Pending | AOT compiler | Full optimization, native code generation |
| **v2.0** | ⏳ Pending | Bootstrapping compiler | New compiler written in YaoXiang |

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
| **Bytecode VM** | 🔄 In Progress | 70% | VM core, instruction execution, call frames, inline caches |
| **Runtime Scheduler** | 🔄 In Progress | 60% | Task descriptors, work-stealing queues, wait queues |
| **Runtime Memory** | 🔄 In Progress | 50% | Memory allocator, GC framework |
| **Standard Library** | 🔄 In Progress | 30% | IO, String, List, Dict, Math, Concurrent |
| **JIT Compiler** | ⏳ Pending | 0% | Pending integration of Cranelift/LLVM |
| **AOT Compiler** | ⏳ Pending | 0% | Pending implementation |

**Code Generation Module Details**:

| Sub-module | Status | Key Features |
|------------|--------|---------------|
| Expression generation | ✅ Complete | Arithmetic, comparison, logic, function calls |
| Statement generation | ✅ Complete | Assignment, return, conditionals, loops |
| Control flow generation | ✅ Complete | Switch pattern matching, loop unrolling |
| Closure handling | ✅ Complete | Environment capture, closure conversion |
| Bytecode serialization | ✅ Complete | Bytecode read/write, test cases |
| Generator code generation | ✅ Complete | yield syntax support, state machine conversion |
| Integration tests | ✅ Complete | End-to-end compilation execution tests |

**Async Implementation Status (spawn model)**:

| Sub-module | Status | Description |
|------------|--------|-------------|
| spawn keyword parsing | ✅ Complete | Lexer/parser support |
| is_async flag | ✅ Complete | AST/type system support |
| Async(T) type design | ✅ Complete | Design documentation complete |
| Scheduler framework | ✅ Complete | Basic work-stealing implementation |
| Send/Sync constraints | ✅ Complete | Type constraint design documentation |
| IR extension | 🔄 In Progress | CallAsync instruction defined |
| VM async instructions | 🔄 In Progress | Instruction framework defined |
| Full implementation | ⏳ Pending | v0.5 milestone |

---

## VI. How to Contribute

YaoXiang is a language born from the community, grown by the community, and serving the community. We sincerely invite every developer who loves programming language design to join this journey of exploration.

### 6.1 Design Discussion

**Suitable for**: Programming language theory researchers, type system enthusiasts, language design enthusiasts

**How to participate**:

- **GitHub Discussions**: Participate in discussions in the "Language Design" category
- **Design Proposals (RFC)**: Propose design documents for new features, following the template under `rfcs/` directory
- **Syntax Review**: Propose improvements or identify potential issues with existing syntax design

| **Current Hot Topics**: | |
| | |
| - Macro system design and implementation | |
| - Interface type mechanism | |
| - Error handling syntax optimization | |
| - Standard library API design | |

**Submitting a design proposal**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros and cons analysis, alternative solutions)
3. Submit a Pull Request for community review
4. After core team review, merge or reject

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, systems programmers, performance optimization experts

**Current implementation priorities** (in order of priority):

| Priority | Module | Description | Difficulty |
|----------|--------|-------------|------------|
| P0 | **Bytecode VM** | VM instruction completion, performance optimization | Medium |
| P0 | **Runtime Memory** | GC implementation, memory allocator | High |
| P0 | **Async Runtime** | Full spawn model implementation | High |
| P1 | Standard Library | IO, String, List, Concurrent | Medium |
| P1 | JIT Compiler | Cranelift integration | High |
| P2 | AOT Compiler | LLVM/Cranelift backend | High |
| P3 | Bootstrapping Compiler | Rewrite in YaoXiang | Extremely High |

**Tech stack**:

- **Implementation language**: Rust (current phase)
- **Code generation**: Cranelift or LLVM
- **Build tool**: Cargo
- **Testing framework**: Rust `#[test]` + `cargo nextest`

**Starting to contribute**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand the architecture
2. Choose a module of interest under `src/`
3. Read `tests/unit/` to understand testing requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, efficiency tool seekers

**Tools that need to be developed**:

| Tool | Status | Description |
|------|--------|-------------|
| **LSP Server** | ⏳ Pending | Language Server Protocol support |
| **Debugger Integration** | ⏳ Pending | GDB/LLDB integration |
| **Formatter** | ⏳ Pending | `yaoxiang fmt` |
| **Package Manager** | ⏳ Pending | Dependency management, version resolution |
| **Package Registry** | ⏳ Pending | Central or decentralized |
| **REPL** | ⏳ Pending | Interactive interpreter |
| **Benchmark Tool** | ⏳ Pending | Performance analysis |
| **VS Code Plugin** | ⏳ Pending | Syntax highlighting, completion, debugging |
| **Vim/Neovim Plugin** | ⏳ Pending | Syntax highlighting, LSP client |

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

**Standard library module plan**:

| Module | Priority | Description |
|--------|----------|-------------|
| `std.io` | P0 | File IO, console input/output |
| `std.string` | P0 | String operations, formatting |
| `std.list` | P0 | List/array operations |
| `std.dict` | P0 | Dictionary/hash table |
| `std.math` | P0 | Math functions, constants |
| `std.time` | P1 | Date/time operations |
| `std.net` | P1 | Network programming, HTTP |
| `std.concurrent` | P1 | Concurrency primitives, channels |
| `std.crypto` | P2 | Cryptographic hashing, signatures |
| `std.json` | P1 | JSON parsing/generation |
| `std.regex` | P2 | Regular expressions |
| `std.database` | P3 | Database connections |
| `std.gui` | P3 | Graphical interface (long-term) |

**Design principles**:

- Consistency: Functions with the same functionality should have consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoiding overdesign
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documentation needed**:

| Document | Status | Description |
|----------|--------|-------------|
| Quick Start | ✅ Complete | 5-minute getting started guide |
| Language Guide | ✅ Complete | Systematic learning of core concepts |
| Language Specification | ✅ Complete | Complete syntax and semantics definition |
| Implementation Plan | ✅ Complete | Compiler implementation technical details |
| API Documentation | ⏳ Pending | Standard library API reference |
| Tutorials | ⏳ Pending | Advanced tutorials and best practices |
| Blog | ⏳ Pending | Technical articles and design stories |
| Translation | ⏳ Pending | Multi-language support |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

**Community activities**:

- Regular online Meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution Sprints (quarterly)
- Offline gatherings and conference talks

**Communication channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Bug reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guidelines

**How to start contributing**:

1. **Understand the project**: Read README and design documents
2. **Choose a direction**: Select a contribution area based on your interests
3. **Set up environment**: Rust 1.75+, cargo, git
4. **Find tasks**: Check GitHub Issues for `good first issue` tags
5. **Submit PR**: Follow commit conventions, write tests
6. **Participate in review**: Review others' code, participate in discussions

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
perf: Performance optimization
test: Testing
chore: Build tools or auxiliary tools

# Examples
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**Code style**:

- Follow `rustfmt.toml` conventions
- Ensure `cargo clippy` produces no warnings
- Write necessary unit tests
- Update related documentation

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
| `@block` | Marks that the function will be executed synchronously |
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
| `Float` | Floating point | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | Raw bytes | Variable |

### A.4 Operator Precedence

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
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

YaoXiang's design draws on the excellent ideas of the following languages and projects:

| Source | Borrowed Points |
|--------|-----------------|
| **Rust** | Ownership model, zero-cost abstraction, type system |
| **Python** | Syntax style, readability, list comprehensions |
| **Idris/Agda** | Dependent types, type-driven development |
| **Curry-Howard Isomorphism** | Types are propositions, programs are proofs, unified theory of type systems and logic |
| **TypeScript** | Type annotations, runtime types |
| **MoonBit** | AI-friendly design, concise syntax |
| **Haskell** | Pure functional, pattern matching |
| **OCaml** | Type inference, variant types |

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have compared to Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstraction but uses simpler syntax and lower cognitive burden. The **spawn model** is more concise than Rust's `async/await`——just one `spawn` marker, no manual Future and Pin management. 「All things rise and act together; I observe their return」, making concurrent programming as intuitive as describing natural laws. Send/Sync constraints provide equivalent thread safety guarantees. Unified type syntax eliminates the conceptual fragmentation of `enum`/`struct`/`union`.

**Q: What types of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing confusion caused by nesting depth. This is a carefully considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. Release time depends on implementation progress. See [Version Planning RFC](./rfc/003-version-planning.md).

**Q: How to contact the core team?**

A: Through GitHub Discussions or Discord community channels. Core team members respond regularly.

---

> **Last Updated**: 2025-01-17
>
> **Document Version**: v1.2.0
>
> **License**: [MIT](LICENSE)

---

> 「The transformation of YaoXiang gives birth to all things. The evolution of types completes the program.」
>
> May the journey of YaoXiang's design walk alongside you.