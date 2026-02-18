# 类型默认值与内置绑定实现计划

> **状态：核心已实现** — Phase 1-4 核心框架 + 遗留项增强已完成，所有 1499 测试通过。

## 概述

本计划实现 YaoXiang 语言的两个核心特性，对应 RFC-004（柯里化多位置绑定）和 RFC-010（统一类型语法）：

1. **类型默认值初始化**（RFC-010）：类型字段支持默认值，构造时可选提供
2. **内置绑定**（RFC-004）：在类型定义体内直接绑定方法（引用外部函数或匿名函数），支持位置索引精确控制参数绑定

### 核心语法（RFC-010 统一模型）

```yaoxiang
Point: Type = {
    x: Float = 0,                                    # 字段 + 默认值
    y: Float = 0,                                    # 字段 + 默认值
    distance = distance[0],                          # 外部函数绑定（RFC-004 位置语法）
    norm: ((p: Point) -> Float)[0] = ((p) => ...)    # 匿名函数绑定
}
```

### 核心数据结构变更

```
类型定义体字段：
├── 字段声明：field: Type
├── 字段默认值：field: Type = expression          (RFC-010)
├── 外部函数绑定：field = function[position]      (RFC-004)
└── 匿名函数绑定：field: ((params) -> Return)[position] = ((params) => body)  (RFC-004)
```

### 涉及模块

| 模块 | 文件 | 职责 |
|------|------|------|
| AST | `ast.rs` | `StructField`, `TypeBodyBinding`, `BindingKind` |
| 解析器 | `declarations.rs` | `parse_struct_type()` - 四种字段类型解析 |
| 类型系统 | `mono.rs` | `StructType.field_has_default` |
| 语义分析 | `checking/mod.rs` | `check_type_def`, `check_field_default`, `check_field_binding` |
| IR 生成 | `ir_gen.rs` | `CreateStruct`, 默认值填充, 绑定方法调用转发 |
| 字节码 | `bytecode.rs` | `CreateStruct` 字节码编解码 |
| 运行时 | `executor.rs` | `CreateStruct` 执行 |

---

## 实现步骤

### Phase 1: 解析器增强 ✅

#### 1.1 扩展类型字段 AST

**目标**：新增字段类型以区分普通字段、默认值字段、绑定字段

**验收方案**：
- [x] 解析 `Point: Type = { x: Float = 0 }` 生成正确的 AST
- [x] 解析 `Point: Type = { distance = distance[0] }` 生成 Binding 节点
- [x] 解析 `Point: Type = { distance: ((a, b) => Float)[0] = ((a, b) => a + b) }` 生成 AnonBinding 节点

**实现说明**：
- `StructField` 新增 `default: Option<Box<Expr>>` 字段
- 新增 `TypeBodyBinding` 结构体和 `BindingKind` 枚举（`External` / `Anonymous`）
- `Type::Struct` 从元组变体改为结构体变体 `Type::Struct { fields, bindings }`
- `parse_struct_type()` 完全重写，支持四种字段类型解析
- 新增辅助函数：`parse_optional_binding_positions()`, `parse_binding_positions()`, `extract_fn_type_info()`

---

### Phase 2: 语义分析增强 ✅

#### 2.1 默认值字段类型检查

**目标**：验证默认值表达式类型与字段类型一致

**验收方案**：
- [x] `x: Float = 0` 通过类型检查（支持 Int → Float 隐式数值提升）
- [x] `x: Float = "str"` 报错类型不匹配（String ≠ Float）
- [x] `x: Int = 1.0` 报错（Float 不能赋给 Int，不支持反向提升）

**实现说明**：
- `BodyChecker::check_type_def()` → 遍历结构体字段和绑定进行检查
- `check_field_default()` → 对默认值表达式进行类型推导并与字段类型统一
  - 支持 Int → Float 隐式数值提升（符合 RFC-010 `x: Float = 0` 用法）
  - 其他类型不匹配（如 String → Float, Float → Int）报 `type_mismatch` 错误
- `StructType` 新增 `field_has_default: Vec<bool>` 字段，在所有类型变换处传播

#### 2.2 绑定字段语义检查（RFC-004 类型安全）

**目标**：验证绑定引用的函数存在，位置索引有效，类型匹配

