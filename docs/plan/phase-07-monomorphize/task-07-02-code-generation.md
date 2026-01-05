# Task 7.2: 代码生成

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

生成单态化后的具体代码。

## 生成过程

```yaoxiang
# 原始泛型函数
identity[T](x: T): T = x

# 单态化后
identity_int(x: Int): Int = x

identity_string(x: String): String = x

identity_float(x: Float): Float = x
```

## 代码生成策略

| 策略 | 描述 | 适用场景 |
|------|------|----------|
| `Eager` | 立即实例化 | 编译时常量 |
| `Lazy` | 延迟实例化 | 减少编译时间 |
| `Demand` | 按需实例化 | 仅实例化用到的 |

## 相关文件

- **specialize.rs**: CodeSpecializer
