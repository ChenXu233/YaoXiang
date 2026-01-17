# Task 5.2: 可变性检查

> **优先级**: P0
> **状态**: ✅ 已完成
> **模块**: `src/middle/lifetime/mut_check.rs`
> **依赖**: task-05-01（需要所有权状态信息）
> **完成时间**: 2026-01-17

## 功能描述

检查 `mut` 标记的使用是否符合规则：
- 所有变量默认不可变
- 只有标记 `mut` 的变量才能被修改
- 编译期检查，无需运行时开销

> **说明**：由于 YaoXiang 是函数式语言，类型透明，不需要 `ref T` 借用语法，因此不需要复杂的借用检查器。只需简单的可变性检查即可。

## 可变性规则

### 默认不可变

```yaoxiang
# ✅ 不可变是默认行为
data: List[Int] = [1, 2, 3]
# data.push(4)  # ❌ 编译错误！未标记 mut

# ✅ 函数式风格：创建新值
data2: List[Int] = data.concat([4])
```

### mut 标记

```yaoxiang
# ✅ mut 标记允许修改
mut counter: Int = 0
counter = counter + 1  # ✅ 允许

# ✅ mut 数据结构
mut list: List[Int] = [1, 2, 3]
list.push(4)           # ✅ 允许

# ❌ 未标记 mut 不能修改
data: List[Int] = [1, 2, 3]
# data.push(4)  # ❌ 编译错误！
```

## 实现架构

### MutChecker 结构

```rust
/// 可变性检查器
///
/// 检测以下错误：
/// - ImmutableAssign: 对不可变变量进行赋值
/// - ImmutableMutation: 调用不可变对象上的变异方法
#[derive(Debug)]
pub struct MutChecker {
    /// 可变变量集合 (Operand -> is_mut)
    mutable_vars: HashMap<Operand, bool>,
    /// 可变变量修改错误
    errors: Vec<OwnershipError>,
    /// 当前检查位置
    location: (usize, usize),
    /// 符号表：变量名 -> 是否可变（从外部传入）
    symbol_table: Option<HashMap<String, bool>>,
    /// 兼容 OwnershipCheck trait 的状态字段
    state: HashMap<Operand, ValueState>,
}

impl MutChecker {
    /// 检查赋值操作
    fn check_store(&mut self, target: &Operand) {
        if self.is_mutable(target) {
            return;
        }
        self.errors.push(OwnershipError::ImmutableAssign { ... });
    }

    /// 检查变异方法调用
    fn check_mutation_method(&mut self, method: &str, target: &Operand) {
        if !is_mutation_method(method) {
            return; // 非变异方法，允许
        }
        if self.is_mutable(target) {
            return; // 可变变量，允许
        }
        self.errors.push(OwnershipError::ImmutableMutation { ... });
    }

    /// 检查变量是否可变（通用逻辑）
    fn is_mutable(&self, target: &Operand) -> bool {
        // 1. 检查可变变量集合
        // 2. 检查符号表
        false // 默认不可变
    }
}
```

### 变异方法识别

```rust
/// 变异方法集合（使用 HashSet 实现 O(1) 查询）
static MUTATION_METHODS: once_cell::sync::Lazy<HashSet<&'static str>> =
    once_cell::sync::Lazy::new(|| {
        [
            "push", "pop", "insert", "remove", "clear",
            "append", "extend", "set", "update", "add",
            "delete", "discard", "swap", "fill",
        ]
        .into_iter()
        .collect()
    });
```

## 错误类型（共享）

使用共享的 `OwnershipError` 枚举，而非独立错误类型：

```rust
/// 所有权检查错误类型
///
/// 包含 Move/Drop/Mut 三种检查的错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipError {
    // ... Move/Drop 错误 ...
    /// 不可变赋值：对不可变变量进行赋值
    ImmutableAssign {
        value: String,
        location: (usize, usize),
    },
    /// 不可变变异：调用不可变对象上的变异方法
    ImmutableMutation {
        value: String,
        method: String,
        location: (usize, usize),
    },
}
```

## 与 OwnershipChecker 集成

```rust
/// 统一的所有权检查器
///
/// 同时运行 Move 检查、Drop 检查和 Mut 检查，返回所有错误。
pub struct OwnershipChecker {
    move_checker: MoveChecker,
    drop_checker: DropChecker,
    mut_checker: MutChecker,  // 新增
}

impl OwnershipChecker {
    pub fn check_function(&mut self, func: &FunctionIR) -> Vec<OwnershipError> {
        let move_errors = self.move_checker.check_function(func);
        let drop_errors = self.drop_checker.check_function(func);
        let mut_errors = self.mut_checker.check_function(func);
        // 合并错误
        move_errors.iter().chain(drop_errors).chain(mut_errors).cloned().collect()
    }
}
```

## 与 RFC-009 v7 对照

| RFC-009 规则 | 实现状态 |
|-------------|---------|
| 默认不可变 | ✅ 已实现 |
| mut 标记允许修改 | ✅ 已实现 |
| 未标记 mut 的修改报错 | ✅ 已实现 |

## 单元测试

```rust
// src/middle/lifetime/tests/mut_check.rs

#[test]
fn test_immutable_var_assignment_error() {
    let mut checker = MutChecker::new();
    let instructions = vec![Instruction::Store {
        dst: Operand::Local(0),
        src: Operand::Const(ConstValue::Int(42)),
    }];
    let func = create_test_function(instructions);
    let errors = checker.check_function(&func);
    assert!(matches!(errors[0], OwnershipError::ImmutableAssign { .. }));
}

#[test]
fn test_immutable_mutation_method_error() {
    let mut checker = MutChecker::new();
    let instructions = vec![Instruction::Call {
        dst: None,
        func: Operand::Const(ConstValue::String("push".to_string())),
        args: vec![Operand::Local(0), Operand::Const(ConstValue::Int(42))],
    }];
    let func = create_test_function(instructions);
    let errors = checker.check_function(&func);
    assert!(matches!(errors[0], OwnershipError::ImmutableMutation { .. }));
}

#[test]
fn test_is_mutation_method() {
    assert!(is_mutation_method("push"));
    assert!(!is_mutation_method("concat"));
}
```

**测试结果**: 10/10 通过

## 相关文件

- **src/middle/lifetime/mut_check.rs**: 可变性检查器实现
- **src/middle/lifetime/error.rs**: 共享错误定义（ImmutableAssign, ImmutableMutation）
- **src/middle/lifetime/mod.rs**: 模块入口，集成到 OwnershipChecker
- **src/middle/lifetime/tests/mut_check.rs**: 单元测试
