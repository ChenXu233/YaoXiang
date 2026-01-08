# Task 5.2: 生命周期分析

> **优先级**: P0
> **状态**: 🔄 待实现

## 功能描述

跟踪引用的生命周期，确保引用不会超过其引用的值：
- 生命周期注解的推导
- 生命周期检查（借用不能超过所有者）
- 生命周期消除（根据使用场景推导）

## 生命周期规则

### 生命周期注解

```yaoxiang
# 显式生命周期注解
type Ref[T] = Ref[T](data: ref T)

# 多参数生命周期
type PairRef[T] = PairRef[T](a: ref T, b: ref T)
```

### 生命周期消除

```yaoxiang
# 省略生命周期注解（编译器推导）
first: [T](ref List[T]) -> ref T = (list) => {
    ref list[0]  # 自动推断生命周期
}

# 生命周期规则：
# 1. 参数生命周期自动推断
# 2. 返回值生命周期取参数中最短的
```

## 分析算法

```rust
struct LifetimeAnalyzer {
    /// 值的作用域
    scopes: Vec<Scope>,
    /// 生命周期约束
    constraints: Vec<LifetimeConstraint>,
    /// 推断的生命周期
    inferred: HashMap<RefId, Lifetime>,
}

impl LifetimeAnalyzer {
    /// 分析生命周期
    fn analyze(&mut self, func: &FunctionIR) -> LifetimeResult {
        // 1. 建立作用域
        self.build_scopes(func);

        // 2. 收集生命周期约束
        self.collect_constraints(func);

        // 3. 推断生命周期
        self.infer_lifetimes();

        // 4. 检查约束
        self.check_constraints()
    }

    /// 建立值的作用域
    fn build_scopes(&mut self, func: &FunctionIR) {
        for block in &func.blocks {
            for instr in &block.instructions {
                match instr {
                    Instruction::AllocLocal { id, .. } => {
                        self.scopes.last().unwrap().insert(*id);
                    }
                    _ => {}
                }
            }
        }
    }

    /// 收集生命周期约束
    fn collect_constraints(&mut self, func: &FunctionIR) {
        for instr in func.all_instructions() {
            match instr {
                Instruction::Borrow { owner, borrower, mutable } => {
                    // borrow 的生命周期 <= owner 的生命周期
                    self.constraints.push(LifetimeConstraint {
                        ref_id: *borrower,
                        owner_id: *owner,
                        relation: LifetimeRelation::Subtype,
                    });
                }
                _ => {}
            }
        }
    }
}
```

## 生命周期关系

```
'a: 'b     # 'a 的生命周期 >= 'b（'a 活得比 'b 久）
ref T: 'a  # T 的引用生命周期是 'a

# 消除规则
first: [T](ref List[T]) -> ref T
# 等价于
first: [T, 'a](list: ref 'a List[T]) -> ref 'a T
```

## 错误类型

```rust
#[derive(Debug, Clone)]
pub enum LifetimeError {
    BorrowOutlivesOwner {
        borrow: ValueId,
        owner: ValueId,
        borrow_scope: ScopeId,
        owner_scope: ScopeId,
    },
    LifetimeTooShort {
        ref_id: ValueId,
        required: Lifetime,
        found: Lifetime,
    },
    CycleInConstraints {
        constraints: Vec<LifetimeConstraint>,
    },
}
```

## 验收测试

```yaoxiang
# test_lifetime.yx

# 有效：借用不超过所有者
data: List[Int] = [1, 2, 3]
ref: ref Int = ref data[0]
assert(ref == 1)

# 有效：生命周期自动推断
get_first: [T](ref List[T]) -> ref T = (list) => {
    ref list[0]
}

# 无效：引用超过所有者生命周期
# dangling_ref: [T]() -> ref T = () => {
#     x: T = 42
#     ref x  # 编译错误！x 在函数返回后失效
# }

print("Lifetime tests passed!")
```

## 相关文件

- **src/core/lifetime/mod.rs**: 生命周期分析器
- **src/core/lifetime/infer.rs**: 生命周期推断
