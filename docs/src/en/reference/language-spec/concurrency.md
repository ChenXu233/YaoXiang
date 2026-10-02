# Concurrency Model Specification

> **Status**: Formal specification. Based on RFC-024 (Concurrency Model), RFC-009 (Ownership Model),
> RFC-008 (Runtime Architecture).

This document defines the concurrency model specification of the YaoXiang programming language,
including `{}` block semantics, the `spawn` concurrency primitive, ownership interactions, error
handling, and resource types.

**Core design—one primitive, one rule**:

```
spawn { ... }        ← the only parallel primitive
direct child assignments create tasks    ← the only rule
synchronously block waiting for results  ← the only behavior
```

---

## Chapter 1: Overview

### 1.1 The Essence of `{}` Blocks

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Attribute         | Description                                                                                                                               |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Dependency-driven | The block checks whether all internal variables are ready when executing; if they are, it runs immediately; otherwise it blocks and waits |
| Execution timing  | Determined by dependencies, unrelated to "immediate" or "deferred"                                                                        |
| Return value      | Use `return` to explicitly return a value; without `return`, the default is `Void`                                                        |
| Unified syntax    | Whether it appears in a function body, variable initialization, or after `spawn`, the semantics are consistent                            |
| Scope isolation   | Variables are strictly confined within `{}` and do not leak to the outer scope                                                            |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y; runs immediately once both are ready
    return x + y
}
```

### 1.2 Return Rules

| Form                            | Return value                                | Description                       |
| ------------------------------- | ------------------------------------------- | --------------------------------- |
| `= expr` (no curly braces)      | Directly returns `expr`                     | Expression as value               |
| `= { ... }` (with curly braces) | Must use `return`, otherwise returns `Void` | Block requires an explicit return |

```yaoxiang
// No curly braces: direct return
add: (a: Int, b: Int) -> Int = a + b

// With curly braces: must use return
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?
    // Variant construction must be type-qualified—a bare ok(...) triggers E1001 Unknown variable: 'ok'
    return Result(Data, Error).ok(transform(validated))
}

// With curly braces but no return: returns Void
log: (message: String) -> Void = {
    print(message)  // No return, returns Void
}
```

### 1.3 spawn Block Semantics

`spawn { ... }` is the **only parallel primitive** in YaoXiang.

**Core rules**:

- Direct child assignments of the spawn block create parallel tasks
- Assignments inside nested `{}` do not count as independent tasks
- The entire spawn block synchronously blocks and returns a result after all tasks complete
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks running in parallel
(a, b) = spawn {
    fetch("url1"),      // task 1
    fetch("url2")       // task 2
}
// Wait until both are complete before continuing
```

### 1.4 User Mental Model

> Your ordinary code runs sequentially. When you want multiple things to happen at once, put them
> inside a `spawn { ... }` block. Every direct assignment in the block starts immediately (in
> parallel), and the result you need is awaited automatically. The whole block waits for everything
> to finish, then gives you the final result. No callbacks, no `await`, no strange annotations.

---

## Chapter 2: Syntax and Semantics

### 2.1 Ordinary Code

Ordinary code (outside spawn blocks) is **executed sequentially**.

```yaoxiang
a = compute_a()     // runs first
b = compute_b(a)    // depends on a; runs after a completes
c = compute_c(b)    // depends on b; runs after b completes
```

### 2.2 spawn Block

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**Semantics**:

1. Direct child assignments within the spawn block run as independent tasks in parallel
2. Each task's result is bound to the corresponding pattern variable
3. The entire block blocks until all tasks complete
4. Returns a tuple of all results

```yaoxiang
// Single task
result = spawn {
    fetch("url")
}

// Multiple tasks
(a, b, c) = spawn {
    fetch("url1"),
    fetch("url2"),
    fetch("url3")
}
```

### 2.3 spawn in Function Bodies

A function body is itself a `{}` block, and `spawn` can be used within it.

```yaoxiang
fetch_and_parse: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        parsed = parse(fetch(url))
    }
    return results
}
```

### 2.4 spawn in Loops

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
```

**Semantics**: A data-parallel loop, where each iteration is an independent task.

```yaoxiang
// Process each element in a list in parallel
results = spawn for item in items {
    result = process(item)
}
```

> **Note**: The loop body of `spawn for` is an independent task and does not support sharing mutable
> state across iterations. If you need to aggregate results, use `spawn for` to collect the results
> and process them outside.

```yaoxiang
// Correct: parallel processing then aggregation outside
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // sequential aggregation
```

### 2.5 Nested spawn

spawn blocks can be nested; an inner spawn creates a new concurrency domain.

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

The direct child assignments of the inner spawn are the tasks; the outer spawn does not pass
through.

---

## Chapter 3: Interaction with the Ownership Model

### 3.1 Move Semantics

Move is YaoXiang's default semantics (zero-copy). Once a variable enters a spawn block, it cannot be
used externally.

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // ownership of data moves into the spawn block
}
// data is unavailable here (already moved)
```

