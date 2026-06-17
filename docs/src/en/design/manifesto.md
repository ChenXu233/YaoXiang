# YaoXiang Design Manifesto

> **Version**: v2.0.0
> **Status**: Official Release
> **Author**: Chenxu + YaoXiang Community
> **Date**: 2026-05-31

---

> "The Tao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth to the myriad creatures."
> — *Tao Te Ching*
>
> Types are the Tao; from them, all things are born.

---

## I. Why Create YaoXiang?

### 1.1 Filling a Language Gap

Throughout the long history of programming languages, we have witnessed the birth and evolution of countless excellent languages: C brought an efficiency revolution to systems programming, Python created a programming experience accessible to everyone, Rust proved that memory safety and performance can coexist, and TypeScript made large frontend projects maintainable. Yet when we examine today's language ecosystem, we still find an obvious gap—**no single language can simultaneously satisfy the following three core needs**:

| Need | Problems with Existing Solutions |
|------|----------------------------------|
| **Type Safety** | Rust is too strict with a steep learning curve; TypeScript uses optional types that cannot provide compile-time guarantees |
| **Natural Syntax** | Rust's syntax is complex and obscure; Haskell's functional style has a high entry barrier; traditional static languages are verbose and cumbersome |
| **AI-Friendliness** | Existing languages have ambiguous syntax, complex ASTs, and unpredictable hidden behavior, limiting the accuracy of AI-generated and AI-modified code |

YaoXiang was born precisely to fill this gap. We believe: **a programming language should be both powerful and approachable, both safe and efficient, both rigorous and elegant**.

### 1.2 The Real Problems We Solve

**Problem One: Fragmentation of Type Systems**

Today's programming languages exhibit serious fragmentation in their type systems. Static-typed languages pursue absolute correctness at compile-time but often sacrifice development efficiency; dynamically-typed languages offer flexibility but expose difficult-to-maintain defects in large projects. YaoXiang proposes a unified abstraction framework of "everything is a type," making types the main thread that runs through language design rather than an afterthought patch.

**Problem Two: The Trade-off Between Memory Safety and Performance**

For a long time, developers have had to make difficult choices between memory safety and runtime performance. GC (garbage collection) liberates developers but brings latency fluctuations and memory overhead; manual memory management is efficient but dangerous like walking a tightrope. YaoXiang adopts a Rust-style ownership model, eliminating data races and memory leaks at compile-time while maintaining zero-cost abstractions, achieving high performance without GC.

**Problem Three: The Cognitive Burden of Asynchronous Programming**

Modern applications cannot do without networking and concurrency, and asynchronous programming has always been a programmer's nightmare. Nested callback functions, Promise chains, async/await syntax—each solution adds complexity to the code. YaoXiang redesigned the asynchronous model: simply add the `spawn` marker after a function signature, and the compiler handles all asynchronous details automatically, making concurrent programming as natural as synchronous code.

**Problem Four: The Bottleneck of AI-Assisted Programming**

When AI began assisting developers in writing code, language design choices became critical. Ambiguous syntax rules, implicit type conversions, complex syntactic sugar—these features that human programmers have grown accustomed to become obstacles for AI to understand and generate. From day one, YaoXiang has made "AI-friendliness" a core design goal: strict indentation rules, explicit code block boundaries, and unambiguous syntactic structures, enabling AI to accurately understand, generate, and modify code.

### 1.3 The Philosophical Foundation of the Language

The name YaoXiang derives from "爻" (Yáo) and "象" (Xiàng) in the *I Ching*. "Yáo" is the basic symbol that forms hexagrams, symbolizing the interplay of yin and yang, the generation of motion and stillness; "Xiàng" is the external manifestation of the essence of things, representing the myriad phenomena that encompass all.

This philosophical thinking is reflected in every detail of the language design:

- **Unity**: Just as the simple symbols of trigrams form complex hexagrams, YaoXiang uses a few core concepts (types, functions, constructors) to construct a complete programming model
- **Hierarchy**: Just as there are innate and acquired distinctions among phenomena, YaoXiang's type system has a clear hierarchical structure, from primitive types to generics, from values to meta types
- **Mutability**: Just as yin and yang flow and transform endlessly, YaoXiang supports dependent types, allowing types to evolve as values change
- **Recognizability**: Just as hexagrams can be interpreted and all things can be represented, YaoXiang provides complete type reflection capabilities, with full runtime type information available
- **Provability**: Just as hexagrams reveal the patterns of things, YaoXiang's type system follows the Curry-Howard correspondence (types as propositions, programs as proofs); the type-checking process is the verification of logical proofs

