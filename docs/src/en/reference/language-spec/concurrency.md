# Concurrency Model Specification

> **Status**: Formal specification. Based on RFC-024 (Concurrency Model), RFC-009 (Ownership Model),
> and RFC-008 (Runtime Architecture).

This document defines the concurrency model specification for the YaoXiang programming language,
including `{}` block semantics, the `spawn` concurrency primitive, ownership interactions, error
handling, and resource types.

**Core design—one primitive, one rule**:

```
spawn { ... }        ← 唯一并行原语
直接子赋值创建任务    ← 唯一规则
同步阻塞等待结果      ← 唯一行为
```

---

## Chapter 1: Overview

### 1.1 The Nature of `{}` Blocks

In YaoXiang, a `{}` is a **dependency-driven unit of computation**.

| Property          | Description                                                                                                                               |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Dependency-driven | When it executes, a block checks whether all variables inside it are ready; if so, it executes immediately, otherwise it blocks and waits |
| Execution timing  | Determined by dependencies, unrelated to being "immediate" or "deferred"                                                                  |
| Return value      | Use `return` to return a value explicitly; without a `return`, `Void` is returned by default                                              |
| Uniform syntax    | The semantics are identical whether it appears in a function body, a variable initializer, or after `spawn`                               |
| Scope isolation   | Variables are strictly confined to the inside of `{}` and do not leak into the outer scope                                                |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y; executes as soon as both are ready
    return x + y
}
```

### 1.2 Return Rules

| Form                      | Return value                                | Description                           |
| ------------------------- | ------------------------------------------- | ------------------------------------- |
| `= expr` (without braces) | Returns `expr` directly                     | The expression is the value           |
| `= { ... }` (with braces) | Must use `return`; otherwise returns `Void` | The block requires an explicit return |

```yaoxiang
// Without braces: return directly
add: (a: Int, b: Int) -> Int = a + b

// With braces: return is required
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?
    // Variant construction must be type-qualified—a bare ok(...) reports E1001 Unknown variable: 'ok'
    return Result(Data, Error).ok(transform(validated))
}

// With braces but without return: returns Void
log: (message: String) -> Void = {
    print(message)  // No return; returns Void
}
```

### 1.3 spawn Block Semantics

`spawn { ... }` is the **only parallelism primitive** in YaoXiang.

**Core rules**:

- Direct child assignments in a spawn block create parallel tasks
- Assignments nested inside `{}` do not count as independent tasks
- The entire spawn block blocks synchronously and returns a result after all tasks complete
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks run in parallel
(a, b) = spawn {
    fetch("url1"),      // Task 1
    fetch("url2")       // Task 2
}
// Continue after both have completed
```

### 1.4 User Mental Model

> The ordinary code you write executes sequentially. When you want several things to happen
> together, put them in a `spawn { ... }` block. Each direct assignment in the block starts
> immediately (in parallel), and the result you need is awaited automatically. The entire block
> waits for everything to finish, then gives you the final result. No callbacks, no `await`, no
> strange annotations.

---

## Chapter 2: Syntax and Semantics

### 2.1 Ordinary Code

Ordinary code outside a spawn block executes **sequentially**.

```yaoxiang
a = compute_a()     // Executes first
b = compute_b(a)    // Depends on a; executes after a completes
c = compute_c(b)    // Depends on b; executes after b completes
```

### 2.2 spawn Blocks

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**Semantics**:

1. Direct child assignments within a spawn block run in parallel as independent tasks
2. Each task's result is bound to its corresponding pattern variable
3. The entire block blocks until all tasks complete
4. A tuple containing all results is returned

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

A function body is itself a `{}` block, so `spawn` can be used within it.

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

**Semantics**: A data-parallel loop in which each iteration is an independent task.

```yaoxiang
// Process each element in the list in parallel
results = spawn for item in items {
    result = process(item)
}
```

> **Note**: The body of a `spawn for` is an independent task and does not support shared mutable
> state across iterations. To aggregate results, use `spawn for` to collect them and then process
> them outside the loop.

```yaoxiang
// Correct: process in parallel, then aggregate outside
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // Sequential aggregation
```

### 2.5 Nested spawn

Spawn blocks can be nested; an inner spawn creates a new concurrency domain.

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

Only the direct child assignments of the inner spawn are tasks; the outer spawn does not see through
it.

---

## Chapter 3: Interaction with the Ownership Model

### 3.1 Move Semantics

Move is YaoXiang's default semantics (zero-copy). Once a variable enters a spawn block, it can no
longer be used outside it.

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // Ownership of data is moved into the spawn block
}
// data is unavailable here (moved)
```

### 3.2 Borrow Tokens

`&T` and `&mut T` are zero-sized compile-time permission proofs and **cannot cross task
boundaries**. This is not a special rule—the token is a compile-time permission proof; to share
across tasks, use `ref`.

```yaoxiang
data = load_data()

