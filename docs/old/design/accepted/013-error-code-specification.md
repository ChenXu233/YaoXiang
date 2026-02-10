# RFC 013: 错误代码规范

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2026-02-02
> **最后更新**: 2026-02-07

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

#### 文件结构

```
yaoxiang/
├── errors/
│   ├── en-US.json    # 英文（默认）
│   └── zh-CN.json    # 简体中文
```

#### 资源文件格式

```json
// errors/en-US.json
{
  "E1001": {
    "message": "Unknown variable: {name}",
    "help": "Did you mean to define it?",
    "examples": [
      "let {name} = value;"
    ]
  },
  "E1002": {
    "message": "Type mismatch: expected {expected}, found {actual}",
    "help": "",
    "examples": []
  }
}
```

```json
// errors/zh-CN.json
{
  "E1001": {
    "message": "未知变量: {name}",
    "help": "你是否想要定义它？",
    "examples": [
      "let {name} = value;"
    ]
  },
  "E1002": {
    "message": "类型不匹配: 期望 {expected}, 得到 {actual}",
    "help": "",
    "examples": []
  }
}
```

#### 参数语法

- 使用 `{name}` 语法表示占位符
- 占位符在编译时由错误处理代码替换为实际值
- 支持的参数类型：
  - `{name}` - 标识符名称
  - `{expected}` - 期望的类型
  - `{actual}` - 实际的类型
  - `{file}` - 文件路径
  - `{line}` - 行号
  - `{column}` - 列号

#### 语言优先级

```
1. CLI 参数: --lang <code>
2. 环境变量: LANG / LC_ALL
3. 默认值: en-US
```

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

### 编译期集成

#### 错误码定义

```rust
// src/diagnostics/error_code.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // Lexer & Parser (E0xxx)
    E0001,
    E0002,
    E0003,
    E0004,
    E0010,
    E0011,
    E0012,
    E0013,
    E0014,

    // Type Checking (E1xxx)
    E1001,
    E1002,
    E1003,
    E1010,
    E1011,
    E1012,
    E1013,
    E1020,
    E1021,
    E1030,
    E1031,
    E1040,
    E1041,
    E1042,

    // Semantic Analysis (E2xxx)
    E2001,
    E2002,
    E2003,
    E2010,
    E2011,
    E2012,

    // Generics & Traits (E4xxx)
    E4001,
    E4002,
    E4003,
    E4004,
    E4005,
    E4006,

    // Modules & Imports (E5xxx)
    E5001,
    E5002,
    E5003,
    E5004,
    E5005,

    // Runtime Errors (E6xxx)
    E6001,
    E6002,
    E6003,
    E6004,
    E6005,
    E6006,
    E6007,

    // I/O & System Errors (E7xxx)
    E7001,
    E7002,
    E7003,
    E7004,

    // Internal Compiler Errors (E8xxx)
    E8001,
    E8002,
    E8003,
    E8004,
}

impl ErrorCode {
    /// 获取错误码字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::E0001 => "E0001",
            ErrorCode::E0002 => "E0002",
            // ...
        }
    }

    /// 渲染错误消息（带参数替换）
    pub fn render_message(
        &self,
        lang: &str,
        args: &HashMap<&str, String>,
    ) -> String {
        let template = self.load_template(lang);
        Self::interpolate(&template, args)
    }

    /// 加载语言模板
    fn load_template(&self, lang: &str) -> &'static str {
        // 从资源文件加载
        // fallback 到 en-US
    }

    /// 参数替换
    fn interpolate(template: &str, args: &HashMap<&str, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in args {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }
}
```

#### 诊断消息构建

```rust
// src/diagnostics/diagnostic.rs

pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: ErrorCode,
    pub message: String,
    pub location: SourceLocation,
    pub labels: Vec<Label>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    /// 创建错误诊断
    pub fn error(
        code: ErrorCode,
        message: String,
        location: SourceLocation,
    ) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            code,
            message,
            location,
            labels: vec![],
            suggestions: vec![],
        }
    }

    /// 添加高亮标签
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// 添加修复建议
    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }
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

---

## 参考文献

- [Rust 编译器错误索引](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC 错误消息格式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 诊断格式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
