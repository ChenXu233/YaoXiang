# Task 5.2: 别名分析

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

分析变量之间的别名关系，用于更精确的逃逸分析。

## 别名关系

```rust
struct AliasInfo {
    /// 变量别名集合
    aliases: HashSet<Var>,
    /// 是否被外部引用
    referenced_externally: bool,
    /// 修改次数
    modify_count: usize,
}
```

## 分析规则

```yaoxiang
# 无别名
a = 42
b = a          # b 是 a 的副本，不是别名

# 有别名
ptr1 = &x
ptr2 = ptr1    # ptr2 是 ptr1 的别名，也指向 x

# 结构体别名
p1 = Point(1, 2)
p2 = p1        # p2 是 p1 的别名
p2.x = 100     # p1.x 也变成 100
```

## 别名分析类型

| 类型 | 说明 |
|------|------|
| `NoAlias` | 无别名，可以优化 |
| `MayAlias` | 可能有别名 |
| `MustAlias` | 必有别名 |
| `PartialAlias` | 部分别名 |

## 相关文件

- **alias.rs**: 别名分析实现
