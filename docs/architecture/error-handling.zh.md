# 错误处理系统设计

## 概述

YaoXiang 编译器的错误处理系统采用分层设计：

1. **错误码注册表** - 编译期确定的错误码元数据（含模板）
2. **i18n 模板系统** - 编译期选择语言，打包进用户 AOT 二进制
3. **Diagnostic 结构** - 运行时生成的完整诊断信息
4. **统一 Builder** - 通用错误构建器，替代 trait-per-error 设计

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           错误处理系统架构                                  │
└─────────────────────────────────────────────────────────────────────────────┘

  yaoxiang.toml           编译期选择               运行时
  (语言偏好)         ─────────────────────>   Diagnostic {
                                                     code: "E1001",
                                                     message: "未知变量：'x'",  // 编译期已渲染
                                                     help: "检查变量名是否拼写正确",
                                                     span: ...
                                                   }
```

## 设计核心原则

**编译期花成本，用户零成本。**

- Rust 编译 YaoXiang 编译器：所有语言的 error 消息模板都打包进编译器
- YaoXiang 编译用户项目：根据 yaoxiang.toml 配置，编译期创建对应语言的 I18nRegistry
- AOT 编译阶段：模板 + 渲染后的消息内联进用户二进制
- 用户程序运行时：无需任何查表，直接显示已渲染的错误消息

## 1. 错误码注册表

### 文件结构

```
src/util/diagnostic/codes/
├── mod.rs              # 主模块，定义 ErrorCodeDefinition + I18nRegistry
├── builder.rs          # 通用 DiagnosticBuilder
├── e0xxx.rs            # 词法/解析错误 (E0001-E0014)
├── e1xxx.rs            # 类型检查错误 (E1001-E1042)
├── e2xxx.rs            # 语义分析错误 (E2001-E2012)
├── e4xxx.rs            # 泛型与特质错误 (E4001-E4004)
├── e5xxx.rs            # 模块与导入错误 (E5001-E5004)
├── e6xxx.rs            # 运行时错误 (E6001-E6005)
├── e7xxx.rs            # I/O 错误 (E7001-E7004)
├── e8xxx.rs            # 内部错误 (E8001-E8003)
└── i18n/               # 国际化模板文件
    ├── en.json         # 英文消息模板
    └── zh.json         # 中文消息模板
