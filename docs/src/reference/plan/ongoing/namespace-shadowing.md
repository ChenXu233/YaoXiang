# 变量命名空间与遮蔽机制代码审查报告

## 审查时间
2026-02-18

## 审查范围
- 变量命名空间管理（作用域）
- 变量遮蔽检测
- For 循环变量语义
- `mut` 声明处理

---

## 需求回顾

根据设计，语言的变量遮蔽规则如下：

1. **禁止遮蔽** - 任何试图在外层作用域已存在的变量名，在内层重新声明都会报错
2. **无 let 关键字** - 通过 `mut` 或隐式声明创建变量
3. **遮蔽即报错** - 无论是普通声明还是 `mut` 声明，遮蔽都报错
4. **局部变量出作用域销毁** - 局部作用域结束时变量被销毁
5. **For 循环语义** - `for i in iter` 中的 `i` 是重新绑定，每次迭代创建新的局部绑定

---

## 评分：🟡 凑合

## 作用域实现情况总览

### 类型推断阶段（ExprInferrer）

| 作用域类型 | 实现状态 | 代码位置 |
|-----------|---------|---------|
| For 循环 | ✅ 有 | expressions.rs:778-784 |
| 函数定义 | ✅ 有 | expressions.rs:846-863 |
| Lambda 表达式 | ✅ 有 | expressions.rs:885-894 |
| 列表推导 | ✅ 有 | expressions.rs:945-956 |
| If 语句分支 | ❌ 没有 | - |
| 普通代码块 `{}` | ❌ 没有 | - |

### 类型检查阶段（BodyChecker）

| 作用域类型 | 实现状态 | 问题 |
|-----------|---------|------|
| 所有 | ❌ 完全没有 | 使用扁平 HashMap，无作用域栈 |

---

## 详细问题分析

### 问题 1：`mut` 声明没有遮蔽检查

**位置**: `src/frontend/typecheck/checking/mod.rs:310-315`

```rust
// 如果变量已存在，统一类型  ← 错误的行为！
if let Some(existing_poly) = self.vars.get(name) {
    let _ = self.solver.unify(&existing_poly.body, &ty);
}
self.vars.insert(name.to_string(), PolyType::mono(ty));
```

**问题**：当用 `mut` 声明一个已存在的变量时，代码尝试统一类型而不是报错。这违反了禁止遮蔽的规则。

**期望行为**：
```rust
// 应该检查遮蔽并报错
if self.var_exists_in_any_scope(name) {
    return Err(ErrorCodeDefinition::variable_shadowing(name).build());
}
```

---

### 问题 2：作用域管理缺失 - 变量无法销毁

**位置**: `src/frontend/typecheck/checking/mod.rs:26-36`

```rust
pub struct BodyChecker {
    solver: TypeConstraintSolver,
    vars: HashMap<String, PolyType>,  // ← 扁平的 HashMap，没有作用域栈！
    ...
}
```

**问题**：`BodyChecker` 使用扁平的 `HashMap` 存储变量，没有作用域栈。变量一旦添加就永远存在，出了作用域不会销毁。

**正确实现参考** - `ExprInferrer`（正确实现）：
```rust
pub struct ExprInferrer {
    scopes: Vec<HashMap<String, PolyType>>,  // ← 作用域栈
    ...
}

pub fn enter_scope(&mut self) {
    self.scopes.push(HashMap::new());
}

pub fn exit_scope(&mut self) {
    if self.scopes.len() > 1 {
        self.scopes.pop();
    }
}
```

---

### 问题 3：For 循环没有创建新作用域

**位置**: `src/frontend/typecheck/checking/mod.rs:320-355`

```rust
fn check_for_stmt(...) {
    // 只检查遮蔽，没有创建新作用域
    if self.vars.contains_key(var) { ... }  // ← 遮蔽检查 ✓
    self.vars.insert(var.to_string(), ...);  // ← 直接插入顶层
    for stmt in &body.stmts { ... }          // ← 没有 enter_scope()
    // ← 也没有 exit_scope()，变量永远存在！
}
```

**问题**：
1. for 循环的循环变量直接插入顶层 `vars`，而不是作用域
2. 循环体内声明的局部变量会泄漏到外层
3. 循环结束后循环变量不会被销毁

