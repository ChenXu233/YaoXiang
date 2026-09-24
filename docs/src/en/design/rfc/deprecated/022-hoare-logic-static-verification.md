---
title:
  'RFC 022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)'
status: 'Deprecated (superseded by RFC-027)'
author: 'Chenxu'
created: '2026-03-16'
updated: '2026-06-07 (Deprecated: superseded by the compile-time evaluation type system)'
---

> **⚠️ DEPRECATED**
>
> This RFC has been superseded by
> **[RFC-027: Compile-Time Evaluation Types and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)**.
>
> **Reason for deprecation**: RFC 022 designed specifications as an external `//!` comment-based
> syntax, which contradicts the fundamental principle of the Curry-Howard correspondence—"No `//!`
> comments. No standalone specification language. Everything lives within the type system." The new
> design treats compile-time evaluation types as first-class citizens, replacing comment-style
> specifications with a unified compile-time Boolean evaluation pipeline. The Debug/Release
> split-verification pattern has also been replaced by a unified True/False/Unknown three-level
> return value model.
>
> This document is preserved for historical reference only.

---

# RFC 022: Hoare Logic Static Verification Support (Specification Comments and Specification Types) [Deprecated]

> **References**:
>
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)

## Summary

This document proposes introducing a **Hoare logic static verification mechanism** for the YaoXiang
language, allowing developers to write preconditions, postconditions, and loop invariants in the
form of `//!` or `/*! ... !*/` comments. In Debug Build mode, static verification is enforced and
must pass before proceeding to Release Build; in Release Build mode, specification comments are
ignored (zero overhead) and the verification cache is cleared. Specifications themselves are treated
as part of the type system, forming "specification types" (such as `Requires(P)`, `Ensures(P)`),
which can be extended by users. This design aims to keep the language simple while providing high
reliability guarantees for critical code, and integrates perfectly with YaoXiang's unified type
model.

## Motivation

### Why is this feature/change needed?

YaoXiang already guarantees memory safety and thread safety through the ownership model (RFC-009)
and the concurrency model (RFC-001), but logical correctness still depends on testing. For systems
programming and safety-critical domains (such as aerospace, finance, operating system kernels),
logical errors can lead to catastrophic consequences. Existing solutions (such as Rust's borrow
checker) cannot capture such errors. Hoare logic provides a mathematical proof mechanism, but
traditional formal verification tools often require a separate specification language and have a
steep learning curve.

### Current Problems

- Logical correctness can only be verified through testing, and cannot be guaranteed at compile-time
- Critical system code lacks formal verification means
- Existing formal verification tools have steep learning curves and are disconnected from mainstream
  programming languages

## Proposal

### Core Design

Our goal is to design a **lightweight, language-integrated** static verification scheme:

- **Debug Build requires verification**: Developers write specifications in modules that require
  high reliability. In Debug Build mode, verification is enforced and must pass before proceeding to
  Release Build
- **Elegant syntax**: Uses `//!` comments, introduces no new keywords, and can be highlighted by
  editors
- **Integrated with the type system**: Specifications become part of types, can participate in type
  checking, and support user-defined specification types
- **Provable and testable**: Can be statically proven or downgraded to runtime assertions,
  facilitating incremental adoption

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
    // implementation...
}
```

- A specification is essentially a **type declaration** with a Boolean expression on the right-hand
  side
- The left-hand side is a specification type instance (with optional type parameters)
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

The Boolean expressions on the right-hand side of specifications use YaoXiang expression syntax,
supporting:

- Arithmetic, comparison, and logical operations
- Quantifiers: `forall i in 0..n: P(i)`, `exists i in 0..n: P(i)` — language-built-in logical
  constructs
- Function calls (must be pure functions)

### 2. Specification Type System

Specification types are essentially ordinary YaoXiang types, fully consistent with the unified
syntax model.

#### 2.1 Built-in Specification Types

The compiler provides the following commonly used specification types built-in (can be used directly
in specifications):

```yaoxiang
// Built-in specification type definitions
NonEmpty: (T: Type) -> Type = { len: T; len > 0 }
Positive: Type = { x: Int; x > 0 }
GreaterOrEqual: (T: Type) -> Type = { result: T, arr: Array(T); result >= arr[0] && forall i in 1..arr.len: result >= arr[i] }
Bounds: (T: Type) -> Type = { i: T, n: T; 0 <= i && i <= n }
SumInvariant: (T: Type) -> Type = { s: T, arr: Array(T); s == sum(arr[0..i]) }

// Quantifier constructs (language built-ins, not functions)
forall: (start: Int, end: Int, pred: (Int) -> Bool) -> Bool
exists: (start: Int, end: Int, pred: (Int) -> Bool) -> Bool
```

#### 2.2 User-Defined Specification Types

Consistent with ordinary type definitions, users can define their own specification types:

```yaoxiang
// Define a positive integer specification
Positive: Type = { x: Int; x > 0 }

