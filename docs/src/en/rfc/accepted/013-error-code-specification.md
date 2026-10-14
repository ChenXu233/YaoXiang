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
Rust-like single-level numbering system, combined with JSON resource files to provide multi-language
support, and an `yaoxiang explain` command to provide error explanation functionality.

## Motivation

### Why need standardized error codes?

1. **User Experience**: Users seeing an error code can quickly determine the error type and severity
2. **Documentation Organization**: Grouping by category facilitates writing and maintaining error
   reference documentation
3. **Tool Integration**: IDE/LSP can provide quick-fix suggestions and documentation links based on
   error codes
4. **Internationalization Support**: Separation of error messages from codes facilitates
   multi-language translation

### Design Goals

- **Concise**: Single-level numbering, users don't need to remember complex classification rules
- **Friendly**: Rust-like error message format, with help information and examples
- **Extensible**: Resource-file driven, easy to add new errors and new languages
- **Tool-friendly**: explain command + JSON output, supports IDE/LSP integration

---

## Proposal

### Core Design: Single-Level Numbering System

A four-digit numbering scheme, grouped by compilation phase:

```
Exxxx
││││
│││└── Sequence number (000-999)
││└─── Compilation phase (0-9)
└───── Fixed prefix 'E'
```

### Phase Division

| Phase | Range | Description                 |
| ----- | ----- | --------------------------- |
| **0** | E0xxx | Lexical and Syntax Analysis |
| **1** | E1xxx | Type Checking               |
| **2** | E2xxx | Semantic Analysis           |
| **3** | E3xxx | Code Generation             |
| **4** | E4xxx | Generics and Traits         |
| **5** | E5xxx | Modules and Imports         |
| **6** | E6xxx | Runtime Errors              |
| **7** | E7xxx | I/O and System Errors       |
| **8** | E8xxx | Internal Compiler Errors    |
| **9** | E9xxx | Reserved/Experimental       |

### Error Category Enumeration

