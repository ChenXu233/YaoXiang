# Task 4.3: 控制流字节码

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

生成控制流语句（if、while、for、break、continue、return）的字节码。

## 设计原则

**基于跳转指令实现**：使用现有的 `Jmp`/`JmpIf`/`JmpIfNot` 指令，通过标签和偏移量实现所有控制流。

## 字节码指令（复用现有指令）

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `Jmp` | 0x03 | 无条件跳转 | label_id (u8，标签ID) |
| `JmpIf` | 0x04 | 条件为真跳转 | cond_reg, label_id |
| `JmpIfNot` | 0x05 | 条件为假跳转 | cond_reg, label_id |
| `Switch` | 0x06 | 多分支跳转 | cond_reg, default_offset, table_idx |
| `LoopStart` | 0x07 | 循环开始 | current_reg, end_reg, step_reg, exit_label |
| `LoopInc` | 0x08 | 循环递增 | current_reg, step_reg, loop_start_label |
| `Label` | 0x0B | 标签定义 | label_id (u8，标签ID) |
| `Return` | 0x01 | 无返回值返回 | |
| `ReturnValue` | 0x02 | 带返回值返回 | value_reg |

**说明**：`break`/`continue` 通过 `Jmp` + 偏移量实现，不需要专用指令。

## 字节码格式

```rust
// 所有指令统一使用：opcode + operands
BytecodeInstruction { opcode: u8, operands: Vec<u8> }

// 跳转指令操作数（两遍生成 + 回填偏移量）
// - Jmp: label_id (u8)
// - JmpIf/JmpIfNot: cond_reg (u8), label_id (u8)
// - Label: label_id (u8)
// - Switch: cond_reg (u8), default_label (u8), table_idx (u8)
// - LoopStart: current_reg, end_reg, step_reg, exit_label (all u8)
// - LoopInc: current_reg, step_reg, loop_start_label (all u8)
```

**生成流程**：
1. 第一遍：生成所有指令，记录跳转目标为 label_id
2. 第二遍：遍历所有跳转指令，将 label_id 转换为相对偏移量

## 生成规则

### if 语句
```yaoxiang
if cond1 {
    then_branch
} elif cond2 {
    elif_branch
} else {
    else_branch
}
```
生成字节码：
```
# 条件检查
LOAD cond1 -> r1
JmpIfNot r1, elif1_label   # 条件为假跳转到第一个 elif

# then 分支
then_label:
then_branch
Jmp end_label              # 跳过 elif 和 else

# elif 分支 1
elif1_label:
LOAD cond2 -> r2
JmpIfNot r2, elif2_label   # 条件为假跳转到下一个 elif

elif1_body_label:
elif_branch1
Jmp end_label              # 跳过其他 elif 和 else

# elif 分支 2（如果有多个）
elif2_label:
...（类似处理）

# else 分支
else_label:
else_branch

end_label:
```

### while 循环
```yaoxiang
while cond {
    body
}
```
生成字节码：
```
# 循环开始标签
loop_label:
# 条件检查
LOAD cond -> r1
JmpIfNot r1, end_label     # 条件为假，退出循环

# 循环体
body
Jmp loop_label             # 跳回继续条件检查

end_label:
```

### break/continue 语句
```yaoxiang
while i < 10 {
    if i == 5 {
        break  # 跳出循环
    }
    if i == 3 {
        continue  # 继续下一次迭代
    }
    i = i + 1
}
```
生成字节码：
```
loop_label:
LOAD i -> r1
CONST 10 -> r2
I64Lt r1, r2 -> r3
JmpIfNot r3, end_label   # 条件为假，退出循环

# break: 跳转到循环结束
LOAD i -> r4
CONST 5 -> r5
I64Eq r4, r5 -> r6
JmpIfNot r6, check_continue
Jmp end_label            # break: 跳转到 end_label

# continue: 跳回循环开始
check_continue:
LOAD i -> r7
CONST 3 -> r8
I64Eq r7, r8 -> r9
JmpIfNot r9, increment
Jmp loop_label           # continue: 跳回 loop_label

increment:
# i = i + 1
LOAD i -> r10
CONST 1 -> r11
I64Add r10, r11 -> r12
StoreLocal r12, i
Jmp loop_label           # 继续下一次迭代

end_label:
```

### for 循环（范围迭代）
```yaoxiang
for i in 0..5 {
    body
}
```
生成字节码：
```
# 生成 start 和 end 表达式
CONST 0 -> r1
CONST 5 -> r2

# 分配循环变量寄存器
current = r3

# 使用 LoopStart 指令开始循环
# 操作数：current_reg, end_reg, step_reg, exit_label
LoopStart current, r2, CONST(1), exit_label

# 循环体
body

# 使用 LoopInc 指令递增循环变量
# 操作数：current_reg, step_reg, loop_start_label
LoopInc current, CONST(1), loop_label

exit_label:
```

## 验收测试

```yaoxiang
# test_control_flow_bytecode.yx

# if 语句
x = 10
result = if x > 5 { "big" } else { "small" }
assert(result == "big")

# if-elif-else 语句
y = 15
result2 = if y < 10 {
    "small"
} elif y < 20 {
    "medium"
} else {
    "large"
}
assert(result2 == "medium")

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

- **src/middle/opcode.rs**: TypedOpcode 枚举定义
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: 控制流生成逻辑