**验收方案**：
- [x] `distance = distance[0]` 对 `distance: (a: Point, b: Point) -> Float` 验证通过
- [x] `distance = distance[5]` 对 2 参数函数报错索引越界
- [x] `distance = distance[0]` 对 `distance: (a: String, b: String)` 报错类型不匹配
- [x] 位置索引列表非空验证

**实现说明**：
- `check_field_binding(type_name, binding, span)` 接收类型名用于验证
- 外部绑定验证流程（RFC-004 §类型检查规则）：
  1. 位置索引列表非空
  2. 查找函数的多态类型，实例化为单态函数类型
  3. 验证每个位置索引 < 函数参数数量
  4. 验证绑定位置的参数类型与当前类型兼容（通过 `unify`）
- 函数未在当前作用域中找到时跳过深度检查（函数可能在外层或后续定义）

#### 2.3 匿名函数绑定语义检查

**目标**：验证匿名函数绑定的参数位置和类型

**验收方案**：
- [x] 位置索引非空验证
- [x] 位置索引在匿名函数参数范围内
- [x] 绑定位置参数类型与当前类型匹配

**实现说明**：
- 匿名绑定验证：位置非空 + 位置 < 参数数 + 绑定位置参数类型与类型名 `unify`

---

### Phase 3: 代码生成增强 ✅

#### 3.1 默认值初始化代码生成

**目标**：生成默认构造器和带默认值覆盖的构造器调用

**验收方案**：
- [x] `Point()` 生成调用默认值的代码
- [x] `Point(1.0)` 只覆盖 x，y 使用默认值
- [x] `Point(1.0, 2.0)` 覆盖所有字段

**实现说明**：
- 新增 `CreateStruct` IR 指令（`ir.rs`）和字节码指令（`bytecode.rs`）
- 新增 `Opcode::CreateStruct = 0x79`
- `generate_struct_constructor_ir()` 重写：加载所有参数 → `CreateStruct` → `Ret`
- 调用端默认值填充：在 `generate_expr_ir` 的 `Expr::Call` 分支中，检测结构体构造器调用，对缺少的参数生成默认值表达式 IR
- `translate_create_struct()` 翻译器 + 字节码解码器实现
- 解释器在 `CreateStruct` 中分配 `HeapValue::Tuple` 并创建 `RuntimeValue::Struct`

#### 3.2 绑定方法代码生成（RFC-004 参数重排）

**目标**：为绑定方法调用生成正确的函数调用转发

**验收方案**：
- [x] `p1.distance(p2)` + `distance = distance[0]` → 生成 `distance(p1, p2)` 调用
- [x] `p1.distance(p2)` + `distance = distance[1]` → 生成 `distance(p2, p1)` 调用
- [x] 多位置绑定 `transform = transform[0, 1]` 正确转发

**实现说明**：
- 新增 `BindingInfo` 结构体（记录原始函数名 + 绑定位置列表）
- 新增 `type_bindings: HashMap<String, HashMap<String, BindingInfo>>`（类型名 → 方法名 → 绑定信息）
- `register_type_bindings()` 在构造函数 IR 生成阶段从 `Type::Struct { bindings }` 提取绑定信息
- 方法调用 IR 生成增强（`Expr::Call` + `FieldAccess` 分支）：
  1. 推导对象类型名（通过 `get_expr_struct_type_name()`）
  2. 查找该类型的绑定信息
  3. 如有绑定：按 RFC-004 位置规则重排参数
     - 创建 `total_params = positions.len() + method_args.len()` 大小的参数槽
     - 将 obj 放入绑定位置
     - 将方法参数按顺序填充剩余位置
  4. 调用原始函数名（非方法名）
- 匿名函数绑定使用 `类型名.__anon_方法名` 命名约定

#### 3.3 字段索引动态解析

**目标**：`resolve_field_index()` 从类型信息动态查找字段索引

**验收方案**：
- [x] 不再依赖硬编码（原 x→0, y→1, z→2）
- [x] 从 `struct_definitions` 按字段名精确查找

**实现说明**：
- `resolve_field_index(expr, field_name)` 重写：
  1. 通过 `get_expr_struct_type_name(expr)` 推导表达式的结构体类型名
  2. 从 `struct_definitions` 查找该类型的字段列表，匹配字段名返回索引
  3. 兜底：遍历所有结构体定义查找字段名（当类型推导不可用时）
