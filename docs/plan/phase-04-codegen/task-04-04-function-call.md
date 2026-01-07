# Task 4.4: 函数调用字节码

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

生成函数调用和返回的字节码，包括参数传递和返回值处理。

## 设计原则

**三种调用分发**：
- **静态分发**：`CallStatic` - 直接函数调用（最快）
- **虚表分发**：`CallVirt` - trait/多态调用
- **动态分发**：`CallDyn` - 反射调用（带内联缓存）

**无 ENTER/LEAVE**：函数序言/尾声通过参数寄存器直接处理，不需要显式指令。

## 字节码指令

| Opcode | 值 | 操作 | 说明 | 实现状态 |
|--------|-----|------|------|----------|
| `CallStatic` | 0x80 | 静态分发调用 | dst, func_id(4), base_arg_reg, arg_count | ✅ 已实现 |
| `CallVirt` | 0x81 | 虚表分发调用 | dst, obj_reg, vtable_idx(2), base_arg_reg, arg_count | ✅ 已实现 |
| `CallDyn` | 0x82 | 动态分发调用 | dst, obj_reg, name_idx(2), base_arg_reg, arg_count | ✅ 已实现 |
| `MakeClosure` | 0x83 | 创建闭包 | dst, func_id(u32, 4字节), upvalue_count | ✅ 已实现 |
| `LoadUpvalue` | 0x84 | 加载 Upvalue | dst, upvalue_idx | ✅ 已实现 |
| `StoreUpvalue` | 0x85 | 存储 Upvalue | src, upvalue_idx | ✅ 已实现 |
| `CloseUpvalue` | 0x86 | 关闭 Upvalue | reg | ✅ 已实现 |
| `Return` | 0x01 | 无返回值返回 | | ✅ 已实现 |
| `ReturnValue` | 0x02 | 带返回值返回 | value_reg | ✅ 已实现 |
| `TailCall` | 0x09 | 尾调用优化 | func_id, base_arg_reg, arg_count | ✅ 已实现 |

## 字节码格式

```rust
// 调用指令操作数
// CallStatic: dst(1), func_id(4), base_arg_reg(1), arg_count(1) = 7 字节
// CallVirt: dst(1), obj_reg(1), vtable_idx(2), base_arg_reg(1), arg_count(1) = 6 字节
// CallDyn: dst(1), obj_reg(1), name_idx(2), base_arg_reg(1), arg_count(1) = 6 字节
// MakeClosure: dst(1), func_id(4), upvalue_count(1) = 6 字节
// LoadUpvalue: dst(1), upvalue_idx(1) = 2 字节
// StoreUpvalue: src(1), upvalue_idx(1) = 2 字节
// CloseUpvalue: reg(1) = 1 字节
// TailCall: func_id(4), base_arg_reg(1), arg_count(1) = 6 字节
// ReturnValue: value_reg(1)
```

## 生成规则

### 直接调用
```yaoxiang
result = add(a, b)
```
生成字节码：
```
LOAD a -> r1
LOAD b -> r2
CallStatic r3, func_id=add, base_arg=r1, arg_count=2
STORE r3 -> result
```

### 闭包调用
```yaoxiang
adder = (x) => (y) => x + y
add5 = adder(5)
result = add5(10)
```
生成字节码：
```
# 创建闭包 adder
# MakeClosure: dst(1), func_id(4), upvalue_count(1)
MakeClosure r1, func_id=0x..., upvalue_count=1

# 填充 upvalue（x）
StoreUpvalue r_x, 0

# 调用 adder(5)
LoadUpvalue r2, upvalue_idx=0  # x
CallStatic r3, func_id=adder$1, base_arg=r1, arg_count=1
STORE r3 -> add5

# 调用 add5(10)
CallStatic r4, func_id=add5, base_arg=r4, arg_count=1
STORE r4 -> result
```

### 虚表调用（多态）
```yaoxiang
# trait Draw { draw(self) }
# type Circle = Circle(radius)
# impl Draw for Circle { draw(self) => ... }

circle = Circle(5)
draw(circle)  # 虚表调用
```
生成字节码：
```
NEW_STRUCT Circle -> r1
CONST 5 -> r2
SetField r1, 0, r2  # 设置 radius

CallVirt r3, obj_reg=r1, vtable_idx=draw, base_arg=r1, arg_count=1
```

### 尾调用优化
```yaoxiang
fact(n, acc) = if n <= 1 { acc } else { fact(n - 1, n * acc) }
```
生成字节码：
```
# 条件检查
LOAD n -> r1
CONST 1 -> r2
I64Le r1, r2 -> r3
JmpIfNot r3, recursive_branch

# 基本情况：返回 acc
LOAD acc -> r4
ReturnValue r4

# 递归分支：尾调用
recursive_branch:
# 计算新参数（复用寄存器）
LOAD n -> r5
CONST 1 -> r6
I64Sub r5, r6 -> r7  # n - 1
LOAD n -> r8
LOAD acc -> r9
I64Mul r8, r9 -> r10 # n * acc

# 尾调用
TailCall func_id=fact, base_arg=r7, arg_count=2
```

## 验收测试

```yaoxiang
# test_function_call_bytecode.yx

# 基本函数调用
add(a, b) = a + b
assert(add(1, 2) == 3)

# 嵌套调用
f(g(x)) = g(x) * 2
double(n) = n * 2
assert(f(double(5)) == 20)

# 递归
fact(n) = if n <= 1 { 1 } else { n * fact(n - 1) }
assert(fact(5) == 120)

# 闭包
make_adder(x) = (y) => x + y
add5 = make_adder(5)
assert(add5(3) == 8)

# 尾调用优化
sum_to(n, acc) = if n <= 0 { acc } else { sum_to(n - 1, acc + n) }
assert(sum_to(100, 0) == 5050)

print("Function call bytecode tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: 函数调用生成逻辑
