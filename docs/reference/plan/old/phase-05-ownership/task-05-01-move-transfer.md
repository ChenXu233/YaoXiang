# Task 5.1: Move 语义（所有权转移）

> **优先级**: P0
> **状态**: ✅ 已实现
> **模块**: `src/middle/lifetime/`
> **依赖**: 无（基础模块）

## 功能描述

跟踪所有权的转移（Move）：

- **Move 语义**：赋值即转移，原所有者失效
- **零拷贝设计**：不自动复制，所有复制必须显式调用 `clone()`
- **Drop 规则**：值离开作用域时自动释放（RAII）

> **RFC-009 v7 核心设计**：默认 Move，零拷贝。复制必须用 `clone()` 显式调用。
> **注意**：此任务是所有权系统的**基础模块**，其他所有任务都依赖于它。

## 所有权规则

### Move 语义（赋值即转移）

```yaoxiang
# Move：所有权转移，零拷贝
data: List[Int] = [1, 2, 3]
new_owner = data    # data 不再可用
# print(data.length)  # 编译错误！UseAfterMove

# 函数调用也转移所有权
process: (List[Int]) -> Int = (input) => input[0]

data = [1, 2, 3]
result = process(data)  # data 移动进函数，不再可用
# print(data.length)    # 编译错误！
```

### 所有类型都是 Move

```yaoxiang
# 基础类型也是 Move
x: Int = 42
y = x           # x 不再可用
# print(x)      # 编译错误！

# 结构体同样是 Move
type Point = Point(x: Int, y: Int)
p: Point = Point(1, 2)
q = p           # p 不再可用
# print(p.x)    # 编译错误！

# 需要保留原值时，使用 clone()
p: Point = Point(1, 2)
q = p.clone()   # p 和 q 都可用
print(p.x)      # ✅ 1
print(q.x)      # ✅ 1
```

### Drop 规则（RAII）

```yaoxiang
# 值离开作用域时自动释放
foo: () -> Void = () => {
    data: List[Int] = [1, 2, 3]  # 分配
    # data 在这里自动释放（RAII）
}

# Drop 顺序：后定义先释放（栈顺序）
bar: () -> Void = () => {
    a: List[Int] = [1, 2]
    b: List[Int] = [3, 4]
    # b 先释放，然后是 a
}
```

## 检查算法

