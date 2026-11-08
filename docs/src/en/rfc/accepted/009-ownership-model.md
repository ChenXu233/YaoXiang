---
title: 'RFC-009: Ownership Model Design'
status: 'Accepted'
author: 'Chenxu'
created: '2025-01-08'
updated:
  '2026-06-13 (token conflict detection corrected to the Hoare proposition; body synchronized with
  RFC-009a)'
issue: '#126'
---

# RFC-009: Ownership Model Design

## Abstract

This document defines the **Ownership Model** of the YaoXiang programming language.

**Core design—five concepts, one gradient**:

```
看一眼/原地改     拿走           共享持有         复制一份        系统级
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         零拷贝        编译器自动      显式深拷贝      *T
  零大小令牌       默认          选Rc/Arc                   用户负责
  类型属性自然
  推导权限
```

- **Move (default)**: assignment/argument passing/return = ownership transfer, zero-copy, with
  automatic release through RAII
- **`&T` / `&mut T` (borrow tokens)**: zero-sized compile-time token types. `&T` is copyable (shared
  read-only access), while `&mut T` is linear (exclusive mutable access). Permissions are derived
  naturally from type attributes, with no special rules required. Tokens may be returned and stored
  in structs.
- **`ref` keyword**: cross-scope sharing. The compiler automatically chooses between Rc (when not
  crossing tasks) and Arc (when crossing tasks)
- **`clone()`**: explicit deep copy
- **`unsafe` + `*T`**: raw pointers, a system-level escape hatch

**Complexity eliminated**:

- ❌ No lifetime `'a`
- ❌ No standalone borrow-checking framework (borrow conflicts are reduced to Hoare propositions and
  share a proof pipeline with type checking)
- ❌ No GC
- ❌ No special rules such as "prohibiting escape" (tokens are ordinary types, and scopes are
  handled uniformly by the type system)
- ❌ Users do not need to know the difference between Rc/Arc (the compiler selects automatically)

> **Programming burden**: `&T` is copyable, `&mut T` is not—two type attributes, zero special rules,
> fully automated by the compiler. **Performance guarantees**: Move has zero overhead, tokens have
> zero overhead (zero-sized types that disappear after compilation), ref incurs costs only when
> needed, and there are no GC pauses.

> **Integration with quantifiers**: Move/Dup/Clone are the syntactic surface of the binding
> quantifiers `q ∈ {0, 1, ω}`, specifically 1 (linear) and ω (free); clone() explicitly crosses
> quantifiers. q=0 (zero quantification: compile-time computation and runtime erasure) is driven by
> strength declarations in RFC-011 §4.1 and follows the same binding discipline as 1/ω—ownership and
> compile-time nature are three readings of the same rule.

## Motivation

### Why is an ownership model needed?

| Language     | Memory management           | Problem                                               |
| ------------ | --------------------------- | ----------------------------------------------------- |
| C/C++        | Manual management           | Memory leaks, dangling pointers, double frees         |
| Java/Python  | GC                          | Latency spikes, memory overhead, unpredictable pauses |
| Rust         | Ownership + borrow checking | Steep learning curve for lifetime `'a`                |
| **YaoXiang** | **Move + Token + ref**      | **Simple, deterministic, GC-free**                    |

### Design goals

```yaoxiang
# 1. Move by default (zero-copy)
p = Point(1.0, 2.0)
p2 = p                         # Move; p can no longer be read

# 2. &T / &mut T borrow tokens (zero overhead; permissions derived naturally from type attributes)
print_info(p2)                 # The compiler automatically creates an &Point token and releases it when finished
shift(p2, 1.0, 1.0)           # The compiler automatically creates an &mut Point token

# 3. ref = sharing (the compiler automatically chooses Rc/Arc)
shared = ref p                # Held across scopes
spawn { use(shared) }          # Compiler: cross-task → Arc

# 4. clone() = explicit copy
backup = p2.clone()            # Deep copy with exclusive ownership

# 5. unsafe + *T = system level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### Key differences from Rust

| Feature           | Rust                                               | YaoXiang                                                                                                |
| ----------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Default semantics | Borrowing `&T` (requires explicit `.clone()`)      | **Move (value passing, zero-copy)**                                                                     |
| Borrowing         | `&T`/`&mut T`; can be returned; requires lifetimes | **`&T`/`&mut T` zero-sized tokens; permissions derived naturally from Dup/Linear type attributes**      |
| Sharing mechanism | `Arc::new()` + manually managed Weak               | **`ref` keyword (compiler automatically chooses Rc/Arc)**                                               |
| Copying           | `clone()`                                          | `clone()`                                                                                               |
| Raw pointers      | `*T`                                               | `*T`                                                                                                    |
| Lifetimes         | `'a`                                               | ❌ None                                                                                                 |
| Borrow checking   | Global inference                                   | **The type checker automatically generates borrow propositions, validated by a unified proof pipeline** |
| Cyclic references | Manually managed Weak                              | **Unified release at task completion / cross-task lint / standard-library Weak**                        |

