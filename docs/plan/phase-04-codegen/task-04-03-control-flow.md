# Task 4.3: 控制流字节码

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

生成控制流语句（if、while、for、break、continue、return）的字节码。

## 字节码指令

| Opcode | 操作 | 说明 |
|--------|------|------|
| `JUMP` | 无条件跳转 | |
| `JUMP_IF_FALSE` | 条件跳转 | 条件为假时跳转 |
| `JUMP_IF_TRUE` | 条件跳转 | 条件为真时跳转 |
| `LABEL` | 标签 | 跳转目标 |
| `RETURN` | 返回 | 函数返回 |

## 字节码格式

```rust
struct Jump { target: Label }
struct JumpIfFalse { condition: Reg, target: Label }
struct Return { value: Reg }
struct Label { id: usize }
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
# 条件判断
LOAD cond -> r1
JUMP_IF_FALSE r1 -> else_label

# then 分支
then_branch
JUMP -> end_label

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
# 循环入口
JUMP -> cond_label

cond_label:
LOAD cond -> r1
JUMP_IF_FALSE r1 -> end_label

# 循环体
body
JUMP -> cond_label

end_label:
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

# break/continue
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

print("Control flow bytecode tests passed!")
```

## 相关文件

- **bytecode.rs**: 控制流指令定义
- **generator.rs**: 控制流生成逻辑
