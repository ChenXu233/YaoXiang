# Task 4.8: 错误处理字节码

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

生成错误处理（panic、try-catch、Result/Option 传播）的字节码。

## 字节码指令

| Opcode | 操作 | 说明 |
|--------|------|------|
| `TRY_ENTER` | 尝试进入 | try 块开始 |
| `TRY_LEAVE` | 尝试离开 | try 块结束 |
| `CATCH` | 捕获异常 | catch 块开始 |
| `RAISE` | 抛出异常 | |
| `RETHROW` | 重新抛出 | |
| `ERROR` | 运行时错误 | |

## 字节码格式

```rust
struct TryEnter {
    try_label: Label,
    catch_label: Label,
}

struct Raise {
    error_reg: Reg,
    error_type: Option<String>,
}
```

## 生成规则

### try-catch
```yaoxiang
try {
    risky_operation()
} catch e {
    handle_error(e)
}
```
生成字节码：
```
TRY_ENTER try_block, catch_block
# try 块
LOAD risky_operation -> r1
TRY_LEAVE
JUMP -> after_catch

catch_block:
# catch 块
LOAD e -> err_reg
CALL handle_error(err_reg)

after_catch:
```

### 错误传播（? 操作符）
```yaoxiang
result = might_fail()?
```
生成字节码：
```
CALL might_fail() -> r1
# 检查是否是 Err
TEST_ISA r1, Err -> is_err
JUMP_IF_TRUE is_err -> propagate_error
JUMP -> continue
propagate_error:
# 提取错误并返回
EXTRACT_ERR r1 -> err
RETURN err
continue:
```

## 验收测试

```yaoxiang
# test_error_handling_bytecode.yx

# try-catch
result = try {
    might_fail()
} catch e {
    default_value()
}
assert(result == default_value())

# 错误传播
fn safe_div(a: Int, b: Int): Result[Int, String] {
    if b == 0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}

result = safe_div(10, 2)?
assert(result == 5)

# panic
try {
    panic("test error")
} catch e {
    assert(e == "test error")
}

print("Error handling bytecode tests passed!")
```

## 相关文件

- **bytecode.rs**: 错误处理指令定义
- **generator.rs**: 错误处理生成逻辑
