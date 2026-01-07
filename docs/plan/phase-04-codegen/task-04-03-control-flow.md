# Task 4.3: 控制流字节码

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

生成控制流语句（if、while、for、break、continue、return）的字节码。

## 设计原则

**基于跳转指令实现**：使用现有的 `Jmp`/`JmpIf`/`JmpIfNot` 指令，通过标签和偏移量实现所有控制流。

## 字节码指令（复用现有指令）

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `Jmp` | 0x03 | 无条件跳转 | offset (i32，相对偏移量) |
| `JmpIf` | 0x04 | 条件为真跳转 | cond_reg, offset (i16) |
| `JmpIfNot` | 0x05 | 条件为假跳转 | cond_reg, offset (i16) |
| `Switch` | 0x06 | 多分支跳转 | jump table |
| `LoopStart` | 0x07 | 循环开始 | 迭代器优化 |
| `LoopInc` | 0x08 | 循环递增 | 迭代器优化 |
| `Label` | 0x0B | 标签定义 | 跳转目标 |
| `Return` | 0x01 | 无返回值返回 | |
| `ReturnValue` | 0x02 | 带返回值返回 | value_reg |

**说明**：`break`/`continue` 通过 `Jmp` + 偏移量实现，不需要专用指令。

## 字节码格式

```rust
// 所有指令统一使用：opcode + operands
BytecodeInstruction { opcode: u8, operands: Vec<u8> }

// 跳转指令操作数
// - Jmp: offset (i32, 4 bytes)
// - JmpIf/JmpIfNot: cond_reg (u8), offset (i16, 2 bytes)
// - Label: label_id (u8)
```

## 生成规则

### if 语句
```yaoxiang
if cond {
    then_branch
} else {
    else_branch
}
```
生成字节码：
```
LOAD cond -> r1
JmpIfNot r1, else_offset   # 条件为假跳转到 else

# then 分支
then_branch
Jmp end_offset             # 跳过 else

# else 分支（偏移量计算）
else_offset:
else_branch

end_offset:
```

### while 循环
```yaoxiang
while cond {
    body
}
```
生成字节码：
```
# 跳转到条件检查
Jmp cond_offset

# 循环体
body_offset:
body
Jmp cond_offset            # 继续下一次条件检查

# 条件检查
cond_offset:
LOAD cond -> r1
JmpIfNot r1, end_offset    # 条件为假，退出循环

Jmp body_offset            # 条件为真，执行循环体

end_offset:
```

### break 语句
```yaoxiang
while i < 10 {
    if i == 5 {
        break  # 跳出循环
    }
    i = i + 1
}
```
生成字节码：
```
# ... 循环结构
JmpIfNot condition, end_offset   # 条件检查

# break: 跳转到循环结束
Jmp end_offset

# 循环体继续...
end_offset:
```

### for 循环（范围迭代）
```yaoxiang
for i in 0..5 {
    body
}
```
生成字节码：
```
# 使用 LoopStart/LoopInc 优化
LoopStart start=0, end=5, step=1, exit_offset=end
# 循环体
body
LoopInc current, step, loop_start_offset
end:
```

## 验收测试

```yaoxiang
# test_control_flow_bytecode.yx

# if 语句
x = 10
result = if x > 5 { "big" } else { "small" }
assert(result == "big")

# while 循环
sum = 0
i = 0
while i < 5 {
    sum = sum + i
    i = i + 1
}
assert(sum == 10)

# for 循环
total = 0
for i in 0..5 {
    total = total + i
}
assert(total == 10)

# return 语句
add(a, b) = a + b
assert(add(3, 4) == 7)

# break 语句
count = 0
i = 0
while i < 10 {
    i = i + 1
    if i == 5 {
        break
    }
    count = count + 1
}
assert(count == 4)

# continue 语句
sum = 0
i = 0
while i < 5 {
    i = i + 1
    if i == 3 {
        continue  # 跳过 3
    }
    sum = sum + i
}
assert(sum == 1 + 2 + 4 + 5)  # 跳过 3

print("Control flow bytecode tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: 控制流生成逻辑
