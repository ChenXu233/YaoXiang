---
title: 'RFC 013: Error Code Specification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-02'
updated: '2026-09-03'
issue: '#125'
issues_impl:
  - '#125'
pr_impl:
  - '#7'
  - '#9'
  - '#29'
  - '#66'
---

# RFC 013: Error Code Specification

## Summary

This RFC proposes an error code classification specification for the YaoXiang compiler. It uses a
Rust-like flat numbering system, supports multiple languages through JSON resource files, and
provides error explanations through the `yaoxiang explain` command.

## Motivation

### Why are standardized error codes needed?

1. **User experience**: An error code lets users quickly identify the error type and severity.
2. **Documentation organization**: Grouping codes by category makes error reference documentation
   easier to write and maintain.
3. **Tool integration**: IDEs/LSPs can provide quick-fix suggestions and documentation links based
   on error codes.
4. **Internationalization**: Separating error messages from codes makes translation into multiple
   languages easier.

### Design Goals

- **Concise**: Flat numbering means users do not need to memorize complex classification rules.
- **Friendly**: A Rust-like error message format with help text and examples.
- **Extensible**: Resource-file-driven, making it easy to add new errors and languages.
- **Tool-friendly**: The explain command and JSON output support IDE/LSP integration.

---

## Proposal

### Core Design: Flat Numbering System

Four-digit numeric identifiers are used, grouped by compilation phase:

```
Exxxx
││││
│││└── 序号 (000-999)
││└─── 编译阶段 (0-9)
└───── 固定前缀 'E'
```

### Phase Classification

| Phase | Range | Description                    |
| ----- | ----- | ------------------------------ |
| **0** | E0xxx | Lexical and syntactic analysis |
| **1** | E1xxx | Type checking                  |
| **2** | E2xxx | Semantic analysis              |
| **3** | E3xxx | Code generation                |
| **4** | E4xxx | Generics and traits            |
| **5** | E5xxx | Modules and imports            |
| **6** | E6xxx | Runtime errors                 |
| **7** | E7xxx | I/O and system errors          |
| **8** | E8xxx | Internal compiler errors       |
| **9** | E9xxx | Reserved/experimental          |

### Error Category Enumeration

```rust
/// Error categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: lexical and syntactic analysis
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E0xxx: type checking
    Semantic,   // E2xxx: semantic analysis
    Generic,    // E4xxx: generics and traits
    Module,     // E5xxx: modules and imports
    Runtime,    // E6xxx: runtime errors
    Io,         // E7xxx: I/O and system errors
    Internal,   // E8xxx: internal compiler errors
}
```

### Error Code Definitions and Generic Builder

**Core principle**: Separate error code definitions from display text.

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without display text.
- `locales/*.json`: Display text for each language (title, message, help, with error codes
  represented as nested objects).
- `DiagnosticBuilder`: A generic builder that replaces the trait-per-error design.

#### Error Code Definitions

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only; display text is in the i18n files)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // Message template; supports {param} placeholders
}

/// Generic diagnostic builder
pub struct DiagnosticBuilder {
    code: &'static str,
    message_template: &'static str,
    params: Vec<(&'static str, String)>,
    span: Option<Span>,
}

impl DiagnosticBuilder {
    pub fn new(code: &'static str, template: &'static str) -> Self {
        Self {
            code,
            message_template: template,
            params: Vec::new(),
            span: None,
        }
    }

