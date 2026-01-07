# Task 4.1: 算术运算字节码

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

生成算术运算（加减乘除、取模、取负）的字节码。

## 字节码设计原则

**类型特化指令**：每条指令携带明确的类型信息（I64/I32/F64/F32），运行时无需类型检查，直接执行对应 CPU 指令。

## 字节码指令

### I64 整数运算 (0x20-0x2F)

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `I64Add` | 0x20 | 加法 | `dst = lhs + rhs` |
| `I64Sub` | 0x21 | 减法 | `dst = lhs - rhs` |
| `I64Mul` | 0x22 | 乘法 | `dst = lhs * rhs` |
| `I64Div` | 0x23 | 除法 | `dst = lhs / rhs` |
| `I64Rem` | 0x24 | 取模 | `dst = lhs % rhs` |
| `I64Neg` | 0x2B | 取负 | `dst = -operand` |

### F64 浮点运算 (0x40-0x4F)

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `F64Add` | 0x40 | 加法 | `dst = lhs + rhs` |
| `F64Sub` | 0x41 | 减法 | `dst = lhs - rhs` |
| `F64Mul` | 0x42 | 乘法 | `dst = lhs * rhs` |
| `F64Div` | 0x43 | 除法 | `dst = lhs / rhs` |
| `F64Rem` | 0x44 | 取模 | `dst = lhs % rhs` |
| `F64Neg` | 0x46 | 取负 | `dst = -operand` |

## 字节码格式

所有算术指令统一使用三操作数格式（二元运算）或两操作数格式（一元运算）：

```rust
// 二元运算：dst, lhs, rhs (每个 1 byte)
BytecodeInstruction { opcode: u8, operands: [dst, lhs, rhs] }

// 一元运算（NEG）：dst, operand (每个 1 byte)
BytecodeInstruction { opcode: u8, operands: [dst, operand] }
```

## 生成规则

### 加法示例
```yaoxiang
x = a + b
```
生成字节码：
```
I64Const 1 -> r1      # 加载 a=1
I64Const 2 -> r2      # 加载 b=2
I64Add r1, r2 -> r3   # r3 = r1 + r2
StoreLocal r3, x      # x = r3
```

### 取负示例
```yaoxiang
x = 42
neg_x = -x
```
生成字节码：
```
I64Const 42 -> r1     # r1 = 42
StoreLocal r1, x      # x = 42
LoadLocal x -> r2     # r2 = x
I64Neg r2 -> r3       # r3 = -r2
StoreLocal r3, neg_x  # neg_x = -42
```

## 字节码生成器

**文件**: `src/middle/codegen/generator.rs`

类型感知的指令选择：根据操作数类型选择对应 TypedOpcode

```rust
Instruction::Add { dst, lhs, rhs } => {
    let type_ = self.get_operand_type(lhs);
    let opcode = match type_ {
        MonoType::Int(_) => TypedOpcode::I64Add,
        MonoType::Float(_) => TypedOpcode::F64Add,
        _ => TypedOpcode::I64Add,  // 默认 fallback
    };
    self.emit_arithmetic(opcode, dst, lhs, rhs);
}
```

## 验收测试

```yaoxiang
# test_arithmetic_bytecode.yx

# 基本算术
assert(1 + 2 == 3)
assert(10 - 4 == 6)
assert(3 * 5 == 15)
assert(10 / 2 == 5)
assert(7 % 3 == 1)

# 混合运算
result = (1 + 2) * 3 - 4 / 2
assert(result == 7)

# 取负
x = 42
neg_x = -x
assert(neg_x == -42)

print("Arithmetic bytecode tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: 算术表达式生成逻辑
