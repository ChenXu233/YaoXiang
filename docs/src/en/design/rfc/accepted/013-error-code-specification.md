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

This RFC proposes an error code classification specification for the YaoXiang compiler. It adopts a
single-level numbering system similar to Rust, paired with JSON resource files to provide
multi-language support, and offers error explanation via the `yaoxiang explain` command.

## Motivation

### Why Do We Need Standardized Error Codes?

1. **User Experience**: Users can quickly determine the error type and severity from the error code.
2. **Documentation Organization**: Grouping by category makes it easier to author and maintain the
   error reference documentation.
3. **Tool Integration**: IDEs/LSP can provide quick-fix suggestions and documentation links based on
   error codes.
4. **Internationalization Support**: Separating error messages from codes makes multi-language
   translation easier.

### Design Goals

- **Concise**: Single-level numbering, no need for users to memorize complex classification rules.
- **Friendly**: Error message format similar to Rust, with help text and examples.
- **Extensible**: Resource-file driven, easy to add new errors and new languages.
- **Tool-friendly**: explain command + JSON output, supports IDE/LSP integration.

---

## Proposal

### Core Design: Single-Level Numbering System

A four-digit numbering system, grouped by compilation phase:

```
Exxxx
││││
│││└── Sequence number (000-999)
││└─── Compilation phase (0-9)
└───── Fixed prefix 'E'
```

### Phase Partitioning

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

### Error Code Definition and Common Builder

**Core Principle**: Error code definitions are separated from display copy.

- `ErrorCodeDefinition`: error code metadata (code, category, template), without display copy.
- `locales/*.json`: per-language display copy (title, message, help; error codes as nested objects).
- `DiagnosticBuilder`: a common builder, replacing the trait-per-error design.

#### Error Code Definition

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only; display copy is in i18n files)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // message template, supports {param} placeholders
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

    /// Set location
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Build a Diagnostic (template rendering completes at compile-time)
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

#### Shortcut Method per Error Code

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

| Feature                   | Description                                              |
| ------------------------- | -------------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` for all error codes              |
| **Type Safety**           | Shortcut methods ensure parameter correctness            |
| **Self-Documenting**      | `E1001::unknown_variable(name)` is self-explanatory      |
| **Template Separation**   | Message templates are separated from code, easy for i18n |
| **Zero Runtime Overhead** | Compile-time rendering, AOT binary has no lookup tables  |

---

### Error Macro Simplification

#### error! Macro (Auto-Injected Context)

```rust
/// A macro that automatically obtains span and i18n configuration at compile-time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: just pass parameters, span and i18n are auto-injected
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Manual Builder Usage

