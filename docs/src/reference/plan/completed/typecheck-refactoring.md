# 类型检查流程完全重构计划

> **状态**：✅ 已完成  
> **完成日期**：2025-07  
> **测试结果**：1469 个测试全部通过（1434 + 30 + 5），0 失败

## 核心目标

彻底消除技术债，让类型检查流程清晰、简洁、易扩展，同时保持与现有特性模块的良好配合。

---

## 现有模块结构分析（重构前）

> 以下为重构前的结构，仅作参考。

```
src/frontend/typecheck/
├── mod.rs                      # 统一入口
├── checking/                   # ❌ 问题：与 inference 职责重叠
│   ├── mod.rs                 # BodyChecker, AssignmentChecker, SubtypeChecker...
│   ├── assignment.rs
│   ├── bounds.rs
│   ├── compatibility.rs
│   └── subtyping.rs
├── inference/                  # ❌ 问题：与 checking 职责重叠
│   ├── mod.rs
│   ├── expressions.rs          # ExprInferrer
│   ├── generics.rs
│   ├── patterns.rs
│   └── statements.rs
├── specialization/             # ✅ 保留（独立特性）
├── traits/                     # ✅ 保留（独立特性）
├── gat/                        # ✅ 保留（独立特性）
├── tests/                      # ✅ 保留
├── overload.rs                 # ✅ 保留（独立特性）
├── type_eval.rs                # ✅ 保留（独立特性）
└── specialize.rs              # ✅ 保留（兼容）
```

**问题**：`checking/` 和 `inference/` 实际上在做同一件事，却分成两个目录！

---

## 重构方案：合并 checking/ 到 inference/

### 目录结构（重构后）

```
src/frontend/typecheck/
├── mod.rs                  # 统一入口，导出所有模块
│
# ✅ 合并后的核心模块 inference/
├── inference/
│   ├── mod.rs             # 导出 + TypeChecker 主入口
│   ├── scope.rs           # 🆕 统一作用域管理
│   ├── types.rs           # 🆕 类型系统工具
│   ├── statements.rs      # 🆕 语句检查（合并 checking + inference 的语句部分）
│   ├── expressions.rs     # 🆕 表达式推断（合并现有 expressions.rs）
│   #
│   # ✅ 从 checking/ 移入
│   ├── assignment.rs      # 赋值检查
│   ├── subtyping.rs       # 子类型检查
│   ├── compatibility.rs   # 兼容性检查
│   ├── bounds.rs          # 边界检查
│   #
│   # ✅ 保留（增强）
│   ├── generics.rs        # 泛型推断
│   └── patterns.rs        # 模式推断
│
# ✅ 保留：独立特性模块（不改动，通过接口调用）
├── specialization/         # 特化逻辑
├── traits/                # Trait 逻辑
├── gat/                   # GAT 逻辑
├── overload.rs            # 重载解析
├── type_eval.rs           # 类型求值
├── specialize.rs          # 兼容
│
# ❌ 删除 checking/ 目录
└── tests/                  # 测试
```

### 模块职责划分

| 模块 | 职责 | 说明 |
|------|------|------|
| `inference/scope.rs` | 统一变量作用域管理 | 所有变量的增删改查 |
| `inference/types.rs` | 类型工具 | unify, infer_element_type 等 |
| `inference/statements.rs` | 语句检查 | Var, Fn, For, If, Expr 等语句 |
| `inference/expressions.rs` | 表达式推断 | Lit, Var, BinOp, Call, For 等表达式 |
| `inference/assignment.rs` | 赋值检查 | 从 checking/ 移入 |
| `inference/subtyping.rs` | 子类型检查 | 从 checking/ 移入 |
| `inference/compatibility.rs` | 兼容性检查 | 从 checking/ 移入 |
| `inference/bounds.rs` | 边界检查 | 从 checking/ 移入 |
| `specialization/*` | 特化 | 独立插件 |
| `traits/*` | Trait | 独立插件 |
| `gat/*` | GAT | 独立插件 |
| `overload.rs` | 重载解析 | 独立插件 |

### 关键设计原则

1. **单一入口**：`inference/` 是唯一的类型推断入口
2. **ScopeManager 单一实例**：整个检查流程共享同一个 ScopeManager
3. **特性模块独立**：specialization/traits/gat/overload 作为插件被调用
4. **无重复代码**：删除 BodyChecker 和 ExprInferrer 中重复的 scopes

