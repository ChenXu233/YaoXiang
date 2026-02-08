# Task 3.6: 错误处理

> **优先级**: P1
> **状态**: ✅ 已实现

## 功能描述

提供详细的类型错误信息和错误恢复机制。

## 错误类型

```rust
enum TypeError {
    // 类型不匹配错误
    TypeMismatch {
        expected: MonoType,
        found: MonoType,
        span: Span,
    },

    // 未知变量错误
    UnknownVariable {
        name: String,
        span: Span,
    },

    // 未知类型错误
    UnknownType {
        name: String,
        span: Span,
    },

    // 参数数量不匹配错误
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    // 递归类型定义错误
    RecursiveType {
        name: String,
        span: Span,
    },

    // 不支持的操作错误
    UnsupportedOp {
        op: String,
        span: Span,
    },

    // 泛型约束违反错误
    GenericConstraint {
        constraint: String,
        span: Span,
    },

    // 无限类型错误
    InfiniteType {
        var: String,
        ty: MonoType,
        span: Span,
    },

    // 未实例化的类型变量错误
    UnboundTypeVar {
        var: String,
        span: Span,
    },

    // 未知标签错误（break/continue）
    UnknownLabel {
        name: String,
        span: Span,
    },

    // 未知字段错误
    UnknownField {
        struct_name: String,
        field_name: String,
        span: Span,
    },

    // 下标越界错误
    IndexOutOfBounds {
        index: i128,
        size: usize,
        span: Span,
    },

    // 函数调用错误
    CallError {
        message: String,
        span: Span,
    },

    // 赋值错误
    AssignmentError {
        message: String,
        span: Span,
    },

    // 类型推断错误
    InferenceError {
        message: String,
        span: Span,
    },

    // 无法推断参数类型错误
    CannotInferParamType {
        name: String,
        span: Span,
    },
}
```

## 错误信息格式

```
Error: Type mismatch
   --> test.yaoxiang:10:15
    |
 10 | x: Int = "hello"
     |               ^^^ expected `Int`, found `String`
```

## 代码实现

### 错误格式化

```rust
impl ErrorFormatter {
    pub fn format_error(&self, error: &TypeError) -> String {
        match error {
            TypeError::TypeMismatch { expected, found, .. } => {
                format!("Type mismatch: expected {}, found {}",
                    expected.type_name(), found.type_name())
            }
            TypeError::UnknownVariable { name, .. } => {
                format!("Unknown variable '{}'", name)
            }
            // ... 其他错误类型
        }
    }
}
```

### 诊断生成

```rust
impl From<TypeError> for Diagnostic {
    fn from(error: TypeError) -> Self {
        let span = error.span();
        match &error {
            TypeError::TypeMismatch { .. } => {
                Diagnostic::error("E0001".to_string(), format!("{}", error), span)
            }
            TypeError::UnknownVariable { .. } => {
                Diagnostic::error("E0002".to_string(), format!("{}", error), span)
            }
            // ... 其他错误类型映射
        }
    }
}
```

## 相关文件

- **errors.rs**: TypeError 定义、ErrorCollector、ErrorFormatter、Diagnostic
