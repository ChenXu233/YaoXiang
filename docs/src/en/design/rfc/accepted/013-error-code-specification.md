---
title: 'RFC 013: Error Code Specification'
status: 'Accepted'
author: 'Chenxi'
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
Rust-like single-layer numbering system, combined with JSON resource files to support multilingual
support, and provides error explanation through the `yaoxiang explain` command.

## Motivation

### Why is a standardized error code needed?

1. **User Experience**: Users can quickly determine the error type and severity by seeing the error
   code.
2. **Documentation Organization**: Grouping by category facilitates writing and maintaining the
   error reference documentation.
3. **Tool Integration**: IDE/LSP can provide quick-fix suggestions and documentation links based on
   error codes.
4. **Internationalization Support**: Separating error messages from codes facilitates multilingual
   translation.

### Design Goals

- **Concise**: Single-layer numbering; users do not need to remember complex classification rules.
- **Friendly**: Rust-like error message format with help information and examples.
- **Extensible**: Resource file driven, easy to add new errors and new languages.
- **Tool-friendly**: explain command + JSON output, supporting IDE/LSP integration.

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

### Error Category Enumeration

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

**Core Principle**: Separation of error code definition and display copy.

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without display copy.
- `locales/*.json`: Display copy for each language (title, message, help, error codes as nested
  objects).
- `DiagnosticBuilder`: Generic builder, replacing the trait-per-error design.

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

    /// Build Diagnostic (template rendering completes at compile-time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Check that all {key} in the template have corresponding parameters
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

#### Shortcut Method for Each Error Code

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

| Feature                   | Description                                            |
| ------------------------- | ------------------------------------------------------ |
| **Single Builder**        | One `DiagnosticBuilder` is generic for all error codes |
| **Type Safety**           | Shortcut methods ensure parameter correctness          |
| **Self-documenting**      | `E1001::unknown_variable(name)` is self-explanatory    |
| **Template Separation**   | Message template separated from code, easy for i18n    |
| **Zero Runtime Overhead** | Compile-time rendering, no lookup table in AOT binary  |

---

### Error Macro Simplification

#### error! Macro (Auto-injecting Context)

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

#### E0xxx: Lexical and Syntax Analysis

| Code  | Error Type                | Description                                                |
| ----- | ------------------------- | ---------------------------------------------------------- |
| E0001 | Invalid character         | Source code contains illegal characters                    |
| E0002 | Invalid number literal    | Number literal format is incorrect                         |
| E0003 | Unterminated string       | Multi-line string missing the closing quote                |
| E0004 | Invalid character literal | Character literal is incorrect                             |
| E0010 | Expected token            | Expected a specific token during parsing                   |
| E0011 | Unexpected token          | Encountered an unexpected token                            |
| E0012 | Invalid syntax            | Expression/statement syntax error                          |
| E0013 | Mismatched brackets       | Parentheses, square brackets, or curly braces do not match |
| E0014 | Missing semicolon         | Statement missing semicolon at the end                     |
| E0016 | Expected expression       | Expected an expression                                     |
| E0018 | Keyword as name           | Keyword cannot be used as a name                           |

#### E1xxx: Type Checking