### 3.2 Borrow Tokens

`&T` and `&mut T` are zero-sized compile-time permission proofs, **and cannot cross task
boundaries**. This is not a special rule—tokens are compile-time permission proofs; for cross-task
sharing, use `ref`.

```yaoxiang
data = load_data()

// Compile error: borrow tokens cannot cross tasks
result = spawn {
    process(&data)   // Error! &T cannot be passed across tasks
}
```

**Token type attributes**:

| Token    | Primary semantics                                                                                 | Secondary attributes                                                               |
| -------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `&T`     | **Freeze the source data**—while a ReadToken is alive, no WriteToken(T) can be obtained           | Zero-sized, copyable (Dup)—multiple read views are safe under the freeze guarantee |
| `&mut T` | **Exclusive read-write**—while a WriteToken is alive, no other tokens (read or write) can coexist | Zero-sized, linear (non-Dup)—copying under exclusive access is meaningless         |

> **Causal order**: The Dup of ReadToken is a corollary of the freeze guarantee, not the other way
> around. Data is frozen (no mutation possible) → multiple read views are safe → Dup can be
> implemented. Treating Dup as the definition and conflict checking as a patch inverts the
> causality.

### 3.3 ref Sharing

`ref` is the only way to share across scopes. The compiler automatically chooses `Rc` (single-task)
or `Arc` (cross-task); the user does not need to care.

```yaoxiang
data = load_data()
shared = ref data       // The compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // shared reference
    process_b(shared)   // shared reference
}
```

**Compiler selection strategy**:

| Condition                                                 | Choice | Reason                         |
| --------------------------------------------------------- | ------ | ------------------------------ |
| Default (cannot prove safety)                             | `Arc`  | Safety first, avoid data races |
| Compiler can prove data is only used within a single task | `Rc`   | No atomic-operation overhead   |

**ref vs borrow tokens**:

|              | `&T` / `&mut T`                 | `ref`                               |
| ------------ | ------------------------------- | ----------------------------------- |
| What it does | Take a look / modify in place   | Share and hold                      |
| Cost         | Zero overhead (zero-sized type) | Rc or Arc (compiler chooses)        |
| Cross-task   | Not allowed                     | Allowed (compiler auto-selects Arc) |

### 3.4 Closures and Capture

**Closures do not implicitly capture outer variables** (RFC-009 2026-06-16 resolution, SPEC §12.3).
A lambda only uses explicit parameters and its own local variables; when outer data is needed, pass
it via explicit parameters, or fix it via currying at the creation point. The compile error code for
implicit capture is E1001.

