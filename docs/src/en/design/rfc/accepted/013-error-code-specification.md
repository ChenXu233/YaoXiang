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

This RFC proposes a standardized error code classification specification for the YaoXiang compiler.
It adopts a Rust-like single-layer numbering system, paired with JSON resource files to support
multiple languages, and provides error explanation functionality through the `yaoxiang explain`
command.

## Motivation

### Why do we need standardized error codes?

1. **User Experience**: Users can quickly identify the error type and severity by seeing the error
   code.
2. **Documentation Organization**: Grouping by category makes it easier to write and maintain error
   reference documentation.
3. **Tool Integration**: IDEs/LSPs can provide quick-fix suggestions and documentation links based
   on error codes.
4. **Internationalization Support**: Separating error messages from codes facilitates multi-language
   translation.

### Design Goals

- **Concise**: Single-layer numbering; users don't need to remember complex classification rules.
- **Friendly**: Rust-like error message format with help information and examples.
- **Extensible**: Driven by resource files, easy to add new errors and new languages.
- **Tool-Friendly**: `explain` command + JSON output, supporting IDE/LSP integration.

---

## Proposal

### Core Design: Single-Layer Numbering System

Uses a four-digit numbering scheme, grouped by compilation phase:

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
| **4** | E4xxx | Generics and Trait          |
| **5** | E5xxx | Modules and Imports         |
| **6** | E6xxx | Runtime Errors              |
| **7** | E7xxx | I/O and System Errors       |
| **8** | E8xxx | Internal Compiler Errors    |
| **9** | E9xxx | Reserved/Experimental       |

### Error Category Enum

```rust
/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Lexical and syntax analysis
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: Type checking
    Semantic,   // E2xxx: Semantic analysis
    Generic,    // E4xxx: Generics and trait
    Module,     // E5xxx: Modules and imports
    Runtime,    // E6xxx: Runtime errors
    Io,         // E7xxx: I/O and system errors
    Internal,   // E8xxx: Internal compiler errors
}
```

### Error Code Definition and Generic Builder

**Core Principle**: Error code definition is separated from display text.

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without display text.
- `locales/*.json`: Display text for each language (title, message, help; error codes are nested
  objects).
- `DiagnosticBuilder`: Generic builder, replacing the trait-per-error design.

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

    /// Build a Diagnostic (template rendering completes at compile-time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Verify that all {key} in the template have corresponding parameters
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

| Feature                   | Description                                             |
| ------------------------- | ------------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` is generic for all error codes  |
| **Type Safety**           | Shortcut methods ensure parameter correctness           |
| **Self-Documenting**      | `E1001::unknown_variable(name)` is self-explanatory     |
| **Template Separation**   | Message templates separated from code, easy for i18n    |
| **Zero Runtime Overhead** | Compile-time rendering; AOT binary has no lookup tables |

---

### Error Macro Simplification

#### error! Macro (Auto-Inject Context)

```rust
/// Macro that automatically retrieves span and i18n configuration at compile-time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: only pass parameters; span and i18n are automatically injected
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
| E0018 | Keyword used as name      |

<!-- code-table:E0xxx end -->

#### E1xxx: Type Checking

<!-- code-table:E1xxx start -->