```

### 错误类别

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

### 错误码定义（通用 Builder 模式）

**核心原则**：错误码定义与展示文案分离

- `ErrorCodeDefinition`：错误码元数据（code、category、template），不含展示文案
- `i18n/*.json`：各语言展示文案（title、message、help）

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

    /// 验证所有占位符都有对应参数（缺失则 panic）
    fn validate_params(&self) {
        // ... 同前
    }
}

/// 从 i18n 获取展示文案
impl ErrorCodeDefinition {
    /// 获取标题
    pub fn title(&self, i18n: &I18nRegistry) -> &'static str {
        i18n.get_title(self.code)
    }

    /// 获取描述
    pub fn message(&self, i18n: &I18nRegistry) -> &'static str {
        i18n.get_message(self.code)
    }

    /// 获取帮助信息
    pub fn help(&self, i18n: &I18nRegistry) -> String {
        i18n.render_help(self.code, &[])
    }
}
```

### 使用方式

```rust
// checking/mod.rs

use crate::util::diagnostic::codes::{ErrorCodeDefinition, E1001};

let diagnostic = ErrorCodeDefinition::find("E1001")
    .builder()  // 获取该错误码的预配置 Builder
    .param("name", var_name)
    .at(span)
    .build(&i18n_registry);

// 简化：直接在 ErrorCodeDefinition 上扩展方法
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));
```

### 每个错误码的快捷方法（自动生成）

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

### 错误码定义示例

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

### 设计优势

| 特性 | 说明 |
|------|------|
| **单一 Builder** | 一个 `DiagnosticBuilder` 通用所有错误码 |
| **类型安全** | 快捷方法确保参数正确性 |
| **自文档** | `E1001::unknown_variable(name)` 一目了然 |
| **模板分离** | 消息模板与代码分离，易于 i18n |
| **零运行时开销** | 编译期渲染，AOT 二进制无查表 |

## 2. i18n 消息文件

### 设计原则

- 所有语言的展示文案（title、message、help）和模板都放在 i18n 文件
- `ErrorCodeDefinition` 只保存模板，减少重复
- 示例代码和错误输出用于文档自动生成

### 文件结构

```json
// diagnostic/codes/i18n/en.json
{
  "E1001": {
    "title": "Unknown variable",
    "message": "Referenced variable is not defined",
    "template": "Unknown variable: '{name}'",
    "help": "Check if the variable name is spelled correctly, or define it first",
    "example": "x = ¥100;  # ¥ is not a valid character",
    "error_output": "error[E0001]: Invalid character\n  --> example.yx:1:10\n   |\n 1 | x = ¥100;\n   |          ^ illegal character '¥'"
  },
  "E1002": {
    "title": "Type mismatch",
    "message": "Expected type does not match actual type",
    "template": "Expected type '{expected}', found type '{found}'",
    "help": "Use the correct type or add a type conversion",
    "example": "x: Int = \"hello\";  # String assigned to Int",
    "error_output": "error[E1002]: Type mismatch\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ expected 'Int', found 'String'"
  }
}
```

```json
// diagnostic/codes/i18n/zh.json
{
  "E1001": {
    "title": "未知变量",
    "message": "引用的变量未定义",
    "template": "未知变量：'{name}'",
    "help": "检查变量名是否拼写正确，或先定义它",
    "example": "x = ¥100;  # ¥ 不是有效字符",
    "error_output": "error[E0001]: 无效字符\n  --> example.yx:1:10\n   |\n 1 | x = ¥100;\n   |          ^ 无效字符 '¥'"
  },
  "E1002": {
    "title": "类型不匹配",
    "message": "期望类型与实际类型不匹配",
    "template": "期望类型 '{expected}'，实际类型 '{found}'",
    "help": "使用正确的类型或添加类型转换",
    "example": "x: Int = \"hello\";  # String 赋值给 Int",
    "error_output": "error[E1002]: 类型不匹配\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ 期望 'Int'，找到 'String'"
  }
}
```

### 模板占位符

模板使用 `{key}` 语法占位符，在渲染时替换为实际值。

#### 预定义占位符（常用）

| 占位符 | 用途 | 示例 |
|--------|------|------|
| `{name}` | 变量名/类型名/特质名等标识符 | `Unknown variable: '{name}'`, `Cannot find trait definition: {name}` |
| `{expected}` | 期望类型 | `Expected type '{expected}', found type '{found}'` |
| `{found}` | 实际/找到的类型 | 同上 |
| `{method}` | 方法名 | `Method {method} is not a function type` |
| `{trait}` | 特质名 | `Trait {trait} is not object-safe` |
| `{path}` | 模块路径 | `Invalid trait path: {path}` |
| `{ty}` | 类型表达式 | `Invalid higher-rank type syntax: {ty}` |
| `{message}` | 内部错误消息 | `Internal error: {message}` |

#### 任意 key 支持

**params 支持任意 key，不限于预定义**。调用方可以传任意 `key`：

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

> **注意**：并非所有错误码都使用占位符。部分错误码（如 E0001、E2001 等）是静态消息，无需参数。

### I18nRegistry 实现

```rust
// diagnostic/codes/i18n/mod.rs

use std::collections::HashMap;
use std::sync::LazyLock;

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

/// JSON 结构（与 i18n/*.json 对应）
#[derive(serde::Deserialize)]
struct ErrorInfoJson {
    title: String,
    message: String,
    help: String,
    example: Option<String>,
    error_output: Option<String>,
}

impl I18nRegistry {
    /// 根据语言代码获取注册表
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// 英文模板（编译期从 JSON 加载）
    fn en() -> Self {
        // 直接从 JSON 文件加载
        static DATA: LazyLock<HashMap<&'static str, ErrorInfoJson>> =
            LazyLock::new(|| {
                serde_json::from_str(include_str!("i18n/en.json")).unwrap()
            });

        let mut titles = HashMap::new();
        let mut messages = HashMap::new();
        let mut helps = HashMap::new();
        let mut examples = HashMap::new();
        let mut error_outputs = HashMap::new();

        for (code, info) in DATA.iter() {
            titles.insert(code, &info.title);
            messages.insert(code, &info.message);
            helps.insert(code, &info.help);
            if let Some(ref ex) = info.example {
                examples.insert(code, ex);
            }
            if let Some(ref out) = info.error_output {
                error_outputs.insert(code, out);
            }
        }

        Self {
            titles,
            messages,
            helps,
            examples,
            error_outputs,
        }
    }

    /// 中文模板
    fn zh() -> Self {
        // 类似地从 zh.json 加载
        Self { /* ... */ }
    }

    /// 获取错误信息（用于 explain 指令和文档生成）
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// 获取标题
    pub fn get_title(&self, code: &str) -> &'static str {
        self.titles.get(code).copied().unwrap_or(code)
    }

    /// 获取描述
    pub fn get_message(&self, code: &str) -> &'static str {
        self.messages.get(code).copied().unwrap_or("")
    }

    /// 获取帮助信息
    pub fn get_help(&self, code: &str) -> &'static str {
        self.helps.get(code).copied().unwrap_or("")
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

    /// 渲染帮助信息
    pub fn render_help(&self, help: &'static str, params: &[(&str, String)]) -> String {
        self.render(help, params)
    }
}
```

### 零查表开销的关键

**渲染发生在编译用户项目时，不是运行时。**

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

### 职责分离

| 组件 | 职责 | 渲染时机 |
|------|------|----------|
| `I18nRegistry` | 提供模板和展示文案 | 编译用户项目时 |
| `DiagnosticBuilder.render()` | 模板 + 参数 → 最终字符串 | 编译用户项目时 |
| `Diagnostic.message` | 已渲染的字符串 | 存储最终结果 |
| AOT 二进制 | 包含最终字符串 | 运行时直接用 |

```rust
// 编译器编译用户项目时
let i18n = I18nRegistry::new("zh");  // 只调用一次
let diagnostic = builder.build(&i18n);  // 模板渲染在编译期完成

