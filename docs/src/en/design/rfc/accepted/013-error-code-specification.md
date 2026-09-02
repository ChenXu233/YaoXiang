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

This RFC proposes a categorization specification for YaoXiang compiler error codes, adopting a
Rust-like single-level numbering system, combined with JSON resource files to provide multi-language
support, and offering error explanation functionality through the `yaoxiang explain` command.

## Motivation

### Why do we need a standardized error code system?

1. **User Experience**: Users can quickly judge the error type and severity when seeing an error
   code
2. **Documentation Organization**: Grouping by category facilitates writing and maintaining error
   reference documentation
3. **Tool Integration**: IDEs/LSPs can provide quick-fix suggestions and documentation links based
   on error codes
4. **Internationalization Support**: Separating error messages from codes makes multi-language
   translation easier

### Design Goals

- **Concise**: Single-level numbering, so users do not need to memorize complex classification rules
- **Friendly**: Rust-like error message format, with help information and examples
- **Extensible**: Driven by resource files, easy to add new errors and new languages
- **Tool-friendly**: `explain` command + JSON output, supporting IDE/LSP integration

---

## Proposal

### Core Design: Single-level Numbering System

Adopt a four-digit numbering scheme, grouped by compilation phase:

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
| **0** | E0xxx | Lexical and syntax analysis |
| **1** | E1xxx | Type check                  |
| **2** | E2xxx | Semantic analysis           |
| **3** | E3xxx | Code generation             |
| **4** | E4xxx | Generics and traits         |
| **5** | E5xxx | Modules and imports         |
| **6** | E6xxx | Runtime errors              |
| **7** | E7xxx | I/O and system errors       |
| **8** | E8xxx | Internal compiler errors    |
| **9** | E9xxx | Reserved / experimental     |

### Error Category Enum

```rust
/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Lexical and syntax analysis
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: Type check
    Semantic,   // E2xxx: Semantic analysis
    Generic,    // E4xxx: Generics and traits
    Module,     // E5xxx: Modules and imports
    Runtime,    // E6xxx: Runtime errors
    Io,         // E7xxx: I/O and system errors
    Internal,   // E8xxx: Internal compiler errors
}
```

### Error Code Definitions and the Generic Builder

**Core principle**: Error code definitions are separated from display copy.

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without display copy
- `locales/*.json`: Display copy in each language (title, message, help, with error codes as nested
  objects)
- `DiagnosticBuilder`: A universal builder, replacing the trait-per-error design

#### Error Code Definition

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only, display copy lives in i18n files)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // Message template, supports {param} placeholders
}

/// Universal diagnostic builder
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

    /// Build the Diagnostic (template rendering completes at compile-time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Verify that every {key} in the template has a corresponding parameter
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

#### Example Error Code Definitions

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

| Feature                   | Description                                               |
| ------------------------- | --------------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` works for all error codes         |
| **Type Safety**           | Shortcut methods ensure parameter correctness             |
| **Self-documenting**      | `E1001::unknown_variable(name)` is self-explanatory       |
| **Template Separation**   | Message templates are separated from code, easing i18n    |
| **Zero Runtime Overhead** | Compile-time rendering, no table lookup in the AOT binary |

---

### Error Macro Simplification

#### `error!` Macro (Auto-injected Context)

```rust
/// Macro that automatically obtains span and i18n config at compile-time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: just pass the parameters; span and i18n are injected automatically
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

| Code  | Error Type                | Description                                    |
| ----- | ------------------------- | ---------------------------------------------- |
| E0001 | Invalid character         | Source code contains an invalid character      |
| E0002 | Invalid number literal    | Number literal format is incorrect             |
| E0003 | Unterminated string       | Multi-line string is missing the closing quote |
| E0004 | Invalid character literal | Character literal is incorrect                 |
| E0010 | Expected token            | Parser expected a specific token               |
| E0011 | Unexpected token          | Encountered an unexpected token                |
| E0012 | Invalid syntax            | Expression / statement syntax error            |
| E0013 | Mismatched brackets       | Parentheses, brackets, or braces do not match  |
| E0014 | Missing semicolon         | Statement is missing the trailing semicolon    |
| E0016 | Expected expression       | Expected an expression                         |
| E0018 | Keyword as name           | A keyword cannot be used as a name             |

#### E1xxx: Type Check

