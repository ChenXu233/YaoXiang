---
title: 'RFC-023: Closure Capture Model'
status: 'Deprecated'
author: 'Chen Xu'
created: '2026-05-29'
updated: '2026-06-16'
---

> **Deprecation Reason**: 2026-06-16 Language Design Decision — Lambda/function values do not
> implicitly capture outer variables, using explicit parameter passing instead. `spawn { }` executes
> in the same frame and does not involve closure capture. The capture analysis system in this RFC
> has been completely removed (~850 lines of code). See
> [RFC-009 Design Decision Record](../accepted/009-ownership-model.md#design-decision-record) for
> details.

# RFC-023: Closure Capture Model

> **References**:
>
> - [RFC-007: Function Syntax Unification](./accepted/007-function-syntax-unification.md)
> - [RFC-009: Ownership Model v9](./accepted/009-ownership-model.md)
> - [RFC-011: Generic Type System Design](./accepted/011-generic-type-system.md) — Section 2.4:
>   Dup/Clone Builtin marker trait

## Summary

This RFC defines the **closure capture model** for YaoXiang language. The compiler automatically
analyzes external variables referenced in the closure body, and based on the variable type
(Dup/non-Dup) and whether the closure escapes, automatically selects the capture mode — Dup types
are directly copied, non-Dup non-escaping uses borrowing, non-Dup escaping uses Move. Zero user
annotations, sharing the same rules as automatic borrowing selection for function calls.

## Motivation

### Why is this needed?

Currently closure capture is **empty implementation** — the `env` field of `MakeClosure` instruction
is always empty, and lambdas cannot reference any external variables. The borrowing token system
requires closures to capture `&T` tokens (zero-cost copying), which is a core use case.

### Current Problems

```yaoxiang
# This code currently cannot compile — lambda cannot reference threshold
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold cannot be captured
}
```

## Proposal

### Core Design

Closure capture is fully automatically determined by the compiler. The rules are **identical** to
automatic borrowing selection for function calls:

```
Variable Type    Closure Escapes    Capture Mode
─────────────────────────────────────────
Dup              Any                Copy (bitwise copy or zero-cost)
Non-Dup          Non-escaping       Auto-borrow (&T or &mut T)
Non-Dup          Escaping           Move (ownership transfer)
```

**Escape Determination**:

```
spawn { || ... }           → Escaping
return || ...              → Escaping
let x = || ... ;  x  stored in field → Escaping
items.filter(|p| ...)      → Non-escaping (sync higher-order function call)
||.method()                → Non-escaping (immediate invocation)
```

Conservative principle: when uncertain, treat as escaping.

### Examples

```yaoxiang
# 1. Dup Token — Direct Copy (Zero Cost)
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → Compiler copies token into closure
    # Zero-size token, zero runtime overhead
    items.filter(|p| p.x > threshold)
}

# 2. Non-Dup + Non-escaping — Auto-borrow
process: (buf: Buffer) -> Void = {
    # buf is not Dup, filter does not escape → Auto-create &Buffer token
    transform(|b| b.read())
    # Token released after closure returns, buf becomes usable again
}

# 3. Closure Escapes — Move
spawn_worker: (data: Data) -> Void = {
    # data is not Dup, spawn → Escaping → Move
    spawn { use(data) }
}

# 4. Mixed Capture
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → Copy token
    # buf: Buffer → Not Dup, non-escaping → &mut Buffer borrow
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. Borrow Conflict Detection
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf is already borrowed by closure, conflict here
}
```

### Syntax Changes

**Zero syntax changes**. Capture mode is automatically determined by the compiler, users do not need
to annotate.

## Detailed Design

### Type System Impact

Lambda type signatures remain unchanged: `(params) -> Return`. Captured variables are not reflected
in the type signature; they are handled by the compiler during IR generation phase.

### Compiler Changes

| Component          | Changes                                             | Note       |
| ------------------ | --------------------------------------------------- | ---------- |
| `capture.rs` (new) | Capture analysis + Escape analysis + Mode selection | ~150 lines |
| `expressions.rs`   | Lambda type inference calls capture analysis        | ~10 lines  |
| `ir_gen.rs`        | MakeClosure env population; ZST skip                | ~80 lines  |
| `ir.rs`            | MakeClosure env type may need adjustment            | ~5 lines   |

**Capture Analysis Flow**:

```
1. Traverse lambda body AST
2. Collect all Expr::Var(name) references
3. Filter: only keep variables from outer closure scope
4. Classify: Read (read-only) / Write (read-write) / Move (transferred)
5. Check type attribute: whether Dup
6. Determine escape: closure usage pattern
7. Select capture mode:
   Dup → Copy
   Non-Dup + Non-escaping + Read → Borrow (&T)
   Non-Dup + Non-escaping + Write → BorrowMut (&mut T)
   Non-Dup + Escaping → Move
```

**IR Generation**:

```rust
// Current (empty)
Instruction::MakeClosure { dst, func, env: Vec::new() }

// Changed to
Instruction::MakeClosure { dst, func, env: captured_env }

// captured_env generation logic:
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // Zero-size type — generate no instructions
            // Closure body directly references outer (compile-time elimination)
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

Capture mode does not affect runtime performance:

- **Dup + ZST** (e.g., `&T` tokens) → Zero instructions, closure body directly references outer
  variable
- **Dup + Non-ZST** (e.g., Int) → One register copy
- **Borrow/BorrowMut** → Token creation (compile-time concept, zero overhead)
- **Move** → Same cost as regular Move

### Backward Compatibility

Fully compatible. Currently all lambdas cannot capture external variables; this RFC only adds
expressive power and does not break any existing code.

## Trade-offs

### Advantages

1. **Zero annotations**: Users do not need to write any capture annotations
2. **Unified with function calls**: Capture rules = Function call automatic borrowing rules
3. **Zero-cost**: Dup token capture is completely eliminated at compile time
4. **Safe**: Escape analysis prevents use-after-free

### Disadvantages

1. **Conservative escape analysis**: When uncertain, treating as escaping may unnecessarily Move
2. **Implicit**: Capture mode is not reflected in code; debugging requires looking at compiler
   output

## Alternative Solutions

| Solution                            | Why Not Chosen                                           |
| ----------------------------------- | -------------------------------------------------------- |
| Rust-style explicit `move` keyword  | Introduces new syntax, increases user cognitive burden   |
| All Move                            | Cannot express zero-cost token borrowing                 |
| All borrow                          | Closure escaping would lead to dangling references       |
| User manual capture mode annotation | Contradicts "compiler fully automatic" design philosophy |

## Implementation Strategy

### Phase Division

1. **Phase 1**: Capture analysis (only identify external variable references, don't distinguish
   capture modes)
2. **Phase 2**: Escape analysis + Mode selection
3. **Phase 3**: IR generation + ZST optimization
4. **Phase 4**: Borrow conflict detection integration

### Dependencies

- Depends on RFC-011 (Generic system, Section 2.4 Dup/Clone trait) — needs Dup trait to determine if
  variable is copyable
- Depends on RFC-009 v9 (Borrowing tokens) — Borrow/BorrowMut capture modes need token types
- After RFC-023 and this RFC implementation, borrowing token system (RFC-009 v9 implementation) can
  begin

### Risks

- Escape analysis may be too conservative, leading to unnecessary Move; can be optimized later
- Capture analysis for generic closures may need additional handling

## Design Decision Record

| Decision                      | Decision                   | Reason                                          | Date       |
| ----------------------------- | -------------------------- | ----------------------------------------------- | ---------- |
| Capture mode selection        | Fully automatic            | Unified with function call rules                | 2026-05-29 |
| Escape analysis               | Conservative principle     | When uncertain, treat as escaping, safety first | 2026-05-29 |
| ZST optimization              | Skip at IR generation      | Simpler than post-optimization pass             | 2026-05-29 |
| Capture not in type signature | Compiler internal handling | Keep lambda types clean                         | 2026-05-29 |

## References

### YaoXiang Official Documentation

- [RFC-007: Function Syntax Unification](./accepted/007-function-syntax-unification.md)
- [RFC-009: Ownership Model v9](./accepted/009-ownership-model.md)
- [RFC-011: Generic Type System Design](./accepted/011-generic-type-system.md) — Section 2.4:
  Dup/Clone Builtin marker trait

### External References

- [Rust Closure Capture Rules](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift Closure Capture Semantics](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)
