# YaoXiang Design Manifesto

> **Version**: v2.0.0
> **Status**: Official Release
> **Authors**: Chen Xu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Tao gives birth to the One, the One gives birth to the Two, the Two gives birth to the Three, the Three gives birth to all things."
> — *Dao De Jing*
>
> Types are like the Tao, from which all things are born.

---

## I. Why Create YaoXiang?

### 1.1 The Language Gap

In the long history of programming languages, we have witnessed the birth and evolution of many excellent languages: C brought an efficiency revolution in systems programming, Python created a programming experience accessible to everyone, Rust proved that memory safety and performance can coexist, and TypeScript made large-scale frontend projects maintainable. However, when we examine today's language ecosystem, we still find a clear gap—**no single language can simultaneously satisfy these three core requirements**:

| Requirement | Problems with Existing Solutions |
|-------------|----------------------------------|
| **Type Safety** | Rust is overly strict with a steep learning curve; TypeScript uses optional types and cannot provide compile-time guarantees |
| **Natural Syntax** | Rust has complex and obscure syntax; Haskell has a high barrier to entry for functional programming; traditional statically typed languages are verbose and cumbersome |
| **AI-Friendly** | Existing languages have syntactic ambiguities, complex ASTs, and unpredictable hidden behaviors, limiting AI accuracy in code generation and modification |

The birth of YaoXiang is precisely to fill this void. We believe: **Programming languages should be both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 Practical Problems to Solve

**Problem One: Fragmentation of Type Systems**

Today's programming languages exhibit severe fragmentation in type systems. Statically typed languages pursue absolute compile-time correctness, but often at the cost of developer productivity; dynamically typed languages provide flexibility but reveal maintainability defects in large projects. YaoXiang proposes a unified abstract framework of "Everything is a Type," making types the central thread running through the language design, rather than patches added after the fact.

**Problem Two: The Binary Choice Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime performance. Garbage Collection (GC) liberates developers but introduces latency jitter and memory overhead; manual memory management is efficient but as dangerous as walking a tightrope. YaoXiang adopts Rust's ownership model, eliminating data races and memory leaks at compile time while maintaining zero-cost abstractions and achieving high performance without GC.

**Problem Three: The Cognitive Burden of Async Programming**

Modern applications are inseparable from networking and concurrency, yet asynchronous programming has always been a nightmare for programmers. Nested callback functions, Promise chaining, async/await syntax—each solution adds complexity to the code. YaoXiang has redesigned the async model: simply add a `spawn` marker after a function signature, and the compiler automatically handles all async details, making concurrent programming as natural as writing synchronous code.

**Problem Four: Bottlenecks in AI-Assisted Programming**

When AI begins to assist developers in writing code, the choices made in language design become crucial. Fuzzy syntax rules, implicit type conversions, complex syntactic sugar—these are features that human programmers have grown accustomed to, yet they become obstacles for AI to understand and generate. YaoXiang made "AI-friendly" a core goal from the very beginning: strict indentation rules, clear code block boundaries, and unambiguous syntactic structures, enabling AI to accurately understand, generate, and modify code.

### 1.3 The Philosophical Foundation of the Language

The name YaoXiang comes from "Yao" and "Xiang" in the *I Ching*. "Yao" are the basic symbols that compose hexagrams, symbolizing the interplay of yin and yang, motion and stillness; "Xiang" is the external manifestation of the essence of things, representing all phenomena and encompassing all things.

This philosophical thought is reflected in every detail of the language design:

- **Unity**: Just as simple Yao symbols compose complex hexagrams, YaoXiang builds a complete programming model from a few core concepts (types, functions, constructors)
- **Hierarchy**: Just as Xiang has distinctions between prior and posterior heaven, YaoXiang's type system has a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Variability**: Just as yin and yang flow and change endlessly, YaoXiang supports dependent types, allowing types to evolve with values
- **Identifiability**: Just as hexagrams can be interpreted and all things can be symbolized, YaoXiang provides complete type reflection capabilities, with runtime type information fully available
- **Provability**: Just as hexagrams reveal the patterns of things, YaoXiang's type system follows the Curry-Howard isomorphism (types are propositions, programs are proofs), where type checking is the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Every feature proposal must pass the test of these principles.

### 2.1 Principle One: Everything is a Type

In YaoXiang's worldview, types are the highest-level abstract units and the core concept running through the language.

**Specific manifestations**:

