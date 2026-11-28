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

## Abstract

This RFC proposes an error code classification specification for the YaoXiang compiler. It uses a
single-level numbering system similar to Rust and supports multiple languages through JSON resource
files, providing error explanations via the `yaoxiang explain` command.

## Motivation

### Why is a standardized error code system needed?

1. **User experience**: Users can quickly determine the type and severity of an error from its error
   code
2. **Documentation organization**: Grouping by category simplifies writing and maintaining the error
   reference documentation
3. **Tool integration**: IDEs/LSPs can provide quick-fix suggestions and documentation links based
   on error codes
4. **Internationalization support**: Separating error messages from codes makes multilingual
   translation easier

### Design goals

- **Concise**: Single-level numbering means users do not need to memorize complex classification
  rules
- **Friendly**: Rust-like error message formatting, with help text and examples
- **Extensible**: Resource-file-driven design makes it easy to add new errors and languages
- **Tool-friendly**: The explain command and JSON output support IDE/LSP integration

---

## Proposal

### Core design: single-level numbering system

Use four-digit numeric codes grouped by compiler phase:

```
Exxxx
││││
│││└── 序号 (000-999)
││└─── 编译阶段 (0-9)
└───── 固定前缀 'E'
```

### Phase classification

| Phase | Range | Description                 |
| ----- | ----- | --------------------------- |
| **0** | E0xxx | Lexical and syntax analysis |
| **1** | E1xxx | Type checking               |
| **2** | E2xxx | Semantic analysis           |
| **3** | E3xxx | Code generation             |
| **4** | E4xxx | Generics and traits         |
| **5** | E5xxx | Modules and imports         |
| **6** | E6xxx | Runtime errors              |
| **7** | E7xxx | I/O and system errors       |
| **8** | E8xxx | Internal compiler errors    |
| **9** | E9xxx | Reserved/experimental       |

### Error category enum

```rust
/// Error categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Lexical and syntax analysis
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: Type checking
    Semantic,   // E2xxx: Semantic analysis
    Generic,    // E4xxx: Generics and traits
    Module,     // E5xxx: Modules and imports
    Runtime,    // E6xxx: Runtime errors
    Io,         // E7xxx: I/O and system errors
    Internal,   // E8xxx: Internal compiler errors
}
```

### Error code definitions and the generic Builder

**Core principle**: Separate error code definitions from presentation text.

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without presentation text
- `locales/*.json`: Presentation text for each language (title, message, help, with error codes as
  nested objects)
- `DiagnosticBuilder`: Generic builder that replaces the trait-per-error design

#### Error code definitions

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only; display text is in the i18n file)
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

    /// Add a template parameter
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// Set the location
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Build a Diagnostic (template rendering is completed at compile time)
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

#### Shortcut methods for each error code

```rust
// diagnostic/codes/e1xxx.rs

impl ErrorCodeDefinition {
    /// E1001 Unknown variable
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 Type mismatch
    pub fn type_mismatch(expected: &str, found: &str) -> DiagnosticBuilder {
        let def = Self::find("E1002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected)
            .param("found", found)
    }
}
```

#### Usage example

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

#### Example error code definitions

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

#### Design advantages

| Feature                   | Description                                                       |
| ------------------------- | ----------------------------------------------------------------- |
| **Single Builder**        | One generic `DiagnosticBuilder` serves all error codes            |
| **Type safety**           | Shortcut methods ensure that parameters are supplied correctly    |
| **Self-documenting**      | `E1001::unknown_variable(name)` is immediately understandable     |
| **Template separation**   | Message templates are separate from codes, facilitating i18n      |
| **Zero runtime overhead** | Rendering occurs at compile time; AOT binaries perform no lookups |

---

### Simplified error macros

#### The error! macro (automatic context injection)

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

/// Usage: only pass parameters; the span and i18n are injected automatically
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Using the Builder manually

```rust
// When manual control is needed
E1001::unknown_variable(&var_name)
    .at(my_span)           // Custom span
    .build(&custom_i18n)   // Custom i18n
```

---

## Detailed Design

### Error code list

#### E0xxx: Lexical and syntax analysis

<!-- code-table:E0xxx start -->

| Code  | Description                 |
| ----- | --------------------------- |
| E0001 | Invalid character           |
| E0002 | Invalid numeric literal     |
| E0003 | Unterminated string         |
| E0004 | Invalid character literal   |
| E0010 | Expected token              |
| E0011 | Unexpected token            |
| E0012 | Invalid syntax              |
| E0013 | Mismatched parentheses      |
| E0014 | Missing semicolon           |
| E0016 | Expected expression         |
| E0018 | Keyword used as a name      |
| E0019 | Expression nesting too deep |