// 用户程序运行时（假设有错误）
println!("{}", diagnostic.message);  // 直接输出 "未知变量：'x'"，无需任何查表
```

## 3. yaoxiang.toml 配置

### 项目级配置

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# 错误消息语言，可选：en, zh, ja, ...
default = "zh"
```

### 用户级配置

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

### 编译期语言选择

```
1. 读取项目级 yaoxiang.toml 的 language.default
2. 若未配置，读取用户级 ~/.yaoxiang/yaoxiang.toml
3. 若都未配置，默认使用 "en"
4. 编译器根据选择的语言创建 I18nRegistry（一次）
5. 所有错误使用该 I18nRegistry 渲染消息
```

## 4. Diagnostic 结构

```rust
// diagnostic/error.rs

/// 诊断严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// 诊断信息（运行时直接使用，message 已渲染完成）
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,       // 严重级别
    pub code: String,           // 错误码
    pub message: String,        // 完整消息（编译期已渲染）
    pub help: String,           // 帮助信息（编译期已渲染）
    pub span: Option<Span>,     // 位置信息
    pub related: Vec<Diagnostic>,// 相关诊断
}

impl Diagnostic {
    /// 创建错误诊断（message 已渲染）
    pub fn error(code: String, message: String, help: String, span: Option<Span>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message,
            help,
            span,
            related: Vec::new(),
        }
    }
}
```

## 5. 错误宏简化

### 自动注入上下文

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

### 手动使用 Builder

```rust
// 需要手动控制时
E1001::unknown_variable(&var_name)
    .at(my_span)           // 自定义 span
    .build(&custom_i18n)   // 自定义 i18n
```

## 6. 工作流程总结

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           完整编译流程                                      │
└─────────────────────────────────────────────────────────────────────────────┘

  ┌─────────────────────────────────────────────────────────────────────────┐
  │  阶段 1: Rust 编译 YaoXiang 编译器                                       │
  │                                                                           │
  │  所有语言的 i18n JSON 打包进编译器二进制                                  │
  │  - i18n/en.json                                                          │
  │  - i18n/zh.json                                                          │
  │                                                                           │
  │  目的：explain 指令能直接从二进制读取 i18n 数据                           │
  └─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
  ┌─────────────────────────────────────────────────────────────────────────┐
  │  阶段 2: YaoXiang 编译用户项目（渲染发生在这里）                          │
  │                                                                           │
  │  2.1 读取 yaoxiang.toml 获取语言偏好                                      │
  │  2.2 从编译器二进制加载对应语言的 i18n JSON                              │
  │  2.3 error! 宏渲染模板（模板 + 参数 → 最终字符串）                       │
  │  2.4 Diagnostic.message = 已渲染的字符串                                 │
  │  2.5 AOT 二进制直接包含最终字符串                                        │
  │                                                                           │
  │  ┌─────────────────────────────────────────────────────────────────────┐ │
  │  │  渲染时机：编译用户项目时                                             │ │
  │  │  模板 "Unknown variable: '{name}'" + name="x"                      │ │
  │  │  → "Unknown variable: 'x'"  ← 已渲染完成                           │ │
  │  └─────────────────────────────────────────────────────────────────────┘ │
  └─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
  ┌─────────────────────────────────────────────────────────────────────────┐
  │  阶段 3: 用户程序运行时                                                  │
  │                                                                           │
  │  println!("{}", diagnostic.message)                                      │
  │  // 直接输出 "Unknown variable: 'x'"，无任何查表                         │
  │                                                                           │
  │  AOT 二进制已包含最终字符串，无需任何 i18n 解析                          │
  └─────────────────────────────────────────────────────────────────────────┘