- **Values are instances of types**: `42` is an instance of the `Int` type, `"hello"` is an instance of the `String` type
- **Types themselves are also types**: `Type` is the language's sole meta type keyword; the type of `Int` is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from `Int × Int` to `Int`
- **Modules are type compositions**: Modules are named compositions of namespaces containing functions and types

**Non-negotiable reason**: Unified type abstraction simplifies language semantics, eliminates the dualism between values and types, and makes the type system a guardian of code correctness rather than a stumbling block.

### 2.2 Principle Two: Strict Structuring

YaoXiang's syntax design pursues "unambiguous, predictable, and easy to parse."

**Specific rules**:

- **Mandatory 4-space indentation**: Tab characters are prohibited; code block boundaries are immediately clear
- **Parentheses cannot be omitted**: Function parameters must have parentheses, list elements must have commas
- **Code blocks must use curly braces**: Control flow structures like `if`, `while`, `for` must use `{ }` to enclose blocks
- **Streamlined keyword count**: Only 17 core keywords are retained; syntactic sugar proliferation is rejected

**Non-negotiable reason**: Strict structuring brings three key advantages—(1) IDE syntax highlighting and code folding are more accurate; (2) AI code generation and modification accuracy improves dramatically; (3) New learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not incur runtime performance overhead.

**Specific guarantees**:

- **Monomorphization**: Generic functions are expanded into concrete versions at compile time, with no vtable lookup overhead
- **Inlining optimization**: Simple functions are automatically inlined, eliminating function call overhead
- **Stack allocation by default**: Small objects are allocated on the stack by default; heap allocation is used only when necessary
- **No GC**: The ownership model guarantees memory safety without the runtime overhead of a garbage collector

**Non-negotiable reason**: Performance is the survival bottom line of programming languages. Any design that sacrifices performance for convenience is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability comes hand in hand with complexity. YaoXiang chooses immutable by default, making code easier to reason about and understand.

**Specific rules**:

- Variables are immutable by default; once assigned, they cannot be modified
- `mut` must be explicitly declared when mutability is needed
- References are immutable by default; mutable references require the `mut` marker
- Transfer of ownership means the original binding becomes invalid

**Non-negotiable reason**: Immutability is the foundation of concurrency safety, the guarantee of code readability, and the crystallization of functional programming wisdom.

### 2.5 Principle Five: Types as Data

Type information should not exist only at compile time; it should be fully available at runtime.

**Specific capabilities**:

- Runtime type queries: Any value can obtain its type information
- Type reflection: Types themselves can be constructed and manipulated
- Pattern matching destructuring: Type constructors can be directly used in pattern matching
- Generic specialization: Runtime access to instantiated types of generic parameters

**Non-negotiable reason**: Complete type reflection capability is the foundation of metaprogramming and the cornerstone of high-performance frameworks and tools.

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
trait Drawable { fn draw(&self, s: &Surface); }
```

**YaoXiang's unified syntax**: Everything is `name: type = value`, and `Type` is the sole meta type keyword.

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

# === Interfaces (records where all fields are function types) ===

Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

Serializable: Type = {
    serialize: () -> String,
}

# === Interface Implementation (interface name written inside the type body) ===

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

**Innovation value**: No fragmentation of keywords like `fn`, `struct`, `enum`, `trait`, `impl`—one unified syntax covers all declarations.

### 3.2 Innovation Two: Constructors are Types

**Value construction is exactly the same as function calls**:

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

YaoXiang adopts a pure functional design, implementing object-method-call-like syntax through currying, without introducing `class` and `method` keywords.

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

# Method syntax sugar binding ([0] means binding to parameter position 0)
Point.distance = distance[0]

# === Usage ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Both calling styles are completely equivalent
d1 = distance(p1, p2)     # Direct call to core function
d2 = p1.distance(p2)      # Method syntax sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, waiting for second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovation value**: Pure functional design, no hidden `self` parameter, functions are values that can be freely passed and composed.

### 3.4 Innovation Four: Spawn Model

> "All things arise together, I observe their return." — *I Ching, Hexagram Fu*
>
> The spawn model draws its meaning from this, describing a programming paradigm: developers describe logic with synchronous, sequential thinking, while the language runtime causes the computational units within to automatically and efficiently execute concurrently like all things arising together, ultimately unifying and coordinating the results.

**Three core principles**:

| Principle | Description |
|-----------|-------------|
| **Synchronous Syntax** | Sequential code that is what you see is what you get |
| **Concurrent Nature** | Automatically extract parallelism at runtime |
| **Unified Coordination** | Results automatically converge when needed, ensuring logical correctness |

**Terminology**:

| Official Term | Corresponding Syntax | Explanation |
|---------------|----------------------|-------------|
| **Spawn Function** | `spawn (params) => body` | Defines a computational unit that can participate in spawn execution |
| **Spawn Block** | `spawn { a(), b() }` | Explicitly declared concurrent scope; tasks within the block execute concurrently |
| **Spawn Loop** | `spawn for x in xs { ... }` | Data parallelism; the loop body executes concurrently across all elements |
| **Async Value** | `Async(T)` | A future value currently in concurrent execution; automatically awaited when used |
| **Spawn Graph** | Lazy computation graph (DAG) | The stage where spawn occurs; describes dependencies and parallelism relationships |
| **Spawn Scheduler** | Runtime task scheduler | The intelligent coordinator that orchestrates all things, letting them spawn at the right moments |

> **See also**: [RFC-001 Spawn Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === Spawn Function ===
# Function marked with spawn
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === Spawn Block ===
# Expressions inside spawn { } execute in parallel
compute_all: () -> (Int, Int, Int) spawn = {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    return (a, b, c)
}

# === Automatic Waiting ===
main: () -> Void = {
    # Two independent requests automatically execute in parallel
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # Wait points are automatically inserted where results are needed
    print(users.length + posts.length)  # Automatically await users and posts
}
```

