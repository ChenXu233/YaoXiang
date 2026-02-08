# Task 5.4: clone() 显式复制

> **优先级**: P0
> **状态**: ✅ 已实现
> **模块**: `src/middle/lifetime/clone.rs`

## 功能描述

`clone()` 方法用于显式复制值：

- **显式复制**：所有复制必须通过 `clone()` 调用
- **语义清晰**：代码明确表示"我需要复制"
- **性能可控**：用户控制复制时机

> **RFC-009 v7 核心设计**：不自动复制，复制必须用 `clone()` 显式调用。

## clone() 规则

### 基本用法

```yaoxiang
# 需要保留原值时使用 clone()
p: Point = Point(1.0, 2.0)
p2 = p.clone()   # p 和 p2 独立

p.x = 0.0        # ✅ p 修改
p2.x = 0.0       # ✅ p2 修改，互不影响

# 函数参数复制
process: (Point) -> Point = (p) => {
    other = p.clone()  # 复制参数，保留原值
    other.x = other.x + 1
    other
}

p: Point = Point(1.0, 2.0)
result = process(p)
# p 已被移动进函数，需要 clone() 保留
```

### 需要 clone() 的场景

```yaoxiang
# 场景 1：函数参数
fn_with_param: (Point) -> Void = (p) => {
    print(p.x)
}

p: Point = Point(1.0, 2.0)
fn_with_param(p)      # p 移动进函数，不再可用
# print(p.x)          # 错误！

# 需要保留原值：
p: Point = Point(1.0, 2.0)
fn_with_param(p.clone())  # 复制后传入
print(p.x)                # ✅ p 仍然可用

# 场景 2：循环中的值
points: List[Point] = [Point(1, 1), Point(2, 2)]
doubled: List[Point] = []

for p in points {
    p2 = p.clone()    # 复制，因为 p 将在下轮迭代被移动
    p2.x = p2.x * 2
    doubled.push(p2)
}

# 场景 3：集合操作
data: List[Int] = [1, 2, 3]
doubled = data.map((x) => x.clone())  # 复制元素
# data 仍然可用
```

### 性能考虑

```yaoxiang
# clone() 应该是有意识的行为
# 频繁 clone() 可能影响性能

# 好的用法：明确需要复制
config = get_default_config()
user_config = config.clone()
user_config.timeout = 5000  # 修改副本

# 需要考虑的场景
# - 大对象：clone() 开销大，考虑用 ref Arc
# - 热点代码：评估 clone() 频率
# - 性能关键：考虑数据结构设计
```

## 检查算法

Clone 检查器集成到现有的 `OwnershipChecker` 架构中，实现 `OwnershipCheck` trait：

```rust
// src/middle/lifetime/clone.rs

#[derive(Debug, Default)]
pub struct CloneChecker {
    state: HashMap<Operand, ValueState>,
    errors: Vec<OwnershipError>,
    location: (usize, usize),
}

impl CloneChecker {
    /// 检查 clone() 调用（核心逻辑）
    fn check_clone(&mut self, receiver: &Operand, dst: Option<&Operand>) {
        if let Some(state) = self.state.get(receiver) {
            match state {
                ValueState::Moved => self.error_clone_moved(receiver),
                ValueState::Dropped => self.error_clone_dropped(receiver),
                ValueState::Owned => {}
            }
            self.state.insert(receiver.clone(), ValueState::Owned);
        }
        if let Some(d) = dst {
            self.state.insert(d.clone(), ValueState::Owned);
        }
    }

    fn check_instruction(&mut self, instr: &Instruction) {
        match instr {
            // clone() 方法调用：检查 receiver 状态
            Instruction::Call { dst, func: Operand::Local(_) | Operand::Temp(_), args } => {
                if let Some(receiver) = args.first() {
                    self.check_clone(receiver, dst.as_ref());
                }
            }
            // Move：src 被移动，dst 成为新所有者
            Instruction::Move { dst, src } => {
                self.state.insert(src.clone(), ValueState::Moved);
                self.state.insert(dst.clone(), ValueState::Owned);
            }
            // 函数调用：参数被移动
            Instruction::Call { args, dst, .. } => {
                for arg in args {
                    self.state.insert(arg.clone(), ValueState::Moved);
                }
                if let Some(d) = dst {
                    self.state.insert(d.clone(), ValueState::Owned);
                }
            }
            // 返回：返回值被移动
            Instruction::Ret(Some(value)) => {
                self.state.insert(value.clone(), ValueState::Moved);
            }
            // Drop：值被释放
            Instruction::Drop(operand) => {
                self.state.insert(operand.clone(), ValueState::Dropped);
            }
            // 堆分配：新值是有效的所有者
            Instruction::HeapAlloc { dst, .. } => {
                self.state.insert(dst.clone(), ValueState::Owned);
            }
            // 闭包：环境变量被移动
            Instruction::MakeClosure { dst, env, .. } => {
                for var in env {
                    self.state.insert(var.clone(), ValueState::Moved);
                }
                self.state.insert(dst.clone(), ValueState::Owned);
            }
            // Arc 操作：不影响原值状态
            Instruction::ArcNew { dst, .. } | Instruction::ArcClone { dst, .. } => {
                self.state.insert(dst.clone(), ValueState::Owned);
            }
            Instruction::ArcDrop(_) => {}
            _ => {}
        }
    }
}
```

