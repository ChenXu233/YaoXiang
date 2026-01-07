# Task 4.8: 错误处理字节码

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

生成错误处理（panic、try-catch、? 运算符、Result/Option 传播）的字节码。

## 设计原则

**基于现有指令实现**：
- `?` 错误传播：用 `TypeCheck` + `JmpIfNot` + `GetField` 实现
- try-catch：用 `TryBegin`/`TryEnd`/`Throw`/`Rethrow` 实现
- panic：用 `Throw` 实现

## 字节码指令（复用现有指令）

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `TryBegin` | 0xA0 | try 块开始 | catch_offset (u16) |
| `TryEnd` | 0xA1 | try 块结束 | |
| `Throw` | 0xA2 | 抛出异常 | exception_reg |
| `Rethrow` | 0xA3 | 重新抛出异常 | |
| `TypeCheck` | 0xC0 | 类型检查 | obj_reg, type_id, dst |
| `GetField` | 0x73 | 获取字段 | dst, obj_reg, field_offset |
| `JmpIfNot` | 0x05 | 条件为假跳转 | cond_reg, offset |
| `ReturnValue` | 0x02 | 带返回值返回 | value_reg |

## 生成规则

### ? 错误传播（核心）
```yaoxiang
result = might_fail()?
```
生成字节码：
```
# 调用可能失败的函数
CallStatic r1, func_id=might_fail, base_arg=?, arg_count=0

# 检查是否是 Err
TypeCheck r1, type_id=Err, r2
JmpIfNot r2, continue      # 不是 Err，继续执行

# 是 Err，提取错误并返回
# Err 类型定义：type Err[E] = Err(error: E)
GetField r3, r1, 0         # 获取 error 字段
ReturnValue r3             # 返回错误

continue:
# 正常路径，r1 是 Ok 值
STORE r1 -> result
```

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
# try 块开始
TryBegin catch_offset

# try 块内容
CallStatic r1, func_id=risky_operation, base_arg=?, arg_count=0

# try 块正常结束
TryEnd
Jmp after_catch

# catch 块（偏移量由 TryBegin 记录）
catch_offset:
# e 是异常值（类型为 Error 或其子类型）
GetField r2, ???, 0        # 提取异常信息（如果有）
CallStatic r3, func_id=handle_error, base_arg=r2, arg_count=1

after_catch:
```

### 嵌套 try-catch
```yaoxiang
try {
    try {
        inner_risky()
    } catch e1 {
        handle1(e1)
    }
    outer_risky()
} catch e2 {
    handle2(e2)
}
```
生成字节码：
```
# 外层 try
TryBegin outer_catch_offset

# 内层 try
TryBegin inner_catch_offset

CallStatic r1, func_id=inner_risky, base_arg=?, arg_count=0
TryEnd
Jmp after_inner

inner_catch_offset:
CallStatic r2, func_id=handle_error, base_arg=?, arg_count=1
Jmp after_outer

after_inner:
CallStatic r3, func_id=outer_risky, base_arg=?, arg_count=0
TryEnd
Jmp after_catch

outer_catch_offset:
CallStatic r4, func_id=handle2, base_arg=?, arg_count=1

after_catch:
```

### panic
```yaoxiang
if x < 0 {
    panic("negative value not allowed")
}
```
生成字节码：
```
# 条件检查
LOAD x -> r1
CONST 0 -> r2
I64Lt r1, r2 -> r3
JmpIfNot r3, continue      # 条件为假，继续执行

# 条件为真，panic
CONST "negative value not allowed" -> r4
Throw r4

continue:
```

### rethrow
```yaoxiang
try {
    risky()
} catch e {
    if is_recoverable(e) {
        recover(e)
    } else {
        rethrow  # 重新抛出
    }
}
```
生成字节码：
```
TryBegin catch_offset

CallStatic r1, func_id=risky, base_arg=?, arg_count=0
TryEnd
Jmp after_catch

catch_offset:
CallStatic r2, func_id=is_recoverable, base_arg=?, arg_count=1
JmpIfNot r2, do_rethrow

CallStatic r3, func_id=recover, base_arg=?, arg_count=1
Jmp after_catch

do_rethrow:
Rethrow  # 重新抛出当前异常
```

## 验收测试

```yaoxiang
# test_error_handling_bytecode.yx

# ? 错误传播
safe_div(a: Int, b: Int): Result[Int, String] = if b == 0 {
    Err("division by zero")
} else {
    Ok(a / b)
}

result = safe_div(10, 2)?
assert(result == 5)

# try-catch
result = try {
    might_fail()
} catch e {
    default_value()
}
assert(result == default_value())

# panic
try {
    panic("test error")
} catch e {
    assert(e == "test error")
}

# 嵌套错误处理
outer_result = try {
    inner_result = try {
        might_fail()?
    } catch e1 {
        recover1(e1)?
    }
    more_risky(inner_result)?
} catch e2 {
    handle_outer(e2)
}
assert(outer_result == expected)

print("Error handling bytecode tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义（TryBegin, TryEnd, Throw, Rethrow）
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: 错误处理生成逻辑
- **RFC-001**: 并作模型与错误处理设计