**Thread Safety**:

```yaoxiang
# The ref keyword automatically handles thread safety (compiler automatically selects Rc/Arc)
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
- See [RFC-001 Spawn Model](./rfc/accepted/001-concurrent-model-error-handling.md)

**Innovation value**: The cognitive burden of asynchronous programming drops to zero; code readability is completely identical to synchronous code, while achieving high-performance parallel execution efficiency.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, enabling true type-driven development.

```yaoxiang
# Matrix type: dimensions determined at compile time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time computation: factorial(3) = 6
vec: Vec(factorial(3)) = Vec(6)()

# Compile-time dimension verification
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimension mismatch
```

**Innovation value**: Catch more errors at compile time and achieve more precise type guarantees.

### 3.6 Innovation Six: Minimalist Keyword Design

YaoXiang defines only 17 core keywords, far fewer than mainstream languages:

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

| Language Comparison | Keyword Count |
|---------------------|---------------|
| YaoXiang | **17** |
| Rust | 51+ |
| Python | 35 |
| TypeScript | 64+ |
| Go | 25 |

**Innovation value**: Lower memory burden for learning, more consistent syntax style, easier-to-parse syntactic structure.

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
p2 = p1              # Move; p1 cannot be read anymore

# &T / &mut T tokens (compile-time zero overhead)
p2.print()           # Compiler automatically creates &Point token
p2.shift(1.0, 1.0)  # Compiler automatically creates &mut Point token

# ref: shared ownership (compiler automatically selects Rc/Arc)
shared = ref p2      # Cross-scope sharing

# clone(): explicit deep copy
backup = p2.clone()

# unsafe + raw pointers: systems level
unsafe {
    ptr: *Point = &p2
    (*ptr).x = 0.0
}
```

