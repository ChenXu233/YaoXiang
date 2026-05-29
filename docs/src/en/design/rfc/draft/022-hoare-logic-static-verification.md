---
title: RFC 022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)
---

# RFC 022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)

> **Status**: Under Review
> **Author**: Chen Xu
> **Created**: 2026-03-16
> **Last Updated**: 2026-03-16

> **References**:
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md)
> - [RFC-009: Ownership Model](./009-ownership-model.md)

## Summary

This document proposes introducing a **Hoare logic static verification mechanism** for the YaoXiang language, allowing developers to write preconditions, postconditions, and loop invariants in comments using `//!` or `/*! ... !*/` syntax. Verification is mandatory during Debug Build, and Release Build is only allowed after verification passes. During Release Build, specification comments are ignored (zero overhead) and verification caches are cleared. Specifications themselves are treated as part of the type system, forming "specification types" (such as `Requires(P)`, `Ensures(P)`), and can be extended by users. This design aims to maintain language simplicity while providing high reliability guarantees for critical code, and integrates seamlessly with YaoXiang's unified type model.

## Motivation

### Why is this feature/change needed?

YaoXiang already guarantees memory safety and thread safety through the ownership model (RFC-009) and concurrency model (RFC-001), but logical correctness still relies on testing. For systems programming, safety-critical domains (such as aerospace, finance, OS kernels), logical errors can lead to catastrophic consequences. Existing solutions (such as Rust's borrow checker) cannot capture such errors. Hoare logic provides a mathematical proof mechanism, but traditional formal verification tools often require a separate specification language and have a steep learning curve.

### Current Problems

- Logical correctness can only be verified through testing, with no compile-time guarantees
- Critical system code lacks formal verification methods
- Existing formal verification tools have steep learning curves and are disconnected from mainstream programming languages

## Proposal

### Core Design

Our goal is to design a **lightweight, language-integrated** static verification solution:

- **Debug Build verification required**: Developers write specifications in modules requiring high reliability; Debug Build mandates successful verification before Release Build
- **Elegant syntax**: Uses `//!` comments, introduces no new keywords, and can be highlighted by editors
- **Integrated with type system**: Specifications become part of types, can participate in type checking, support user-defined specification types
- **Proovable and testable**: Can be statically proven, or degraded to runtime assertions, facilitating gradual adoption

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

The boolean expression on the right side of a specification uses YaoXiang expression syntax, supporting:
- Arithmetic, comparison, and logical operations
- Quantifiers: `forall i in 0..n: P(i)`, `exists i in 0..n: P(i)` — language-built-in logical constructs
- Function calls (must be pure functions)

### 2. Specification Type System

Specification types are essentially ordinary types in YaoXiang, fully consistent with the unified syntax model.

#### 2.1 Built-in Specification Types

The compiler provides the following commonly used specification types (can be used directly in specifications):

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

Completely consistent with ordinary type definitions, users can define custom specification types:

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
| **Debug Build** | Parses specifications, generates verification conditions, invokes SMT solver for proof; Release Build only allowed after verification passes | `yaoxiangc --debug source.yx` |
| **Release Build** | Ignores all `//!` comments, generates no code; clears all verification caches; enables aggressive optimization | `yaoxiangc --release source.yx` |
| **Runtime Checks** | Converts specifications to runtime assertions, panics on violation | `yaoxiangc --enable-runtime-checks source.yx` |

In verification mode, if proof fails, the compiler will report an error and provide possible counterexamples (such as input values).

### 4. Verification Mechanism

The compiler converts specifications into Verification Conditions (VCs) and sends them to an integrated SMT solver (such as Z3). The verification process roughly follows these steps:

1. Collect `requires` and `ensures` for functions, `invariant` for loops
2. Generate loop invariant proof obligations for each loop: valid before entering loop, maintained after each iteration, implies postcondition after loop exit
3. Convert function body into logical formulas, combine with specifications, form verification conditions
4. Invoke SMT solver to check satisfiability

If the solver returns `unsat` (unsatisfiable), the specification holds; otherwise, report a counterexample.

### 5. Integration with Testing

Runtime checking mode converts specifications into assertions for testing. Combined with specification coverage tools, one can evaluate the degree to which tests cover specifications. Future work may include specification mining tools that automatically infer candidate specifications from tests.

### 6. Editor Support

`//!` and `/*! ... !*/` can be recognized by editors as special comments, given different coloring (such as purple), distinguishing them from ordinary comments. The language server can provide hover tooltips, completions, and verification error reporting for specifications.

## Detailed Design

### Syntax Changes

| Before | After |
|--------|-------|
| No specification comment syntax | Allow `//!` and `/*! ... !*/` specification comments |

### 7.1 Syntax Extension

On the basis of existing syntax (RFC-010), zero or more `//!` or `/*! ... !*/` comments are allowed at the beginning of function bodies and loop bodies. The specification syntax is consistent with the unified type syntax:

```
spec_comment     ::= ('//!' spec_line) | ('/*!' spec_block '!*/')
spec_line        ::= spec_name ':' type_expr '=' expr
spec_name        ::= 'requires' | 'ensures' | 'invariant'
spec_block       ::= (spec_name ':' type_expr '=' expr ';')*
```

- A specification is essentially a type declaration: `spec_name: spec_type = boolean_expression`
- `type_expr` is a specification type expression, can carry type parameters
- `expr` uses YaoXiang expression syntax, supports quantifiers

### 7.2 Type Checking

In verification mode, the compiler converts specification comments into corresponding specification type instances and records them in the metadata of functions or loops.

### 7.3 Verification Condition Generation

Using weakest precondition or strongest postcondition calculus, combined with loop invariants, first-order logic formulas are generated. The generated VCs use SMT-LIB format, invoking an external solver.

### 7.4 Error Reporting

If proof fails, the solver may provide a model (counterexample). The compiler should convert these counterexamples into readable forms, such as concrete input values, to help users debug.

### 7.5 Runtime Checks

In `--enable-runtime-checks` mode, the compiler converts specifications into `assert` statements:
- `requires`: Insert `assert(cond)` at function entry
- `ensures`: Insert `assert(cond)` before all return points of the function, with `result` replaced by the actual return value
- `invariant`: Insert `assert(cond)` at the beginning of the loop body

### 7.6 Integration with Existing Design

- **Ownership Model**: Expressions in specifications follow ownership rules, can only read, cannot write (pure functions), avoiding side effects
- **Generic System**: Specification types support generic parameters (such as `Requires(P)`), can be combined with generic functions/types
- **Dependent Types**: Value-dependent types (such as array length `n`) in specifications are naturally available

### Type System Impact

- Specification types are ordinary types in YaoXiang, consistent with the unified syntax model
- The compiler provides built-in commonly used specification types (`Positive`, `NonEmpty`, `GreaterOrEqual`, etc.)
- Users can define custom specification types through ordinary type definitions
- Specification types can carry generic parameters and support type constraints

### Runtime Behavior

- **Debug Build**: Invokes SMT solver for static verification, increased compilation time; caches verification results after successful verification
- **Release Build**: Specification comments are ignored, zero runtime overhead; clears all verification caches; enables aggressive optimizations such as span cache clearing
- **Runtime Check Mode**: Generates assert statements, detects violations at runtime

### Compiler Changes

- Parser: Recognizes specification comment syntax
- Semantic analysis: Collects specifications, converts to specification types
- Verification backend: Generates verification conditions, invokes SMT solver
- Code generation: Supports runtime check mode

### Backward Compatibility

- ✅ Fully backward compatible
- Specification comments are ignored in normal compilation, not affecting existing code
- Specifications are ignored during Release Build, no extra overhead

## Trade-offs

### Advantages

- **Debug Build Verification**: Mandatory verification during Debug Build, ensuring logical correctness
- **Elegant Syntax**: Pure comments, no new keywords, editor-friendly
- **Integrated with Type System**: Specifications are types, extensible
- **Gradual Adoption**: Can transition from runtime checks to static verification progressively
- **Improved Reliability**: Can catch logical errors that are difficult to find through testing

### Disadvantages

- **Compilation Time**: Verification mode may significantly increase compilation time
- **Learning Curve**: Need to learn how to write effective specifications and quantifiers
- **SMT Solver Limitations**: Certain complex properties may not be automatically provable

## Alternative Solutions

| Solution | Advantages | Disadvantages |
|----------|------------|---------------|
| New keywords (such as `requires`) | Intuitive syntax | Introduces new keywords, breaks simplicity |
| Separate specification files (such as CVL) | Separates specifications from code | Increases file count, hard to keep synchronized |
| Runtime assertions only | Simple implementation | Cannot guarantee statically |
| **This solution (comments + specification types)** | Balances simplicity and functionality | Requires editor support |

## Implementation Strategy

### Phased Approach

| Phase | Content |
|-------|---------|
| **Phase 1: Basic Support** | Extend parser to recognize `//!` and `/*! ... !*/` comments, attach them to AST nodes; in verification mode, collect specifications and generate simple verification conditions (arithmetic comparisons only); integrate Z3 solver |
| **Phase 2: Quantifier Support** | Support quantifier expressions, translate to SMT-LIB `forall`/`exists`; provide IDE highlighting and hover tooltips for specifications |
| **Phase 3: Optimization and Toolchain** | Incremental verification, cache verified modules; specification coverage reports; specification mining tools (generate candidate specifications from tests) |

### Dependencies

- RFC-009: Ownership Model — Specification expressions require pure function semantics
- RFC-010: Unified Type Syntax — Specification type system based on the type system
- RFC-011: Generic Type System Design — Specification types support generic parameters

### Risks

1. **SMT Solver Integration Complexity**: Integration with solvers like Z3 may encounter technical challenges
   - Mitigation: Use mature Rust Z3 bindings, gradually expand supported expression types

2. **Difficult Debugging of Verification Failures**: When the SMT solver cannot prove a specification, users may have difficulty understanding why
   - Mitigation: Provide clear error messages and counterexample explanations

3. **Performance Overhead**: Verification mode may significantly increase compilation time
   - Mitigation: Implement incremental verification and caching mechanisms

## Open Questions

- [ ] **Quantifier Support Scope**: Support for nested quantifiers? Support for higher-order quantifiers?
- [ ] **Loop Invariant Inference**: Provide automatic inference for simple invariants?
- [ ] **Counterexample Format for Proof Failures**: What is the most effective way to present counterexamples?
- [ ] **Integration with Other Verification Tools**: Consider integration with proof assistants like Coq, Lean?

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
│ Under Review│  ← Community discussion
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
│ accepted/   │    │    rfc/     │
│ (Official)  │    │ (Preserved) │
└─────────────┘    └─────────────┘
```

### Status Descriptions

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author draft, awaiting submission for review |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes an official design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory, status updated |

### Actions After Acceptance

1. Move the RFC to the `docs/design/accepted/` directory
2. Update the filename to a descriptive name (such as `hoare-logic-static-verification.md`)
3. Update status to "Official"
4. Update status to "Accepted", add acceptance date

### Actions After Rejection

1. Preserve in `docs/design/rfc/` directory
2. Add rejection reason and date at the top of the file
3. Update status to "Rejected"