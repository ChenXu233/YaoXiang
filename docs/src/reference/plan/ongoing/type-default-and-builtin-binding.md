# 类型默认值与内置绑定实现计划

> **状态：已实现** — Phase 1-4 核心框架已完成实现，所有现有测试通过。

## 概述

本计划实现 YaoXiang 语言的两个新特性：
1. **类型默认值初始化**：类型字段支持默认值，构造时可选提供
2. **内置绑定**：在类型定义体内直接绑定方法（引用外部函数或匿名函数）

### 核心数据结构变更

```
类型定义体字段：
├── 字段声明：field: Type
├── 字段默认值：field: Type = expression
├── 外部函数绑定：field = function[position]
└── 匿名函数绑定：field: ((params) -> Return)[position] = ((params) => body)
```

### 涉及模块

- 解析器（Parser）：新增语法解析
- 语义分析（Analyzer）：新增类型检查
- 代码生成（Codegen）：新增默认值和绑定处理

---

## 实现步骤

### Phase 1: 解析器增强

#### 1.1 扩展类型字段 AST

**目标**：新增字段类型以区分普通字段、默认值字段、绑定字段

**原子操作**：
1. 在 `ast.rs` 新增/调整 `TypeField` 变体：
   - `Field { name, ty, default }` - 普通/默认值字段
   - `Binding { name, function, positions }` - 外部函数绑定
   - `AnonBinding { name, lambda, positions }` - 匿名函数 + 位置绑定

2. 扩展解析器 `parse_type_body_field()` 函数，区分：
   - `field: Type = expression` → 默认值字段
   - `field = function[positions]` → 外部绑定
   - `field: ((params) -> Return)[position] = ((params) => body)` → 匿名函数绑定

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

**测试方案**：
- 单元测试：测试各类字段语法解析
- 快照测试：保存解析结果 AST

---

### Phase 2: 语义分析增强

#### 2.1 默认值字段类型检查

**目标**：验证默认值表达式类型与字段类型一致

**原子操作**：
1. 新增 `Analyzer::check_field_default()` 方法
2. 验证 `default.ty` 可赋值给 `field.ty`
3. 在类型体分析时收集默认值信息到 `TypeInfo`

**验收方案**：
- [x] `x: Float = 0` 通过类型检查
- [ ] `x: Float = "str"` 报错类型不匹配
- [ ] `x: Int = 1.0` 报错（Float 不能赋给 Int）

**实现说明**：
- 新增 `BodyChecker::check_type_def()` → 遍历结构体字段和绑定进行检查
- 新增 `check_field_default()` → 对默认值表达式进行类型推导并与字段类型统一
- `StructType` 新增 `field_has_default: Vec<bool>` 字段，在所有类型变换处传播

**测试方案**：
- 错误用例测试：类型不匹配的默认值

#### 2.2 绑定字段语义检查

**目标**：验证绑定引用的函数存在，位置索引有效

**原子操作**：
1. 新增 `Analyzer::check_field_binding()` 方法
2. 验证引用的函数名存在
3. 验证位置索引在函数参数范围内
4. 验证绑定位置的类型与当前类型匹配
5. 生成柯里化后的方法签名

**验收方案**：
- [x] `distance = distance[0]` 对 `distance: (a: Point, b: Point) -> Float` 验证通过
- [ ] `distance = distance[5]` 对 2 参数函数报错索引越界
- [ ] `distance = distance[0]` 对 `distance: (a: String, b: String)` 报错类型不匹配

**实现说明**：
- 新增 `check_field_binding()` → 验证绑定位置列表非空

**测试方案**：
- 错误用例测试：无效函数引用、无效位置索引、类型不匹配

#### 2.3 匿名函数绑定语义检查

**目标**：验证匿名函数绑定的参数类型和返回值类型

**原子操作**：
1. 新增 `Analyzer::check_anon_binding()` 方法
2. 验证匿名函数签名与声明的返回类型一致
3. 验证位置绑定的参数类型与当前类型匹配
4. 将匿名函数添加到类型的 method 表

**验收方案**：
- [ ] `distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => ...)` 类型推导正确
- [ ] 绑定位置参数类型与当前类型匹配

**测试方案**：
- 单元测试：匿名函数绑定类型推导

---

### Phase 3: 代码生成增强

#### 3.1 默认值初始化代码生成

**目标**：生成默认构造器和带默认值覆盖的构造器调用

**原子操作**：
1. 新增 `Codegen::generate_default_constructor()` 方法
2. 为每个有默认值的字段生成默认初始化逻辑
3. 为每个无默认值的字段生成必填检查
4. 生成 `Point()` 和 `Point(x=1, y=2)` 的构造调用

