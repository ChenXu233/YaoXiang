---
title: 'RFC 013: 错误代码规范'
status: '已接受'
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

# RFC 013: 错误代码规范

## 摘要

本 RFC 提出 YaoXiang 编译器的错误代码分类规范，采用类似 Rust 的单层编号系统，配合 JSON 资源文件实现多语种支持，通过
`yaoxiang explain` 命令提供错误解释功能。

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

| 阶段  | 范围  | 描述           |
| ----- | ----- | -------------- |
| **0** | E0xxx | 词法与语法分析 |
| **1** | E1xxx | 类型检查       |
| **2** | E2xxx | 语义分析       |
| **3** | E3xxx | 代码生成       |
| **4** | E4xxx | 泛型与特质     |
| **5** | E5xxx | 模块与导入     |
| **6** | E6xxx | 运行时错误     |
| **7** | E7xxx | I/O 与系统错误 |
| **8** | E8xxx | 内部编译器错误 |
| **9** | E9xxx | 保留/实验性    |

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
- `locales/*.json`：各语言展示文案（title、message、help，错误码为嵌套对象）
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

| 特性             | 说明                                     |
| ---------------- | ---------------------------------------- |
| **单一 Builder** | 一个 `DiagnosticBuilder` 通用所有错误码  |
| **类型安全**     | 快捷方法确保参数正确性                   |
| **自文档**       | `E1001::unknown_variable(name)` 一目了然 |
| **模板分离**     | 消息模板与代码分离，易于 i18n            |
| **零运行时开销** | 编译期渲染，AOT 二进制无查表             |

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
<!-- code-table:E0xxx start -->
| 代码  | 说明    |
| ----- | ------- |
| E0001 | 无效字符    |
| E0002 | 无效数字字面量 |
| E0003 | 未终止的字符串 |
| E0004 | 无效字符字面量 |
| E0010 | 期望的令牌   |
| E0011 | 意外的令牌   |
| E0012 | 无效语法    |
| E0013 | 不匹配的括号  |
| E0014 | 缺少分号    |
| E0016 | 期望表达式   |
| E0018 | 关键字作名称  |
<!-- code-table:E0xxx end -->

#### E1xxx：类型检查
<!-- code-table:E1xxx start -->
| 代码  | 说明                     |
| ----- | ------------------------ |
| E1001 | 未知变量                     |
| E1002 | 类型不匹配                    |
| E1003 | 未知类型                     |
| E1010 | 参数数量不匹配                  |
| E1011 | 参数类型不匹配                  |
| E1012 | 返回类型不匹配                  |
| E1013 | 函数未找到                    |
| E1020 | 无法推断类型                   |
| E1021 | 类型推断冲突                   |
| E1030 | 模式不完整                    |
| E1031 | 不可达模式                    |
| E1040 | 操作不支持                    |
| E1041 | 索引越界                     |
| E1042 | 字段未找到                    |
| E1050 | 需要布尔操作数                  |
| E1051 | 逻辑 NOT 需要布尔操作数           |
| E1052 | 无效解引用                    |
| E1053 | 非结构体字段访问                 |
| E1054 | 条件类型不匹配                  |
| E1055 | 约束在非泛型上下文中               |
| E1060 | 类型参数数量不匹配                |
| E1061 | 无法实例化泛型                  |
| E1062 | const 泛型约束失败             |
| E1064 | 绑定位置索引无效                 |
| E1071 | 类型定义只能在模块级               |
| E1081 | `?` 仅允许在返回 Result 的函数内使用 |
| E1082 | `?` 只能用于 Result 表达式      |
| E1083 | `?` 的错误类型不匹配             |
| E1090 | ✨ 不可言说 ✨                 |
| E1091 | 无效的泛型元类型                 |
| E1092 | 精化类型实参形态非法               |
| E1093 | 精化实参个数不匹配                |
| E1094 | 未使用的编译期值参数               |
| E1095 | 未知接口                     |
| E1096 | 接口参数数量不匹配                |
| E1097 | 接口成员命名冲突                 |
| E1098 | 接口方法未实现                  |
| E1099 | 接口方法签名不匹配                |
| E1100 | 接口方法重复实现                 |
| E1101 | 类型未实现接口                  |
| E1102 | 循环控制语句出现在循环外             |
<!-- code-table:E1xxx end -->