| Code  | Error Type                                             | Description                                                |
| ----- | ------------------------------------------------------ | ---------------------------------------------------------- |
| E1001 | Unknown variable                                       | Referenced variable is not defined                         |
| E1002 | Type mismatch                                          | Expected type does not match the actual type               |
| E1003 | Unknown type                                           | Referenced type does not exist                             |
| E1010 | Parameter count mismatch                               | Function call argument count does not match the definition |
| E1011 | Parameter type mismatch                                | Argument type check failed                                 |
| E1012 | Return type mismatch                                   | Function return type is wrong                              |
| E1013 | Function not found                                     | Calling an undefined function                              |
| E1020 | Cannot infer type                                      | Context cannot infer the type                              |
| E1021 | Type inference conflict                                | Multiple constraints lead to a type conflict               |
| E1030 | Pattern non-exhaustive                                 | `match` expression does not cover all cases                |
| E1031 | Unreachable pattern                                    | Pattern can never be matched                               |
| E1040 | Operation not supported                                | The type does not support this operation                   |
| E1041 | Index out of bounds                                    | Array / list index is out of range                         |
| E1042 | Field not found                                        | Accessing a non-existent struct field                      |
| E1050 | Boolean operand required                               | Boolean operand required                                   |
| E1051 | Logical NOT requires boolean operand                   | Logical NOT requires a boolean operand                     |
| E1052 | Invalid dereference                                    | Invalid dereference                                        |
| E1053 | Non-struct field access                                | Field access on a non-struct                               |
| E1054 | Conditional type mismatch                              | Conditional type mismatch                                  |
| E1055 | Constraint in non-generic context                      | Constraint appears in a non-generic context                |
| E1060 | Type parameter count mismatch                          | Type parameter count mismatch                              |
| E1061 | Cannot instantiate generic                             | Cannot instantiate the generic                             |
| E1062 | Const generics constraint failed                       | const generics constraint failed                           |
| E1064 | Invalid binding position                               | Invalid binding position index (RFC-004)                   |
| E1071 | Type definitions are only allowed at module level      | Type definitions are only allowed at module level          |
| E1081 | `?` can only be used within functions returning Result | `?` is only allowed in functions returning Result          |
| E1082 | `?` can only be used with Result expressions           | `?` can only be used with Result expressions               |
| E1083 | Error type mismatch for `?`                            | Error type mismatch for `?`                                |
| E1090 | Type universe easter egg                               | Type: Type = Type easter egg (Note level)                  |
| E1091 | Invalid generic meta type                              | Invalid generic meta type                                  |
| E1092 | Invalid refinement type argument form                  | Invalid refinement type argument form                      |
| E1093 | Refinement argument count mismatch                     | Refinement argument count mismatch                         |
| E1094 | Unused compile-time value parameter                    | Unused compile-time value parameter                        |
| E1095 | Unknown interface                                      | Unknown interface                                          |
| E1096 | Interface arity mismatch                               | Interface parameter count mismatch                         |
| E1097 | Interface member name conflict                         | Interface member name conflict                             |
| E1098 | Interface method not implemented                       | Interface method not implemented                           |
| E1099 | Interface method signature mismatch                    | Interface method signature mismatch                        |
| E1100 | Duplicate interface method implementation              | Duplicate interface method implementation                  |
| E1101 | Type does not implement interface                      | Type does not implement the interface                      |
| E1102 | Loop control statement outside of a loop               | Loop control statement appears outside of a loop           |

#### E2xxx: Semantic Analysis

| Code  | Error Type                        | Description                                       |
| ----- | --------------------------------- | ------------------------------------------------- |
| E2001 | Scope error                       | Variable is not in the current scope              |
| E2002 | Duplicate definition              | Duplicate definition within the same scope        |
| E2003 | Lifetime error                    | Lifetime constraint is not satisfied              |
| E2010 | Immutable assignment              | Attempting to modify an immutable variable        |
| E2011 | Uninitialized use                 | Using an uninitialized variable                   |
| E2012 | Mutability conflict               | Using a mutable reference in an immutable context |
| E2013 | Variable shadowing                | Variable shadowing                                |
| E2014 | Use of moved value                | Using a moved value                               |
| E2016 | Immutable assignment              | Immutable assignment                              |
| E2018 | Mutable/immutable borrow conflict | Mutable / immutable borrow conflict               |
| E2019 | Double free                       | Double free                                       |
| E2020 | Use after free                    | Use after free                                    |
| E2027 | Unsafe dereference                | unsafe dereference                                |
| E2090 | Invalid signature                 | Function signature parse error                    |
| E2091 | Unknown type in signature         | Unknown type in signature                         |
| E2092 | Missing arrow in signature        | Signature missing return arrow                    |
| E2093 | Duplicate parameter name          | Duplicate parameter name                          |
| E2094 | Generic parameter shadowing       | Generic parameter shadowing                       |
| E2095 | Parameter name shadows generic    | Parameter name shadows generic                    |