| Code  | Error Type                                             | Description                                            |
| ----- | ------------------------------------------------------ | ------------------------------------------------------ |
| E1001 | Unknown variable                                       | Referenced variable is not defined                     |
| E1002 | Type mismatch                                          | Expected type does not match the actual type           |
| E1003 | Unknown type                                           | Referenced type does not exist                         |
| E1010 | Parameter count mismatch                               | Function call argument count does not match definition |
| E1011 | Parameter type mismatch                                | Parameter type check failed                            |
| E1012 | Return type mismatch                                   | Function return value type is incorrect                |
| E1013 | Function not found                                     | Called an undefined function                           |
| E1020 | Cannot infer type                                      | Type cannot be inferred from context                   |
| E1021 | Type inference conflict                                | Multiple constraints lead to type contradiction        |
| E1030 | Pattern non-exhaustive                                 | match expression does not cover all cases              |
| E1031 | Unreachable pattern                                    | A pattern that can never match                         |
| E1040 | Operation not supported                                | Type does not support the operation                    |
| E1041 | Index out of bounds                                    | Array/list index is out of range                       |
| E1042 | Field not found                                        | Accessed a non-existent struct field                   |
| E1050 | Boolean operand required                               | Boolean operand is required                            |
| E1051 | Logical NOT requires boolean operand                   | Logical NOT requires a boolean operand                 |
| E1052 | Invalid dereference                                    | Invalid dereference                                    |
| E1053 | Non-struct field access                                | Field access on a non-struct                           |
| E1054 | Conditional type mismatch                              | Conditional type does not match                        |
| E1055 | Constraint in non-generic context                      | Constraint appears in a non-generic context            |
| E1060 | Type parameter count mismatch                          | Type parameter count does not match                    |
| E1061 | Cannot instantiate generic                             | Cannot instantiate a generic                           |
| E1062 | Const generic constraint failed                        | const generic constraint failed                        |
| E1064 | Invalid binding position                               | Binding position index is invalid (RFC-004)            |
| E1071 | Type definitions are only allowed at module level      | Type definitions are only allowed at module level      |
| E1081 | `?` can only be used within functions returning Result | `?` is only allowed in functions returning Result      |
| E1082 | `?` can only be used with Result expressions           | `?` can only be used with Result expressions           |
| E1083 | Error type mismatch for `?`                            | `?` error type does not match                          |
| E1090 | Type universe easter egg                               | Type: Type = Type easter egg (Note level)              |
| E1091 | Invalid generic meta type                              | Invalid generic meta type                              |
| E1092 | Invalid refinement type argument form                  | Refinement type argument form is illegal               |
| E1093 | Refinement argument count mismatch                     | Refinement argument count does not match               |
| E1094 | Unused compile-time value parameter                    | Unused compile-time value parameter                    |
| E1095 | Unknown interface                                      | Unknown interface                                      |
| E1096 | Interface arity mismatch                               | Interface parameter count does not match               |
| E1097 | Interface member name conflict                         | Interface member name conflict                         |
| E1098 | Interface method not implemented                       | Interface method not implemented                       |
| E1099 | Interface method signature mismatch                    | Interface method signature does not match              |
| E1100 | Duplicate interface method implementation              | Duplicate interface method implementation              |
| E1101 | Type does not implement interface                      | Type does not implement interface                      |
| E1102 | Loop control statement outside of a loop               | Loop control statement outside of a loop               |

#### E2xxx: Semantic Analysis

| Code  | Error Type                        | Description                                      |
| ----- | --------------------------------- | ------------------------------------------------ |
| E2001 | Scope error                       | Variable is not in the current scope             |
| E2002 | Duplicate definition              | Duplicate definition in the same scope           |
| E2003 | Lifetime error                    | Lifetime constraint not satisfied                |
| E2010 | Immutable assignment              | Attempted to modify an immutable variable        |
| E2011 | Uninitialized use                 | Used an uninitialized variable                   |
| E2012 | Mutability conflict               | Used a mutable reference in an immutable context |
| E2013 | Variable shadowing                | Variable shadowing                               |
| E2014 | Use of moved value                | Used a value that has been moved                 |
| E2016 | Immutable assignment              | Immutable assignment                             |
| E2018 | Mutable/immutable borrow conflict | Mutable/immutable borrow conflict                |
| E2019 | Double free                       | Double free                                      |
| E2020 | Use after free                    | Use after free                                   |
| E2027 | Unsafe dereference                | unsafe dereference                               |
| E2090 | Invalid signature                 | Function signature parsing error                 |
| E2091 | Unknown type in signature         | Unknown type in signature                        |
| E2092 | Missing arrow in signature        | Signature missing return arrow                   |
| E2093 | Duplicate parameter name          | Duplicate parameter name                         |
| E2094 | Generic parameter shadowing       | Generic parameter shadowing                      |
| E2095 | Parameter name shadows generic    | Parameter name shadows generic                   |

#### E3xxx: Code Generation

| Code  | Error Type                             | Description                                      |
| ----- | -------------------------------------- | ------------------------------------------------ |
| E3004 | Unsupported iterator                   | Unsupported iterator                             |
| E3005 | IR generation error                    | IR generation internal error                     |
| E3006 | Unresolved variable                    | Variable unresolved during IR generation         |
| E3007 | Top-level initializer must be constant | Top-level binding initializer must be a constant |
| E3014 | Register overflow                      | Register overflow                                |
| E3017 | Invalid operand (code generation)      | Invalid operand (code generation)                |

#### E4xxx: Generics and Trait

