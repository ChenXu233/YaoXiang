---
title:
  'RFC 022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)'
status: 'Deprecated'
author: '晨煦'
created: '2026-03-16'
updated: '2026-06-07 (Deprecated: superseded by the compile-time evaluation type system)'
---

> **⚠️ DEPRECATED**
>
> This RFC has been superseded by
> **[RFC-027: Compile-Time Evaluation Types and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)**.
>
> **Reason for deprecation**: RFC 022 designed specifications as an external syntax in the form of
> `//!` comments, which contradicts the fundamental principle of Curry-Howard isomorphism — "No
> `//!` comments. No separate specification language. Everything lives inside the type system." The
> new design makes compile-time evaluation types first-class citizens and replaces comment-style
> specifications with a unified compile-time Bool evaluation pipeline. The Debug/Release split
> verification mode has also been replaced by a unified three-level return value model of
> True/False/Unknown.
>
> This document is retained for historical reference only.

---

# RFC 022: Hoare Logic Static Verification Support (Specification Comments and Specification Types) [Deprecated]

> **References**:
>
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)

## Summary

This document proposes introducing a **Hoare logic static verification mechanism** for the YaoXiang
language, allowing developers to write preconditions, postconditions, and loop invariants in the
form of `//!` or `/*! ... !*/` comments. In Debug Build, mandatory static verification is performed,
and only after verification passes can a Release Build proceed; during Release Build, specification
comments are ignored (zero overhead) and the verification cache is cleared. Specifications
themselves are treated as part of the type system, forming "specification types" (e.g.
`Requires(P)`, `Ensures(P)`), which can be extended by users. This design aims to maintain language
simplicity while providing high-reliability guarantees for critical code, and integrates perfectly
with YaoXiang's unified type model.

## Motivation

### Why is this feature/change needed?

YaoXiang already guarantees memory safety and thread safety through its ownership model (RFC-009)
and concurrency model (RFC-001), but logical correctness still relies on testing. For systems
programming and safety-critical domains (such as aerospace, finance, and operating system kernels),
logical errors can lead to catastrophic consequences. Existing solutions (such as Rust's borrow
checker) cannot catch such errors. Hoare logic provides a means of mathematical proof, but
traditional formal verification tools often require a separate specification language and a steep
learning curve.

### Current Problems

- Logical correctness can only be verified through testing and cannot be guaranteed at compile time
- Critical system code lacks formal verification means
- Existing formal verification tools have a steep learning curve and are disconnected from
  mainstream programming languages

## Proposal

### Core Design

Our goal is to design a **lightweight, language-integrated** static verification solution:

- **Debug Build must verify**: Developers write specifications in modules that require high
  reliability, and during Debug Build, verification is mandatory before a Release Build can proceed
- **Elegant syntax**: Uses `//!` comments, introduces no new keywords, and can be highlighted by
  editors
- **Fused with the type system**: Specifications become part of types, can participate in type
  checking, and support user-defined specification types
- **Provable and testable**: Can be statically proven or downgraded to runtime assertions,
  facilitating gradual adoption

### 1. Specification Comment Syntax

At the beginning of a function body or loop body, use `//!` (single-line) or `/*! ... !*/`
(multi-line) to write specifications.

#### 1.1 Unified Specification Syntax

Specifications adopt YaoXiang's unified `name: Type = expression` syntax model, fully integrated
with the type system:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0
    //! ensures: GreaterOrEqual(result, arr[0..n])
    //! ensures: ExistsMax(result, arr[0..n])
    // Implementation...
}
```

- A specification is essentially a **type declaration** with a Boolean expression on the right-hand
  side
- The left-hand side is a specification type instance (may carry type parameters)
- The special variable `result` may be used to represent the return value

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

The Boolean expressions on the right-hand side of specifications use YaoXiang expression syntax and
support:

- Arithmetic, comparison, and logical operations
- Quantifiers: `forall i in 0..n: P(i)`, `exists i in 0..n: P(i)` — language built-in logical
  constructs
- Function calls (must be pure functions)

### 2. Specification Type System

Specification types are essentially ordinary types in YaoXiang, fully consistent with the unified
syntax model.

#### 2.1 Built-in Specification Types

The compiler has the following commonly used specification types built in (usable directly in
specifications):

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

Consistent with ordinary type definitions, users can define their own specification types:

```yaoxiang
// Definition of a positive integer specification
Positive: Type = { x: Int; x > 0 }