```rust
/// Error category
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

### Error Code Definition and Generic Builder

**Core principle**: Error code definitions are separated from display text

- `ErrorCodeDefinition`: Error code metadata (code, category, template), not including display text
- `locales/*.json`: Display text for each language (title, message, help, with error codes as nested
  objects)
- `DiagnosticBuilder`: Generic builder, replacing the trait-per-error design

#### Error Code Definition

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only; display text is in i18n files)
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

    /// Add template parameter
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// Set position
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Build Diagnostic (template rendering completes at compile-time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Check that all {key} in template have corresponding parameters
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

#### Shortcut Methods for Each Error Code

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

// Simplified way
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// Manual way
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

| Feature                   | Description                                           |
| ------------------------- | ----------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` for all error codes           |
| **Type-safe**             | Shortcut methods ensure parameter correctness         |
| **Self-documenting**      | `E1001::unknown_variable(name)` is self-explanatory   |
| **Template separation**   | Message templates separated from code, easy for i18n  |
| **Zero runtime overhead** | Compile-time rendering, no table lookup in AOT binary |

---

### Error Macro Simplification

#### error! macro (automatic context injection)

```rust
/// Macro that automatically obtains span and i18n configuration at compile-time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: only need to pass parameters, span and i18n are auto-injected
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

#### E0xxx: Lexical and Syntax Analysis

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
| E0018 | Keyword as name           |

<!-- code-table:E0xxx end -->

#### E1xxx: Type Checking

<!-- code-table:E1xxx start -->

| Code  | Description                                                |
| ----- | ---------------------------------------------------------- |
| E1001 | Unknown variable                                           |
| E1002 | Type mismatch                                              |
| E1003 | Unknown type                                               |
| E1010 | Argument count mismatch                                    |
| E1011 | Argument type mismatch                                     |
| E1012 | Return type mismatch                                       |
| E1013 | Function not found                                         |
| E1014 | Unknown named argument name                                |
| E1015 | Argument specified multiple times                          |
| E1020 | Cannot infer type                                          |
| E1021 | Type inference conflict                                    |
| E1030 | Pattern incomplete                                         |
| E1031 | Unreachable pattern                                        |
| E1032 | Pattern duplicate binding                                  |
| E1033 | Inconsistent binding in or-pattern                         |
| E1034 | Struct pattern missing field                               |
| E1040 | Operation not supported                                    |
| E1041 | Index out of bounds                                        |
| E1042 | Field not found                                            |
| E1043 | Module member not found                                    |
| E1050 | Boolean operand required                                   |
| E1051 | Logical NOT requires boolean operand                       |
| E1052 | Invalid dereference                                        |
| E1053 | Field access on non-struct                                 |
| E1054 | Condition type mismatch                                    |
| E1055 | Constraint in non-generic context                          |
| E1060 | Type parameter count mismatch                              |
| E1061 | Cannot instantiate generic                                 |
| E1062 | const generic constraint failed                            |
| E1064 | Invalid index at binding position                          |
| E1065 | Call on non-function value                                 |
| E1071 | Type definition only allowed at module level               |
| E1081 | `?` only allowed in functions returning propagatable types |
| E1082 | `?` can only be used on types implementing Try             |
| E1083 | `?` error type mismatch                                    |
| E1090 | ✨ Unspeakable ✨                                          |
| E1091 | Invalid generic meta-type                                  |
| E1092 | Invalid refinement type argument form                      |
| E1093 | Refinement argument count mismatch                         |
| E1094 | Unused compile-time value argument                         |
| E1095 | Unknown interface                                          |
| E1096 | Interface parameter count mismatch                         |
| E1097 | Interface member name conflict                             |
| E1098 | Interface method not implemented                           |
| E1099 | Interface method signature mismatch                        |
| E1100 | Interface method implemented multiple times                |
| E1101 | Type does not implement interface                          |
| E1102 | Loop control statement outside loop                        |
| E1103 | Type position cannot use square brackets                   |
| E1104 | Interface implementation not in type's defining module     |
| E1105 | Variant constructor not accessible as field                |
| E1106 | Constraint not satisfied                                   |
| E1107 | Method overload ambiguous                                  |
| E1108 | Empty block falls into container expected position         |
| E1109 | return outside function                                    |
| E1110 | Method binding missing signature annotation                |

<!-- code-table:E1xxx end -->

> **RFC-011b Reference (2026-09-22 Note)**:
> [RFC-011b: Operator Overloading](011b-operator-overloading.md) The implementation will affect
> three places in this section—① `E1081` / `E1082` text removing the "Result" wording (`?` changed
> to be determined by the `Try` interface, no longer bound to a specific type name), synchronized
> with phase 2 according to the "three-way consistency" process in this document (codes/*.rs ↔
> locales ↔ code table); ② `Equal` precondition (linear token) not satisfied rejection diagnostic
> reuses the `E1101` (type does not implement interface) family; ③ After phase 1 wiring,
> `Struct == Struct` transitions from `E6007` runtime error to compile-time determination, with
> `E6007` trigger surface shrinking. The registered text in the table maintains its current state
> until the implementation lands.

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
| E2091 | Signature unknown type                 |
| E2092 | Signature missing arrow                |
| E2093 | Duplicate parameter name               |
| E2094 | Generic parameter shadowing            |
| E2095 | Parameter name shadows generic         |
| E2096 | Signature bare container type          |

<!-- code-table:E2xxx end -->

> Reserved code notes (2026-09-14 inventory, #251 release): E2019 (Double free), E2020 (Use after
> free), E2027 (unsafe dereference), E2029 (spawn internal ref cycle) have completed registration
> and are anchored with unit tests, but there is no reachable yx source code surface layer (explicit
> drop statements, Ptr dereference syntax, spawn ref cycle creation paths)—the claim of
> "semantically fully correct" does not cover these four codes; they are treated as "reserved" until
> the implementation is complete.

#### E3xxx: Code Generation

<!-- code-table:E3xxx start -->

| Code  | Description                                       |
| ----- | ------------------------------------------------- |
| E3004 | Unsupported iterator                              |
| E3005 | IR generation error                               |
| E3006 | Unresolved variable                               |
| E3007 | Top-level binding initialization must be constant |
| E3008 | Unsupported match pattern                         |
| E3014 | Register overflow                                 |
| E3017 | Invalid operand (code generation)                 |
| E3018 | Monomorphization instantiation failure            |
| E3019 | Top-level binding circular dependency             |
| E3020 | Missing program entry                             |
| E3021 | Entry is not a function                           |
| E3022 | Entry main signature mismatch                     |
| E3023 | Top-level does not allow executable statements    |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and Traits

<!-- code-table:E4xxx start -->

| Code  | Description                                     |
| ----- | ----------------------------------------------- |
| E4001 | Generic constraint violation                    |
| E4002 | Trait not found                                 |
| E4003 | Trait implementation missing                    |
| E4004 | Trait implementation conflict                   |
| E4005 | Associated type not found                       |
| E4010 | Constant division by zero                       |
| E4011 | Constant overflow                               |
| E4012 | Constant recursion too deep                     |
| E4014 | Constant evaluation failure                     |
| E4018 | Refinement predicate violation                  |
| E4019 | Type equality does not hold                     |
| E4020 | Proof function required                         |
| E4021 | Loop termination cannot be automatically proven |
| E4022 | Measure does not hold                           |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission points (reserved codes): Sized constraint and optimization
> error paths are pending implementation; wire according to actual trigger surface when implemented.

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

| Code  | Description                  |
| ----- | ---------------------------- |
| E6001 | Division by zero             |
| E6003 | Array index out of bounds    |
| E6004 | Stack overflow               |
| E6005 | Assertion failed             |
| E6006 | Function not found (runtime) |
| E6007 | Runtime error                |
| E6008 | Key does not exist           |
| E6009 | Range step invalid           |
| E6010 | Integer parse failure        |
| E6011 | Float parse failure          |
| E6012 | Invalid code point           |
| E6013 | JSON parse failure           |

<!-- code-table:E6xxx end -->

> **Code Table Revision (2026-08-09)**: The code table was originally defined based on Rust semantic
> draft (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed), which does
> not match the actual implementation needs. YaoXiang has no concepts of null pointer/heap
> allocation failure/type conversion (value semantics + Rust memory safety), and runtime overflow
> paths have not implemented detection. After calibration:
>
> - E6002 deleted (original Assertion failed moved to E6005; original null pointer semantics have no
>   language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (actual trigger surface)
> - E6005 changed from Heap allocation failed to Assertion failed (std.assert actual path)
> - E6006 changed from Runtime index out of bounds to Function not found (implementation already so)
> - E6007 changed from Type cast failed to generic Runtime error (unified landing point for unmapped
>   ExecutorError variants)

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
| W1006 | Local module shadowing dependency package    |
| W1063 | const generic constraint cannot be evaluated |
| W1080 | Compile-time proof degraded                  |
| W1081 | Termination obligation not determined        |
| W1090 | Obligation produced but no consumer          |

<!-- code-table:W1xxx end -->

> W code position rule: Isomorphic to E codes, grouped by phase (W + phase thousands digit), W1xxx =
> Type checking phase warnings.
>
> **Dead code code semantics (RFC-029f role semantics + RFC-029g)**: W1001/W1002/W1004/W1005 report
> **unreferenced top-level bindings and type definitions**—reported if never referenced in the
> file-local reachability analysis starting from `main`; methods (W1005) match by call site short
> name. The only exemption is the **in-package reference pool** (RFC-029f: names referenced by any
> file in the project are considered live—cross-file consumers are not visible in this file, biased
> towards under-reporting).
>
> **Implementation correction (2026-10-08, RFC-029g)**: The original text of this section was "`pub`
> definition is an external interface, never reported (#321 Decision B)". The `pub` keyword has been
> removed, modifiers no longer produce any exemption; the original role-distinguished exemption
> policy (Bin reports immediately / Internal tightens in two phases / Lib absolute exemption)
> converges to a single condition "whether to provide an in-package reference pool"; for behavioral
> changes, see [029g](029g-remove-pub-and-auto-bind.md).
>
> **Unused import (W1003)**: Detected by typecheck's use elaboration (pass2 records imported local
> names; considered used if hit in expression resolution and type annotation positions), covering
> both whole imports (`use std.io` → module alias) and named imports (`use std.io.{print}`).
>
> **Emission channel**: W code diagnostics are tagged by the builder with `Severity::Warning` by
> default for the W prefix (explicit specification takes priority), collected and presented on the
> same track as errors (`warning[W####]` prefix rendering), but does not block compilation or affect
> the success exit code. `yaoxiang check --deny-warnings` upgrades warnings to failures (exits with
> non-zero code when warnings exist), used for CI strict mode. per-code suppression (allow
> attributes etc.) is a future extension.
>
> **Code family and emission level decoupling (W1090 same code escalated to E, 2026-10-13 user
> ruling)**: **The code number prefix only determines number attribution, not emission level**—the
> level is determined by the explicit `severity` at the diagnostic construction point (E prefix
> defaults to Error, W prefix defaults to Warning, explicit specification takes priority).
> Accordingly, `W1090` (obligation produced but no consumer) maintains **the W family code number
> unchanged** (no new E code added: two codes for the same fact would be parallel representation),
> but starting from WBS 4.3.2 **the emission level is error**: trigger counts in `error_count`,
> rendered as `error[W1090]`, changes exit code (the **end state** of RFC-039 D5; belongs to the
> D-class breaking item of D59, truthfully recorded in 02-stage-contract §2). Healthy programs emit
> zero (no report when all obligations have delivery markers), so this upgrade only affects the
> build result when a stage contract defect actually appears.

### Message Quality Specification

> This section was introduced by the message single-track and quality revision (2026-09-03).
> Enforced by `scripts/audit_diagnostics.py` in CI.

1. **Message single-track**: All user-visible diagnostic messages must go through authoritative
   registry shortcut methods + locales template rendering; the code only passes structured
   parameters. It is forbidden to bypass the registry and directly construct raw values like
   `Diagnostic::error(...)`—this path bypasses code validation and i18n.
2. **Code validity**: Forbidden to use unregistered codes and pseudo-codes (such as `E_INTERNAL`);
   the code literal at the use point must already be defined in the registry. Internal errors
   uniformly fall to E8001 (`internal_error`).
3. **Type display**: Type Display must distinguish between pre- and post-instantiation forms
   (`Expected 'Container', found 'Container'` bare names are indistinguishable).
4. **Solver internal state isolation**: Solver intermediate state TypeVar (Display form `t<N>`) must
   not enter user-visible messages. Test anchor: `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is only for compiler internal consistency issues (ICE). Errors that
   users can fix are forbidden from using E8001 as a catch-all; ICE messages must include minimum
   reproduction guidance.

---

### Runtime Error Values and Code Integration

> This section was introduced by the runtime Error value with code revision (2026-09-03). The
> E6xxx/E7xxx semantic space carries two channels simultaneously, with the same code space but
> different presentation channels.

#### Two Channels

| Channel                         | Carrier                                            | Presentation                                      |
| ------------------------------- | -------------------------------------------------- | ------------------------------------------------- |
| Compiler/CLI diagnostic channel | `ExecutorError` and other host-layer hard errors   | stderr `error[E####]:` (wired E6003/E6005/E6007)  |
| In-program error value channel  | std library `Result(T, Error)` Err carrier `Error` | Language value, consumed by program match/compare |

#### Error Structure (since v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses this specification's E6xxx/E7xxx numbers, in string form (such as `"E6008"`).
- **Stable contract**: Allocated codes have unchanging semantics across versions; the same semantics
  do not reuse deleted codes (E6002 precedent).
- **Consumption surface**: In-program `e.code == "E6xxx"` comparison is the only programmable
  judgment contract; `yaoxiang explain E6xxx` documentation integration; toolchain (LSP / DAP, see
  RFC-034) uses codes as exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: `Result(T, E)`'s E is a generic parameter, serious modeling uses
  user-defined types; std `Error` is only a convenient fallback carrier, its code system does not
  constrain user E types.

#### Code Assignment Rules

1. Runtime error value codes and compiler diagnostic codes share the E6xxx/E7xxx space; new codes
   are assigned based on **actual trigger surface**, not reserved for imagined scenarios.
2. Register first, then use: New codes enter the authoritative registry and pass three-way
   consistency check (codes/*.rs ↔ locales ↔ this document's code table) before emission. The
   registration source for runtime error value codes is the `RUNTIME_ERROR_CODES` table in
   `src/std/result.rs` (subject to the same `build.rs` build-time threshold + `tools/code-tables`
   verification as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error values (currently empty, enabled when io/net becomes
   Result-typed).
4. Emission points: std modules construct Error values via `error_new(code, message)`; on the
   consumer side, `std.result.unwrap_err` extracts the Err carrier, and `std.result.code/message`
   reads the fields.

#### Evolution Path (Line C, not implemented)

After pattern matching completion (RFC-010b) lands, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` converted to an attribute derived from kind (the
variant definition site is the code registry). During the evolution period, the code stable contract
in this section remains unchanged; this upgrade is an independent decision and does not constitute a
commitment of this section.

---

### Multi-language Resource Files

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
// locales/*.json (error code objects)

/// i18n display text registry (loaded from JSON at compile-time, zero table lookup at runtime)
pub struct I18nRegistry {
    /// Titles
    titles: HashMap<&'static str, &'static str>,
    /// Descriptions
    messages: HashMap<&'static str, &'static str>,
    /// Help information
    helps: HashMap<&'static str, &'static str>,
    /// Example code
    examples: HashMap<&'static str, &'static str>,
    /// Error output examples
    error_outputs: HashMap<&'static str, &'static str>,
}

/// Single error code information
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// Get registry based on language code
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

    /// Render template (completes at compile-time, zero overhead at runtime)
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

| Placeholder  | Purpose                                    | Example                             |
| ------------ | ------------------------------------------ | ----------------------------------- |
| `{name}`     | Identifiers like variable/type/trait names | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                              | `Expected type '{expected}'`        |
| `{found}`    | Actual/found type                          | `, found type '{found}'`            |
| `{method}`   | Method name                                | `Method {method} is not a function` |
| `{trait}`    | Trait name                                 | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                | `Invalid path: {path}`              |
| `{ty}`       | Type expression                            | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                     | `Internal error: {message}`         |

##### Arbitrary Key Support

**params support arbitrary keys, not limited to predefined ones**. The caller can pass any `key`:

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

> **Note**: Not all error codes use placeholders. Some error codes (such as E0001) are static
> messages that don't require parameters.

#### Language Priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. 默认值: en
```

### yaoxiang.toml Configuration

#### Project-level Configuration

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# Error message language, options: en, zh, ja, ...
default = "zh"
```

#### User-level Configuration

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### Compile-time Language Selection

```
1. 读取项目级 yaoxiang.toml 的 language.default
2. 若未配置，读取用户级 ~/.yaoxiang/yaoxiang.toml
3. 若都未配置，默认使用 "en"
4. 编译器根据选择的语言创建 I18nRegistry（一次）
5. 所有错误使用该 I18nRegistry 渲染消息
```

#### Key to Zero Table Lookup Overhead

**Rendering happens when compiling user projects, not at runtime.**

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

| Component                    | Responsibility                       | Render timing                |
| ---------------------------- | ------------------------------------ | ---------------------------- |
| `I18nRegistry`               | Provide templates and display text   | When compiling user projects |
| `DiagnosticBuilder.render()` | Template + parameters → final string | When compiling user projects |
| `Diagnostic.message`         | Rendered string                      | Stores the final result      |
| AOT binary                   | Contains final string                | Directly used at runtime     |

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

Error severity is managed through the `DiagnosticLevel` enum, decoupled from error code numbers:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation failure
    Warning,  // Does not affect compilation, but recommended to fix
    Note,     // Supplementary information
    Help,     // Fix suggestions
}
```

| Level   | Prefix            | Description                 |
| ------- | ----------------- | --------------------------- |
| Error   | `error[E####]:`   | Causes compilation failure  |
| Warning | `warning[E####]:` | Does not affect compilation |
| Note    | `note[E####]:`    | Supplementary information   |
| Help    | `help[E####]:`    | Fix suggestions             |

---

### `yaoxiang explain` command

#### Command Syntax

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Options

| Option          | Description                                    |
| --------------- | ---------------------------------------------- |
| `--lang <code>` | Specify language (en-US, zh-CN, default en-US) |
| `--json`        | JSON format output (for IDE/LSP use)           |
| `--json-pretty` | Formatted JSON output                          |
| `--examples`    | Show only example code                         |
| `--help`        | Show help information                          |

#### Usage Example

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

Since this RFC designs the error code system from scratch, there is no backward compatibility issue.

**Future migration strategy** (for reference in subsequent versions):

1. Maintain mapping from old error codes to new error codes
2. Display both old and new codes during migration
3. Provide a deprecation timeline

---

## Implementation Strategy

### Phase 1: Error Code Infrastructure

1. Create `src/diagnostics/` directory structure
2. Implement `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create resource file directory and example JSON

### Phase 2: explain command

1. Implement `yaoxiang explain` CLI command
2. Support `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase 3: Compile-time Integration

1. Update all error reporting points to use the new system
2. Implement message template parameter injection
3. Add language priority logic
4. Unit test coverage

### Phase 4: IDE/LSP Integration

1. LSP server integrates explain JSON output
2. Display error code links in IDE
3. Hover to show error explanation
4. Quick fix suggestions

---

## Appendix

### Complete Error Code Quick Reference

| Range | Category                    |
| ----- | --------------------------- |
| E0xxx | Lexical and Syntax Analysis |
| E1xxx | Type Checking               |
| E2xxx | Semantic Analysis           |
| E3xxx | Code Generation             |
| E4xxx | Generics and Traits         |
| E5xxx | Modules and Imports         |
| E6xxx | Runtime Errors              |
| E7xxx | I/O and System Errors       |
| E8xxx | Internal Compiler Errors    |
| E9xxx | Reserved                    |

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
          help: 你是否想要定义它？
```

## References

- [Rust Compiler Error Index](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC Error Message Format](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang Diagnostic Format](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