**Ownership gradient**:
```
&T / &mut T    Move       ref        clone()    unsafe
    |             |          |           |          |
Borrow token   Default  Shared owned   Deep copy  Raw pointer
Zero-cost      Zero-copy Auto Rc/Arc  Explicit   Systems-level
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

## V. Roadmap and Pending Items

### 5.1 Decided Design Decisions

The following decisions have been thoroughly discussed and reviewed, **no longer accepting changes**:

| Module | Decision | Description |
|--------|----------|-------------|
| **Type System** | Everything is a Type | Values, functions, modules, and generics are all types |
| **Type Syntax** | Unified `name: type = value` | One declaration form covers all cases; `Type` is the sole meta type keyword |
| **Keywords** | 17 core keywords | Excludes `type`/`fn`/`struct`/`enum`/`trait`/`impl` |
| **Function Syntax** | Signature + expression | `name: (params) -> ReturnType = body` |
| **Method Binding** | RFC-004 curried binding | `Type.method = function[position]` |
| **Async Model** | Spawn model | `spawn` marker; lazy evaluation; automatic parallelism |
| **Memory Management** | Ownership model (RFC-009 v9) | Move + &T/&mut T tokens + ref + clone + unsafe; no GC |
| **File as Module** | Module system | Each `.yx` file is a module |
| **Main Function** | `main: () -> Void` | Program entry point |
| **Thread Safety** | ref auto-selects Rc/Arc | Compiler escape analysis; invisible to users |

### 5.2 Design Topics Under Discussion

The following topics are still under discussion; **community contributions are welcome**:

| Topic | Current Status | Open Questions |
|-------|----------------|----------------|
| **Literal Syntax** | Float support | Should scientific notation like `3.14e-10` be supported? |
| **Generic Inference** | Basic support | Should return-type generic inference be supported? |
| **Macro System** | Not yet designed | Are hygienic macros needed? What is the syntax design direction? |
| **Package Manager** | Not yet designed | Is a centralized package registry needed? What is the dependency resolution strategy? |
| **FFI** | Not yet designed | What is the specific plan for C interoperability? |
| **Generic Constraints** | Basic support | Is a trait/bounds mechanism to be supported? |
| **Reflection Depth** | Basic support | Is access to private members to be supported? |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           YaoXiang Implementation Roadmap                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ────────→ v0.5: Rust Compiler ────────→ v1.0: AOT  │
│        ✅ Completed                  │ (current phase)              Compiler   │
│                                      │                                      │
│                                      ▼                                      │
│  v0.6: YaoXiang Interpreter ←────── v1.0: YaoXiang JIT Compiler ←──── v2.0 │
│        (bootstrapping)               (bootstrapping)                 YaoXiang  │
│                                                                              AOT      │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Milestone details**:

| Version | Status | Goal | Deliverables |
|---------|--------|------|--------------|
| **v0.1** | ✅ Done | Interpreter prototype | Basic interpreter, lexer, parser, basic types |
| **v0.2** | ✅ Done | Full interpreter | Type checking, pattern matching, module system |
| **v0.3** | 🔄 In progress | Bytecode generation | IR intermediate representation, bytecode generation, closure optimization, monomorphization |
| **v0.4** | 🔄 In progress | Bytecode VM | VM core, instruction execution, call frame management, inline caching |
| **v0.5** | ⏳ Not started | Runtime system | GC, scheduler, standard library IO |
| **v1.0** | ⏳ Not started | AOT compiler | Full optimization, native code generation |
| **v2.0** | ⏳ Not started | Bootstrapping compiler | New compiler written in YaoXiang |

### 5.4 Current Implementation Status

| Module | Status | Completion | Description |
|--------|--------|------------|-------------|
| **Lexer** | ✅ Done | 100% | Token definition, keyword recognition, test cases |
| **Parser** | ✅ Done | 100% | AST definition, expression/statement parsing, boundary tests |
| **Type Checker** | ✅ Done | 95% | Type inference, monomorphization, generic specialization, error handling |
| **IR Intermediate Representation** | ✅ Done | 90% | IR instruction definition, type representation, control flow graph |
| **Bytecode Generation** | ✅ Done | 85% | Expression/statement/control flow bytecode, closure transformation |
| **Ownership System** | ✅ Done | 100% | Move semantics, Clone/Drop semantics, mutability checks, Send/Sync constraints |
| **Monomorphization** | ✅ Done | 100% | Generic instantiation, specialization implementation |
| **Escape Analysis** | 🔄 In progress | 40% | Basic framework, variable escape determination |
| **Bytecode VM** | 🔄 In progress | 70% | VM core, instruction execution, call frames, inline caching |
| **Runtime Scheduler** | 🔄 In progress | 60% | Task descriptors, work-stealing queues, wait queues |
| **Runtime Memory** | 🔄 In progress | 50% | Memory allocator, GC framework |
| **Standard Library** | 🔄 In progress | 30% | IO, String, List, Dict, Math, Concurrent |
| **JIT Compiler** | ⏳ Not started | 0% | Pending integration of Cranelift/LLVM |
| **AOT Compiler** | ⏳ Not started | 0% | Pending implementation |

**Code generation module details**:

| Submodule | Status | Key Features |
|-----------|--------|--------------|
| Expression generation | ✅ Done | Arithmetic, comparison, logic, function calls |
| Statement generation | ✅ Done | Assignment, return, conditionals, loops |
| Control flow generation | ✅ Done | Switch pattern matching, loop unrolling |
| Closure handling | ✅ Done | Environment capture, closure conversion |
| Bytecode serialization | ✅ Done | Bytecode read/write, test cases |
| Generator code generation | ✅ Done | yield syntax support, state machine transformation |
| Integration tests | ✅ Done | End-to-end compilation execution tests |

**Async implementation status (Spawn Model)**:

| Submodule | Status | Description |
|-----------|--------|-------------|
| spawn keyword parsing | ✅ Done | Lexer/parser support |
| is_async flag | ✅ Done | AST/type system support |
| Async(T) type design | ✅ Done | Design document completed |
| Scheduler framework | ✅ Done | Basic work-stealing implementation |
| Send/Sync constraints | ✅ Done | Type constraint design document |
| IR extensions | 🔄 In progress | CallAsync instruction defined |
| VM async instructions | 🔄 In progress | Instruction framework defined |
| Full implementation | ⏳ Not started | v0.5 milestone |

---

## VI. How to Contribute

YaoXiang is a language born from the community, growing through the community, and serving the community. We sincerely invite every developer passionate about programming language design to join this journey of exploration.

### 6.1 Design Discussions

**Suited for**: Programming language theory researchers, type system enthusiasts, language design fanatics

**How to participate**:

- **GitHub Discussions**: Participate in discussions in the "Language Design" category
- **Design proposals (RFC)**: Propose design documents for new features, following the templates under `rfcs/`
- **Syntax reviews**: Provide improvement suggestions or discover potential issues with existing syntax design

| **Currently hot topics**: |
| |
| - Design and implementation of the macro system |
| - Interface type mechanism |
| - Error handling syntax optimization |
| - Standard library API design |

**Submitting design proposals**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros and cons analysis, alternatives)
3. Open a Pull Request for community review
4. After core team review, merge or reject

### 6.2 Compiler Implementation

**Suited for**: Compiler developers, systems programmers, performance optimization experts

**Current implementation focus** (in priority order):

| Priority | Module | Description | Difficulty |
|----------|--------|-------------|------------|
| P0 | **Bytecode VM** | VM instruction completion, performance optimization | Medium |
| P0 | **Runtime Memory** | GC implementation, memory allocator | High |
| P0 | **Async Runtime** | Full spawn model implementation | High |
| P1 | Standard library | IO, String, List, Concurrent | Medium |
| P1 | JIT compiler | Cranelift integration | High |
| P2 | AOT compiler | LLVM/Cranelift backend | High |
| P3 | Bootstrapping compiler | Rewrite in YaoXiang | Extremely High |

**Tech stack**:

- **Implementation language**: Rust (current phase)
- **Code generation**: Cranelift or LLVM
- **Build tool**: Cargo
- **Testing framework**: Rust `#[test]` + `cargo nextest`

