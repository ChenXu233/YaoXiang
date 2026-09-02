---
title: 'RFC 013: Error Code Specification'
status: 'accepted'
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
Rust-like single-level numbering system, combined with JSON resource files to achieve multi-language
support, and providing error explanation functionality through the `yaoxiang explain` command.

## Motivation

### Why do we need standardized error codes?

1. **User Experience**: Users can quickly determine the error type and severity from the error code
2. **Documentation Organization**: Grouping by category facilitates writing and maintaining error
   reference documentation
3. **Tool Integration**: IDE/LSP can provide quick fix suggestions and documentation links based on
   error codes
4. **Internationalization Support**: Error messages are separated from codes, facilitating
   multi-language translation

### Design Goals

- **Concise**: Single-level numbering, users don't need to remember complex classification rules
- **Friendly**: Rust-like error message format with help information and examples
- **Extensible**: Resource file driven, easy to add new errors and new languages
- **Tool-friendly**: explain command + JSON output, supports IDE/LSP integration

---

## Proposal

### Core Design: Single-Level Numbering System

Adopt a four-digit numbering system, grouped by compilation phase:

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
| **1** | E1xxx | Type checking               |
| **2** | E2xxx | Semantic analysis           |
| **3** | E3xxx | Code generation             |
| **4** | E4xxx | Generics and traits         |
| **5** | E5xxx | Modules and imports         |
| **6** | E6xxx | Runtime errors              |
| **7** | E7xxx | I/O and system errors       |
| **8** | E8xxx | Internal compiler errors    |
| **9** | E9xxx | Reserved/experimental       |

### Error Category Enum