| Code  | Description                                                        |
| ----- | ------------------------------------------------------------------ |
| E1001 | Unknown variable                                                   |
| E1002 | Type mismatch                                                      |
| E1003 | Unknown type                                                       |
| E1010 | Argument count mismatch                                            |
| E1011 | Argument type mismatch                                             |
| E1012 | Return type mismatch                                               |
| E1013 | Function not found                                                 |
| E1014 | Unknown named argument                                             |
| E1015 | Duplicate argument specification                                   |
| E1020 | Cannot infer type                                                  |
| E1021 | Conflicting type inference                                         |
| E1030 | Incomplete pattern                                                 |
| E1031 | Unreachable pattern                                                |
| E1032 | Duplicate pattern binding                                          |
| E1033 | Inconsistent or-pattern bindings                                   |
| E1034 | Missing field in struct pattern                                    |
| E1040 | Operation not supported                                            |
| E1041 | Index out of bounds                                                |
| E1042 | Field not found                                                    |
| E1050 | Boolean operand required                                           |
| E1051 | Logical NOT requires a boolean operand                             |
| E1052 | Invalid dereference                                                |
| E1053 | Field access on non-struct                                         |
| E1054 | Conditional type mismatch                                          |
| E1055 | Constraint in non-generic context                                  |
| E1060 | Type parameter count mismatch                                      |
| E1061 | Cannot instantiate generic                                         |
| E1062 | const generic constraint failed                                    |
| E1064 | Invalid binding position index                                     |
| E1065 | Calling a non-function value                                       |
| E1071 | Type definitions only allowed at module level                      |
| E1081 | `?` is only allowed inside functions returning a propagatable type |
| E1082 | `?` can only be used on types implementing Try                     |
| E1083 | `?` error type mismatch                                            |
| E1090 | ✨ Unspeakable ✨                                                  |
| E1091 | Invalid generic meta type                                          |
| E1092 | Refined type argument form is illegal                              |
| E1093 | Refined argument count mismatch                                    |
| E1094 | Unused compile-time value parameter                                |
| E1095 | Unknown interface                                                  |
| E1096 | Interface parameter count mismatch                                 |
| E1097 | Interface member naming conflict                                   |
| E1098 | Interface method not implemented                                   |
| E1099 | Interface method signature mismatch                                |
| E1100 | Duplicate interface method implementation                          |
| E1101 | Type does not implement the interface                              |
| E1102 | Loop control statement appears outside a loop                      |
| E1103 | Brackets cannot be used at type position                           |
| E1104 | Interface implementation not in the type's definition module       |
| E1105 | Variant constructor cannot be accessed as a field                  |
| E1106 | Constraint not satisfied                                           |
| E1107 | Method overload ambiguity                                          |

<!-- code-table:E1xxx end -->

