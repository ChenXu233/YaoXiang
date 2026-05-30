---
title: 诊断系统
description: YaoXiang 诊断系统的架构设计
---

# 诊断系统

## 错误码体系

错误码按类别分组：

| 范围 | 类别 | 说明 |
|------|------|------|
| E0xxx | 词法/语法 | 词法分析和语法分析错误 |
| E1xxx | 类型检查 | 类型不匹配、未定义变量等 |
| E2xxx | 语义分析 | 语义错误 |
| E4xxx | 泛型/特质 | 泛型和特质系统错误 |
| E5xxx | 模块/导入 | 模块系统错误 |
| E6xxx | 运行时 | 运行时错误 |
| E7xxx | I/O | I/O 和系统错误 |
| E8xxx | 内部 | 内部编译器错误 |
| W1xxx | 警告 | 死代码、未使用变量等 |

## Diagnostic 数据结构

```rust
pub struct Diagnostic {
    pub code: String,           // 错误码，如 "E1001"
    pub severity: Severity,     // Error / Warning / Info / Hint
    pub message: String,        // 渲染后的消息
    pub span: Option<Span>,     // 源码位置
    pub help: Option<String>,   // 修复建议
    pub related: Vec<Box<Diagnostic>>,  // 关联诊断
}
```

## DiagnosticBuilder 模式

通过 `ErrorCodeDefinition` 获取 builder，链式调用设置参数：

```rust
let diagnostic = ErrorCodeDefinition::unknown_variable("x")
    .at(span)
    .help("did you mean 'y'?")
    .build();
```

## i18n 支持

所有错误码的标题和帮助文本通过 `I18nRegistry` 管理，支持中英文切换。消息模板支持 `{param}` 占位符。

## Emitter 输出

- `TextEmitter`：文本格式输出，支持颜色、Unicode 符号
- `JsonEmitter`：JSON 格式输出，用于 CI 和 LSP