    /// Adds a template parameter
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// Sets the location
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Builds a Diagnostic (template rendering is completed at compile time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Check that every {key} in the template has a corresponding parameter
        self.validate_params();

        let message = i18n.render(self.message_template, &self.params);
        let help = self.help(i18n);

        Diagnostic {
            severity: Severity::Error,
            code: self.code.to_string(),
            message,
            help,
            span: self.span,
            related: Vec::new(),
        }
    }
}
```

#### Convenience Methods for Each Error Code

```rust
// diagnostic/codes/e1xxx.rs

impl ErrorCodeDefinition {
    /// E1001 unknown variable
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 type mismatch
    pub fn type_mismatch(expected: &str, found: &str) -> DiagnosticBuilder {
        let def = Self::find("E1002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected)
            .param("found", found)
    }
}
```

#### Usage Example

```rust
// checking/mod.rs

use crate::util::diagnostic::codes::{ErrorCodeDefinition, E1001};

// Simplified form
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// Manual form
return Err(ErrorCodeDefinition::find("E1001")
    .builder()
    .param("name", var_name)
    .at(span)
    .build(&i18n_registry));
```

#### Error Code Definition Example

```rust
// diagnostic/codes/e1xxx.rs

pub static E1XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E1001",
        category: ErrorCategory::TypeCheck,
        message_template: "Unknown variable: '{name}'",
    },
    ErrorCodeDefinition {
        code: "E1002",
        category: ErrorCategory::TypeCheck,
        message_template: "Expected type '{expected}', found type '{found}'",
    },
    // ... Other error codes
];
```

#### Design Advantages

| Feature                   | Description                                                             |
| ------------------------- | ----------------------------------------------------------------------- |
| **Single Builder**        | One generic `DiagnosticBuilder` serves all error codes                  |
| **Type safety**           | Convenience methods ensure argument correctness                         |
| **Self-documenting**      | `E1001::unknown_variable(name)` is immediately clear                    |
| **Template separation**   | Message templates are separated from code, facilitating i18n            |
| **Zero runtime overhead** | Rendering occurs at compile time; AOT binaries contain no lookup tables |

---

### Simplified Error Macros

#### error! Macro (Automatic Context Injection)

```rust
/// Macro that automatically obtains the span and i18n configuration at compile time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: only pass the arguments; the span and i18n are injected automatically
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Manual Use of the Builder

```rust
// Use when manual control is required
E1001::unknown_variable(&var_name)
    .at(my_span)           // Custom span
    .build(&custom_i18n)   // Custom i18n
```

---

## Detailed Design

### Error Code List

#### E0xxx: Lexical and Syntactic Analysis

<!-- code-table:E0xxx start -->

| Code  | Description               |
| ----- | ------------------------- |
| E0001 | Invalid character         |
| E0002 | Invalid numeric literal   |
| E0003 | Unterminated string       |
| E0004 | Invalid character literal |
| E0010 | Expected token            |
| E0011 | Unexpected token          |
| E0012 | Invalid syntax            |
| E0013 | Mismatched parentheses    |
| E0014 | Missing semicolon         |
| E0016 | Expected expression       |
| E0018 | Keyword used as a name    |

<!-- code-table:E0xxx end -->

#### E1xxx: Type Checking

<!-- code-table:E1xxx start -->

| Code  | Description                                                     |
| ----- | --------------------------------------------------------------- |
| E1001 | Unknown variable                                                |
| E1002 | Type mismatch                                                   |
| E1003 | Unknown type                                                    |
| E1010 | Argument count mismatch                                         |
| E1011 | Impl parameter annotation does not match declaration            |
| E1012 | Return type mismatch                                            |
| E1013 | Function not found                                              |
| E1014 | Unknown named argument                                          |
| E1015 | Duplicate argument specified                                    |
| E1020 | Unable to infer type                                            |
| E1021 | Conflicting type inference                                      |
| E1030 | Incomplete pattern                                              |
| E1031 | Unreachable pattern                                             |
| E1032 | Duplicate binding in pattern                                    |
| E1033 | Inconsistent bindings in or-patterns                            |
| E1034 | Missing field in struct pattern                                 |
| E1040 | Unsupported operation                                           |
| E1041 | Index out of bounds                                             |
| E1042 | Field not found                                                 |
| E1043 | Module member not found                                         |
| E1050 | Boolean operand required                                        |
| E1051 | Logical NOT requires a boolean operand                          |
| E1052 | Invalid dereference                                             |
| E1053 | Field access on a non-struct type                               |
| E1054 | Conditional type mismatch                                       |
| E1055 | Constraint used in a non-generic context                        |
| E1060 | Type parameter count mismatch                                   |
| E1061 | Unable to instantiate generic                                   |
| E1062 | const generic constraint failed                                 |
| E1064 | Invalid binding position index                                  |
| E1065 | Call on a non-function value                                    |
| E1071 | Type definitions are only allowed at module scope               |
| E1081 | `?` may only be used in functions returning a propagatable type |
| E1082 | `?` may only be used with types implementing Try                |
| E1083 | Error type of `?` does not match                                |
| E1090 | ✨ Unspeakable ✨                                               |
| E1091 | Invalid generic meta type                                       |
| E1092 | Illegal form of refinement type argument                        |
| E1093 | Refinement argument count mismatch                              |
| E1094 | Unused compile-time value parameter                             |
| E1095 | Unknown interface                                               |
| E1096 | Interface parameter count mismatch                              |
| E1097 | Naming conflict among interface members                         |
| E1098 | Interface method not implemented                                |
| E1099 | Interface method signature mismatch                             |
| E1100 | Duplicate implementation of an interface method                 |
| E1101 | Type does not implement the interface                           |
| E1102 | Loop control statement outside a loop                           |
| E1103 | Square brackets cannot be used in type position                 |
| E1104 | Interface implementation is not in the type's defining module   |
| E1105 | Variant constructor cannot be accessed as a field               |
| E1106 | Constraint method dispatch failed                               |
| E1107 | Ambiguous method overload                                       |
| E1108 | Empty block falls into the expected container position          |
| E1109 | `return` outside a function                                     |
| E1110 | Method binding lacks a signature annotation                     |
| E1111 | Argument used more than once or in an illegal order             |
| E1112 | Number of impl parameters exceeds the declaration               |
| E1113 | Compile-time value parameter lacks compile-time evidence        |

