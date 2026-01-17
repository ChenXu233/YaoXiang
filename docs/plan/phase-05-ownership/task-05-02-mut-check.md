# Task 5.2: 可变性检查

> **优先级**: P0
> **状态**: 🔄 待实现
> **模块**: `src/core/ownership/mut_check.rs`
> **依赖**: task-05-01（需要所有权状态信息）

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

## 检查算法

```rust
struct MutChecker {
    /// 可变变量集合
    mutable_vars: HashSet<ValueId>,
    /// 可变变量修改错误
    errors: Vec<MutCheckError>,
}

impl MutChecker {
    /// 检查变量修改
    fn check_assignment(&mut self, target: &ValueId) -> Result<(), MutCheckError> {
        if self.mutable_vars.contains(target) {
            Ok(())  // 可变变量，允许修改
        } else {
            Err(MutCheckError::ImmutableAssign {
                value: *target,
            })
        }
    }

    /// 检查方法调用（修改方法）
    fn check_method_call(&mut self, method: &str, target: &ValueId) -> Result<(), MutCheckError> {
        // 检查是否是修改方法（如 push, insert, remove 等）
        if is_mutation_method(method) {
            self.check_assignment(target)?;
        }
        Ok(())
    }

    /// 记录 mut 声明
    fn record_mut_declaration(&mut self, value_id: ValueId) {
        self.mutable_vars.insert(value_id);
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone)]
pub enum MutCheckError {
    ImmutableAssign {
        value: ValueId,
    },
    ImmutableMutation {
        value: ValueId,
        method: String,
    },
}
```

## 与 RFC-009 v7 对照

| RFC-009 规则 | 实现状态 |
|-------------|---------|
| 默认不可变 | ✅ 已实现 |
| mut 标记允许修改 | ✅ 已实现 |
| 未标记 mut 的修改报错 | ✅ 已实现 |

## 验收测试

```yaoxiang
# test_mut_check.yx

# === 不可变测试 ===
data: List[Int] = [1, 2, 3]
# data.push(4)  # 应该编译错误

# === mut 标记测试 ===
mut counter: Int = 0
counter = counter + 1  # ✅ 允许

mut list: List[Int] = [1, 2, 3]
list.push(4)           # ✅ 允许
assert(list.length == 4)

# === 函数式风格测试 ===
data: List[Int] = [1, 2, 3]
data2: List[Int] = data.concat([4])  # ✅ 创建新值
assert(data2.length == 4)
assert(data.length == 3)  # 原数据不变

print("Mut check tests passed!")
```

## 相关文件

- **src/core/ownership/mut_check.rs**: 可变性检查器
- **src/core/ownership/errors.rs**: 错误定义
