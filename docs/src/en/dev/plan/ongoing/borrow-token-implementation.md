---
title: Borrow Token System Implementation Roadmap
status: ongoing
created: 2026-05-29
---

# Borrow Token System Implementation Roadmap

## Goal

Fully implement RFC-009 v9's borrow token system, replacing the old bare-bones borrowing.

## Dependency Chain

```
RFC-009 v9 (Borrow Token Design) ← Completed
    │
    ├── 1. Type Property System [Design Complete → type-property-system-dup.md]
    │      ├── Dup trait definition and implementation
    │      ├── trait solver recursive struct checking
    │      └── auto-derive recursive field checking
    │
    ├── 2. Closure Capture Model [Design Complete → closure-capture-model.md]
    │      ├── Variable capture analysis
    │      ├── Escape analysis
    │      └── MakeClosure env population
    │
    └── 3. Borrow Token Implementation [Pending Phases 1 & 2]
           ├── MonoType::Ref { mutable, inner }
           ├── borrow checker pass (middle/passes/lifetime/)
           ├── Token conflict detection (flow-sensitive liveness analysis)
           └── ZST optimization (tokens disappear after compilation)
```

## Phases

### Phase 1: Type Property System

**Status**: Design complete → [type-property-system-dup.md](type-property-system-dup.md)

**Scope**:
- Dup trait registered as a builtin marker trait
- Primitive types automatically marked as Dup
- struct/enum/tuple auto-derivation: all fields Dup → type Dup
- trait solver supports recursive struct/enum/tuple checking
- auto-derive supports generic container fields (`List(Int)`, etc.)
- Remove Send/Sync (not user-visible, fully automatic by compiler)

**Related Files**:
- `src/frontend/core/types/base/mono.rs`
- `src/frontend/core/typecheck/traits/std_traits.rs`
- `src/frontend/core/typecheck/traits/auto_derive.rs`
- `src/frontend/core/typecheck/traits/solver.rs`

### Phase 2: Closure Capture Model

**Status**: Pending design

**Scope**:
- Analyze external variables referenced by lambda during type checking
- Determine capture mode for each variable (borrow token vs Move)
- Populate MakeClosure env during IR generation
- Support borrow token propagation in closures

### Phase 3: Borrow Token Implementation

**Status**: Pending Phases 1 & 2

**Scope**:
- AST: `Type::Ref`, `Expr::Borrow`
- Lexer: `&` and `&mut` tokens
- MonoType: `Ref { mutable, inner }`
- IR: Borrow instructions (as needed)
- Passes: `BorrowChecker` (flow-sensitive liveness analysis)
- ZST optimization: tokens eliminated after compilation

## References

- [RFC-009 Ownership Model v9](../../design/rfc/accepted/009-ownership-model.md)
- [RFC-010 Unified Type Syntax](../../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011 Generic Type System Design](../../design/rfc/accepted/011-generic-type-system.md)