---

## 详细设计

### inference/scope.rs - 统一作用域管理

```rust
/// 作用域管理器
/// 单一职责：管理变量作用域栈
pub struct ScopeManager {
    scopes: Vec<HashMap<String, PolyType>>,
}

impl ScopeManager {
    pub fn new() -> Self
    pub fn enter_scope(&mut self)
    pub fn exit_scope(&mut self)
    pub fn add_var(&mut self, name: String, poly: PolyType)
    pub fn get_var(&self, name: &str) -> Option<&PolyType>
    pub fn update_var(&mut self, name: &str, poly: PolyType)
    pub fn var_in_current_scope(&self, name: &str) -> bool
    pub fn var_in_any_scope(&self, name: &str) -> bool
}
```

### inference/types.rs - 类型系统工具

```rust
/// 类型系统工具
pub struct TypeSystem;

impl TypeSystem {
    /// 统一两个类型
    pub fn unify(ty1: &MonoType, ty2: &MonoType, solver: &mut TypeConstraintSolver) -> Result<(), Box<Diagnostic>>

    /// 从可迭代对象类型推导元素类型
    pub fn infer_element_type(iter_ty: &MonoType) -> MonoType

    /// 构造列表类型
    pub fn make_list_type(elem_ty: MonoType) -> MonoType

    /// 检查类型是否可迭代
    pub fn is_iterable(ty: &MonoType) -> bool

    /// 调用特性模块检查 trait 约束
    pub fn check_trait_bounds(ty: &MonoType, bounds: &[TraitBound], trait_table: &TraitTable) -> Result<(), Box<Diagnostic>>

    /// 调用特化模块进行实例化
    pub fn instantiate(ty: &MonoType, args: &[MonoType]) -> Result<MonoType, Box<Diagnostic>>
}
```

### inference/statements.rs - 语句检查

```rust
use crate::inference::scope::ScopeManager;
use crate::inference::types::TypeSystem;
use crate::inference::assignment::AssignmentChecker;
use crate::inference::subtyping::SubtypeChecker;

/// 语句检查器
pub struct StatementChecker<'a> {
    scope: &'a mut ScopeManager,
    solver: &'a mut TypeConstraintSolver,
    type_system: &'a TypeSystem,
}

impl<'a> StatementChecker<'a> {
    pub fn new(scope: &'a mut ScopeManager, solver: &'a mut TypeConstraintSolver) -> Self

    pub fn check(&mut self, stmt: &Stmt) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            StmtKind::Var { .. } => self.check_var(),
            StmtKind::Fn { .. } => self.check_fn(),
            StmtKind::For { .. } => self.check_for(),
            StmtKind::If { .. } => self.check_if(),
            StmtKind::Expr { .. } => self.check_expr_stmt(),
            // ...
        }
    }

    fn check_var(&mut self, name: &str, init: Option<&Expr>, annot: Option<&Type>) -> Result<(), Box<Diagnostic>>
    fn check_fn(&mut self, ...) -> Result<(), Box<Diagnostic>>
    fn check_for(&mut self, ...) -> Result<(), Box<Diagnostic>>
}
```

### inference/expressions.rs - 表达式推断

```rust
use crate::inference::scope::ScopeManager;
use crate::inference::types::TypeSystem;

/// 表达式推断器（使用统一的 ScopeManager）
pub struct ExpressionInferrer<'a> {
    scope: &'a ScopeManager,  // 只读引用
    solver: &'a mut TypeConstraintSolver,
    type_system: &'a TypeSystem,
}

impl<'a> ExpressionInferrer<'a> {
    pub fn infer(&mut self, expr: &Expr) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            Expr::Lit(..) => self.infer_literal(),
            Expr::Var(..) => self.infer_var(),
            Expr::BinOp(..) => self.infer_binop(),
            Expr::Call(..) => self.infer_call(),
            Expr::For(..) => self.infer_for(),
            Expr::Lambda(..) => self.infer_lambda(),
            // ...
        }
    }

    fn infer_literal(&mut self, lit: &Literal) -> Result<MonoType, Box<Diagnostic>>
    fn infer_var(&mut self, name: &str, span: Span) -> Result<MonoType, Box<Diagnostic>>
    fn infer_binop(&mut self, op: &BinOp, left: &Expr, right: &Expr) -> Result<MonoType, Box<Diagnostic>>
}
```

