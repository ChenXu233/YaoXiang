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

This RFC proposes an error code classification specification for the YaoXiang compiler, adopting a
single-layer numbering system similar to Rust's, paired with JSON resource files to enable
multi-language support, and providing error explanation via the `yaoxiang explain` command.

## Motivation

### Why do we need standardized error codes?

1. **User Experience**: Users can quickly identify the error type and severity when seeing an error
   code
2. **Documentation Organization**: Grouping by category facilitates writing and maintaining error
   reference documentation
3. **Tooling Integration**: IDEs/LSPs can provide quick-fix suggestions and documentation links
   based on error codes
4. **Internationalization Support**: Separating error messages from codes facilitates multi-language
   translation

### Design Goals

- **Concise**: Single-layer numbering, no need to memorize complex classification rules
- **Friendly**: Similar to Rust's error message format, with help information and examples
- **Extensible**: Resource-file driven, easy to add new errors and new languages
- **Tool-Friendly**: `explain` command + JSON output, supporting IDE/LSP integration

---

## Proposal

### Core Design: Single-Layer Numbering System

Adopt a four-digit numbering, grouped by compilation phase:

```
Exxxx
││││
│││└── Serial number (000-999)
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
    Lexer,      // E0xxx: 词法和语法分析
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: 类型检查
    Semantic,   // E2xxx: 语义分析
    Generic,    // E4xxx: 泛型与特质
    Module,     // E5xxx: 模块与导入
    Runtime,    // E6xxx: 运行时错误
    Io,         // E7xxx: I/O与系统错误
    Internal,   // E8xxx: 内部编译器错误
}
```

### Error Code Definitions and Generic Builder

**Core principle**: Separation of error code definitions and display strings

- `ErrorCodeDefinition`: error code metadata (code, category, template), no display strings
- `locales/*.json`: per-language display strings (title, message, help, error codes as nested
  objects)
- `DiagnosticBuilder`: a generic builder, replacing the trait-per-error design

#### Error Code Definition

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only; display strings in i18n files)
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

    /// Build a Diagnostic (template rendering completed at compile time)
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

#### Usage Examples

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

| Feature                   | Description                                         |
| ------------------------- | --------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` works for all error codes   |
| **Type Safety**           | Convenience methods ensure parameter correctness    |
| **Self-Documenting**      | `E1001::unknown_variable(name)` is self-explanatory |
| **Template Separation**   | Message templates separated from code, easy i18n    |
| **Zero Runtime Overhead** | Rendered at compile time, no lookup in AOT binary   |

---

### Error Macro Simplification

#### `error!` macro (automatically injects context)

```rust
/// Macro that automatically fetches span and i18n config at compile time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: only parameters needed; span and i18n injected automatically
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Manual Use of Builder