#### E3xxx: Code Generation

| Code  | Error Type                             | Description                                      |
| ----- | -------------------------------------- | ------------------------------------------------ |
| E3004 | Unsupported iterator                   | Unsupported iterator                             |
| E3005 | IR generation error                    | Internal IR generation error                     |
| E3006 | Unresolved variable                    | Variable not resolved during IR generation       |
| E3007 | Top-level initializer must be constant | Top-level binding initializer must be a constant |
| E3014 | Register overflow                      | Register overflow                                |
| E3017 | Invalid operand (code generation)      | Invalid operand (code generation)                |

#### E4xxx: Generics and Traits

| Code  | Error Type                              | Description                                           |
| ----- | --------------------------------------- | ----------------------------------------------------- |
| E4001 | Generic parameter mismatch              | Generics parameter count / type mismatch              |
| E4002 | Trait bound violated                    | Trait bound not satisfied                             |
| E4003 | Associated type error                   | Associated type definition / use error                |
| E4004 | Duplicate trait implementation          | Duplicate implementation of the same trait            |
| E4005 | Trait not found                         | Required trait cannot be found                        |
| E4006 | Sized bound violated                    | Sized bound violated (reserved, not implemented)      |
| E4010 | Division by zero in constant expression | Division by zero in constant expression               |
| E4011 | Constant overflow                       | Constant overflow                                     |
| E4012 | Constant recursion too deep             | Constant recursion too deep                           |
| E4014 | Constant evaluation failed              | Constant evaluation failed                            |
| E4018 | Refinement predicate violation          | Refinement predicate violated                         |
| E4019 | Type equality does not hold             | Type equality does not hold                           |
| E4020 | Proof function required                 | A proof function is required to verify the constraint |

#### E5xxx: Modules and Imports

| Code  | Error Type            | Description                                  |
| ----- | --------------------- | -------------------------------------------- |
| E5001 | Module not found      | Imported module does not exist               |
| E5002 | Cyclic import         | Cyclic dependency between modules            |
| E5003 | Symbol not exported   | Attempting to access a non-exported symbol   |
| E5004 | Invalid module path   | Module path format is incorrect              |
| E5005 | Private access        | Accessing a private symbol                   |
| E5006 | Duplicate import      | Duplicate import                             |
| E5007 | Module export listing | Module export listing (supporting hint info) |

#### E6xxx: Runtime Errors