- 新增 `get_expr_struct_type_name(expr)` 辅助方法：
  - 变量：从 `type_result.local_var_types`、`bindings`、`local_var_types` 查找
  - 构造器调用：`Point(...)` 直接返回 `"Point"`
- 新增 `mono_type_to_struct_name(mono_type)` 辅助方法：
  - `MonoType::TypeRef(name)` → `Some(name)`
  - `MonoType::Struct(st)` → `Some(st.name)`

---

### Phase 4: 运行时支持 ✅

#### 4.1 默认值表达式求值

**目标**：运行时正确求值默认值表达式

**验收方案**：
- [x] 简单字面量默认值 `0`, `"hello"` 正确求值
- [x] 表达式默认值 `x: Int = 1 + 2` 正确求值为 3

**实现说明**：
- 默认值在调用端 IR 生成阶段求值（通过 `generate_expr_ir` 处理默认值表达式）
- 支持任意表达式作为默认值（字面量、算术表达式、函数调用等）
- 解释器通过 `CreateStruct` 字节码执行实际的结构体创建和字段初始化

---

## 测试计划

### 单元测试

| 测试类别 | 测试内容 | 状态 |
|----------|----------|------|
| 解析测试 | 各类字段语法解析 | ✅ |
| 类型检查 | 默认值类型匹配（含 Int→Float 提升） | ✅ |
| 类型检查 | 绑定位置有效性（越界、类型匹配） | ✅ |
| 类型检查 | 匿名函数绑定位置验证 | ✅ |
| 代码生成 | 默认值生成 | ✅ |
| 代码生成 | 绑定方法调用转发 | ✅ |

### 集成测试

| 测试用例 | 预期结果 | 状态 |
|----------|----------|------|
| `Point: Type = { x: Float = 0, y: Float = 0 }` + `Point()` | 构造成功 | ✅ |
| `Point(1.0, 2.0)` 位置参数构造 | 字段正确赋值 | ✅ |
| `Point(1.0)` 部分参数 + 默认值 | x=1.0, y=0 | ✅ |
| 绑定方法调用（RFC-004 位置重排） | 参数正确转发 | ✅ |

### 回归测试

- [x] 所有 1464 lib 测试通过
- [x] 所有 30 integration 测试通过
- [x] 所有 5 runtime 测试通过
- 现有类型定义语法不受影响
- 现有绑定语法不受影响
- 现有构造器调用不受影响

---

## 风险与依赖

### 依赖

- **RFC-004**（柯里化多位置绑定）：`[position]` 位置绑定语法，参数重排规则
- **RFC-010**（统一类型语法）：`name: Type = { ... }` 统一模型，字段默认值语法

### 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| 解析歧义 | `field = value` 可能是赋值或绑定 | 根据 `=` 右侧语法区分（函数引用+位置 vs Lambda） |
| 类型推导不完整 | `resolve_field_index` 兜底遍历可能不精确 | 优先使用类型检查结果 |

---

## 里程碑

1. **M1**: ✅ 解析器支持所有字段类型（默认值字段、外部绑定、匿名绑定）
2. **M2**: ✅ 语义检查字段规则（默认值类型检查+数值提升、绑定位置越界+类型匹配验证）
3. **M3**: ✅ 代码生成支持默认值（`CreateStruct` 指令、调用端默认值填充）
4. **M4**: ✅ 绑定方法代码生成（RFC-004 参数重排、`BindingInfo` + `type_bindings` 映射）
5. **M5**: ✅ 字段索引动态解析（`resolve_field_index` 从 `struct_definitions` 查找）

### 后续可选增强

以下为非核心功能，可在后续版本中按需实现：

| 功能 | 对应 RFC | 说明 |
|------|---------|------|
| 命名参数构造 | RFC-010 | `Point(x=1, y=2)` 语法需要解析器支持命名参数 |
| 负数索引绑定 | RFC-004 | `func[-1]` 绑定到最后一个参数 |
| 范围绑定 | RFC-004 | `func[0..2]` 绑定到多个连续位置 |
| 默认绑定 | RFC-004 | `Type.method = function`（无位置）自动查找第一个类型匹配位置 |
| 外部绑定语句 | RFC-004 | `Point.distance = distance[0]` 独立绑定语句 |
| 接口约束 | RFC-010 | 类型体内的接口名作为约束 |
| 匿名函数 IR 生成 | RFC-004 | 为匿名绑定生成独立的函数 IR |
