# Task 6.3: 移动语义

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

实现值的移动语义和 Copy 语义区分。

## 移动规则

```rust
enum Ownership {
    Owned,
    Moved(MovedFrom),
    Copied,
}

struct MovedFrom {
    /// 移动来源位置
    location: Span,
    /// 是否已移动
    is_moved: bool,
}
```

## 移动语义示例

```yaoxiang
# 可移动类型
fn move_semantics() {
    x = [1, 2, 3]  # 拥有所有权
    y = x          # x 被移动到 y
    # print(x)     # 错误：x 已被移动

    z = x.clone()  # 克隆副本
}

# Copy 类型（标量类型）
fn copy_semantics() {
    x = 42  # Copy 类型
    y = x   # x 被复制（不是移动）
    print(x)  # OK，x 仍然可用
}

# 移动语义函数
fn take_ownership(arr: List[Int]) {
    # arr 的所有权被转移
    process(arr)
}

fn give_ownership() -> List[Int] {
    data = [1, 2, 3]
    data  # 返回时移动所有权
}
```

## 相关文件

- **move.rs**: Move semantics handler