---

## Proposal

### 1. Move (Default Ownership Transfer)

```yaoxiang
# Rule: assignment / argument passing / return = Move, zero-copy

p: Point = Point(1.0, 2.0)
p2 = p                           # Move; p can no longer be read

# Variables can be reassigned (Python style, without shadowing)
p = Point(3.0, 4.0)              # p is rebound; the type must remain the same

# Function argument: Move
process: (p: Point) -> Point = {
    p.transform()
    p                            # Move return
}

# Function return: Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    p                            # Move return, zero-copy
}
```

**Characteristics**:

- Zero-copy (the compiler moves the pointer)
- The original binding cannot be read after the move (compile error)
- RAII: automatically released when the scope ends
- The function signature `(T) -> T` is itself documentation—it consumes T and returns T

---

### 2. &T / &mut T (Borrow Tokens)

**Core principle: `&T` and `&mut T` are zero-sized compile-time token types. They are not
"references" but "type-level proofs of access permissions".**

#### 2.1 Two type attributes

```
&T      →  零大小，冻结源数据（ReadToken 存活期间禁止 WriteToken），
          冻结保证下多份只读视图安全 → 可复制（Dup）
&mut T  →  零大小，独占读写（WriteToken 存活期间禁止任何其他令牌），
          独占访问下复制无意义 → 线性（非 Dup）
```

**The causal relationship must not be reversed: freezing is the cause, and Dup is the result.** It
is not because `&T` implements Dup that they can coexist—rather, because the data is frozen
(mutation is impossible), multiple read-only views are safe, and Dup can therefore be implemented.
If Dup is treated as the definition and conflict checking as an "additional patch", the design is
wrong.

#### 2.2 Basic usage

```yaoxiang
# Method side: declare the parameter type to determine the required permission
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # The &Point token grants read permission
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # The &mut Point token grants write permission
    self.y = self.y + dy
}

# Call side: the compiler automatically chooses borrowing or Move
p = Point(1.0, 2.0)
p.print()                          # The compiler automatically creates an &Point token
p.shift(1.0, 1.0)                  # The compiler automatically creates an &mut Point token
p.print()                          # OK; the previous token was released when the shift call ended

# The same applies to free functions
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # Two &Point tokens coexist—a Dup type
}
d = distance(p, p2)
```

#### 2.3 Why no "prohibition on escape" is needed

RFC-009 v8 imposed three special rules on `&T`/`&mut T`: they could only be used as arguments, could
not be returned, and could not be stored in structs. This was patching up the concept of
"borrowing".

The token system does not need these rules. Tokens are **ordinary types** and follow the same scope
rules as every other type.

**Returning references—naturally supported**:

```yaoxiang
# ✅ The token propagates together with the return value
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # The subtoken and parent token are returned together
}

# Usage
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # The token is returned to the caller
print(px_ref)               # OK; the token is still in scope
```

**Storing tokens in structs—naturally supported**:

```yaoxiang
# ✅ The struct carries a token as a field
Window: Type = {
    target: Point,
    view: &Point,      # Token field—holds a read-only view of target
}

# The view token is derived from target, and Window owns both
# As long as Window exists, the view token remains valid
```

#### 2.3 Closures and Lambda Explicit Parameters

A Lambda is a function value—it may be returned, stored, or passed out of the current scope.
Therefore, a Lambda **does not implicitly capture outer local variables**. When outer data is
needed, pass it in through an explicit parameter:

