---
title: 'RFC 022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)'
status: 'Deprecated (superseded by RFC-027)'
author: 'Chen Xu'
created: '2026-03-16'
updated: '2026-06-07 (deprecated: superseded by compile-time evaluation type system)'
---

> **⚠️ Deprecated (DEPRECATED)**
>
> This RFC has been superseded by
> **[RFC-027: Compile-Time Evaluation Types and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)**.
>
> **Deprecation Reason**: RFC-022 designs specifications as external syntax in `//!`
> comment form, which contradicts the fundamental principle of Curry-Howard isomorphism — "No `//!`
> comments. No separate specification language. Everything is within the type system." The new design treats compile-time evaluation types as first-class citizens, replacing comment-style specifications with a unified compile-time Bool evaluation pipeline. The Debug/Release split verification mode is also replaced by a unified True/False/Unknown three-level return value model.
>
> This document is retained solely for historical reference.

---

# RFC 022: Hoare Logic Static Verification Support (Specification Comments and Specification Types) [Deprecated]

> **References**:
>
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)

## Summary

This document proposes introducing a **Hoare Logic static verification mechanism** for the YaoXiang language, allowing developers to write preconditions, postconditions, and loop invariants in comments using `//!` or `/*! ... !*/` syntax. During Debug Build, static verification is mandatory and must pass before Release Build can proceed; during Release Build, specification comments are ignored (zero overhead) and verification cache is cleared. Specifications themselves are treated as part of the type system, forming "specification types" (such as `Requires(P)`, `Ensures(P)`), and can be extended by users. This design aims to maintain language simplicity while providing high reliability guarantees for critical code, and integrates perfectly with YaoXiang's unified type model.

## Motivation

### Why is this feature/change needed?

YaoXiang already guarantees memory safety and thread safety through the ownership model (RFC-009) and concurrency model (RFC-001), but logical correctness still relies on testing. In systems programming, safety-critical domains (such as aerospace, finance, OS kernels), logical errors can lead to catastrophic consequences. Existing solutions (such as Rust's borrow checker) cannot catch these kinds of errors. Hoare Logic provides a mathematical proof method, but traditional formal verification tools often require a separate specification language and have a steep learning curve.

### Current Problems

- Logical correctness can only be verified through testing, cannot be guaranteed at compile time
- Critical system code lacks formal verification means
- Existing formal verification tools have steep learning curves and are disconnected from mainstream programming languages

## Proposal

### Core Design

Our goal is to design a **lightweight, language-integrated** static verification scheme:

- **Debug Build verification mandatory**: Developers write specifications in modules requiring high reliability; Debug Build must pass verification before Release Build
- **Elegant syntax**: Uses `//!` comments, introduces no new keywords, and can be syntax-highlighted by editors
- **Integrated with type system**: Specifications become part of types, participate in type checking, support user-defined specification types
- **Provable and testable**: Can be statically proven, or degraded to runtime assertions, facilitating gradual adoption

### 1. Specification Comment Syntax

At the beginning of a function body or loop body, use `//!` (single line) or `/*! ... !*/` (multi-line) to write specifications.

#### 1.1 Unified Specification Syntax

Specifications adopt YaoXiang's unified `name: Type = expression` syntax model, fully integrated with the type system:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0
    //! ensures: GreaterOrEqual(result, arr[0..n])
    //! ensures: ExistsMax(result, arr[0..n])
    // implementation...
}
```

- A specification is essentially a **type declaration**, with a Boolean expression on the right side
- The left side is a specification type instance (can carry type parameters)
- The special variable `result` can be used to represent the return value

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

The Boolean expression on the right side of a specification uses YaoXiang expression syntax, supporting:

- Arithmetic operations, comparison operations, logical operations
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

// Quantifier constructs (language built-in, not functions)
forall: (start: Int, end: Int, pred: (Int) -> Bool) -> Bool
exists: (start: Int, end: Int, pred: (Int) -> Bool) -> Bool
```

#### 2.2 User-Defined Specification Types

Completely consistent with ordinary type definitions, users can define their own specification types:

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
    // implementation...
}