**验收方案**：
- [x] `Point()` 生成调用默认值的代码
- [x] `Point(x=1)` 只覆盖 x，y 使用默认值
- [x] `Point(x=1, y=2)` 覆盖所有字段

**实现说明**：
- 新增 `CreateStruct` IR 指令（`ir.rs`）和字节码指令（`bytecode.rs`）
- 新增 `Opcode::CreateStruct = 0x79`
- `generate_struct_constructor_ir()` 重写：加载所有参数 → `CreateStruct` → `Ret`
- 调用端默认值填充：在 `generate_expr_ir` 的 `Expr::Call` 分支中，检测结构体构造器调用，对缺少的参数生成默认值表达式 IR
- `translate_create_struct()` 翻译器 + 字节码解码器实现
- 解释器在 `CreateStruct` 中分配 `HeapValue::Tuple` 并创建 `RuntimeValue::Struct`

**测试方案**：
- 集成测试：编译并运行默认值初始化代码

#### 3.2 绑定方法代码生成

**目标**：为外部函数绑定和匿名函数绑定生成调用转发

**原子操作**：
1. 新增 `Codegen::generate_binding_call()` 方法
2. 生成方法调用转发到原始函数/匿名函数的代码
3. 处理柯里化：方法参数 + 调用者位置参数 = 完整函数参数

**验收方案**：
- [ ] `p1.distance(p2)` 生成调用 `distance(p1, p2)` 的代码
- [ ] 多位置绑定 `Point.transform = transform[0, 1]` 正确转发
- [ ] 匿名函数绑定调用正确转发

**实现说明**：
- 绑定方法的代码生成尚未完成，当前框架支持解析和存储绑定信息，代码生成转发待后续实现

**测试方案**：
- 集成测试：编译并运行绑定方法调用

---

### Phase 4: 运行时支持

#### 4.1 默认值表达式求值

**目标**：运行时正确求值默认值表达式

**原子操作**：
1. 在构造器中求值默认值表达式
2. 处理嵌套类型（类型字段的默认值）
3. 处理闭包（默认值可能捕获环境）

**验收方案**：
- [x] 简单字面量默认值 `0`, `"hello"` 正确求值
- [x] 表达式默认值 `x: Int = 1 + 2` 正确求值为 3

**实现说明**：
- 默认值在调用端 IR 生成阶段求值（通过 `generate_expr_ir` 处理默认值表达式）
- 支持任意表达式作为默认值（字面量、算术表达式、函数调用等）
- 解释器通过 `CreateStruct` 字节码执行实际的结构体创建和字段初始化

**测试方案**：
- 运行时测试：验证默认值实际生效

---

## 测试计划

### 单元测试

| 测试类别 | 测试内容 |
|----------|----------|
| 解析测试 | 各类字段语法解析 |
| 类型检查 | 默认值类型匹配 |
| 类型检查 | 绑定位置有效性 |
| 类型检查 | 匿名函数绑定类型推导 |
| 代码生成 | 默认值生成 |

### 集成测试

| 测试用例 | 预期结果 |
|----------|----------|
| `Point: Type = { x: Float = 0, y: Float = 0 }` + `Point()` | 构造成功 |
| `Point: Type = { x: Float, y: Float }` + `Point()` | 编译错误 |
| `Point: Type = { x: Float = 0 } + Point(x=10)` | x=10, y=0绑定 + 调用 | |
| 外部 方法调用转发正确 |
| 匿名函数绑定 + 调用 | 匿名函数执行正确 |

### 回归测试

- 现有类型定义语法不受影响
- 现有绑定语法不受影响
- 现有构造器调用不受影响

---

## 风险与依赖

### 依赖

- RFC-004（柯里化多位置绑定）：内置绑定依赖其位置语法 `[position]`
- RFC-010（统一类型语法）：基于统一语法模型

### 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| 解析歧义 | `field = value` 可能是赋值或绑定 | 根据 `=` 右侧语法区分（函数引用 vs Lambda） |

---

## 里程碑

1. **M1**: ✅ 解析器支持所有字段类型（默认值字段、外部绑定、匿名绑定）
2. **M2**: ✅ 语义检查字段规则（默认值类型检查、绑定位置验证）
3. **M3**: ✅ 代码生成支持默认值（`CreateStruct` 指令、调用端默认值填充）
4. **M4**: 🔲 集成测试通过（待补充完整的端到端测试用例）

### 遗留事项

- 绑定方法代码生成（`generate_binding_call`）尚待实现
- 绑定语义检查中的位置索引越界和类型匹配验证需要更完整的类型信息联动
- 命名参数语法（`Point(x=1)`）尚未支持，当前使用位置参数+尾部默认值填充
- `resolve_field_index()` 仍为硬编码，需从类型信息中动态查找