// Compile error: borrow tokens cannot cross tasks
result = spawn {
    process(&data)   // Error! &T cannot be passed across tasks
}
```

**Token type properties**:

| Token    | Primary semantics                                                                                       | Secondary properties                                                                                  |
| -------- | ------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `&T`     | **Freezes the source data**—while a ReadToken is alive, no WriteToken(T) can be obtained                | Zero-sized and copyable (Dup)—multiple read-only views are inherently safe under the freeze guarantee |
| `&mut T` | **Exclusive read/write access**—while a WriteToken is alive, no other token (read or write) can coexist | Zero-sized and linear (non-Dup)—copying is meaningless under exclusive access                         |

> **Causal ordering**: Dup for a ReadToken is a consequence of the freeze guarantee, not the other
> way around. The data is frozen (mutation is impossible) → multiple read-only views are safe → Dup
> can be implemented. Treating Dup as the definition and conflict checking as a patch reverses the
> causality.

### 3.3 ref Sharing

`ref` is the only way to share across scopes. The compiler automatically selects `Rc` (single task)
or `Arc` (across tasks), so users do not need to worry about it.

```yaoxiang
data = load_data()
shared = ref data       // The compiler automatically selects Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

**Compiler selection strategy**:

| Condition                                                     | Selection | Reason                          |
| ------------------------------------------------------------- | --------- | ------------------------------- |
| Default (safety cannot be proven)                             | `Arc`     | Safety first; avoids data races |
| The compiler can prove that the data is used by only one task | `Rc`      | No atomic-operation overhead    |

**ref vs. borrow tokens**:

|              | `&T` / `&mut T`                 | `ref`                                               |
| ------------ | ------------------------------- | --------------------------------------------------- |
| What it does | Look at it / modify it in place | Shared ownership                                    |
| Cost         | Zero overhead (zero-sized type) | `Rc` or `Arc` (selected by the compiler)            |
| Across tasks | Not possible                    | Possible (the compiler automatically selects `Arc`) |

### 3.4 Closures and Captures

**Closures do not implicitly capture outer variables** (RFC-009 resolution of 2026-06-16, SPEC
§12.3). A lambda uses only its explicit parameters and its own local variables. When outer data is
needed, pass it as an explicit parameter or fix it at the creation point through currying. The
compile error code for implicit capture is E1001.

> Why this is prohibited: After a closure escapes, the outer scope in which it was defined may no
> longer be alive, so references captured implicitly cannot be guaranteed to remain alive. A value
> fixed through currying is evaluated at its creation point (while the call-site scope is alive),
> which is safe and has no hidden cost.

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // ❌ Compile error E1001: implicit capture of data
```

```yaoxiang
// ✅ Correct approach 1: pass through an explicit parameter
add: (data: Data, x: Int) -> Int = data.value + x

// ✅ Correct approach 2: fix through currying (the context is fixed at the creation point; the closure only takes parameters)
mk: (data: Data) -> (x: Int) -> Int = (data) => (x) => data.value + x
```

**Sharing within `spawn`**: To share across tasks, explicitly share ownership with `ref` (RFC-024):

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

**There is no `spawn while` construct**: After `spawn`, only `for` or `{` may follow (the
`parse_spawn` function at `src/frontend/core/parser/pratt/nud.rs:117-136` dispatches only `KwFor` →
`parse_spawn_for` and `LBrace` → `parse_spawn_block`), and the AST likewise has only two variants:
`Expr::Spawn` (blocks) and `Expr::SpawnFor` (data-parallel loops,
`src/frontend/core/parser/ast.rs:68-72`). To "repeat conditionally" in a concurrent context, write
an ordinary `while` loop.

The `spawn for` iteration variable is mutable by default; writing `spawn for mut x in items` enables
the binding to be reassigned within the loop body (the `var_mut` field at
`src/frontend/core/parser/ast.rs:71`, which is used to register mutability by
`src/frontend/core/typecheck/layers/ownership.rs:1660-1671`).

---

## Chapter 4: Error Handling

### 4.1 The `?` Operator

The `?` operator is used for explicit error propagation and is consistent with Rust semantics.

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // If an error occurs, propagate it immediately
    return content.read_all()
}
```

### 4.2 Error Propagation Within spawn Blocks

**Rules**:

1. Wait for all tasks to complete, even if some tasks have failed
2. Propagate the first error encountered
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // May fail
    fetch("url2")?      // May fail
}
// If either task fails, the entire spawn block propagates the first error
```

### 4.3 Error Types

**Automatically generated**: The compiler automatically generates a union error type.

```yaoxiang
// The compiler infers the error type as HttpError | IoError
(a, b) = spawn {
    fetch("url"),           // May raise HttpError
    read_file("data.txt")  // May raise IoError
}
```

**Manual override**: Users can manually define a unified error type. Note that `Result` **does not
have `map_err`** (it is absent from the 8 exports at `src/std/result.rs:71-126`, and
`result.map_err(...)` reports `E1042`), so the upstream error must be moved into an `AppError`
variant before using `?`:

```
AppError: Type = {
    Http: (http_error: HttpError) -> AppError,
    Io: (io_error: IoError) -> AppError,
    Parse: (parse_error: ParseError) -> AppError
}

