---
title: 'RFC 013: Error Code Specification'
status: 'Accepted'
author: '晨煦'
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

This RFC proposes an error code classification specification for the YaoXiang compiler, adopting a
single-layer numbering system similar to Rust, paired with JSON resource files to enable
multilingual support, and providing an error explanation feature through the `yaoxiang explain`
command.

## Motivation

### Why a Standardized Error Code System?

1. **User experience**: Users can quickly judge the error type and severity by looking at the error
   code
2. **Documentation organization**: Grouping by category makes it easier to write and maintain error
   reference documentation
3. **Tooling integration**: IDE/LSP can provide quick-fix suggestions and documentation links based
   on error codes
4. **Internationalization support**: Separating error messages from codes makes multilingual
   translation easier

### Design Goals

- **Concise**: Single-layer numbering, users don't need to memorize complex classification rules
- **Friendly**: Rust-like error message format, with help information and examples
- **Extensible**: Resource-file driven, easy to add new errors and new languages
- **Tooling-friendly**: `explain` command + JSON output, supports IDE/LSP integration

---

## Proposal

### Core Design: Single-Layer Numbering System

A four-digit numbering scheme, grouped by compilation phase:

```
Exxxx
││││
│││└── Sequence number (000-999)
││└─── Compilation phase (0-9)
└───── Fixed prefix 'E'
```

### Phase Division

| Phase | Range | Description                    |
| ----- | ----- | ------------------------------ |
| **0** | E0xxx | Lexical and syntactic analysis |
| **1** | E1xxx | Type checking                  |
| **2** | E2xxx | Semantic analysis              |
| **3** | E3xxx | Code generation                |
| **4** | E4xxx | Generics and traits            |
| **5** | E5xxx | Module and import              |
| **6** | E6xxx | Runtime errors                 |
| **7** | E7xxx | I/O and system errors          |
| **8** | E8xxx | Internal compiler errors       |
| **9** | E9xxx | Reserved/Experimental          |

### Error Category Enum

```rust
/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Lexical and syntactic analysis
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: Type checking
    Semantic,   // E2xxx: Semantic analysis
    Generic,    // E4xxx: Generics and traits
    Module,     // E5xxx: Module and import
    Runtime,    // E6xxx: Runtime errors
    Io,         // E7xxx: I/O and system errors
    Internal,   // E8xxx: Internal compiler errors
}
```

### Error Code Definitions and the Generic Builder

**Core principle**: error code definition is separated from display copy

- `ErrorCodeDefinition`: error code metadata (code, category, template), no display copy
- `locales/*.json`: per-language display copy (title, message, help, error code as nested object)
- `DiagnosticBuilder`: generic builder, replacing the trait-per-error design

#### Error Code Definition

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only, display copy in i18n file)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // Message template, supports {param} placeholders
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

    /// Set position
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Build a Diagnostic (template rendering done at compile time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Check that every {key} in the template has a matching parameter
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

#### Shortcut Methods per Error Code

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
    // ... other error codes
];
```

#### Design Advantages

| Feature                 | Description                                              |
| ----------------------- | -------------------------------------------------------- |
| **Single Builder**      | One `DiagnosticBuilder` is generic for all error codes   |
| **Type safety**         | Shortcut methods ensure parameter correctness            |
| **Self-documenting**    | `E1001::unknown_variable(name)` is self-explanatory      |
| **Template separation** | Message templates are separated from code, easy for i18n |
| **Zero runtime cost**   | Compile-time rendering, no table lookup in AOT binary    |

---

### Error Macro Simplification

#### `error!` Macro (Auto-Injecting Context)

```rust
/// Macro that automatically obtains span and i18n config at compile time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: only pass parameters; span and i18n are auto-injected
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Manual Builder Usage