| Code  | Error Type                              | Description                                                |
| ----- | --------------------------------------- | ---------------------------------------------------------- |
| E4001 | Generic parameter mismatch              | Generic parameter count/type does not match                |
| E4002 | Trait bound violated                    | Trait constraint not satisfied                             |
| E4003 | Associated type error                   | Associated type definition/usage error                     |
| E4004 | Duplicate trait implementation          | Duplicate implementation of the same trait                 |
| E4005 | Trait not found                         | Required trait cannot be found                             |
| E4006 | Sized bound violated                    | Sized constraint not satisfied (reserved, not implemented) |
| E4010 | Division by zero in constant expression | Division by zero in constant expression                    |
| E4011 | Constant overflow                       | Constant overflow                                          |
| E4012 | Constant recursion too deep             | Constant recursion too deep                                |
| E4014 | Constant evaluation failed              | Constant evaluation failed                                 |
| E4018 | Refinement predicate violation          | Refinement predicate violated                              |
| E4019 | Type equality does not hold             | Type equality does not hold                                |
| E4020 | Proof function required                 | Proof function required to verify constraint               |

#### E5xxx: Modules and Imports

| Code  | Error Type            | Description                               |
| ----- | --------------------- | ----------------------------------------- |
| E5001 | Module not found      | Imported module does not exist            |
| E5002 | Cyclic import         | Cyclic dependency between modules         |
| E5003 | Symbol not exported   | Attempted to access a non-exported symbol |
| E5004 | Invalid module path   | Module path format is incorrect           |
| E5005 | Private access        | Accessed a private symbol                 |
| E5006 | Duplicate import      | Duplicate import                          |
| E5007 | Module export listing | Module export listing (companion hint)    |

#### E6xxx: Runtime Errors

