# Task 3.6: 错误处理

> **优先级**: P1
> **状态**: ⚠️ 部分实现

## 功能描述

提供详细的类型错误信息和错误恢复机制。

## 错误类型

```rust
enum TypeError {
    UnknownVariable {
        name: String,
        span: Span,
    },
    UnknownType {
        name: String,
        span: Span,
    },
    TypeMismatch {
        expected: MonoType,
        found: MonoType,
        span: Span,
    },
    TypeInferenceFailed {
        reason: String,
        span: Span,
    },
    InfiniteType {
        type_var: TypeVar,
        occuring_type: MonoType,
        span: Span,
    },
    UnknownField {
        struct_name: String,
        field_name: String,
        span: Span,
    },
    UnknownVariant {
        enum_name: String,
        variant_name: String,
        span: Span,
    },
    UnmatchedPattern {
        pattern: Pattern,
        value_type: MonoType,
        span: Span,
    },
    IndexOutOfBounds {
        index: i128,
        size: usize,
        span: Span,
    },
    NonExhaustivePatterns {
        missing: Vec<MonoType>,
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

## 相关文件

- **errors.rs**: TypeError 定义