```rust
/// 错误类别
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

**Core Principle**: Separation of error code definition from display copy

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without display copy
- `locales/*.json`: Display copy for each language (title, message, help, error codes as nested
  objects)
- `DiagnosticBuilder`: Generic builder, replacing the trait-per-error design

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

| Feature                   | Description                                              |
| ------------------------- | -------------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` generic for all error codes      |
| **Type Safety**           | Shortcut methods ensure parameter correctness            |
| **Self-documenting**      | `E1001::unknown_variable(name)` is self-explanatory      |
| **Template Separation**   | Message templates are separated from code, easy for i18n |
| **Zero Runtime Overhead** | Compile-time rendering, no table lookup in AOT binaries  |

---

### Error Macro Simplification

#### error! Macro (Auto-inject Context)

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

#### Manual Use of Builder

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

| Code  | Error Type                | Description                                               |
| ----- | ------------------------- | --------------------------------------------------------- |
| E0001 | Invalid character         | Source code contains illegal characters                   |
| E0002 | Invalid number literal    | Number literal format is incorrect                        |
| E0003 | Unterminated string       | Multi-line string missing closing quote                   |
| E0004 | Invalid character literal | Character literal is incorrect                            |
| E0010 | Expected token            | Expected specific token during syntax analysis            |
| E0011 | Unexpected token          | Encountered unexpected token                              |
| E0012 | Invalid syntax            | Expression/statement syntax error                         |
| E0013 | Mismatched brackets       | Parentheses, square brackets, or curly braces don't match |
| E0014 | Missing semicolon         | Statement missing trailing semicolon                      |
| E0016 | Expected expression       | Expected expression                                       |
| E0018 | Keyword as name           | Keyword cannot be used as a name                          |

#### E1xxx: Type Checking

| Code  | Error Type                                             | Description                                             |
| ----- | ------------------------------------------------------ | ------------------------------------------------------- |
| E1001 | Unknown variable                                       | Referenced variable is not defined                      |
| E1002 | Type mismatch                                          | Expected type does not match actual type                |
| E1003 | Unknown type                                           | Referenced type does not exist                          |
| E1010 | Parameter count mismatch                               | Function call parameter count does not match definition |
| E1011 | Parameter type mismatch                                | Parameter type check failed                             |
| E1012 | Return type mismatch                                   | Function return value type is incorrect                 |
| E1013 | Function not found                                     | Called undefined function                               |
| E1020 | Cannot infer type                                      | Type cannot be inferred from context                    |
| E1021 | Type inference conflict                                | Multiple constraints lead to type contradiction         |
| E1030 | Pattern non-exhaustive                                 | match expression does not cover all cases               |
| E1031 | Unreachable pattern                                    | Pattern that can never be matched                       |
| E1040 | Operation not supported                                | Type does not support this operation                    |
| E1041 | Index out of bounds                                    | Array/list index out of range                           |
| E1042 | Field not found                                        | Accessing non-existent struct field                     |
| E1050 | Boolean operand required                               | Boolean operand required                                |
| E1051 | Logical NOT requires boolean operand                   | Logical NOT requires boolean operand                    |
| E1052 | Invalid dereference                                    | Invalid dereference                                     |
| E1053 | Non-struct field access                                | Field access on non-struct                              |
| E1054 | Conditional type mismatch                              | Conditional type mismatch                               |
| E1055 | Constraint in non-generic context                      | Constraint appears in non-generic context               |
| E1060 | Type parameter count mismatch                          | Type parameter count mismatch                           |
| E1061 | Cannot instantiate generic                             | Cannot instantiate generic                              |
| E1062 | Const generic constraint failed                        | const generic constraint failed                         |
| E1064 | Invalid binding position                               | Invalid binding position index (RFC-004)                |
| E1071 | Type definitions are only allowed at module level      | Type definitions are only allowed at module level       |
| E1081 | `?` can only be used within functions returning Result | `?` is only allowed in functions returning Result       |
| E1082 | `?` can only be used with Result expressions           | `?` can only be used with Result expressions            |
| E1083 | Error type mismatch for `?`                            | Error type mismatch for `?`                             |
| E1090 | Type universe easter egg                               | Type: Type = Type easter egg (Note level)               |
| E1091 | Invalid generic meta type                              | Invalid generic meta type                               |
| E1092 | Invalid refinement type argument form                  | Illegal refinement type argument form                   |
| E1093 | Refinement argument count mismatch                     | Refinement argument count mismatch                      |
| E1094 | Unused compile-time value parameter                    | Unused compile-time value parameter                     |
| E1095 | Unknown interface                                      | Unknown interface                                       |
| E1096 | Interface arity mismatch                               | Interface parameter count mismatch                      |
| E1097 | Interface member name conflict                         | Interface member name conflict                          |
| E1098 | Interface method not implemented                       | Interface method not implemented                        |
| E1099 | Interface method signature mismatch                    | Interface method signature mismatch                     |
| E1100 | Duplicate interface method implementation              | Duplicate interface method implementation               |
| E1101 | Type does not implement interface                      | Type does not implement interface                       |
| E1102 | Loop control statement outside of a loop               | Loop control statement outside of a loop                |

#### E2xxx: Semantic Analysis

| Code  | Error Type                        | Description                                 |
| ----- | --------------------------------- | ------------------------------------------- |
| E2001 | Scope error                       | Variable not in current scope               |
| E2002 | Duplicate definition              | Duplicate definition in same scope          |
| E2003 | Lifetime error                    | Lifetime constraint not satisfied           |
| E2010 | Immutable assignment              | Attempt to modify immutable variable        |
| E2011 | Uninitialized use                 | Use of uninitialized variable               |
| E2012 | Mutability conflict               | Mutable reference used in immutable context |
| E2013 | Variable shadowing                | Variable shadowing                          |
| E2014 | Use of moved value                | Use of moved value                          |
| E2016 | Immutable assignment              | Immutable assignment                        |
| E2018 | Mutable/immutable borrow conflict | Mutable/immutable borrow conflict           |
| E2019 | Double free                       | Double free                                 |
| E2020 | Use after free                    | Use after free                              |
| E2027 | Unsafe dereference                | unsafe dereference                          |
| E2090 | Invalid signature                 | Function signature parsing error            |
| E2091 | Unknown type in signature         | Unknown type in signature                   |
| E2092 | Missing arrow in signature        | Signature missing return arrow              |
| E2093 | Duplicate parameter name          | Duplicate parameter name                    |
| E2094 | Generic parameter shadowing       | Generic parameter shadowing                 |
| E2095 | Parameter name shadows generic    | Parameter name shadows generic              |

#### E3xxx: Code Generation

| Code  | Error Type                             | Description                                    |
| ----- | -------------------------------------- | ---------------------------------------------- |
| E3004 | Unsupported iterator                   | Unsupported iterator                           |
| E3005 | IR generation error                    | IR generation internal error                   |
| E3006 | Unresolved variable                    | Variable not resolved during IR generation     |
| E3007 | Top-level initializer must be constant | Top-level binding initializer must be constant |
| E3014 | Register overflow                      | Register overflow                              |
| E3017 | Invalid operand (code generation)      | Invalid operand (code generation)              |

#### E4xxx: Generics and Traits

| Code  | Error Type                              | Description                                           |
| ----- | --------------------------------------- | ----------------------------------------------------- |
| E4001 | Generic parameter mismatch              | Generic parameter count/type mismatch                 |
| E4002 | Trait bound violated                    | trait constraint not satisfied                        |
| E4003 | Associated type error                   | Associated type definition/use error                  |
| E4004 | Duplicate trait implementation          | Duplicate implementation of same trait                |
| E4005 | Trait not found                         | Required trait not found                              |
| E4006 | Sized bound violated                    | Sized bound not satisfied (reserved, not implemented) |
| E4010 | Division by zero in constant expression | Division by zero in constant expression               |
| E4011 | Constant overflow                       | Constant overflow                                     |
| E4012 | Constant recursion too deep             | Constant recursion too deep                           |
| E4014 | Constant evaluation failed              | Constant evaluation failed                            |
| E4018 | Refinement predicate violation          | Refinement predicate violation                        |
| E4019 | Type equality does not hold             | Type equality does not hold                           |

#### E5xxx: Modules and Imports

| Code  | Error Type            | Description                                    |
| ----- | --------------------- | ---------------------------------------------- |
| E5001 | Module not found      | Imported module does not exist                 |
| E5002 | Cyclic import         | Cyclic dependency between modules              |
| E5003 | Symbol not exported   | Attempt to access non-exported symbol          |
| E5004 | Invalid module path   | Module path format error                       |
| E5005 | Private access        | Access to private symbol                       |
| E5006 | Duplicate import      | Duplicate import                               |
| E5007 | Module export listing | Module export listing (companion hint message) |

#### E6xxx: Runtime Errors

| Code  | Error Type                  | Description                                 |
| ----- | --------------------------- | ------------------------------------------- |
| E6001 | Division by zero            | Integer division by zero                    |
| E6002 | ~~Assertion failed~~        | ~~Reserved (no language concept, removed)~~ |
| E6003 | Runtime index out of bounds | Runtime index out of bounds (#280 wiring)   |
| E6004 | Stack overflow              | Stack space exhausted                       |
| E6005 | Assertion failed            | assert failure (#280 wiring)                |
| E6006 | Function not found          | Function not found at runtime               |
| E6007 | Runtime error (generic)     | Generic runtime error                       |
| E6008 | Key not found               | Dict key missing (#299 §4)                  |

> **#280 Revision (2026-08-09)**: The code table was originally defined according to a Rust semantic
> draft (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed), which didn't
> match actual implementation needs. YaoXiang has no null pointer/heap allocation failure/type cast
> concept (value semantics + Rust memory safety), and runtime overflow paths haven't implemented
> detection. After calibration:
>
> - E6002 deleted (original Assertion failed moved to E6005; original null pointer semantics have no
>   language concept)
> - E6003 changed from Arithmetic overflow to Runtime index out of bounds (real trigger surface,
>   #279/#271)
> - E6005 changed from Heap allocation failed to Assertion failed (real path for std.assert)
> - E6006 changed from Runtime index out of bounds to Function not found (implementation has been
>   like this, #255)
> - E6007 changed from Type cast failed to generic Runtime error (unified landing point for unmapped
>   ExecutorError variants)

#### E7xxx: I/O and System Errors

| Code  | Error Type        | Description                       |
| ----- | ----------------- | --------------------------------- |
| E7001 | File not found    | Attempt to read non-existent file |
| E7002 | Permission denied | Insufficient file permissions     |
| E7003 | I/O error         | Generic I/O error                 |
| E7004 | Network error     | Network operation failed          |

#### E8xxx: Internal Compiler Errors

| Code  | Error Type              | Description                                             |
| ----- | ----------------------- | ------------------------------------------------------- |
| E8001 | Internal compiler error | Internal compiler error                                 |
| E8002 | Codegen error           | IR/bytecode generation failed                           |
| E8003 | Unimplemented feature   | Used unimplemented feature                              |
| E8004 | Optimization error      | Compiler optimization error (reserved, not implemented) |

#### W1xxx: Warning Codes

| Code  | Warning Type                                 | Description                                  |
| ----- | -------------------------------------------- | -------------------------------------------- |
| W1001 | Unused exported function                     | Unused exported function                     |
| W1002 | Unused exported type                         | Unused exported type                         |
| W1003 | Unused import                                | Unused import                                |
| W1004 | Unused exported variable                     | Unused exported variable                     |
| W1005 | Unused exported method                       | Unused exported method                       |
| W1063 | Const generic constraint cannot be evaluated | Const generic constraint cannot be evaluated |

> W code position rule: Same structure as E codes, grouped by phase (W + phase thousand segment),
> W1xxx = type checking phase warnings.
>
> **Emission Channel (#321 M2)**: W code diagnostics are labeled with `Severity::Warning` by default
> by the builder based on the W prefix (explicit specification takes priority), collection and
> presentation are on the same track as errors (`warning[W####]` prefix rendering), but do not block
> compilation and do not affect successful exit code. `yaoxiang check --deny-warnings` escalates
> warnings to failures (exits with non-zero code when warnings exist), used for CI strict mode.
> Per-code suppression (allow attribute, etc.) is a future extension.

---

### Runtime Error Value and Code Integration

> This section is introduced by #323 (M4 runtime Error value with code, 2026-09-03). The E6xxx/E7xxx
> semantic space carries two channels simultaneously, with the same code space but different
> presentation channels.

#### Two Channels

| Channel                              | Carrier                                            | Presentation                                                       |
| ------------------------------------ | -------------------------------------------------- | ------------------------------------------------------------------ |
| Compiler/CLI diagnostic channel      | `ExecutorError` and other host-level hard errors   | stderr `error[E####]:` (#280/#281 already wired E6003/E6005/E6007) |
| Program-internal error value channel | std library `Result(T, Error)` Err carrier `Error` | Language value, consumed by program match/comparison               |

#### Error Structure (from v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` reuses the E6xxx/E7xxx numbers from this specification, in string form (e.g., `"E6008"`).
- **Stability Contract**: Assigned codes maintain semantics across versions; deleted codes are not
  reused for the same semantics (E6002 precedent).
- **Consumption Surface**: Program-internal `e.code == "E6xxx"` comparison is the only programmable
  judgment contract; `yaoxiang explain E6xxx` documentation integration; toolchain (LSP / DAP, see
  RFC-034) uses the code as exceptionId.
- **Accessors**: `std.result.code(e)` / `std.result.message(e)`.
- **User-defined Errors**: `Result(T, E)` has E as a generic parameter, with serious modeling via
  user-defined types; std `Error` is only a convenient fallback carrier, its code system does not
  constrain user E types.

#### Code Allocation Rules

1. Runtime error value codes and compile-time diagnostic codes share the E6xxx/E7xxx space, new
   codes are allocated according to **real trigger surfaces**, not reserved for imagined scenarios.
2. Register before use: New codes enter the authoritative registry and pass three-party consistency
   verification (codes/*.rs ↔ locales ↔ this document's code table) before they can be emitted.
3. E7xxx is reserved for std.io / std.net error value segments (currently empty, to be enabled when
   io/net Result-ification happens).

#### Evolution Path (Line C, Not Implemented)

After pattern matching completeness (RFC-039) lands, `Error` can be upgraded to
`{ kind: ErrorKind, message: String }`, with `code` becoming a property derived from kind (variant
definition is the code registry). During the evolution period, the code stability contract in this
section remains unchanged; this upgrade is an independent decision and does not constitute a
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

##### Arbitrary Key Support

**params supports arbitrary keys, not limited to predefined ones**. The caller can pass any `key`:

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

> **Note**: Not all error codes use placeholders. Some error codes (like E0001) are static messages
> without parameters.

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
4. The compiler creates an I18nRegistry based on the selected language (once)
5. All errors use this I18nRegistry to render messages
```

#### Key to Zero Table Lookup Overhead

**Rendering happens when compiling the user project, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Rust compiles the YaoXiang compiler                            │
│                                                                           │
│  JSON is packaged into the compiler binary                                │
│  Purpose: explain command can directly read i18n data                    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: YaoXiang compiles user project (rendering happens here)        │
│                                                                           │
│  When error! macro is called:                                            │
│  1. Read yaoxiang.toml to get language preference                        │
│  2. Load corresponding language's i18n JSON from compiler binary         │
│  3. Template + params → render() → "Unknown variable: 'x'"              │
│  4. Diagnostic.message = rendered string                                 │
│                                                                           │
│  AOT binary stores final string directly, no template, no table lookup   │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runtime                                            │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Directly outputs final string, no table lookup                       │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                     | Render Timing               |
| ---------------------------- | ---------------------------------- | --------------------------- |
| `I18nRegistry`               | Provide templates and display copy | When compiling user project |
| `DiagnosticBuilder.render()` | Template + params → final string   | When compiling user project |
| `Diagnostic.message`         | Rendered string                    | Stores final result         |
| AOT binary                   | Contains final strings             | Used directly at runtime    |

---

### Error Message Format

Error messages use the following format:

```
error[E####]: <brief description>
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

Error severity is managed through the `DiagnosticLevel` enum, decoupled from the error code number:

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

**Future Migration Strategy** (for reference in subsequent versions):

1. Maintain mapping from old error codes to new error codes
2. Display both old and new codes during the migration period
3. Provide deprecation schedule

---

## Implementation Strategy

### Phase One: Error Code Infrastructure

1. Create `src/diagnostics/` directory structure
2. Implement `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create resource file directory and example JSON

### Phase Two: explain Command

1. Implement `yaoxiang explain` CLI command
2. Support `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase Three: Compile-time Integration

1. Update all error reporting points to use the new system
2. Implement message template parameter injection
3. Add language priority logic
4. Unit test coverage

### Phase Four: IDE/LSP Integration

1. LSP server integrates explain JSON output
2. Display error code links in IDE
3. Hover to show error explanation
4. Quick fix suggestions

---

## Appendix

### Complete Error Code Quick Reference

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

### Supported Languages

| Code  | Language     | Status  |
| ----- | ------------ | ------- |
| en-US | English (US) | Default |
| zh-CN | 简体中文     | Planned |

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
