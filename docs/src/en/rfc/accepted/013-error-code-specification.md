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
Rust-like single-level numbering system, combined with JSON resource files to support multiple
languages, and providing error explanation functionality through the `yaoxiang explain` command.

## Motivation

### Why do we need standardized error codes?

1. **User experience**: Users seeing an error code can quickly determine the error type and severity
2. **Documentation organization**: Grouping by category makes it easy to write and maintain error
   reference documentation
3. **Tool integration**: IDEs/LSPs can provide quick-fix suggestions and documentation links based
   on error codes
4. **Internationalization support**: Separating error messages from codes makes multi-language
   translation easy

### Design goals

- **Concise**: Single-level numbering, users don't need to remember complex classification rules
- **Friendly**: Rust-like error message format, with help information and examples
- **Extensible**: Resource-file driven, easy to add new errors and new languages
- **Tool-friendly**: `explain` command + JSON output, supports IDE/LSP integration

---

## Proposal

### Core design: single-level numbering system

Adopt a four-digit numbering scheme, grouped by compilation phase:

```
Exxxx
││││
│││└── Serial number (000-999)
││└─── Compilation phase (0-9)
└───── Fixed prefix 'E'
```

### Phase partitioning

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

### Error code definitions and generic Builder

**Core principle**: Error code definitions are separated from display text

- `ErrorCodeDefinition`: error code metadata (code, category, template), without display text
- `locales/*.json`: display text per language (title, message, help, error codes as nested objects)
- `DiagnosticBuilder`: generic builder, replacing the trait-per-error design

#### Error code definition

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

    /// Set the position
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Build a Diagnostic (template rendering is done at compile time)
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

#### Error code definition example

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

#### Design advantages

| Feature                   | Description                                              |
| ------------------------- | -------------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` is used by all error codes       |
| **Type safety**           | Shortcut methods ensure parameter correctness            |
| **Self-documenting**      | `E1001::unknown_variable(name)` is self-explanatory      |
| **Template separation**   | Message template separated from code, easy for i18n      |
| **Zero runtime overhead** | Rendering at compile time, no lookup table in AOT binary |

---

### Error macro simplification

#### `error!` macro (auto-injects context)

```rust
/// Macro that automatically fetches span and i18n configuration at compile time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: just pass parameters, span and i18n are injected automatically
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Manual Builder usage

```rust
// When manual control is needed
E1001::unknown_variable(&var_name)
    .at(my_span)           // Custom span
    .build(&custom_i18n)   // Custom i18n
```

---

## Detailed design

### Error code list

#### E0xxx: Lexical and syntax analysis

<!-- code-table:E0xxx start -->

| Code  | Description               |
| ----- | ------------------------- |
| E0001 | Invalid character         |
| E0002 | Invalid number literal    |
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

#### E1xxx: Type checking

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
| E1015 | Duplicate argument                                        |
| E1020 | Cannot infer type                                         |
| E1021 | Type inference conflict                                   |
| E1030 | Incomplete pattern                                        |
| E1031 | Unreachable pattern                                       |
| E1032 | Duplicate pattern binding                                 |
| E1033 | Inconsistent or-pattern binding                           |
| E1034 | Struct pattern missing field                              |
| E1040 | Operation not supported                                   |
| E1041 | Index out of bounds                                       |
| E1042 | Field not found                                           |
| E1043 | Module member not found                                   |
| E1050 | Boolean operand required                                  |
| E1051 | Logical NOT requires boolean operand                      |
| E1052 | Invalid dereference                                       |
| E1053 | Non-struct field access                                   |
| E1054 | Condition type mismatch                                   |
| E1055 | Constraint in non-generic context                         |
| E1060 | Type argument count mismatch                              |
| E1061 | Cannot instantiate generic                                |
| E1062 | const generic constraint failure                          |
| E1064 | Invalid binding position index                            |
| E1065 | Call on non-function value                                |
| E1071 | Type definitions only allowed at module level             |
| E1081 | `?` only allowed in functions returning propagatable type |
| E1082 | `?` can only be used on types implementing Try            |
| E1083 | `?` error type mismatch                                   |
| E1090 | ✨ unspeakable ✨                                         |
| E1091 | Invalid generic meta type                                 |
| E1092 | Refinement type argument form illegal                     |
| E1093 | Refinement argument count mismatch                        |
| E1094 | Unused compile-time value parameter                       |
| E1095 | Unknown interface                                         |
| E1096 | Interface argument count mismatch                         |
| E1097 | Interface member naming conflict                          |
| E1098 | Interface method not implemented                          |
| E1099 | Interface method signature mismatch                       |
| E1100 | Duplicate interface method implementation                 |
| E1101 | Type does not implement interface                         |
| E1102 | Loop control statement outside loop                       |
| E1103 | Brackets not allowed at type position                     |
| E1104 | Interface implementation not in type's defining module    |
| E1105 | Variant constructor cannot be accessed as field           |
| E1106 | Constraint not satisfied                                  |
| E1107 | Method overload ambiguity                                 |
| E1108 | Empty block falling into container expected position      |

