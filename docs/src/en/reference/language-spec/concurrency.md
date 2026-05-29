# Concurrency Model Specification

This document defines the concurrency model specification for the YaoXiang programming language, including asynchronous programming, concurrency primitives, and the memory model.

---

## Chapter 1: Spawn Functions

### 1.1 Spawn Functions

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**Function Annotations**:

| Annotation | Position | Behavior |
|------------|----------|----------|
| `@block` | After return type | Disables concurrency optimization, fully sequential execution |
| `@eager` | After return type | Forces eager evaluation |

**Syntax Examples**:

```
// Spawn function: can execute concurrently
fetch_data: (url: String) -> JSON spawn = { ... }

// @block synchronous function: fully sequential execution
main: () -> Void @block = { ... }

// @eager eager function: executes immediately
compute: (n: Int) -> Int @eager = { ... }
```

### 1.2 Spawn Blocks

Explicitly declared concurrency boundaries where tasks within the block execute concurrently:

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**Examples**:

```
// Spawn block: explicit concurrency
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

### 1.3 Spawn Loops

Data-parallel loops where the loop body executes concurrently across all data elements:

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**Examples**:

```
// Spawn loop: data parallelism
results = spawn for item in items {
    process(item)
}
```

### 1.4 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

**Examples**:

```
process: (p: Point) -> Result[Data, Error] = {
    data = fetch_data()?      // Automatically propagates errors
    transform(data)?
}
```

---

## Chapter 2: Memory Management

### 2.1 Ownership Model

YaoXiang uses an **ownership model** to manage memory, where each value has a unique owner:

| Semantic | Description | Syntax |
|----------|-------------|--------|
| **Move** | Default semantic, ownership transfer | `p2 = p` |
| **ref** | Shared (Arc reference counting) | `shared = ref p` |
| **clone()** | Explicit copy | `p2 = p.clone()` |

### 2.2 Move Semantics (Default)

```yaoxiang
// Assignment = Move (zero-copy)
p: Point = Point(1.0, 2.0)
p2 = p              // Move, p is invalidated

// Function parameter = Move
process: (p: Point) -> Void = {
    // Ownership of p is transferred in
}

// Return value = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        // Move, ownership is transferred
}
```

### 2.3 ref Keyword (Arc)

The `ref` keyword creates a **reference-counted pointer** (Arc) for safe sharing:

```yaoxiang
// Create Arc
p: Point = Point(1.0, 2.0)
shared = ref p      // Arc, thread-safe

// Shared access
spawn(() => print(shared.x))   // Safe

// Arc automatically manages lifecycle
// When shared goes out of scope, count reaches zero and is freed
```

**Characteristics**:
- Thread-safe reference counting
- Automatic lifecycle management
- Safe across spawn boundaries

### 2.4 clone() Explicit Copy

```yaoxiang
// Explicitly copy a value
p: Point = Point(1.0, 2.0)
p2 = p.clone()      // p and p2 are independent

// Both can be modified without affecting each other
p.x = 0.0           // Correct
p2.x = 0.0          // Correct
```

### 2.5 unsafe Code Blocks

`unsafe` code blocks allow the use of raw pointers for system-level programming:

```yaoxiang
// Raw pointer type
PtrType ::= '*' TypeExpr

// unsafe code block
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'
```

**Examples**:

```yaoxiang
p: Point = Point(1.0, 2.0)

// Raw pointers can only be used in unsafe blocks
unsafe {
    ptr: *Point = &p     // Get raw pointer
    (*ptr).x = 0.0       // Dereference
}
```

**Restrictions**:
- Raw pointers can only be used in `unsafe` blocks
- User guarantees no dangling pointers or use-after-free
- Not subject to Send/Sync checks

### 2.6 Ownership Syntax BNF

```bnf
// === Ownership Expressions ===

// Move (default)
MoveExpr     ::= Expr

// ref Arc
RefExpr      ::= 'ref' Expr

// clone
CloneExpr    ::= Expr '.clone' '(' ')'

// === Raw Pointers (unsafe only) ===

PtrType       ::= '*' TypeExpr
UnsafeBlock   ::= 'unsafe' '{' Stmt* '}'
```

---

## Chapter 3: Concurrency Safety

### 3.1 Send / Sync Constraints

| Constraint | Semantic | Description |
|------------|----------|-------------|
| **Send** | Can be safely transferred across threads | Value can be moved to another thread |
| **Sync** | Can be safely shared across threads | Immutable references can be shared to another thread |

**Automatic Derivation**:

```
// Send derivation rule
Struct[T1, T2]: Send ⇐ T1: Send 且 T2: Send

// Sync derivation rule
Struct[T1, T2]: Sync ⇐ T1: Sync 且 T2: Sync
```

**Type Constraints**:

| Type | Send | Sync | Description |
|------|------|------|-------------|
| `T` (value) | ✅ | ✅ | Immutable data |
| `ref T` | ✅ | ✅ | Arc thread-safe |
| `*T` | ❌ | ❌ | Raw pointer unsafe |

### 3.2 Send/Sync Constraint Hierarchy

```
Send ──► Can be safely transferred across threads
  │
  └──► Sync ──► Can be safely shared across threads
       │
       └──► Types satisfying Send + Sync can auto-concurrent

Arc[T] implements Send + Sync (thread-safe reference counting)
Mutex[T] provides interior mutability
```

### 3.3 Concurrency-Safe Types

| Type | Semantic | Concurrency Safe | Description |
|------|----------|------------------|-------------|
| `T` | Immutable data | ✅ Safe | Default type, multi-task reads without race |
| `Ref[T]` | Mutable reference | ⚠️ Needs sync | Marked for concurrent modification, compiler checks lock usage |
| `Atomic[T]` | Atomic type | ✅ Safe | Low-level atomic operations, lock-free concurrency |
| `Mutex[T]` | Mutex wrapper | ✅ Safe | Auto lock/unlock, compiler guaranteed |
| `RwLock[T]` | Read-write lock wrapper | ✅ Safe | Optimized for read-heavy scenarios |

**Syntax**:

```
Mutex[T]    // Mutex-wrapped mutable data
Atomic[T]   // Atomic type (only for Int, Float, etc.)
RwLock[T]   // Read-write lock wrapper
```

**with Syntax Sugar**:

```
with mutex.lock() {
    // Critical section: protected by Mutex
    ...
}
```

---

## Appendix: Concurrency Syntax Quick Reference

### A.1 Spawn Syntax

```yaoxiang
// Spawn function
fetch_data: (url: String) -> JSON spawn = { ... }

// Spawn block
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}

// Spawn loop
results = spawn for item in items {
    process(item)
}
```

### A.2 Ownership Syntax

```yaoxiang
// Move (default)
p2 = p

// ref Arc
shared = ref p

// clone
p2 = p.clone()

// unsafe
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### A.3 Concurrency-Safe Types

```yaoxiang
// Mutex
mutex: Mutex[Int] = Mutex(0)
with mutex.lock() {
    // Critical section
}

// Atomic type
counter: Atomic[Int] = Atomic(0)
counter.increment()

// Read-write lock
data: RwLock[Data] = RwLock(data)
with data.read() {
    // Read operation
}
```