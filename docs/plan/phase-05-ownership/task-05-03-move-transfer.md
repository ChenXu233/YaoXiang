# Task 5.3: 所有权转移

> **优先级**: P0
> **状态**: 🔄 待实现

## 功能描述

跟踪所有权的转移和复制：
- `move` 语义：转移后原所有者失效
- `copy` 语义：浅拷贝（针对 Copy 类型）
- `drop`：值离开作用域时释放

## 所有权规则

### Move 语义

```yaoxiang
# move：所有权转移
data = [1, 2, 3]
new_owner = data  # data 不再可用
# print(data.length)  # 编译错误！

# 函数调用也转移所有权
fn process[T](input: List[T]) -> T {
    input[0]
}

data = [1, 2, 3]
result = process(data)  # data 移动进函数，不再可用
```

### Copy 语义

```yaoxiang
# Copy 类型：浅拷贝
# - 原类型（Int, Float, Bool）
# - 不可变引用 ref T
# - 不包含 Move 类型的元组/结构体

x: Int = 42
y = x  # x 仍然可用（Copy）

# 自定义 Copy 类型
type Point = Point(x: Int, y: Int)  # 自动实现 Copy

p: Point = Point(1, 2)
q = p  # p 仍然可用
```

### Drop 规则

```yaoxiang
# 值离开作用域时自动释放
foo: () -> Void = () => {
    data: List[Int] = [1, 2, 3]  # 分配
    # data 在这里自动释放
}

# Drop 顺序：后定义先释放
bar: () -> Void = () => {
    a: List[Int] = [1, 2]
    b: List[Int] = [3, 4]
    # b 先释放，然后是 a
}
```

## 检查算法

```rust
struct OwnershipAnalyzer {
    /// 每个值的所有者
    owner_of: HashMap<ValueId, ValueId>,
    /// 值的状态（Owned, Moved, Copied）
    state: HashMap<ValueId, ValueState>,
    /// 所有权错误
    errors: Vec<OwnershipError>,
}

impl OwnershipAnalyzer {
    /// 分析所有权转移
    fn analyze(&mut self, func: &FunctionIR) -> OwnershipResult {
        for instr in func.all_instructions() {
            match instr {
                Instruction::Move { dst, src } => {
                    self.analyze_move(dst, src)?;
                }
                Instruction::Copy { dst, src } => {
                    self.analyze_copy(dst, src)?;
                }
                Instruction::Drop { value } => {
                    self.analyze_drop(value)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn analyze_move(&mut self, dst: &Operand, src: &Operand) -> Result<(), OwnershipError> {
        let src_id = self.get_value_id(src)?;

        // 检查 src 是否可移动
        if let Some(state) = self.state.get(&src_id) {
            match state {
                ValueState::Moved => {
                    return Err(OwnershipError::UseAfterMove {
                        value: src_id,
                    });
                }
                ValueState::Copied => {
                    return Err(OwnershipError::InvalidMove {
                        value: src_id,
                        reason: "value is Copy",
                    });
                }
                _ => {}
            }
        }

        // 转移所有权
        self.state.insert(src_id, ValueState::Moved);
        self.owner_of.insert(self.get_value_id(dst)?, src_id);

        Ok(())
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone)]
pub enum OwnershipError {
    UseAfterMove {
        value: ValueId,
    },
    InvalidMove {
        value: ValueId,
        reason: String,
    },
    MoveOfCopyType {
        value: ValueId,
    },
    DoubleDrop {
        value: ValueId,
    },
}
```

## 验收测试

```yaoxiang
# test_ownership.yx

# Move 测试
data: List[Int] = [1, 2, 3]
new_owner = data
# assert(data.length)  # 应该编译错误

# Copy 测试
x: Int = 42
y = x
assert(x == 42)
assert(y == 42)

# Drop 测试
count: Int = 0
with_drop: () -> Void = () => {
    temp: Int = count + 1
    # temp 在这里释放
}
with_drop()

print("Ownership tests passed!")
```

## 相关文件

- **src/core/ownership/move.rs**: 所有权转移检查
- **src/core/ownership/drop.rs**: Drop 顺序分析
