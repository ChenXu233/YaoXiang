# Task 4.2: 逻辑运算字节码

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

生成逻辑运算（比较、短路求值）的字节码。

## 字节码设计原则

**无专用逻辑指令**：逻辑运算直接使用比较指令和跳转实现，保持编译器简洁。

- `!a` → 用 `I64Eq a, 0` 实现
- `a && b` → 短路求值（跳转实现）
- `a || b` → 短路求值（跳转实现）

## 字节码指令

### 比较指令 (0x60-0x6F)

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `I64Eq` | 0x60 | 相等 | `dst = (lhs == rhs) ? 1 : 0` |
| `I64Ne` | 0x61 | 不等 | `dst = (lhs != rhs) ? 1 : 0` |
| `I64Lt` | 0x62 | 小于 | `dst = (lhs < rhs) ? 1 : 0` |
| `I64Le` | 0x63 | 小于等于 | `dst = (lhs <= rhs) ? 1 : 0` |
| `I64Gt` | 0x64 | 大于 | `dst = (lhs > rhs) ? 1 : 0` |
| `I64Ge` | 0x65 | 大于等于 | `dst = (lhs >= rhs) ? 1 : 0` |
| `F64Eq` | 0x66 | 相等 | 浮点版本 |
| `F64Ne` | 0x67 | 不等 | 浮点版本 |
| `F64Lt` | 0x68 | 小于 | 浮点版本 |
| `F64Le` | 0x69 | 小于等于 | 浮点版本 |
| `F64Gt` | 0x6A | 大于 | 浮点版本 |
| `F64Ge` | 0x6B | 大于等于 | 浮点版本 |

### 跳转指令 (用于短路求值)

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `JmpIf` | 0x04 | 条件为真跳转 | `if cond != 0 { jmp offset }` |
| `JmpIfNot` | 0x05 | 条件为假跳转 | `if cond == 0 { jmp offset }` |

## 字节码格式

```rust
// 比较指令：dst, lhs, rhs (每个 1 byte)
BytecodeInstruction { opcode: u8, operands: [dst, lhs, rhs] }

// 条件跳转：cond_reg, offset (i16)
BytecodeInstruction { opcode: u8, operands: [cond_reg, offset_lo, offset_hi] }
```

## 生成规则

### 短路求值示例
```yaoxiang
if a && b { body }
```
生成字节码：
```
LOAD a -> r1
I64Eq r1, 0 -> r2    # r2 = (a == 0)
JmpIfNot r2, skip    # if a != 0，继续；否则跳过 b
LOAD b -> r3
I64Eq r3, 0 -> r4    # r4 = (b == 0)
JmpIfNot r4, body    # if b != 0，执行 body
skip:
```

### 取反示例
```yaoxiang
x = !a
```
生成字节码：
```
LOAD a -> r1
I64Eq r1, 0 -> r2    # r2 = (a == 0)，即 !a
StoreLocal r2, x
```

## 验收测试

```yaoxiang
# test_logic_bytecode.yx

# 比较运算
assert(1 == 1)
assert(2 != 3)
assert(1 < 2)
assert(2 <= 2)
assert(3 > 1)
assert(3 >= 3)

# 逻辑运算（用比较实现）
assert(!(false))
assert(!(0))

# 短路求值
x = 5
if x > 0 && x < 10 {
    assert(true)
}

# 或运算
y = 0
if y == 0 || y > 100 {
    assert(true)
}

# 混合
assert(1 < 2 && 3 < 4)
assert(1 > 0 || 2 > 100)

print("Logic bytecode tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: 逻辑表达式生成逻辑
- **src/middle/codegen/control_flow.rs**: 短路求值控制流处理