---

## II. Core Philosophy and Principles

The following design tenets are the cornerstone of YaoXiang, **non-negotiable and inviolable**. Any feature proposal must pass the test of these principles.

### 2.1 Principle One: Everything Is a Type

In YaoXiang's worldview, types are the highest-level abstraction unit and the core concept running through the language.

**Concrete manifestations**:

- **Values are instances of types**: `42` is an instance of type `Int`; `"hello"` is an instance of type `String`
- **Types themselves are types**: `Type` is the language's only meta type keyword; the type of `Int` is `Type`
- **Functions are type mappings**: `add: (a: Int, b: Int) -> Int` describes a type mapping from `Int × Int` to `Int`
- **Modules are type combinations**: Modules are namespace combinations containing functions and types

**Why this is non-negotiable**: Unified type abstraction simplifies language semantics, eliminates the binary opposition between values and types, and lets the type system become the guardian of code correctness rather than an obstacle.

### 2.2 Principle Two: Strict Structure

YaoXiang's syntax design pursues "unambiguous, predictable, easy to parse."

**Specific rules**:

- **Mandatory 4-space indentation**: Tab characters are forbidden; code block boundaries are clear at a glance
- **Brackets cannot be omitted**: Function parameters must have parentheses, list elements must have commas
- **Code blocks must use curly braces**: Control flow constructs like `if`, `while`, `for` must be wrapped in `{ }`
- **Minimal set of keywords**: Only 17 core keywords are retained; syntactic sugar proliferation is rejected

**Why this is non-negotiable**: Strict structure brings three key advantages—(1) more accurate IDE syntax highlighting and code folding; (2) significantly improved accuracy in AI code generation and modification; (3) new learners can quickly understand code structure.

### 2.3 Principle Three: Zero-Cost Abstractions

High-level abstractions should not bring runtime performance overhead.

**Specific guarantees**:

- **Monomorphization**: Generic functions expand into concrete versions at compile-time, with no vtable lookup overhead
- **Inline optimization**: Simple functions are automatically inlined, eliminating function call overhead
- **Stack allocation priority**: Small objects are stack-allocated by default; heap allocation is used only when necessary
- **No GC**: The ownership model guarantees memory safety, requiring no garbage collector runtime overhead

**Why this is non-negotiable**: Performance is the survival baseline of a programming language. Any design that exchanges performance for convenience is a betrayal of programmers.

### 2.4 Principle Four: Immutable by Default

Mutability and complexity are inseparable companions. YaoXiang chooses immutability by default, making code easier to reason about and understand.

**Specific rules**:

- Variables are immutable by default; they cannot be modified after assignment
- Mutability must be explicitly declared with `mut` when needed
- References are immutable by default; mutable references require the `mut` marker
- Transfer of ownership means the original binding becomes invalid

**Why this is non-negotiable**: Immutability is the foundation of concurrency safety, the guarantee of code readability, and the crystallized wisdom of functional programming.

### 2.5 Principle Five: Types Are Data

Type information should not exist only at compile-time but should be fully available at runtime.

**Specific capabilities**:

- Runtime type query: any value can obtain its type information
- Type reflection: types themselves can be constructed and manipulated
- Pattern matching destructuring: type constructors can be used directly in pattern matching
- Generic specialization: the specialized types of generic parameters can be obtained at runtime

**Why this is non-negotiable**: Complete type reflection capability is the foundation of metaprogramming and the cornerstone of high-performance frameworks and tools.

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

**YaoXiang's unified syntax**: everything is `name: type = value`; `Type` is the only meta type keyword.

