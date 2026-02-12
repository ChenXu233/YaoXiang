# RFC 013: 错误代码规范

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2026-02-02
> **最后更新**: 2026-02-12

## 摘要

本 RFC 提出 YaoXiang 编译器的错误代码分类规范，采用类似 Rust 的单层编号系统，配合 JSON 资源文件实现多语种支持，通过 `yaoxiang explain` 命令提供错误解释功能。

## 动机

### 为什么需要标准化的错误代码？

1. **用户体验**：用户看到错误代码能快速判断错误类型和严重程度
2. **文档组织**：按类别分组便于编写和维护错误参考文档
3. **工具集成**：IDE/LSP 可以根据错误代码提供快速修复建议和文档链接
4. **国际化支持**：错误消息与代码分离，便于多语言翻译

### 设计目标

- **简洁**：单层编号，用户无需记忆复杂分类规则
- **友好**：类似 Rust 的错误消息格式，带帮助信息和示例
- **可扩展**：资源文件驱动，易于添加新错误和新语言
- **工具友好**：explain 命令 + JSON 输出，支持 IDE/LSP 集成

---

## 提案

### 核心设计：单层编号系统

采用四位数字编号，按编译阶段分组：

```
Exxxx
││││
│││└── 序号 (000-999)
││└─── 编译阶段 (0-9)
└───── 固定前缀 'E'
```

### 阶段划分

| 阶段 | 范围 | 描述 |
|------|------|------|
| **0** | E0xxx | 词法与语法分析 |
| **1** | E1xxx | 类型检查 |
| **2** | E2xxx | 语义分析 |
| **3** | E3xxx | 代码生成 |
| **4** | E4xxx | 泛型与特质 |
| **5** | E5xxx | 模块与导入 |
| **6** | E6xxx | 运行时错误 |
| **7** | E7xxx | I/O 与系统错误 |
| **8** | E8xxx | 内部编译器错误 |
| **9** | E9xxx | 保留/实验性 |

### 错误类别枚举

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

### 错误码定义与通用 Builder

**核心原则**：错误码定义与展示文案分离

- `ErrorCodeDefinition`：错误码元数据（code、category、template），不含展示文案
- `i18n/*.json`：各语言展示文案（title、message、help）
- `DiagnosticBuilder`：通用构建器，替代 trait-per-error 设计

#### 错误码定义

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

#### 每个错误码的快捷方法

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

#### 使用示例

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

#### 错误码定义示例

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

#### 设计优势

| 特性 | 说明 |
|------|------|
| **单一 Builder** | 一个 `DiagnosticBuilder` 通用所有错误码 |
| **类型安全** | 快捷方法确保参数正确性 |
| **自文档** | `E1001::unknown_variable(name)` 一目了然 |
| **模板分离** | 消息模板与代码分离，易于 i18n |
| **零运行时开销** | 编译期渲染，AOT 二进制无查表 |

---

### 错误宏简化

#### error! 宏（自动注入上下文）

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

#### 手动使用 Builder

```rust
// 需要手动控制时
E1001::unknown_variable(&var_name)
    .at(my_span)           // 自定义 span
    .build(&custom_i18n)   // 自定义 i18n
```

---

## 详细设计

### 错误代码列表

#### E0xxx：词法与语法分析

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E0001 | Invalid character | 源代码包含非法字符 |
| E0002 | Invalid number literal | 数字字面量格式不正确 |
| E0003 | Unterminated string | 多行字符串缺少结束引号 |
| E0004 | Invalid character literal | 字符字面量不正确 |
| E0010 | Expected token | 语法分析时期望特定 token |
| E0011 | Unexpected token | 遇到意外的 token |
| E0012 | Invalid syntax | 表达式/语句语法错误 |
| E0013 | Mismatched brackets | 圆括号、方括号、花括号不匹配 |
| E0014 | Missing semicolon | 语句末尾缺少分号 |