> Why is it prohibited: the outer scope at the closure's definition point may be dead after the
> closure escapes, so the implicitly captured reference cannot be guaranteed to be alive; values
> fixed via currying are taken at the creation point (where the caller's scope is alive), which is
> safe and has zero hidden cost.

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // ❌ Compile error E1001: implicitly captures data
```

```yaoxiang
// ✅ Correct way 1: pass via explicit parameter
add: (data: Data, x: Int) -> Int = data.value + x

// ✅ Correct way 2: fix via currying (context is fixed at the creation point, the closure only takes parameters)
mk: (data: Data) -> (x: Int) -> Int = (data) => (x) => data.value + x
```

**Sharing inside spawn**: when cross-task sharing is needed, use `ref` to explicitly share and hold
(RFC-024):

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

**There is no `spawn while` construct**: after `spawn` you can only follow with `for` or `{` (the
`parse_spawn` in `src/frontend/core/parser/pratt/nud.rs:117-136` only dispatches `KwFor` →
`parse_spawn_for` and `LBrace` → `parse_spawn_block`); the AST also only has two variants:
`Expr::Spawn` (block) and `Expr::SpawnFor` (data-parallel loop,
`src/frontend/core/parser/ast.rs:68-72`). To do "repeat conditionally" in a concurrent context,
write an ordinary `while` loop.

The iteration variable of `spawn for` is mutable by default; only when written as
`spawn for mut x in items` is mutation of that binding allowed in the loop body (the `var_mut` field
of `src/frontend/core/parser/ast.rs:71`; `src/frontend/core/typecheck/layers/ownership.rs:1660-1671`
registers mutability accordingly).

---

## Chapter 4: Error Handling

### 4.1 The ? Operator

The `?` operator is used for explicit error propagation, with semantics consistent with Rust.

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // if error, propagate immediately
    return content.read_all()
}
```

### 4.2 Error Propagation in spawn Blocks

**Rules**:

1. Wait for all tasks to complete (even if some have already failed)
2. Propagate the first error encountered
3. Use `?` to explicitly mark error-propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // may fail
    fetch("url2")?      // may fail
}
// If any task fails, the entire spawn block propagates the first error
```

### 4.3 Error Types

**Automatically generated**: the compiler automatically generates a union error type.

```yaoxiang
// The compiler infers the error type as HttpError | IoError
(a, b) = spawn {
    fetch("url"),           // may throw HttpError
    read_file("data.txt")  // may throw IoError
}
```

**Manual override**: users can manually define a unified error type. Note that `Result` **does not
have `map_err`** (it's not among the 8 exports of `src/std/result.rs:71-126`; `result.map_err(...)`
triggers `E1042`), so before `?` you need to move the upstream error into an `AppError` variant
yourself:

```
AppError: Type = {
    Http: (http_error: HttpError) -> AppError,
    Io: (io_error: IoError) -> AppError,
    Parse: (parse_error: ParseError) -> AppError
}

// Conceptual form (map_err is unavailable; use match to move explicitly)
process: (url: String, path: FilePath) -> Result(Data, AppError) = {
    fetched = match fetch(url) {
        ok(v) => v
        err(e) => return AppError.Http(e)
    }
    content = match read_file(path) {
        ok(v) => v
        err(e) => return AppError.Io(e)
    }
    return parse(fetched + content)
}
```

---

## Chapter 5: Resource Types and Side Effects

### 5.1 Built-in Resource Types

| Resource type | Description         | Compiler behavior                         |
| ------------- | ------------------- | ----------------------------------------- |
| `FilePath`    | Filesystem path     | Same-path operations auto-serialize       |
| `HttpUrl`     | HTTP endpoint       | Same-URL operations auto-serialize        |
| `DBUrl`       | Database connection | Same-connection operations auto-serialize |
| `Console`     | Standard output     | All Console operations auto-serialize     |

```yaoxiang
// Operations on the same file are auto-serialized
(a, b) = spawn {
    read_file("data.txt"),      // runs first
    write_file("data.txt", x)   // waits for the read to complete
}
```

### 5.2 User-Defined Resource Types

User-defined resource types need to be explicitly marked.

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 Side-Effect Tracking

The compiler tracks the use of resource types to ensure concurrency safety.

```yaoxiang
// Compiler warning: Console operations may interleave
spawn {
    print("Hello"),     // may interleave with the next line
    print("World")
}

// Correct: explicit serialization
spawn {
    print("Hello\nWorld")
}
```

---

## Chapter 6: Compiler Behavior

### 6.1 DAG Analysis

The compiler analyzes the dependency relationships (DAG) within a spawn block at compile time,
determining:

1. Which expressions can run in parallel
2. Which must be sequential
3. How to assign tasks

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // task 1
    y = fetch("url2"),      // task 2 (parallel with task 1)
    z = process(x, y)       // task 3 (depends on x and y; must wait)
}
```

### 6.2 Rc/Arc Selection (Conservative Strategy)

The compiler adopts a **conservative strategy**, defaulting to `Arc` to ensure thread safety:

- **Default `Arc`**: when the compiler cannot determine whether a `ref` is only used within a single
  task, it conservatively selects `Arc`
- **Degrade to `Rc`**: only when the compiler can **prove** through DAG analysis that the data will
  absolutely not be shared across tasks does it degrade to `Rc`
- **Better slow than wrong**: the extra overhead of choosing `Arc` is far smaller than the risk of a
  data race

### 6.3 No-Parallelism Warning

If the tasks inside a spawn block have no real opportunity for parallelism, the compiler issues a
warning.

```yaoxiang
// Compiler warning: no opportunity for parallelism
result = spawn {
    a = fetch("url")    // the only task
}
// Suggestion: use ordinary code directly
result = fetch("url")
```

### 6.4 Resource Conflict Detection

The compiler detects potential conflicts on resource types.

```yaoxiang
// Compile error: concurrent writes to the same file
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // Error!
}
```

---

## Chapter 7: Runtime Tiers

The compile phase is exactly the same; the difference lies only in how execution is performed at
runtime (RFC-008).

| Tier             | spawn support | DAG analysis                        | Use case                                      |
| ---------------- | ------------- | ----------------------------------- | --------------------------------------------- |
| Embedded Runtime | ❌            | None                                | WASM, game scripting, rule engines            |
| Standard Runtime | ✅            | Inside spawn blocks                 | Web services, data pipelines                  |
| Full Runtime     | ✅            | Inside spawn blocks + work stealing | Scientific computing, large-scale parallelism |

**Embedded Runtime**: just-in-time executor, no spawn support, high performance with low overhead.

**Standard Runtime**: supports `spawn {}` blocks, performs DAG analysis and automatic concurrency
inside spawn blocks. `num_workers=1` means single-threaded mode.

**Full Runtime**: Standard + WorkStealer load balancing.

---

## Appendix: Syntax Quick Reference

### A.1 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
SpawnBody   ::= Assignment (',' Assignment)*
```

### A.2 Error Handling

```
Expr '?'              // error propagation (Result type)
```

### A.3 ref Expression

```
RefExpr     ::= 'ref' Expr
```

### A.4 Resource Type Annotation

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```