```yaoxiang
# ✅ Lambda uses explicit parameters
double: (x: Int) -> Int = (x) => x * 2
filter_by: (items: List(Int), f: (Int) -> Bool) -> List(Int) = { ... }

# ✅ spawn { } is unaffected by this rule—spawn is an immediately executed concurrent block, and the parent task blocks while waiting
shared = ref data
spawn { use(shared) }

# ❌ A Lambda cannot implicitly capture outer variables
x = 42
f = () => { x + 1 }  # Compile error: x is not in scope

# ✅ Correct approach: pass arguments explicitly
f = (x) => { x + 1 }
f(x)

# ✅ Correct approach 2: the context is fixed at the creation point (currying)—the closure consumes only arguments and does not capture
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
evens = list.filter(nums, gt(threshold))
```

> Supplement (2026-08-17): The correct solution for context dependencies is fixed values through
> currying, not capture. After a closure escapes, the scope where it was defined may no longer be
> alive, so implicit capture is forbidden. However, the scope at the call site (the creation point)
> is necessarily alive; fixing the context there as a value that enters the closure is safe. See
> SPEC §12.3.

**spawn { } is not a function value.** A block marked with spawn behaves like an if/while body—it
executes immediately and completes while the parent stack frame remains alive. A spawn body can
normally access outer variables.

**Cross-task—tokens cannot cross task boundaries**:

```yaoxiang
# ❌ Tokens cannot cross task boundaries
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ Compile error: a token cannot be passed across tasks
}

# This is not a special rule—tokens are compile-time permission proofs; use ref for cross-task sharing
# If cross-task sharing is needed, use ref
```

**Tokens cannot be ref**:

```yaoxiang
# ❌ A token is a permission proof, not ownership
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ Compile error: &T is not an ownable type
}
```

#### 2.4 Token lifetime

A token's lifetime is determined by **ordinary scope rules** and requires no lifetime parameters:

- A token used as a function parameter lives for the duration of the call and is released afterward
- A returned token has its ownership transferred to the caller
- A token stored in a struct lives for as long as the struct

The compiler does not need `'a` annotations because tokens are **values**, whose lifetimes are
managed uniformly by the ownership system (Move/RAII). **The borrowing problem is reduced to an
ownership problem.**

#### 2.5 Token conflict detection

Token conflict detection is a **Hoare logical proposition**, not an independent flow-sensitive
analysis.

```
{冲突的 ReadToken 全部死亡} write(data) {WriteToken 安全获取}
```

It shares RFC-027's proof pipeline with type checking and user predicate verification. The compiler
automatically generates borrow propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`,
`mut_violation`) and sends them to the pipeline for validation. The pipeline returns Proved /
Disproved / Unproven.

```yaoxiang
# ❌ &mut tokens are linear and cannot be copied
bad_dup: (p: &mut Point) -> Void = {
    p2: &mut Point = p              # Move; p can no longer be read
    p.x = 10.0                      # ❌ Compile error: WriteToken has been moved
}

