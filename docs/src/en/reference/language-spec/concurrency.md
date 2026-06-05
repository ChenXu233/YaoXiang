# Concurrency Model Specification

> **Status**: This document describes the new concurrency model design for the YaoXiang language. It replaces the old concurrency scheme based on `@block`/`@eager`/`@auto` annotations, `Send`/`Sync` trait and `Mutex`/`RwLock`. Some content is not yet implemented; actual compiler behavior takes precedence.

This document defines the concurrency model specification for the YaoXiang programming language, including `{}` block semantics, `spawn` concurrency primitive, error handling, and resource types.

---

## Chapter 1: Overview

### 1.1 Essence of `{}` Blocks

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property | Description |
|----------|-------------|
| Dependency-driven | The block checks whether all internal variables are ready when executing; if ready, it executes immediately; otherwise, it blocks and waits |
| Execution timing | Determined by dependencies, unrelated to "immediate" or "delayed" |
| Return value | Use `return` to explicitly return a value; defaults to `Void` when no `return` is present |
| Unified syntax | Consistent semantics whether appearing in function body, variable initialization, or after `spawn` |
| Scoped isolation | Variables are strictly limited to inside `{}`, not leaking to outer scope |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y, executes immediately once both are ready
    return x + y
}
```

### 1.2 Return Rules

YaoXiang's return rules are unified and clear:

| Syntax | Return value | Description |
|--------|--------------|-------------|
| `= expr` (no braces) | Directly returns `expr` | Expression is value |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` | Block requires explicit return |

```yaoxiang
// No braces: direct return
add: (a: Int, b: Int) -> Int = a + b

// With braces: must use return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// With braces but no return: returns Void
log: (message: String) -> Void = {
    print(message)  // No return, returns Void
}
```

### 1.3 spawn Block Semantics

`spawn { ... }` is the only parallel primitive in YaoXiang.

**Core rules**:
- Direct child assignments in a spawn block create parallel tasks
- Assignments inside nested `{}` are not considered independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning results
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks execute in parallel
(a, b) = spawn {
    fetch("url1"),      // Task 1
    fetch("url2")       // Task 2
}
// Continues after both complete
```

### 1.4 User Mental Model

> The ordinary code you write executes sequentially.
> When you want multiple things to happen together, put them in a `spawn { ... }` block.
> Each direct assignment in the block starts immediately (in parallel), and the results you need are automatically awaited.
> The entire block waits for everything to finish, then gives you the final result.
> No callbacks, no `await`, no weird annotations.

---

## Chapter 2: Syntax and Semantics

### 2.1 Ordinary Code

Ordinary code (outside spawn blocks) executes **sequentially**.

```yaoxiang
a = compute_a()     // Executes first
b = compute_b(a)    // Depends on a, executes after a completes
c = compute_c(b)    // Depends on b, executes after b completes
```

### 2.2 spawn Blocks

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' SpawnBody '}'
SpawnBody   ::= Assignment (',' Assignment)*
```

**Semantics**:
1. Direct child assignments inside spawn block execute as independent parallel tasks
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

A function body itself is a `{}` block, where `spawn` can be used.

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

**Semantics**: Data-parallel loop, where each iteration is an independent task.

```yaoxiang
// Process each element in the list in parallel
results = spawn for item in items {
    result = process(item)
}
```

> **Note**: The loop body of `spawn for` is an independent task and does not support sharing mutable state across iterations. If you need to aggregate results, use `spawn for` to collect results and process externally (see example below).

```yaoxiang
// Correct: aggregate externally after parallel processing
transformed = spawn for item in items {
    result = transform(item)
}
total = sum(transformed)   // Sequential aggregation
```

### 2.5 Nested spawn

spawn blocks can be nested, with inner spawn creating new concurrency domains.

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**Note**: Only direct child assignments of inner spawn are tasks; outer spawn does not penetrate through.

---

## Chapter 3: Interaction with Ownership Model

### 3.1 Move Semantics

Move is YaoXiang's default semantics (zero-copy).

```yaoxiang
data = load_data()
result = spawn {
    process(data)   // Ownership of data moves into spawn block
}
// data is unavailable here (already moved)
```

**Rules**:
- Once a variable enters a spawn block, it cannot be used outside
- If sharing across multiple tasks is needed, use `ref`

### 3.2 Borrow Tokens

`&T` and `&mut T` are zero-sized compile-time permission proofs, **cannot cross task boundaries**.

```yaoxiang
data = load_data()
ref_data = &data

// Compile error: borrow token cannot cross task boundary
result = spawn {
    process(ref_data)   // Error!
}
```

### 3.3 ref Sharing

`ref` is the only way to share across scopes.

```yaoxiang
data = load_data()
shared = ref data       // Compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

**Compiler selection (conservative strategy)**:
| Condition | Choice |
|-----------|--------|
| Default | `Arc` (safety first) |
| Compiler can prove usage only within a single task | `Rc` (no atomic operation overhead) |

### 3.4 Closure Capture

Closure capture = Move; a closure can only be used by one task.

```yaoxiang
data = load_data()
fn = (x: Int) -> Int = data.value + x   // Closure move-captures data

// Compile error: closure can only be used by one task
result = spawn {
    fn(1),      // Use closure
    fn(2)       // Error! Closure already moved
}
```

**Correct approach**: Create independent closures for each task or use `ref`.

```yaoxiang
data = load_data()
shared = ref data

