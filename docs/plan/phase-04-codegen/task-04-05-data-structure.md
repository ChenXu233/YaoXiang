# Task 4.5: 数据结构字节码

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

生成数据结构操作（列表、字典、元组、结构体）的字节码。

## 字节码指令

| Opcode | 操作 | 说明 |
|--------|------|------|
| `NEW_LIST` | 创建列表 | |
| `NEW_DICT` | 创建字典 | |
| `NEW_TUPLE` | 创建元组 | |
| `NEW_STRUCT` | 创建结构体 | |
| `LIST_PUSH` | 列表追加 | |
| `LIST_INDEX` | 列表访问 | |
| `DICT_INSERT` | 字典插入 | |
| `DICT_LOOKUP` | 字典查找 | |
| `FIELD_ACCESS` | 字段访问 | |
| `FIELD_UPDATE` | 字段更新 | |

## 字节码格式

```rust
struct NewList { size: usize, result: Reg }
struct ListIndex { list: Reg, index: Reg, result: Reg }
struct FieldAccess { struct_reg: Reg, field: String, result: Reg }
struct DictLookup { dict: Reg, key: Reg, result: Reg }
```

## 生成规则

### 列表创建和访问
```yaoxiang
nums = [1, 2, 3]
first = nums[0]
```
生成字节码：
```
NEW_LIST size=3 -> r1
CONST 1 -> r2
LIST_PUSH r1, r2
CONST 2 -> r3
LIST_PUSH r1, r3
CONST 3 -> r4
LIST_PUSH r1, r4
CONST 0 -> r5
LIST_INDEX r1, r5 -> r6
STORE r6 -> first
```

### 结构体创建
```yaoxiang
Point = struct { x: Int, y: Int }
p = Point(x: 1, y: 2)
x = p.x
```
生成字节码：
```
NEW_STRUCT Point -> r1
CONST 1 -> r2
FIELD_UPDATE r1, "x", r2
CONST 2 -> r3
FIELD_UPDATE r1, "y", r3
STORE r1 -> p
FIELD_ACCESS r1, "x" -> r4
STORE r4 -> x
```

## 验收测试

```yaoxiang
# test_data_structure_bytecode.yx

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

# 结构体
Point = struct { x: Int, y: Int }
p = Point(x: 1, y: 2)
assert(p.x == 1)
assert(p.y == 2)

print("Data structure bytecode tests passed!")
```

## 相关文件

- **bytecode.rs**: 数据结构指令定义
- **generator.rs**: 数据结构生成逻辑
