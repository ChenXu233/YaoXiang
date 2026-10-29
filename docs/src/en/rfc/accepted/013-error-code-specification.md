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

This RFC proposes an error-code classification specification for the YaoXiang compiler. It uses a
Rust-like single-level numbering system and JSON resource files to support multiple languages,
providing error explanations through the `yaoxiang explain` command.

## Motivation

### Why is a standardized error code needed?

1. **User experience**: Users can quickly determine the type and severity of an error when they see
   its error code
2. **Documentation organization**: Grouping codes by category makes error-reference documentation
   easier to write and maintain
3. **Tool integration**: IDEs/LSPs can provide quick-fix suggestions and documentation links based
   on error codes
4. **Internationalization**: Separating error messages from codes makes messages easier to translate
   into multiple languages

### Design goals

- **Concise**: Single-level numbering means users do not need to memorize complex classification
  rules
- **Friendly**: Rust-like error message format with help text and examples
- **Extensible**: Resource-file-driven design makes it easy to add new errors and languages
- **Tool-friendly**: The `explain` command and JSON output support IDE/LSP integration

---

## Proposal

### Core design: Single-level numbering system

Four-digit numeric identifiers are used and grouped by compiler stage:

```
Exxxx
││││
│││└── 序号 (000-999)
││└─── 编译阶段 (0-9)
└───── 固定前缀 'E'
```

### Stage classification

| Stage | Range | Description                    |
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

### Error category enum