**Getting started with contributions**:

1. Read `docs/YaoXiang-implementation-plan.md` to understand the architecture design
2. Choose a module of interest under `src/`
3. Look at `tests/unit/` to understand testing requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suited for**: IDE plugin developers, toolchain enthusiasts, efficiency tool seekers

**Tools that need development**:

| Tool | Status | Description |
|------|--------|-------------|
| **LSP Server** | ⏳ Not started | Language Server Protocol support |
| **Debugger integration** | ⏳ Not started | GDB/LLDB integration |
| **Formatter** | ⏳ Not started | `yaoxiang fmt` |
| **Package Manager** | ⏳ Not started | Dependency management, version resolution |
| **Package Registry** | ⏳ Not started | Centralized or decentralized |
| **REPL** | ⏳ Not started | Interactive interpreter |
| **Benchmarking tool** | ⏳ Not started | Performance analysis |
| **VS Code plugin** | ⏳ Not started | Syntax highlighting, completion, debugging |
| **Vim/Neovim plugin** | ⏳ Not started | Syntax highlighting, LSP client |

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

**Suited for**: Library developers, API designers, domain experts

**Standard library module planning**:

| Module | Priority | Description |
|--------|----------|-------------|
| `std.io` | P0 | File IO, console input/output |
| `std.string` | P0 | String operations, formatting |
| `std.list` | P0 | List/array operations |
| `std.dict` | P0 | Dictionary/hash table |
| `std.math` | P0 | Mathematical functions, constants |
| `std.time` | P1 | Date/time operations |
| `std.net` | P1 | Network programming, HTTP |
| `std.concurrent` | P1 | Concurrency primitives, channels |
| `std.crypto` | P2 | Cryptographic hashing, signatures |
| `std.json` | P1 | JSON parsing/generation |
| `std.regex` | P2 | Regular expressions |
| `std.database` | P3 | Database connections |
| `std.gui` | P3 | Graphical interface (long-term) |

