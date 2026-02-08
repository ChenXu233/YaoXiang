# Task 15.1: 核心模块

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

提供基础功能模块，包括 `Option`、`Result`、`panic` 等。

## 核心类型

```yaoxiang
# Option 类型
Option[T] = enum {
    Some(T),
    None,
}

# Result 类型
Result[T, E] = enum {
    Ok(T),
    Err(E),
}

# 常用函数
assert(condition: Bool, message: String = "assertion failed")
panic(message: String) -> !
print(value: T)
println(value: T)
format(template: String, args: ...T) -> String
```

## 验收测试

```yaoxiang
# test_core.yx

# Option
opt = Option::Some(42)
value = opt.unwrap_or(0)
assert(value == 42)

# Result
result = Result::Ok(100)
value = result.unwrap()
assert(value == 100)

# print
print("Hello, YaoXiang!")
println("With newline")

# format
msg = format("{} + {} = {}", 1, 2, 3)
assert(msg == "1 + 2 = 3")

print("Core tests passed!")
```

## 相关文件

- **core/mod.rs**
- **core/option.rs**
- **core/result.rs**