```rust
// When manual control is needed
E1001::unknown_variable(&var_name)
    .at(my_span)           // custom span
    .build(&custom_i18n)   // custom i18n
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

| Code  | Description                                    |
| ----- | ---------------------------------------------- |
| E1001 | Unknown variable                               |
| E1002 | Type mismatch                                  |
| E1003 | Unknown type                                   |
| E1010 | Argument count mismatch                        |
| E1011 | Argument type mismatch                         |
| E1012 | Return type mismatch                           |
| E1013 | Function not found                             |
| E1014 | Unknown named argument                         |
| E1015 | Duplicate argument                             |
| E1020 | Cannot infer type                              |
| E1021 | Conflicting type inference                     |
| E1030 | Incomplete pattern                             |
| E1031 | Unreachable pattern                            |
| E1040 | Operation not supported                        |
| E1041 | Index out of bounds                            |
| E1042 | Field not found                                |
| E1050 | Boolean operand required                       |
| E1051 | Logical NOT requires a boolean operand         |
| E1052 | Invalid dereference                            |
| E1053 | Field access on non-struct                     |
| E1054 | Condition type mismatch                        |
| E1055 | Constraint in non-generic context              |
| E1060 | Type parameter count mismatch                  |
| E1061 | Cannot instantiate generic                     |
| E1062 | const generics constraint failure              |
| E1064 | Invalid binding-position index                 |
| E1065 | Call on non-function value                     |
| E1071 | Type definition allowed only at module level   |
| E1081 | `?` allowed only in functions returning Result |
| E1082 | `?` can only be used on a Result expression    |
| E1083 | `?` error type mismatch                        |
| E1090 | ✨ Unspeakable ✨                              |
| E1091 | Invalid generic meta type                      |
| E1092 | Refinement type argument shape illegal         |
| E1093 | Refinement argument count mismatch             |
| E1094 | Unused compile-time value parameter            |
| E1095 | Unknown interface                              |
| E1096 | Interface parameter count mismatch             |
| E1097 | Interface member name conflict                 |
| E1098 | Interface method not implemented               |
| E1099 | Interface method signature mismatch            |
| E1100 | Duplicate interface method implementation      |
| E1101 | Type does not implement interface              |
| E1102 | Loop control statement outside loop            |
| E1103 | Square brackets not allowed at type position   |
| E1104 | Interface impl not in type's defining module   |
| E1105 | Variant constructor not accessible as field    |

<!-- code-table:E1xxx end -->

> **RFC-011b related (note dated 2026-09-22)**:
> [RFC-011b: Operator Overloading](./011b-operator-overloading.md) When implemented, three places in
> this section will be affected — ① the wording of `E1081` / `E1082` drops the word "Result" (`?`
> switches to be judged by the `Try` interface, no longer bound to a specific type name), and
> synchronizes per this document's "Three-Party Consistency" process in phase 2 (codes/*.rs ↔
> locales ↔ code table); ② the rejection diagnostic when the `Equal` pre-constraint (linear token)
> is not satisfied reuses the `E1001` family (type does not implement interface); ③ after phase 1
> wiring, `Struct == Struct` changes from an `E6007` runtime error to a compile-time judgment,
> narrowing the trigger surface of `E6007`. The registered copy in the table remains as-is until
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
| E2029 | Reference cycle in spawn            |
| E2030 | Refinement type constraint violated |
| E2090 | Invalid signature                   |
| E2091 | Unknown type in signature           |
| E2092 | Missing arrow in signature          |
| E2093 | Duplicate parameter name            |
| E2094 | Generic parameter shadowed          |
| E2095 | Parameter name shadows generic      |

<!-- code-table:E2xxx end -->

> Reserved code note (2026-09-14 inventory, #251 release baseline): E2019 (double free), E2020 (use
> after free), E2027 (unsafe dereference), and E2029 (ref cycle in spawn) have been registered and
> anchored by unit tests, but no yx source-level surface (explicit drop statements, Ptr dereference
> syntax, or ref cycle paths in spawn) is currently triggerable — the claim of full semantic
> correctness does not cover these four codes, which are treated as "reserved" until implementation
> is complete.

#### E3xxx: Code Generation

<!-- code-table:E3xxx start -->

| Code  | Description                                      |
| ----- | ------------------------------------------------ |
| E3004 | Unsupported iterator                             |
| E3005 | IR generation error                              |
| E3006 | Unresolved variable                              |
| E3007 | Top-level binding initializer must be a constant |
| E3008 | Unsupported match pattern                        |
| E3014 | Register overflow                                |
| E3017 | Invalid operand (codegen)                        |
| E3018 | Monomorphization instantiation failure           |
| E3019 | Top-level binding cyclic dependency              |
| E3020 | Missing program entry                            |
| E3021 | Entry is not a function                          |
| E3022 | Entry main signature mismatch                    |
| E3023 | Executable statements not allowed at top level   |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and Traits

<!-- code-table:E4xxx start -->

| Code  | Description                      |
| ----- | -------------------------------- |
| E4001 | Trait constraint violated        |
| E4002 | Trait not found                  |
| E4003 | Missing trait implementation     |
| E4004 | Conflicting trait implementation |
| E4005 | Associated type not found        |
| E4010 | Constant division by zero        |
| E4011 | Constant overflow                |
| E4012 | Constant recursion too deep      |
| E4014 | Constant evaluation failed       |
| E4018 | Refinement predicate violated    |
| E4019 | Type equality does not hold      |
| E4020 | Proof function required          |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission points (reserved codes): Sized constraint and optimization
> error path are pending implementation; when implemented, they will be wired to the actual trigger
> surface.

#### E5xxx: Modules and Imports

<!-- code-table:E5xxx start -->

| Code  | Description         |
| ----- | ------------------- |
| E5001 | Module not found    |
| E5002 | Import error        |
| E5003 | Export not found    |
| E5004 | Cyclic dependency   |
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
| E6009 | Range step is illegal        |
| E6010 | Integer parse failure        |
| E6011 | Float parse failure          |

<!-- code-table:E6xxx end -->

> **Code table revision (2026-08-09)**: The code table was originally defined following a Rust
> semantic draft (Assertion failed / Arithmetic overflow / Heap allocation failed / Type cast
> failed), which did not match actual implementation needs. YaoXiang has no concept of null pointer
> / heap allocation failure / type cast (value semantics + Rust memory safety), and the runtime
> overflow detection path is unimplemented. After calibration:
>
> - E6002 removed (the original Assertion failed moved to E6005; the original null-pointer semantics
>   has no language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (the actual trigger
>   surface)
> - E6005 changed from Heap allocation failed to Assertion failed (the actual path of std.assert)
> - E6006 changed from Runtime index out of bounds to Function not found (implementation has long
>   been this way)
> - E6007 changed from Type cast failed to generic Runtime error (unified landing point for unmapped
>   variants of ExecutorError)

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

| Code  | Description                                   |
| ----- | --------------------------------------------- |
| W1001 | Unused private function                       |
| W1002 | Unused private type                           |
| W1003 | Unused import                                 |
| W1004 | Unused private variable                       |
| W1005 | Unused private method                         |
| W1063 | const generics constraint cannot be evaluated |
| W1080 | Compile-time proof downgraded                 |

<!-- code-table:W1xxx end -->

> W-code slot rule: isomorphic to E-codes, grouped by phase (W + phase-thousands segment); W1xxx =
> type-checking phase warnings.
>
> **Dead-code semantics (decision B from #321)**: `pub` definitions are an external interface and
> are never reported — whether an external consumer uses them is beyond single-file analysis scope,
> so it is better to remain silent than to false-report. W1001 / W1002 / W1004 / W1005 only target
> **private (non-`pub`) definitions**: a definition that is never referenced in the reachability
> analysis starting from `main` and `pub` definitions is reported. Methods (W1005) are matched by
> short name at the call site. bin/lib target semantics (warning on unused pub inside bin) is a
> future extension (option A of #289), requiring project model support.
>
> **Unused import (W1003)**: detected by use elaboration in typecheck (pass 2 records the imported
> local name; an expression parse or type annotation site hitting the name counts as used); covers
> whole imports (`use std.io` → module alias) and named imports (`use std.io.{print}`).
>
> **Emission channel**: W-code diagnostics are marked with `Severity::Warning` by default by the
> builder according to the W prefix (explicit specification takes precedence). Collection and
> presentation follow the same track as errors (rendered with a `warning[W####]` prefix), but they
> do not block compilation and do not affect the success exit code. `yaoxiang check --deny-warnings`
> escalates warnings to failures (exits with a non-zero code when warnings are present), for strict
> CI mode. Per-code suppression (allow attributes, etc.) is left for future extension.

### Message Quality Specification

> This section is introduced by the message single-track and quality revision (2026-09-03). It is
> enforced in CI by `scripts/audit_diagnostics.py`.

1. **Message single-track**: All user-visible diagnostic messages must be rendered through the
   authoritative registry's shortcut method + locales template; the code only passes structured
   parameters. Bypassing the registry to directly construct raw values such as
   `Diagnostic::error(...)` is forbidden — this path bypasses code validation and i18n.
2. **Code validity**: Using unregistered codes and pseudo-codes (e.g., `E_INTERNAL`) is forbidden;
   code literals at use sites must already be defined in the registry. Internal errors always land
   on E8001 (`internal_error`).
3. **Type display**: A type's Display must distinguish pre- and post-instantiation form
   (`Expected 'Container', found 'Container'` bare names are indistinguishable).
4. **Solver internal-state isolation**: Intermediate TypeVar of the solver (Display form `t<N>`)
   must not appear in user-visible messages. Test anchor:
   `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is reserved for compiler internal consistency issues (ICE).
   User-fixable errors must not fall back to E8001; ICE messages must include a minimal reproduction
   guide.

---

### Runtime Error Value and Code Unification

> This section is introduced by the runtime Error value with code revision (2026-09-03). The E6xxx /
> E7xxx semantic space carries two channels; the code space is shared while the presentation channel
> differs.

#### Two Channels

| Channel                           | Carrier                                                   | Presentation                                                 |
| --------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------ |
| Compiler / CLI diagnostic channel | Hard errors at the host layer such as `ExecutorError`     | stderr `error[E####]:` (E6003 / E6005 / E6007 already wired) |
| In-program error value channel    | The `Error` carried by Err in `Result(T, Error)` from std | Language value, consumed by program match / comparison       |

#### Error Structure (from v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses E6xxx / E7xxx numbers from this spec, in string form (e.g., `"E6008"`).
- **Stability contract**: assigned codes retain their semantics across versions; the same semantics
  must not reuse a deleted code (E6002 is the precedent).
- **Consumption surface**: in-program `e.code == "E6xxx"` comparison is the only programmable
  judgment contract; `yaoxiang explain E6xxx` provides the documentation; the toolchain (LSP / DAP,
  see RFC-034) uses the code as exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: the `E` in `Result(T, E)` is a generic parameter; for serious modeling,
  use a user-defined type. The std `Error` is only a convenient fallback carrier, and its code
  system does not constrain user `E` types.

#### Code Allocation Rules

1. Runtime error value codes share the E6xxx / E7xxx space with compiler diagnostic codes; new codes
   are allocated by **actual trigger surface**, not reserved for imagined scenarios.
2. Register-before-use: a new code must enter the authoritative registry and pass three-party
   consistency check (codes/*.rs ↔ locales ↔ this document's code table) before it can be emitted.
   The registration source for runtime error value codes is the `RUNTIME_ERROR_CODES` table in
   `src/std/result.rs` (subject to the same `build.rs` build-time gate + `tools/code-tables`
   validation as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error value slots (currently empty, to be enabled when io
   / net is Result-ified).
4. Emission point: each std module constructs an Error value via `error_new(code, message)`; on the
   consumption side, `std.result.unwrap_err` extracts the Err carrier, and `std.result.code/message`
   reads the fields.

#### Evolution Path (line C, not implemented)

After pattern-matching completeness (RFC-010b) lands, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming an attribute derived from kind (the
variant definition site becomes the code registry). The stable contract in this section remains
unchanged during the evolution period; that upgrade is an independent decision and does not
constitute a commitment in this section.

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

/// i18n display copy registry (loaded from JSON at compile-time, zero lookup at runtime)
pub struct I18nRegistry {
    /// Titles
    titles: HashMap<&'static str, &'static str>,
    /// Descriptions
    messages: HashMap<&'static str, &'static str>,
    /// Help text
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

    /// Render a template (completes at compile-time, zero-overhead at runtime)
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

| Placeholder  | Purpose                                                   | Example                             |
| ------------ | --------------------------------------------------------- | ----------------------------------- |
| `{name}`     | Identifier such as variable name / type name / trait name | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                                             | `Expected type '{expected}'`        |
| `{found}`    | Actual / found type                                       | `, found type '{found}'`            |
| `{method}`   | Method name                                               | `Method {method} is not a function` |
| `{trait}`    | Trait name                                                | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                               | `Invalid path: {path}`              |
| `{ty}`       | Type expression                                           | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                                    | `Internal error: {message}`         |

##### Arbitrary Key Support

**params supports arbitrary keys, not limited to predefined ones.** The caller can pass any `key`:

```rust
// Using arbitrary keys
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// Template definition
"Unknown variable: '{name}' at {location}. {hint}"
```

> **Note**: Not all error codes use placeholders. Some error codes (e.g., E0001) are static messages
> with no parameters.

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
1. Read project-level yaoxiang.toml's language.default
2. If not configured, read user-level ~/.yaoxiang/yaoxiang.toml
3. If neither is configured, default to "en"
4. The compiler creates an I18nRegistry based on the selected language (once)
5. All errors use that I18nRegistry to render messages
```

#### The Key to Zero-Lookup Overhead

**Rendering happens when compiling the user project, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Rust compiles the YaoXiang compiler                                      │
│                                                                           │
│  JSON is packed into the compiler binary                                                 │
│  Purpose: the explain command can read i18n data directly                                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: YaoXiang compiles the user project (rendering happens here)                          │
│                                                                           │
│  On an error! macro call:                                                       │
│  1. Read yaoxiang.toml to get the language preference                                      │
│  2. Load the corresponding language's i18n JSON from the compiler binary                                │
│  3. template + params → render() → "Unknown variable: 'x'"                    │
│  4. Diagnostic.message = rendered string                                   │
│                                                                           │
│  The AOT binary stores the final string directly, no template, no lookup tables                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runtime                                                  │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Outputs the final string directly, with no lookup                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                      | Rendering time                  |
| ---------------------------- | ----------------------------------- | ------------------------------- |
| `I18nRegistry`               | Provides templates and display copy | When compiling the user project |
| `DiagnosticBuilder.render()` | template + params → final string    | When compiling the user project |
| `Diagnostic.message`         | The rendered string                 | Stores the final result         |
| AOT binary                   | Contains the final string           | Directly used at runtime        |

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

Error severity is managed by the `DiagnosticLevel` enumeration, decoupled from the error code
number:

```rust
pub enum DiagnosticLevel {
    Error,    // causes compilation to fail
    Warning,  // does not affect compilation, but should be fixed
    Note,     // supplementary information
    Help,     // fix suggestion
}
```

| Level   | Prefix            | Description                 |
| ------- | ----------------- | --------------------------- |
| Error   | `error[E####]:`   | Causes compilation failure  |
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
| `--lang <code>` | Specify language (en-US, zh-CN; default en-US) |
| `--json`        | JSON format output (for IDE/LSP)               |
| `--json-pretty` | Formatted JSON output                          |
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

**Future migration strategy** (for later versions):

1. Maintain a mapping from old error codes to new ones
2. Display both old and new codes during the migration period
3. Provide a deprecation schedule

---

## Implementation Strategy

### Phase 1: Error Code Infrastructure

1. Create the `src/diagnostics/` directory structure
2. Implement the `ErrorCode` enumeration
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create the resource file directory and sample JSON

### Phase 2: explain Command

1. Implement the `yaoxiang explain` CLI command
2. Support `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase 3: Compile-Time Integration

1. Update all error report sites to use the new system
2. Implement message template parameter injection
3. Add language priority logic
4. Unit test coverage

### Phase 4: IDE/LSP Integration

1. LSP server integrates the explain JSON output
2. Show error code links in the IDE
3. Show error explanations on hover
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