```rust
// When manual control is needed
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
| E0013 | Mismatched brackets       |
| E0014 | Missing semicolon         |
| E0016 | Expected expression       |
| E0018 | Keyword used as name      |

<!-- code-table:E0xxx end -->

#### E1xxx: Type Checking

<!-- code-table:E1xxx start -->

| Code  | Description                                               |
| ----- | --------------------------------------------------------- |
| E1001 | Unknown variable                                          |
| E1002 | Type mismatch                                             |
| E1003 | Unknown type                                              |
| E1010 | Argument count mismatch                                   |
| E1011 | Argument type mismatch                                    |
| E1012 | Return type mismatch                                      |
| E1013 | Function not found                                        |
| E1014 | Unknown named argument                                    |
| E1015 | Duplicate argument specification                          |
| E1020 | Cannot infer type                                         |
| E1021 | Type inference conflict                                   |
| E1030 | Incomplete pattern                                        |
| E1031 | Unreachable pattern                                       |
| E1032 | Pattern duplicate binding                                 |
| E1033 | Or-pattern binding inconsistent                           |
| E1034 | Struct pattern missing field                              |
| E1040 | Operation not supported                                   |
| E1041 | Index out of bounds                                       |
| E1042 | Field not found                                           |
| E1043 | Module member not found                                   |
| E1050 | Boolean operand required                                  |
| E1051 | Logical NOT requires boolean operand                      |
| E1052 | Invalid dereference                                       |
| E1053 | Non-struct field access                                   |
| E1054 | Conditional type mismatch                                 |
| E1055 | Constraint in non-generic context                         |
| E1060 | Type argument count mismatch                              |
| E1061 | Cannot instantiate generic                                |
| E1062 | const generic constraint failed                           |
| E1064 | Invalid binding position index                            |
| E1065 | Call on non-function value                                |
| E1071 | Type definition only allowed at module level              |
| E1081 | `?` only allowed in functions returning propagatable type |
| E1082 | `?` only usable for types implementing Try                |
| E1083 | `?` error type mismatch                                   |
| E1090 | ✨ Unspeakable ✨                                         |
| E1091 | Invalid generic meta-type                                 |
| E1092 | Refinement type argument form illegal                     |
| E1093 | Refinement argument count mismatch                        |
| E1094 | Unused compile-time value parameter                       |
| E1095 | Unknown interface                                         |
| E1096 | Interface argument count mismatch                         |
| E1097 | Interface member name conflict                            |
| E1098 | Interface method not implemented                          |
| E1099 | Interface method signature mismatch                       |
| E1100 | Interface method duplicate implementation                 |
| E1101 | Type does not implement interface                         |
| E1102 | Loop control statement outside loop                       |
| E1103 | Brackets not usable in type position                      |
| E1104 | Interface implementation not in type's defining module    |
| E1105 | Variant constructor not accessible as field               |
| E1106 | Constraint not satisfied                                  |
| E1107 | Method overload ambiguity                                 |
| E1108 | Empty block falls into container expected position        |
| E1109 | `return` outside function                                 |
| E1110 | Method binding missing signature annotation               |
| E1111 | Arguments duplicate occupancy or order illegal            |

<!-- code-table:E1xxx end -->

> **RFC-011b cross-reference (2026-09-22 note)**:
> [RFC-011b: Operator Overloading](011b-operator-overloading.md), when landed, will affect three
> places in this section — ① the `E1081` / `E1082` copy removes the word "Result" (the `?` operator
> switches to judging by the `Try` interface and is no longer tied to a specific type name), and the
> change is propagated through the "three-way consistency" flow of this document (codes/*.rs ↔
> locales ↔ code table) at phase 2; ② the refusal diagnostic for an unsatisfied `Equal` precondition
> (linear token) reuses the `E1101` family (type does not implement interface); ③ after phase 1
> wiring, `Struct == Struct` moves from the `E6007` runtime error to a compile-time judgment, and
> the trigger surface of `E6007` shrinks. The registered copy in the table remains as is until the
> implementation lands.

#### E2xxx: Semantic Analysis

<!-- code-table:E2xxx start -->

| Code  | Description                         |
| ----- | ----------------------------------- |
| E2001 | Scope error                         |
| E2002 | Duplicate definition                |
| E2003 | Ownership error                     |
| E2010 | Immutable assignment                |
| E2011 | Use of uninitialized variable       |
| E2012 | Mutability conflict                 |
| E2013 | Variable shadowing                  |
| E2014 | Use of moved value                  |
| E2016 | Immutable assignment                |
| E2018 | Mutable/immutable borrow conflict   |
| E2019 | Double free                         |
| E2020 | Use after free                      |
| E2027 | unsafe dereference                  |
| E2029 | Reference cycle inside spawn        |
| E2030 | Refinement type constraint violated |
| E2031 | Refinement constraint unprovable    |
| E2090 | Invalid signature                   |
| E2091 | Unknown type in signature           |
| E2092 | Missing arrow in signature          |
| E2093 | Duplicate parameter name            |
| E2094 | Generic parameter shadowed          |
| E2095 | Parameter name shadows generic      |
| E2096 | Bare container type in signature    |

<!-- code-table:E2xxx end -->

> Reserved-code note (2026-09-14 inventory, #251 release wording): E2019 (double free), E2020 (use
> after free), E2027 (unsafe dereference), E2029 (ref cycle inside spawn) have all been registered
> and anchored by unit tests, but no reachable yx source-code surface exists yet (explicit drop
> statement, Ptr dereference syntax, spawn ref cycle construction path) — the claim of full semantic
> correctness does not cover these four codes, which are treated as "reserved" until implementation
> is filled in.

#### E3xxx: Code Generation

<!-- code-table:E3xxx start -->

| Code  | Description                                    |
| ----- | ---------------------------------------------- |
| E3004 | Unsupported iterator                           |
| E3005 | IR generation error                            |
| E3006 | Unresolved variable                            |
| E3007 | Top-level binding initializer must be constant |
| E3008 | Unsupported match pattern                      |
| E3014 | Register overflow                              |
| E3017 | Invalid operand (code generation)              |
| E3018 | Monomorphization instantiation failed          |
| E3019 | Top-level binding cyclic dependency            |
| E3020 | Missing program entry                          |
| E3021 | Entry is not a function                        |
| E3022 | Entry `main` signature mismatch                |
| E3023 | Executable statements not allowed at top level |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and Traits

<!-- code-table:E4xxx start -->

| Code  | Description                        |
| ----- | ---------------------------------- |
| E4001 | Generic constraint violated        |
| E4002 | Trait not found                    |
| E4003 | Trait implementation missing       |
| E4004 | Trait implementation conflict      |
| E4005 | Associated type not found          |
| E4010 | Constant division by zero          |
| E4011 | Constant overflow                  |
| E4012 | Constant recursion too deep        |
| E4014 | Constant evaluation failed         |
| E4018 | Refinement predicate violated      |
| E4019 | Type equality does not hold        |
| E4020 | Proof function required            |
| E4021 | Loop termination not auto-provable |
| E4022 | Measure does not hold              |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission point (reserved codes): the Sized constraint and the
> optimization-error path await implementation; they will be wired according to the real trigger
> surface when implemented.

#### E5xxx: Module and Import

<!-- code-table:E5xxx start -->

| Code  | Description          |
| ----- | -------------------- |
| E5001 | Module not found     |
| E5002 | Import error         |
| E5003 | Export not found     |
| E5004 | Cyclic dependency    |
| E5005 | Invalid module path  |
| E5006 | Duplicate import     |
| E5007 | Module export        |
| E5008 | Import name conflict |

<!-- code-table:E5xxx end -->

#### E6xxx: Runtime Errors

<!-- code-table:E6xxx start -->

| Code  | Description                  |
| ----- | ---------------------------- |
| E6001 | Division by zero             |
| E6003 | Array index out of bounds    |
| E6004 | Stack overflow               |
| E6005 | Assertion failed             |
| E6006 | Function not found (runtime) |
| E6007 | Runtime error                |
| E6008 | Key does not exist           |
| E6009 | Invalid Range step           |
| E6010 | Integer parse failure        |
| E6011 | Float parse failure          |
| E6012 | Invalid code point           |
| E6013 | JSON parse failure           |

<!-- code-table:E6xxx end -->

> **Code table revision (2026-08-09)**: the table was originally defined under a Rust-style semantic
> draft (Assertion failed / Arithmetic overflow / Heap allocation failed / Type cast failed), which
> does not match the actual implementation needs. YaoXiang has no concept of null pointers, heap
> allocation failure, or type casting (value semantics + Rust-like memory safety), and runtime
> overflow paths do not implement detection. After calibration:
>
> - E6002 removed (the original Assertion failed moved to E6005; the original null-pointer semantics
>   have no language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (real trigger surface)
> - E6005 changed from Heap allocation failed to Assertion failed (the real path of `std.assert`)
> - E6006 changed from Runtime index out of bounds to Function not found (this is how the
>   implementation has always been)
> - E6007 changed from Type cast failed to the generic Runtime error (unified landing for
>   `ExecutorError` unmapped variants)

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

| Code  | Description                             |
| ----- | --------------------------------------- |
| W1001 | Unused private function                 |
| W1002 | Unused private type                     |
| W1003 | Unused import                           |
| W1004 | Unused private variable                 |
| W1005 | Unused private method                   |
| W1006 | Local module shadows dependency package |
| W1063 | const generic constraint unevaluable    |
| W1080 | Compile-time proof downgraded           |
| W1081 | Termination obligation unjudged         |
| W1082 | Ownership analysis downgraded           |
| W1090 | Obligation produced but no consumer     |

<!-- code-table:W1xxx end -->

> W-code position rule: isomorphic to E-codes, grouped by phase (W + phase-thousands digit); W1xxx =
> type-checking-phase warnings.
>
> **Dead-code semantics (RFC-029f role semantics + RFC-029g)**: W1001/W1002/W1004/W1005 report
> **top-level bindings and type definitions that are not referenced** — if a binding is never
> reached in the file-local reachability analysis starting from `main`, it is reported; methods
> (W1005) are matched by short name at the call site. The only exemption is the **package-internal
> reference pool** (RFC-029f: any name referenced by a file inside the project is considered live —
> cross-file consumers are not visible to this file, so we err on the side of under-reporting).
>
> **Landing correction (2026-10-08, RFC-029g)**: the original text of this section said "a `pub`
> definition is an external interface and is never reported (#321 decision B)". The `pub` keyword
> has been removed, and modifiers no longer produce any exemption; the role-distinguished exemption
> policy (Bin reports immediately / Internal tightened in two phases / Lib absolutely exempt) is
> converged into a single condition "does it provide a package-internal reference pool". See
> [029g](029g-remove-pub-and-auto-bind.md) for the behavior change.
>
> **Unused imports (W1003)**: detected by typecheck's use elaboration (pass2 records the
> locally-imported name; if it is hit at an expression-resolution or type-annotation position it is
> considered used), covering whole imports (`use std.io` → module alias) and named imports
> (`use std.io.{print}`).
>
> **Emission channel**: W-code diagnostics are marked `Severity::Warning` by default by the builder
> based on the W prefix (explicit specification takes priority). Collection and presentation follow
> the same path as errors (rendered with the `warning[W####]` prefix), but they do not block
> compilation and do not affect the successful exit code. `yaoxiang check --deny-warnings` escalates
> warnings to failure (exits with a non-zero code when warnings exist), for use in CI strict mode.
> Per-code suppression (allow attributes, etc.) is a future extension.
>
> **Code family decoupled from emission level (W1090 same code escalated to E, 2026-10-13 user
> ruling)**: **the code-number prefix only determines the numbering affiliation, not the emission
> level** — the level is determined by the explicit `severity` at the diagnostic construction site
> (`E` prefix defaults to Error, `W` prefix defaults to Warning, explicit specification takes
> priority). Accordingly, `W1090` (obligation produced but no consumer) **keeps the W-family code
> number unchanged** (no new E code: two codes for the same fact would be parallel representations),
> but starting from WBS 4.3.2 **its emission level is `error`**: when triggered, it is counted in
> `error_count`, rendered as `error[W1090]`, and changes the exit code (the **end state** of RFC-039
> D5; it is a class-D breaking item of D59, honestly recorded in 02-stage-contract §2). A healthy
> program emits zero, so this escalation only affects build results when a phase-contract defect
> really occurs.

### Message Quality Specification

> This section is introduced by the message single-track and quality revision (2026-09-03). Enforced
> in CI by `scripts/audit_diagnostics.py`.

1. **Single message track**: all user-visible diagnostic messages must go through the authoritative
   registry's shortcut method + locales template rendering; the code only passes structured
   parameters. It is forbidden to bypass the registry and directly construct raw values like
   `Diagnostic::error(...)` — that path bypasses code validation and i18n.
2. **Code legality**: using unregistered codes and pseudo-codes (such as `E_INTERNAL`) is forbidden;
   the code literal at the use site must already be defined in the registry. Internal errors always
   fall to E8001 (`internal_error`).
3. **Type display**: type Display must distinguish the pre-/post-instantiation form (bare names like
   `Expected 'Container', found 'Container'` cannot be distinguished).
4. **Solver internal-state isolation**: solver intermediate TypeVars (Display form `t<N>`) must not
   enter user-visible messages. Test anchor: `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is only used for compiler internal consistency issues (ICEs).
   User-fixable errors must not be swept into E8001; ICE messages must include a
   minimal-reproduction guide.

---

### Runtime Error Values and Code Unification

> This section is introduced by the runtime Error-value-with-code revision (2026-09-03). The
> E6xxx/E7xxx semantic space carries two channels; the code space is shared and the presentation
> channels differ.

#### Two Channels

| Channel                         | Carrier                                          | Presentation                                              |
| ------------------------------- | ------------------------------------------------ | --------------------------------------------------------- |
| Compiler/CLI diagnostic channel | `ExecutorError` and other host-level hard errors | stderr `error[E####]:` (already wired: E6003/E6005/E6007) |
| In-program error-value channel  | std-lib `Result(T, Error)`'s Err carrier `Error` | Language value, consumed by program match/comparison      |

#### Error Structure (since v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbers of this specification, in string form (e.g. `"E6008"`).
- **Stability contract**: assigned codes keep their semantics across versions; the same semantics
  must not reuse a deleted code (E6002 precedent).
- **Consumption side**: in-program `e.code == "E6xxx"` comparison is the only programmable judgment
  contract; `yaoxiang explain E6xxx` documentation unifies the surface; the toolchain (LSP / DAP,
  see RFC-034) uses the code as `exceptionId`.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: the `E` of `Result(T, E)` is a generic parameter; serious modeling uses a
  user-defined type; std `Error` is only a convenient fallback carrier, and its code system does not
  constrain user E types.

#### Code Allocation Rules

1. Runtime error-value codes share the E6xxx/E7xxx space with compiler diagnostic codes; new codes
   are allocated by **real trigger surface**, and not reserved for imagined scenarios.
2. Register before use: a new code enters the authoritative registry and passes the three-way
   consistency check (codes/*.rs ↔ locales ↔ the code table in this document) before it can be
   emitted. The registration source for runtime error-value codes is the `RUNTIME_ERROR_CODES` table
   in `src/std/result.rs` (subject to the same `build.rs` compile-time gate + `tools/code-tables`
   check as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error values (currently empty; enabled when io/net is
   Result-ified).
4. Emission point: each std module constructs Error values via `error_new(code, message)`; on the
   consumer side `std.result.unwrap_err` retrieves the Err carrier, and `std.result.code/message`
   read the fields.

#### Evolution Path (line C, not implemented)

Once pattern-matching completeness (RFC-010b) lands, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming an attribute derived from `kind` (the
variant definition site is the code registry). During the evolution period the code stability
contract of this section remains unchanged; the upgrade is a separate decision and does not
constitute a commitment of this section.

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
// locales/*.json (error-code object)

/// i18n display-text registry (loaded from JSON at compile time, zero table lookup at runtime)
pub struct I18nRegistry {
    /// Titles
    titles: HashMap<&'static str, &'static str>,
    /// Descriptions
    messages: HashMap<&'static str, &'static str>,
    /// Help information
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
    /// Get a registry by language code
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// Get error information
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// Render a template (done at compile time, zero runtime cost)
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

| Placeholder  | Purpose                            | Example                             |
| ------------ | ---------------------------------- | ----------------------------------- |
| `{name}`     | Variable / type / trait name, etc. | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                      | `Expected type '{expected}'`        |
| `{found}`    | Actual / found type                | `, found type '{found}'`            |
| `{method}`   | Method name                        | `Method {method} is not a function` |
| `{trait}`    | Trait name                         | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                        | `Invalid path: {path}`              |
| `{ty}`       | Type expression                    | `Invalid type: {ty}`                |
| `{message}`  | Internal error message             | `Internal error: {message}`         |

##### Arbitrary Keys Supported

**`params` supports arbitrary keys, not limited to the predefined ones.** The caller can pass any
`key`:

```rust
// Use arbitrary keys
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// Template definition
"Unknown variable: '{name}' at {location}. {hint}"
```

> **Note**: not all error codes use placeholders. Some error codes (such as E0001) are static
> messages that need no parameters.

#### Language Priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. Default: en
```

### yaoxiang.toml Configuration

#### Project-Level Configuration

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# Error message language, options: en, zh, ja, ...
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
1. Read language.default from the project-level yaoxiang.toml
2. If not set, read the user-level ~/.yaoxiang/yaoxiang.toml
3. If neither is set, default to "en"
4. The compiler creates an I18nRegistry according to the chosen language (once)
5. All errors are rendered using that I18nRegistry
```

#### The Key to Zero Table-Lookup Cost

**Rendering happens when the user's project is compiled, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Rust compiles the YaoXiang compiler                            │
│                                                                           │
│  JSON is packed into the compiler binary                                │
│  Purpose: the explain command can read i18n data directly              │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: YaoXiang compiles the user project (rendering happens here)    │
│                                                                           │
│  When the error! macro is called:                                        │
│  1. Read yaoxiang.toml to obtain the language preference                 │
│  2. Load the i18n JSON of the corresponding language from the compiler   │
│  3. Template + parameters → render() → "Unknown variable: 'x'"          │
│  4. Diagnostic.message = the rendered string                             │
│                                                                           │
│  The AOT binary directly stores the final string — no template, no      │
│  table lookup                                                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runs at runtime                                   │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Outputs the final string directly, no table lookup at all            │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                       | Render time                     |
| ---------------------------- | ------------------------------------ | ------------------------------- |
| `I18nRegistry`               | Provides templates and display text  | When compiling the user project |
| `DiagnosticBuilder.render()` | Template + parameters → final string | When compiling the user project |
| `Diagnostic.message`         | The already-rendered string          | Stores the final result         |
| AOT binary                   | Contains the final strings           | Used directly at runtime        |

---

### Error Message Format

Error messages use the following format:

```
error[E####]: <short description>
  --> <file>:<line>:<col>
   <line> | <code snippet>
          ^^^<highlight>
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

Severity is managed through the `DiagnosticLevel` enum, decoupled from the error code numbering:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation to fail
    Warning,  // Does not affect compilation, but should be fixed
    Note,     // Supplementary information
    Help,     // Fix suggestion
}
```

| Level   | Prefix            | Description                 |
| ------- | ----------------- | --------------------------- |
| Error   | `error[E####]:`   | Causes compilation to fail  |
| Warning | `warning[E####]:` | Does not affect compilation |
| Note    | `note[E####]:`    | Supplementary information   |
| Help    | `help[E####]:`    | Fix suggestion              |

---

### `yaoxiang explain` Command

#### Command Syntax

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Options

| Option          | Description                                    |
| --------------- | ---------------------------------------------- |
| `--lang <code>` | Specify language (en-US, zh-CN, default en-US) |
| `--json`        | JSON format output (for IDE/LSP)               |
| `--json-pretty` | Pretty-printed JSON output                     |
| `--examples`    | Show only example code                         |
| `--help`        | Show help information                          |

#### Usage Examples

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

Since this RFC designs the error code system from scratch, there is no backward-compatibility issue.

**Future migration strategy** (for reference in later versions):

1. Keep the mapping from old error codes to new error codes
2. Display both old and new codes during the migration period
3. Provide a deprecation schedule

---

## Implementation Strategy

### Phase 1: Error Code Infrastructure

1. Create the `src/diagnostics/` directory structure
2. Implement the `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create the resource file directory and example JSON

### Phase 2: `explain` Command

1. Implement the `yaoxiang explain` CLI command
2. Support the `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase 3: Compile-Time Integration

1. Update all error-reporting sites to use the new system
2. Implement message template parameter injection
3. Add language priority logic
4. Unit test coverage

### Phase 4: IDE/LSP Integration

1. LSP server integrates `explain` JSON output
2. Display error code links in the IDE
3. Show error explanation on hover
4. Quick-fix suggestions

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
| E5xxx | Module and import              |
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
- [Clang Diagnostics Format](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