```rust
/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Lexical and syntactic analysis
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

### Error code definitions and common Builder

**Core principle**: Separate error-code definitions from display text

- `ErrorCodeDefinition`: Error-code metadata (code, category, template), without display text
- `locales/*.json`: Display text for each locale (title, message, help; error codes are nested
  objects)
- `DiagnosticBuilder`: A common builder that replaces the trait-per-error design

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

/// Common diagnostic builder
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
        // Check that all {key} entries in the template have corresponding parameters
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

#### Convenience method for each error code

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

| Feature                   | Description                                                  |
| ------------------------- | ------------------------------------------------------------ |
| **Single Builder**        | One `DiagnosticBuilder` serves all error codes               |
| **Type safety**           | Convenience methods ensure parameter correctness             |
| **Self-documenting**      | `E1001::unknown_variable(name)` is immediately clear         |
| **Template separation**   | Message templates are separated from codes, making i18n easy |
| **Zero runtime overhead** | Compile-time rendering; no lookup table in the AOT binary    |

---

### Error macro simplification

#### `error!` macro (automatic context injection)

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

#### Manual use of the Builder

```rust
// When manual control is needed
E1001::unknown_variable(&var_name)
    .at(my_span)           // Custom span
    .build(&custom_i18n)   // Custom i18n
```

---

## Detailed design

### Error code list

#### E0xxx: Lexical and syntactic analysis

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

#### E1xxx: Type checking

<!-- code-table:E1xxx start -->

| Code  | Description                                                                         |
| ----- | ----------------------------------------------------------------------------------- |
| E1001 | Unknown variable                                                                    |
| E1002 | Type mismatch                                                                       |
| E1003 | Unknown type                                                                        |
| E1010 | Argument count mismatch                                                             |
| E1011 | Argument type mismatch                                                              |
| E1012 | Return type mismatch                                                                |
| E1013 | Function not found                                                                  |
| E1014 | Unknown named argument                                                              |
| E1015 | Duplicate argument specified                                                        |
| E1020 | Unable to infer type                                                                |
| E1021 | Type inference conflict                                                             |
| E1030 | Non-exhaustive pattern                                                              |
| E1031 | Unreachable pattern                                                                 |
| E1032 | Duplicate pattern binding                                                           |
| E1033 | Inconsistent or-pattern bindings                                                    |
| E1034 | Missing field in struct pattern                                                     |
| E1040 | Operation not supported                                                             |
| E1041 | Index out of bounds                                                                 |
| E1042 | Field not found                                                                     |
| E1043 | Module member not found                                                             |
| E1050 | Boolean operand required                                                            |
| E1051 | Logical NOT requires a boolean operand                                              |
| E1052 | Invalid dereference                                                                 |
| E1053 | Field access on a non-struct type                                                   |
| E1054 | Conditional type mismatch                                                           |
| E1055 | Constraint in a non-generic context                                                 |
| E1060 | Type argument count mismatch                                                        |
| E1061 | Unable to instantiate a generic                                                     |
| E1062 | const generic constraint failure                                                    |
| E1064 | Invalid binding-site index                                                          |
| E1065 | Call of a non-function value                                                        |
| E1071 | Type definitions are only allowed at module scope                                   |
| E1081 | `?` may only be used in a function returning a type that supports error propagation |
| E1082 | `?` may only be used with types that implement Try                                  |
| E1083 | The error type of `?` does not match                                                |
| E1090 | ✨ Unspeakable ✨                                                                   |
| E1091 | Invalid generic metatype                                                            |
| E1092 | Illegal shape of refined type argument                                              |
| E1093 | Refined argument count mismatch                                                     |
| E1094 | Unused compile-time value parameter                                                 |
| E1095 | Unknown interface                                                                   |
| E1096 | Interface argument count mismatch                                                   |
| E1097 | Conflicting interface member names                                                  |
| E1098 | Unimplemented interface method                                                      |
| E1099 | Interface method signature mismatch                                                 |
| E1100 | Duplicate implementation of an interface method                                     |
| E1101 | Type does not implement the interface                                               |
| E1102 | Loop control statement outside a loop                                               |
| E1103 | Brackets cannot be used in a type position                                          |
| E1104 | The interface implementation is not in the module where the type is defined         |
| E1105 | A variant constructor cannot be accessed as a field                                 |
| E1106 | Constraint not satisfied                                                            |
| E1107 | Ambiguous method overload                                                           |
| E1108 | An empty block falls into the container's expected slot                             |
| E1109 | `return` outside a function                                                         |
| E1110 | Method binding lacks a signature annotation                                         |
| E1111 | An argument is specified more than once or in an invalid order                      |

<!-- code-table:E1xxx end -->

> **RFC-011b association (note dated 2026-09-22)**:
> [RFC-011b: Operator Overloading](011b-operator-overloading.md) Implementation will affect three
> items in this section—① the wording of `E1081` / `E1082` will be changed to remove "Result" (the
> behavior of `?` will instead be determined by the `Try` interface, and will no longer be tied to a
> specific type name), and this will be synchronized during stage 2 according to the "three-way
> consistency" process in this document (codes/*.rs ↔ locales ↔ code tables); ② the rejection
> diagnostic for an unsatisfied `Equal` precondition constraint (linear token) will reuse the
> `E1101` (type does not implement the interface) family; ③ after stage 1 is wired up,
> `Struct == Struct` will change from an `E6007` runtime error to a compile-time determination,
> reducing the set of triggers for `E6007`. The registered wording in the table will remain
> unchanged until the implementation lands.

#### E2xxx: Semantic analysis

<!-- code-table:E2xxx start -->

| Code  | Description                              |
| ----- | ---------------------------------------- |
| E2001 | Scope error                              |
| E2002 | Duplicate definition                     |
| E2003 | Ownership error                          |
| E2010 | Assignment to an immutable value         |
| E2011 | Use of an uninitialized variable         |
| E2012 | Mutability conflict                      |
| E2013 | Variable shadowing                       |
| E2014 | Use of a moved value                     |
| E2016 | Assignment to an immutable value         |
| E2018 | Mutable/immutable borrow conflict        |
| E2019 | Double free                              |
| E2020 | Use after free                           |
| E2027 | unsafe dereference                       |
| E2029 | Reference cycle within spawn             |
| E2030 | Refinement type constraint violation     |
| E2031 | Refinement constraint cannot be proven   |
| E2090 | Invalid signature                        |
| E2091 | Unknown type in signature                |
| E2092 | Signature missing an arrow               |
| E2093 | Duplicate parameter name                 |
| E2094 | Generic parameter shadowing              |
| E2095 | Parameter name shadows a generic         |
| E2096 | Signature contains a bare container type |

<!-- code-table:E2xxx end -->

> Reserved code note (inventory dated 2026-09-14, release scope in #251): E2019 (double free), E2020
> (use after free), E2027 (unsafe dereference), and E2029 (ref cycle within spawn) have completed
> registration and are anchored by unit tests, but currently have no reachable yx source-level
> surface (explicit drop statements, Ptr dereference grammar, or a spawn ref cycle construction
> path). The claim of complete semantic correctness does not cover these four codes; treat them as
> "reserved" until implementation is completed.

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
| E3018 | Monomorphization instantiation failure             |
| E3019 | Top-level binding cycle                            |
| E3020 | Missing program entry point                        |
| E3021 | Entry point is not a function                      |
| E3022 | Entry point `main` signature mismatch              |
| E3023 | Executable statements are not allowed at top level |
| E3024 | Project manifest is unreadable                     |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and traits

<!-- code-table:E4xxx start -->

| Code  | Description                                     |
| ----- | ----------------------------------------------- |
| E4001 | Generic constraint violation                    |
| E4002 | Trait not found                                 |
| E4003 | Missing trait implementation                    |
| E4004 | Conflicting trait implementations               |
| E4005 | Associated type not found                       |
| E4010 | Constant division by zero                       |
| E4011 | Constant overflow                               |
| E4012 | Constant recursion is too deep                  |
| E4014 | Constant evaluation failed                      |
| E4018 | Refinement predicate violation                  |
| E4019 | Type equality does not hold                     |
| E4020 | Proof function required                         |
| E4021 | Loop termination cannot be automatically proved |
| E4022 | Measure does not hold                           |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission points (reserved codes): the Sized constraint and
> optimization error path are pending implementation and will be wired to the actual trigger surface
> when implemented.

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

> **Code table revision (2026-08-09)**: The code table was originally defined from a Rust-semantics
> draft (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed), which did
> not match the implementation's actual requirements. YaoXiang has no
> null-pointer/heap-allocation-failure/type-conversion concepts (value semantics + Rust memory
> safety), and detection has not been implemented for runtime overflow paths. After calibration:
>
> - E6002 was removed (the original Assertion failed was moved to E6005; the original null-pointer
>   semantics have no corresponding language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (the actual trigger
>   surface)
> - E6005 changed from Heap allocation failed to Assertion failed (the real std.assert path)
> - E6006 changed from Runtime index out of bounds to Function not found (the implementation has
>   always behaved this way)
> - E6007 changed from Type cast failed to the generic Runtime error (the common landing point for
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
| E8002 | Unexpected Panic        |
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
| W1006 | Local module shadows dependency package      |
| W1007 | Module import cycle                          |
| W1063 | const generic constraint cannot be evaluated |
| W1080 | Compile-time proof downgrade                 |
| W1081 | Termination obligation not determined        |
| W1082 | Ownership analysis downgrade                 |
| W1090 | Obligation produced but has no consumer      |

<!-- code-table:W1xxx end -->

> W code-numbering rules: W codes have the same structure as E codes and are grouped by stage (W +
> stage-thousands segment); W1xxx = warnings in the type-checking stage.
>
> **Dead-code semantics (RFC-029f role semantics + RFC-029g)**: W1001/W1002/W1004/W1005 report
> **top-level bindings and type definitions that are never referenced**—they are reported if they
> are never referenced by the file-local reachability analysis starting from `main`; methods (W1005)
> are matched by call-site short name. The only exemption is the **package-internal reference pool**
> (RFC-029f: a name referenced by any file in the project is considered live—a cross-file consumer
> is not visible in this file, so the policy favors underreporting).
>
> **Implementation correction (2026-10-08, RFC-029g)**: The original text stated that "`pub`
> definitions are public interfaces and are never reported (#321, Final Decision B)." The `pub`
> keyword has since been removed, and modifiers no longer create any exemption; the previous
> role-based exemption policy (Bin: reportable immediately / Internal: tightened in stage two / Lib:
> absolute exemption) has been consolidated into the single condition of whether a package-internal
> reference pool is provided. See [029g](029g-remove-pub-and-auto-bind.md) for behavioral changes.
>
> **Unused imports (W1003)**: Detected by typecheck's use elaboration (pass2 registers the imported
> local name; a hit during expression resolution or at a type-annotation position counts as used).
> This covers both module imports (`use std.io` → module alias) and named imports
> (`use std.io.{print}`).
>
> **Emission channel**: W-code diagnostics are marked with `Severity::Warning` by default by the
> builder based on the W prefix (an explicit value takes precedence). They are collected and
> presented through the same pipeline as errors (`warning[W####]` prefix), but do not block
> compilation or affect a successful exit code. `yaoxiang check --deny-warnings` escalates warnings
> to failures (exiting with a nonzero code when warnings exist), for use in strict CI modes.
> Per-code suppression (such as allow attributes) is a future extension.
>
> **Decoupling code families from emission levels (W1090 promoted to the same code with an E level,
> user decision on 2026-10-13)**: The code-number prefix determines only the numbering family, not
> the emission level—the level is determined by the explicit `severity` at the diagnostic
> construction site (`E` defaults to Error, `W` defaults to Warning, and an explicit value takes
> precedence). Accordingly, `W1090` (obligation produced but no consumer) retains its **W-family
> code number** (no new E code is introduced: two codes for the same fact provide parallel
> representations), but since WBS 4.3.2 its **emission level is error**: a trigger counts toward
> `error_count`, is rendered as `error[W1090]`, and changes the exit code (the **final state** of
> RFC-039 D5; a Class D breaking change under D59, accurately recorded in 02-stage-contract §2).
> Healthy programs emit nothing (all obligations have delivery markers), so this promotion affects
> the build only when a stage-contract defect actually occurs.

### Message quality specification

> This section was introduced by the single-track message and quality revision (2026-09-03). It is
> enforced in CI by `scripts/audit_diagnostics.py`.

1. **Single-track messages**: All user-visible diagnostic messages must be rendered through
   convenience methods in the authoritative registry and locale templates; code may pass only
   structured parameters. Directly constructing native values such as `Diagnostic::error(...)` is
   forbidden—this path bypasses code validation and i18n.
2. **Code validity**: Unregistered codes and pseudo-codes such as `E_INTERNAL` are forbidden; every
   code literal used at an emission site must already be defined in the registry. Internal errors
   must always use E8001 (`internal_error`).
3. **Type display**: A type's Display implementation must distinguish forms before and after
   instantiation (`Expected 'Container', found 'Container'` cannot be distinguished by bare names).
4. **Solver internal-state isolation**: Intermediate solver states such as TypeVar (Display form
   `t<N>`) must not appear in user-visible messages. Test anchor:
   `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is reserved for internal compiler consistency problems (ICEs). E8001
   must not be used as a fallback for user-fixable errors; ICE messages must include instructions
   for producing a minimal reproduction.

---

### Runtime error values and code integration

> This section was introduced by the runtime Error value-with-code revision (2026-09-03). The
> E6xxx/E7xxx semantic space carries two channels simultaneously: they share the same code space but
> use different presentation channels.

#### Two channels

| Channel                         | Carrier                                                         | Presentation                                                    |
| ------------------------------- | --------------------------------------------------------------- | --------------------------------------------------------------- |
| Compiler/CLI diagnostic channel | Host-layer fatal errors such as `ExecutorError`                 | stderr `error[E####]:` (E6003/E6005/E6007 already wired)        |
| In-program error value channel  | The `Error` Err carrier in the std library's `Result(T, Error)` | A language value consumed by matching/comparison in the program |

#### Error structure (since v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbering from this specification in string form (for example,
  `"E6008"`).
- **Stable contract**: The semantics of an assigned code do not change across versions; a deleted
  code is never reused for the same semantics (the E6002 precedent).
- **Consumption surface**: Comparing `e.code == "E6xxx"` within a program is the only programmable
  determination contract; `yaoxiang explain E6xxx` aligns with the documentation; toolchains (LSP /
  DAP, see RFC-034) use the code as the exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: E in `Result(T, E)` is a generic parameter. Serious error modeling should
  use a user-defined type; std `Error` is merely a convenient fallback carrier, whose code system
  does not constrain user E types.

#### Code allocation rules

1. Runtime error value codes and compiler diagnostic codes share the E6xxx/E7xxx space. New codes
   are allocated according to their **actual trigger surface**, without reserving codes for
   hypothetical scenarios.
2. Register before use: A new code must be added to the authoritative registry and pass three-way
   consistency validation (codes/*.rs ↔ locales ↔ the code tables in this document) before it can be
   emitted. The registration source for runtime error value codes is the `RUNTIME_ERROR_CODES` table
   in `src/std/result.rs` (subject to the same `build.rs` build-time gate and `tools/code-tables`
   validation as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error values (currently empty and unused; it will be
   enabled when the io/net Result APIs are implemented).
4. Emission points: Each std module constructs an Error value using `error_new(code, message)`. On
   the consumer side, `std.result.unwrap_err` extracts the Err carrier, and
   `std.result.code/message` read its fields.

#### Evolution path (Line C, not implemented)

After exhaustive pattern matching (RFC-010b) is implemented, `Error` may be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming a property derived from the kind (the
code registry would then be located at the variant definition). The stable code contract in this
section remains unchanged during the transition; this upgrade is an independent decision and does
not constitute a commitment in this section.

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

#### `I18nRegistry` implementation

```rust
// locales/*.json（错误码对象）

/// i18n 展示文案注册表（编译期从 JSON 加载，运行时零查表）
pub struct I18nRegistry {
    /// 标题
    titles: HashMap<&'static str, &'static str>,
    /// 描述
    messages: HashMap<&'static str, &'static str>,
    /// 帮助信息
    helps: HashMap<&'static str, &'static str>,
    /// 示例代码
    examples: HashMap<&'static str, &'static str>,
    /// 错误输出示例
    error_outputs: HashMap<&'static str, &'static str>,
}

/// 单个错误码信息
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// 根据语言代码获取注册表
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// 获取错误信息
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// 渲染模板（编译期完成，运行时零开销）
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

**params supports arbitrary keys and is not limited to the predefined ones.** Callers may pass any
`key`:

```rust
// 使用任意 key
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// 模板定义
"Unknown variable: '{name}' at {location}. {hint}"
```

> **Note**: Not every error code uses placeholders. Some error codes, such as E0001, use static
> messages and require no parameters.

#### Language priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. 默认值: en
```

### `yaoxiang.toml` configuration

#### Project-level configuration

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# 错误消息语言，可选：en, zh, ja, ...
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

**Rendering occurs while compiling the user's project, not at runtime.**

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
│  // Print the final string directly, with no lookups                      │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                       | Rendering time                     |
| ---------------------------- | ------------------------------------ | ---------------------------------- |
| `I18nRegistry`               | Provide templates and display text   | While compiling the user's project |
| `DiagnosticBuilder.render()` | Template + parameters → final string | While compiling the user's project |
| `Diagnostic.message`         | Rendered string                      | Stores the final result            |
| AOT binary                   | Contains the final string            | Used directly at runtime           |

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

Error severity is managed through the `DiagnosticLevel` enum and is decoupled from error-code
numbering:

```rust
pub enum DiagnosticLevel {
    Error,    // 导致编译失败
    Warning,  // 不影响编译，但建议修复
    Note,     // 补充信息
    Help,     // 修复建议
}
```

| Level   | Prefix            | Description                 |
| ------- | ----------------- | --------------------------- |
| Error   | `error[E####]:`   | Causes compilation to fail  |
| Warning | `warning[E####]:` | Does not affect compilation |
| Note    | `note[E####]:`    | Supplementary information   |
| Help    | `help[E####]:`    | Suggested fix               |

---

### `yaoxiang explain` command

#### Command syntax

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Options

| Option          | Description                                         |
| --------------- | --------------------------------------------------- |
| `--lang <code>` | Specify the language (en-US, zh-CN; default: en-US) |
| `--json`        | JSON output (for IDE/LSP use)                       |
| `--json-pretty` | Formatted JSON output                               |
| `--examples`    | Show only example code                              |
| `--help`        | Show help information                               |

#### Usage examples

```bash
# 默认英文
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# 中文输出
$ yaoxiang explain E1001 --lang zh
error[E1001]: 未知变量: {name}
  --> <file>:<line>:<col>

帮助: 你是否想要定义它？

示例:
  let {name} = value;

# JSON 输出（LSP 集成）
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

Because this RFC designs the error-code system from scratch, there are no backward-compatibility
issues.

**Future migration strategy** (for reference in later versions):

1. Maintain a mapping from old error codes to new error codes
2. Display both old and new codes during the migration period
3. Provide a deprecation timeline

---

## Implementation strategy

### Phase one: Error-code infrastructure

1. Create the `src/diagnostics/` directory structure
2. Implement the `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create the resource-file directory and example JSON

### Phase two: `explain` command

1. Implement the `yaoxiang explain` CLI command
2. Support the `--lang` and `--json` options
3. Integrate resource-file loading
4. Implement template parameter rendering

### Phase three: Compile-time integration

1. Update all error-reporting sites to use the new system
2. Implement message-template parameter injection
3. Add language-priority logic
4. Add unit-test coverage

### Phase four: IDE/LSP integration

1. Integrate the `explain` JSON output into the LSP server
2. Display error-code links in the IDE
3. Show error explanations on hover
4. Provide quick-fix suggestions

---

## Appendix

### Complete error code quick reference

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