```yaoxiang
# === Record Type ===

Point: Type = {
    x: Float,
    y: Float,
}

# Field with default value
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

# === Interface (record whose fields are all function types) ===

Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

Serializable: Type = {
    serialize: () -> String,
}

# === Interface implementation (interface name written within the type body) ===

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

**Innovation value**: No fragmentation of `fn`, `struct`, `enum`, `trait`, `impl` keywords—one unified syntax covers all declarations.

### 3.2 Innovation Two: Constructors Are Types

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

YaoXiang adopts a purely functional design, achieving object-method-call-like syntactic sugar through currying, without introducing the `class` and `method` keywords.

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

# Both calling styles are completely equivalent
d1 = distance(p1, p2)     # Direct call to the core function
d2 = p1.distance(p2)      # Method syntactic sugar

# Curried usage
dist_from_p1 = p1.distance  # Partial application, awaiting the second argument
d3 = dist_from_p1(p2)       # 2.828
```

**Innovation value**: Pure functional design, no hidden `self` parameter; functions are values and can be freely passed and composed.

### 3.4 Innovation Four: The Spawn Model

> "All things arise together; through this, I observe the return." — *I Ching, Hexagram 24 (Fu)*
>
> The spawn model draws its inspiration from this, describing a programming paradigm: developers describe logic with synchronous, sequential thinking, while the language runtime makes the computation units within it automatically and efficiently execute concurrently, like all things arising together, ultimately converging in unified coordination.

**Three core principles**:

| Principle | Description |
|-----------|-------------|
| **Synchronous syntax** | Sequential code that is what you see is what you get |
| **Concurrent essence** | The runtime automatically extracts parallelism |
| **Unified coordination** | Results automatically converge when needed, guaranteeing logical correctness |

**Terminology**:

| Official Term | Corresponding Syntax | Explanation |
|---------------|----------------------|-------------|
| **spawn function** | `spawn (params) => body` | Defines a computation unit that can participate in spawn execution |
| **spawn block** | `spawn { a(), b() }` | An explicitly declared concurrent region; tasks within the block execute in spawn fashion |
| **spawn loop** | `spawn for x in xs { ... }` | Data parallelism; the loop body executes in spawn fashion over all elements |
| **spawn value** | `Async(T)` | A future value currently being spawned; automatically awaited when used |
| **spawn graph** | Lazy computation graph (DAG) | The stage where spawn execution happens, describing dependencies and parallelism |
| **spawn scheduler** | Runtime task scheduler | The intelligent core that coordinates all things, letting them spawn at the right moment |

> **See**: [RFC-001 Spawn Model](./rfc/001-concurrent-model-error-handling.md)

```yaoxiang
# === spawn function ===
# Function marked with spawn
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# === spawn block ===
# Expressions within spawn { } are forced to execute in parallel
compute_all: () -> (Int, Int, Int) spawn = {
    (a, b, c) = spawn {
        heavy_calc(1),    # Task 1
        heavy_calc(2),    # Task 2
        another_calc(3)   # Task 3
    }
    return (a, b, c)
}

# === Automatic awaiting ===
main: () -> Void = {
    # Two independent requests automatically execute in parallel
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # Await points are automatically inserted when results are needed
    print(users.length + posts.length)  # Automatically awaits users and posts
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
- See [RFC-001 Spawn Model](./rfc/accepted/001-concurrent-model-error-handling.md)

**Innovation value**: The cognitive burden of asynchronous programming is reduced to zero; code readability is identical to that of synchronous code, while achieving high-performance parallel execution efficiency.

### 3.5 Innovation Five: Value-Dependent Types (RFC-011)

> **Status**: In design, partially implemented

Types can depend on values, enabling truly type-driven development.

```yaoxiang
# Matrix type: dimensions are determined at compile-time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time computation: factorial(3) = 6
vec: Vec(factorial(3)) = Vec(6)()