<!-- code-table:E1xxx end -->

> **RFC-011b association (note dated 2026-09-22)**:
> [RFC-011b: Operator Overloading](011b-operator-overloading.md) Implementation will affect three
> areas in this section—① remove the word "Result" from the `E1081` / `E1082` messages (`?` will be
> determined according to the `Try` interface instead of being bound to a specific type name), and
> synchronize them during stage 2 under this document's "three-way consistency" process (codes/*.rs
> ↔ locales ↔ code table); ② rejection diagnostics for an unmet `Equal` prerequisite constraint
> (linear token) reuse the `E1101` (type does not implement the interface) family; ③ after stage 1
> is wired up, `Struct == Struct` changes from an `E6007` runtime error to a compile-time
> determination, reducing the trigger surface of `E6007`. The registered wording in the table
> remains unchanged until the implementation lands.

#### E2xxx: Semantic Analysis

<!-- code-table:E2xxx start -->

| Code  | Description                            |
| ----- | -------------------------------------- |
| E2001 | Scope error                            |
| E2002 | Duplicate definition                   |
| E2003 | Ownership error                        |
| E2010 | Immutable assignment                   |
| E2011 | Use of uninitialized variable          |
| E2012 | Mutability conflict                    |
| E2013 | Variable shadowing                     |
| E2014 | Use of moved value                     |
| E2016 | Immutable assignment                   |
| E2018 | Mutable/immutable borrow conflict      |
| E2019 | Double free                            |
| E2020 | Use after free                         |
| E2027 | unsafe dereference                     |
| E2029 | Reference cycle within spawn           |
| E2030 | Refinement type constraint violation   |
| E2031 | Refinement constraint cannot be proven |
| E2090 | Invalid signature                      |
| E2091 | Unknown type in signature              |
| E2092 | Signature missing an arrow             |
| E2093 | Duplicate parameter name               |
| E2094 | Generic parameter shadowing            |
| E2095 | Parameter name shadows generic         |
| E2096 | Bare container type in signature       |

<!-- code-table:E2xxx end -->