#### E2xxx：语义分析
<!-- code-table:E2xxx start -->
| 代码  | 说明        |
| ----- | ----------- |
| E2001 | 作用域错误       |
| E2002 | 重复定义        |
| E2003 | 所有权错误       |
| E2010 | 不可变赋值       |
| E2011 | 使用未初始化变量    |
| E2012 | 可变性冲突       |
| E2013 | 变量遮蔽        |
| E2014 | 使用已移动的值     |
| E2016 | 不可变赋值       |
| E2018 | 可变/不可变借用冲突  |
| E2019 | 双重释放        |
| E2020 | 释放后使用       |
| E2027 | unsafe 解引用  |
| E2029 | spawn 内引用循环 |
| E2090 | 无效签名        |
| E2091 | 签名未知类型      |
| E2092 | 签名缺少箭头      |
| E2093 | 重复参数名       |
| E2094 | 泛型参数遮蔽      |
| E2095 | 参数名遮蔽泛型     |
<!-- code-table:E2xxx end -->

#### E3xxx：代码生成
<!-- code-table:E3xxx start -->
| 代码  | 说明         |
| ----- | ------------ |
| E3004 | 不支持的迭代器      |
| E3005 | IR 生成错误      |
| E3006 | 未解析变量        |
| E3007 | 顶层绑定初始化必须为常量 |
| E3014 | 寄存器溢出        |
| E3017 | 无效操作数（代码生成）  |
<!-- code-table:E3xxx end -->

#### E4xxx：泛型与特质
<!-- code-table:E4xxx start -->
| 代码  | 说明    |
| ----- | ------- |
| E4001 | 泛型约束违反  |
| E4002 | 特质未找到   |
| E4003 | 特质实现缺失  |
| E4004 | 特质实现冲突  |
| E4005 | 关联类型未找到 |
| E4010 | 常量除零    |
| E4011 | 常量溢出    |
| E4012 | 常量递归过深  |
| E4014 | 常量求值失败  |
| E4018 | 精化谓词违反  |
| E4019 | 类型等式不成立 |
| E4020 | 需要证明函数  |
<!-- code-table:E4xxx end -->

> E4006/E8004 当前无发射点（预留码）：Sized 约束与优化错误路径待实现，实现时按真实触发面接线。

#### E5xxx：模块与导入
<!-- code-table:E5xxx start -->
| 代码  | 说明    |
| ----- | ------- |
| E5001 | 模块未找到   |
| E5002 | 导入错误    |
| E5003 | 导出未找到   |
| E5004 | 循环依赖    |
| E5005 | 无效的模块路径 |
| E5006 | 重复导入    |
| E5007 | 模块导出    |
<!-- code-table:E5xxx end -->

#### E6xxx：运行时错误
<!-- code-table:E6xxx start -->
| 代码  | 说明       |
| ----- | ---------- |
| E6001 | 除零错误       |
| E6003 | 数组索引越界     |
| E6004 | 栈溢出        |
| E6005 | 断言失败       |
| E6006 | 函数未找到（运行时） |
| E6007 | 运行时错误      |
| E6008 | 键不存在       |
| E6009 | Range 步长非法 |
| E6010 | 整数解析失败     |
| E6011 | 浮点解析失败     |
<!-- code-table:E6xxx end -->

> **码表修订（2026-08-09）**：码表原按 Rust 语义草案（Assertion failed/Arithmetic overflow/Heap
> allocation failed/Type cast failed）定义，与实现实际需求不符。YaoXiang 无空指针/堆分配失败/类型
> 转换概念（值语义 + Rust 内存安全），运行时溢出路径未实现检测。校准后：
>
> - E6002 删除（原 Assertion failed 移至 E6005；原空指针语义无语言概念）
> - E6003 从 Arithmetic overflow 改为 Runtime index out of bounds（真实触发面）
> - E6005 从 Heap allocation failed 改为 Assertion failed（std.assert 真实路径）
> - E6006 从 Runtime index out of bounds 改为 Function not found（实现早已如此）
> - E6007 从 Type cast failed 改为通用 Runtime error（ExecutorError 未映射变体统一落点）

