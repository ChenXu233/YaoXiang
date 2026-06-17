---
title: "RFC-023: Closure Capture Model"
status: "Deprecated"
author: "Chenxu"
created: "2026-05-29"
updated: "2026-06-16"
---

> **Deprecation Reason**: 2026-06-16 language design decision — Lambda/function values do not implicitly capture outer variables; explicit parameter passing is used instead. `spawn { }` executes within the same frame and does not involve closure capture. The capture analysis system of this RFC has been entirely removed (~850 lines of code). See [RFC-009 Design Decision](../accepted/009-ownership-model.md#design-decision-records) for details.

# RFC-023: Closure Capture Model

> **References**:
> - [RFC-007: Function Syntax Unification](./accepted/007-function-syntax-unification.md)
> - [RFC-009: Ownership Model v9](./accepted/009-ownership-model.md)
> - [RFC-011: Generics System Design](./accepted/011-generic-type-system.md) — Section 2.4: Dup/Clone built-in marker trait

## Summary

This RFC defines the **Closure Capture Model** of the YaoXiang language. The compiler automatically analyzes external variables referenced by a closure body, and based on the variable's type (Dup/non-Dup) and whether the closure escapes, automatically selects the capture mode — direct copy for Dup types, borrow for non-Dup non-escaping closures, and Move for non-Dup escaping closures. Users provide zero annotations, and the rules are shared with the automatic borrow selection for function calls.

## Motivation

### Why is this needed?

Currently, closure capture is an **empty implementation** — the `env` field of the `MakeClosure` instruction is always empty, and lambdas cannot reference any external variables. The borrow token system requires closures to be able to capture `&T` tokens (zero-cost copies), which is a core use case.

### Current Problems

```yaoxiang
# This code currently cannot compile — lambda cannot reference threshold
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold cannot be captured
}
```

## Proposal

### Core Design

Closure capture is fully automatically determined by the compiler. The rules are **completely identical** to the automatic borrow selection for function calls:

```
Variable Type    Escape Status    Capture Mode
─────────────────────────────────────────
Dup              any              Copy (bit-copy or zero-cost)
Non-Dup          not escape       Automatic borrow (&T or &mut T)
Non-Dup          escape           Move (ownership transfer)
```

**Escape Determination**:

```
spawn { || ... }           → escape
return || ...              → escape
let x = || ... ;  x stored in a field → escape
items.filter(|p| ...)      → not escape (sync higher-order function call)
||.method()                → not escape (called on the spot)
```

Conservative principle: when uncertain, treat as escape.

### Examples

```yaoxiang
# 1. Dup token — direct copy (zero-cost)
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → compiler copies the token into the closure
    # zero-sized token, zero runtime overhead
    items.filter(|p| p.x > threshold)
}

# 2. Non-Dup + not escape — automatic borrow
process: (buf: Buffer) -> Void = {
    # buf is not Dup, filter does not escape → automatically create &Buffer token
    transform(|b| b.read())
    # token is released after the closure returns, buf becomes usable again
}

# 3. Closure escape — Move
spawn_worker: (data: Data) -> Void = {
    # data is not Dup, spawn → escape → Move
    spawn { use(data) }
}

# 4. Mixed capture
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → copy token
    # buf: Buffer → not Dup, not escape → &mut Buffer borrow
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. Borrow conflict detection
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf has already been borrowed by the closure, conflict here
}
```

### Syntax Changes

**Zero syntax changes**. The capture mode is automatically determined by the compiler, and users do not need to provide annotations.

## Detailed Design

### Type System Impact

The lambda's type signature remains unchanged: `(params) -> Return`. Captured variables are not reflected in the type signature, and are handled by the compiler during the IR generation phase.

### Compiler Changes

| Component | Change | Description |
|------|------|------|
| `capture.rs` (new) | Capture analysis + escape analysis + mode selection | ~150 lines |
| `expressions.rs` | Lambda type inference invokes capture analysis | ~10 lines |
| `ir_gen.rs` | MakeClosure env population; ZST skip | ~80 lines |
| `ir.rs` | MakeClosure env type may need adjustment | ~5 lines |

**Capture Analysis Workflow**:

```
1. Traverse the lambda body AST
2. Collect all Expr::Var(name) references
3. Filter: keep only variables from outer scopes of the closure
4. Classify: Read (read-only) / Write (read-write) / Move (transferred)
5. Check type attribute: whether Dup
6. Determine escape: based on how the closure is used
7. Select capture mode:
   Dup → Copy
   Non-Dup + not escape + Read → Borrow (&T)
   Non-Dup + not escape + Write → BorrowMut (&mut T)
   Non-Dup + escape → Move
```

**IR Generation**:

```rust
// Current (empty)
Instruction::MakeClosure { dst, func, env: Vec::new() }

// Changed to
Instruction::MakeClosure { dst, func, env: captured_env }

// Logic for generating captured_env:
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // Zero-sized type — generate no instructions
            // The closure body directly references the outer scope (compile-time elimination)
        }
        Copy => {
            // Generate Move dst, src (shallow copy of Dup type)
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

- **Dup + ZST** (e.g., `&T` token) → zero instructions, the closure body directly references outer variables
- **Dup + non-ZST** (e.g., Int) → one register copy
- **Borrow/BorrowMut** → token creation (compile-time concept, zero overhead)
- **Move** → same cost as a normal Move

### Backward Compatibility

Fully compatible. Currently no lambda can capture external variables; this RFC will only add expressive power and will not break any existing code.

## Trade-offs

### Advantages

1. **Zero annotations**: users do not need to write any capture annotations
2. **Unified with function calls**: capture rules = automatic borrow rules for function calls
3. **Zero cost**: capture of Dup tokens is entirely eliminated at compile-time
4. **Safe**: escape analysis prevents use-after-free

### Disadvantages

1. **Conservative escape analysis**: when uncertain, treated as escape, which may cause unnecessary Moves
2. **Implicit**: capture mode is not reflected in the code, requires inspecting compiler output for debugging

## Alternatives

| Approach | Why not chosen |
|------|--------------|
| Rust-style explicit `move` keyword | Introduces new syntax, increases user cognitive load |
| Always Move | Cannot express zero-cost token borrowing |
| Always borrow | Closure escape would lead to dangling references |
| User manually annotates capture mode | Violates the "fully automatic compiler" design philosophy |

## Implementation Strategy

### Phases

1. **Phase 1**: Capture analysis (only identify external variable references, do not distinguish capture modes)
2. **Phase 2**: Escape analysis + mode selection
3. **Phase 3**: IR generation + ZST optimization
4. **Phase 4**: Borrow conflict detection integration

### Dependencies

- Depends on RFC-011 (Generics System, Section 2.4 Dup/Clone trait) — the Dup trait is needed to determine whether a variable is copyable
- Depends on RFC-009 v9 (Borrow Tokens) — the Borrow/BorrowMut capture modes require token types
- After RFC-023 and this RFC are implemented, the borrow token system (RFC-009 v9 implementation) can commence

### Risks

- Escape analysis may be overly conservative, causing unnecessary Moves; can be optimized later
- Capture analysis for generic closures may require additional handling

## Design Decision Records

| Decision | Determination | Reason | Date |
|------|------|------|------|
| Capture mode selection | Fully automatic | Unified with function call rules | 2026-05-29 |
| Escape analysis | Conservative principle | When uncertain, treat as escape, safety first | 2026-05-29 |
| ZST optimization | Skipped during IR generation | Simpler than subsequent optimization passes | 2026-05-29 |
| Capture not reflected in type signature | Handled internally by the compiler | Keep lambda types concise | 2026-05-29 |

## References

### YaoXiang Official Documentation

- [RFC-007: Function Syntax Unification](./accepted/007-function-syntax-unification.md)
- [RFC-009: Ownership Model v9](./accepted/009-ownership-model.md)
- [RFC-011: Generics System Design](./accepted/011-generic-type-system.md) — Section 2.4: Dup/Clone built-in marker trait

### External References

- [Rust Closure Capture Rules](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift Closure Capture Semantics](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)