<!-- code-table:E1xxx end -->

> **RFC-011b reference (2026-09-22 note)**:
> [RFC-011b: Operator overloading](011b-operator-overloading.md) implementation will touch three
> points in this section — ① `E1081` / `E1082` text to drop the "Result" word (`?` will be judged by
> the `Try` interface, no longer tied to a specific type name), synced in phase 2 via the
> "three-party consistency" process (codes/*.rs ↔ locales ↔ code table); ② rejection diagnostics for
> unsatisfied `Equal` preconditions (linear tokens) reuse the `E1101` (type does not implement
> interface) family; ③ After phase 1 wiring, `Struct == Struct` shifts from `E6007` runtime error to
> compile-time judgment, narrowing `E6007`'s trigger surface. The registered text in the table is
> maintained as-is until the implementation lands.

#### E2xxx: Semantic analysis

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
| E2029 | spawn reference loop                   |
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

> Reserved code notes (2026-09-14 inventory, #251 release wording): E2019 (double free), E2020 (use
> after free), E2027 (unsafe dereference), E2029 (spawn reference loop) have been registered with
> unit-test anchors, but lack a reachable yx source-level surface (explicit drop statements, Ptr
> dereference grammar, spawn ref loop construction path) — the claim of full semantic correctness
> does not cover these four codes; they are treated as "reserved" until the implementation is filled
> in.

#### E3xxx: Code generation

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
| E3023 | Executable statements not allowed at top level    |

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
| E4019 | Type equality not satisfied                     |
| E4020 | Proof function required                         |
| E4021 | Loop termination cannot be automatically proven |
| E4022 | Measure not satisfied                           |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission point (reserved codes): Sized constraint and optimization
> error path are pending implementation; wire them according to the real trigger surface when
> implemented.

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

| Code  | Description                  |
| ----- | ---------------------------- |
| E6001 | Division by zero             |
| E6003 | Array index out of bounds    |
| E6004 | Stack overflow               |
| E6005 | Assertion failed             |
| E6006 | Function not found (runtime) |
| E6007 | Runtime error                |
| E6008 | Key not found                |
| E6009 | Invalid range step           |
| E6010 | Integer parse failed         |
| E6011 | Float parse failed           |
| E6012 | Invalid code point           |
| E6013 | JSON parse failed            |

<!-- code-table:E6xxx end -->

> **Code table revision (2026-08-09)**: The code table was originally drafted per Rust semantics
> (Assertion failed / Arithmetic overflow / Heap allocation failed / Type cast failed), which
> doesn't match actual implementation needs. YaoXiang has no null pointer / heap allocation failure
> / type cast concepts (value semantics + Rust memory safety), and runtime overflow paths lack
> detection. After calibration:
>
> - E6002 removed (original Assertion failed moved to E6005; original null pointer semantics has no
>   language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (real trigger surface)
> - E6005 changed from Heap allocation failed to Assertion failed (real path of std.assert)
> - E6006 changed from Runtime index out of bounds to Function not found (already so in
>   implementation)
> - E6007 changed from Type cast failed to generic Runtime error (unified landing for unmapped
>   ExecutorError variants)

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
| W1006 | Local module shadows dependency package      |
| W1063 | const generic constraint cannot be evaluated |
| W1080 | Compile-time proof downgraded                |

<!-- code-table:W1xxx end -->

> W code position rules: isomorphic to E codes, grouped by phase (W + phase-thousand segment), W1xxx
> = type-checking-phase warnings.
>
> **Dead code semantics (#321 decision B)**: `pub` definitions are external interfaces, never
> reported — whether an external consumer uses them is beyond the single-file analysis boundary;
> better to stay silent than misreport. W1001/W1002/W1004/W1005 only target **private (non-`pub`)
> definitions**: anything never referenced in the reachability analysis starting from `main` and
> `pub` definitions is reported. Methods (W1005) match by call-site short name. bin/lib target
> semantics (reporting unused pub inside bin) is a future extension (#289 plan A), requiring
> project-model support.
>
> **Unused imports (W1003)**: detected by typecheck's use elaboration (pass 2 records imported local
> names; hit by expression resolution or type annotation positions counts as used), covering whole
> imports (`use std.io` → module alias) and named imports (`use std.io.{print}`).
>
> **Emission channel**: W-code diagnostics are tagged `Severity::Warning` by the builder by default
> based on the W prefix (explicit specification takes priority). Collection and rendering follow the
> same track as errors (`warning[W####]` prefix render), but do not block compilation and do not
> affect the success exit code. `yaoxiang check --deny-warnings` upgrades warnings to failures
> (non-zero exit when warnings exist), for CI strict mode. Per-code suppression (allow attributes
> etc.) is a follow-up extension.

### Message quality specification

> This section is introduced by the message single-track and quality revision (2026-09-03). Enforced
> in CI by `scripts/audit_diagnostics.py`.

1. **Message single-track**: All user-visible diagnostic messages must go through the authoritative
   registry's shortcut methods + locales template rendering; the code only passes structured
   parameters. Bypassing the registry to directly construct native values like
   `Diagnostic::error(...)` is forbidden — that path bypasses code validation and i18n.
2. **Code legality**: Using unregistered codes and pseudo-codes (such as `E_INTERNAL`) is forbidden;
   code literals used at emission points must already be defined in the registry. Internal errors
   all land on E8001 (`internal_error`).
3. **Type display**: Type Display must distinguish pre- and post-instantiation forms
   (`Expected 'Container', found 'Container'` bare names are indistinguishable).
4. **Solver internal state isolation**: Solver intermediate TypeVars (Display form `t<N>`) must not
   enter user-visible messages. Test anchor: `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx boundary**: E8xxx is only for compiler internal consistency issues (ICE). User-fixable
   errors must not use E8001 as a catch-all; ICE messages must include a minimal reproduction guide.

---

### Runtime error value and code linkage

> This section is introduced by the runtime Error value with code revision (2026-09-03). E6xxx/E7xxx
> semantic space carries two channels simultaneously, sharing the code space but using different
> presentation channels.

#### Two channels

| Channel                         | Carrier                                              | Presentation                                             |
| ------------------------------- | ---------------------------------------------------- | -------------------------------------------------------- |
| Compiler/CLI diagnostic channel | Host-level hard errors like `ExecutorError`          | stderr `error[E####]:` (already wired E6003/E6005/E6007) |
| In-program error value channel  | std library `Result(T, Error)`'s `Error` Err carrier | Language value, consumed by program match/comparison     |

#### Error structure (since v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses this specification's E6xxx/E7xxx numbering, in string form (e.g. `"E6008"`).
- **Stability contract**: Allocated codes keep their semantics across versions; deleted codes are
  not reused for new semantics (E6002 precedent).
- **Consumption surface**: In-program `e.code == "E6xxx"` comparison is the only programmable
  judgment contract; `yaoxiang explain E6xxx` documentation is linked; toolchains (LSP / DAP, see
  RFC-034) use the code as `exceptionId`.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined errors**: `Result(T, E)`'s E is a generic parameter; serious modeling uses
  user-defined types; std `Error` is only a convenient catch-all carrier, its code system does not
  constrain user E types.

#### Code allocation rules

1. Runtime error value codes and compiler diagnostic codes share the E6xxx/E7xxx space; new codes
   are allocated by **real trigger surface**, not reserved for imagined scenarios.
2. Register before use: new codes must enter the authoritative registry and pass three-party
   consistency verification (codes/*.rs ↔ locales ↔ this document's code table) before emission. The
   registration source for runtime error value codes is the `RUNTIME_ERROR_CODES` table in
   `src/std/result.rs` (subject to the same `build.rs` compile-time gate + `tools/code-tables`
   verification as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error values (currently empty, enabled when io/net become
   Result-ized).
4. Emission point: std modules construct Error values via `error_new(code, message)`; consumers use
   `std.result.unwrap_err` to extract the Err carrier, `std.result.code/message` to read fields.

#### Evolution path (line C, not implemented)

After pattern matching completeness (RFC-010b) lands, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming an attribute derived from kind (the
variant definition site is the code registry). During evolution, the code stability contract in this
section remains unchanged; this upgrade is an independent decision and is not a commitment of this
section.

---

### Multi-language resource files

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
// locales/*.json (error code objects)

/// i18n display text registry (loaded from JSON at compile time, zero lookup at runtime)
pub struct I18nRegistry {
    /// Titles
    titles: HashMap<&'static str, &'static str>,
    /// Descriptions
    messages: HashMap<&'static str, &'static str>,
    /// Help information
    helps: HashMap<&'static str, &'static str>,
    /// Example code
    examples: HashMap<&'static str, &'static str>,
    /// Error output example
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
    /// Get registry by language code
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

#### Template placeholders

##### Predefined placeholders (common)

| Placeholder  | Purpose                                     | Example                             |
| ------------ | ------------------------------------------- | ----------------------------------- |
| `{name}`     | Identifier such as variable/type/trait name | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                               | `Expected type '{expected}'`        |
| `{found}`    | Actual/found type                           | `, found type '{found}'`            |
| `{method}`   | Method name                                 | `Method {method} is not a function` |
| `{trait}`    | Trait name                                  | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                 | `Invalid path: {path}`              |
| `{ty}`       | Type expression                             | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                      | `Internal error: {message}`         |

##### Arbitrary key support

**params supports arbitrary keys, not limited to the predefined ones.** The caller can pass any
`key`:

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
> messages and don't need parameters.

#### Language priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. Default value: en
```

### `yaoxiang.toml` configuration

#### Project-level configuration

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# Error message language, options: en, zh, ja, ...
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
1. Read project-level yaoxiang.toml's language.default
2. If not configured, read user-level ~/.yaoxiang/yaoxiang.toml
3. If neither is configured, default to "en"
4. Compiler creates I18nRegistry based on the selected language (once)
5. All errors use that I18nRegistry to render messages
```

#### The key to zero lookup-table overhead

**Rendering happens when compiling the user project, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Rust compiles the YaoXiang compiler                            │
│                                                                           │
│  JSON is packed into the compiler binary                                 │
│  Purpose: explain command can read i18n data directly                    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: YaoXiang compiles the user project (rendering happens here)    │
│                                                                           │
│  When error! macro is called:                                            │
│  1. Read yaoxiang.toml to get language preference                       │
│  2. Load i18n JSON for the corresponding language from compiler binary  │
│  3. Template + params → render() → "Unknown variable: 'x'"              │
│  4. Diagnostic.message = rendered string                                 │
│                                                                           │
│  AOT binary directly stores the final string, no template, no lookup    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runtime                                            │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Outputs the final string directly, no lookup at all                  │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                      | Rendering time              |
| ---------------------------- | ----------------------------------- | --------------------------- |
| `I18nRegistry`               | Provides templates and display text | When compiling user project |
| `DiagnosticBuilder.render()` | Template + params → final string    | When compiling user project |
| `Diagnostic.message`         | Rendered string                     | Stores final result         |
| AOT binary                   | Contains final string               | Used directly at runtime    |

---

### Error message format

Error messages use the following format:

```
error[E####]: <short description>
  --> <file>:<line>:<column>
   <line> | <code snippet>
          ^^^<highlight>
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

Error severity is managed via the `DiagnosticLevel` enum, decoupled from error code numbering:

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

### `yaoxiang explain` command

#### Command syntax

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Options

| Option          | Description                                    |
| --------------- | ---------------------------------------------- |
| `--lang <code>` | Specify language (en-US, zh-CN, default en-US) |
| `--json`        | JSON format output (for IDE/LSP use)           |
| `--json-pretty` | Pretty-printed JSON output                     |
| `--examples`    | Show only example code                         |
| `--help`        | Show help information                          |

#### Usage examples

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

Since this RFC designs the error code system from scratch, there is no backward compatibility issue.

**Future migration strategy** (for reference in subsequent versions):

1. Maintain mapping from old error codes to new error codes
2. Display both old and new codes during migration
3. Provide a deprecation timeline

---

## Implementation strategy

### Phase 1: Error code infrastructure

1. Create the `src/diagnostics/` directory structure
2. Implement the `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create the resource file directory and sample JSON

### Phase 2: `explain` command

1. Implement the `yaoxiang explain` CLI command
2. Support `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase 3: Compile-time integration

1. Update all error reporting points to use the new system
2. Implement message template parameter injection
3. Add language priority logic
4. Unit test coverage

### Phase 4: IDE/LSP integration

1. LSP server integration with `explain` JSON output
2. Display error code links in IDE
3. Hover to show error explanation
4. Quick-fix suggestions

---

## Appendix

### Complete error code quick-reference table

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