```

### 编译期错误 vs 运行时错误

| 场景 | i18n 处理方式 | 原因 |
|------|--------------|------|
| **编译期错误** | 编译用户项目时渲染 | 渲染开销由编译器承担 |
| **运行时错误** | AOT 编译期渲染 | 用户程序无编译器依赖 |

## 7. 设计优势

| 特性 | 说明 |
|------|------|
| **零运行时开销** | 模板在编译期渲染，AOT 二进制直接存已渲染消息 |
| **完整 i18n** | 所有语言打包，用户自由选择 yaoxiang.toml |
| **好品味** | 编译期做复杂的事，运行时做简单的事 |
| **通用 Builder** | 一个 DiagnosticBuilder 替代 20 个 Trait |
| **用户透明** | 只需配置 yaoxiang.toml，错误消息自动切换 |
| **文档自动生成** | i18n 文件作为单一数据源，自动生成参考文档 |

## 8. 文档自动生成

### 数据流

```
i18n/en.json      i18n/zh.json
      │                 │
      └────────┬────────┘
               ▼
      ┌───────────────────────┐
      │  generate.rs          │  ← 脚本读取所有语言 i18n
      └───────────────────────┘
               │
      ┌────────┴────────┐
      ▼                 ▼
┌─────────────────┐  ┌─────────────────┐
│ docs/en/...    │  │ docs/zh/...    │
│ (英文文档)      │  │ (中文文档)      │
└─────────────────┘  └─────────────────┘
```

### 生成器脚本

```rust
// scripts/generate_error_docs.rs

use std::fs;
use std::path::PathBuf;

/// 支持的语言
const LANGUAGES: &[&str] = &["en", "zh", "ja"];

fn main() {
    for &lang in LANGUAGES {
        let i18n = I18nRegistry::new(lang);
        let output_dir = format!("docs/{}", lang);

        for code_def in ErrorCodeDefinition::all() {
            let info = i18n.get_info(code_def.code)
                .expect(&format!("[{}] Missing i18n for {}", lang, code_def.code));

            let content = format!(
                r#"# {code}: {title}

> {message}

## Help

{help}

## Example

```{yaoxiang}
{example}
```

```{error}
{error_output}
```
"#,
                code = code_def.code,
                title = info.title,
                message = info.message,
                help = info.help,
                example = info.example.unwrap_or(""),
                error_output = info.error_output.unwrap_or("")
            );

            let filename = format!("{}/reference/error-code/{}.md", output_dir, code_def.code);
            fs::write(&filename, content).unwrap();
        }
    }
}
```

### 文档结构

```
docs/
├── en/
│   └── reference/
│       └── error-code/
│           ├── E0xxx.md    # Lexer & Parser
│           ├── E1xxx.md    # Type Check
│           └── index.md    # 索引
├── zh/
│   └── reference/
│       └── error-code/
│           ├── E0xxx.md
│           └── E1xxx.md
└── ja/
    └── reference/
        └── error-code/
            ├── E0xxx.md
            └── E1xxx.md
```

### 手动补充

以下内容仍需手动补充：

| 字段 | 来源 | 说明 |
|------|------|------|
| 章节标题 | 自动 | 如 `# E0xxx: Lexer & Parser` |
| 错误详情 | 自动 | title + message + help |
| 示例代码 | 自动 | 从 i18n.example 读取 |
| 错误输出 | 自动 | 从 i18n.error_output 读取 |
| 详细说明 | 手动 | 添加使用场景、注意事项 |

### index.md 自动生成

```rust
// 生成索引文件
fn generate_index(lang: &str, codes: &[&str]) {
    let content = format!(
        r#"# Error Code Reference

## Categories

- [E0xxx: Lexer & Parser](E0xxx.md)
- [E1xxx: Type Check](E1xxx.md)
- [E2xxx: Semantic](E2xxx.md)
...

## Full List

| Code | Title |
|------|-------|
{}

"#,
        codes.iter()
            .map(|code| format!("| [{}]({}.md) | ... |", code, code))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
```

## 9. explain 指令优化

`explain` 指令直接使用 i18n 数据，无需解析文档：

```bash
# 直接从编译器二进制读取
yaoxiang explain E1001

# 输出（英文）：
# E1001: Unknown variable
#
# Referenced variable is not defined
#
# Help: Check if the variable name is spelled correctly, or define it first

# 中文输出（根据 yaoxiang.toml 配置）
yaoxiang explain E1001
# E1001: 未知变量
#
# 引用的变量未定义
#
# 帮助: 检查变量名是否拼写正确，或先定义它
```

### 实现

```rust
// cli/explain.rs

pub fn explain(code: &str) {
    let i18n = I18nRegistry::current();  // 根据配置选择语言

    if let Some(info) = i18n.get_info(code) {
        println!("{}: {}", code, info.title);
        println!();
        println!("> {}", info.message);
        println!();
        println!("Help:");
        println!("{}", info.help);
    } else {
        println!("Unknown error code: {}", code);
    }
}
```