# ✅ &T tokens are Dup types and can be copied freely
good_dup: (p: &Point) -> Void = {
    p2: &Point = p                  # OK; &T is a Dup type
    print(p.x)                      # OK
    print(p2.x)                     # OK; two read-only tokens coexist
}
```

**Borrow checking has not disappeared—it has been reduced in dimensionality.** The existing
`BorrowChecker` becomes a `BorrowPredicateEmitter` (proposition emitter), and the generated borrow
propositions share the same proof pipeline as all other type propositions. This is exactly parallel
to the type checker's model: the type checker generates type-equality propositions, the borrow
proposition emitter generates borrow propositions, and the same pipeline validates them. For
details, see [RFC-009a](009a-borrow-proof-pipeline.md).

#### 2.7 Compiler internals: the brand mechanism

Users never encounter brands. Internally, the compiler assigns every token a unique compile-time
identifier:

```
用户看到的         编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N 是编译期唯一整数
&mut Point     →  WriteToken(Point, #M)   // #M 是编译期唯一整数
```

Brands serve the following purposes:

- **Anti-forgery**: tokens can only be obtained from an owning capsule and cannot be fabricated
- **Relationship tracking**: when deriving `&Float` from `&Point` through field access, `&Float`
  carries a derived brand (`#N.field_x`), allowing the compiler to trace it back to the parent token
- **Conflict detection**: a `WriteToken` and a derived `ReadToken` from the same source cannot be
  active simultaneously

Brands disappear completely after monomorphization and inlining and do not exist in the generated
machine code. **Zero runtime overhead.**

#### 2.8 Automatic borrowing selection rules

At call sites, the compiler automatically selects according to the following priorities:

```
1. 如果实参后续还有使用 → 优先创建令牌（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
# Example: automatic selection
p = Point(1.0, 2.0)
p.print()        # print declares &self → the compiler creates an &Point token
p.shift(1.0, 1.0) # shift declares &mut self → the compiler creates an &mut Point token
p2 = p           # Move; p is no longer used
```

#### 2.9 Comparison with RFC-009 v8's bare-bones borrowing

| Feature                    | Bare-bones borrowing (v8)                           | Borrow tokens (v9)                                       |
| -------------------------- | --------------------------------------------------- | -------------------------------------------------------- |
| Returning references       | ❌ Hard-coded prohibition                           | ✅ Tokens propagate with return values                   |
| Storing in structs         | ❌ Hard-coded prohibition                           | ✅ Tokens serve as struct fields                         |
| Lambda explicit parameters | ❌ Hard-coded prohibition                           | ✅ Lambdas use explicit parameters                       |
| Special rules              | 3 rules (argument-only/non-returnable/non-storable) | 0—permissions derived naturally from type attributes     |
| Borrow checking            | Dedicated cross-borrow checking                     | Type checker's flow-sensitive liveness analysis          |
| Lifetime annotations       | Not required                                        | Not required                                             |
| Runtime overhead           | Zero                                                | Zero (zero-sized types that disappear after compilation) |
| Error message              | "Borrows cannot escape"                             | "WriteToken(#3) has been moved" (an ordinary type error) |
| User mental model          | Understanding the special status of "borrowing"     | `&T` is copyable, `&mut T` is not copyable               |

---

### 3. The ref keyword (automatic compiler optimization)

`ref` is the only way to share across scopes. Users do not need to care whether the underlying
implementation is Rc or Arc.

#### 3.1 Basic usage

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # Shared; the compiler automatically chooses the implementation

# Cross-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }           # Compiler: cross-task → Arc
    spawn { use(data) }           # Compiler: cross-task → Arc
}

# Single-task sharing
@block
main: () -> Void = {
    data = ref heavy_data
    use(data)                     # Compiler: does not cross tasks → Rc
}
```

**User mental model**: `ref` = shared ownership. That's all.

#### 3.2 Compiler escape analysis: Rc vs Arc

```
ref 的数据流分析：

不逃逸到其他任务 → Rc（非原子引用计数，开销低）
逃逸到其他任务   → Arc（原子引用计数，线程安全）
```

#### 3.3 Cycle detection strategy

```
任务内环 → 静默允许。
  ├── 每个任务有明确的生命周期边界——任务结束时所有资源（含 ref 环）统一释放。
  ├── 长期运行的服务应按请求/连接创建子任务——子任务结束自动回收，不会累积泄漏。
  ├── ref 永远保活，语义不掺水。
  └── 用户有权在任务内构建双向强引用（例如图计算中间态）。

跨任务环 → lint（默认 warn，可配置）。
  ├── 程序行为正确，不会真泄漏（父任务结束时子任务资源全释放）。
  ├── 但跨任务强引用意味着所有权边界模糊，值得停下来重新思考。
  ├── 默认 warn 级别，编译通过但有提示。
  └── 团队可在项目配置中设为 deny，纳入 CI 质量门。
```

**Lint levels** (similar to Rust clippy):

| Level            | Behavior                                | Scenario                         |
| ---------------- | --------------------------------------- | -------------------------------- |
| `allow`          | No checking                             | Personal projects                |
| `warn` (default) | Compilation succeeds, with warnings     | Development phase                |
| `deny`           | Compilation fails                       | Team CI quality gate             |
| `forbid`         | Compilation fails; cannot be overridden | Organization-wide mandatory rule |

```yaoxiang
# Intra-task cycle: silently permitted, with bidirectional strong references
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # Cycle. Released together when the task ends.
}