binary_search: (T: Ord) -> ((arr: Sorted(Array(T)), key: T) -> Option(Index)) = {
    //! ensures: SearchResult(result, arr, key)
    // implementation...
}
```

Specification types, like other types, support generic parameters, type constraints, and participate in type inference.

### 3. Build Modes

| Mode                | Behavior                                                                                             | Option                                        |
| ------------------- | ---------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| **Debug Build**     | Parse specifications, generate verification conditions, call SMT solver for proof; must pass before Release Build | `yaoxiangc --debug source.yx`                 |
| **Release Build**   | Ignore all `//!` comments, generate no code; clear all verification cache; enable aggressive optimizations | `yaoxiangc --release source.yx`               |
| **Runtime Checks**  | Convert specifications to runtime assertions, panic on violation                                     | `yaoxiangc --enable-runtime-checks source.yx` |

In verification mode, if proof fails, the compiler will report the error and provide possible counterexamples (such as input values).

### 4. Verification Mechanism

The compiler converts specifications to Verification Conditions (VCs) and sends them to an integrated SMT solver (such as Z3). The verification process roughly proceeds as follows:

1. Collect `requires` and `ensures` for functions, `invariant` for loops
2. Generate loop invariant proof obligations for each loop: holds before entering loop, preserved after each iteration, implies postcondition after loop exit
3. Transform function body into logical formulas, combine with specifications, form verification conditions
4. Call SMT solver to check satisfiability

If the solver returns `unsat` (unsatisfiable), the specification holds; otherwise, report a counterexample.

### 5. Integration with Testing

The runtime check mode can convert specifications to assertions for testing. Combined with specification coverage tools, one can evaluate the degree to which tests cover specifications. In the future, specification mining tools can be considered to automatically infer candidate specifications from tests.

### 6. Editor Support

`//!` and `/*! ... !*/` can be recognized by editors as special comments, given different coloring (such as purple), distinguishing them from ordinary comments. Language servers can provide hover tooltips, completions, and verification error reporting for specifications.

## Detailed Design

### Syntax Changes

| Before               | After                              |
| -------------------- | ---------------------------------- |
| No specification comment syntax | Allow `//!` and `/*! ... !*/` specification comments |

### 7.1 Syntax Extension

On top of the existing syntax (RFC-010), zero or more `//!` or `/*! ... !*/` comments are allowed at the beginning of function bodies and loop bodies. The specification syntax is consistent with the unified type syntax:

```
spec_comment     ::= ('//!' spec_line) | ('/*!' spec_block '!*/')
spec_line        ::= spec_name ':' type_expr '=' expr
spec_name        ::= 'requires' | 'ensures' | 'invariant'
spec_block       ::= (spec_name ':' type_expr '=' expr ';')*
```

- A specification is essentially a type declaration: `spec_name: specification_type = boolean_expression`
- `type_expr` is a specification type expression, can carry type parameters
- `expr` uses YaoXiang expression syntax, supporting quantifiers

### 7.2 Type Checking

In verification mode, the compiler converts specification comments to corresponding specification type instances and records them in the metadata of functions or loops.

### 7.3 Verification Condition Generation

Using weakest precondition or strongest postcondition calculus, combined with loop invariants, generate first-order logic formulas. Generated VCs use SMT-LIB format, calling external solvers.

### 7.4 Error Reporting

If proof fails, the solver may provide a model (counterexample). The compiler should convert these counterexamples into readable form, such as concrete input values, to help users debug.

### 7.5 Runtime Checks

In `--enable-runtime-checks` mode, the compiler converts specifications to `assert` statements:

- `requires`: Insert `assert(cond)` at function entry
- `ensures`: Insert `assert(cond)` before all return points of the function, with `result` substituted by the actual return value
- `invariant`: Insert `assert(cond)` at the beginning of the loop body

### 7.6 Integration with Existing Design

- **Ownership model**: Expressions in specifications follow ownership rules, can only read not write (pure functions), avoiding side effects
- **Generic system**: Specification types support generic parameters (such as `Requires(P)`), can combine with generic functions/types
- **Dependent types**: Value-dependent types (such as array length `n`) in specifications are naturally usable

### Type System Impact

- Specification types are ordinary types in YaoXiang, consistent with the unified syntax model
- The compiler includes commonly used specification types built-in (`Positive`, `NonEmpty`, `GreaterOrEqual`, etc.)
- Users can define custom specification types through ordinary type definitions
- Specification types can carry generic parameters, support type constraints

### Runtime Behavior

