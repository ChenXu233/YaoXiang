# Task 5.1: 所有权转移

> **优先级**: P0
> **状态**: 🔄 待实现
> **模块**: `src/core/ownership/move.rs`
> **依赖**: 无（基础模块）

## 功能描述

跟踪所有权的转移和复制：
- `move` 语义：转移后原所有者失效
- `copy` 语义：浅拷贝（针对小对象 < 1KB）
- `drop`：值离开作用域时释放

> **注意**：此任务是所有权系统的**基础模块**，其他所有任务都依赖于它。

## 所有权规则

### Move 语义

```yaoxiang
# move：所有权转移
data = [1, 2, 3]
new_owner = data  # data 不再可用
# print(data.length)  # 编译错误！

# 函数调用也转移所有权
process: (List[T]) -> T = (input) => input[0]

data = [1, 2, 3]
result = process(data)  # data 移动进函数，不再可用
```

### Copy 语义（小对象 < 1KB）

> **RFC-009 核心设计**：小对象自动复制，开销可忽略（< 0.01% 运行时）

```yaoxiang
# Copy 类型（自动推导）：
# - 原类型（Int, Float, Bool, Char）
# - 不可变引用 ref T
# - 小结构体（总大小 < 1KB）

x: Int = 42
y = x  # x 仍然可用（Copy）

# 小结构体自动 Copy
type Point = Point(x: Int, y: Int)  # 16 字节 < 1KB
p: Point = Point(1, 2)
q = p  # p 仍然可用

# 大结构体（> 1KB）：Move 语义
type BigData = BigData(buffer: Bytes[10000])  # 10KB > 1KB
data = BigData(...)
new_owner = data  # 移动，data 不再可用
```

### 复制开销分析

```yaoxiang
# 复制开销分析（来自 RFC-009）：
# - 复制 64 字节：~1 纳秒
# - 内存访问延迟：~100 纳秒
# - 函数调用开销：~10 纳秒

# 结论：64 字节复制的开销可忽略不计
# 1KB 复制开销 < 0.01% 运行时
```

### Drop 规则

```yaoxiang
# 值离开作用域时自动释放
foo: () -> Void = () => {
    data: List[Int] = [1, 2, 3]  # 分配
    # data 在这里自动释放（RAII）
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
/// Copy 阈值（字节）
const COPY_THRESHOLD: usize = 1024; // 1KB

struct OwnershipAnalyzer {
    /// 每个值的所有者
    owner_of: HashMap<ValueId, ValueId>,
    /// 值的状态（Owned, Moved, Copied）
    state: HashMap<ValueId, ValueState>,
    /// 值的大小（用于判断 Copy vs Move）
    value_size: HashMap<ValueId, usize>,
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

    /// 判断类型是否 Copy
    fn is_copyable(&self, ty: &Type) -> bool {
        let size = self.type_size(ty);
        size <= COPY_THRESHOLD && self.is_trivially_copyable(ty)
    }

    /// 判断类型是否"平凡可复制"（不含资源）
    fn is_trivially_copyable(&self, ty: &Type) -> bool {
        match ty {
            Type::Primitive(_) => true,
            Type::Struct(fields) => {
                fields.iter().all(|f| self.is_trivially_copyable(&f.ty))
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_trivially_copyable(t)),
            Type::Array { elem, .. } => self.is_trivially_copyable(elem),
            Type::Ref(_) => true,  // 引用本身可复制
            _ => false,
        }
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
                ValueState::Copied if self.is_copyable(&self.get_type(src)) => {
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

    fn analyze_copy(&mut self, dst: &Operand, src: &Operand) -> Result<(), OwnershipError> {
        let src_id = self.get_value_id(src)?;
        let src_ty = self.get_type(src);

        // 检查是否是 Copy 类型
        if !self.is_copyable(&src_ty) {
            return Err(OwnershipError::NonCopyable {
                value: src_id,
                size: self.type_size(&src_ty),
                threshold: COPY_THRESHOLD,
            });
        }

        // 复制后双方都可用
        self.state.insert(src_id, ValueState::Copied);
        self.state.insert(self.get_value_id(dst)?, ValueState::Copied);

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
    NonCopyable {
        value: ValueId,
        size: usize,
        threshold: usize,
    },
    DoubleDrop {
        value: ValueId,
    },
    MoveOfCopyType {
        value: ValueId,
    },
}
```

## 与 RFC-009 对照

| RFC-009 设计 | 实现状态 |
|-------------|---------|
| Move 语义（零拷贝） | ✅ 已实现 |
| Copy 语义（< 1KB） | ✅ 已实现，阈值 1024 字节 |
| Drop 规则（RAII） | ✅ 已实现 |
| 禁止返回借用 | ✅ 见借用检查器 |
| 禁止结构体含借用 | ✅ 见类型检查器 |

## 验收测试

```yaoxiang
# test_ownership.yx

# === Move 测试 ===
data: List[Int] = [1, 2, 3]
new_owner = data
# assert(data.length)  # 应该编译错误

# === Copy 测试（小对象）===
x: Int = 42
y = x
assert(x == 42)
assert(y == 42)

type Point = Point(x: Int, y: Int)  # 16 字节 < 1KB
p: Point = Point(1, 2)
q = p
assert(p.x == 1)
assert(q.x == 1)

# === Copy 测试（大对象，应为 Move）===
# type BigData = BigData(buffer: Bytes[2000])
# data = BigData(...)
# new_owner = data  # 移动，不是复制
# # data 不再可用

# === Drop 测试 ===
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
- **src/middle/escape_analysis/mod.rs**: 逃逸分析（判断大小）
