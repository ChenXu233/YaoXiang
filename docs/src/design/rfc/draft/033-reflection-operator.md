---
title: "RFC-033: `^^` 反射运算符"
status: "草案"
author: "晨煦"
created: "2026-06-16"
updated: "2026-06-16"
---

# RFC-033: `^^` 反射运算符

> **参考**:
>
> - [RFC-010: 统一类型语法 - name: type = value 模型](../accepted/010-unified-type-syntax.md)
> - [RFC-011: 泛型系统设计](../accepted/011-generic-type-system.md)
> - [RFC-027: 编译期谓词与统一静态验证](../accepted/027-compile-time-evaluation-types.md)
> - [RFC-011a: 接口实现与动态分发](../draft/011a-interface-implementation.md)

## 摘要

本文提出引入 `^^` 运算符作为反射入口，用于获取类型和值的元数据。`^^T` 返回类型 `T` 的静态元数据对象，`^^obj` 返回值 `obj` 的动态类型元数据。元数据对象是普通记录类型，包含名称、参数、字段等信息，可在编译期和运行时使用。

## 动机

### 为什么需要这个特性？

1. **序列化/反序列化**：需要访问类型的字段信息，自动生成序列化代码
2. **编译期元编程**：需要在编译期访问类型结构，生成代码或验证约束
3. **运行时调试/工具**：需要在运行时打印类型信息，辅助调试
4. **运行时类型检查**：需要在运行时判断类型关系，如 "obj 是什么类型？"

### 当前的问题

当前 YaoXiang 没有反射机制，无法在编译期或运行时访问类型的元数据。如果直接使用 `.name`、`.fields` 访问类型元数据，会与用户定义的字段冲突：

```yaoxiang
Person: Type = { name: String, age: Int }

# 如果 Person.name 是类型元数据的名字，还是字段 name？
# 这会导致解析困难和语义混乱
```

需要一种**不侵入普通字段命名空间**的语法来访问类型元数据。

## 提案

### 核心设计

引入 `^^` 运算符作为反射入口，明确区分普通代码和元数据查询。

**两种用法**：

1. **静态反射（作用于类型）**：`^^T` 返回类型 `T` 的静态元数据对象
2. **动态反射（作用于值）**：`^^obj` 返回值 `obj` 的动态类型元数据

**元数据结构**：

```yaoxiang
TypeMeta: Type = {
    name: String,
    params: Array(ParamMeta),
    fields: Array(FieldMeta),
    return_type: Type,
    refinement: Option(Expr)  # 编译期 Some(Expr)，运行时 None
}

ParamMeta: Type = {
    name: String,
    type: Type
}

FieldMeta: Type = {
    name: String,
    type: Type
}
```

**宇宙层级**：若 `T: Type_n`，则 `^^T: Type_{n+1}`，符合类型论标准宇宙提升规则。

**优先级**：`^^` 是一元前缀运算符，优先级最高。`^^T.name` 等价于 `(^^T).name`。

### 示例

#### 基本使用

```yaoxiang
Point: Type = { x: Float, y: Float }

# 静态反射
meta = ^^Point
print(meta.name)           # "Point"
print(meta.fields.len)     # 2
print(meta.fields[0].name) # "x"
print(fields[0].type)      # Float

# 动态反射（需要启用运行时反射）
obj = Point(1.0, 2.0)
meta = ^^obj
print(meta.name)           # "Point"
```

#### 泛型类型

```yaoxiang
List: (T: Type) -> Type = { data: Array(T), length: Int }

# 反射泛型类型本身
meta = ^^List
print(meta.name)           # "List"
print(meta.params)         # [{ name: "T", type: Type }]

# 反射具体实例化类型
meta = ^^List(Int)
print(meta.name)           # "List(Int)"
print(meta.params)         # []
```

#### 函数

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

meta = ^^add
print(meta.name)           # "add"
print(meta.params)         # [{ name: "a", type: Int }, { name: "b", type: Int }]
print(meta.return_type)    # Int
```

#### 精化类型

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

# 编译期：refinement 为 Some(Expr)
meta = ^^Positive
print(meta.name)           # "Positive"
print(meta.refinement)     # Some(AST(x > 0))

# 运行时：refinement 为 None（擦除）
```

#### 编译期谓词中使用

```yaoxiang
# 检查类型是否有字段
HasFields: (T: Type) -> Type = { ^^T.fields.len > 0 }

# 检查字段类型
HasFloatField: (T: Type) -> Type = {
    exists field in ^^T.fields: field.type == Float
}

# 使用
obj: HasFields(Point) = Point(1.0, 2.0)  # ✅ 验证通过
# obj: HasFields(Int) = 42  # ❌ 验证失败
```

#### 序列化示例

```yaoxiang
# 编译期纯函数：生成 JSON 字符串
to_json: (T: Type) -> ((obj: T) -> String) = {
    meta = ^^T
    parts: Array(String) = []
    for field in meta.fields {
        # 编译期生成字段访问代码
        parts.push("\"${field.name}\": ${obj.${field.name}}")
    }
    return "{" + parts.join(", ") + "}"
}

# 使用
point_to_json = to_json(Point)
print(point_to_json(Point(1.0, 2.0)))  # '{"x": 1.0, "y": 2.0}'
```

### 语法变化

| 之前 | 之后 |
|------|------|
| 无反射机制 | `^^T` 获取类型元数据 |
| 无反射机制 | `^^obj` 获取值的动态类型元数据 |

## 详细设计

### 类型系统影响

- **新增类型**：`TypeMeta`、`ParamMeta`、`FieldMeta`
- **宇宙层级**：`^^T` 返回的类型比 `T` 高一级
- **泛型交互**：`^^List` 和 `^^List(Int)` 都支持
- **函数交互**：`^^add` 返回函数的元数据（包含参数和返回类型）
- **精化类型交互**：`^^Positive` 返回精化类型的元数据（包含精化表达式）