### inference/mod.rs - 统一入口

```rust
// 导出所有模块
pub mod scope;
pub mod types;
pub mod statements;
pub mod expressions;
pub mod assignment;
pub mod subtyping;
pub mod compatibility;
pub mod bounds;
pub mod generics;
pub mod patterns;

pub use scope::ScopeManager;
pub use types::TypeSystem;
pub use statements::StatementChecker;
pub use expressions::ExpressionInferrer;
pub use assignment::AssignmentChecker;
pub use subtyping::SubtypeChecker;
pub use compatibility::CompatibilityChecker;
pub use bounds::BoundsChecker;

// 统一的类型检查器入口
pub struct TypeChecker {
    scope: ScopeManager,
    solver: TypeConstraintSolver,
    type_system: TypeSystem,
    // 特性模块引用
    trait_table: TraitTable,
    specialization_context: SpecializationContext,
}

impl TypeChecker {
    pub fn new() -> Self

    pub fn check_module(&mut self, module: &Module) -> Result<TypeCheckResult, Vec<Diagnostic>> {
        // 1. 收集类型定义
        // 2. 收集函数签名
        // 3. 检查所有语句
        // 4. 求解约束
    }
}
```

---

## 重构步骤

### 阶段 1：创建 scope.rs 和 types.rs ✅

**目标**：创建基础模块

**交付物**：
- ✅ `inference/scope.rs` - ScopeManager（含 enter_scope/exit_scope/add_var/get_var/update_var/var_in_current_scope/var_in_any_scope/vars/scope_level）
- ✅ `inference/types.rs` - TypeSystem（含 unify/infer_element_type/make_list_type/is_iterable）

### 阶段 2：创建 statements.rs ✅

**目标**：合并 BodyChecker + StmtInferrer 的语句检查逻辑

**交付物**：
- ✅ `inference/statements.rs` - StatementChecker（861 行，包含完整的语句检查逻辑）

**实现细节**：
- StatementChecker 拥有 `scope: ScopeManager` 和 `solver: TypeConstraintSolver`
- `check_expr()` 通过 Rust 部分借用将 `&mut self.scope` 和 `&mut self.solver` 传给 ExpressionInferrer，消除了变量拷贝
- 保留向后兼容别名：`pub type BodyChecker = StatementChecker;`

### 阶段 3：创建 expressions.rs ✅

**目标**：合并 ExprInferrer 的表达式推断逻辑

**交付物**：
- ✅ `inference/expressions.rs` - ExpressionInferrer（897 行，使用共享 ScopeManager）

**实现细节**：
- ExpressionInferrer 借用 `scope: &'a mut ScopeManager` 和 `solver: &'a mut TypeConstraintSolver`
- 构造函数签名：`new(scope, solver, overload_candidates)` / `with_native_signatures(scope, solver, overloads, natives)`
- 保留向后兼容别名：`pub type ExprInferrer<'a> = ExpressionInferrer<'a>;`

### 阶段 4：移动 checking/ 的文件到 inference/ ✅

**目标**：合并 checking/ 到 inference/

**移动**：
- ✅ `checking/assignment.rs` → `inference/assignment.rs`
- ✅ `checking/subtyping.rs` → `inference/subtyping.rs`
- ✅ `checking/compatibility.rs` → `inference/compatibility.rs`
- ✅ `checking/bounds.rs` → `inference/bounds.rs`

### 阶段 5：修改 mod.rs 入口 ✅

**文件**：`src/frontend/typecheck/mod.rs`

**修改**：
- ✅ 删除 `pub mod checking;`
- ✅ 更新 `pub use inference::*;` 导出
- ✅ 更新 `infer_expression()` 使用 ScopeManager + ExpressionInferrer
- ✅ 更新 `TypeChecker` 引用 `inference::StatementChecker`

### 阶段 6：删除旧代码和目录 ✅

**删除**：
- ✅ `checking/` 目录已完全删除
- ✅ 旧 BodyChecker 代码已替换为 StatementChecker
- ✅ ExprInferrer.scopes 已替换为共享 ScopeManager

### 阶段 7：回归测试 ✅

```bash
cargo test
# test result: ok. 1434 passed; 0 failed; 4 ignored
# test result: ok. 30 passed; 0 failed
# test result: ok. 5 passed; 0 failed; 11 ignored
```