# Cross-task cycle: lint (warn by default)
@block
parent_task: () -> Void = {
    shared_a = ref a
    shared_b = ref b
    spawn {
        shared_a.child = ref shared_b   # ⚠️ warn: cross-task cyclic reference
    }
}
```

**Project configuration example**:

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # Cross-task cycles are rejected outright in CI
```

| Cycle type           | Behavior               | Reason                                                 |
| -------------------- | ---------------------- | ------------------------------------------------------ |
| Intra-task ref cycle | No checking            | A user privilege; released together when the task ends |
| Cross-task ref cycle | lint (warn by default) | Encourages reconsideration; deny can be configured     |

#### 3.4 Weak: provided by the standard library

```yaoxiang
use std.weak

# Advanced users explicitly opt in
a.next = ref b
b.prev = std.weak.new(a.next)   # The user explicitly controls which direction is weak
```

**`Weak` is not built into the language; it is a standard-library type.** For everyday use, `ref` is
sufficient. Advanced users who need fine-grained memory control can manually import `Weak`.

> Revision of 2026-08-03: Implemented as a standalone `std.weak` module (`std.rc` does not
> exist—`ref` is a language keyword rather than a module; the module path is uniformly `std.weak`,
> and the construction/upgrade entry points are `std.weak.new` / `std.weak.upgrade`). The
> `std.rc.Weak` envisioned in the initial draft was not implemented; this revision takes precedence.

#### 3.5 Borrow tokens vs ref

|              | `&T` / `&mut T`                                                                           | `ref`                                            |
| ------------ | ----------------------------------------------------------------------------------------- | ------------------------------------------------ |
| What it does | Take a look/modify in place                                                               | Shared ownership                                 |
| Scope        | Follows the token value's scope                                                           | Across scopes                                    |
| Cost         | Zero overhead (zero-sized types)                                                          | Rc or Arc (compiler-selected)                    |
| Escape       | Allowed (tokens propagate through return values/structs/closures)                         | Intended for escaping                            |
| Cross-task   | Not allowed (tokens are compile-time permission proofs and cannot be passed across tasks) | Allowed (the compiler automatically selects Arc) |
| Cycles       | Not applicable                                                                            | Silently allowed within a task; cross-task lint  |

---

### 4. clone() — Explicit Copying

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # Deep copy
# p and p2 are independent and do not affect each other
```

**When to use it**: when the original value must be retained and Move or sharing is inappropriate.

### 5. unsafe + raw pointers (system-level programming)

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # Raw pointer
    (*ptr).x = 0.0                # Dereference (the user guarantees safety)
    ptr2 = ptr + 1                # Pointer arithmetic
}
```

**Restrictions**:

- May only be used inside an `unsafe` block
- The user guarantees that pointers do not dangle and are not used after being freed
- Used for FFI, memory operations, and other system-level programming

---

### 6. Overview of the Ownership Gradient

```
  借用令牌（零开销）     Move（零开销）      共享（按需付费）    复制
   │                      │                  │                │
  &T 可复制令牌        默认所有权转移     ref Rc/Arc       clone()
  &mut T 线性令牌      链式消费回流       编译器自动选      显式深拷贝
   │                      │                  │                │
  令牌值作用域           作用域内           跨作用域         任何时候
  可返回/存结构体        T -> T 回流        ref 跨任务 → Arc  独立副本
  零大小编译后消失        T -> Void 消费     ref 不跨任务 → Rc
  零大小编译后消失                          任务内环静默
                                            跨任务环 lint
                                            标准库 Weak 逃生
```

---

## Comprehensive Example

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,

    # &T: read-only token
    print: (self: &Point) -> Void = {
        print(self.x)
        print(self.y)
    }

    # &mut T: mutable token
    shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
        self.x = self.x + dx
        self.y = self.y + dy
    }

    # Move → Move: consume and return
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # Take it, modify it, and give it back
    }

    # Return a reference: the token propagates with the return value
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# Lambda explicit parameters
double: (x: Int) -> Int = (x) => x * 2

# Comprehensive usage
p = Point(1.0, 2.0)
p.print()                           # &Point token
p.shift(1.0, 1.0)                   # &mut Point token
p = p.scale(2.0)                    # Move → return
shared = ref p                      # ref sharing
spawn { use(shared) }

# Independent copy from clone
backup = p.clone()

