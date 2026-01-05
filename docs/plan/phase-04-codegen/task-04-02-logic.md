# Task 4.2: 逻辑运算字节码

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

生成逻辑运算（与、或、非、比较）的字节码。

## 字节码指令

| Opcode | 操作 | 说明 |
|--------|------|------|
| `AND` | 逻辑与 | `a && b` |
| `OR` | 逻辑或 | `a \|\| b` |
| `NOT` | 逻辑非 | `!a` |
| `EQ` | 相等 | `a == b` |
| `NE` | 不等 | `a != b` |
| `LT` | 小于 | `a < b` |
| `LE` | 小于等于 | `a <= b` |
| `GT` | 大于 | `a > b` |
| `GE` | 大于等于 | `a >= b` |

## 字节码格式

```rust
struct Not { result: Reg, operand: Reg }
struct Eq { result: Reg, left: Reg, right: Reg }
```

## 生成规则

### 短路求值
```yaoxiang
if a && b { ... }
```
生成字节码：
```
LOAD a -> r1
IF_FALSE r1 -> end
LOAD b -> r2
IF_FALSE r2 -> end
# 两个都为 true
JUMP -> body
end:
```

## 验收测试

```yaoxiang
# test_logic_bytecode.yx

# 逻辑运算
assert(true && true)
assert(true || false)
assert(!false)

# 比较运算
assert(1 == 1)
assert(2 != 3)
assert(1 < 2)
assert(2 <= 2)
assert(3 > 1)
assert(3 >= 3)

# 混合
assert(1 < 2 && 3 < 4)
assert(true || false && false)

print("Logic bytecode tests passed!")
```

## 相关文件

- **bytecode.rs**: 逻辑指令定义
- **generator.rs**: 逻辑表达式生成逻辑
