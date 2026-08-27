---
title: 'RFC-023: Closure Capture Model'
status: 'Deprecated'
author: 'Chenxu'
created: '2026-05-29'
updated: '2026-06-16'
---

> **Deprecation reason**: 2026-06-16 language design decision—Lambda/function values do not
> implicitly capture outer variables; instead, explicit parameters are used. `spawn { }` executes in
> the same frame and does not involve closure capture. The capture analysis system in this RFC has
> been completely removed (~850 lines of code). The correct approach for context dependencies =
> closures only consume parameters + currying is fixed at the creation point (SPEC §12.3 / RFC-009
> §2.3). See [RFC-009 design decision](../accepted/009-ownership-model.md#设计决策记录) for details.

# RFC-023: Closure Capture Model

> **References**:
>
> - [RFC-007: Function Syntax Unification](./accepted/007-function-syntax-unification.md)
> - [RFC-009: Ownership Model v9](./accepted/009-ownership-model.md)
> - [RFC-011: Generic System Design](./accepted/011-generic-type-system.md) — Section 2.4: Dup/Clone
>   built-in marker Trait

## Abstract

This RFC defines the **Closure Capture Model** for the YaoXiang language. The compiler automatically
analyzes the outer variables referenced by a closure body and selects the capture mode based on the
variable type (Dup/non-Dup) and whether the closure escapes—Dup types are directly copied, non-Dup
non-escaping variables are borrowed, and non-Dup escaping variables are Moved. Users need zero
annotations, and the rules are shared with the automatic borrow selection for function calls.

## Motivation

### Why is this needed?

Currently, closure capture is an **empty implementation**—the `env` field of the `MakeClosure`
instruction is always empty, and lambdas cannot reference any outer variables. The borrow token
system requires closures to be able to capture `&T` tokens (zero-cost copying), which is a core use
case.

### The current problem

```yaoxiang
# This code currently cannot compile—the lambda cannot reference threshold
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold cannot be captured
}
```

## Proposal

### Core Design

Closure capture is fully determined by the compiler. The rules are **exactly the same** as the
automatic borrow selection for function calls:

```
Variable Type    Closure Escapes?    Capture Mode
─────────────────────────────────────────
Dup              Any                 Copy (bit copy or zero-cost)
Non-Dup          No                  Auto Borrow (&T or &mut T)
Non-Dup          Yes                 Move (ownership transfer)
```

**Escape determination**:

```
spawn { || ... }           → escapes
return || ...              → escapes
let x = || ... ;  x stored as field → escapes
items.filter(|p| ...)      → does not escape (sync higher-order function call)
||.method()                → does not escape (called on the spot)
```

Conservative principle: when uncertain, treat as escaping.

### Examples

```yaoxiang
# 1. Dup token—direct copy (zero-cost)
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → compiler copies token into closure
    # Zero-size token, zero runtime overhead
    items.filter(|p| p.x > threshold)
}

# 2. Non-Dup + non-escaping—auto borrow
process: (buf: Buffer) -> Void = {
    # buf is not Dup, filter does not escape → auto-create &Buffer token
    transform(|b| b.read())
    # Token released after closure returns, buf becomes usable again
}

# 3. Closure escapes—Move
spawn_worker: (data: Data) -> Void = {
    # data is not Dup, spawn → escapes → Move
    spawn { use(data) }
}

# 4. Mixed capture
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → copy token
    # buf: Buffer → not Dup, non-escaping → &mut Buffer borrow
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. Borrow conflict detection
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf already borrowed by closure, conflict here
}
```

### Syntax Changes

**Zero syntax changes**. The capture mode is automatically determined by the compiler; users do not
need to annotate.

## Detailed Design

### Type System Impact

The type signature of a Lambda remains unchanged: `(params) -> Return`. Captured variables are not
reflected in the type signature; the compiler handles them during IR generation.

### Compiler Changes

| Component          | Change                                              | Description |
| ------------------ | --------------------------------------------------- | ----------- |
| `capture.rs` (new) | Capture analysis + escape analysis + mode selection | ~150 lines  |
| `expressions.rs`   | Lambda type inference invokes capture analysis      | ~10 lines   |
| `ir_gen.rs`        | MakeClosure env population; ZST skipping            | ~80 lines   |
| `ir.rs`            | MakeClosure env type may need adjustment            | ~5 lines    |

**Capture analysis flow**:

```
1. Traverse the lambda body AST
2. Collect all Expr::Var(name) references
3. Filter: keep only variables from outer scopes of the closure
4. Classify: Read (read-only) / Write (read-write) / Move (transferred)
5. Check type attribute: is it Dup
6. Determine escape: how the closure is used
7. Select capture mode:
   Dup → Copy
   Non-Dup + non-escaping + Read → Borrow (&T)
   Non-Dup + non-escaping + Write → BorrowMut (&mut T)
   Non-Dup + escaping → Move
```

**IR generation**:

```rust
// Current (empty)
Instruction::MakeClosure { dst, func, env: Vec::new() }

// Changed to
Instruction::MakeClosure { dst, func, env: captured_env }

// captured_env generation logic:
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // Zero-size type—do not generate any instruction
            // The closure body directly references the outer scope (compile-time eliminated)
        }
        Copy => {
            // Generate Move dst, src (shallow copy for Dup types)
        }
        Borrow => {
            // Generate Borrow dst, src (create ReadToken)
        }
        BorrowMut => {
            // Generate Borrow dst, src (create WriteToken)
        }
        Move => {
            // Generate Move dst, src (ownership transfer)
        }
    }
}
```

### Runtime Behavior

The capture mode does not affect runtime performance:

- **Dup + ZST** (e.g., `&T` token) → zero instructions; the closure body directly references the
  outer variable
- **Dup + non-ZST** (e.g., Int) → a single register copy
- **Borrow/BorrowMut** → token creation (compile-time concept, zero overhead)
- **Move** → same cost as a normal Move

### Backward Compatibility

Fully compatible. Currently all lambdas cannot capture outer variables; this RFC only adds
expressive power and does not break any existing code.

## Trade-offs

### Advantages

1. **Zero annotations**: users do not need to write any capture annotations
2. **Unified with function calls**: capture rules = function call auto-borrow rules
3. **Zero-cost**: capture of Dup tokens is fully eliminated at compile time
4. **Safe**: escape analysis prevents use-after-free

### Disadvantages

1. **Conservative escape analysis**: when uncertain, treated as escaping, which may cause
   unnecessary Moves
2. **Implicit**: the capture mode is not reflected in the code; debugging requires checking compiler
   output

## Alternatives

| Option                               | Why not chosen                                            |
| ------------------------------------ | --------------------------------------------------------- |
| Rust-style explicit `move` keyword   | Introduces new syntax, increases user cognitive load      |
| All Move                             | Cannot express zero-cost token borrowing                  |
| All Borrow                           | Escaping closures would cause dangling references         |
| User manually annotates capture mode | Violates the "fully automatic compiler" design philosophy |

## Implementation Strategy

### Phases

1. **Phase 1**: Capture analysis (only identifying outer variable references, without distinguishing
   capture modes)
2. **Phase 2**: Escape analysis + mode selection
3. **Phase 3**: IR generation + ZST optimization
4. **Phase 4**: Borrow conflict detection integration

### Dependencies

- Depends on RFC-011 (Generic System, Section 2.4 Dup/Clone trait)—needs the Dup trait to determine
  whether a variable is copyable
- Depends on RFC-009 v9 (Borrow Token)—Borrow/BorrowMut capture modes require token types
- Once RFC-023 and this RFC are implemented, the borrow token system (RFC-009 v9 implementation) can
  begin work

### Risks

- Escape analysis may be too conservative, causing unnecessary Moves; can be optimized later
- Capture analysis for generic closures may require additional handling

## Design Decision Records

| Decision                                | Choice                         | Reason                                          | Date       |
| --------------------------------------- | ------------------------------ | ----------------------------------------------- | ---------- |
| Capture mode selection                  | Fully automatic                | Unified with function call rules                | 2026-05-29 |
| Escape analysis                         | Conservative                   | When uncertain, treat as escaping; safety first | 2026-05-29 |
| ZST optimization                        | Skip during IR generation      | Simpler than later optimization passes          | 2026-05-29 |
| Capture not reflected in type signature | Handled internally by compiler | Keep lambda types simple                        | 2026-05-29 |

## References

### YaoXiang Official Documents

- [RFC-007: Function Syntax Unification](./accepted/007-function-syntax-unification.md)
- [RFC-009: Ownership Model v9](./accepted/009-ownership-model.md)
- [RFC-011: Generic System Design](./accepted/011-generic-type-system.md) — Section 2.4: Dup/Clone
  built-in marker trait

### External References

- [Rust Closure Capture Rules](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift Closure Capture Semantics](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)