# Intra-task cycle: silently permitted
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # Cycle, released together when the task ends

# unsafe system-level
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## Type System Constraints

### The Dup Type Attribute

`Dup` (Duplicable) is a type attribute automatically managed by the compiler and means a **shallow
copy**: assignment or argument passing copies the handle/token while the underlying data is shared.
This forms a three-level gradient with Move (ownership transfer) and Clone (explicit deep copy that
creates an independent copy).

**Dup and Clone are orthogonal concepts**—Dup copies a handle and shares data, while Clone creates
an independent copy. A type may support both Dup and Clone, or only one of them.

| Type          | Dup                                                                          | Clone | Description                                              |
| ------------- | ---------------------------------------------------------------------------- | ----- | -------------------------------------------------------- |
| `&T`          | ✅ (copies the token; multiple views point to the same data)                 | ✅    | Read-only token                                          |
| `ref T`       | ✅ (reference count +1; shares heap data)                                    | ✅    | Shared ownership (compiler automatically chooses Rc/Arc) |
| String, Bytes | ✅ (internal reference counting; copied handles share the underlying buffer) | ✅    | Strings/bytes                                            |
| `&mut T`      | ❌ (linear, exclusive)                                                       | ❌    | Mutable token                                            |
| `*T`          | ❌                                                                           | ❌    | Raw pointer                                              |
| struct        | Derived (see "Derivation Rules" below, #398)                                 | ✅    | Struct                                                   |
| tuple         | Derived (element by element, following struct rules, #398)                   | ✅    | Tuple                                                    |

The assignment behavior of **primitive value types** (`Int`, `Float`, `Bool`, `Char`) is the
compiler's built-in value copying—the two values are completely independent rather than shallow
copies. They do not have the Dup type attribute; they are handled natively by the compiler.

#### Derivation Rules (Finalized in #398)

“Automatically derive Dup when all fields are Dup” cannot be executed literally—primitive fields
such as Int do not themselves have Dup, so `{ x: Int, y: Int }` would be misclassified as Move. The
executable form is:

1. **Copyable field set** = ValueCopy (Int / Float / Bool / Char / Range) ∪ Dup (`&T`, `ref T`,
   String / Bytes, function values (#352), and composite types that already have Dup);
2. **struct**: all fields are in the copyable field set → derive Dup; **any** field is Linear
   (`&mut T`) or Move (a nested Move struct / containers such as Vec and Dict / resources) → the
   entire type remains Move (no intermediate “partially copyable” state is introduced—under copy
   semantics, it must be transferred);
3. **tuple**: follows the same rules as struct and is determined element by element; the empty tuple
   (unit) is `Void`;
4. **Derivation is recursive**: when a field has a named type (such as `target: Point`), its
   definition is expanded before determination; `A = { b: B }` follows B's derivation result;
   recursive aliases conservatively fall back to Move according to a depth limit;
5. **Outside the scope of this section** (remain Move and are handled separately): containers (Vec /
   Dict / Set / Option / Result / Array) and enums.

---

## Performance Analysis

| Operation                 | Cost                | Description                                                                   |
| ------------------------- | ------------------- | ----------------------------------------------------------------------------- |
| Move                      | Zero                | Pointer movement                                                              |
| `&T` / `&mut T`           | Zero                | Zero-sized types that disappear after compilation, with zero runtime overhead |
| `ref` (no cross-task use) | Low                 | Compiled to Rc, using non-atomic operations                                   |
| `ref` (cross-task)        | Medium              | Compiled to Arc, using atomic operations                                      |
| `clone()`                 | Depends on the type | Fast for small objects, slow for large objects                                |
| `unsafe + *T`             | Zero                | Direct memory operations                                                      |

### Comparison

| Language     | Sharing mechanism             | Memory management   | Cycle handling                                                           | Complexity |
| ------------ | ----------------------------- | ------------------- | ------------------------------------------------------------------------ | ---------- |
| Rust         | Arc / Mutex + borrow checking | Compile-time checks | Manual Weak                                                              | High       |
| Go           | chan / pointer                | GC                  | GC                                                                       | Low        |
| C++          | shared_ptr                    | RAII                | weak_ptr                                                                 | Medium     |
| **YaoXiang** | **ref + borrow tokens**       | **RAII**            | **Release at task boundaries / cross-task lint / standard-library Weak** | **Low**    |

---

## Trade-offs

### Advantages

1. **Unified**: `&T`/`&mut T` are ordinary types rather than special language features. They are
   fully consistent with RFC-010's `name: type = value`
2. **Simple**: no lifetimes; borrow checking is reduced to type-system propositions. `&T` is
   copyable and `&mut T` is not—two type attributes
3. **Powerful**: supports returning references, storing them in structs, and closure
   capture—expressive power on par with Rust
4. **Compiler intelligence**: ref automatically chooses Rc/Arc, and call sites automatically select
   borrowing
5. **Deterministic**: ref keeps data alive and never silently becomes a weak reference
6. **High performance**: Move is zero-copy, and tokens have zero overhead (zero-sized types that
   disappear after compilation)
7. **Flexible**: `unsafe + *T` supports system-level programming

### Disadvantages

1. **Generic brand parameter contagion**: tokens carry brand identifiers, resulting in additional
   generic parameters in the signatures of functions that return references
2. **Runtime overhead for ref**: atomic operations have a cost, although this is an unavoidable
   consequence of sharing
3. **unsafe risks**: users must guarantee correctness
4. **Cross-task cycles produce a lint rather than a compile error**: unlike Rust, the default is
   warn, and teams must configure deny for it to serve as a quality gate

---

## Alternatives

| Alternative               | Why it was not selected                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------ |
| GC                        | Runtime overhead and unpredictable pauses                                                                    |
| Rust's borrow checker     | Requires lifetime `'a` and has a steep learning curve                                                        |
| Move only                 | Cannot handle concurrent sharing                                                                             |
| No raw pointers           | Cannot support system-level programming                                                                      |
| Expose Rc/Arc to users    | Pushes implementation details onto users and increases cognitive burden                                      |
| Bare-bones borrowing (v8) | Its escape-prohibition strategy sacrifices key capabilities such as closure capture and returning references |

---

## Design Decision Record

| Decision                                                                   | Choice                                                                                                                                                   | Reason                                                                                                                                        | Date       |
| -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| **Default**                                                                | Move (zero-copy)                                                                                                                                         | High performance, zero overhead                                                                                                               | 2025-01-15 |
| **Sharing mechanism**                                                      | `ref` keyword with automatic compiler optimization                                                                                                       | Simple for users; handled by the compiler                                                                                                     | 2025-01-15 |
| **Borrowing**                                                              | `&T`/`&mut T` as zero-sized token types                                                                                                                  | Permissions derived naturally from type attributes (Dup/Linear), unified with the type system                                                 | 2025-01-15 |
| **Borrow tokens**                                                          | Replace bare-bones borrowing; `&T` is Dup and `&mut T` is Linear                                                                                         | Eliminate special rules such as "prohibiting escape" and support closure capture, returning references, and storing tokens in structs         | 2026-05-29 |
| **Copying**                                                                | `clone()`                                                                                                                                                | Explicit semantics                                                                                                                            | 2025-01-15 |
| **System level**                                                           | `*T` + `unsafe`                                                                                                                                          | Support systems programming                                                                                                                   | 2025-01-15 |
| **Lifetimes**                                                              | Not implemented                                                                                                                                          | Tokens are values whose lifetimes are managed uniformly by Move/RAII, reducing borrowing to an ownership problem                              | 2025-01-15 |
| **Rc/Arc**                                                                 | Automatically selected by the compiler; invisible to users                                                                                               | Reduce cognitive burden                                                                                                                       | 2025-01-15 |
| **Cyclic references**                                                      | No checking within a task; cross-task lint (warn by default)                                                                                             | Structured concurrency provides the guarantee naturally; lint can be configured to deny                                                       | 2025-01-16 |
| **Weak**                                                                   | Provided by the standard library                                                                                                                         | Explicit opt-in for advanced users                                                                                                            | 2025-01-16 |
| **Consumption analysis**                                                   | Removed                                                                                                                                                  | A miniature borrow checker is unnecessary                                                                                                     | 2026-05-11 |
| **Ownership return flow**                                                  | Removed                                                                                                                                                  | The `(T) -> T` signature is itself documentation                                                                                              | 2026-05-11 |
| **Empty-state reuse**                                                      | Removed as a feature                                                                                                                                     | Reassigning after a Move is natural behavior                                                                                                  | 2026-05-11 |
| **Inverse functions/partial consumption/field-level tri-layer mutability** | Removed                                                                                                                                                  | Overengineering                                                                                                                               | 2026-05-11 |
| **Lambdas do not implicitly capture**                                      | Lambdas use only explicit parameters and do not implicitly capture outer variables; context is fixed at the creation point through currying (SPEC §12.3) | The scope where a closure was defined may no longer be alive; fixed values are safe at the creation point, where the call-site scope is alive | 2026-06-16 |

### Version History

| Version | Major changes                                                                                                                                                                                              | Date           |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| v1      | Initial draft: based on Rust's ownership model                                                                                                                                                             | 2025-01-08     |
| **v8**  | **Removed overengineering (inverse functions/partial consumption/field-level tri-layer mutability/consumption analysis/ownership return flow/empty-state reuse) and added bare-bones borrowing &T/&mut T** | **2026-05-11** |
| **v9**  | **The borrow-token system replaced bare-bones borrowing and unified the type system; token conflict detection was corrected to Hoare propositions, as specified in RFC-009a**                              | **2026-06-13** |

### Pending Issues

| Issue                     | Description                                     | Status                     |
| ------------------------- | ----------------------------------------------- | -------------------------- |
| Drop syntax               | Whether an explicit `drop()` function is needed | Under discussion           |
| Escape analysis algorithm | Implementation of ref's cross-task detection    | Under discussion           |
| Token conflict detection  | Hoare logical propositions; see below           | ✅ Resolved (see RFC-009a) |

### Token Conflict Detection: Hoare Logical Propositions

The complete proposal for token conflict detection is specified in
[RFC-009a: Token Lifetime Analysis—Based on the Hoare Proof Pipeline](009a-borrow-proof-pipeline.md).
Its key points are:

**Token liveness is a Hoare logical proposition.**
`{冲突的 ReadToken 全部死亡} write(data) {WriteToken 安全获取}`—it shares RFC-027's proof pipeline
with type checking and user predicate verification. The compiler automatically generates borrow
propositions (`borrow_conflict`, `use_after_move`, `use_after_drop`, `mut_violation`), and the
pipeline returns Proved / Disproved / Unproven.

**Borrow checking has not disappeared—it has been reduced in dimensionality.** `BorrowChecker`
becomes `BorrowPredicateEmitter`, which generates propositions rather than performing checks. This
is exactly parallel to the concept of a "type checker": the type checker generates type-equality
propositions, the borrow proposition emitter generates borrow propositions, and the same pipeline
validates them.

**A brand ID (`#42`) is `'a`.** The information is exactly the same; only the encoding differs. `'a`
is visible in type signatures, while `#42` is internal to the compiler. No new analysis has been
invented—lifetimes have been lowered from the type layer to the proof layer.

**Algorithm summary** (see RFC-009a for details):

- Brand-tree prefix matching → determine conflicting tokens (O(depth), depth ≤ 3)
- Reverse BFS → start from the consumer, use break to cut back edges, with structural analysis
  covering 95%+ of scenarios (fast path)
- SMT logical cutting → invoked only for while + path conditions (slow path, extremely rare)

---

## References

### YaoXiang Official Documentation

- [Language Specification](../../reference/language-spec/index.md)
- [Design Manifesto](../../explanation/manifesto.md)
- [RFC-009a: Token Lifetime Analysis—Based on the Hoare Proof Pipeline](009a-borrow-proof-pipeline.md)
- [RFC-010 Unified Type Syntax](010-unified-type-syntax.md)
- [tutorial/ Tutorial](../../tutorial/index.md)

### External References

- [Rust Ownership Model](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang Message Passing](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## Lifecycle and Disposition

| Status        | Location                | Description                                    |
| ------------- | ----------------------- | ---------------------------------------------- |
| **Draft**     | `docs/design/rfc/`      | Author's draft, awaiting submission for review |
| **In review** | `docs/design/rfc/`      | Open for community discussion and feedback     |
| **Accepted**  | `docs/design/accepted/` | Becomes an official design document            |
| **Rejected**  | `docs/design/rfc/`      | Retained in the RFC directory                  |