// Conceptual form (map_err is unavailable; use match to move the error explicitly)
process: (url: String, path: FilePath) -> Result(Data, AppError) = {
    fetched = match fetch(url) {
        ok(v) -> v
        err(e) -> return AppError.Http(e)
    }
    content = match read_file(path) {
        ok(v) -> v
        err(e) -> return AppError.Io(e)
    }
    return parse(fetched + content)
}
```

---

## Chapter 5: Resource Types and Side Effects

### 5.1 Built-in Resource Types

| Resource type | Description         | Compiler behavior                                              |
| ------------- | ------------------- | -------------------------------------------------------------- |
| `FilePath`    | Filesystem path     | Operations on the same path are automatically serialized       |
| `HttpUrl`     | HTTP endpoint       | Operations on the same URL are automatically serialized        |
| `DBUrl`       | Database connection | Operations on the same connection are automatically serialized |
| `Console`     | Standard output     | All Console operations are automatically serialized            |

```yaoxiang
// Operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for the read to complete
}
```

### 5.2 User-defined Resource Types

User-defined resource types must be explicitly marked.

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 Side-effect Tracking

The compiler tracks the use of resource types to ensure concurrency safety.

```yaoxiang
// Compiler warning: Console operations may interleave
spawn {
    print("Hello"),     // May interleave with the next line
    print("World")
}

// Correct: explicitly serial
spawn {
    print("Hello\nWorld")
}
```

---

## Chapter 6: Compiler Behavior

### 6.1 DAG Analysis

At compile-time, the compiler analyzes the dependency relationships (DAG) within a spawn block to
determine:

1. Which expressions can run in parallel
2. Which expressions must run serially
3. How to allocate tasks

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // Task 1
    y = fetch("url2"),      // Task 2 (runs in parallel with Task 1)
    z = process(x, y)       // Task 3 (depends on x and y and must wait)
}
```

### 6.2 Rc/Arc Selection (Conservative Strategy)

The compiler uses a **conservative strategy**, defaulting to `Arc` to ensure thread safety:

- **Default to `Arc`**: When the compiler cannot determine whether a `ref` is used only within a
  single task, it conservatively selects `Arc`
- **Downgrade to `Rc`**: It downgrades to `Rc` only when DAG analysis can **prove** that the data
  will never be shared across tasks
- **Rather slow than wrong**: The additional overhead of `Arc` is far smaller than the risk of a
  data race

### 6.3 No-Parallelism Warning

If the tasks in a spawn block have no actual opportunity for parallelism, the compiler emits a
warning.

```yaoxiang
// Compiler warning: no opportunity for parallelism
result = spawn {
    a = fetch("url")    // The only task
}
// Recommendation: use ordinary code directly
result = fetch("url")
```

### 6.4 Resource Conflict Detection

The compiler detects potential conflicts between resource types.

```yaoxiang
// Compile error: concurrent writes to the same file
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // Error!
}
```

---

## Chapter 7: Runtime Tiers

The compilation stage is exactly the same; the only difference is how execution is performed at
runtime (RFC-008).

| Tier             | spawn support | DAG analysis                        | Use cases                                     |
| ---------------- | ------------- | ----------------------------------- | --------------------------------------------- |
| Embedded Runtime | ❌            | None                                | WASM, game scripts, rule engines              |
| Standard Runtime | ✅            | Within spawn blocks                 | Web services, data pipelines                  |
| Full Runtime     | ✅            | Within spawn blocks + work stealing | Scientific computing, large-scale parallelism |

**Embedded Runtime**: An immediate executor with no spawn support, offering high performance and low
overhead.

**Standard Runtime**: Supports `spawn {}` blocks and performs DAG analysis and automatic concurrency
within spawn blocks. `num_workers=1` means single-threaded mode.

**Full Runtime**: Standard plus WorkStealer load balancing.

---

## Appendix: Syntax Quick Reference

### A.1 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Assignment '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
SpawnBody   ::= Assignment (',' Assignment)*
```

### A.2 Error Handling

```
Expr '?'              // Error propagation (Result type)
```

### A.3 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

### A.4 Resource Type Declarations

```
ResourceDecl ::= Identifier ':' 'Type' '=' RecordType
```
