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
Rust-like single-layer numbering system, combined with JSON resource files to support multiple
languages, and providing error explanation functionality through the `yaoxiang explain` command.

## Motivation

### Why Do We Need Standardized Error Codes?

1. **User Experience**: When users see an error code, they can quickly determine the type and
   severity of the error.
2. **Documentation Organization**: Grouping by category makes it easier to write and maintain error
   reference documentation.
3. **Tool Integration**: IDE/LSP can provide quick-fix suggestions and documentation links based on
   error codes.
4. **Internationalization Support**: Separating error messages from codes makes multilingual
   translation easier.

### Design Goals

- **Concise**: Single-layer numbering, users don't need to memorize complex classification rules.
- **Friendly**: Rust-like error message format with help information and examples.
- **Extensible**: Resource file driven, easy to add new errors and new languages.
- **Tool-friendly**: `explain` command + JSON output, supports IDE/LSP integration.

---

## Proposal

### Core Design: Single-Layer Numbering System

Adopt a four-digit numbering scheme, grouped by compilation phase:

```
Exxxx
││││
│││└── Sequence Number (000-999)
││└─── Compilation Phase (0-9)
└───── Fixed Prefix 'E'
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

### Error Code Definition and Generic Builder

**Core Principle**: Error code definitions are separated from display text.

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without display text.
- `locales/*.json`: Display text for each language (title, message, help, error codes as nested
  objects).
- `DiagnosticBuilder`: Generic builder, replacing the trait-per-error design.

#### Error Code Definition

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// 错误码定义（仅元数据，展示文案在 i18n 文件）
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // 消息模板，支持 {param} 占位符
}

/// 通用诊断构建器
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

    /// 添加模板参数
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// 设置位置
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// 构建 Diagnostic（模板渲染在编译期完成）
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // 检查模板中所有 {key} 都有对应参数
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
    /// E1001 未知变量
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 类型不匹配
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

// 简化方式
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// 手动方式
return Err(ErrorCodeDefinition::find("E1001")
    .builder()
    .param("name", var_name)
    .at(span)
    .build(&i18n_registry));
```

#### Error Code Definition Examples

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
    // ... 其他错误码
];
```

#### Design Advantages

| Feature                   | Description                                         |
| ------------------------- | --------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` for all error codes         |
| **Type Safety**           | Shortcut methods ensure parameter correctness       |
| **Self-documenting**      | `E1001::unknown_variable(name)` is self-explanatory |
| **Template Separation**   | Message templates separated from code, easy to i18n |
| **Zero Runtime Overhead** | Compile-time rendering, no lookup in AOT binary     |

---

### Error Macro Simplification

#### `error!` Macro (Auto-Inject Context)

```rust
/// 编译期自动获取 span 和 i18n 配置的宏
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// 使用：只需传参数，span 和 i18n 自动注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Manual Use of the Builder

```rust
// 需要手动控制时
E1001::unknown_variable(&var_name)
    .at(my_span)           // 自定义 span
    .build(&custom_i18n)   // 自定义 i18n
```

---

## Detailed Design

### Error Code List

#### E0xxx: Lexical and Syntax Analysis

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
| E0013 | Mismatched parentheses    |
| E0014 | Missing semicolon         |
| E0016 | Expected expression       |
| E0018 | Keyword used as name      |

<!-- code-table:E0xxx end -->

#### E1xxx: Type Checking

<!-- code-table:E1xxx start -->

| Code  | Description                                       |
| ----- | ------------------------------------------------- |
| E1001 | Unknown variable                                  |
| E1002 | Type mismatch                                     |
| E1003 | Unknown type                                      |
| E1010 | Argument count mismatch                           |
| E1011 | Argument type mismatch                            |
| E1012 | Return type mismatch                              |
| E1013 | Function not found                                |
| E1014 | Unknown named argument                            |
| E1015 | Duplicate argument specification                  |
| E1020 | Cannot infer type                                 |
| E1021 | Type inference conflict                           |
| E1030 | Incomplete pattern                                |
| E1031 | Unreachable pattern                               |
| E1040 | Operation not supported                           |
| E1041 | Index out of bounds                               |
| E1042 | Field not found                                   |
| E1050 | Boolean operand required                          |
| E1051 | Logical NOT requires boolean operand              |
| E1052 | Invalid dereference                               |
| E1053 | Field access on non-struct                        |
| E1054 | Conditional type mismatch                         |
| E1055 | Constraint in non-generic context                 |
| E1060 | Type argument count mismatch                      |
| E1061 | Cannot instantiate generic                        |
| E1062 | const generic constraint failed                   |
| E1064 | Invalid binding position index                    |
| E1065 | Call on non-function value                        |
| E1071 | Type definition only allowed at module level      |
| E1081 | `?` is only allowed in functions returning Result |
| E1082 | `?` can only be used on Result expressions        |
| E1083 | `?` error type mismatch                           |
| E1090 | ✨ Unspeakable ✨                                 |
| E1091 | Invalid generic meta type                         |
| E1092 | Refinement type argument form illegal             |
| E1093 | Refinement argument count mismatch                |
| E1094 | Unused compile-time value parameter               |
| E1095 | Unknown interface                                 |
| E1096 | Interface argument count mismatch                 |
| E1097 | Interface member name conflict                    |
| E1098 | Interface method not implemented                  |
| E1099 | Interface method signature mismatch               |
| E1100 | Interface method duplicate implementation         |
| E1101 | Type does not implement interface                 |
| E1102 | Loop control statement outside loop               |
| E1103 | Brackets not allowed at type position             |

<!-- code-table:E1xxx end -->

> **RFC-011b Related (2026-09-22 Note)**:
> [RFC-011b: Operator Overloading](./011b-operator-overloading.md) When implementation lands, it
> will affect three places in this section — ① The text of `E1081` / `E1082` removes the word
> "Result" (`?` is changed to be judged by the `Try` interface, no longer bound to a specific type
> name), synchronized at phase 2 according to the "three-party consistency" process in this document
> (codes/*.rs ↔ locales ↔ code table); ② When the `Equal` pre-constraint (linear token) is not
> satisfied, the rejection diagnostic reuses the `E1101` (Type does not implement interface) family;
> ③ After phase 1 wiring, `Struct == Struct` transitions from `E6007` runtime error to compile-time
> determination, and the trigger surface of `E6007` contracts. The registered text in the table
> remains as is until the implementation lands.

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
| E2018 | Mutable/Immutable borrow conflict    |
| E2019 | Double free                          |
| E2020 | Use after free                       |
| E2027 | unsafe dereference                   |
| E2029 | Reference loop in spawn              |
| E2030 | Refinement type constraint violation |
| E2090 | Invalid signature                    |
| E2091 | Unknown signature type               |
| E2092 | Signature missing arrow              |
| E2093 | Duplicate parameter name             |
| E2094 | Generic parameter shadowing          |
| E2095 | Parameter name shadows generic       |

<!-- code-table:E2xxx end -->

> Reserved Code Note (2026-09-14 inventory, #251 release guideline): E2019 (Double free), E2020 (Use
> after free), E2027 (unsafe dereference), E2029 (Reference loop in spawn) have completed
> registration and have unit test anchors, but there is no reachable yx source code surface yet
> (explicit drop statement, Ptr dereference grammar, spawn ref loop creation path) — the claim of
> complete semantic correctness does not cover these four codes, and they are treated as "reserved"
> before implementation is completed.

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
| E3018 | Monomorphization instantiation failed             |
| E3019 | Top-level binding circular dependency             |
| E3020 | Missing program entry                             |
| E3021 | Entry is not a function                           |
| E3022 | Entry main signature mismatch                     |
| E3023 | Top-level does not allow executable statements    |

<!-- code-table:E3xxx end -->

#### E4xxx: Generics and Traits

<!-- code-table:E4xxx start -->

| Code  | Description                   |
| ----- | ----------------------------- |
| E4001 | Generic constraint violated   |
| E4002 | Trait not found               |
| E4003 | Trait implementation missing  |
| E4004 | Trait implementation conflict |
| E4005 | Associated type not found     |
| E4010 | Constant division by zero     |
| E4011 | Constant overflow             |
| E4012 | Constant recursion too deep   |
| E4014 | Constant evaluation failed    |
| E4018 | Refinement predicate violated |
| E4019 | Type equality does not hold   |
| E4020 | Proof function required       |

<!-- code-table:E4xxx end -->

> E4006/E8004 currently have no emission points (reserved codes): Sized constraints and optimization
> error paths are pending implementation; wire up to the real trigger surfaces during
> implementation.

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
| E6001 | Division by zero error       |
| E6003 | Array index out of bounds    |
| E6004 | Stack overflow               |
| E6005 | Assertion failed             |
| E6006 | Function not found (runtime) |
| E6007 | Runtime error                |
| E6008 | Key not found                |
| E6009 | Range step illegal           |
| E6010 | Integer parse failed         |
| E6011 | Float parse failed           |

<!-- code-table:E6xxx end -->

> **Code Table Revision (2026-08-09)**: The code table was originally defined according to the Rust
> semantic draft (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed),
> which does not match the actual implementation needs. YaoXiang has no concept of null pointer/heap
> allocation failure/type cast (value semantics + Rust memory safety), and the runtime overflow path
> has not implemented detection. After calibration:
>
> - E6002 removed (original Assertion failed moved to E6005; original null pointer semantics has no
>   language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (real trigger surface)
> - E6005 changed from Heap allocation failed to Assertion failed (std.assert real path)
> - E6006 changed from Runtime index out of bounds to Function not found (implementation has long
>   been this way)
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
| W1080 | Compile-time proof downgrade                 |

<!-- code-table:W1xxx end -->

> W code position rule: Isomorphic to E codes grouped by phase (W + phase thousands digit), W1xxx =
> type checking phase warnings.
>
> **Dead code semantics (#321 Decision B)**: `pub` definitions are external interfaces and are never
> reported — whether external consumers use them is beyond the single-file analysis boundary, so
> it's better to remain silent than to misreport. W1001/W1002/W1004/W1005 only target **private
> (non-pub) definitions**: reported when never referenced in reference reachability analysis
> starting from `main` and `pub` definitions. Methods (W1005) match by call point short name.
> bin/lib target semantics (warning for unused pub in bin) is a future extension (#289 Plan A),
> requiring project model support.
>
> **Unused imports (W1003)**: Detected by typecheck's use elaboration (pass2 registers imported
> local names, considered used when hit by expression resolution or type annotation position),
> covering both whole imports (`use std.io` → module alias) and named imports
> (`use std.io.{print}`).
>
> **Emission channel**: W code diagnostics are tagged with `Severity::Warning` by default by the
> builder based on the W prefix (explicit specification takes priority), collected and presented on
> the same track as errors (`warning[W####]` prefix rendered), but does not block compilation or
> affect the success exit code. `yaoxiang check --deny-warnings` escalates warnings to failure
> (exits with a non-zero code when warnings exist), used for CI strict mode. Per-code suppression
> (allow attributes, etc.) is a subsequent extension item.

### Message Quality Specification

> This section is introduced by the message unification and quality revision (2026-09-03). Enforced
> by `scripts/audit_diagnostics.py` in CI.

1. **Message Unification**: All user-visible diagnostic messages must go through the authoritative
   registry shortcut methods + locales template rendering, and the code only passes structured
   parameters. It is forbidden to bypass the registry and directly construct native values like
   `Diagnostic::error(...)` — this path bypasses code validation and i18n.
2. **Code Validity**: Use of unregistered codes and pseudo codes (such as `E_INTERNAL`) is
   prohibited; the code literal at the use point must already be defined in the registry. Internal
   errors uniformly fall to E8001 (`internal_error`).
3. **Type Display**: Type Display must distinguish between pre- and post-instantiation forms
   (`Expected 'Container', found 'Container'` with bare names is indistinguishable).
4. **Solver Internal State Isolation**: Solver intermediate state TypeVar (Display form `t<N>`) must
   not enter user-visible messages. Test anchor: `test_type_error_message_no_solver_typevar_leak`.
5. **E8xxx Boundary**: E8xxx is only used for compiler internal consistency issues (ICE).
   User-fixable errors are prohibited from using E8001 as a fallback; ICE messages must include
   minimal reproduction guidance.

---

### Runtime Error Values and Code Integration

> This section is introduced by the runtime Error value with code revision (2026-09-03). The
> E6xxx/E7xxx semantic space simultaneously carries two channels, with the same code space but
> different presentation channels.

#### Two Channels

| Channel                         | Carrier                                            | Presentation                                             |
| ------------------------------- | -------------------------------------------------- | -------------------------------------------------------- |
| Compiler/CLI Diagnostic Channel | `ExecutorError` and other host-level hard errors   | stderr `error[E####]:` (already wired E6003/E6005/E6007) |
| In-Program Error Value Channel  | std library `Result(T, Error)` Err carrier `Error` | Language value, consumed by program match/comparison     |

#### Error Structure (From v0.8, Breaking Change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbers of this specification, in string form (e.g., `"E6008"`).
- **Stability Contract**: Allocated codes have unchanged semantics across versions; the same
  semantics does not reuse deleted codes (E6002 precedent).
- **Consumption Surface**: In-program `e.code == "E6xxx"` comparison is the only programmable
  determination contract; `yaoxiang explain E6xxx` documentation integration; toolchain (LSP / DAP,
  see RFC-034) uses the code as exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined Errors**: The E in `Result(T, E)` is a generic parameter, and serious modeling uses
  user-defined types; std `Error` is only a convenient fallback carrier, and its code system does
  not constrain user E types.

#### Code Allocation Rules

1. Runtime error value codes and compiler diagnostic codes share the E6xxx/E7xxx space, and new
   codes are allocated according to **real trigger surfaces**, not reserved for imagined scenarios.
2. Register first, then use: New codes must enter the authoritative registry and pass the
   three-party consistency check (codes/*.rs ↔ locales ↔ this document's code table) before being
   emitted. The registration source of runtime error value codes is the `RUNTIME_ERROR_CODES` table
   in `src/std/result.rs` (subject to the same `build.rs compile-time gate + `tools/code-tables`
   checks as diagnostic codes).
3. E7xxx is reserved for std.io / std.net error value slots (currently empty, enabled when io/net
   becomes Result-based).
4. Emission point: Each std module constructs an Error value via `error_new(code, message)`; the
   consumption side uses `std.result.unwrap_err` to extract the Err carrier, and
   `std.result.code/message` to read fields.

#### Evolution Path (Line C, Not Implemented)

After pattern matching completeness (RFC-039) is implemented, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, and `code` becomes a property derived from kind (variant
definition location is the code registry). The code stability contract in this section remains
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

#### Template Placeholders

##### Predefined Placeholders (Common)

| Placeholder  | Use                                                      | Example                             |
| ------------ | -------------------------------------------------------- | ----------------------------------- |
| `{name}`     | Variable name/type name/trait name and other identifiers | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                                            | `Expected type '{expected}'`        |
| `{found}`    | Actual/found type                                        | `, found type '{found}'`            |
| `{method}`   | Method name                                              | `Method {method} is not a function` |
| `{trait}`    | Trait name                                               | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                              | `Invalid path: {path}`              |
| `{ty}`       | Type expression                                          | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                                   | `Internal error: {message}`         |

##### Arbitrary Key Support

**params supports arbitrary keys, not limited to predefined ones.** The caller can pass any `key`:

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

> **Note**: Not all error codes use placeholders. Some error codes (such as E0001) are static
> messages and do not require parameters.

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
# 错误消息语言，可选：en, zh, ja, ...
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
4. Compiler creates an I18nRegistry based on the selected language (once)
5. All errors use that I18nRegistry to render messages
```

#### Key to Zero Lookup Overhead

**Rendering happens when compiling the user project, not at runtime.**

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

| Component                    | Responsibility                       | Rendering Timing            |
| ---------------------------- | ------------------------------------ | --------------------------- |
| `I18nRegistry`               | Provide templates and display text   | When compiling user project |
| `DiagnosticBuilder.render()` | Template + parameters → final string | When compiling user project |
| `Diagnostic.message`         | Rendered string                      | Store final result          |
| AOT Binary                   | Contains final string                | Used directly at runtime    |

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

Error severity is managed through the `DiagnosticLevel` enum, decoupled from error code numbering:

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

Since this RFC designs the error code system from scratch, there is no backward compatibility issue.

**Future Migration Strategy** (for reference in subsequent versions):

1. Maintain the mapping from old error codes to new error codes.
2. Display both old and new codes during the migration period.
3. Provide a deprecation timeline.

---

## Implementation Strategy

### Phase 1: Error Code Infrastructure

1. Create the `src/diagnostics/` directory structure.
2. Implement the `ErrorCode` enum.
3. Implement `Diagnostic` and `DiagnosticLevel`.
4. Create the resource file directory and example JSON.

### Phase 2: explain Command

1. Implement the `yaoxiang explain` CLI command.
2. Support `--lang` and `--json` options.
3. Integrate resource file loading.
4. Implement parameter template rendering.

### Phase 3: Compile-time Integration

1. Update all error reporting points to use the new system.
2. Implement message template parameter injection.
3. Add language priority logic.
4. Add unit test coverage.

### Phase 4: IDE/LSP Integration

1. LSP server integrates the explain JSON output.
2. Display error code links in the IDE.
3. Hover to display error explanations.
4. Provide quick-fix suggestions.

---

## Appendix

### Complete Error Code Quick Reference Table

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
# 英文 (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# 中文 (zh-CN)
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