**Design principles**:

- Consistency: Functions with the same functionality maintain consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoiding overdesign
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suited for**: Technical writers, educators, community managers

**Documentation that needs contributions**:

| Document | Status | Description |
|----------|--------|-------------|
| Quick start | ✅ Done | 5-minute getting started guide |
| Language guide | ✅ Done | Systematic learning of core concepts |
| Language specification | ✅ Done | Complete syntax and semantics definition |
| Implementation plan | ✅ Done | Compiler implementation technical details |
| API documentation | ⏳ Not started | Standard library API reference |
| Tutorials | ⏳ Not started | Advanced tutorials and best practices |
| Blog | ⏳ Not started | Technical articles and design stories |
| Translations | ⏳ Not started | Multi-language support |

### 6.6 Community Building

**Suited for**: Community managers, event organizers, evangelists

**Community activities**:

- Regular online Meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution Sprints (quarterly)
- Offline gatherings and conference talks

**Channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Bug reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guide

**How to start contributing**:

1. **Understand the project**: Read README and design documents
2. **Choose a direction**: Select a contribution area based on your interests
3. **Set up environment**: Rust 1.75+, cargo, git
4. **Find tasks**: Look at GitHub Issues for the `good first issue` tag
5. **Submit PR**: Follow submission guidelines, write tests
6. **Participate in reviews**: Review others' code, join discussions

**Submission format**:

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

- Follow `rustfmt.toml` specifications
- Ensure `cargo clippy` produces no warnings
- Write necessary unit tests
- Update relevant documentation

---

## Appendix A: Language Quick Reference

### A.1 Keywords

| Keyword | Purpose |
|---------|---------|
| `pub` | Public export |
| `use` | Import module |
| `spawn` | Spawn marker |
| `ref` | Shared ownership (compiler automatically selects Rc/Arc) |
| `mut` | Mutable variable |
| `if/elif/else` | Conditional branches |
| `match` | Pattern matching |
| `while/for` | Loops |
| `return/break/continue` | Control flow |
| `as` | Type casting |
| `in` | Membership test/list comprehension |
| `unsafe` | unsafe code block (raw pointers) |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type` keyword has been removed in RFC-010; use the unified `name: Type = value` syntax instead.

### A.2 Annotations

| Annotation | Purpose |
|------------|---------|
| `@block` | Marks that the function following it should execute synchronously |
| `@eager` | Marks an expression that needs eager evaluation |
| `@Send` | Explicitly declares satisfying the Send constraint |
| `@Sync` | Explicitly declares satisfying the Sync constraint |

### A.3 Primitive Types

| Type | Description | Default Size |
|------|-------------|--------------|
| `Void` | Empty value | 0 bytes |
| `Bool` | Boolean | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating-point number | 8 bytes |
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

| Source | Borrowed Concepts |
|--------|-------------------|
| **Rust** | Ownership model, zero-cost abstractions, type system |
| **Python** | Syntax style, readability, list comprehensions |
| **Idris/Agda** | Dependent types, type-driven development |
| **Curry-Howard Isomorphism** | Types are propositions, programs are proofs, unified theory of type systems and logic |
| **TypeScript** | Type annotations, runtime types |
| **MoonBit** | AI-friendly design, concise syntax |
| **Haskell** | Pure functional programming, pattern matching |
| **OCaml** | Type inference, variant types |

---

## Appendix C: FAQ

**Q: What advantages does YaoXiang have over Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions but uses simpler syntax and lower cognitive burden. The **spawn model** is more concise than Rust's `async/await`—just one `spawn` marker, no manual management of Futures and Pin. "All things arise together, I observe their return," making concurrent programming as intuitive as describing natural laws. The **ownership model** (RFC-009 v9) replaces lifetime annotations with Move + &T/&mut T tokens, and uses type properties (Dup/Linear) instead of the borrow checker. Unified type syntax eliminates the conceptual fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What types of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing confusion caused by nesting depth. This is a carefully considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. Release time depends on implementation progress; see [Version Planning RFC](./rfc/003-version-planning.md) for details.

**Q: How do I contact the core team?**

A: Through GitHub Discussions or the Discord community channel. Core team members respond regularly.

---

> **Last updated**: 2026-05-31
>
> **Document version**: v2.0.0
>
> **License**: [MIT](LICENSE)

---

> "The changes of Yao and Xiang give birth to all things. The evolution of types creates programs."
>
> May the journey of YaoXiang's design walk alongside you.