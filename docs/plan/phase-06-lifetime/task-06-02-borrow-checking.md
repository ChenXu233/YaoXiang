# Task 6.2: 借用检查

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

检查借用是否符合生命周期规则。

## 借用类型

| 类型 | 说明 | 规则 |
|------|------|------|
| `Shared` | 共享借用 | 可多个，不可变 |
| `Mutable` | 可变借用 | 唯一，可变 |
| `Unique` | 唯一借用 | 独占所有权 |

## 借用规则

```rust
enum BorrowError {
    ImmutableBorrowWhileMutablyBorrowed {
        borrower: Var,
        borrowed: Var,
        span: Span,
    },
    MutableBorrowWhileMutablyBorrowed {
        borrower: Var,
        borrowed: Var,
        span: Span,
    },
    BorrowedValueDropped {
        borrower: Var,
        span: Span,
    },
}
```

## 借用检查示例

```yaoxiang
# 正确示例
fn correct_borrow() {
    x = 42
    r1 = &x  # 共享借用
    r2 = &x  # 另一个共享借用
    print(r1 + r2)  # OK

    # r1, r2 在此处结束
}

# 错误示例：可变借用时共享借用
fn invalid_borrow() {
    mut x = 42
    r1 = &mut x  # 可变借用
    r2 = &x      # 错误：共享借用与可变借用冲突
}

# 错误示例：借用检查
fn use_after_free() {
    x = 42
    r = &x
    drop(x)      # 错误：x 被释放但仍在被借用
    print(r)
}
```

## 相关文件

- **borrow.rs**: BorrowChecker
