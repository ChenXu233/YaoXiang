---
title: "RFC 022: Hoare Logic Static Verification Support (Specification Comments & Specification Types)"
status: "Deprecated (Superseded by RFC-027)"
author: "晨煦"
created: "2026-03-16"
updated: "2026-06-07 (Deprecated: Superseded by compile-time evaluation type system)"
---

> **⚠️ Deprecated**
>
> This RFC has been superseded by **[RFC-027: Compile-time Evaluation Types and Unified Static Verification](../review/027-compile-time-evaluation-types.md)**.
>
> **Reason for Deprecation**: RFC 022 designs specifications as external syntax in `//!` comment form, which contradicts the fundamental principle of Curry-Howard isomorphism — "No `//! ` comments. No separate specification language. Everything is within the type system." The new design treats compile-time evaluation types as first-class citizens, replacing comment-based specifications through a unified compile-time Bool evaluation pipeline. The Debug/Release split verification mode is also replaced by a unified True/False/Unknown three-level return value model.
>
> This document is retained for historical reference only.

---

# RFC 022: Hoare Logic Static Verification Support (Specification Comments & Specification Types) [Deprecated]

> **References**:
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)

## Summary

This document proposes introducing a **Hoare logic static verification mechanism** for the YaoXiang language, allowing developers to write preconditions, postconditions, and loop invariants in comments using `//!` or `/*! ... !*/` syntax. Static verification is mandatory during Debug Build, and Release Build can only proceed after verification passes; Release Build ignores specification comments (zero overhead) and clears verification cache. Specifications themselves are treated as part of the type system, forming "specification types" (such as `Requires(P)`, `Ensures(P)`), and can be extended by users. This design aims to maintain language simplicity while providing high reliability guarantees for critical code, and integrates perfectly with YaoXiang's unified type model.

## Motivation

### Why is this feature/change needed?

YaoXiang already guarantees memory safety and thread safety through the ownership model (RFC-009) and concurrency model (RFC-001), but logical correctness still relies on testing. For systems programming, safety-critical domains (such as aerospace, finance, operating system kernels), logical errors can lead to catastrophic consequences. Existing solutions (such as Rust's borrow checking) cannot capture such errors. Hoare logic provides a mathematical proof method, but traditional formal verification tools often require a separate specification language and have a steep learning curve.

### Current Problems

- Logical correctness can only be verified through testing, with no compile-time guarantees
- Critical system code lacks formal verification means
- Existing formal verification tools have steep learning curves and are disconnected from mainstream programming languages

## Proposal

### Core Design

Our goal is to design a **lightweight, language-integrated** static verification solution:

- **Debug Build verification mandatory**: Developers write specifications in modules requiring high reliability; Debug Build enforces verification before Release Build can proceed
- **Elegant syntax**: Uses `//!` comments, introduces no new keywords, and can be syntax-highlighted by editors
- **Integrated with type system**: Specifications become part of types, can participate in type checking, support user-defined specification types
- **Provable and testable**: Can be statically proven or degraded to runtime assertions, facilitating gradual adoption

### 1. Specification Comment Syntax

At the beginning of a function body or loop body, use `//!` (single line) or `/*! ... !*/` (multi-line) to write specifications.

#### 1.1 Unified Specification Syntax

Specifications adopt YaoXiang's unified `name: Type = expression` syntax model, fully integrated with the type system:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0
    //! ensures: GreaterOrEqual(result, arr[0..n])
    //! ensures: ExistsMax(result, arr[0..n])
    // Implementation...
}
```

- A specification is essentially a **type declaration**, with the right side being a boolean expression
- The left side is a specification type instance (can carry type parameters)
- The special variable `result` can be used to denote the return value

#### 1.2 Loop Specifications

```yaoxiang
while i < n {
    /*! invariant: Bounds[i, n] = 0 <= i <= n
                  && SumInvariant[s, arr[0..i]] !*/
    s = s + arr[i]
    i = i + 1
}
```

#### 1.3 Specification Expressions

The boolean expressions on the right side of specifications use YaoXiang expression syntax, supporting:
- Arithmetic, comparison, and logical operations
- Quantifiers: `forall i in 0..n: P(i)`, `exists i in 0..n: P(i)` — language-built-in logical constructs
- Function calls (must be pure functions)

### 2. Specification Type System

Specification types are essentially ordinary types in YaoXiang, fully consistent with the unified syntax model.

#### 2.1 Built-in Specification Types

The compiler includes the following commonly used specification types (can be used directly in specifications):

```yaoxiang
// Built-in specification type definitions
NonEmpty: (T: Type) -> Type = { len: T; len > 0 }
Positive: Type = { x: Int; x > 0 }
GreaterOrEqual: (T: Type) -> Type = { result: T, arr: Array(T); result >= arr[0] && forall i in 1..arr.len: result >= arr[i] }
Bounds: (T: Type) -> Type = { i: T, n: T; 0 <= i && i <= n }
SumInvariant: (T: Type) -> Type = { s: T, arr: Array(T); s == sum(arr[0..i]) }

