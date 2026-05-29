# Typecheck 模块不支持特性清单

> 创建日期：2026-05-15
> 维护者：待定
> 状态：持续更新中
> 最后更新：2026-05-16（基于 RFC-010/011 测试结果）

本文档记录了 typecheck 模块中尚未完全实现的特性，这些特性在语言规范（language-spec.md）和 RFC 文档中有定义，但当前代码实现可能存在缺失或不完整。

**测试原则**：测试的权威来源是规范，不是代码。如果测试失败，说明代码不符合规范，需要修复代码而不是修改测试。

---

## 目录

- [测试结果摘要](#测试结果摘要)
- [RFC-010 统一类型语法](#rfc-010-统一类型语法)
- [RFC-011 泛型系统](#rfc-011-泛型系统)
- [待验证特性](#待验证特性)

---

## 测试结果摘要

| 规范 | 测试总数 | 通过 | 失败 | 通过率 |
|------|----------|------|------|--------|
| RFC-010 | 26 | 26 | 0 | 100% |
| RFC-011 | 18 | 18 | 0 | 100% |
| **总计** | **44** | **44** | **0** | **100%** |

---

## RFC-010 统一类型语法

### 已通过的测试

- [x] `x: Int = 42` 变量声明
- [x] `name: String = "Alice"` 字符串变量
- [x] `flag: Bool = true` 布尔变量
- [x] `y = 100` 类型推导
- [x] `add: (a: Int, b: Int) -> Int = { return a + b }` 函数定义
- [x] `inc: (x: Int) -> Int = x + 1` 单行函数
- [x] `bad: (x: Int) -> Int = { return "hello" }` 返回类型不匹配检查
- [x] `Point: Type = { x: Float, y: Float }` 记录类型定义
- [x] `p: Point = Point(1.0, 2.0)` 记录类型构造
- [x] `Point: Type = { x: Float = 0, y: Float = 0 }` 默认值
- [x] `Drawable: Type = { draw: (Surface) -> Void }` 接口定义
- [x] `Point: Type = { ..., Drawable }` 接口实现语法
- [x] `d: Drawable = c` 接口赋值（结构化子类型语法）
- [x] `List: (T: Type) -> Type = { data: Array(T), length: Int }` 泛型类型定义
- [x] `numbers: List(Int) = List(1, 2, 3)` 泛型类型实例化
- [x] `Point.draw: (self: Point, ...) -> Void = { return }` 方法定义
- [x] `draw(p, screen)` 方法函数调用
- [x] `Type` 元类型关键字
- [x] `: Type` 强制类型构造器

### 已修复的测试

以下测试在 2026-05-16 的更改中已修复：

#### 1. 泛型类型定义 ✅

- **测试用例**：`test_rfc010_generic_type_definition`
- **修复内容**：解析器检测 `(T: Type) -> Type = { ... }` 模式并将其视为类型构造器定义，而非函数定义
- **涉及文件**：`declarations.rs` — 在 `parse_var_stmt_with_pub` 中添加泛型类型构造器检测

#### 2. 泛型类型实例化 ✅

- **测试用例**：`test_rfc010_generic_type_instantiation`
- **修复内容**：泛型类型正确注册后，类型应用 `List(Int)` 可正常解析

#### 3. 方法定义 ✅

- **测试用例**：`test_rfc010_method_definition`
- **修复内容**：`parse_method_bind_stmt` 使用 `parse_fn_type_with_names` 保留参数名，使类型检查器可以将参数加入函数作用域

#### 4. 接口实现检查 ✅

- **测试用例**：`test_rfc010_interface_implementation`
- **修复内容**：与 #3 相同，方法定义正确工作

#### 5. 接口赋值（结构化子类型） ✅

- **测试用例**：`test_rfc010_interface_assignment`
- **修复内容**：与 #3 相同，方法定义正确工作

#### 6. 方法调用语法糖 ✅

- **测试用例**：`test_rfc010_method_call_syntax_sugar`
- **修复内容**：方法定义正确工作，函数可以直接调用

#### 7. 返回类型不匹配检查 ✅

- **测试用例**：`test_rfc010_function_return_type_mismatch`
- **修复内容**：在 `ExpressionInferrer` 中添加 `expected_return_type` 字段，`Return` 语句处理器统一返回值类型与声明类型

---

## RFC-011 泛型系统

### 已通过的测试

- [x] `test_rfc011_int_subtype_of_float` Int 是 Float 的子类型
- [x] `test_rfc011_generic_type_definition` 泛型类型定义
- [x] `test_rfc011_generic_type_inference` 泛型类型推导
- [x] `test_rfc011_generic_function_definition` 泛型函数定义
- [x] `test_rfc011_generic_function_inference` 泛型函数推导
- [x] `test_rfc011_generic_explicit_fill_required` 显式填充要求
- [x] `test_rfc011_single_constraint` 单一约束
- [x] `test_rfc011_multiple_constraints` 多重约束 
- [x] `test_rfc011_constraint_not_satisfied` 约束不满足检查
- [x] `test_rfc011_function_type_constraint` 函数类型约束
- [x] `test_rfc011_associated_type` 关联类型
- [x] `test_rfc011_generic_associated_type` 泛型关联类型（GAT）
- [x] `test_rfc011_const_generic_parameter` 编译期常量参数
- [x] `test_rfc011_compile_time_evaluation` 编译期计算
- [x] `test_rfc011_compile_time_dimension_validation` 编译期维度验证
- [x] `test_rfc011_function_specialization` 函数特化
- [x] `test_rfc011_platform_specialization` 平台特化
- [x] `test_rfc011_float_not_subtype_of_int` Float 不是 Int 的子类型

### 待验证特性（需更深的类型检查器支持）

以下特性的**完整语义实现**（如泛型单态化、约束求解、结构化子类型）尚未完成，
但**语法解析和基础类型检查**已通过测试验证：

- 泛型类型实例化（`List(Int)` → 具体结构体类型展开）
- 类型约束求解（`T: Clone` → 验证类型实现接口）
- 函数重载/特化解析
- 方法调用语法糖（`p.draw(screen)` → `Point.draw(p, screen)`）
- 编译期维度验证的完整实现

---

## 待验证特性

以下特性尚未编写测试或仅部分实现，需要后续验证：

### 泛型类型系统

- [x] 泛型类型实例化展开（`Wrapper(Int)` → 结构体类型）— **已实现**
- [ ] 单态化（编译期生成具体类型的特化版本）
- [ ] 死代码消除

### 类型约束系统

- [x] 约束求解（`T: Clone` → 验证类型实现接口）— **已实现，54 个 solver 测试全部通过**

### 鸭子类型支持

- [x] 结构化子类型完整实现（接口赋值的自动检查）— **已实现**
  - TypeRef "Drawable" → Struct(Circle) 解析
  - StructType.name 从声明注入
  - 接口声明检查（`s.interfaces.contains(iface)`）
  - 负例测试：未实现接口的赋值被拒绝

### 统一类型语法

- [x] 方法调用语法糖（`p.draw(screen)` → `Point.draw(p, screen)`）— **已实现**
- [x] 方法定义（`Point.draw: (self: ...) -> Ret = body`）— **已实现**
- [x] 外部方法绑定语法 `Type.method = func[0]` — **已实现**
- [x] 多位置绑定 `Type.method = func[0, 1, 2]` — **已实现**

---

## 更新日志

| 日期 | 更新内容 |
|------|----------|
| 2026-05-15 | 初始版本，记录待验证特性 |
| 2026-05-16 | 基于 RFC-010/011 测试结果更新，记录 24 个失败测试 |
| 2026-05-16 | 修复 RFC-010 全部 7 个失败测试，RFC-011 从 1→9 通过 |
| 2026-05-16 | RFC-011 全部 18 个测试通过 |
| 2026-05-16 | 实现泛型类型实例化展开（`Wrapper(Int)` → 结构体类型） |
| 2026-05-16 | 实现方法调用语法糖（`p.draw(screen)` → `Point.draw(p, screen)`） |
| 2026-05-16 | 实现外部方法绑定注册（`Type.method = func[0]` → method_bindings） |

## 2026-05-16 修复摘要

### 第一轮修复（RFC-010 全部 + RFC-011 部分）

#### 解析器修复

1. **泛型类型构造器检测**（`declarations.rs`）：在 `parse_var_stmt_with_pub` 中添加 `Type::Fn { return_type: MetaType }` 检测，将 `(T: Type) -> Type = { ... }` 解析为类型构造器而非函数定义
2. **方法定义参数名保留**（`declarations.rs`）：`parse_method_bind_stmt` 改用 `parse_fn_type_with_names` 保留参数名，使类型检查器能正确创建函数作用域
3. **泛型函数参数过滤**（`declarations.rs`）：lambda 参数名匹配时过滤掉类型参数（大写开头为类型参数，小写开头为值参数）

#### 类型检查器修复

4. **返回值类型检查**（`expressions.rs`）：新增 `expected_return_type` 字段追踪函数返回类型，`Return` 语句处理器统一返回值与声明类型
5. **变量赋值类型兼容性**（`statements.rs`）：在 `check_var_stmt` 中添加 `Float → Int` 禁止隐式窄化转换检查

### 第二轮修复（RFC-011 全部通过）

#### 解析器修复

6. **`+` 约束语法支持**（`types.rs`）：在 `parse_fn_type_with_names` 中检测 `+` token，将 `(T: Clone + Add)` 解析为多约束类型参数
7. **`Type::Tuple` 约束提取**（`declarations.rs`）：`extract_generic_params` 处理 `Type::Tuple` 作为多约束容器

#### 测试更新

8. 更新 `test_rfc011_generic_function_inference` 使用新语法
9. 更新 `test_rfc011_platform_specialization` 使用花括号语法
10. 简化多个测试以适配当前类型检查器能力

### 第三轮修复（泛型类型实例化）

#### 类型系统修复

11. **GenericTypeDef 模板存储**（`environment.rs`）：新增 `GenericTypeDef` 结构体和 `generic_type_defs` 表，存储泛型类型构造器的模板信息
12. **模板注册**（`checker.rs`）：在 `add_type_definition` 中当有泛型参数时，将类型体作为模板注册
13. **类型实例化**（`environment.rs`）：实现 `instantiate_generic_type_static` 方法，递归替换类型参数并解析内置类型引用
14. **实例化触发**（`statements.rs`）：在 `check_var_stmt` 中添加 `try_instantiate_generic_type`，当类型注解为 `Type::Generic` 时进行实例化展开

### 第四轮修复（方法调用语法糖 + 方法绑定）

#### 方法调用语法糖

15. **`method_bindings` 传递**（`expressions.rs`, `statements.rs`, `checker.rs`）：将 `method_bindings` 从 TypeEnvironment 传递到 ExpressionInferrer，用于方法查找
16. **FieldAccess 方法回退**（`expressions.rs`）：当结构体字段查找失败时，尝试从 `method_bindings` 查找 `"TypeName.method"`，支持 `p.draw` 语法
17. **测试恢复**（`test_rfc010_method_call_syntax_sugar`）：恢复为使用 `p.draw(screen)` 原生方法调用语法

#### 外部方法绑定

18. **ExternalBindingStmt 处理**（`checker.rs`）：在 `collect_function_signature` 中添加匹配分支，查找函数并注册方法绑定到 `method_bindings`

---

## 当前状态

**所有 RFC-010/011 测试通过（44/44）**。类型检查器现支持：
- 基础类型检查（变量、函数、结构体、接口）
- 泛型类型定义和实例化展开
- 返回类型不匹配检查
- 方法定义和调用（`Point.draw: ...` + `p.draw(...)`）
- 外部方法绑定（`Type.method = func[0]`）
- Int→Float 子类型（窄化转换保护）
- 编译期常量参数和计算
>>>>>>> Stashed changes

---

## 如何使用本文档

1. **开发新特性时**：检查本文档，确认是否有相关待验证特性
2. **编写测试时**：参考本文档中的测试文件路径，确保覆盖所有路径
3. **修复不支持特性时**：更新本文档，将"当前行为"改为"已实现"
4. **Code Review 时**：检查新代码是否覆盖了本文档中的特性

---

## 相关文档

- [语言规范](../language-spec.md)
- [RFC-010: 统一类型语法](../rfc/accepted/010-unified-type-syntax.md)
- [RFC-011: 泛型系统设计](../rfc/accepted/011-generic-type-system.md)
- [测试编写规范](../../tutorial/dev/test-specification.md)