<!-- code-table:E0xxx end -->

#### E1xxx: Type checking

<!-- code-table:E1xxx start -->

| Code  | Description                                                           |
| ----- | --------------------------------------------------------------------- |
| E1001 | Unknown variable                                                      |
| E1002 | Type mismatch                                                         |
| E1003 | Unknown type                                                          |
| E1010 | Argument count mismatch                                               |
| E1011 | Implementation parameter annotation does not match the declaration    |
| E1012 | Return type mismatch                                                  |
| E1013 | Function not found                                                    |
| E1014 | Unknown named argument                                                |
| E1015 | Duplicate argument specification                                      |
| E1020 | Unable to infer type                                                  |
| E1021 | Type inference conflict                                               |
| E1030 | Incomplete pattern                                                    |
| E1031 | Unreachable pattern                                                   |
| E1032 | Duplicate pattern binding                                             |
| E1033 | Inconsistent or-pattern bindings                                      |
| E1034 | Missing field in struct pattern                                       |
| E1040 | Operation not supported                                               |
| E1041 | Index out of bounds                                                   |
| E1042 | Field not found                                                       |
| E1043 | Module member not found                                               |
| E1050 | Boolean operand required                                              |
| E1051 | Logical NOT requires a Boolean operand                                |
| E1052 | Invalid dereference                                                   |
| E1053 | Access to a non-struct field                                          |
| E1054 | Conditional type mismatch                                             |
| E1055 | Constraint in a non-generic context                                   |
| E1060 | Type parameter count mismatch                                         |
| E1061 | Unable to instantiate generic                                         |
| E1062 | const generic constraint failed                                       |
| E1064 | Invalid binding position index                                        |
| E1065 | Call to a non-function value                                          |
| E1071 | Type definitions may only appear at module scope                      |
| E1081 | `?` is only allowed in functions returning a propagatable type        |
| E1082 | `?` can only be used with types implementing Try                      |
| E1083 | Error type of `?` does not match                                      |
| E1090 | ✨ Unspeakable ✨                                                     |
| E1091 | Invalid generic metatype                                              |
| E1092 | Illegal form of refined type argument                                 |
| E1093 | Mismatched number of refined arguments                                |
| E1094 | Unused compile-time value argument                                    |
| E1095 | Unknown interface                                                     |
| E1096 | Interface argument count mismatch                                     |
| E1097 | Interface member name conflict                                        |
| E1098 | Interface method not implemented                                      |
| E1099 | Interface method signature mismatch                                   |
| E1100 | Duplicate implementation of an interface method                       |
| E1101 | Type does not implement the interface                                 |
| E1102 | Loop control statement outside a loop                                 |
| E1103 | Brackets cannot be used in type position                              |
| E1104 | Interface implementation is not in the type's defining module         |
| E1105 | Variant constructor cannot be accessed as a field                     |
| E1106 | Constraint method dispatch failed                                     |
| E1107 | Ambiguous method overload                                             |
| E1108 | Empty block fell through to a container's expected slot               |
| E1109 | return outside a function                                             |
| E1110 | Method binding missing a signature annotation                         |
| E1111 | Argument occupies a slot more than once or is in an illegal order     |
| E1112 | Implementation parameter count exceeds the declaration                |
| E1113 | Compile-time value argument missing compile-time evidence             |
| E1114 | witness field cannot be read as a value                               |
| E1115 | witness-bearing record is immutable                                   |
| E1116 | Bare name conflicts with exports from multiple standard library units |

<!-- code-table:E1xxx end -->