// Define a sorted array specification
Sorted: (T: Ord) -> Type = {
    arr: Array(T);
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

// Define a maximum value specification
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

Specification types, like other types, support generic parameters and type constraints, and can
participate in type inference.

### 3. Compilation Modes

| Mode               | Behavior                                                                                                                                          | Option                                        |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| **Debug Build**    | Parses specifications, generates verification conditions, calls the SMT solver to prove; Release Build can only proceed after verification passes | `yaoxiangc --debug source.yx`                 |
| **Release Build**  | Ignores all `//!` comments, generates no code; clears all verification caches; enables aggressive optimization                                    | `yaoxiangc --release source.yx`               |
| **Runtime Checks** | Converts specifications to runtime assertions, panics on violation                                                                                | `yaoxiangc --enable-runtime-checks source.yx` |

In verification mode, if the proof fails, the compiler reports an error and provides possible
counterexamples (such as input values).

### 4. Verification Mechanism

The compiler converts specifications into verification conditions (VCs) and sends them to an
integrated SMT solver (such as Z3). The verification process is roughly as follows:

1. Collect `requires` and `ensures` for functions, and `invariant` for loops
2. For each loop, generate loop invariant proof obligations: holds before entering the loop,
   maintained after each iteration, and implies the postcondition after loop exit
3. Transform the function body into logical formulas, combine with specifications to form
   verification conditions
4. Call the SMT solver to check satisfiability

If the solver returns `unsat` (unsatisfiable), the specification holds; otherwise, report a
counterexample.

### 5. Integration with Testing

Runtime check mode can convert specifications into assertions for testing. Combined with
specification coverage tools, the extent to which tests cover specifications can be evaluated. In
the future, specification mining tools may be considered to automatically infer candidate
specifications from tests.

### 6. Editor Support

`//!` and `/*! ... !*/` can be recognized by editors as special comments and given a different color
(such as purple) to distinguish them from ordinary comments. The language server can provide hover
hints, completion, and verification error reports for specifications.

## Detailed Design

### Syntax Changes

| Before                          | After                                                |
| ------------------------------- | ---------------------------------------------------- |
| No specification comment syntax | Allow `//!` and `/*! ... !*/` specification comments |

### 7.1 Syntax Extension

On top of the existing syntax (RFC-010), allow zero or more `//!` or `/*! ... !*/` comments at the
beginning of function bodies and loop bodies. The specification syntax is consistent with the
unified type syntax:

```
spec_comment     ::= ('//!' spec_line) | ('/*!' spec_block '!*/')
spec_line        ::= spec_name ':' type_expr '=' expr
spec_name        ::= 'requires' | 'ensures' | 'invariant'
spec_block       ::= (spec_name ':' type_expr '=' expr ';')*
```

- A specification is essentially a type declaration: `spec_name: spec_type = Boolean expression`
- `type_expr` is a specification type expression, with optional type parameters
- `expr` uses YaoXiang expression syntax, supporting quantifiers

### 7.2 Type Checking

In verification mode, the compiler converts specification comments into corresponding specification
type instances and records them in the metadata of the function or loop.

### 7.3 Verification Condition Generation

Using weakest precondition or strongest postcondition calculus, combined with loop invariants,
first-order logic formulas are generated. The generated VCs use the SMT-LIB format and call external
solvers.

### 7.4 Error Reporting

If the proof fails, the solver may provide a model (counterexample). The compiler should convert
these counterexamples into a readable form, such as concrete input values, to help users debug.

### 7.5 Runtime Checks

In `--enable-runtime-checks` mode, the compiler converts specifications into `assert` statements:

- `requires`: insert `assert(cond)` at the function entry
- `ensures`: insert `assert(cond)` before all return points of the function, where `result` is
  replaced with the actual return value
- `invariant`: insert `assert(cond)` at the beginning of the loop body

### 7.6 Integration with Existing Designs

- **Ownership model**: Expressions in specifications follow ownership rules, can only read but not
  write (pure functions), avoiding side effects
- **Generics system**: Specification types support generic parameters (such as `Requires(P)`), and
  can be combined with generic functions/types
- **Dependent types**: Value-dependent types in specifications (such as array length `n`) are
  naturally available

### Impact on the Type System

- Specification types are ordinary YaoXiang types, consistent with the unified syntax model
- The compiler has commonly used specification types built-in (`Positive`, `NonEmpty`,
  `GreaterOrEqual`, etc.)
- Users can define custom specification types through ordinary type definitions
- Specification types can carry generic parameters and support type constraints

### Runtime Behavior

- **Debug Build**: Calls the SMT solver for static verification, increasing compile time; caches
  verification results upon success
- **Release Build**: Specification comments are ignored, zero runtime overhead; clears all
  verification caches; enables aggressive optimizations such as Span cache clearing
- **Runtime check mode**: Generates assert statements that detect violations at runtime

### Compiler Changes

- Parser: recognize specification comment syntax
- Semantic analysis: collect specifications, convert to specification types
- Verification backend: generate verification conditions, call the SMT solver
- Code generation: support runtime check mode

### Backward Compatibility

- ✅ Fully backward compatible
- Specification comments are ignored under normal compilation, not affecting existing code
- Specifications are ignored in Release Build, with no additional overhead

## Trade-offs

### Advantages

- **Debug Build verification**: Forced verification in Debug Build mode ensures logical correctness
- **Elegant syntax**: Pure comments, no new keywords, editor-friendly
- **Integrated with the type system**: Specifications are types, extensible
- **Incremental adoption**: Can gradually transition from runtime checks to static verification
- **Improved reliability**: Can catch logical errors that are difficult to find through testing

### Disadvantages

- **Compile time**: Verification mode may significantly increase compile time
- **Learning curve**: Requires learning how to write effective specifications and quantifiers
- **SMT solver limitations**: Some complex properties may not be provable automatically

## Alternatives

| Scheme                                           | Advantages                            | Disadvantages                              |
| ------------------------------------------------ | ------------------------------------- | ------------------------------------------ |
| New keywords (such as `requires`)                | Intuitive syntax                      | Introduces new keywords, breaks simplicity |
| Separate specification files (such as CVL)       | Separates specifications from code    | Increases file count, hard to keep in sync |
| Runtime assertions only                          | Simple to implement                   | Cannot guarantee statically                |
| **This scheme (comments + specification types)** | Balances simplicity and functionality | Requires editor support                    |

## Implementation Strategy

### Phased Plan

| Phase                                 | Content                                                                                                                                                                                                            |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Phase 1: Basic Support**            | Extend the parser to recognize `//!` and `/*! ... !*/` comments, attach them to AST nodes; collect specifications in verification mode, generate simple VCs (arithmetic comparisons only); integrate the Z3 solver |
| **Phase 2: Quantifier Support**       | Support quantifier expressions, translate to SMT-LIB's `forall`/`exists`; provide IDE highlighting and hover hints for specifications                                                                              |
| **Phase 3: Optimization and Tooling** | Incremental verification, caching of verified modules; specification coverage reports; specification mining tools (generate candidate specifications from tests)                                                   |

### Dependencies

- RFC-009: Ownership Model - specification expressions require pure function semantics
- RFC-010: Unified Type Syntax - the specification type system is based on the type system
- RFC-011: Generics System Design - specification types support generic parameters

### Risks

1. **SMT solver integration complexity**: Integrating solvers like Z3 may encounter technical
   challenges
   - Mitigation: Use mature Rust Z3 bindings, gradually expand the supported expression types

2. **Difficulty debugging verification failures**: When the SMT solver cannot prove a specification,
   users may find it hard to understand the reason
   - Mitigation: Provide clear error messages and counterexample explanations

3. **Performance overhead**: Verification mode may significantly increase compile time
   - Mitigation: Implement incremental verification and caching mechanisms

## Open Questions

- [ ] **Quantifier support scope**: Are nested quantifiers supported? Are higher-order quantifiers
      supported?
- [ ] **Loop invariant inference**: Is automatic inference of simple invariants provided?
- [ ] **Format of counterexamples for failed proofs**: How to present counterexamples most
      effectively?
- [ ] **Integration with other verification tools**: Should integration with proof assistants like
      Coq and Lean be considered?

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
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
│ (formal    │    │ (kept in    │
│  design)   │    │   place)    │
└─────────────┘    └─────────────┘
```

### Status Description

| Status           | Location                | Description                                             |
| ---------------- | ----------------------- | ------------------------------------------------------- |
| **Draft**        | `docs/design/rfc/`      | Author's draft, awaiting submission for review          |
| **Under Review** | `docs/design/rfc/`      | Open to community discussion and feedback               |
| **Accepted**     | `docs/design/accepted/` | Becomes a formal design document, enters implementation |
| **Rejected**     | `docs/design/rfc/`      | Kept in the RFC directory, status updated               |

### Actions After Acceptance

1. Move the RFC to the `docs/design/accepted/` directory
2. Update the file name to a descriptive one (e.g. `hoare-logic-static-verification.md`)
3. Update status to "Formal"
4. Update status to "Accepted", add the acceptance date

### Actions After Rejection

1. Keep in the `docs/design/rfc/` directory
2. Add the reason for rejection and date at the top of the file
3. Update status to "Rejected"
