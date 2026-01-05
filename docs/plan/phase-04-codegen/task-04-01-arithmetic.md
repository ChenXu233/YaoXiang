# Task 4.1: 算术运算字节码

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

生成算术运算（加减乘除、取模、取负）的字节码。

## 字节码指令

| Opcode | 操作 | 说明 |
|--------|------|------|
| `ADD` | 加法 | `a + b` |
| `SUB` | 减法 | `a - b` |
| `MUL` | 乘法 | `a * b` |
| `DIV` | 除法 | `a / b` |
| `MOD` | 取模 | `a % b` |
| `NEG` | 取负 | `-a` |

## 字节码格式

```rust
// 一元运算
struct Neg { result: Reg, operand: Reg }

// 二元运算
struct Add { result: Reg, left: Reg, right: Reg }
```

## 生成规则

### 加法
```yaoxiang
x = a + b
```
生成字节码：
```
LOAD a -> r1
LOAD b -> r2
ADD r1, r2 -> r3
STORE r3 -> x
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

- **bytecode.rs**: 算术指令定义
- **generator.rs**: 算术表达式生成逻辑