| Code  | Error Type                  | Description                                 |
| ----- | --------------------------- | ------------------------------------------- |
| E6001 | Division by zero            | Integer divided by zero                     |
| E6002 | ~~Assertion failed~~        | ~~Reserved (no language concept, removed)~~ |
| E6003 | Runtime index out of bounds | Runtime index out of bounds (#280 wiring)   |
| E6004 | Stack overflow              | Stack space exhausted                       |
| E6005 | Assertion failed            | assert failed (#280 wiring)                 |
| E6006 | Function not found          | Function not found at runtime               |
| E6007 | Runtime error (generic)     | Generic runtime error                       |
| E6008 | Key not found               | Dict key missing (#299 §4)                  |

> **#280 Revision (2026-08-09)**: The code table was originally drafted following Rust semantics
> (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed), which did not
> match the actual implementation needs. YaoXiang has no null pointer/heap allocation failure/type
> cast concept (value semantics + Rust memory safety), and the runtime overflow path has no
> detection implemented. After calibration:
>
> - E6002 removed (original Assertion failed moved to E6005; original null pointer semantics has no
>   language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (real trigger surface,
>   #279/#271)
> - E6005 changed from Heap allocation failed to Assertion failed (real path of std.assert)
> - E6006 changed from Runtime index out of bounds to Function not found (implementation has long
>   been this way, #255)
> - E6007 changed from Type cast failed to generic Runtime error (unified landing point for unmapped
>   ExecutorError variants)

#### E7xxx: I/O and System Errors

| Code  | Error Type        | Description                           |
| ----- | ----------------- | ------------------------------------- |
| E7001 | File not found    | Attempted to read a non-existent file |
| E7002 | Permission denied | Insufficient file permissions         |
| E7003 | I/O error         | Generic I/O error                     |
| E7004 | Network error     | Network operation failed              |

#### E8xxx: Internal Compiler Errors

| Code  | Error Type              | Description                                             |
| ----- | ----------------------- | ------------------------------------------------------- |
| E8001 | Internal compiler error | Internal compiler error                                 |
| E8002 | Codegen error           | IR/bytecode generation failed                           |
| E8003 | Unimplemented feature   | Used an unimplemented feature                           |
| E8004 | Optimization error      | Compiler optimization error (reserved, not implemented) |

#### W1xxx: Warning Codes

| Code  | Warning Type                                 | Description                                                              |
| ----- | -------------------------------------------- | ------------------------------------------------------------------------ |
| W1001 | Unused exported function                     | Unused exported function                                                 |
| W1002 | Unused exported type                         | Unused exported type                                                     |
| W1003 | Unused import                                | Unused import                                                            |
| W1004 | Unused exported variable                     | Unused exported variable                                                 |
| W1005 | Unused exported method                       | Unused exported method                                                   |
| W1063 | Const generic constraint cannot be evaluated | const generic constraint cannot be evaluated                             |
| W1080 | Constraint demoted to runtime check          | Constraint could not be proven at compile-time, demoted to runtime check |

> W-code rules: Isomorphic with E-codes, grouped by phase (W + phase thousand-segment), W1xxx = type
> checking phase warnings.
>
> **Emission channel (#321 M2)**: W-code diagnostics are tagged with `Severity::Warning` by default
> by the builder based on the W prefix (explicit specification takes priority). Collection and
> presentation share the same track as errors (rendered with the `warning[W####]` prefix), but do
> not block compilation and do not affect the success exit code. `yaoxiang check --deny-warnings`
> upgrades warnings to failures (exits with a non-zero code when warnings exist), for strict CI
> mode. Per-code suppression (allow attributes, etc.) is for future extensions.

### Message Quality Specification

> This section was introduced by #322 (M3 Message Single-Track and Quality, 2026-09-03). Enforced in
> CI by `scripts/audit_diagnostics.py`.

1. **Message Single-Track**: All user-visible diagnostic messages must be rendered through the
   authoritative registry shortcut methods + locales templates; the code only passes structured
   parameters. Bypassing the registry to directly construct native values like
   `Diagnostic::error(...)` is prohibited—this path bypasses code validation and i18n.
2. **Code Validity**: Use of unregistered codes and pseudo-codes (such as `E_INTERNAL`) is
   prohibited; point-code literals at the use site must already be defined in the registry. Internal
   errors uniformly land on E8001 (`internal_error`).
3. **Type Display**: Type Display must distinguish the form before and after instantiation (#286:
   `Expected 'Container', found 'Container'` with bare names is not distinguishable).
4. **Solver Internal State Isolation**: The solver's intermediate state TypeVar (Display form
   `t<N>`) must not appear in user-visible messages (#287). Test anchor:
   `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx Boundary**: E8xxx is only used for compiler internal consistency issues (ICE).
   User-fixable errors are prohibited from using E8001 as a fallback; ICE messages must include
   minimal reproduction guidance.

---

### Runtime Error Values and Code Linkage

> This section was introduced by #323 (M4 Runtime Error Values with Codes, 2026-09-03). The
> E6xxx/E7xxx semantic space carries two channels simultaneously: the code space is shared, but the
> presentation channels are different.

#### Two Channels

| Channel                         | Carrier                                                          | Presentation Method                                                   |
| ------------------------------- | ---------------------------------------------------------------- | --------------------------------------------------------------------- |
| Compiler/CLI diagnostic channel | `ExecutorError` and other host-level hard errors                 | stderr `error[E####]:` (E6003/E6005/E6007 already wired in #280/#281) |
| In-program error value channel  | The Err carrier `Error` of `Result(T, Error)` in the std library | Language value, consumed by program match/comparison                  |

#### Error Structure (From v0.8, Breaking Change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbers in this specification, in string form (e.g., `"E6008"`).
- **Stable Contract**: The semantics of allocated codes do not change across versions; the same
  semantics does not reuse deleted codes (E6002 is a precedent).
- **Consumption Surface**: Programmatic `e.code == "E6xxx"` comparison within a program is the only
  programmable judgment contract; `yaoxiang explain E6xxx` documentation is linked; the toolchain
  (LSP / DAP, see RFC-034) uses the code as exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-Defined Errors**: In `Result(T, E)`, E is a generic parameter; for serious modeling, use a
  user-defined type; std `Error` is only a convenient fallback carrier, and its code system does not
  constrain user E types.

#### Code Allocation Rules

1. Runtime error value codes share the E6xxx/E7xxx space with compile-time diagnostic codes. New
   codes are allocated based on the **real trigger surface**; no pre-reservation for imagined
   scenarios.
2. Register before use: A new code must enter the authoritative registry and pass three-way
   consistency verification (codes/*.rs ↔ locales ↔ code table in this document) before it can be
   emitted.
3. E7xxx is the reserved segment for std.io / std.net error values (currently empty, activated when
   io/net become Result-based).

#### Evolution Path (Line C, Not Implemented)

After pattern matching completeness (RFC-039) lands, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, and `code` becomes a property derived from kind (the variant
definition location is the code registry). The stable contract of code in this section remains
unchanged during the evolution period; this upgrade is an independent decision and does not
constitute a commitment in this section.

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

/// i18n display copy registry (loaded from JSON at compile-time, zero lookup at runtime)
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
    /// Get the registry based on the language code
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

    /// Render template (completed at compile-time, zero overhead at runtime)
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

| Placeholder  | Purpose                                             | Example                             |
| ------------ | --------------------------------------------------- | ----------------------------------- |
| `{name}`     | Variable name/type name/trait name, etc. identifier | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                                       | `Expected type '{expected}'`        |
| `{found}`    | Actual/found type                                   | `, found type '{found}'`            |
| `{method}`   | Method name                                         | `Method {method} is not a function` |
| `{trait}`    | Trait name                                          | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                         | `Invalid path: {path}`              |
| `{ty}`       | Type expression                                     | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                              | `Internal error: {message}`         |

##### Arbitrary Key Support

**params supports any key, not limited to predefined ones**. The caller can pass any `key`:

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

> **Note**: Not all error codes use placeholders. Some error codes (such as E0001) are static
> messages that do not require parameters.

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
3. If neither is configured, use "en" by default
4. The compiler creates an I18nRegistry (once) based on the selected language
5. All errors use this I18nRegistry to render messages
```

#### Key to Zero Lookup Overhead

**Rendering happens when compiling the user's project, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Rust compiles the YaoXiang compiler                            │
│                                                                           │
│  JSON is packaged into the compiler binary                                │
│  Purpose: The explain command can directly read i18n data                 │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: YaoXiang compiles the user's project (rendering happens here)   │
│                                                                           │
│  When the error! macro is called:                                         │
│  1. Read yaoxiang.toml to obtain language preference                      │
│  2. Load the corresponding language's i18n JSON from the compiler binary  │
│  3. Template + parameters → render() → "Unknown variable: 'x'"          │
│  4. Diagnostic.message = rendered string                                 │
│                                                                           │
│  The AOT binary directly stores the final string, with no template       │
│  and no lookup                                                           │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runtime                                           │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Directly outputs the final string, with no lookup at all              │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                       | Rendering Timing                  |
| ---------------------------- | ------------------------------------ | --------------------------------- |
| `I18nRegistry`               | Provide templates and display copy   | When compiling the user's project |
| `DiagnosticBuilder.render()` | Template + parameters → final string | When compiling the user's project |
| `Diagnostic.message`         | Rendered string                      | Stores the final result           |
| AOT binary                   | Contains the final string            | Used directly at runtime          |

---

### Error Message Format

Error messages use the following format:

```
error[E####]: <Short description>
  --> <file>:<line>:<column>
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

Error severity is managed through the `DiagnosticLevel` enumeration, decoupled from the error code
number:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation failure
    Warning,  // Does not affect compilation, but recommended to fix
    Note,     // Supplementary information
    Help,     // Fix suggestion
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
| `--lang <code>` | Specify language (en-US, zh-CN, default en-US) |
| `--json`        | JSON format output (for IDE/LSP use)           |
| `--json-pretty` | Formatted JSON output                          |
| `--examples`    | Show example code only                         |
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

**Future migration strategy** (for reference by subsequent versions):

1. Maintain a mapping from old error codes to new error codes.
2. Display both old and new codes during the migration period.
3. Provide a deprecation schedule.

---

## Implementation Strategy

### Phase 1: Error Code Infrastructure

1. Create the `src/diagnostics/` directory structure.
2. Implement the `ErrorCode` enumeration.
3. Implement `Diagnostic` and `DiagnosticLevel`.
4. Create the resource file directory and example JSON.

### Phase 2: explain Command

1. Implement the `yaoxiang explain` CLI command.
2. Support `--lang` and `--json` options.
3. Integrate resource file loading.
4. Implement parameter template rendering.

### Phase 3: Compile-Time Integration

1. Update all error reporting points to use the new system.
2. Implement message template parameter injection.
3. Add language priority logic.
4. Unit test coverage.

### Phase 4: IDE/LSP Integration

1. LSP server integrates explain JSON output.
2. Display error code links in the IDE.
3. Show error explanations on hover.
4. Quick-fix suggestions.

---

## Appendix

### Complete Error Code Quick Reference Table

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