// Quantifier constructs (language-built-in, not functions)
forall: (start: Int, end: Int, pred: (Int) -> Bool) -> Bool
exists: (start: Int, end: Int, pred: (Int) -> Bool) -> Bool
```

#### 2.2 User-Defined Specification Types

Identical to ordinary type definitions, users can define custom specification types:

```yaoxiang
// Define positive integer specification
Positive: Type = { x: Int; x > 0 }

// Define sorted array specification
Sorted: (T: Ord) -> Type = {
    arr: Array(T);
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

// Define maximum value specification
ExistsMax: (T: Ord) -> Type = {
    result: T, arr: Array(T);
    exists i in 0..arr.len: result == arr[i]
    && forall j in 0..arr.len: result >= arr[j]
}
```

Using custom specifications:

```yaoxiang
sqrt: (x: Positive) -> Float = {
    //! ensures: SquareRootResult(result, x) = result * result <= x && (result+1)*(result+1) > x
    // Implementation...
}

binary_search: (T: Ord) -> ((arr: Sorted(Array(T)), key: T) -> Option(Index)) = {
    //! ensures: SearchResult(result, arr, key)
    // Implementation...
}
```

Specification types, like other types, support generic parameters, type constraints, and can participate in type inference.

### 3. Build Modes

| Mode | Behavior | Options |
|------|----------|---------|
| **Debug Build** | Parse specifications, generate verification conditions, invoke SMT solver for proof; Release Build can only proceed after verification passes | `yaoxiangc --debug source.yx` |
| **Release Build** | Ignore all `//!` comments, generate no code; clear all verification caches; enable aggressive optimizations | `yaoxiangc --release source.yx` |
| **Runtime Checks** | Convert specifications to runtime assertions, panic on violation | `yaoxiangc --enable-runtime-checks source.yx` |

In verification mode, if proof fails, the compiler reports an error and provides possible counterexamples (such as input values).

### 4. Verification Mechanism

The compiler converts specifications into Verification Conditions (VCs) and sends them to an integrated SMT solver (such as Z3). The verification process roughly follows these steps:

1. Collect `requires` and `ensures` for functions, `invariant` for loops
2. Generate loop invariant proof obligations for each loop: holds before entering loop, maintained after each iteration, implies postcondition after loop exit
3. Transform function body into logical formulas, combine with specifications, form verification conditions
4. Invoke SMT solver to check satisfiability

If the solver returns `unsat` (unsatisfiable), the specification holds; otherwise, report a counterexample.

### 5. Integration with Testing

Runtime checking mode converts specifications into assertions for testing. Combined with specification coverage tools, one can evaluate the degree to which tests cover specifications. Future consideration may be given to specification mining tools that automatically infer candidate specifications from tests.

### 6. Editor Support

`//!` and `/*! ... !*/` can be recognized by editors as special comments, given different colors (such as purple), distinguishing them from ordinary comments. The language server can provide hover tooltips for specifications, completion, and verification error reporting.

## Detailed Design

### Syntax Changes

| Before | After |
|--------|-------|
| No specification comment syntax | Allow `//!` and `/*! ... !*/` specification comments |

### 7.1 Syntax Extension

On top of existing syntax (RFC-010), zero or more `//!` or `/*! ... !*/` comments are allowed at the beginning of function bodies and loop bodies. Specification syntax is consistent with unified type syntax:

```
spec_comment     ::= ('//!' spec_line) | ('/*!' spec_block '!*/')
spec_line        ::= spec_name ':' type_expr '=' expr
spec_name        ::= 'requires' | 'ensures' | 'invariant'
spec_block       ::= (spec_name ':' type_expr '=' expr ';')*
```

- A specification is essentially a type declaration: `spec_name: specification_type = boolean_expression`
- `type_expr` is a specification type expression, can carry type parameters
- `expr` uses YaoXiang expression syntax, supports quantifiers

### 7.2 Type Checking

In verification mode, the compiler converts specification comments into corresponding specification type instances and records them in the metadata of functions or loops.

### 7.3 Verification Condition Generation

Using weakest precondition or strongest postcondition calculus, combined with loop invariants, generate first-order logic formulas. Generated VCs use SMT-LIB format and invoke external solvers.

### 7.4 Error Reporting

If proof fails, the solver may provide a model (counterexample). The compiler should convert these counterexamples into readable form, such as concrete input values, helping users debug.

### 7.5 Runtime Checking

In `--enable-runtime-checks` mode, the compiler converts specifications into `assert` statements:
- `requires`: Insert `assert(cond)` at function entry
- `ensures`: Insert `assert(cond)` before all return points in the function, with `result` replaced by the actual return value
- `invariant`: Insert `assert(cond)` at the beginning of the loop body

### 7.6 Integration with Existing Design

- **Ownership model**: Expressions in specifications obey ownership rules, can only read (pure functions), avoid side effects
- **Generic system**: Specification types support generic parameters (such as `Requires(P)`), can combine with generic functions/types
- **Dependent types**: Value-dependent types in specifications (such as array length `n`) are naturally usable

### Type System Impact

- Specification types are ordinary types in YaoXiang, consistent with the unified syntax model
- Compiler includes commonly used specification types (`Positive`, `NonEmpty`, `GreaterOrEqual`, etc.)
- Users can define custom specification types through ordinary type definitions
- Specification types can carry generic parameters and support type constraints

### Runtime Behavior

- **Debug Build**: Invoke SMT solver for static verification, increased compilation time; cache verification results after successful verification
- **Release Build**: Specification comments are ignored, zero runtime overhead; clear all verification caches; enable aggressive optimizations such as span cache clearing
- **Runtime checking mode**: Generate assert statements, detect violations at runtime

### Compiler Changes

- Parser: Recognize specification comment syntax
- Semantic analysis: Collect specifications, convert to specification types
- Verification backend: Generate verification conditions, invoke SMT solver
- Code generation: Support runtime checking mode

### Backward Compatibility

- ✅ Fully backward compatible
- Specification comments are ignored in normal compilation, not affecting existing code
- Specifications are ignored in Release Build, no extra overhead

## Trade-offs

### Advantages

- **Debug Build verification**: Mandatory verification during Debug Build ensures logical correctness
- **Elegant syntax**: Pure comments, no new keywords, editor-friendly
- **Integrated with type system**: Specifications are types, extensible
- **Gradual adoption**: Can transition from runtime checking to static verification progressively
- **Improved reliability**: Can catch logical errors that testing struggles to find

### Disadvantages

- **Compilation time**: Verification mode may significantly increase compilation time
- **Learning curve**: Need to learn how to write effective specifications and quantifiers
- **SMT solver limitations**: Some complex properties may not be automatically provable

## Alternative Solutions

| Solution | Advantages | Disadvantages |
|----------|------------|---------------|
| New keywords (such as `requires`) | Intuitive syntax | Introduces new keywords, breaks simplicity |
| Separate specification files (such as CVL) | Separates specifications from code | Increases file count, hard to synchronize |
| Runtime assertions only | Simple implementation | Cannot statically guarantee |
| **This proposal (comments + specification types)** | Balances simplicity and functionality | Requires editor support |

## Implementation Strategy

### Phase Division

| Phase | Content |
|-------|---------|
| **Phase 1: Basic Support** | Extend parser to recognize `//!` and `/*! ... !*/` comments, attach them to AST nodes; in verification mode, collect specifications, generate simple verification conditions (arithmetic comparisons only); integrate Z3 solver |
| **Phase 2: Quantifier Support** | Support quantifier expressions, translate to SMT-LIB `forall`/`exists`; provide IDE highlighting and hover tooltips for specifications |
| **Phase 3: Optimization & Toolchain** | Incremental verification, cache verified modules; specification coverage reports; specification mining tools (generate candidate specifications from tests) |

### Dependencies

- RFC-009: Ownership Model - Specification expressions require pure function semantics
- RFC-010: Unified Type Syntax - Specification type system is based on the type system
- RFC-011: Generic Type System Design - Specification types support generic parameters

### Risks

1. **SMT solver integration complexity**: Integration of Z3 and other solvers may encounter technical challenges
   - Mitigation: Use mature Rust Z3 bindings, progressively expand supported expression types

2. **Difficult debugging of verification failures**: When SMT solver cannot prove specifications, users may find it hard to understand why
   - Mitigation: Provide clear error messages and counterexample explanations

3. **Performance overhead**: Verification mode may significantly increase compilation time
   - Mitigation: Implement incremental verification and caching mechanisms

## Open Questions

- [ ] **Quantifier support scope**: Should nested quantifiers be supported? Higher-order quantifiers?
- [ ] **Loop invariant inference**: Should functionality to automatically infer simple invariants be provided?
- [ ] **Counterexample format for proof failures**: What is the most effective way to present counterexamples?
- [ ] **Integration with other verification tools**: Should integration with proof assistants such as Coq or Lean be considered?

## References

- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](./011-generic-type-system.md)
- [RFC-009: Ownership Model](./009-ownership-model.md)
- [JML Reference Manual](https://www.openjml.org/)
- [The SPARK Toolset](https://www.adacore.com/about-spark)
- [Z3 SMT Solver](https://github.com/Z3Prover/z3)

---

## Lifecycle and Disposition

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← Community discussion
│  Review     │
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│ accepted/  │    │    rfc/     │
│ (Formal     │    │ (Retained   │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```

### Status Description

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission for review |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes a formal design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Retained in RFC directory, status updated |

### Post-Acceptance Actions

1. Move RFC to `docs/design/accepted/` directory
2. Update filename to descriptive name (such as `hoare-logic-static-verification.md`)
3. Update status to "Formal"
4. Update status to "Accepted", add acceptance date

### Post-Rejection Actions

1. Retain in `docs/design/rfc/` directory
2. Add rejection reason and date at the top of the file
3. Update status to "Rejected"