#### E1xxx：类型检查

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E1001 | Unknown variable | 引用的变量未定义 |
| E1002 | Type mismatch | 期望类型与实际类型不符 |
| E1003 | Unknown type | 引用的类型不存在 |
| E1010 | Parameter count mismatch | 函数调用参数数量与定义不符 |
| E1011 | Parameter type mismatch | 参数类型检查失败 |
| E1012 | Return type mismatch | 函数返回值类型错误 |
| E1013 | Function not found | 调用未定义的函数 |
| E1020 | Cannot infer type | 上下文无法推断类型 |
| E1021 | Type inference conflict | 多处约束导致类型矛盾 |
| E1030 | Pattern non-exhaustive | match 表达式未覆盖所有情况 |
| E1031 | Unreachable pattern | 永远无法匹配的模式 |
| E1040 | Operation not supported | 类型不支持该操作 |
| E1041 | Index out of bounds | 数组/列表索引超出范围 |
| E1042 | Field not found | 访问不存在的结构体字段 |

#### E2xxx：语义分析

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E2001 | Scope error | 变量不在当前作用域 |
| E2002 | Duplicate definition | 同一作用域内重复定义 |
| E2003 | Lifetime error | 生命周期约束不满足 |
| E2010 | Immutable assignment | 尝试修改不可变变量 |
| E2011 | Uninitialized use | 使用未初始化的变量 |
| E2012 | Mutability conflict | 不可变上下文中使用可变引用 |

#### E4xxx：泛型与特质

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E4001 | Generic parameter mismatch | 泛型参数数量/类型不匹配 |
| E4002 | Trait bound violated | 不满足 trait 约束 |
| E4003 | Associated type error | 关联类型定义/使用错误 |
| E4004 | Duplicate trait implementation | 重复实现同一 trait |
| E4005 | Trait not found | 找不到要求的 trait |
| E4006 | Sized bound violated | Sized 约束不满足 |

#### E5xxx：模块与导入

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E5001 | Module not found | 导入的模块不存在 |
| E5002 | Cyclic import | 模块间循环依赖 |
| E5003 | Symbol not exported | 尝试访问未导出的符号 |
| E5004 | Invalid module path | 模块路径格式错误 |
| E5005 | Private access | 访问私有符号 |

#### E6xxx：运行时错误

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E6001 | Division by zero | 整数除以零 |
| E6002 | Assertion failed | assert! 宏失败 |
| E6003 | Arithmetic overflow | 算术运算溢出 |
| E6004 | Stack overflow | 栈空间耗尽 |
| E6005 | Heap allocation failed | 内存分配失败 |
| E6006 | Runtime index out of bounds | 运行时索引越界 |
| E6007 | Type cast failed | 尝试将类型断言为不兼容类型 |

#### E7xxx：I/O 与系统错误

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E7001 | File not found | 尝试读取不存在的文件 |
| E7002 | Permission denied | 文件权限不足 |
| E7003 | I/O error | 通用 I/O 错误 |
| E7004 | Network error | 网络操作失败 |

#### E8xxx：内部编译器错误

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E8001 | Internal compiler error | 编译器内部错误 |
| E8002 | Codegen error | IR/字节码生成失败 |
| E8003 | Unimplemented feature | 使用未实现的功能 |
| E8004 | Optimization error | 编译器优化错误 |

---

### 多语种资源文件

#### 资源文件格式

```json
// diagnostic/codes/i18n/en.json
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
// diagnostic/codes/i18n/zh.json
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

#### I18nRegistry 实现

```rust
// diagnostic/codes/i18n/mod.rs

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

#### 模板占位符

##### 预定义占位符（常用）

| 占位符 | 用途 | 示例 |
|--------|------|------|
| `{name}` | 变量名/类型名/特质名等标识符 | `Unknown variable: '{name}'` |
| `{expected}` | 期望类型 | `Expected type '{expected}'` |
| `{found}` | 实际/找到的类型 | `, found type '{found}'` |
| `{method}` | 方法名 | `Method {method} is not a function` |
| `{trait}` | 特质名 | `Cannot find trait: {trait}` |
| `{path}` | 模块路径 | `Invalid path: {path}` |
| `{ty}` | 类型表达式 | `Invalid type: {ty}` |
| `{message}` | 内部错误消息 | `Internal error: {message}` |

##### 任意 key 支持

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

> **注意**：并非所有错误码都使用占位符。部分错误码（如 E0001）是静态消息，无需参数。

#### 语言优先级

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. 默认值: en
```

### yaoxiang.toml 配置

#### 项目级配置

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# 错误消息语言，可选：en, zh, ja, ...
default = "zh"
```

#### 用户级配置

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### 编译期语言选择