**期望行为**：
```rust
fn check_for_stmt(...) {
    self.enter_scope();  // 创建循环体作用域
    // 遮蔽检查...
    self.add_var(var, ...);
    // 检查循环体...
    self.exit_scope();    // 退出时销毁循环变量
}
```

---

### 问题 4：For 循环的"重新绑定"语义未完全实现

**需求**：for 循环的 `i` 语义应该是"重新绑定"，每次迭代时 i 相当于重新获取值。

**当前实现**：只是类型检查阶段的一次性插入

**说明**：这点的实现可以在 IR 生成或解释器层面处理，类型检查阶段只需确保变量在作用域内即可。当前的实现可以接受，但需要在文档中明确这一点。

---

## 正确实现的部分

### 1. 遮蔽检测机制已存在

**位置**:
- `src/util/diagnostic/codes/e2xxx.rs:85` - 错误码定义
- `src/frontend/typecheck/inference/expressions.rs:92-108` - `try_add_var` 方法

```rust
/// 尝试添加变量到当前作用域（带遮蔽检查）
pub fn try_add_var(...) -> Result<()> {
    if self.var_exists_in_any_scope(&name) {
        return Err(ErrorCodeDefinition::variable_shadowing(&name).build());
    }
    ...
}
```

### 2. For 循环遮蔽检查已部分实现

**位置**: `src/frontend/typecheck/checking/mod.rs:334-339`

```rust
// 遮蔽检查：如果变量已存在，报错
if self.vars.contains_key(var) {
    return Err(Box::new(
        ErrorCodeDefinition::variable_shadowing(var).build(),
    ));
}
```

### 3. 作用域栈在推断阶段已实现

**位置**: `src/frontend/typecheck/inference/expressions.rs:146-155`

```rust
pub fn enter_scope(&mut self) {
    self.scopes.push(HashMap::new());
}

pub fn exit_scope(&mut self) {
    if self.scopes.len() > 1 {
        self.scopes.pop();
    }
}
```

---

## 修复建议

### 修复 1：为 BodyChecker 添加作用域栈

在 `src/frontend/typecheck/checking/mod.rs` 中：

1. 将 `vars: HashMap<String, PolyType>` 替换为 `scopes: Vec<HashMap<String, PolyType>>`
2. 添加 `enter_scope()` 和 `exit_scope()` 方法
3. 实现 `var_exists_in_any_scope()`, `var_exists_in_current_scope()`, `get_var()` 等方法

### 修复 2：mut 声明添加遮蔽检查

在 `check_var_stmt` 方法中添加遮蔽检查逻辑。

### 修复 3：For 循环使用作用域

在 `check_for_stmt` 方法中调用 `enter_scope()` 和 `exit_scope()`。

---

## 测试建议

建议添加以下测试用例：

```rust
#[test]
fn test_mut_shadowing_error() {
    // mut 声明同名变量应该报错
}

#[test]
fn test_for_loop_variable_destroyed() {
    // for 循环结束后变量应该不存在
}

#[test]
fn test_nested_scope_variable_destroyed() {
    // 代码块结束后变量应该不存在
}
```

---

## 总结

### 推断阶段（ExprInferrer）

| 功能 | 状态 |
|------|------|
| For 循环作用域 | ✅ 已实现 |
| 函数定义作用域 | ✅ 已实现 |
| Lambda 作用域 | ✅ 已实现 |
| 列表推导作用域 | ✅ 已实现 |
| If 语句作用域 | ❌ 缺失 |
| 普通代码块作用域 | ❌ 缺失 |

### 检查阶段（BodyChecker）

| 功能 | 状态 |
|------|------|
| 作用域栈 | ❌ 完全缺失 |
| For 循环作用域 | ❌ 缺失 |
| 函数参数作用域 | ❌ 缺失 |
| If 语句作用域 | ❌ 缺失 |
| 普通代码块作用域 | ❌ 缺失 |
| mut 遮蔽检查 | ❌ 缺失 |
| For 循环遮蔽检查 | ✅ 已实现 |

---

## 核心问题

1. **BodyChecker 完全没有作用域管理** - 使用扁平 HashMap，变量无法销毁
2. **推断阶段缺少 If/代码块作用域** - 只有特定结构有作用域
3. **遮蔽规则不完整** - mut 声明没有遮蔽检查