#### E7xxx：I/O 与系统错误
<!-- code-table:E7xxx start -->
| 代码  | 说明   |
| ----- | ------ |
| E7001 | 文件未找到  |
| E7002 | 权限被拒绝  |
| E7003 | I/O 错误 |
| E7004 | 网络错误   |
<!-- code-table:E7xxx end -->

#### E8xxx：内部编译器错误
<!-- code-table:E8xxx start -->
| 代码  | 说明     |
| ----- | -------- |
| E8001 | 内部编译器错误  |
| E8002 | 意外 Panic |
| E8003 | 编译器阶段错误  |
<!-- code-table:E8xxx end -->

#### W1xxx：警告码
<!-- code-table:W1xxx start -->
| 代码  | 说明           |
| ----- | -------------- |
| W1001 | 未使用的导出函数       |
| W1002 | 未使用的导出类型       |
| W1003 | 未使用的导入         |
| W1004 | 未使用的导出变量       |
| W1005 | 未使用的导出方法       |
| W1063 | const 泛型约束无法求值 |
| W1080 | 编译期证明降级        |
<!-- code-table:W1xxx end -->

> W 码位规则：与 E 码同构按阶段分组（W+阶段千位段），W1xxx = 类型检查阶段警告。
>
> **发射通道**：W 码诊断由 builder 按 W 前缀缺省标注 `Severity::Warning`（显式指定优先），
> 收集与呈现与错误同轨（`warning[W####]` 前缀渲染），但不阻断编译、不影响成功退出码。
> `yaoxiang check --deny-warnings` 将警告升级为失败（存在警告时以非零码退出），用于 CI 严格模式。
> per-code 压制（allow 属性等）为后续扩展项。

### 消息质量规范

> 本节由消息单轨与质量修订（2026-09-03）引入。由 `scripts/audit_diagnostics.py` 在 CI 强制执行。

1. **消息单轨**：所有用户可见诊断消息必须经权威注册表快捷方法 + locales 模板渲染，代码只传结构化参数。禁止绕过注册表直接构造 `Diagnostic::error(...)` 等原生值——该路径绕过码校验与 i18n。
2. **码合法性**：禁止使用未注册码与伪码（如 `E_INTERNAL`）；使用点码字面量必须已在注册表定义。内部错误一律落 E8001（`internal_error`）。
3. **类型显示**：类型 Display 必须区分实例化前后形态（`Expected 'Container', found 'Container'` 裸名不可区分）。
4. **求解器内部态隔离**：求解器中间态 TypeVar（Display 形态 `t<N>`）不得进入用户可见消息。测试锚定：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 边界**：E8xxx 仅用于编译器内部一致性问题（ICE）。用户可修复的错误禁止使用 E8001 兜底；ICE 消息必须附最小复现指引。

---

### 运行时错误值与码贯通

> 本节由运行时 Error 值带码修订（2026-09-03）引入。E6xxx/E7xxx 语义空间同时承载两个通道，码空间同一、呈现通道不同。

#### 两个通道

| 通道                         | 载体                                        | 呈现方式                     |
| ---------------------------- | ------------------------------------------- | ---------------------------- |
| 编译器/CLI 诊断通道          | `ExecutorError` 等宿主层硬错误              | stderr `error[E####]:`（已接线 E6003/E6005/E6007） |
| 程序内错误值通道             | std 库 `Result(T, Error)` 的 Err 载体 `Error` | 语言值，由程序 match/比较消费 |

#### Error 结构（v0.8 起，破坏性变更）

```
Error { code: String, message: String }
```

- `code` 复用本规范 E6xxx/E7xxx 编号，字符串形态（如 `"E6008"`）。
- **稳定契约**：已分配的码跨版本语义不变；同一语义不复用已删码（E6002 先例）。
- **消费面**：程序内 `e.code == "E6xxx"` 比较是唯一可编程判定契约；`yaoxiang explain E6xxx` 文档贯通；工具链（LSP / DAP，见 RFC-034）以码为 exceptionId。
- **访问器**：`std.result.code(e)` / `std.result.message(e)`。
- **用户自定义错误**：`Result(T, E)` 的 E 为泛型参数，认真建模走用户自定义类型；std `Error` 仅为便捷兜底载体，其码体系不约束用户 E 类型。