result = spawn {
    ((x: Int) -> Int = shared.value + x)(1),
    ((x: Int) -> Int = shared.value + x)(2)
}
```

---

## Chapter 4: Error Handling

### 4.1 `?` Operator

The `?` operator is used for explicit error propagation, consistent with Rust semantics.

```yaoxiang
read_file: (path: FilePath) -> Result(String, IoError) = {
    content = open(path)?      // If error, propagate immediately
    return content.read_all()
}
```

### 4.2 Error Propagation within spawn Blocks

**Rules**:
1. Wait for all tasks to complete (even if some tasks have failed)
2. Propagate the first encountered error
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // May fail
    fetch("url2")?      // May fail
}
// If either task fails, the entire spawn block propagates the first error
```

### 4.3 Error Types

**Auto-generation**: The compiler automatically generates union error types (similar to TypeScript union types).

```yaoxiang
// Compiler infers error type as HttpError | IoError
(a, b) = spawn {
    fetch("url"),           // May throw HttpError
    read_file("data.txt")  // May throw IoError
}
```

**Manual override**: Users can manually define a unified error type.

```yaoxiang
AppError: Type = {
    Http: (HttpError) -> Self,
    Io: (IoError) -> Self,
    Parse: (ParseError) -> Self
}

process: (url: String, path: FilePath) -> Result(Data, AppError) = {
    (a, b) = spawn {
        fetch(url).map_err(AppError.Http)?,
        read_file(path).map_err(AppError.Io)?
    }
    return parse(a + b).map_err(AppError.Parse)?
}
```

---

## Chapter 5: Resource Types and Side Effects

### 5.1 Built-in Resource Types

| Resource type | Description | Compiler behavior |
|---------------|-------------|-------------------|
| `FilePath` | File system path | Operations on the same path are automatically serialized |
| `HttpUrl` | HTTP endpoint | Operations on the same URL are automatically serialized |
| `DBUrl` | Database connection | Operations on the same connection are automatically serialized |
| `Console` | Standard output | All Console operations are automatically serialized |

```yaoxiang
// Operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for read to complete
}
```

### 5.2 User-Defined Resource Types

User-defined resource types require explicit marking.

```yaoxiang
Database: Type = {
    connection_string: String,
    query: (db: Database, sql: String) -> Result(Rows, DbError)
}
```

### 5.3 Side Effect Tracking

The compiler tracks resource type usage to ensure concurrency safety.

```yaoxiang
// Compiler warning: Console operations may interleave
spawn {
    print("Hello"),     // May interleave with next line
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

The compiler analyzes the dependency relationships (DAG) within spawn blocks at compile time to determine:
1. Which expressions can execute in parallel
2. Which must execute sequentially
3. How to schedule tasks

```yaoxiang
(a, b, c) = spawn {
    x = fetch("url1"),      // Task 1
    y = fetch("url2"),      // Task 2 (parallel with Task 1)
    z = process(x, y)       // Task 3 (depends on x and y, must wait)
}
```

### 6.2 Rc/Arc Selection (Conservative Strategy)

The compiler adopts a **conservative strategy**, defaulting to `Arc` to ensure thread safety:

| Condition | Choice | Reason |
|-----------|--------|--------|
| Default (cannot prove safety) | `Arc` | Safety first, avoid data races |
| Compiler can **prove** data is only used within a single task | `Rc` | No atomic operation overhead |

**Strategy explanation**:
- **Default `Arc`**: When the compiler cannot determine whether `ref` is used only within a single task, it conservatively chooses `Arc`
- **Downgrade to `Rc`**: Only when the compiler can **prove** through DAG analysis that data will absolutely not be shared across tasks does it downgrade to `Rc`
- **Better slow than wrong**: The extra overhead of choosing `Arc` is far less than the risk of data races

```yaoxiang
data = load_data()

// Default: compiler chooses Arc (conservative strategy)
result = spawn {
    shared = ref data
    process(shared)
}

// Only when compiler can prove single-task usage: downgrade to Rc
// (requires compiler's DAG analysis to explicitly exclude cross-task possibility)
```

### 6.3 No-Parallelization Warning

If tasks within a spawn block have no actual opportunity for parallelization, the compiler issues a warning.

```yaoxiang
// Compiler warning: no opportunity for parallelization
result = spawn {
    a = fetch("url")    // Only task
}
// Suggestion: use ordinary code directly
result = fetch("url")
```

### 6.4 Resource Conflict Detection

The compiler detects potential conflicts with resource types.

```yaoxiang
// Compile error: concurrent writes to the same file
spawn {
    write_file("data.txt", "a"),
    write_file("data.txt", "b")  // Error!
}
```

---

## Chapter 7: Comparison with Old Design

### 7.1 Deprecated Features

| Old feature | Status | Alternative |
|-------------|--------|-------------|
| `@block`, `@eager`, `@auto` annotations | Deprecated | None; dependency-driven automatically handles |
| Whole-program automatic DAG analysis | Deprecated | Only analysis within spawn blocks |
| `Send`, `Sync` trait | Deprecated | Ownership + ref automatically handles |
| future/non-blocking handles | Deprecated | spawn blocks synchronously return |
| `Mutex[T]`, `Atomic[T]`, `RwLock[T]` | Deprecated | ref automatically selects Rc/Arc |

### 7.2 Design Philosophy Shift

**Old model**:
- Explicit annotations control concurrency behavior
- Complex trait constraints
- Asynchronous programming model

**New model**:
- Dependency-driven, implicit concurrency
- Ownership + ref simplifies sharing
- Synchronous programming model, spawn blocks block and return

### 7.3 Migration Guide

```yaoxiang
// Old code (pseudocode, demonstrating old model style; @block/let/await are not YaoXiang keywords)
@block async fetch_data(): Future<Data> = {
    let data = await fetch("url")
    return data
}

// New code
fetch_data: () -> Data = {
    data = fetch("url")     // Synchronous call
    return data
}

// Concurrent version
fetch_multiple: (urls: List(String)) -> List(Data) = {
    results = spawn for url in urls {
        result = fetch(url)
    }
    return results
}
```

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