**设计要点**：
- **类型可克隆性**：在类型检查阶段确保（前端）
- **值状态检查**：在所有权检查阶段确保（CloneChecker）
- **状态管理**：clone() 后原值保持 Owned
- **代码风格**：使用 `#[derive(Default)]`，状态操作内聚

## 错误类型

复用在 `src/middle/lifetime/error.rs` 中定义的 `OwnershipError` 枚举：

```rust
pub enum OwnershipError {
    // ... 现有错误 ...
    /// clone 已移动的值
    CloneMovedValue {
        value: String,
        location: (usize, usize),
    },
    /// clone 已释放的值
    CloneDroppedValue {
        value: String,
        location: (usize, usize),
    },
}
```

## 与 RFC-009 v7 对照

| RFC-009 v7 设计 | 实现状态 |
|----------------|---------|
| clone() 显式复制 | ✅ 待实现 |
| 所有类型可克隆 | ✅ 待实现 |
| Arc clone（引用计数增加） | ✅ 见 task-05-03 |
| Clone trait 实现检查 | ✅ 待实现 |

## 验收测试

```yaoxiang
# test_clone.yx

# === 基础 clone() 测试 ===
p: Point = Point(1.0, 2.0)
p2 = p.clone()
assert(p.x == 1.0)     # ✅ 原值可用
assert(p2.x == 1.0)    # ✅ 副本可用

p.x = 0.0
assert(p.x == 0.0)
assert(p2.x == 1.0)    # ✅ 互不影响

# === 函数参数复制 ===
process: (Point) -> Point = (p) => {
    other = p.clone()
    other.x = other.x + 10
    other
}

p: Point = Point(1.0, 2.0)
result = process(p.clone())
assert(p.x == 1.0)     # ✅ p 保留
assert(result.x == 11.0)

# === 集合操作 ===
data: List[Int] = [1, 2, 3]
doubled = data.map((x) => x.clone())
assert(data[0] == 1)
assert(doubled[0] == 1)
data[0] = 100
assert(doubled[0] == 1)  # ✅ 独立副本

# === Arc clone（引用计数）===
p: Point = Point(1.0, 2.0)
shared = ref p
shared2 = shared.clone()  # 引用计数增加

assert(shared.x == 1.0)
assert(shared2.x == 1.0)
# shared 和 shared2 释放后 p 才释放

print("clone() tests passed!")
```

## 相关文件

- **src/middle/lifetime/clone.rs**: CloneChecker 实现
- **src/middle/lifetime/error.rs**: Clone 错误类型定义
- **src/middle/lifetime/mod.rs**: OwnershipChecker 集成
- **src/middle/lifetime/ref_semantics.rs**: Ref/Arc 语义检查（参考）