# Compile-time dimension validation
identity_3x3: Matrix(Float, 3, 3) = identity(Float, 3)(3)
# multiply(matrix_2x3, matrix_4x2)  # Compile error: dimensions do not match
```

**Innovation value**: Catch more errors at compile-time, achieving more precise type guarantees.

### 3.6 Innovation Six: Minimal Keyword Design

YaoXiang defines only 17 core keywords—far fewer than mainstream languages:

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

| Compared Language | Number of Keywords |
|-------------------|--------------------|
| YaoXiang | **17** |
| Rust | 51+ |
| Python | 35 |
| TypeScript | 64+ |
| Go | 25 |

**Innovation value**: Lower memory burden, more consistent syntactic style, and easier-to-parse syntactic structure.

---

## IV. Preliminary Syntax Preview

The following code examples showcase the look and feel of YaoXiang, helping you quickly appreciate its design aesthetic.

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
p2 = p1              # Move, p1 can no longer be read

# &T / &mut T tokens (zero compile-time cost)
p2.print()           # The compiler automatically creates an &Point token
p2.shift(1.0, 1.0)   # The compiler automatically creates an &mut Point token

# ref: shared ownership (the compiler automatically chooses Rc/Arc)
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
Borrow token   Default   Shared hold  Deep copy  Raw pointer
 Zero-cost   Zero-copy  Auto Rc/Arc  Explicit  System-level
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

# Handling with match
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

## V. Roadmap and Pending Items

### 5.1 Decided Design Decisions

The following decisions have been thoroughly discussed and reviewed, and **will no longer accept changes**:

| Module | Decision | Description |
|--------|----------|-------------|
| **Type system** | Everything is a type | Values, functions, modules, and generics are all types |
| **Type syntax** | Unified `name: type = value` | One declaration form covers all cases; `Type` is the only meta type keyword |
| **Keywords** | 17 core keywords | Excluding `type`/`fn`/`struct`/`enum`/`trait`/`impl` |
| **Function syntax** | Signature + expression | `name: (params) -> ReturnType = body` |
| **Method binding** | RFC-004 Curried binding | `Type.method = function[position]` |
| **Asynchronous model** | Spawn model | `spawn` marker, lazy evaluation, automatic parallelism |
| **Memory management** | Ownership model (RFC-009 v9) | Move + &T/&mut T tokens + ref + clone + unsafe, no GC |
| **File as module** | Module system | Every `.yx` file is a module |
| **Main function** | `main: () -> Void` | Program entry point |
| **Thread safety** | ref automatically chooses Rc/Arc | Compiler performs escape analysis; users are unaware |

### 5.2 Open Design Topics

The following topics are still under discussion, and **community input is welcome**:

| Topic | Current Status | Open Questions |
|-------|----------------|----------------|
| **Literal syntax** | Float support | Should scientific notation like `3.14e-10` be supported? |
| **Generic inference** | Basic support | Should return type generic inference be supported? |
| **Macro system** | Not yet designed | Are hygienic macros needed? What is the syntax design direction? |
| **Package manager** | Not yet designed | Is a centralized package repository needed? What is the dependency resolution strategy? |
| **FFI** | Not yet designed | What is the specific plan for C interoperability? |
| **Generic constraints** | Basic support | Should trait/bounds mechanisms be supported? |
| **Reflection depth** | Basic support | Should access to private members be supported? |

### 5.3 Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang Implementation Roadmap                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  v0.1: Rust Interpreter ───────→ v0.5: Rust Compiler ───────→ v1.0: Rust AOT│
│        ✅ Completed                    │ (Current Phase)          Compiler     │
│                                       │                                      │
│                                       ▼                                      │
│  v0.6: YaoXiang Interpreter ←────── v1.0: YaoXiang JIT Compiler ←─── v2.0:  │
│        (Self-Hosting)                  (Self-Hosting)              YaoXiang AOT│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Milestone details**:

| Version | Status | Goal | Deliverable |
|---------|--------|------|-------------|
| **v0.1** | ✅ Completed | Interpreter prototype | Basic interpreter, lexer, parser, basic types |
| **v0.2** | ✅ Completed | Complete interpreter | Type checking, pattern matching, module system |
| **v0.3** | 🔄 In progress | Bytecode generation | IR intermediate representation, bytecode generation, closure optimization, monomorphization |
| **v0.4** | 🔄 In progress | Bytecode virtual machine | VM core, instruction execution, call frame management, inline cache |
| **v0.5** | ⏳ Not started | Runtime system | GC, scheduler, standard library IO |
| **v1.0** | ⏳ Not started | AOT compiler | Complete optimization, native code generation |
| **v2.0** | ⏳ Not started | Self-hosting compiler | A new compiler written in YaoXiang |

### 5.4 Current Implementation Status

| Module | Status | Completion | Description |
|--------|--------|------------|-------------|
| **Lexer** | ✅ Completed | 100% | Token definitions, keyword recognition, test cases |
| **Parser** | ✅ Completed | 100% | AST definitions, expression/statement parsing, boundary tests |
| **Type checker** | ✅ Completed | 95% | Type inference, monomorphization, generic specialization, error handling |
| **IR intermediate representation** | ✅ Completed | 90% | IR instruction definitions, type representation, control flow graph |
| **Bytecode generation** | ✅ Completed | 85% | Expression/statement/control flow bytecode, closure conversion |
| **Ownership system** | ✅ Completed | 100% | Move semantics, Clone/Drop semantics, mutability checking, Send/Sync constraints |
| **Monomorphization** | ✅ Completed | 100% | Generic instantiation, specialization implementation |
| **Escape analysis** | 🔄 In progress | 40% | Basic framework, variable escape determination |
| **Bytecode virtual machine** | 🔄 In progress | 70% | VM core, instruction execution, call frames, inline cache |
| **Runtime scheduler** | 🔄 In progress | 60% | Task descriptors, work-stealing queue, wait queue |
| **Runtime memory** | 🔄 In progress | 50% | Memory allocator, GC framework |
| **Standard library** | 🔄 In progress | 30% | IO, String, List, Dict, Math, Concurrent |
| **JIT compiler** | ⏳ Not started | 0% | Awaiting Cranelift/LLVM integration |
| **AOT compiler** | ⏳ Not started | 0% | Awaiting implementation |

**Code generation module details**:

| Submodule | Status | Key Features |
|-----------|--------|--------------|
| Expression generation | ✅ Completed | Arithmetic, comparison, logic, function calls |
| Statement generation | ✅ Completed | Assignment, return, conditionals, loops |
| Control flow generation | ✅ Completed | Switch pattern matching, loop unrolling |
| Closure handling | ✅ Completed | Environment capture, closure conversion |
| Bytecode serialization | ✅ Completed | Bytecode read/write, test cases |
| Generator code generation | ✅ Completed | yield syntax support, state machine conversion |
| Integration testing | ✅ Completed | End-to-end compilation and execution tests |

**Asynchronous implementation status (Spawn Model)**:

| Submodule | Status | Description |
|-----------|--------|-------------|
| spawn keyword parsing | ✅ Completed | Lexer/parser support |
| is_async flag | ✅ Completed | AST/type system support |
| Async(T) type design | ✅ Completed | Design document completed |
| Scheduler framework | ✅ Completed | Basic work-stealing implementation |
| Send/Sync constraints | ✅ Completed | Type constraint design document |
| IR extension | 🔄 In progress | CallAsync instruction defined |
| VM asynchronous instructions | 🔄 In progress | Instruction framework defined |
| Complete implementation | ⏳ Not started | v0.5 milestone |

---

## VI. How to Contribute

YaoXiang is a language born of the community, growing with the community, and serving the community. We sincerely invite every developer passionate about programming language design to join us on this exploratory journey.

### 6.1 Design Discussion

**Suitable for**: Programming language theory researchers, type system enthusiasts, language design fanatics

**How to participate**:

- **GitHub Discussions**: Participate in discussions under the "Language Design" category
- **Design proposals (RFCs)**: Propose design documents for new features, following the template in the `rfcs/` directory
- **Syntax review**: Suggest improvements or identify potential issues with existing syntax designs

| **Currently Hot Topics**: |
| |
| - Macro system design and implementation |
| - Interface type mechanism |
| - Error handling syntax optimization |
| - Standard library API design |

**Submitting a design proposal**:

1. Create a new file in the `rfcs/` directory
2. Fill in the RFC template (motivation, detailed design, pros/cons analysis, alternatives)
3. Initiate a Pull Request for community review
4. Merge or reject after deliberation by the core team

### 6.2 Compiler Implementation

**Suitable for**: Compiler developers, systems programmers, performance optimization experts

**Current implementation focus** (sorted by priority):

| Priority | Module | Description | Difficulty |
|----------|--------|-------------|------------|
| P0 | **Bytecode virtual machine** | VM instruction completion, performance optimization | Medium |
| P0 | **Runtime memory** | GC implementation, memory allocator | High |
| P0 | **Asynchronous runtime** | Complete implementation of the spawn model | High |
| P1 | Standard library | IO, String, List, Concurrent | Medium |
| P1 | JIT compiler | Cranelift integration | High |
| P2 | AOT compiler | LLVM/Cranelift backend | High |
| P3 | Self-hosting compiler | Rewrite in YaoXiang | Extreme |

**Technology stack**:

- **Implementation language**: Rust (current phase)
- **Code generation**: Cranelift or LLVM
- **Build tool**: Cargo
- **Testing framework**: Rust `#[test]` + `cargo nextest`

