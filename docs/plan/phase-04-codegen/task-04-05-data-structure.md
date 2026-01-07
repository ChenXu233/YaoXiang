# Task 4.5: 数据结构字节码

> **优先级**: P1
> **状态**: 🔄 部分实现

## 功能描述

生成数据结构操作（列表、字典、元组、结构体）的字节码。

## 设计原则

**复用基础指令**：数据结构操作基于更底层的指令实现，不需要专用指令。
- 结构体：结构体是构造器，字段访问用 `GetField`/`SetField`
- 列表/字典：作为引用类型，用 `HeapAlloc` + 元素操作
- 列表访问：边界检查 + `LoadElement`/`StoreElement`
- 元组：匿名结构体，字段用数字索引 (0, 1, 2...)

## 字节码指令（复用现有指令）

| Opcode | 值 | 操作 | 说明 | 实现状态 |
|--------|-----|------|------|----------|
| `HeapAlloc` | 0x71 | 堆分配 | dst, type_id(u16) | ✅ 已实现 |
| `StackAlloc` | 0x70 | 栈分配 | size | ✅ 已实现 |
| `GetField` | 0x73 | 读取字段 | dst, obj_reg, field_offset(u16) | ✅ 已实现 |
| `SetField` | 0x75 | 写入字段 | obj_reg, field_offset(u16), src_reg | ✅ 已实现 |
| `LoadElement` | 0x76 | 加载元素 | dst, array_reg, index_reg | ✅ 已实现 |
| `StoreElement` | 0x77 | 存储元素 | array_reg, index_reg, src_reg | ✅ 已实现 |
| `NewListWithCap` | 0x78 | 预分配列表 | dst, capacity(u16) | ✅ 已实现 |
| `BoundsCheck` | 0xB0 | 边界检查 | array_reg, index_reg | ✅ 已实现 |
| `TypeCheck` | 0xC0 | 类型检查 | obj_reg, type_id(u16), dst | ⏳ 待实现 |
| **字典** | - | 字典字面量 | 调用 Dict.new + Dict.insert | ✅ 已实现 |
| **元组** | - | 元组字面量 | HeapAlloc + SetField(0,1,2...) | ✅ 已实现 |

## 字节码格式

```rust
// GetField: dst(1), obj_reg(1), field_offset(u16, 2字节) = 4 字节
// SetField: obj_reg(1), field_offset(u16, 2字节), src_reg(1) = 4 字节
// LoadElement: dst(1), array_reg(1), index_reg(1) = 3 字节
// StoreElement: array_reg(1), index_reg(1), src_reg(1) = 3 字节
// NewListWithCap: dst(1), capacity(u16, 2字节) = 3 字节
// BoundsCheck: array_reg(1), index_reg(1) = 2 字节
// TypeCheck: obj_reg(1), type_id(u16, 2字节), dst(1) = 4 字节
```

## 生成规则

### 结构体创建与访问
```yaoxiang
type Point = Point(x: Float, y: Float)
p = Point(x: 1.0, y: 2.0)
x = p.x
```
生成字节码：
```
# 创建 Point
HeapAlloc r1, type_id=Point
CONST 1.0 -> r2
SetField r1, 0, r2  # x 字段偏移 0
CONST 2.0 -> r3
SetField r1, 1, r3  # y 字段偏移 1
STORE r1 -> p

# 访问 p.x
GetField r4, r1, 0  # dst=r4, obj=r1, field_offset=0
STORE r4 -> x
```

### 列表创建和访问
```yaoxiang
nums = [1, 2, 3]
first = nums[0]
```
生成字节码：
```
# 创建列表（预分配容量）
NewListWithCap r1, capacity=3

# 添加元素
CONST 1 -> r2
StoreElement r1, r2, ???  # 需要运行时实现
CONST 2 -> r3
StoreElement r1, r3, ???
CONST 3 -> r4
StoreElement r1, r4, ???

STORE r1 -> nums

# 访问 nums[0]
CONST 0 -> r5
BoundsCheck r1, r5        # 边界检查
LoadElement r6, r1, r5    # 加载元素
STORE r6 -> first
```

### 字典（哈希表）
```yaoxiang
scores = {"alice": 90, "bob": 85}
alice_score = scores["alice"]
```
生成字节码：
```
# 字典是标准库类型，运行时提供
# 这里调用标准库构造函数
CallStatic r1, func_id=Dict.new, base_arg=?, arg_count=0
CallStatic r2, func_id=Dict.insert, base_arg=r1, arg_count=3
# ...

STORE r1 -> scores

# 查找 scores["alice"]
CONST "alice" -> r3
CallStatic r4, func_id=Dict.get, base_arg=r1, arg_count=2
STORE r4 -> alice_score
```

### 元组
```yaoxiang
pair = (1, "hello")
first = pair[0]
```
生成字节码：
```
# 元组是固定大小的结构体
HeapAlloc r1, type_id=Tuple(Int, String)
CONST 1 -> r2
SetField r1, 0, r2
CONST "hello" -> r3
SetField r1, 1, r3
STORE r1 -> pair

# 访问 pair[0] - 元组字段访问
GetField r4, r1, 0
STORE r4 -> first
```

## 验收测试

```yaoxiang
# test_data_structure_bytecode.yx

# 结构体
type Point = Point(x: Int, y: Int)
p = Point(x: 1, y: 2)
assert(p.x == 1)
assert(p.y == 2)

# 列表
nums = [1, 2, 3]
assert(nums[0] == 1)
assert(nums.length == 3)

# 字典
scores = {"alice": 90, "bob": 85}
assert(scores["alice"] == 90)

# 元组
pair = (1, "hello")
assert(pair[0] == 1)
assert(pair[1] == "hello")

# 嵌套结构
type Person = Person(name: String, age: Int)
alice = Person(name: "Alice", age: 30)
assert(alice.name == "Alice")
assert(alice.age == 30)

print("Data structure bytecode tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: 数据结构生成逻辑
- **标准库**: Dict/List 等复杂类型实现