// Definition of a sorted array specification
Sorted: (T: Ord) -> Type = {
    arr: Array(T);
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

// Definition of a maximum value specification
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

Like other types, specification types support generic parameters and type constraints, and can
participate in type inference.

### 3. Compilation Modes

| Mode              | Behavior                                                                                                                                          | Option                                        |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| **Debug Build**   | Parse specifications, generate verification conditions, invoke the SMT solver to prove; verification must pass before a Release Build can proceed | `yaoxiangc --debug source.yx`                 |
| **Release Build** | Ignore all `//!` comments, generate no code; clear all verification caches; enable aggressive optimizations                                       | `yaoxiangc --release source.yx`               |
| **Runtime Check** | Convert specifications into runtime assertions, panic on violation                                                                                | `yaoxiangc --enable-runtime-checks source.yx` |

In verification mode, if a proof fails, the compiler will report an error and provide possible
counterexamples (e.g., input values).

### 4. Verification Mechanism

The compiler converts specifications into Verification Conditions (VCs) and sends them to an
integrated SMT solver (e.g., Z3). The verification process is roughly as follows:

1. Collect the function's `requires` and `ensures`, and the loop's `invariant`
2. For each loop, generate loop invariant proof obligations: holds before entering the loop,
   maintained after each iteration, and implies the postcondition after the loop exits
3. Convert the function body into logical formulas, combine with the specifications to form
   verification conditions
4. Invoke the SMT solver to check satisfiability

If the solver returns `unsat` (unsatisfiable), the specification holds; otherwise, a counterexample
is reported.

### 5. Combination with Testing

In runtime check mode, specifications can be converted into assertions for use in testing. Combined
with a specification coverage tool, one can evaluate the degree to which tests cover the
specifications. In the future, a specification mining tool could be considered to automatically
infer candidate specifications from tests.

### 6. Editor Support

`//!` and `/*! ... !*/` can be recognized by editors as special comments, assigned a different color
(e.g., purple), to distinguish them from ordinary comments. The language server can provide hover
hints, completions, and verification error reports for specifications.

## Detailed Design

### Syntax Changes

| Before                          | After                                                |
| ------------------------------- | ---------------------------------------------------- |
| No specification comment syntax | Allow `//!` and `/*! ... !*/` specification comments |

### 7.1 Syntax Extension

On top of the existing syntax (RFC-010), allow zero or more `//!` or `/*! ... !*/` comments to
appear at the beginning of function bodies and loop bodies. The specification syntax is consistent
with the unified type syntax:

```
spec_comment     ::= ('//!' spec_line) | ('/*!' spec_block '!*/')
spec_line        ::= spec_name ':' type_expr '=' expr
spec_name        ::= 'requires' | 'ensures' | 'invariant'
spec_block       ::= (spec_name ':' type_expr '=' expr ';')*
```

- A specification is essentially a type declaration: `spec_name: spec_type = boolean_expression`
- `type_expr` is a specification type expression, which may carry type parameters
- `expr` uses YaoXiang expression syntax and supports quantifiers

### 7.2 Type Checking

In verification mode, the compiler converts specification comments into corresponding specification
type instances and records them in the metadata of the function or loop.

### 7.3 Verification Condition Generation

Use weakest precondition or strongest postcondition calculus, combined with loop invariants, to
generate first-order logic formulas. The generated VCs use the SMT-LIB format and invoke an external
solver.

### 7.4 Error Reporting

If a proof fails, the solver may provide a model (counterexample). The compiler should convert these
counterexamples into a human-readable form, such as specific input values, to help users debug.

### 7.5 Runtime Checks

In `--enable-runtime-checks` mode, the compiler converts specifications into `assert` statements:

- `requires`: Insert `assert(cond)` at the function entry
- `ensures`: Insert `assert(cond)` before all return points of the function, where `result` is
  replaced by the actual return value
- `invariant`: Insert `assert(cond)` at the beginning of the loop body

### 7.6 Integration with Existing Designs

- **Ownership model**: Expressions in specifications obey ownership rules and can only read, not
  write (pure functions), to avoid side effects
- **Generic system**: Specification types support generic parameters (e.g., `Requires(P)`) and can
  be combined with generic functions/types
- **Dependent types**: Values that depend on types in specifications (e.g., array length `n`) are
  naturally available

### Type System Impact

- Specification types are ordinary types in YaoXiang, consistent with the unified syntax model
- The compiler has commonly used specification types built in (`Positive`, `NonEmpty`,
  `GreaterOrEqual`, etc.)
- Users can define custom specification types through ordinary type definitions
- Specification types can carry generic parameters and support type constraints

### Runtime Behavior

- **Debug Build**: Calls the SMT solver for static verification, increasing compilation time; caches
  verification results after success
- **Release Build**: Specification comments are ignored, zero runtime overhead; clears all
  verification caches; enables aggressive optimizations such as Span cache clearing
- **Runtime Check Mode**: Generates assert statements to detect violations at runtime

### Compiler Changes

- Parser: Recognize the specification comment syntax
- Semantic analysis: Collect specifications and convert them into specification types
- Verification backend: Generate verification conditions and call the SMT solver
- Code generation: Support runtime check mode

### Backward Compatibility

- ✅ Fully backward compatible
- Specification comments are ignored in normal compilation and do not affect existing code
- Specifications are ignored during Release Build with no additional overhead

## Trade-offs

### Advantages

- **Debug Build verification**: Mandatory verification during Debug Build ensures logical
  correctness
- **Elegant syntax**: Pure comments, no new keywords, editor-friendly
- **Fused with the type system**: Specifications are types and can be extended
- **Gradual adoption**: Can transition from runtime checks to static verification incrementally
- **Improved reliability**: Can catch logical errors that are difficult to discover through testing

### Disadvantages

- **Compilation time**: Verification mode may significantly increase compilation time
- **Learning curve**: Requires learning how to write effective specifications and quantifiers
- **SMT solver limitations**: Some complex properties may not be provable automatically

## Alternatives

| Approach                                           | Advantages                            | Disadvantages                              |
| -------------------------------------------------- | ------------------------------------- | ------------------------------------------ |
| New keywords (e.g., `requires`)                    | Intuitive syntax                      | Introduces new keywords, breaks simplicity |
| Separate specification files (e.g., CVL)           | Separates specifications from code    | Increases file count, hard to keep in sync |
| Runtime assertions only                            | Simple to implement                   | Cannot guarantee statically                |
| **This approach (comments + specification types)** | Balances simplicity and functionality | Requires editor support                    |

## Implementation Strategy

### Phases

| Phase                                 | Content                                                                                                                                                                                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Phase 1: Basic Support**            | Extend the parser to recognize `//!` and `/*! ... !*/` comments, attach them to AST nodes; in verification mode, collect specifications and generate simple verification conditions (arithmetic comparisons only); integrate the Z3 solver |
| **Phase 2: Quantifier Support**       | Support quantifier expressions, translating them to SMT-LIB `forall`/`exists`; provide IDE highlighting and hover hints for specifications                                                                                                 |
| **Phase 3: Optimization and Tooling** | Incremental verification, caching of verified modules; specification coverage reports; specification mining tools (generating candidate specifications from tests)                                                                         |

### Dependencies

- RFC-009: Ownership Model - Specification expressions require pure function semantics
- RFC-010: Unified Type Syntax - The specification type system is based on the type system
- RFC-011: Generic Type System Design - Specification types support generic parameters

### Risks

1. **SMT solver integration complexity**: Integrating solvers like Z3 may encounter technical
   challenges
   - Mitigation: Use mature Rust Z3 bindings, and gradually expand the range of supported expression
     types

2. **Difficulty of debugging failed verification**: When the SMT solver cannot prove a
   specification, users may find it hard to understand why
   - Mitigation: Provide clear error messages and counterexample explanations

3. **Performance overhead**: Verification mode may significantly increase compilation time
   - Mitigation: Implement incremental verification and caching mechanisms

## Open Questions

- [ ] **Scope of quantifier support**: Should nested quantifiers be supported? Should higher-order
      quantifiers be supported?
- [ ] **Loop invariant inference**: Should automatic inference of simple invariants be provided?
- [ ] **Counterexample format for failed proofs**: How to present counterexamples most effectively?
- [ ] **Integration with other verification tools**: Should integration with proof assistants such
      as Coq and Lean be considered?

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
- [JML Reference Manual](https://www.openjml.org/)
- [The SPARK Toolset](https://www.adacore.com/about-spark)
- [Z3 SMT Solver](https://github.com/Z3Prover/z3)

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review│  ← Community discussion
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
│ (Official)  │    │ (Remain in place) │
└─────────────┘    └─────────────┘
```

### Status Descriptions

| Status           | Location                | Description                                                        |
| ---------------- | ----------------------- | ------------------------------------------------------------------ |
| **Draft**        | `docs/design/rfc/`      | Author's draft, awaiting submission for review                     |
| **Under Review** | `docs/design/rfc/`      | Open for community discussion and feedback                         |
| **Accepted**     | `docs/design/accepted/` | Becomes an official design document, entering implementation phase |
| **Rejected**     | `docs/design/rfc/`      | Remains in the RFC directory, with status updated                  |

### Actions After Acceptance

1. Move the RFC to the `docs/design/accepted/` directory
2. Update the filename to a descriptive name (e.g., `hoare-logic-static-verification.md`)
3. Update the status to "Official"
4. Update the status to "Accepted" and add the acceptance date

### Actions After Rejection

1. Remain in the `docs/design/rfc/` directory
2. Add the reason for rejection and the date at the top of the file
3. Update the status to "Rejected"