> Reserved code notes (inventory dated 2026-09-14, release criteria for #251): E2019 (double free),
> E2020 (use after free), E2027 (unsafe dereference), and E2029 (ref cycle within spawn) have been
> registered and are anchored by unit tests, but no reachable yx source surface yet exists (explicit
> drop statements, Ptr dereference grammar, or a spawn ref-cycle construction path)—the claim of
> complete semantic correctness does not cover these four codes; treat them as "reserved" until the
> implementation is completed.

#### E3xxx: Code Generation

<!-- code-table:E3xxx start -->

| Code  | Description                                        |
| ----- | -------------------------------------------------- |
| E3004 | Unsupported iterator                               |
| E3005 | IR generation error                                |
| E3006 | Unresolved variable                                |
| E3007 | Top-level binding initializer must be constant     |
| E3008 | Unsupported match pattern                          |
| E3014 | Register overflow                                  |
| E3017 | Invalid operand (code generation)                  |
| E3018 | Monomorphization instantiation failed              |
| E3019 | Top-level binding cyclic dependency                |
| E3020 | Missing program entry point                        |
| E3021 | Entry point is not a function                      |
| E3022 | Entry point `main` signature mismatch              |
| E3023 | Executable statements are not allowed at top level |
| E3024 | Project configuration is unreadable                |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and Traits

<!-- code-table:E4xxx start -->

| Code  | Description                                     |
| ----- | ----------------------------------------------- |
| E4001 | Generic constraint violation                    |
| E4002 | Trait not found                                 |
| E4003 | Missing trait implementation                    |
| E4004 | Conflicting trait implementation                |
| E4005 | Associated type not found                       |
| E4010 | Constant division by zero                       |
| E4011 | Constant overflow                               |
| E4012 | Constant recursion depth exceeded               |
| E4014 | Constant evaluation failed                      |
| E4018 | Refinement predicate violation                  |
| E4019 | Type equality does not hold                     |
| E4020 | Proof function required                         |
| E4021 | Loop termination cannot be automatically proven |
| E4022 | Measure does not hold                           |
| E4023 | Cannot prove within the kernel                  |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission points (reserved codes): the Sized constraint and
> optimization error path are not yet implemented; wire them to the actual trigger surface when
> implementing them.

#### E4023: General "Unable to Prove"

> **Status: Registered (2026-11-13)**. This section records the code's meaning and division of
> responsibility; the code is emitted through the general `Unproven` path, and all four mechanical
> registration steps were submitted together.

- **Code**: `E4023`
- **Category (face)**: `Generic` (E4xxx, generics and proof surface), in the same family as `E4021`
  and `E4022`.
- **Semantics**: A general "unable to prove" diagnostic. It is emitted when a proof cannot be
  constructed for an obligation within resource limits; it is not tied to a specific proposition
  shape.
- **Template**: `Unable to prove: {proposition} ({reason})`
- **Parameters**: `proposition` (renderable text for the unresolved proposition) and `reason`
  (capability boundary / exhausted budget / contains an unbound symbol name).
- **Location**: Non-exempt—the emission point supplies the source location; if a decision point
  truly has no location, use the default location and document the reason in an implementation
  comment.
- **Division of responsibility with E2031**: `E2031` is specific to refinement constraints, is
  emitted through the refinement channel, and carries a real location. Its message includes the
  assigned name, variable, and constraint. `E4023` is the general fallback for `Unproven` at
  equality and other obligation points, carrying the proposition and reason. The two codes must not
  replace one another: a missing proof for a refinement constraint still reports E2031, while an
  inability to determine equality or another obligation reports E4023.
- **Boundaries with existing codes**: `E4019` (type equality does not hold) corresponds to
  `Disproved`; `E4023` corresponds to `Unproven`. `E4014` (constant evaluation failed) corresponds
  to deterministic failure of closed-term evaluation and is not used for "not proven within resource
  limits".
- **Mechanical registration steps (completed)**: `define_codes!` entries in
  `src/util/diagnostic/codes/e4xxx.rs` → templates and parameters in six locale files → the E4xxx
  code table in this document (regenerated by the error-code documentation generator) → generated
  pages under `docs/src/reference/error-code/`. All four were submitted together.

#### E5xxx: Modules and Imports

<!-- code-table:E5xxx start -->

| Code  | Description          |
| ----- | -------------------- |
| E5001 | Module not found     |
| E5002 | Import error         |
| E5003 | Export not found     |
| E5004 | Circular dependency  |
| E5005 | Invalid module path  |
| E5006 | Duplicate import     |
| E5007 | Module export        |
| E5008 | Import name conflict |

<!-- code-table:E5xxx end -->

#### E6xxx: Runtime Errors

<!-- code-table:E6xxx start -->

| Code  | Description                   |
| ----- | ----------------------------- |
| E6001 | Division by zero              |
| E6003 | Array index out of bounds     |
| E6004 | Stack overflow                |
| E6005 | Assertion failed              |
| E6006 | Function not found (runtime)  |
| E6007 | Runtime error                 |
| E6008 | Key does not exist            |
| E6009 | Invalid Range step            |
| E6010 | Integer parsing failed        |
| E6011 | Floating-point parsing failed |
| E6012 | Invalid code point            |
| E6013 | JSON parsing failed           |

<!-- code-table:E6xxx end -->

> **Code table revision (2026-08-09)**: The original table followed Rust semantics as a draft
> (`Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed`), which did not
> match the implementation's actual requirements. YaoXiang has no concepts of null pointers, heap
> allocation failure, or type casts (value semantics + Rust memory safety), and overflow detection
> has not been implemented for runtime overflow paths. After calibration:
>
> - E6002 was removed (the original Assertion failed was moved to E6005; the original null-pointer
>   semantics have no language concept).
> - E6003 was changed from Arithmetic overflow to Runtime index out of bounds (the actual trigger
>   surface).
> - E6005 was changed from Heap allocation failed to Assertion failed (the actual `std.assert`
>   path).
> - E6006 was changed from Runtime index out of bounds to Function not found (the implementation has
>   always behaved this way).
> - E6007 was changed from Type cast failed to a generic Runtime error (all unmapped `ExecutorError`
>   variants use this as their common landing point).

#### E7xxx: I/O and System Errors

<!-- code-table:E7xxx start -->

| Code  | Description       |
| ----- | ----------------- |
| E7001 | File not found    |
| E7002 | Permission denied |
| E7003 | I/O error         |
| E7004 | Network error     |

<!-- code-table:E7xxx end -->

#### E8xxx: Internal Compiler Errors

<!-- code-table:E8xxx start -->

| Code  | Description             |
| ----- | ----------------------- |
| E8001 | Internal compiler error |
| E8002 | Unexpected panic        |
| E8003 | Compiler phase error    |

<!-- code-table:E8xxx end -->

#### W1xxx: Warning Codes

<!-- code-table:W1xxx start -->

| Code  | Description                                  |
| ----- | -------------------------------------------- |
| W1001 | Unused private function                      |
| W1002 | Unused private type                          |
| W1003 | Unused import                                |
| W1004 | Unused private variable                      |
| W1005 | Unused private method                        |
| W1006 | Local module shadows a dependency package    |
| W1007 | Module import cycle                          |
| W1063 | const generic constraint cannot be evaluated |
| W1080 | Compile-time proof degraded                  |
| W1081 | Termination obligation not determined        |
| W1082 | Ownership analysis degraded                  |
| W1090 | Obligation produced but has no consumer      |

<!-- code-table:W1xxx end -->

> W code-space rule: E codes and W codes are structurally identical and grouped by phase (W +
> phase-thousand segment); W1xxx = type-checking-phase warnings.
>
> **Dead code semantics (RFC-029f role semantics + RFC-029g)**: W1001/W1002/W1004/W1005 report
> **unreferenced top-level bindings and type definitions**—a file-local reachability analysis
> starting from `main` reports them when they are never referenced; methods (W1005) are matched by
> call-site short name. The sole exemption is the **package-internal reference pool** (RFC-029f: a
> name referenced by any file in the project is considered live—cross-file consumers are not visible
> in this file, so err toward underreporting).
>
> **Implementation correction (2026-10-08, RFC-029g)**: This section originally stated, "`pub`
> definitions are public interfaces and are never reported (#321 decision B)." The `pub` keyword has
> been removed, and modifiers no longer provide any exemption; the original role-based exemption
> policy (Bin: report immediately / Internal: tighten in the second stage / Lib: absolute exemption)
> has converged to a single condition—whether a package-internal reference pool is provided. For
> behavior changes, see [029g](029g-remove-pub-and-auto-bind.md).
>
> **Unused imports (W1003)**: Detected by typecheck's use elaboration (pass2 registers imported
> local names; an expression-resolution or type-annotation location is considered a use). It covers
> both whole-module imports (`use std.io` → module alias) and named imports (`use std.io.{print}`).
>
> **Emission channel**: W-code diagnostics are marked `Severity::Warning` by the builder by default
> based on the W prefix (an explicit value takes precedence). They are collected and presented on
> the same track as errors (`warning[W####]` prefix rendered), but do not block compilation or
> affect a successful exit code. `yaoxiang check --deny-warnings` upgrades warnings to failures
> (exits with a nonzero code when warnings exist) for strict CI mode. Per-code suppression (such as
> allow attributes) is a future extension.
>
> **Decoupling code families and emission severity (W1090 promoted to E, user decision on
> 2026-10-13)**: **A code prefix determines only its numbering family, not its emission
> severity**—the severity is determined by the explicit `severity` at the diagnostic construction
> point (`E` prefix defaults to Error, `W` prefix defaults to Warning, and an explicit value takes
> precedence). Accordingly, `W1090` (obligation produced but no consumer) retains its **W-family
> code number** (no new E code: two codes for the same fact would be parallel representations), but
> from WBS 4.3.2 onward its **emission severity is error**: when triggered, it counts toward
> `error_count`, is rendered as `error[W1090]`, and changes the exit code (the **terminal state** of
> RFC-039 D5; a D-class breaking change under D59, faithfully recorded in 02-stage-contract §2).
> Healthy programs produce no diagnostics (there is no report when all obligations have delivery
> markers), so this promotion affects build results only when an actual stage-contract defect
> occurs.

### Message Quality Specification

> This section was introduced by the single-track message and quality revision (2026-09-03). It is
> enforced in CI by `scripts/audit_diagnostics.py`.

1. **Single-track messages**: All user-visible diagnostic messages must be rendered through the
   authoritative registry's convenience methods and locale templates; the code must pass only
   structured parameters. Bypassing the registry to construct native values such as
   `Diagnostic::error(...)` is forbidden, because that path bypasses code validation and i18n.
2. **Code validity**: Unregistered codes and pseudo-codes such as `E_INTERNAL` are forbidden. Any
   code literal used at an emission point must already be defined in the registry. All internal
   errors must use E8001 (`internal_error`).
3. **Type display**: Type Display must distinguish pre-instantiation and post-instantiation forms
   (`Expected 'Container', found 'Container'` cannot distinguish bare names).
4. **Isolation of solver internal state**: Intermediate solver state TypeVar (Display form `t<N>`)
   must not enter user-visible messages. Test anchor:
   `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is only for internal compiler consistency problems (ICEs). E8001 must
   not be used as a fallback for user-correctable errors; ICE messages must include instructions for
   a minimal reproduction.

---

### Connecting Runtime Error Values to Codes

> This section was introduced by the runtime Error value carrying codes revision (2026-09-03). The
> E6xxx/E7xxx semantic space simultaneously carries two channels. They share the same code space but
> use different rendering channels.

#### Two Channels

| Channel                         | Carrier                                                           | Rendering                                                        |
| ------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------- |
| Compiler/CLI diagnostic channel | Host-layer hard errors such as `ExecutorError`                    | stderr `error[E####]:` (already wired for E6003/E6005/E6007)     |
| In-program error value channel  | The `Error` carrier for the Err variant of std `Result(T, Error)` | A language value consumed by program match/comparison operations |

#### Error Structure (Since v0.8, Breaking Change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbering from this specification, in string form such as `"E6008"`.
- **Stable contract**: The semantics of an allocated code do not change across versions; the same
  semantics are never reused for a deleted code (as with E6002).
- **Consumption surface**: In-program `e.code == "E6xxx"` comparison is the only programmable
  determination contract; `yaoxiang explain E6xxx` documentation is integrated; toolchains (LSP /
  DAP, see RFC-034) use the code as exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: E in `Result(T, E)` is a generic parameter. Proper modeling should use a
  user-defined type; std `Error` is only a convenient fallback carrier, and its code system does not
  constrain the user's E type.

#### Code Allocation Rules

1. Runtime error value codes and compiler diagnostic codes share the E6xxx/E7xxx space. New codes
   are allocated according to their **actual trigger surface**, without reserving codes for
   hypothetical scenarios.
2. Register before use: A new code must enter the authoritative registry and pass the three-way
   consistency check (codes/*.rs ↔ locales ↔ this document's code table) before it can be emitted.
   The registration source for runtime error value codes is the `RUNTIME_ERROR_CODES` table in
   `src/std/result.rs` (subject to the same `build.rs` build-time gate and `tools/code-tables`
   validation as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error values (currently unused; enabled when io/net become
   Result-based).
4. Emission points: std modules construct Error values through `error_new(code, message)`; consumers
   retrieve the Err carrier with `std.result.unwrap_err` and read its fields with
   `std.result.code/message`.

#### Evolution Path (Line C, Not Implemented)

After pattern-matching completeness (RFC-010b) is implemented, `Error` may be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming a property derived from the kind (the
code registry is defined at the variant declaration). During the evolution period, the code
stability contract in this section remains unchanged; this upgrade is an independent decision and
does not constitute a commitment in this section.

---

### Multilingual Resource Files

#### Resource File Format

```json
// locales/en.json
{
  "E1001": {
    "title": "Unknown variable",
    "message": "Referenced variable is not defined",
    "template": "Unknown variable: '{name}'",
    "help": "Check if the variable name is spelled correctly, or define it first",
    "example": "x = 100;",
    "error_output": "error[E1001]: Unknown variable: 'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ unknown variable 'x'"
  },
  "E1002": {
    "title": "Type mismatch",
    "message": "Expected type does not match actual type",
    "template": "Expected type '{expected}', found type '{found}'",
    "help": "Use the correct type or add a type conversion",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: Type mismatch\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ expected 'Int', found 'String'"
  }
}
```

```json
// locales/zh.json
{
  "E1001": {
    "title": "未知变量",
    "message": "引用的变量未定义",
    "template": "未知变量：'{name}'",
    "help": "检查变量名是否拼写正确，或先定义它",
    "example": "x = 100;",
    "error_output": "error[E1001]: 未知变量：'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ 未知变量 'x'"
  },
  "E1002": {
    "title": "类型不匹配",
    "message": "期望类型与实际类型不匹配",
    "template": "期望类型 '{expected}'，实际类型 '{found}'",
    "help": "使用正确的类型或添加类型转换",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: 类型不匹配\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ 期望 'Int'，找到 'String'"
  }
}
```

#### I18nRegistry Implementation

```rust
// locales/*.json（错误码对象）

/// i18n display-text registry (loaded from JSON at compile time, zero runtime lookup)
pub struct I18nRegistry {
    /// Title
    titles: HashMap<&'static str, &'static str>,
    /// Description
    messages: HashMap<&'static str, &'static str>,
    /// Help text
    helps: HashMap<&'static str, &'static str>,
    /// Example code
    examples: HashMap<&'static str, &'static str>,
    /// Example error output
    error_outputs: HashMap<&'static str, &'static str>,
}

/// Information for a single error code
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// Returns the registry for a language code
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// Gets error information
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// Renders a template (completed at compile time, zero runtime overhead)
    pub fn render(&self, template: &'static str, params: &[(&str, String)]) -> String {
        let mut result = String::with_capacity(template.len() + 64);
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if let Some((_, value)) = params.iter().find(|(k, _)| k == &key) {
                            result.push_str(value);
                        } else {
                            result.push_str(&format!("{{{}}}", key));
                        }
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}
```

#### Template Placeholders

##### Predefined Placeholders (Common)

| Placeholder  | Purpose                                            | Example                             |
| ------------ | -------------------------------------------------- | ----------------------------------- |
| `{name}`     | Identifiers such as variable, type, or trait names | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                                      | `Expected type '{expected}'`        |
| `{found}`    | Actual/found type                                  | `, found type '{found}'`            |
| `{method}`   | Method name                                        | `Method {method} is not a function` |
| `{trait}`    | Trait name                                         | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                        | `Invalid path: {path}`              |
| `{ty}`       | Type expression                                    | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                             | `Internal error: {message}`         |

##### Support for Arbitrary Keys

**params supports any key and is not limited to predefined ones.** Callers may pass any `key`:

```rust
// Use an arbitrary key
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// Template definition
"Unknown variable: '{name}' at {location}. {hint}"
```

> **Note**: Not every error code uses placeholders. Some error codes, such as E0001, use static
> messages and require no parameters.

#### Language Priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. 默认值: en
```

### yaoxiang.toml Configuration

#### Project-Level Configuration

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# Error message language; options: en, zh, ja, ...
default = "zh"
```

#### User-Level Configuration

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### Compile-Time Language Selection

```
1. 读取项目级 yaoxiang.toml 的 language.default
2. 若未配置，读取用户级 ~/.yaoxiang/yaoxiang.toml
3. 若都未配置，默认使用 "en"
4. 编译器根据选择的语言创建 I18nRegistry（一次）
5. 所有错误使用该 I18nRegistry 渲染消息
```

#### Key to Zero Lookup Overhead

**Rendering occurs when compiling the user's project, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 1: Rust 编译 YaoXiang 编译器                                      │
│                                                                           │
│  JSON 打包进编译器二进制                                                 │
│  目的：explain 指令能直接读取 i18n 数据                                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 2: YaoXiang 编译用户项目（渲染发生在这里）                          │
│                                                                           │
│  error! 宏调用时：                                                       │
│  1. 读取 yaoxiang.toml 获取语言偏好                                      │
│  2. 从编译器二进制加载对应语言的 i18n JSON                                │
│  3. 模板 + 参数 → render() → "Unknown variable: 'x'"                    │
│  4. Diagnostic.message = 已渲染的字符串                                   │
│                                                                           │
│  AOT 二进制直接存储最终字符串，无模板，无查表                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 3: 用户程序运行时                                                  │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Directly output the final string; no lookups are performed           │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                       | Rendering time                    |
| ---------------------------- | ------------------------------------ | --------------------------------- |
| `I18nRegistry`               | Provides templates and display text  | When compiling the user's project |
| `DiagnosticBuilder.render()` | Template + parameters → final string | When compiling the user's project |
| `Diagnostic.message`         | Rendered string                      | Stores the final result           |
| AOT binary                   | Contains final strings               | Used directly at runtime          |

---

### Error Message Format

Error messages use the following format:

```
error[E####]: <简短描述>
  --> <文件>:<行>:<列>
   <行> | <代码片段>
          ^^^<高亮>
```

#### Complete Example

```
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?
```

---

### Severity Levels

Error severity is managed through the `DiagnosticLevel` enum and is decoupled from error code
numbering:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation to fail
    Warning,  // Does not affect compilation, but a fix is recommended
    Note,     // Additional information
    Help,     // Suggested fix
}
```

| Level   | Prefix            | Description                 |
| ------- | ----------------- | --------------------------- |
| Error   | `error[E####]:`   | Causes compilation to fail  |
| Warning | `warning[E####]:` | Does not affect compilation |
| Note    | `note[E####]:`    | Additional information      |
| Help    | `help[E####]:`    | Suggested fix               |

---

### `yaoxiang explain` Command

#### Command Syntax

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Options

| Option          | Description                                        |
| --------------- | -------------------------------------------------- |
| `--lang <code>` | Specify the language (en-US, zh-CN; default en-US) |
| `--json`        | JSON-formatted output for IDE/LSP use              |
| `--json-pretty` | Formatted JSON output                              |
| `--examples`    | Show only example code                             |
| `--help`        | Show help information                              |

#### Usage Examples

```bash
# English by default
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# Chinese output
$ yaoxiang explain E1001 --lang zh
error[E1001]: 未知变量: {name}
  --> <file>:<line>:<col>

帮助: 你是否想要定义它？

示例:
  let {name} = value;

# JSON output (LSP integration)
$ yaoxiang explain E1001 --json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

#### JSON Output Format

```json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

---

### Backward Compatibility

Because this RFC designs the error code system from scratch, there are no backward compatibility
issues.

**Future migration strategy** (for reference by later versions):

1. Maintain a mapping from old error codes to new error codes.
2. Display both old and new codes during migration.
3. Provide a deprecation schedule.

---

## Implementation Strategy

### Stage One: Error Code Infrastructure

1. Create the `src/diagnostics/` directory structure.
2. Implement the `ErrorCode` enum.
3. Implement `Diagnostic` and `DiagnosticLevel`.
4. Create the resource file directory and sample JSON.

### Stage Two: explain Command

1. Implement the `yaoxiang explain` CLI command.
2. Support the `--lang` and `--json` options.
3. Integrate resource file loading.
4. Implement parameter template rendering.

### Stage Three: Compile-Time Integration

1. Update all error reporting points to use the new system.
2. Implement message template parameter injection.
3. Add language-priority logic.
4. Add unit-test coverage.

### Stage Four: IDE/LSP Integration

1. Integrate the explain JSON output into the LSP server.
2. Display error code links in IDEs.
3. Show error explanations on hover.
4. Provide quick-fix suggestions.

---

## Appendix

### Complete Error Code Quick Reference

| Range | Category                       |
| ----- | ------------------------------ |
| E0xxx | Lexical and syntactic analysis |
| E1xxx | Type checking                  |
| E2xxx | Semantic analysis              |
| E3xxx | Code generation                |
| E4xxx | Generics and traits            |
| E5xxx | Modules and imports            |
| E6xxx | Runtime errors                 |
| E7xxx | I/O and system errors          |
| E8xxx | Internal compiler errors       |
| E9xxx | Reserved                       |

### Supported Languages

| Code  | Language           | Status  |
| ----- | ------------------ | ------- |
| en-US | English (US)       | Default |
| zh-CN | Simplified Chinese | Planned |

### Error Message Example Comparison

```
# English (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# Chinese (zh-CN)
error[E1001]: 未知变量: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          帮助: 你是否想要定义它？
```

## References

- [Rust Compiler Error Index](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC Error Message Format](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang Diagnostic Format](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