```rust
// When manual control is needed
E1001::unknown_variable(&var_name)
    .at(my_span)           // custom span
    .build(&custom_i18n)   // custom i18n
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

| Code  | Description                                        |
| ----- | -------------------------------------------------- |
| E1001 | Unknown variable                                   |
| E1002 | Type mismatch                                      |
| E1003 | Unknown type                                       |
| E1010 | Argument count mismatch                            |
| E1011 | Argument type mismatch                             |
| E1012 | Return type mismatch                               |
| E1013 | Function not found                                 |
| E1020 | Cannot infer type                                  |
| E1021 | Type inference conflict                            |
| E1030 | Incomplete pattern                                 |
| E1031 | Unreachable pattern                                |
| E1040 | Operation not supported                            |
| E1041 | Index out of bounds                                |
| E1042 | Field not found                                    |
| E1050 | Boolean operand required                           |
| E1051 | Logical NOT requires a boolean operand             |
| E1052 | Invalid dereference                                |
| E1053 | Non-struct field access                            |
| E1054 | Conditional type mismatch                          |
| E1055 | Constraint in non-generic context                  |
| E1060 | Type argument count mismatch                       |
| E1061 | Cannot instantiate generic                         |
| E1062 | const generic constraint failure                   |
| E1064 | Invalid index at binding position                  |
| E1071 | Type definition only allowed at module level       |
| E1081 | `?` only allowed inside functions returning Result |
| E1082 | `?` can only be used on Result expressions         |
| E1083 | `?` error type mismatch                            |
| E1090 | ✨ Unspeakable ✨                                  |
| E1091 | Invalid generic meta type                          |
| E1092 | Invalid refinement type argument form              |
| E1093 | Refinement argument count mismatch                 |
| E1094 | Unused compile-time value parameter                |
| E1095 | Unknown interface                                  |
| E1096 | Interface parameter count mismatch                 |
| E1097 | Interface member name conflict                     |
| E1098 | Interface method not implemented                   |
| E1099 | Interface method signature mismatch                |
| E1100 | Interface method duplicate implementation          |
| E1101 | Type does not implement interface                  |
| E1102 | Loop control statement outside loop                |

<!-- code-table:E1xxx end -->

#### E2xxx: Semantic Analysis

<!-- code-table:E2xxx start -->

| Code  | Description                          |
| ----- | ------------------------------------ |
| E2001 | Scope error                          |
| E2002 | Duplicate definition                 |
| E2003 | Ownership error                      |
| E2010 | Immutable assignment                 |
| E2011 | Use of uninitialized variable        |
| E2012 | Mutability conflict                  |
| E2013 | Variable shadowing                   |
| E2014 | Use of moved value                   |
| E2016 | Immutable assignment                 |
| E2018 | Mutable/immutable borrow conflict    |
| E2019 | Double free                          |
| E2020 | Use after free                       |
| E2027 | unsafe dereference                   |
| E2029 | Reference loop inside spawn          |
| E2030 | Refinement type constraint violation |
| E2090 | Invalid signature                    |
| E2091 | Signature has unknown type           |
| E2092 | Signature missing arrow              |
| E2093 | Duplicate parameter name             |
| E2094 | Generic parameter shadowing          |
| E2095 | Parameter name shadows generic       |

<!-- code-table:E2xxx end -->

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
| E3017 | Invalid operand (code generation)                |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and Trait

<!-- code-table:E4xxx start -->

| Code  | Description                      |
| ----- | -------------------------------- |
| E4001 | Generic constraint violation     |
| E4002 | Trait not found                  |
| E4003 | Missing trait implementation     |
| E4004 | Conflicting trait implementation |
| E4005 | Associated type not found        |
| E4010 | Constant division by zero        |
| E4011 | Constant overflow                |
| E4012 | Constant recursion too deep      |
| E4014 | Constant evaluation failed       |
| E4018 | Refinement predicate violation   |
| E4019 | Type equality does not hold      |
| E4020 | Proof function required          |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission sites (reserved codes): the Sized constraint and
> optimization error path are pending implementation, to be wired up according to the actual trigger
> surface once implemented.

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
| E6009 | Invalid range step           |
| E6010 | Integer parse failure        |
| E6011 | Float parse failure          |

<!-- code-table:E6xxx end -->

> **Code table revision (2026-08-09)**: The code table was originally defined according to a
> Rust-semantic draft (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast
> failed), which did not match the implementation's actual needs. YaoXiang has no
> null-pointer/heap-allocation-failure/type-casting concepts (value semantics + Rust memory safety),
> and the runtime overflow detection path is not implemented. After calibration:
>
> - E6002 removed (original Assertion failed moved to E6005; the original null-pointer semantics
>   have no language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (actual trigger surface)
> - E6005 changed from Heap allocation failed to Assertion failed (the real path of std.assert)
> - E6006 changed from Runtime index out of bounds to Function not found (the implementation has
>   long done so)
> - E6007 changed from Type cast failed to the generic Runtime error (unified landing point for
>   unmapped ExecutorError variants)

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
| W1080 | Compile-time proof downgraded                |

<!-- code-table:W1xxx end -->

> W-code placement rule: isomorphic to E-codes, grouped by phase (W + phase-thousand segment), so
> W1xxx = type-checking-phase warnings.
>
> **Dead code semantics (#321 conclusion B)**: `pub` definitions are external interfaces and are
> never reported — whether external consumers use them is beyond single-file analysis, so we err on
> the side of silence over false positives. W1001/W1002/W1004/W1005 only target **private (non-pub)
> definitions**: any definition never referenced in the reachability analysis starting from `main`
> and `pub` definitions is reported. Methods (W1005) match by short name at the call site. bin/lib
> target semantics (reporting unused pub in bin) is a future extension (#289 plan A), requiring
> project-model support.
>
> **Unused imports (W1003)**: detected by the typechecker's use elaboration (pass2 registers
> imported local names; an expression-resolution or type-annotation position hit counts as used),
> covering both module imports (`use std.io` → module alias) and named imports
> (`use std.io.{print}`).
>
> **Emission channel**: W-code diagnostics are tagged `Severity::Warning` by default by the builder
> according to the W prefix (an explicit specification takes precedence). Collection and
> presentation share the same pipeline as errors (rendered with the `warning[W####]` prefix), but
> they do not block compilation nor affect the success exit code. `yaoxiang check --deny-warnings`
> upgrades warnings to failures (exits with a non-zero code if any warning is present), for strict
> CI mode. Per-code suppression (allow attributes, etc.) is left for a later extension.

### Message Quality Specification

> This section was introduced by the message single-rail and quality revision (2026-09-03). It is
> enforced in CI by `scripts/audit_diagnostics.py`.

1. **Message single-rail**: All user-visible diagnostic messages must go through an authoritative
   registry convenience method + locales template rendering; code only passes structured parameters.
   Bypassing the registry to directly construct raw values like `Diagnostic::error(...)` is
   forbidden — that path bypasses code validation and i18n.
2. **Code legality**: Using unregistered codes and pseudo-codes (e.g. `E_INTERNAL`) is forbidden;
   literal codes at emission sites must already be defined in the registry. Internal errors must
   land on E8001 (`internal_error`).
3. **Type display**: The Type Display implementation must distinguish instantiated from
   un-instantiated forms (the bare name `Expected 'Container', found 'Container'` is not
   distinguishable).
4. **Solver-internal-state isolation**: Solver intermediate-state TypeVars (display form `t<N>`)
   must not appear in user-visible messages. Test anchor:
   `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is only used for compiler internal consistency problems (ICEs).
   User-fixable errors are forbidden from being absorbed into E8001; ICE messages must include a
   minimal reproduction guide.

---

### Runtime Error Values and Code Interoperability

> This section was introduced by the runtime Error value-with-code revision (2026-09-03). The
> E6xxx/E7xxx semantic space carries both channels, sharing the same code space with different
> presentation channels.

#### The Two Channels

| Channel                         | Carrier                                               | Presentation                                                |
| ------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------- |
| Compiler/CLI diagnostic channel | `ExecutorError` and other host-layer hard errors      | stderr `error[E####]:` (already wired to E6003/E6005/E6007) |
| In-program error-value channel  | The `Error` Err carrier of std-lib `Result(T, Error)` | Language value, consumed by the program via match/compare   |

#### Error Structure (from v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbers of this specification, in string form (e.g. `"E6008"`).
- **Stable contract**: allocated codes retain their semantics across versions; the same semantics
  must not reuse a deleted code (the E6002 precedent).
- **Consumption surface**: in-program `e.code == "E6xxx"` comparison is the only programmable
  judgment contract; `yaoxiang explain E6xxx` documentation is consistent; the toolchain (LSP/DAP,
  see RFC-034) uses the code as exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: The `E` in `Result(T, E)` is a generic parameter; serious modeling should
  use user-defined types; std `Error` is only a convenient fallback carrier, and its code system
  does not constrain the user's E type.

#### Code Allocation Rules

1. Runtime error-value codes share the E6xxx/E7xxx space with compiler diagnostic codes; new codes
   are allocated per the **actual trigger surface**, with no reservations for imagined scenarios.
2. Register before use: new codes must enter the authoritative registry and pass three-way
   consistency validation (codes/*.rs ↔ locales ↔ this document's code table) before they may be
   emitted. The registration source for runtime error-value codes is the `RUNTIME_ERROR_CODES` table
   in `src/std/result.rs` (subject to the same `build.rs` compile-time gate + `tools/code-tables`
   validation as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error values (currently empty, to be enabled when io/net
   becomes Result-based).
4. Emission points: each std module constructs an Error value via `error_new(code, message)`; the
   consumer side uses `std.result.unwrap_err` to extract the Err carrier, and
   `std.result.code/message` to read the fields.

#### Evolution Path (line C, not yet implemented)

After pattern-matching completeness (RFC-039) lands, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming a property derived from kind (the
variant definition site becomes the code registry). During the evolution period, the stable contract
of this section remains unchanged; the upgrade is an independent decision and does not constitute a
commitment in this section.

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

/// i18n display-string registry (loaded from JSON at compile time, zero lookup at runtime)
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

/// Single error-code information
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// Obtain registry by language code
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

    /// Render template (done at compile time, zero overhead at runtime)
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

##### Predefined placeholders (common)

| Placeholder  | Purpose                                    | Example                             |
| ------------ | ------------------------------------------ | ----------------------------------- |
| `{name}`     | Identifiers like variable/type/trait names | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                              | `Expected type '{expected}'`        |
| `{found}`    | Actual / found type                        | `, found type '{found}'`            |
| `{method}`   | Method name                                | `Method {method} is not a function` |
| `{trait}`    | Trait name                                 | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                | `Invalid path: {path}`              |
| `{ty}`       | Type expression                            | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                     | `Internal error: {message}`         |

##### Arbitrary keys supported

**params supports arbitrary keys, not limited to the predefined set.** Callers can pass any `key`:

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

> **Note**: Not all error codes use placeholders. Some error codes (e.g. E0001) have static messages
> and require no parameters.

#### Language Priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. Default: en
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
1. Read the project-level yaoxiang.toml's language.default
2. If unset, read the user-level ~/.yaoxiang/yaoxiang.toml
3. If neither is set, default to "en"
4. The compiler creates an I18nRegistry (once) according to the selected language
5. All errors are rendered with that I18nRegistry
```

#### The Key to Zero Lookup Overhead

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
│  On error! macro call:                                                       │
│  1. Read yaoxiang.toml to get the language preference                                      │
│  2. Load the corresponding language's i18n JSON from the compiler binary                                │
│  3. template + params → render() → "Unknown variable: 'x'"                    │
│  4. Diagnostic.message = rendered string                                   │
│                                                                           │
│  The AOT binary stores the final string directly, no templates, no lookups                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runtime                                                  │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Outputs the final string directly, no lookups of any kind                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                         | Render timing                   |
| ---------------------------- | -------------------------------------- | ------------------------------- |
| `I18nRegistry`               | Provides templates and display strings | When compiling the user project |
| `DiagnosticBuilder.render()` | template + params → final string       | When compiling the user project |
| `Diagnostic.message`         | Rendered string                        | Stores the final result         |
| AOT binary                   | Contains the final strings             | Used directly at runtime        |

---

### Error Message Format

Error messages use the following format:

```
error[E####]: <short description>
  --> <file>:<line>:<col>
   <line> | <code snippet>
          ^^^<highlight>
```

#### Full Example

```
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?
```

---

### Severity Levels

Error severity is managed through the `DiagnosticLevel` enum, decoupled from the error code number:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation to fail
    Warning,  // Does not affect compilation, but fix is recommended
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
| `--lang <code>` | Specify language (en-US, zh-CN; default en-US) |
| `--json`        | JSON output (for IDE/LSP use)                  |
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

1. Maintain a mapping from old error codes to new error codes
2. Show both old and new codes during the migration period
3. Provide a deprecation timeline

---

## Implementation Strategy

### Phase 1: Error Code Infrastructure

1. Create the `src/diagnostics/` directory structure
2. Implement the `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create the resource-file directory and example JSON

### Phase 2: `explain` Command

1. Implement the `yaoxiang explain` CLI command
2. Support `--lang` and `--json` options
3. Integrate resource-file loading
4. Implement parameter template rendering

### Phase 3: Compile-time Integration

1. Update all error-reporting sites to use the new system
2. Implement message template parameter injection
3. Add language priority logic
4. Add unit-test coverage

### Phase 4: IDE/LSP Integration

1. LSP server integrates `explain` JSON output
2. Show error code links in the IDE
3. Show error explanation on hover
4. Quick-fix suggestions

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