- **Debug Build**: Call SMT solver for static verification, compile time increases; after successful verification, cache verification results
- **Release Build**: Specification comments are ignored, zero runtime overhead; clear all verification cache; enable aggressive optimizations such as Span cache clearing
- **Runtime check mode**: Generate assert statements, runtime detection of violations

### Compiler Changes

- Parser: Recognize specification comment syntax
- Semantic analysis: Collect specifications, convert to specification types
- Verification backend: Generate verification conditions, call SMT solver
- Code generation: Support runtime check mode

### Backward Compatibility

- ✅ Fully backward compatible
- Specification comments are ignored in normal compilation, not affecting existing code
- Specifications are ignored in Release Build, no extra overhead

## Trade-offs

### Advantages

- **Debug Build verification**: Mandatory verification in Debug Build ensures logical correctness
- **Elegant syntax**: Pure comments, no new keywords, editor-friendly
- **Integrated with type system**: Specifications are types, extensible
- **Gradual adoption**: Can transition from runtime checks to static verification incrementally
- **Improved reliability**: Can catch logical errors that are difficult to discover through testing

### Disadvantages

- **Compile time**: Verification mode may significantly increase compile time
- **Learning curve**: Need to learn how to write effective specifications and quantifiers
- **SMT solver limitations**: Some complex properties may not be provable automatically

## Alternative Approaches

| Approach                        | Advantages           | Disadvantages                   |
| ------------------------------- | -------------------- | ------------------------------- |
| New keywords (such as `requires`) | Intuitive syntax     | Introduces new keywords, breaks simplicity |
| Separate specification files (such as CVL) | Separates specifications from code | Increases file count, hard to sync   |
| Runtime assertions only         | Simple to implement  | Cannot guarantee statically            |
| **This approach (comments + specification types)** | Balances simplicity and functionality | Requires editor support        |

## Implementation Strategy

### Phase Breakdown

| Phase                      | Content                                                                                                                                                    |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1: Basic support**     | Extend parser to recognize `//!` and `/*! ... !*/` comments, attach to AST nodes; in verification mode, collect specifications, generate simple verification conditions (arithmetic comparisons only); integrate Z3 solver |
| **Phase 2: Quantifier support** | Support quantifier expressions, translate to SMT-LIB `forall`/`exists`; provide IDE highlighting and hover tooltips for specifications                      |
| **Phase 3: Optimization and toolchain** | Incremental verification, cache verified modules; specification coverage reports; specification mining tools (generate candidate specifications from tests) |

### Dependencies

- RFC-009: Ownership Model - Specification expressions require pure function semantics
- RFC-010: Unified Type Syntax - Specification type system is based on the type system
- RFC-011: Generic Type System Design - Specification types support generic parameters

### Risks

1. **SMT solver integration complexity**: Integration of Z3 and other solvers may encounter technical challenges
   - Mitigation: Use mature Rust Z3 bindings, gradually expand supported expression types

2. **Difficult debugging of verification failures**: When SMT solver cannot prove a specification, users may find it hard to understand the reason
   - Mitigation: Provide clear error messages and counterexample explanations

3. **Performance overhead**: Verification mode may significantly increase compile time
   - Mitigation: Implement incremental verification and caching mechanisms

## Open Questions

- [ ] **Quantifier support scope**: Support nested quantifiers? Support higher-order quantifiers?
- [ ] **Loop invariant inference**: Provide automatic inference of simple invariants?
- [ ] **Counterexample format for proof failures**: What is the most effective way to present counterexamples?
- [ ] **Integration with other verification tools**: Consider integration with proof assistants such as Coq, Lean?

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
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
│ Under Review │  ← Community discussion
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
│ (official) │    │ (preserved) │
└─────────────┘    └─────────────┘
```

### Status Description

| Status       | Location                   | Description                              |
| ------------ | -------------------------- | ---------------------------------------- |
| **Draft**    | `docs/design/rfc/`         | Author draft, awaiting review submission  |
| **Under Review** | `docs/design/rfc/`     | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/`    | Becomes official design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/`         | Preserved in RFC directory, status updated |

### Actions After Acceptance

1. Move RFC to `docs/design/accepted/` directory
2. Update filename to descriptive name (such as `hoare-logic-static-verification.md`)
3. Update status to "official"
4. Update status to "accepted", add acceptance date

### Actions After Rejection

1. Preserve in `docs/design/rfc/` directory
2. Add rejection reason and date at the top of the file
3. Update status to "rejected"