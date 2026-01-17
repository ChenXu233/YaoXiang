# Task 5.4: clone() 显式复制

> **优先级**: P0
> **状态**: 🔄 待实现
> **模块**: `src/core/ownership/clone.rs`

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

```rust
struct CloneAnalyzer {
    /// clone() 调用
    clone_calls: Vec<CloneCall>,
    /// 需要检查的 clone 上下文
    contexts: Vec<CloneContext>,
    /// 错误
    errors: Vec<CloneError>,
}

impl CloneAnalyzer {
    /// 分析 clone() 调用
    fn analyze_clone(&mut self, call: &MethodCall) -> Result<(), CloneError> {
        let receiver = &call.receiver;
        let receiver_id = self.get_value_id(receiver)?;

        // 检查接收者状态
        match self.get_value_state(receiver_id) {
            ValueState::Moved => {
                return Err(CloneError::CloneMovedValue {
                    value: receiver_id,
                    span: call.span,
                });
            }
            ValueState::Dropped => {
                return Err(CloneError::CloneDroppedValue {
                    value: receiver_id,
                    span: call.span,
                });
            }
            ValueState::Owned => {
                // 正常 clone()
            }
        }

        // 检查类型是否可克隆
        let ty = self.get_type(receiver_id);
        if !self.is_cloneable(&ty) {
            return Err(CloneError::NonCloneableType {
                ty,
                span: call.span,
            });
        }

        // 记录 clone 调用
        self.clone_calls.push(CloneCall {
            id: self.get_value_id(&call.result)?,
            receiver: receiver_id,
            span: call.span,
        });

        // clone() 后原值仍然可用（双方都是 Owned）
        self.value_states.insert(receiver_id, ValueState::Owned);

        Ok(())
    }

    /// 检查类型是否可克隆
    fn is_cloneable(&self, ty: &Type) -> bool {
        match ty {
            // 基础类型都可克隆
            Type::Primitive(_) => true,
            // 结构体需要所有字段都可克隆
            Type::Struct(s) => s.fields.iter().all(|f| self.is_cloneable(&f.ty)),
            // 元组
            Type::Tuple(ts) => ts.iter().all(|t| self.is_cloneable(t)),
            // 数组
            Type::Array { elem, .. } => self.is_cloneable(elem),
            // Arc 可克隆（增加引用计数）
            Type::Arc(_) => true,
            // 其他类型需要检查是否实现 Clone trait
            _ => self.implements_trait(ty, "Clone"),
        }
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum CloneError {
    /// clone 已移动的值
    CloneMovedValue {
        value: ValueId,
        span: Span,
    },
    /// clone 已释放的值
    CloneDroppedValue {
        value: ValueId,
        span: Span,
    },
    /// 类型不可克隆
    NonCloneableType {
        ty: Type,
        span: Span,
    },
    /// 缺少 clone 方法
    MissingCloneMethod {
        ty: Type,
        span: Span,
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

- **src/core/ownership/clone.rs**: clone() 分析
- **src/core/ownership/ref.rs**: Arc clone 实现
- **src/core/traits/mod.rs**: Clone trait 定义
