# Task 4.6: 模式匹配字节码

> **优先级**: P1
> **状态**: ✅ 已实现

## 功能描述

生成 match 表达式和模式匹配的字节码。

## 设计原则

**基于现有指令实现**：使用 `TypeCheck` + `GetField` + 跳转指令实现模式匹配，不需要专用的 PAT_xxx 指令。

语言规范（language-spec §4.8）：
```
MatchArm ::= Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

## 字节码指令（复用现有指令）

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `TypeCheck` | 0xC0 | 类型检查 | obj_reg, type_id, dst |
| `GetField` | 0x73 | 获取字段 | dst, obj_reg, field_offset |
| `Jmp` | 0x03 | 无条件跳转 | offset |
| `JmpIfNot` | 0x05 | 条件为假跳转 | cond_reg, offset |

**说明**：模式匹配不需要专用指令，用类型检查 + 字段提取 + 跳转实现。

## 字节码格式

```rust
// TypeCheck: obj_reg(1), type_id(2), dst(1)
// GetField: dst(1), obj_reg(1), field_offset(2)
// JmpIfNot: cond_reg(1), offset(2)
```

## 生成规则

### 字面量模式
```yaoxiang
match n {
    0 => "zero"
    1 => "one"
    _ => "many"
}
```
生成字节码：
```
LOAD n -> r1

# 测试 n == 0
CONST 0 -> r2
I64Eq r1, r2 -> r3
JmpIfNot r3, try_one    # 为假，跳到下一个模式
Jmp match_zero

try_one:
# 测试 n == 1
CONST 1 -> r4
I64Eq r1, r4 -> r5
JmpIfNot r5, try_wildcard
Jmp match_one

try_wildcard:
# 通配符，总是匹配
Jmp match_many

match_zero:
CONST "zero" -> r6
Jmp match_end

match_one:
CONST "one" -> r7
Jmp match_end

match_many:
CONST "many" -> r8

match_end:
STORE r8 -> result
```

### 构造器模式
```yaoxiang
match opt {
    Some(n) => n * 2
    None => 0
}
```
生成字节码：
```
LOAD opt -> r1

# 测试 Some
TypeCheck r1, type_id=Some, r2
JmpIfNot r2, try_none    # 不是 Some，跳到 None

# 提取 Some 的内部值
# Some 类型定义：type Some[T] = Some(value: T)
GetField r3, r1, 0       # 字段偏移 0 是 value

# 执行分支体
I64Mul r3, CONST(2) -> r4
Jmp match_end

try_none:
# 测试 None
TypeCheck r1, type_id=None, r5
JmpIfNot r5, match_fail   # 也不是 None，匹配失败

# None 分支
CONST 0 -> r6
Jmp match_end

match_fail:
# 无匹配 - 运行时错误
Throw NonExhaustivePatterns

match_end:
STORE r4 -> result
```

### 守卫模式
```yaoxiang
sign(n) = match n {
    x if x < 0 => "negative"
    0 => "zero"
    x => "positive"
}
```
生成字节码：
```
LOAD n -> r1

# 第一个分支：x if x < 0
# 先匹配通配符 x（总是成功）
Jmp test_guard

# 测试守卫条件
test_guard:
I64Lt r1, CONST(0) -> r2
JmpIfNot r2, try_zero   # 守卫为假，跳到下一个

CONST "negative" -> r3
Jmp match_end

try_zero:
# 匹配 0
CONST 0 -> r4
I64Eq r1, r4 -> r5
JmpIfNot r5, try_wildcard2

CONST "zero" -> r6
Jmp match_end

try_wildcard2:
# 通配符分支
CONST "positive" -> r7

match_end:
STORE result -> result
```

### 嵌套模式
```yaoxiang
result = match data {
    Ok(Some(value)) => value
    Err(_) => "error"
    _ => "unknown"
}
```
生成字节码：
```
LOAD data -> r1

# 测试 Ok
TypeCheck r1, type_id=Ok, r2
JmpIfNot r2, try_err

# Ok 的第一个字段是 Result 的泛型参数
# Ok[T, E] = Ok(value: T) | Err(error: E)
# 假设 Ok(Some(Int))，需要检查嵌套

# 获取 Ok 的 value 字段
GetField r3, r1, 0

# 检查 value 是否是 Some
TypeCheck r3, type_id=Some, r4
JmpIfNot r4, err_branch

# 获取 Some 的内部值
GetField r5, r3, 0
Jmp match_end

err_branch:
# Err 分支
CONST "error" -> r6
Jmp match_end

# 通配符分支
CONST "unknown" -> r7

match_end:
STORE result -> result
```

## 验收测试

```yaoxiang
# test_pattern_matching_bytecode.yx

# 基础模式匹配
describe(n) = match n {
    0 => "zero"
    1 => "one"
    _ => "many"
}
assert(describe(0) == "zero")
assert(describe(5) == "many")

# 构造器模式
result = match Option::Some(42) {
    Some(n) => n * 2
    None => 0
}
assert(result == 84)

# 守卫模式
sign(n) = match n {
    x if x < 0 => "negative"
    0 => "zero"
    x => "positive"
}
assert(sign(-5) == "negative")
assert(sign(0) == "zero")
assert(sign(5) == "positive")

# 嵌套模式
result = match Result::Ok(Option::Some("value")) {
    Ok(Some(s)) => s
    _ => "default"
}
assert(result == "value")

# 多个守卫
classify(n) = match n {
    x if x < 0 => "negative"
    x if x == 0 => "zero"
    x if x < 10 => "small positive"
    _ => "large"
}
assert(classify(-5) == "negative")
assert(classify(0) == "zero")
assert(classify(5) == "small positive")
assert(classify(100) == "large")

print("Pattern matching bytecode tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义（TypeCheck, GetField, Jmp, JmpIfNot）
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: match 表达式生成逻辑