**测试文件更新**：
- ✅ `tests/shadowing.rs` - 更新 BodyChecker 导入路径，ExprInferrer 添加 ScopeManager 参数
- ✅ `tests/scope.rs` - ExprInferrer 添加 ScopeManager 参数
- ✅ `tests/infer.rs` - 39 处 ExprInferrer 签名更新，StmtInferrer 测试重写为 StatementChecker
- ✅ `tests/constraint.rs` - 6 处 checking:: → inference:: 导入路径更新
- ✅ `tests/basic.rs` - 18 处 ExprInferrer 签名更新

---

## 需要清理的代码

### 1. BodyChecker → statements.rs

| 原位置 | 目标位置 |
|--------|---------|
| `checking/mod.rs` - `BodyChecker` | `inference/statements.rs` - `StatementChecker` |
| `check_stmt`, `check_var_stmt` 等 | `StatementChecker::check_*` |

### 2. ExprInferrer → expressions.rs

| 原位置 | 目标位置 |
|--------|---------|
| `inference/expressions.rs` - `ExprInferrer` | `inference/expressions.rs` - `ExpressionInferrer` |
| `scopes` 字段 | 使用 `ScopeManager` |

### 3. checking/ → inference/

| 原位置 | 目标位置 |
|--------|---------|
| `checking/assignment.rs` | `inference/assignment.rs` |
| `checking/subtyping.rs` | `inference/subtyping.rs` |
| `checking/compatibility.rs` | `inference/compatibility.rs` |
| `checking/bounds.rs` | `inference/bounds.rs` |

### 4. 删除

| 删除项 | 说明 |
|--------|------|
| `checking/` 目录 | 完全删除 |
| `BodyChecker` 结构体 | 已迁移到 StatementChecker |
| `ExprInferrer.scopes` | 改用 ScopeManager |

---

## 扩展性设计

### 添加新语句类型

```rust
// inference/statements.rs
impl StatementChecker {
    pub fn check(&mut self, stmt: &Stmt) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            // ... 现有语句
            StmtKind::Match { .. } => self.check_match(),  // 🆕
            StmtKind::While { .. } => self.check_while(),  // 🆕
        }
    }
}
```

### 添加新表达式类型

```rust
// inference/expressions.rs
impl ExpressionInferrer {
    pub fn infer(&mut self, expr: &Expr) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            // ... 现有表达式
            Expr::Macro { .. } => self.infer_macro(),  // 🆕
            Expr::Await { .. } => self.infer_await(),  // 🆕
        }
    }
}
```

---

## 验收标准

### 架构验收

- [x] `inference/scope.rs` 独立负责作用域管理
- [x] `inference/statements.rs` 独立负责语句检查
- [x] `inference/expressions.rs` 独立负责表达式推断
- [x] `inference/types.rs` 提供类型系统工具
- [x] `inference/assignment.rs`, `subtyping.rs`, `compatibility.rs`, `bounds.rs` 正常工作
- [x] 特性模块（specialization/traits/gat/overload）保持独立
- [x] 删除 `checking/` 目录
- [x] 没有手动同步变量的逻辑（使用共享 ScopeManager 的 Rust 部分借用模式）

### 功能验收

| 测试用例 | 预期结果 |
|---------|---------|
| `nums = [1,2,3]; for n in nums { print(n) }` | 编译成功 |
| `x = 10; for i in 1..3 { x = i }` | 编译成功 |
| `entry: FileEntry = item` | 类型标注正确工作 |

### 回归测试

```bash
cargo test
```

预期：所有测试通过

---

## 测试计划

### 阶段 1：单元测试

| 测试名称 | 模块 |
|---------|------|
| test_enter_scope | scope.rs |
| test_exit_scope | scope.rs |
| test_add_var | scope.rs |
| test_get_var_outer | scope.rs |
| test_unify_int | types.rs |
| test_infer_element_type | types.rs |

### 阶段 2：集成测试

| 测试名称 | 测试代码 | 预期结果 |
|---------|---------|---------|
| test_for_list | `for n in [1,2,3] { print(n) }` | 编译成功 |
| test_var_scope | 变量作用域正确 | 通过 |
| test_type_annotation | `x: Int = 1` | 编译成功 |
| test_generic_fn | 泛型函数 | 正确工作 |
| test_trait_bound | Trait 约束 | 正确工作 |

### 阶段 3：回归测试

```bash
cargo test
```

预期：所有测试通过