**Getting started**:

1. Review `docs/YaoXiang-implementation-plan.md` for architectural design
2. Choose an interesting module under the `src/` directory
3. Check `tests/unit/` for testing requirements
4. Ensure `cargo fmt` and `cargo clippy` pass before submitting code

### 6.3 Toolchain Development

**Suitable for**: IDE plugin developers, toolchain enthusiasts, productivity tool seekers

**Tools that need to be developed**:

| Tool | Status | Description |
|------|--------|-------------|
| **LSP server** | ⏳ Not started | Language Server Protocol support |
| **Debugger integration** | ⏳ Not started | GDB/LLDB integration |
| **Formatter** | ⏳ Not started | `yaoxiang fmt` |
| **Package manager** | ⏳ Not started | Dependency management, version resolution |
| **Package repository** | ⏳ Not started | Centralized or decentralized |
| **REPL** | ⏳ Not started | Interactive interpreter |
| **Benchmark tool** | ⏳ Not started | Performance analysis |
| **VS Code extension** | ⏳ Not started | Syntax highlighting, completion, debugging |
| **Vim/Neovim plugin** | ⏳ Not started | Syntax highlighting, LSP client |

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
│   └── vim/                      # Vim/Neovim
```

### 6.4 Standard Library Development

**Suitable for**: Library developers, API designers, domain experts

**Standard library module planning**:

| Module | Priority | Description |
|--------|----------|-------------|
| `std.io` | P0 | File IO, console input/output |
| `std.string` | P0 | String manipulation, formatting |
| `std.list` | P0 | List/array operations |
| `std.dict` | P0 | Dictionary/hash table |
| `std.math` | P0 | Mathematical functions, constants |
| `std.time` | P1 | Time and date operations |
| `std.net` | P1 | Network programming, HTTP |
| `std.concurrent` | P1 | Concurrency primitives, channels |
| `std.crypto` | P2 | Cryptographic hashing, signatures |
| `std.json` | P1 | JSON parsing/generation |
| `std.regex` | P2 | Regular expressions |
| `std.database` | P3 | Database connection |
| `std.gui` | P3 | Graphical interface (long-term) |

**Design principles**:

- Consistency: Functions with the same functionality maintain consistent naming and behavior
- Simplicity: APIs should be intuitive and easy to use, avoiding over-engineering
- Performance: Standard library functions should be efficient, avoiding unnecessary copies
- Testability: Every function should have corresponding unit tests

### 6.5 Documentation and Tutorials

**Suitable for**: Technical writers, educators, community managers

**Documentation contributions needed**:

| Documentation | Status | Description |
|---------------|--------|-------------|
| Quick start | ✅ Completed | 5-minute getting-started guide |
| Language guide | ✅ Completed | Systematic learning of core concepts |
| Language specification | ✅ Completed | Complete syntax and semantic definition |
| Implementation plan | ✅ Completed | Compiler implementation technical details |
| API documentation | ⏳ Not started | Standard library API reference |
| Tutorials | ⏳ Not started | Advanced tutorials and best practices |
| Blog | ⏳ Not started | Technical articles and design stories |
| Translations | ⏳ Not started | Multilingual support |

### 6.6 Community Building

**Suitable for**: Community managers, event organizers, evangelists

**Community activities**:

- Regular online meetups (monthly)
- Design and implementation discussions (weekly)
- Code contribution sprints (quarterly)
- In-person gatherings and conference talks

**Communication channels**:

- GitHub Discussions: Technical discussions
- GitHub Issues: Bug reports and feature requests
- Discord/Slack: Real-time communication
- Twitter/X: Project updates
- Blog: In-depth articles

### 6.7 Contribution Guide

**How to start contributing**:

1. **Understand the project**: Read the README and design documents
2. **Choose a direction**: Select a contribution area based on your interests
3. **Set up the environment**: Rust 1.75+, cargo, git
4. **Find a task**: Check GitHub Issues for the `good first issue` label
5. **Submit a PR**: Follow the commit conventions and write tests
6. **Participate in reviews**: Review others' code and participate in discussions

**Commit conventions**:

```bash
# Commit message format
<type>(<scope>): <subject>