```
1. 读取项目级 yaoxiang.toml 的 language.default
2. 若未配置，读取用户级 ~/.yaoxiang/yaoxiang.toml
3. 若都未配置，默认使用 "en"
4. 编译器根据选择的语言创建 I18nRegistry（一次）
5. 所有错误使用该 I18nRegistry 渲染消息
```

#### 零查表开销的关键

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

| 组件 | 职责 | 渲染时机 |
|------|------|----------|
| `I18nRegistry` | 提供模板和展示文案 | 编译用户项目时 |
| `DiagnosticBuilder.render()` | 模板 + 参数 → 最终字符串 | 编译用户项目时 |
| `Diagnostic.message` | 已渲染的字符串 | 存储最终结果 |
| AOT 二进制 | 包含最终字符串 | 运行时直接用 |

---

### 错误消息格式

错误消息采用以下格式：

```
error[E####]: <简短描述>
  --> <文件>:<行>:<列>
   <行> | <代码片段>
          ^^^<高亮>
```

#### 完整示例

```
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?
```

---

### 严重程度级别

错误严重程度通过 `DiagnosticLevel` 枚举管理，与错误码编号解耦：

```rust
pub enum DiagnosticLevel {
    Error,    // 导致编译失败
    Warning,  // 不影响编译，但建议修复
    Note,     // 补充信息
    Help,     // 修复建议
}
```

| 级别 | 前缀 | 说明 |
|------|------|------|
| Error | `error[E####]:` | 导致编译失败 |
| Warning | `warning[E####]:` | 不影响编译 |
| Note | `note[E####]:` | 补充信息 |
| Help | `help[E####]:` | 修复建议 |

---

### `yaoxiang explain` 命令

#### 命令语法

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### 选项

| 选项 | 描述 |
|------|------|
| `--lang <code>` | 指定语言 (en-US, zh-CN，默认 en-US) |
| `--json` | JSON 格式输出（供 IDE/LSP 使用） |
| `--json-pretty` | 格式化的 JSON 输出 |
| `--examples` | 只显示示例代码 |
| `--help` | 显示帮助信息 |

#### 使用示例

```bash
# 默认英文
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# 中文输出
$ yaoxiang explain E1001 --lang zh
error[E1001]: 未知变量: {name}
  --> <file>:<line>:<col>

帮助: 你是否想要定义它？

示例:
  let {name} = value;

# JSON 输出（LSP 集成）
$ yaoxiang explain E1001 --json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

#### JSON 输出格式

```json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": [
    "let {name} = value;"
  ],
  "language": "en-US"
}
```

---

### 向后兼容性

由于本 RFC 从零设计错误码系统，不存在向后兼容性问题。

**未来迁移策略**（供后续版本参考）：

1. 保持旧错误码到新错误码的映射
2. 在迁移期间同时显示新旧代码
3. 提供废弃时间表

---

## 实施策略

### 阶段一：错误码基础架构

1. 创建 `src/diagnostics/` 目录结构
2. 实现 `ErrorCode` 枚举
3. 实现 `Diagnostic` 和 `DiagnosticLevel`
4. 创建资源文件目录和示例 JSON

### 阶段二：explain 命令

1. 实现 `yaoxiang explain` CLI 命令
2. 支持 `--lang` 和 `--json` 选项
3. 集成资源文件加载
4. 实现参数模板渲染

### 阶段三：编译期集成

1. 更新所有错误报告点使用新系统
2. 实现消息模板参数注入
3. 添加语言优先级逻辑
4. 单元测试覆盖

### 阶段四：IDE/LSP 集成

1. LSP 服务器集成 explain JSON 输出
2. 在 IDE 中显示错误代码链接
3. 悬停显示错误解释
4. 快速修复建议

---

## 附录

### 完整错误代码速查表

| 范围 | 类别 |
|------|------|
| E0xxx | 词法与语法分析 |
| E1xxx | 类型检查 |
| E2xxx | 语义分析 |
| E3xxx | 代码生成 |
| E4xxx | 泛型与特质 |
| E5xxx | 模块与导入 |
| E6xxx | 运行时错误 |
| E7xxx | I/O 与系统错误 |
| E8xxx | 内部编译器错误 |
| E9xxx | 保留 |

### 支持的语言

| 代码 | 语言 | 状态 |
|------|------|------|
| en-US | English (US) | 默认 |
| zh-CN | 简体中文 | 计划中 |

### 错误消息示例对比

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

## 参考文献

- [Rust 编译器错误索引](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC 错误消息格式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 诊断格式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