| Code  | Error Type                  | Description                                                  |
| ----- | --------------------------- | ------------------------------------------------------------ |
| E6001 | Division by zero            | Integer division by zero                                     |
| E6002 | ~~Assertion failed~~        | ~~Reserved (no language concept, removed)~~                  |
| E6003 | Runtime index out of bounds | Runtime index out of bounds (#280 wiring)                    |
| E6004 | Stack overflow              | Stack space exhausted                                        |
| E6005 | Assertion failed            | assert failed (#280 wiring)                                  |
| E6006 | Function not found          | Runtime function not found                                   |
| E6007 | Runtime error (generic)     | Generic runtime error                                        |
| E6008 | Key not found               | Dict key missing (#299 §4)                                   |
| E6009 | Invalid range step          | Range step invalid (step=0, std.range Result-ification #316) |
| E6010 | Integer parse failed        | Integer parse failed (std.string.parse_int)                  |
| E6011 | Float parse failed          | Float parse failed (std.string.parse_float)                  |

> **#280 Revision (2026-08-09)**: The code table was originally drafted following Rust semantics
> (Assertion failed / Arithmetic overflow / Heap allocation failed / Type cast failed), which does
> not match the implementation's actual needs. YaoXiang has no concept of null pointers / heap
> allocation failure / type casts (value semantics + Rust memory safety), and the runtime overflow
> path has no detection implemented. After reconciliation:
>
> - E6002 removed (original Assertion failed moved to E6005; the original null-pointer semantics has
>   no language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (real trigger surface,
>   #279/#271)
> - E6005 changed from Heap allocation failed to Assertion failed (real path of std.assert)
> - E6006 changed from Runtime index out of bounds to Function not found (implementation already did
>   this, #255)
> - E6007 changed from Type cast failed to a generic Runtime error (unified landing point for
>   unmapped ExecutorError variants)

#### E7xxx: I/O and System Errors

| Code  | Error Type        | Description                        |
| ----- | ----------------- | ---------------------------------- |
| E7001 | File not found    | Trying to read a non-existent file |
| E7002 | Permission denied | Insufficient file permissions      |
| E7003 | I/O error         | Generic I/O error                  |
| E7004 | Network error     | Network operation failed           |

#### E8xxx: Internal Compiler Errors

| Code  | Error Type              | Description                                             |
| ----- | ----------------------- | ------------------------------------------------------- |
| E8001 | Internal compiler error | Internal compiler error                                 |
| E8002 | Codegen error           | IR / bytecode generation failed                         |
| E8003 | Unimplemented feature   | Using an unimplemented feature                          |
| E8004 | Optimization error      | Compiler optimization error (reserved, not implemented) |

#### W1xxx: Warning Codes

| Code  | Warning Type                                  | Description                                                              |
| ----- | --------------------------------------------- | ------------------------------------------------------------------------ |
| W1001 | Unused exported function                      | Unused exported function                                                 |
| W1002 | Unused exported type                          | Unused exported type                                                     |
| W1003 | Unused import                                 | Unused import                                                            |
| W1004 | Unused exported variable                      | Unused exported variable                                                 |
| W1005 | Unused exported method                        | Unused exported method                                                   |
| W1063 | Const generics constraint cannot be evaluated | const generics constraint cannot be evaluated                            |
| W1080 | Constraint demoted to runtime check           | Constraint could not be proven at compile-time, demoted to runtime check |

> W code position rule: isomorphic to E codes and grouped by phase (W + phase thousands segment), so
> W1xxx = type check phase warnings.
>
> **Emission channel (#321 M2)**: W code diagnostics are marked with `Severity::Warning` by the
> builder based on the W prefix by default (explicit specification takes priority). Collection and
> presentation share the same pipeline as errors (rendered with the `warning[W####]` prefix), but
> they do not block compilation and do not affect the successful exit code.
> `yaoxiang check --deny-warnings` escalates warnings to failure (exits with a non-zero code when
> warnings are present), for use in CI strict mode. Per-code suppression (allow attribute, etc.) is
> a future extension item.

### Message Quality Specification

> This section was introduced by #322 (M3 message unification and quality, 2026-09-03). It is
> enforced in CI by `scripts/audit_diagnostics.py`.

1. **Unified Message Pipeline**: All user-visible diagnostic messages must be rendered through the
   authoritative registry's shortcut methods + locale templates; the code only passes structured
   parameters. It is forbidden to bypass the registry and directly construct raw values like
   `Diagnostic::error(...)` — that path bypasses code validation and i18n.
2. **Code Validity**: Using unregistered codes or pseudo-codes (e.g., `E_INTERNAL`) is forbidden;
   any literal code used at a call site must already be defined in the registry. Internal errors
   uniformly fall under E8001 (`internal_error`).
3. **Type Display**: Type `Display` must distinguish pre-/post-instantiation forms (#286: a bare
   name like `Expected 'Container', found 'Container'` is indistinguishable).
4. **Solver Internal State Isolation**: Solver intermediate state `TypeVar` (in `Display` form
   `t<N>`) must not appear in user-visible messages (#287). Test anchor:
   `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx Boundary**: E8xxx is only used for internal compiler consistency issues (ICE).
   User-fixable errors must not use E8001 as a fallback; ICE messages must include minimal
   reproduction instructions.

---

### Runtime Error Values and Code Unification

> This section was introduced by #323 (M4 runtime Error value carrying a code, 2026-09-03). The
> E6xxx/E7xxx semantic space carries two channels simultaneously, sharing one code space but with
> different presentation channels.

#### Two Channels

| Channel                           | Carrier                                                     | Presentation                                                          |
| --------------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------- |
| Compiler / CLI diagnostic channel | `ExecutorError` and other host-layer hard errors            | stderr `error[E####]:` (E6003/E6005/E6007 already wired in #280/#281) |
| In-program error value channel    | The `Err` carrier `Error` of the std lib `Result(T, Error)` | A language value, consumed by program match / comparison              |

#### Error Structure (from v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbers in this specification, in string form (e.g., `"E6008"`).
- **Stability contract**: Allocated codes keep their semantics unchanged across versions; the same
  semantics will not reuse a deleted code (the E6002 precedent).
- **Consumption side**: Comparing `e.code == "E6xxx"` in-program is the only programmable judgment
  contract; documentation is unified via `yaoxiang explain E6xxx`; toolchains (LSP / DAP, see
  RFC-034) use the code as the `exceptionId`.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: In `Result(T, E)`, the `E` is a generics parameter. Properly modeled
  cases use user-defined types; std `Error` is only a convenient fallback carrier, and its code
  system does not constrain user E types.

#### Code Allocation Rules

1. Runtime error value codes share the E6xxx/E7xxx space with compiler diagnostic codes. New codes
   are allocated based on **actual trigger surface**, never reserved for imagined scenarios.
2. Register before use: A new code must enter the authoritative registry and pass three-way
   consistency check (codes/*.rs ↔ locales ↔ the code table in this document) before it can be
   emitted. The registration source for runtime error value codes is the `RUNTIME_ERROR_CODES` table
   in `src/std/result.rs` (also validated by `scripts/check_error_codes.py` together with the
   diagnostic codes).
3. E7xxx is the reserved range for std.io / std.net error values (currently empty; to be activated
   when io/net become Result-typed).
4. Emission sites (#323 M4): each std module constructs an `Error` value via
   `error_new(code, message)`; on the consumer side, `std.result.unwrap_err` extracts the `Err`
   carrier, and `std.result.code/message` reads the fields.

#### Evolution Path (Track C, not implemented)

Once pattern match exhaustiveness (RFC-039) lands, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` derived from `kind` (the variant definition site
is the code registry). The `code` stability contract in this section remains unchanged during the
evolution period; this upgrade is an independent decision and does not constitute a commitment of
this section.

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
// locales/*.json (error code object)

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

**`params` supports any keys, not limited to predefined ones.** Callers may pass any `key`:

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
> and need no parameters.

#### Language Priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. Default value: en
```

### `yaoxiang.toml` Configuration

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
1. Read project-level yaoxiang.toml's language.default
2. If not configured, read user-level ~/.yaoxiang/yaoxiang.toml
3. If neither is configured, default to "en"
4. The compiler creates an I18nRegistry according to the chosen language (once)
5. All errors use that I18nRegistry to render messages
```

#### The Key to Zero Lookup Overhead

**Rendering happens when compiling the user's project, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Rust compiles the YaoXiang compiler                             │
│                                                                           │
│  JSON is packed into the compiler binary                                 │
│  Purpose: the explain command can read i18n data directly                │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: YaoXiang compiles a user project (rendering happens here)        │
│                                                                           │
│  When the error! macro is called:                                        │
│  1. Read yaoxiang.toml to get language preference                        │
│  2. Load the i18n JSON for the corresponding language from the compiler  │
│     binary                                                                │
│  3. Template + params → render() → "Unknown variable: 'x'"              │
│  4. Diagnostic.message = the already-rendered string                      │
│                                                                           │
│  The AOT binary directly stores the final string — no template, no      │
│  table lookup                                                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runtime                                            │
│                                                                           │
│  println!("{}", diagnostic.message)                                       │
│  // Outputs the final string directly, no table lookup                    │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                     | Render Time                |
| ---------------------------- | ---------------------------------- | -------------------------- |
| `I18nRegistry`               | Provide templates and display copy | Compiling the user project |
| `DiagnosticBuilder.render()` | Template + params → final string   | Compiling the user project |
| `Diagnostic.message`         | Already-rendered string            | Stores the final result    |
| AOT binary                   | Contains the final string          | Used directly at runtime   |

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

| Option          | Description                                        |
| --------------- | -------------------------------------------------- |
| `--lang <code>` | Specify the language (en-US, zh-CN, default en-US) |
| `--json`        | JSON format output (for IDE/LSP use)               |
| `--json-pretty` | Pretty-formatted JSON output                       |
| `--examples`    | Only show example code                             |
| `--help`        | Show help information                              |

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

**Future migration strategy** (for reference in later versions):

1. Keep a mapping from old error codes to new error codes
2. Display both old and new codes during the migration period
3. Provide a deprecation timeline

---

## Implementation Strategy

### Phase 1: Error Code Infrastructure

1. Create the `src/diagnostics/` directory structure
2. Implement the `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create the resource file directory and sample JSON

### Phase 2: `explain` Command

1. Implement the `yaoxiang explain` CLI command
2. Support `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase 3: Compile-time Integration

1. Update all error reporting sites to use the new system
2. Implement message template parameter injection
3. Add language priority logic
4. Unit test coverage

### Phase 4: IDE / LSP Integration

1. LSP server integration of explain JSON output
2. Display error code links in the IDE
3. Hover to show error explanation
4. Quick-fix suggestions

---

## Appendix

### Full Error Code Quick Reference

| Range | Category                    |
| ----- | --------------------------- |
| E0xxx | Lexical and syntax analysis |
| E1xxx | Type check                  |
| E2xxx | Semantic analysis           |
| E3xxx | Code generation             |
| E4xxx | Generics and traits         |
| E5xxx | Modules and imports         |
| E6xxx | Runtime errors              |
| E7xxx | I/O and system errors       |
| E8xxx | Internal compiler errors    |
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
