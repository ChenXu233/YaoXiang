# Task 4.8: 错误处理字节码

> **优先级**: P1
> **状态**: ✅ 已实现

## 功能描述

生成错误处理（`?` 运算符、Result/Option 传播）的字节码。

## 设计原则（基于 RFC-008 决策）

**核心原则**：**错误处理是类型问题，不是控制流问题**

根据 RFC-008 的设计哲学（并发是运行时问题），错误处理同样遵循：
- **`?` 运算符**：类型检查 + 条件跳转，不是什么"异常机制"
- **Result/Option**：普通代数数据类型，不需要特殊字节码
- **try-catch**：不需要，Rust 不用异常
- **panic**：仅用于编程错误，不是日常错误处理

## 代码生成策略

| 功能 | 字节码实现 | 说明 |
|------|-----------|------|
| `?` 错误传播 | `TypeCheck` + `JmpIfNot` + `GetField` + `ReturnValue` | 模式匹配变体 |
| Result 类型 | **不需要特殊字节码** | 普通结构体 `Ok(value) \| Err(error)` |
| Option 类型 | **不需要特殊字节码** | 普通结构体 `Some(value) \| None` |
| `panic` | `Throw` | 仅用于编程错误（不可恢复） |
| TryBegin/TryEnd/Rethrow | **不存在** | Java 风格异常，YaoXiang 不需要 |

## 当前实现状态

```rust
// src/middle/codegen/mod.rs

// 根据 RFC-008 原则，错误处理是类型问题：
// - ? 运算符使用 TypeCheck + JmpIfNot + GetField + ReturnValue
// - TryBegin/TryEnd/Throw/Rethrow 已从 IR 中移除
```

**说明**：`?` 运算符的字节码生成与模式匹配完全一致：
1. `TypeCheck` 检查是否为 `Err` 变体
2. `JmpIfNot` 为假则继续执行（成功路径）
3. `GetField` 提取错误值
4. `ReturnValue` 返回错误

## 字节码指令（复用现有指令）

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `TypeCheck` | 0xC0 | 类型检查 | obj_reg, type_id, dst |
| `GetField` | 0x73 | 获取字段 | dst, obj_reg, field_offset |
| `JmpIfNot` | 0x05 | 条件为假跳转 | cond_reg, offset |
| `ReturnValue` | 0x02 | 带返回值返回 | value_reg |
| `Throw` | 0xA2 | 抛出异常 | exception_reg（仅用于 panic） |

**说明**：模式匹配和错误传播复用同一套指令。

## 生成规则

### `?` 错误传播（核心）
```yaoxiang
result = might_fail()?
```

生成字节码：
```
# 调用可能失败的函数
CallStatic r1, func_id=might_fail, base_arg=?, arg_count=0

# 检查是否是 Err（与模式匹配完全相同）
TypeCheck r1, type_id=Err, r2
JmpIfNot r2, continue      # 不是 Err，继续执行

# 是 Err，提取错误并返回（提前返回）
GetField r3, r1, 0         # 获取 error 字段
ReturnValue r3             # 返回错误

continue:
# 正常路径，r1 是 Ok 值
STORE r1 -> result
```

**这与 `match` 表达式生成的字节码完全一致**：
```yaoxiang
result = match might_fail() {
    Ok(v) => v
    Err(e) => return Err(e)
}
```

### 链式 `?` 传播
```yaoxiang
final = step1()?.step2()?.step3()?
```

生成字节码：
```
# step1()?
CallStatic r1, func_id=step1, base_arg=?, arg_count=0
TypeCheck r1, type_id=Err, r2
JmpIfNot r2, step2
GetField r3, r1, 0
ReturnValue r3

# step2()?
step2:
CallStatic r4, func_id=step2, base_arg=?, arg_count=0
TypeCheck r4, type_id=Err, r5
JmpIfNot r5, step3
GetField r6, r4, 0
ReturnValue r6

# step3()?
step3:
CallStatic r7, func_id=step3, base_arg=?, arg_count=0
TypeCheck r7, type_id=Err, r8
JmpIfNot r8, done
GetField r9, r7, 0
ReturnValue r9

done:
STORE r7 -> final
```

### Result 类型定义（普通代数数据类型）
```yaoxiang
# Result 是普通类型，不需要特殊处理
type Result[T, E] = Ok(value: T) | Err(error: E)
```

字节码生成与普通联合类型完全相同。

### panic（仅用于编程错误）
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
JmpIfNot r3, continue

# 条件为真，panic
CONST "negative value not allowed" -> r4
Throw r4

continue:
```

**说明**：`panic` 不是日常错误处理，是程序无法继续执行的严重错误。

### 完整示例
```yaoxiang
safe_div(a: Int, b: Int): Result[Int, String] = if b == 0 {
    Err("division by zero")
} else {
    Ok(a / b)
}

main: () -> Void = () => {
    # ? 传播
    result = safe_div(10, 2)?
    print(result)

    # 处理错误情况
    error = match safe_div(5, 0) {
        Ok(v) => v
        Err(e) => {
            print("Error: " + e)
            0
        }
    }
}
```

生成字节码：
```
# safe_div(10, 2)?
CallStatic r1, func_id=safe_div, base_arg=?, arg_count=2
# 参数加载...
TypeCheck r1, type_id=Err, r2
JmpIfNot r2, print_result
GetField r3, r1, 0
ReturnValue r3  # 提前返回

print_result:
# print(result)...
```

## 验收测试

```yaoxiang
# test_error_handling_bytecode.yx

# 基础 ? 传播
safe_div(a: Int, b: Int): Result[Int, String] = if b == 0 {
    Err("division by zero")
} else {
    Ok(a / b)
}

result = safe_div(10, 2)?
assert(result == 5)

# 错误情况
error_result = safe_div(10, 0)?
# 这行不应该被执行，因为 safe_div(10, 0) 返回 Err

# 链式传播
combined: Result[Int, String] = safe_div(100, 10)?.into()

# match 处理（与 ? 底层机制相同）
value = match might_fail() {
    Ok(v) => v
    Err(e) => return Err(e)  # 等价于 ?
}

print("Error handling bytecode tests passed!")
```

## 相关文件

- **src/middle/opcode.rs**: TypedOpcode 枚举定义（TypeCheck, GetField, JmpIfNot, ReturnValue, Throw）
- **src/middle/codegen/mod.rs**: `?` 运算符生成逻辑（复用模式匹配）
- **src/middle/codegen/control_flow.rs**: 模式匹配代码生成
- **RFC-008**: Runtime 并发模型（同样的设计哲学：错误处理是类型问题）

## 设计决策

| 决策 | 原因 |
|------|------|
| `?` 用 TypeCheck + JmpIfNot | 与模式匹配一致，消除特殊情况 |
| 不需要 TryBegin/TryEnd | Rust 不使用异常机制 |
| Result 是普通类型 | 代数数据类型，模式匹配处理 |
| panic 仅用于编程错误 | 分离可恢复错误和不可恢复错误 |