> **RFC-011b association (note dated 2026-09-22)**:
> [RFC-011b: Operator Overloading](011b-operator-overloading.md) Implementation will involve three
> points in this section—① remove the wording "Result" from `E1081`/`E1082` (`?` will instead be
> determined through the `Try` interface and no longer tied to a specific type name), and
> synchronize them in phase 2 according to this document's "three-way consistency" process
> (codes/*.rs ↔ locales ↔ code table); ② rejection diagnostics for an unsatisfied `Equal`
> precondition (linear token) reuse the `E1101` (type does not implement the interface) family; ③
> after phase 1 is wired up, `Struct == Struct` changes from an `E6007` runtime error to a
> compile-time determination, reducing the `E6007` trigger surface. The registered wording in the
> table remains as-is until the implementation lands.

#### E2xxx: Semantic analysis

<!-- code-table:E2xxx start -->

| Code  | Description                            |
| ----- | -------------------------------------- |
| E2001 | Scope error                            |
| E2002 | Duplicate definition                   |
| E2003 | Ownership error                        |
| E2010 | Assignment to an immutable binding     |
| E2011 | Use of an uninitialized variable       |
| E2012 | Mutability conflict                    |
| E2013 | Variable shadowing                     |
| E2014 | Use of a moved value                   |
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
| E2095 | Parameter name shadows a generic       |
| E2096 | Signature uses a bare container type   |

<!-- code-table:E2xxx end -->

> Reserved code note (inventory dated 2026-09-14, release criteria for #251): E2019 (double free),
> E2020 (use after free), E2027 (unsafe dereference), and E2029 (reference cycle within spawn) have
> completed registration and are anchored by unit tests, but they have no reachable yx source
> surface (explicit drop statements, Ptr dereference grammar, or a path for constructing a spawn ref
> cycle). The claim of complete semantic correctness does not cover these four codes; they should be
> treated as "reserved" until implementation is completed.

#### E3xxx: Code generation

<!-- code-table:E3xxx start -->

| Code  | Description                                        |
| ----- | -------------------------------------------------- |
| E3004 | Unsupported iterator                               |
| E3005 | IR generation error                                |
| E3006 | Unresolved variable                                |
| E3007 | Top-level binding initialization must be constant  |
| E3008 | Unsupported match pattern                          |
| E3014 | Register overflow                                  |
| E3017 | Invalid operand (code generation)                  |
| E3018 | Monomorphization instantiation failed              |
| E3019 | Circular dependency among top-level bindings       |
| E3020 | Missing program entry point                        |
| E3021 | Entry point is not a function                      |
| E3022 | Entry point main signature mismatch                |
| E3023 | Executable statements are not allowed at top level |
| E3024 | Project configuration is unreadable                |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and traits

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
| E4012 | Constant recursion too deep                     |
| E4014 | Constant evaluation failed                      |
| E4018 | Refinement predicate violation                  |
| E4019 | Type equality does not hold                     |
| E4020 | Proof function required                         |
| E4021 | Loop termination cannot be proved automatically |
| E4022 | Measure does not hold                           |
| E4023 | Cannot be proved within the kernel              |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission points (reserved codes): the Sized constraint and
> optimization error paths are pending implementation; wire them to the actual trigger surfaces
> during implementation.

#### E4023: Generic "Unable to Prove"

> **Status: Registered (2026-11-13)**. This section records the code's meaning and division of
> responsibilities; the code is emitted through the generic `Unproven` path, and all four
> registration plumbing items were submitted in the same batch.

- **Code**: `E4023`
- **Category (face)**: `Generic` (E4xxx, generics and proof surface), in the same family as `E4021`
  and `E4022`.
- **Semantics**: A generic "unable to prove". It is emitted when an obligation point cannot
  construct a proof within the resource limit; it is not tied to a specific proposition form.
- **Template**: `Unable to prove: {proposition} ({reason})`
- **Parameters**: `proposition` (renderable text for the unresolved proposition), `reason`
  (capability limit / budget exhausted / contains unbound symbol names).
- **Location**: Non-exempt—the source location is provided by the emission point; a judgment point
  with no location uses the default location, with the reason documented in the implementation
  comments.
- **Division of responsibilities with E2031**: `E2031` is specific to refinement constraints, is
  emitted by the refinement channel, and carries an actual location; its message includes the
  assigned name, variable, and constraint. `E4023` is the generic fallback for `Unproven` at
  equality checks and other obligation points, carrying the proposition and reason. The two codes
  must not replace one another: a missing proof for a refinement constraint still reports E2031,
  while an undecidable equality check or other obligation reports E4023.
- **Boundaries with existing codes**: `E4019` (type equality does not hold) corresponds to
  `Disproved`; `E4023` corresponds to `Unproven`. `E4014` (constant evaluation failed) corresponds
  to a deterministic failure in closed-term evaluation and is not used for "not proved within the
  resource limit".
- **Registration plumbing (completed)**: the `define_codes!` entry in
  `src/util/diagnostic/codes/e4xxx.rs` → templates and parameters in the six `locales/` language
  files → the E4xxx code table in this document (regenerated by the error code documentation
  generator) → the generated page under `docs/src/reference/error-code/`. All four items were
  submitted in the same batch.

#### E5xxx: Modules and imports

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

#### E6xxx: Runtime errors

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

> **Code table revision (2026-08-09)**: The code table was originally defined according to a
> Rust-semantics draft (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast
> failed), which did not match actual implementation requirements. YaoXiang has no null-pointer,
> heap-allocation-failure, or type-conversion concepts (value semantics + Rust memory safety), and
> runtime overflow detection paths have not been implemented. After calibration:
>
> - E6002 was removed (the original Assertion failed semantics moved to E6005; the original
>   null-pointer semantics have no language concept)
> - E6003 was changed from Arithmetic overflow to Runtime index out of bounds (the actual trigger
>   surface)
> - E6005 was changed from Heap allocation failed to Assertion failed (the real std.assert path)
> - E6006 was changed from Runtime index out of bounds to Function not found (the implementation has
>   always behaved this way)
> - E6007 was changed from Type cast failed to a generic Runtime error (the common destination for
>   unmapped ExecutorError variants)

#### E7xxx: I/O and system errors

<!-- code-table:E7xxx start -->

| Code  | Description       |
| ----- | ----------------- |
| E7001 | File not found    |
| E7002 | Permission denied |
| E7003 | I/O error         |
| E7004 | Network error     |

<!-- code-table:E7xxx end -->

#### E8xxx: Internal compiler errors

<!-- code-table:E8xxx start -->

| Code  | Description             |
| ----- | ----------------------- |
| E8001 | Internal compiler error |
| E8002 | Unexpected panic        |
| E8003 | Compiler phase error    |

<!-- code-table:E8xxx end -->

#### W1xxx: Warning codes

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
| W1081 | Termination obligation not decided           |
| W1082 | Ownership analysis degraded                  |
| W1090 | Obligation produced but has no consumer      |

<!-- code-table:W1xxx end -->

> W-code allocation rules: structurally identical to E codes and grouped by phase (W +
> phase-thousand segment); W1xxx = warnings in the type-checking phase.
>
> **Dead-code semantics (RFC-029f role semantics + RFC-029g)**: W1001/W1002/W1004/W1005 report
> **unreferenced top-level bindings and type definitions**—they are reported when they are never
> referenced in the within-file reachability analysis starting from `main`; methods (W1005) are
> matched by short name at call sites. The only exemption is the **package-internal reference pool**
> (RFC-029f: a name referenced by any file in the project is considered live—cross-file consumers
> are not visible in this file, favoring underreporting).
>
> **Implementation correction (2026-10-08, RFC-029g)**: This section originally stated that "`pub`
> definitions are public interfaces and are never reported (#321, final decision B)." The `pub`
> keyword has since been removed, and modifiers no longer provide any exemption; the original
> role-specific exemption policy (Bin may report immediately / Internal tightens in phase two / Lib
> is always exempt) has converged on a single condition: whether a package-internal reference pool
> is provided. For behavior changes, see [029g](029g-remove-pub-and-auto-bind.md).
>
> **Unused imports (W1003)**: Detected by typecheck's use elaboration (pass2 registers imported
> local names; a hit in expression resolution or a type annotation position counts as use), covering
> both whole-module imports (`use std.io` → module alias) and named imports (`use std.io.{print}`).
>
> **Emission channel**: W-code diagnostics are marked `Severity::Warning` by default by the builder
> according to the W prefix (an explicit value takes precedence). Collection and presentation follow
> the same path as errors (`warning[W####]` prefix rendering), but they do not block compilation or
> affect the successful exit code. `yaoxiang check --deny-warnings` promotes warnings to failures (a
> non-zero exit code when warnings exist), for use in strict CI modes. Per-code suppression (such as
> allow attributes) is a future extension.
>
> **Decoupling code families from emission severity (W1090 promoted to E, user decision on
> 2026-10-13)**: **The code prefix determines only numbering membership, not emission
> severity**—severity is determined by the explicit `severity` at the diagnostic construction site
> (the `E` prefix defaults to Error and the `W` prefix defaults to Warning, with explicit values
> taking precedence). Accordingly, `W1090` (obligation produced but has no consumer) retains its
> **W-family code number** (no new E code is introduced: two codes for the same fact provide
> parallel representations), but from WBS 4.3.2 onward its **emission severity is error**: it is
> counted in `error_count`, rendered as `error[W1090]`, and changes the exit code (RFC-039 D5's
> **terminal state**; it belongs to D59's class D breaking changes and has been truthfully recorded
> in 02-stage-contract §2). A healthy program produces no emissions (the report contains no warning
> when all obligations have delivery markers), so this promotion affects build results only when a
> phase-contract defect actually occurs.

### Message quality specification

> This section was introduced by the message pipeline and quality revision (2026-09-03). It is
> enforced in CI by `scripts/audit_diagnostics.py`.

1. **Single message pipeline**: All user-visible diagnostic messages must be rendered through
   shortcut methods in the authoritative registry and locale templates; code must pass only
   structured parameters. Directly constructing native values such as `Diagnostic::error(...)` is
   forbidden because this path bypasses code validation and i18n.
2. **Code validity**: Unregistered and pseudo-codes such as `E_INTERNAL` must not be used; every
   code literal used at a call site must already be defined in the registry. All internal errors
   must use E8001 (`internal_error`).
3. **Type display**: A type's Display implementation must distinguish its forms before and after
   instantiation (`Expected 'Container', found 'Container'` cannot distinguish bare names).
4. **Solver internal-state isolation**: Intermediate solver states such as TypeVar (with the Display
   form `t<N>`) must not appear in user-visible messages. Test anchor:
   `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is reserved exclusively for internal compiler consistency issues
   (ICEs). User-fixable errors must not use E8001 as a fallback; ICE messages must include minimal
   reproduction guidance.

---

### Runtime error values and code interoperability

> This section was introduced by the runtime Error value code revision (2026-09-03). The E6xxx/E7xxx
> semantic space carries two channels simultaneously, with the same code space and different
> presentation channels.

#### The two channels

| Channel                              | Carrier                                                       | Presentation                                                         |
| ------------------------------------ | ------------------------------------------------------------- | -------------------------------------------------------------------- |
| Compiler/CLI diagnostic channel      | Host-level hard errors such as `ExecutorError`                | stderr `error[E####]:` (E6003/E6005/E6007 already wired)             |
| Program-internal error value channel | The `Error` carried by the Err side of std `Result(T, Error)` | A language value consumed by the program through matching/comparison |

#### Error structure (since v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbering from this specification, in string form (such as
  `"E6008"`).
- **Stable contract**: The semantics of an allocated code do not change across versions; a deleted
  code is never reused for the same semantics (the E6002 precedent).
- **Consumption surface**: Program-internal `e.code == "E6xxx"` comparison is the only programmable
  decision contract; `yaoxiang explain E6xxx` provides interoperable documentation; toolchains (LSP
  / DAP, see RFC-034) use the code as the exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: The E in `Result(T, E)` is a generic parameter. Careful modeling should
  use a user-defined type; std `Error` is only a convenient fallback carrier, whose code system does
  not constrain user-defined E types.

#### Code allocation rules

1. Runtime error value codes and compiler diagnostic codes share the E6xxx/E7xxx space. New codes
   are allocated according to **actual trigger surfaces**, not hypothetical scenarios.
2. Register before use: A new code must enter the authoritative registry and pass three-way
   consistency validation (codes/*.rs ↔ locales ↔ the code table in this document) before it can be
   emitted. The registration source for runtime error value codes is the `RUNTIME_ERROR_CODES` table
   in `src/std/result.rs` (subject to the same build-time gate in `build.rs` and validation by
   `tools/code-tables` as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error values (currently unused; it will be activated when
   io/net become Result-based).
4. Emission points: std modules construct Error values through `error_new(code, message)`; the
   consumer side extracts the Err carrier through `std.result.unwrap_err` and reads the fields
   through `std.result.code/message`.

#### Evolution path (Line C, not implemented)

Once exhaustive pattern matching (RFC-010b) is implemented, `Error` may be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming a property derived from `kind` (the
variant definition becomes the code registry). During the transition, the stable code contract in
this section remains unchanged; the upgrade is an independent decision and does not constitute a
commitment made by this section.

---

### Multilingual resource files

#### Resource file format

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

#### I18nRegistry implementation

```rust
// locales/*.json（错误码对象）

/// i18n display-text registry (loaded from JSON at compile time, zero lookups at runtime)
pub struct I18nRegistry {
    /// Title
    titles: HashMap<&'static str, &'static str>,
    /// Description
    messages: HashMap<&'static str, &'static str>,
    /// Help information
    helps: HashMap<&'static str, &'static str>,
    /// Example code
    examples: HashMap<&'static str, &'static str>,
    /// Error output example
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
    /// Obtain the registry from a language code
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// Obtain error information
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// Render a template (completed at compile time, with zero runtime overhead)
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

#### Template placeholders

##### Predefined placeholders (common)

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

##### Support for arbitrary keys

**params supports arbitrary keys and is not limited to the predefined ones**. Callers may pass any
`key`:

```rust
// Use any key
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

#### Language precedence

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. 默认值: en
```

### yaoxiang.toml configuration

#### Project-level configuration

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# Error message language; options: en, zh, ja, ...
default = "zh"
```

#### User-level configuration

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### Compile-time language selection

```
1. 读取项目级 yaoxiang.toml 的 language.default
2. 若未配置，读取用户级 ~/.yaoxiang/yaoxiang.toml
3. 若都未配置，默认使用 "en"
4. 编译器根据选择的语言创建 I18nRegistry（一次）
5. 所有错误使用该 I18nRegistry 渲染消息
```

#### Key to zero lookup overhead

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
│  // 直接输出最终字符串，无任何查表                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                       | Rendering time                    |
| ---------------------------- | ------------------------------------ | --------------------------------- |
| `I18nRegistry`               | Provides templates and display text  | When compiling the user's project |
| `DiagnosticBuilder.render()` | Template + parameters → final string | When compiling the user's project |
| `Diagnostic.message`         | Rendered string                      | Stores the final result           |
| AOT binary                   | Contains the final string            | Used directly at runtime          |

---

### Error message format

Error messages use the following format:

```
error[E####]: <简短描述>
  --> <文件>:<行>:<列>
   <行> | <代码片段>
          ^^^<高亮>
```

#### Complete example

```
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?
```

---

### Severity levels

Error severity is managed through the `DiagnosticLevel` enum and is decoupled from error code
numbering:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation to fail
    Warning,  // Does not affect compilation, but suggests a fix
    Note,     // Supplementary information
    Help,     // Suggested fix
}
```

| Level   | Prefix            | Description                 |
| ------- | ----------------- | --------------------------- |
| Error   | `error[E####]:`   | Causes compilation to fail  |
| Warning | `warning[E####]:` | Does not affect compilation |
| Note    | `note[E####]:`    | Supplementary information   |
| Help    | `help[E####]:`    | Suggested fix               |

---

### The `yaoxiang explain` command

#### Command syntax

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Options

| Option          | Description                                        |
| --------------- | -------------------------------------------------- |
| `--lang <code>` | Specify the language (en-US, zh-CN, default en-US) |
| `--json`        | JSON output (for IDE/LSP use)                      |
| `--json-pretty` | Formatted JSON output                              |
| `--examples`    | Show only example code                             |
| `--help`        | Show help information                              |

#### Usage example

```bash
# Default English
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

#### JSON output format

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

### Backward compatibility

Because this RFC designs the error code system from scratch, there are no backward compatibility
concerns.

**Future migration strategy** (for reference by subsequent versions):

1. Maintain a mapping from old error codes to new error codes
2. Display both old and new codes during the migration period
3. Provide a deprecation timeline

---

## Implementation Strategy

### Phase one: Error code infrastructure

1. Create the `src/diagnostics/` directory structure
2. Implement the `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create the resource file directory and example JSON

### Phase two: The explain command

1. Implement the `yaoxiang explain` CLI command
2. Support the `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase three: Compile-time integration

1. Update all error reporting sites to use the new system
2. Implement message template parameter injection
3. Add language precedence logic
4. Add unit test coverage

### Phase four: IDE/LSP integration

1. Integrate the explain JSON output into the LSP server
2. Display error code links in IDEs
3. Display error explanations on hover
4. Provide quick-fix suggestions

---

## Appendix

### Complete error code quick reference

| Range | Category                    |
| ----- | --------------------------- |
| E0xxx | Lexical and syntax analysis |
| E1xxx | Type checking               |
| E2xxx | Semantic analysis           |
| E3xxx | Code generation             |
| E4xxx | Generics and traits         |
| E5xxx | Modules and imports         |
| E6xxx | Runtime errors              |
| E7xxx | I/O and system errors       |
| E8xxx | Internal compiler errors    |
| E9xxx | Reserved                    |

### Supported languages

| Code  | Language           | Status  |
| ----- | ------------------ | ------- |
| en-US | English (US)       | Default |
| zh-CN | Simplified Chinese | Planned |

### Error message example comparison

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
