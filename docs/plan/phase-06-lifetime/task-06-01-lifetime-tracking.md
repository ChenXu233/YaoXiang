# Task 6.1: 生命周期跟踪

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

跟踪每个值的生命周期范围。

## 生命周期表示

```rust
struct Lifetime {
    /// 生命周期开始位置
    start: Position,
    /// 生命周期结束位置
    end: Position,
    /// 父生命周期
    parent: Option<LifetimeId>,
    /// 子生命周期
    children: Vec<LifetimeId>,
}

enum ValueSource {
    Owned(Lifetime),
    Borrowed {
        owner: Var,
        lifetime: Lifetime,
        borrow_type: BorrowType,
    },
}
```

## 生命周期规则

```yaoxiang
fn lifetime_example() {
    # 'a 开始
    x = 42
    # 'a 结束

    # 'b 开始
    y = "hello"
    # 'b 结束
}

fn nested_lifetime() {
    outer = Point(1, 2)

    # inner 开始，嵌套在 outer 内
    inner = &outer.x

    use(inner)
    # inner 结束

    # outer 结束
}

fn return_lifetime() -> &Int {
    x = 42
    &x  # 错误：x 的生命周期在返回后结束
}
```

## 相关文件

- **tracker.rs**: LifetimeTracker