#### 码分配规则

1. 运行时错误值码与编译器诊断码共用 E6xxx/E7xxx 空间，新码按**真实触发面**分配，不为想象中的场景预留。
2. 先注册后使用：新码进入权威注册表并经三方一致性校验（codes/*.rs ↔ locales ↔ 本文档码表）后方可发射。运行时错误值码的注册源为 `src/std/result.rs` 的 `RUNTIME_ERROR_CODES` 表（与诊断码同受 `build.rs 构建期门槛 + `tools/code-tables`` 校验）。
3. E7xxx 为 std.io / std.net 错误值预留段位（当前空挂，io/net Result 化时启用）。
4. 发射点：std 各模块经 `error_new(code, message)` 构造 Error 值；消费侧 `std.result.unwrap_err` 取出 Err 载体，`std.result.code/message` 读取字段。

#### 演进路径（线 C，未实施）

模式匹配完备化（RFC-039）落地后，`Error` 可升级为 `{ kind: ErrorKind, message: String }`，`code` 转为由 kind 派生的属性（变体定义处即码注册表）。演进期本节 code 稳定契约保持不变；该升级为独立决策，不构成本节承诺。

---

### 多语种资源文件

#### 资源文件格式

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

#### I18nRegistry 实现

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

#### 模板占位符

##### 预定义占位符（常用）

| 占位符       | 用途                         | 示例                                |
| ------------ | ---------------------------- | ----------------------------------- |
| `{name}`     | 变量名/类型名/特质名等标识符 | `Unknown variable: '{name}'`        |
| `{expected}` | 期望类型                     | `Expected type '{expected}'`        |
| `{found}`    | 实际/找到的类型              | `, found type '{found}'`            |
| `{method}`   | 方法名                       | `Method {method} is not a function` |
| `{trait}`    | 特质名                       | `Cannot find trait: {trait}`        |
| `{path}`     | 模块路径                     | `Invalid path: {path}`              |
| `{ty}`       | 类型表达式                   | `Invalid type: {ty}`                |
| `{message}`  | 内部错误消息                 | `Internal error: {message}`         |

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

| 组件                         | 职责                     | 渲染时机       |
| ---------------------------- | ------------------------ | -------------- |
| `I18nRegistry`               | 提供模板和展示文案       | 编译用户项目时 |
| `DiagnosticBuilder.render()` | 模板 + 参数 → 最终字符串 | 编译用户项目时 |
| `Diagnostic.message`         | 已渲染的字符串           | 存储最终结果   |
| AOT 二进制                   | 包含最终字符串           | 运行时直接用   |

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

| 级别    | 前缀              | 说明         |
| ------- | ----------------- | ------------ |
| Error   | `error[E####]:`   | 导致编译失败 |
| Warning | `warning[E####]:` | 不影响编译   |
| Note    | `note[E####]:`    | 补充信息     |
| Help    | `help[E####]:`    | 修复建议     |

---

### `yaoxiang explain` 命令

#### 命令语法

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### 选项

| 选项            | 描述                                |
| --------------- | ----------------------------------- |
| `--lang <code>` | 指定语言 (en-US, zh-CN，默认 en-US) |
| `--json`        | JSON 格式输出（供 IDE/LSP 使用）    |
| `--json-pretty` | 格式化的 JSON 输出                  |
| `--examples`    | 只显示示例代码                      |
| `--help`        | 显示帮助信息                        |

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
  "examples": ["let {name} = value;"],
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

| 范围  | 类别           |
| ----- | -------------- |
| E0xxx | 词法与语法分析 |
| E1xxx | 类型检查       |
| E2xxx | 语义分析       |
| E3xxx | 代码生成       |
| E4xxx | 泛型与特质     |
| E5xxx | 模块与导入     |
| E6xxx | 运行时错误     |
| E7xxx | I/O 与系统错误 |
| E8xxx | 内部编译器错误 |
| E9xxx | 保留           |

### 支持的语言

| 代码  | 语言         | 状态   |
| ----- | ------------ | ------ |
| en-US | English (US) | 默认   |
| zh-CN | 简体中文     | 计划中 |

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