```rust
/// 所有权状态
#[derive(Debug, Clone, PartialEq)]
enum ValueState {
    /// 有效，所有者可用
    Owned,
    /// 已被移动，所有者不可用
    Moved,
    /// 已被释放
    Dropped,
}

struct OwnershipAnalyzer {
    /// 每个值的状态
    state: HashMap<ValueId, ValueState>,
    /// 作用域栈（用于 Drop 顺序）
    scopes: Vec<Scope>,
    /// 所有权错误
    errors: Vec<OwnershipError>,
}

impl OwnershipAnalyzer {
    /// 分析所有权转移
    fn analyze(&mut self, func: &FunctionIR) -> OwnershipResult {
        for instr in func.all_instructions() {
            match instr {
                Instruction::Assign { dst, src } => {
                    self.analyze_assign(dst, src)?;
                }
                Instruction::Drop { value } => {
                    self.analyze_drop(value)?;
                }
                _ => {}
            }
        }
        self.check_double_drop()?;
        Ok(())
    }

    /// 分析赋值（Move 语义）
    fn analyze_assign(&mut self, dst: &Operand, src: &Operand) -> Result<(), OwnershipError> {
        let src_id = self.get_value_id(src)?;

        // 检查 src 是否已被移动
        if let Some(state) = self.state.get(&src_id) {
            match state {
                ValueState::Moved => {
                    return Err(OwnershipError::UseAfterMove {
                        value: src_id,
                        location: src.location,
                    });
                }
                ValueState::Dropped => {
                    return Err(OwnershipError::UseAfterDrop {
                        value: src_id,
                        location: src.location,
                    });
                }
                ValueState::Owned => {
                    // 正常 Move：标记原值已移动
                    self.state.insert(src_id, ValueState::Moved);
                }
            }
        } else {
            // 首次赋值
            self.state.insert(src_id, ValueState::Owned);
        }

        // 目标值状态
        self.state.insert(self.get_value_id(dst)?, ValueState::Owned);

        Ok(())
    }

    /// 分析 Drop
    fn analyze_drop(&mut self, value: &Operand) -> Result<(), OwnershipError> {
        let value_id = self.get_value_id(value)?;

        match self.state.get(&value_id) {
            Some(ValueState::Moved) => {
                return Err(OwnershipError::DropMovedValue {
                    value: value_id,
                });
            }
            Some(ValueState::Dropped) => {
                return Err(OwnershipError::DoubleDrop {
                    value: value_id,
                });
            }
            Some(ValueState::Owned) => {
                self.state.insert(value_id, ValueState::Dropped);
            }
            None => {
                // 未跟踪的值，忽略
            }
        }

        Ok(())
    }

    /// 检查双重释放
    fn check_double_drop(&self) -> Result<(), OwnershipError> {
        for (value, state) in &self.state {
            if *state == ValueState::Dropped {
                // 检查是否有其他引用指向此值
                // ...
            }
        }
        Ok(())
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum OwnershipError {
    /// 使用已移动的值
    UseAfterMove {
        value: ValueId,
        location: Location,
    },
    /// 使用已释放的值
    UseAfterDrop {
        value: ValueId,
        location: Location,
    },
    /// 释放已移动的值
    DropMovedValue {
        value: ValueId,
    },
    /// 双重释放
    DoubleDrop {
        value: ValueId,
    },
}
```

## 与 RFC-009 v7 对照

| RFC-009 v7 设计 | 实现状态 |
|----------------|---------|
| Move 语义（赋值即转移） | ✅ 已实现 |
| 零拷贝（不自动复制） | ✅ 已实现 |
| Drop 规则（RAII） | ✅ 已实现 |
| clone() 显式复制 | ❌ 见 task-05-04 |

## 模块结构

```
src/middle/lifetime/
├── mod.rs              # 主模块：协调者
├── error.rs            # 错误类型定义
│                       #   - UseAfterMove
│                       #   - UseAfterDrop
│                       #   - DropMovedValue
│                       #   - DoubleDrop
├── move_semantics.rs   # Move 语义检查
│                       #   - MoveChecker
│                       #   - UseAfterMove 检测
├── drop_semantics.rs   # Drop 语义检查
│                       #   - DropChecker
│                       #   - UseAfterDrop 检测
│                       #   - DropMovedValue 检测
│                       #   - DoubleDrop 检测
└── tests/
    ├── mod.rs          # 测试入口
    ├── move_semantics.rs
    └── drop_semantics.rs
```

## 验收测试

```yaoxiang
# test_move.yx

# === Move 测试（基础类型）===
x: Int = 42
y = x
# assert(x == 42)  # 编译错误！x 已被移动

# === Move 测试（结构体）===
type Point = Point(x: Int, y: Int)
p: Point = Point(1, 2)
q = p
# print(p.x)       # 编译错误！p 已被移动

# === Move 测试（函数参数）===
process: (List[Int]) -> Int = (input) => input[0]
data = [1, 2, 3]
result = process(data)
# print(data.length)  # 编译错误！data 已移动

# === clone() 测试（需要保留原值时）===
x: Int = 42
y = x.clone()    # 必须显式 clone()
assert(x == 42)  # ✅ x 仍然可用
assert(y == 42)

# === Drop 测试 ===
drop_count: Int = 0
create_and_drop: () -> Void = () => {
    temp: List[Int] = [1, 2, 3]
    # temp 在这里自动释放
}
create_and_drop()
# 资源已正确释放

print("Move semantics tests passed!")
```