> **RFC-011b Reference (2026-09-22 note)**:
> [RFC-011b: Operator Overloading](./011b-operator-overloading.md) When landed, this will affect
> three places in this section — ① The text of `E1081` / `E1082` will drop the word "Result" (`?` is
> judged by the `Try` interface rather than bound to a specific type name), synced with the
> "Three-Way Consistency" process per this document (codes/*.rs ↔ locales ↔ code table) in phase 2;
> ② The `Equal` precondition constraint (linear token) rejection diagnostic reuses the `E1101`
> family (Type does not implement the interface); ③ After phase 1 wiring, `Struct == Struct`
> switches from `E6007` runtime error to compile-time judgement, causing `E6007` trigger surface to
> contract. The registered text in the table remains as-is until the implementation lands.

#### E2xxx: Semantic Analysis

<!-- code-table:E2xxx start -->

| Code  | Description                       |
| ----- | --------------------------------- |
| E2001 | Scope error                       |
| E2002 | Duplicate definition              |
| E2003 | Ownership error                   |
| E2010 | Immutable assignment              |
| E2011 | Use of uninitialized variable     |
| E2012 | Mutability conflict               |
| E2013 | Variable shadowing                |
| E2014 | Use of moved value                |
| E2016 | Immutable assignment              |
| E2018 | Mutable/immutable borrow conflict |
| E2019 | Double free                       |
| E2020 | Use after free                    |
| E2027 | unsafe dereference                |
| E2029 | Ref loop inside spawn             |
| E2030 | Refined type constraint violation |
| E2090 | Invalid signature                 |
| E2091 | Unknown type in signature         |
| E2092 | Missing arrow in signature        |
| E2093 | Duplicate parameter name          |
| E2094 | Generic parameter shadowing       |
| E2095 | Parameter name shadows generic    |

<!-- code-table:E2xxx end -->

> Reserved Code Notes (2026-09-14 review, #251 release statement): E2019 (Double free), E2020 (Use
> after free), E2027 (unsafe dereference), E2029 (ref loop inside spawn) have been registered with
> unit test anchors, but currently have no reachable yx source surface (explicit drop statements,
> Ptr dereference syntax, spawn ref loop construction paths) — the assertion of complete semantic
> correctness does not cover these four codes; they are treated as "reserved" until implementation
> is complete.

#### E3xxx: Code Generation

<!-- code-table:E3xxx start -->

| Code  | Description                                         |
| ----- | --------------------------------------------------- |
| E3004 | Unsupported iterator                                |
| E3005 | IR generation error                                 |
| E3006 | Unresolved variable                                 |
| E3007 | Top-level binding initialization must be a constant |
| E3008 | Unsupported match pattern                           |
| E3014 | Register overflow                                   |
| E3017 | Invalid operand (code generation)                   |
| E3018 | Monomorphization instantiation failure              |
| E3019 | Circular dependency in top-level bindings           |
| E3020 | Missing program entry point                         |
| E3021 | Entry point is not a function                       |
| E3022 | Entry main signature mismatch                       |
| E3023 | Executable statements not allowed at top level      |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and Trait

<!-- code-table:E4xxx start -->

| Code  | Description                                     |
| ----- | ----------------------------------------------- |
| E4001 | Generic constraint violation                    |
| E4002 | Trait not found                                 |
| E4003 | Missing trait implementation                    |
| E4004 | Conflicting trait implementation                |
| E4005 | Associated type not found                       |
| E4010 | Division by zero (constant)                     |
| E4011 | Constant overflow                               |
| E4012 | Constant recursion too deep                     |
| E4014 | Constant evaluation failed                      |
| E4018 | Refinement predicate violation                  |
| E4019 | Type equality does not hold                     |
| E4020 | Proof function required                         |
| E4021 | Loop termination cannot be automatically proven |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission points (reserved codes): Sized constraint and optimization
> error paths are pending implementation; wire them to real trigger surfaces when implemented.

#### E5xxx: Modules and Imports

<!-- code-table:E5xxx start -->

| Code  | Description         |
| ----- | ------------------- |
| E5001 | Module not found    |
| E5002 | Import error        |
| E5003 | Export not found    |
| E5004 | Circular dependency |
| E5005 | Invalid module path |
| E5006 | Duplicate import    |
| E5007 | Module export       |

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
| E6010 | Integer parse failed         |
| E6011 | Float parse failed           |

<!-- code-table:E6xxx end -->

> **Code Table Revision (2026-08-09)**: The code table was originally defined per Rust semantic
> draft (Assertion failed / Arithmetic overflow / Heap allocation failed / Type cast failed), which
> did not match actual implementation needs. YaoXiang has no null pointer / heap allocation failure
> / type cast concepts (value semantics + Rust memory safety), and the runtime overflow path has no
> detection implemented. After calibration:
>
> - E6002 deleted (original Assertion failed moved to E6005; original null pointer semantics has no
>   language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (real trigger surface)
> - E6005 changed from Heap allocation failed to Assertion failed (std.assert real path)
> - E6006 changed from Runtime index out of bounds to Function not found (implementation has been
>   this way)
> - E6007 changed from Type cast failed to generic Runtime error (ExecutorError unmapped variants
>   unified landing point)

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
| E8002 | Unexpected Panic        |
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
| W1063 | const generic constraint cannot be evaluated |
| W1080 | Compile-time proof degraded                  |

<!-- code-table:W1xxx end -->

> W code rules: isomorphic to E codes, grouped by phase (W + phase thousands digit), W1xxx = type
> checking phase warnings.
>
> **Dead code semantics (#321 decision B)**: `pub` definitions are external interfaces and are never
> reported — whether external consumers use them is beyond single-file analysis boundaries, so we'd
> rather stay silent than false-positive. W1001/W1002/W1004/W1005 only target **private (non-`pub`)
> definitions**: if never referenced in reachability analysis starting from `main` and `pub`
> definitions, report it. Methods (W1005) are matched by short name at call sites. bin/lib target
> semantics (unused pub in bin triggers warning) is a future extension (#289 plan A), requiring
> project model support.
>
> **Unused imports (W1003)**: Detected by typecheck's use elaboration (pass 2 records imported local
> names, hit positions in expression resolution and type annotations count as used), covering both
> whole imports (`use std.io` → module alias) and named imports (`use std.io.{print}`).
>
> **Emission channel**: W code diagnostics are labeled `Severity::Warning` by default by the builder
> per W prefix (explicit specification takes priority), collected and rendered on the same track as
> errors (`warning[W####]` prefix rendering), but do not block compilation and do not affect
> successful exit code. `yaoxiang check --deny-warnings` escalates warnings to failure (non-zero
> exit code when warnings exist), for strict CI mode. Per-code suppression (allow attributes, etc.)
> is a future extension.

### Message Quality Specification

> This section is introduced by the message single-track and quality revision (2026-09-03). Enforced
> in CI by `scripts/audit_diagnostics.py`.

1. **Message single-track**: All user-visible diagnostic messages must go through the authoritative
   registry shortcut methods + locales template rendering; code only passes structured parameters.
   Bypassing the registry to directly construct native values like `Diagnostic::error(...)` is
   prohibited — that path bypasses code validation and i18n.
2. **Code validity**: Using unregistered codes and pseudo-codes (such as `E_INTERNAL`) is
   prohibited; code literals at usage points must already be defined in the registry. Internal
   errors uniformly land at E8001 (`internal_error`).
3. **Type display**: Type Display must distinguish pre- and post-instantiation forms
   (`Expected 'Container', found 'Container'` bare names are indistinguishable).
4. **Solver internal state isolation**: Solver intermediate state TypeVar (Display form `t<N>`) must
   not enter user-visible messages. Test anchor: `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is only used for compiler internal consistency issues (ICE).
   User-fixable errors are prohibited from using E8001 as a fallback; ICE messages must include a
   minimal reproduction guide.

---

### Runtime Error Values and Code Connectivity

> This section is introduced by the runtime Error value with code revision (2026-09-03). E6xxx/E7xxx
> semantic space carries two channels simultaneously; code space is shared, presentation channels
> differ.

#### Two Channels

| Channel                         | Carrier                                          | Presentation                                             |
| ------------------------------- | ------------------------------------------------ | -------------------------------------------------------- |
| Compiler/CLI diagnostic channel | `ExecutorError` and other host-level hard errors | stderr `error[E####]:` (E6003/E6005/E6007 already wired) |
| In-program error value channel  | std lib `Result(T, Error)` Err carrier `Error`   | Language value, consumed by program match/compare        |

#### Error Structure (from v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses E6xxx/E7xxx numbers in this specification, in string form (e.g. `"E6008"`).
- **Stability contract**: Assigned codes have unchanged semantics across versions; the same
  semantics does not reuse deleted codes (E6002 precedent).
- **Consumption surface**: In-program `e.code == "E6xxx"` comparison is the only programmable
  judgement contract; `yaoxiang explain E6xxx` documentation is connected; tooling (LSP / DAP, see
  RFC-034) uses code as exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: The E in `Result(T, E)` is a generic parameter; serious modeling goes
  through user-defined types; std `Error` is only a convenient fallback carrier, its code system
  does not constrain user E types.

#### Code Allocation Rules

1. Runtime error value codes and compiler diagnostic codes share the E6xxx/E7xxx space; new codes
   are allocated per **real trigger surface**, not reserved for imagined scenarios.
2. Register before use: new codes must enter the authoritative registry and pass the three-way
   consistency check (codes/*.rs ↔ locales ↔ this document's code table) before they can be emitted.
   The registration source for runtime error value codes is the `RUNTIME_ERROR_CODES` table in
   `src/std/result.rs` (subject to the same `build.rs` compile-time threshold + `tools/code-tables`
   validation as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error values (currently empty, to be enabled when io/net
   are Result-ified).
4. Emission point: std modules construct Error values via `error_new(code, message)`; the
   consumption side extracts Err carrier via `std.result.unwrap_err`, and reads fields via
   `std.result.code/message`.

#### Evolution Path (Line C, not implemented)

After pattern matching completeness (RFC-010b) is landed, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming an attribute derived from kind (variant
definition sites become the code registry). During evolution, the code stability contract in this
section remains unchanged; this upgrade is a separate decision and does not constitute a commitment
of this section.

---

### Multi-Language Resource Files

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
// locales/*.json (error code object)

/// i18n display text registry (loaded from JSON at compile-time, zero lookup at runtime)
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
    /// Get a registry based on language code
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

| Placeholder  | Purpose                                        | Example                             |
| ------------ | ---------------------------------------------- | ----------------------------------- |
| `{name}`     | Variable/type/trait name and other identifiers | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                                  | `Expected type '{expected}'`        |
| `{found}`    | Actual/found type                              | `, found type '{found}'`            |
| `{method}`   | Method name                                    | `Method {method} is not a function` |
| `{trait}`    | Trait name                                     | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                    | `Invalid path: {path}`              |
| `{ty}`       | Type expression                                | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                         | `Internal error: {message}`         |

##### Arbitrary Key Support

**params support any key, not limited to predefined ones**. Callers can pass any `key`:

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

> **Note**: Not all error codes use placeholders. Some error codes (e.g. E0001) are static messages
> without parameters.

#### Language Priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. Default value: en
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
1. Read language.default from project-level yaoxiang.toml
2. If not configured, read from user-level ~/.yaoxiang/yaoxiang.toml
3. If neither is configured, default to "en"
4. The compiler creates I18nRegistry (once) based on the selected language
5. All errors use that I18nRegistry to render messages
```

#### Key to Zero Lookup Overhead

**Rendering happens when compiling user projects, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Rust compiles the YaoXiang compiler                          │
│                                                                           │
│  JSON is packed into the compiler binary                                │
│  Purpose: the explain command can directly read i18n data               │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: YaoXiang compiles user project (rendering happens here)       │
│                                                                           │
│  When error! macro is called:                                            │
│  1. Read yaoxiang.toml to get language preference                       │
│  2. Load i18n JSON for the corresponding language from compiler binary  │
│  3. Template + params → render() → "Unknown variable: 'x'"             │
│  4. Diagnostic.message = rendered string                                │
│                                                                           │
│  AOT binary directly stores final string, no template, no lookup        │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runtime                                           │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Directly outputs the final string, no lookup                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                     | Render Time                 |
| ---------------------------- | ---------------------------------- | --------------------------- |
| `I18nRegistry`               | Provide templates and display text | When compiling user project |
| `DiagnosticBuilder.render()` | Template + params → final string   | When compiling user project |
| `Diagnostic.message`         | Rendered string                    | Stores final result         |
| AOT binary                   | Contains final string              | Used directly at runtime    |

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

Error severity is managed through the `DiagnosticLevel` enum, decoupled from error code numbering:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation to fail
    Warning,  // Does not affect compilation, but recommended to fix
    Note,     // Supplementary information
    Help,     // Fix suggestions
}
```

| Level   | Prefix            | Description                 |
| ------- | ----------------- | --------------------------- |
| Error   | `error[E####]:`   | Causes compilation to fail  |
| Warning | `warning[E####]:` | Does not affect compilation |
| Note    | `note[E####]:`    | Supplementary information   |
| Help    | `help[E####]:`    | Fix suggestions             |

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
| `--json-pretty` | Pretty-formatted JSON output                   |
| `--examples`    | Only show example code                         |
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

Since this RFC designs the error code system from scratch, there is no backward compatibility issue.

**Future migration strategy** (for later versions):

1. Maintain mapping from old error codes to new error codes.
2. Display both old and new codes during migration.
3. Provide a deprecation timeline.

---

## Implementation Strategy

### Phase One: Error Code Infrastructure

1. Create `src/diagnostics/` directory structure.
2. Implement `ErrorCode` enum.
3. Implement `Diagnostic` and `DiagnosticLevel`.
4. Create resource file directory and example JSON.

### Phase Two: explain Command

1. Implement `yaoxiang explain` CLI command.
2. Support `--lang` and `--json` options.
3. Integrate resource file loading.
4. Implement parameter template rendering.

### Phase Three: Compile-Time Integration

1. Update all error reporting points to use the new system.
2. Implement message template parameter injection.
3. Add language priority logic.
4. Unit test coverage.

### Phase Four: IDE/LSP Integration

1. LSP server integrates explain JSON output.
2. Display error code links in IDE.
3. Show error explanation on hover.
4. Quick-fix suggestions.

---

## Appendix

### Complete Error Code Quick Reference

| Range | Category                    |
| ----- | --------------------------- |
| E0xxx | Lexical and Syntax Analysis |
| E1xxx | Type Checking               |
| E2xxx | Semantic Analysis           |
| E3xxx | Code Generation             |
| E4xxx | Generics and Trait          |
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
          帮助: 你是否想要定义它？
```

## References

- [Rust Compiler Error Index](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC Error Message Format](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang Diagnostic Format](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