### 运行时行为

**编译期反射**：
- `^^T` 在编译期完全求值，结果内联为常量
- 精化表达式在编译期可用

**运行时反射**：
- 默认关闭，零开销
- 通过 `--enable-runtime-reflection` 编译选项启用
- 启用后，`^^obj` 返回动态类型元数据
- 精化表达式在运行时擦除为 `None`

**按需生成 + treeshake**：
- 只有实际使用 `^^` 的类型才生成元数据
- 未被引用的类型不生成元数据（treeshake）

### 编译器改动

1. **词法分析器**：识别 `^^` 为单个 token
2. **语法分析器**：添加 `^^` 前缀表达式规则
3. **类型系统**：添加 `TypeMeta`、`ParamMeta`、`FieldMeta` 类型定义
4. **类型检查器**：为每个类型生成元数据实例
5. **编译期求值器**：支持 `^^T` 的编译期求值
6. **运行时（可选）**：为被反射的类型生成 RTTI

### 向后兼容性

- ✅ 现有语法无影响：`^^` 是新运算符，不与现有语法冲突
- ✅ 现有类型无影响：所有类型自动支持 `^^`
- ✅ 现有函数无影响：函数可以使用 `^^` 但不强制
- ✅ 编译期谓词无影响：`^^T` 在谓词中与普通内容一致
- ✅ 运行时无影响：运行时反射默认关闭，零开销

## 权衡

### 优点

- **统一性**：函数、泛型、精化类型统一处理
- **零开销**：编译期反射完全擦除，运行时反射可选
- **与现有系统集成**：与编译期谓词（RFC-027）无缝集成
- **简洁**：`^^` 是纯符号，不与用户定义的标识符冲突
- **按需生成**：treeshake 优化，未使用的类型零开销

### 缺点

- **学习曲线**：需要理解 `^^` 的语义和元数据结构
- **运行时开销**：启用运行时反射会增加内存开销（每个实例一个指针）
- **实现复杂度**：需要修改编译器多个组件

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| `reflect(T)` 函数 | 会引入额外标识符到作用域，可能被用户遮蔽 |
| `type_info(T)` 函数 | 同上 |
| 单个 `^` 运算符 | 可能与位运算冲突，且 C++26 就是因为冲突才选的 `^^` |
| `@@`、`##` 等符号 | 没有先例，不如 `^^` 易于解释 |

## 实现阶段

| 阶段 | 内容 | 依赖 |
|------|------|------|
| Phase 1 | 编译期 `^^` 运算符解析 | 无 |
| Phase 2 | `TypeMeta` 数据结构定义 | Phase 1 |
| Phase 3 | 编译期元数据生成 | Phase 2 |
| Phase 4 | 运行时反射支持（可选） | Phase 3 |
| Phase 5 | 编译期谓词集成 | Phase 3 |

### 依赖关系

```
Phase 1 (解析)
    ↓
Phase 2 (数据结构)
    ↓
Phase 3 (编译期元数据)
    ↓
    ├────────────┐
    ↓            ↓
Phase 4        Phase 5
(运行时反射)  (编译期谓词)
```

### 风险

- **解析冲突**：`^^` 可能与现有语法冲突（经分析无冲突）
- **性能影响**：编译期元数据生成可能增加编译时间（可通过 treeshake 优化）
- **运行时开销**：启用运行时反射会增加内存开销（按需生成缓解）

## 开放问题

- [x] `^^` 作用范围：仅作用于类型和值，不作用于表达式
- [x] 链式访问：支持，`^^T` 返回的元数据对象可以正常访问属性
- [x] 模式匹配：支持，`TypeMeta` 是普通记录类型，可以正常模式匹配
- [x] 比较：支持，同类型的元数据对象相等
- [x] 内存开销：按需生成 + treeshake 优化

---

## 附录

### 附录A：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| `^^` 作用范围 | 仅作用于类型和值，不作用于表达式 | 2026-06-16 | 晨煦 |
| 链式访问 | 支持 | 2026-06-16 | 晨煦 |
| 模式匹配 | 支持 | 2026-06-16 | 晨煦 |
| 比较 | 支持，同类型元数据相等 | 2026-06-16 | 晨煦 |
| 内存开销 | 按需生成 + treeshake | 2026-06-16 | 晨煦 |
| 泛型交互 | `^^List` 和 `^^List(Int)` 都支持 | 2026-06-16 | 晨煦 |
| 精化表达式存储 | 编译期可用，运行时擦除为 None | 2026-06-16 | 晨煦 |

### 附录B：术语表

| 术语 | 定义 |
|------|------|
| 反射 | 在运行时或编译时访问类型元数据的能力 |
| 元数据 | 描述类型结构的信息（名称、字段、参数等） |
| RTTI | Run-Time Type Information，运行时类型信息 |
| treeshake | 编译器优化，移除未使用的代码 |
| 精化类型 | 带有约束条件的类型，如 `Positive: (x: Int) -> Type = { x > 0 }` |

---

## 参考文献

- [RFC-010: 统一类型语法](../accepted/010-unified-type-syntax.md)
- [RFC-011: 泛型系统设计](../accepted/011-generic-type-system.md)
- [RFC-027: 编译期谓词与统一静态验证](../accepted/027-compile-time-evaluation-types.md)
- [RFC-011a: 接口实现与动态分发](../draft/011a-interface-implementation.md)
- [C++26 反射提案](https://wg21.link/P2996)

---

## 生命周期与归宿

```
┌─────────────┐
│   草案      │  ← 当前状态
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 开放社区讨论和反馈
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```