# Types
feat: New feature
fix: Bug fix
docs: Documentation update
style: Code formatting (no functional impact)
refactor: Refactor
perf: Performance optimization
test: Tests
chore: Build tools or auxiliary tools

# Examples
feat(typecheck): add generic type inference
fix(parser): fix infinite loop on invalid input
docs(readme): update installation instructions
```

**Code style**:

- Follow the `rustfmt.toml` conventions
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
| `ref` | Shared ownership (compiler automatically chooses Rc/Arc) |
| `mut` | Mutable variable |
| `if/elif/else` | Conditional branches |
| `match` | Pattern matching |
| `while/for` | Loops |
| `return/break/continue` | Control flow |
| `as` | Type conversion |
| `in` | Membership check/list comprehension |
| `unsafe` | Unsafe code block (raw pointers) |

> **Note**: `Type`, `true`, `false`, `void`, etc. are reserved words, not keywords. The `type` keyword was removed in RFC-010; the unified `name: Type = value` syntax is used instead.

### A.3 Primitive Types

| Type | Description | Default Size |
|------|-------------|---------------|
| `Void` | Empty value | 0 bytes |
| `Bool` | Boolean value | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating-point number | 8 bytes |
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

## Appendix B: Design Inspirations

YaoXiang's design draws on the excellent ideas of the following languages and projects:

| Source | Inspiration |
|--------|-------------|
| **Rust** | Ownership model, zero-cost abstractions, type system |
| **Python** | Syntax style, readability, list comprehension |
| **Idris/Agda** | Dependent types, type-driven development |
| **Curry-Howard correspondence** | Types as propositions, programs as proofs; the unified theory of type systems and logic |
| **TypeScript** | Type annotations, runtime types |
| **MoonBit** | AI-friendly design, concise syntax |
| **Haskell** | Pure functional, pattern matching |
| **OCaml** | Type inference, variant types |

---

## Appendix C: Frequently Asked Questions

**Q: What are the advantages of YaoXiang compared to Rust?**

A: YaoXiang retains Rust's memory safety and zero-cost abstractions, but adopts simpler syntax and a lower cognitive burden. The **spawn model** is more concise than Rust's `async/await`—only a single `spawn` marker is needed, with no manual management of Future and Pin. "All things arise together; through this, I observe the return," making concurrent programming as intuitive as describing natural laws. The **ownership model** (RFC-009 v9) uses Move + &T/&mut T tokens to replace lifetime annotations, and uses type attributes (Dup/Linear) to replace the borrow checker. The unified type syntax eliminates the conceptual fragmentation of `enum`/`struct`/`trait`/`impl`.

**Q: What kinds of development is YaoXiang suitable for?**

A: Systems programming, application development, web services, scripting tools, AI-assisted programming. The goal is to become a general-purpose programming language.

**Q: Why choose 4-space indentation?**

A: 4 spaces provide clear visual separation of code blocks, reducing the confusion caused by deep nesting. This is a carefully considered "AI-friendly" design decision.

**Q: When will version 1.0 be released?**

A: v1.0 goal: production-ready. The release time depends on the implementation progress; see [Version Planning RFC](./rfc/003-version-planning.md) for details.

**Q: How do I contact the core team?**

A: Through GitHub Discussions or the Discord community channel. Core team members will respond regularly.

---

> **Last updated**: 2026-05-31
>
> **Document version**: v2.0.0
>
> **License**: [MIT](LICENSE)

---

> "Yáo and Xiàng transform, the myriad things arise. Types evolve, programs take form."
>
> May the design journey of YaoXiang travel with you.