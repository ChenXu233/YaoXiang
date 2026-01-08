# Task 5.1: 借用检查

> **优先级**: P0
> **状态**: 🔄 待实现

## 功能描述

检查借用是否满足语言规则：
- 不可变引用（`ref T`）可以同时存在多个
- 可变引用（`mut T`）只能存在一个
- 借用不能超过所有者的生命周期

## 借用规则

### 不可变借用

```yaoxiang
# ✅ 多个不可变引用同时存在
data: List[Int] = [1, 2, 3]
ref1: ref List[Int] = ref data
ref2: ref List[Int] = ref data
print(ref1.length)  # 可以读取
```

### 可变借用

```yaoxiang
# ✅ 只有一个可变引用
mut_data: List[Int] = [1, 2, 3]
ref: mut List[Int] = mut mut_data
ref.push(4)

# ❌ 不能同时存在多个可变引用
mut_data: List[Int] = [1, 2, 3]
ref1: mut List[Int] = mut mut_data
# ref2: mut List[Int] = mut mut_data  # 编译错误！

# ❌ 不可变和可变不能同时存在
data: List[Int] = [1, 2, 3]
ref1: ref List[Int] = ref data
# ref2: mut List[Int] = mut data  # 编译错误！
```

## 检查算法

```rust
struct BorrowChecker {
    /// 借用图：谁借用了我
    borrows: HashMap<ValueId, Vec<ValueId>>,
    /// 活跃的可变借用
    mutable_borrows: HashSet<ValueId>,
    /// 借用检查错误
    errors: Vec<BorrowError>,
}

impl BorrowChecker {
    /// 检查借用表达式
    fn check_borrow(&mut self, expr: &Expr) -> Result<(), BorrowError> {
        match expr {
            Expr::Borrow { mutable, owner, borrower } => {
                if *mutable {
                    self.check_mutable_borrow(owner, borrower)
                } else {
                    self.check_immutable_borrow(owner, borrower)
                }
            }
            _ => Ok(())
        }
    }

    fn check_mutable_borrow(
        &mut self,
        owner: &ValueId,
        borrower: &ValueId,
    ) -> Result<(), BorrowError> {
        // 检查是否已有可变借用
        if self.mutable_borrows.contains(owner) {
            return Err(BorrowError::AlreadyMutablyBorrowed {
                owner: *owner,
            });
        }

        // 检查是否已有不可变借用
        if let Some(borrowers) = self.borrows.get(owner) {
            if !borrowers.is_empty() {
                return Err(BorrowError::AlreadyBorrowed {
                    owner: *owner,
                    borrowers: borrowers.clone(),
                });
            }
        }

        self.mutable_borrows.insert(*owner);
        Ok(())
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone)]
pub enum BorrowError {
    AlreadyMutablyBorrowed {
        owner: ValueId,
    },
    AlreadyBorrowed {
        owner: ValueId,
        borrowers: Vec<ValueId>,
    },
    MutablyBorrowed {
        owner: ValueId,
    },
    BorrowedOutOfScope {
        value: ValueId,
    },
}
```

## 验收测试

```yaoxiang
# test_borrow_check.yx

# 不可变借用测试
data: List[Int] = [1, 2, 3]
ref1: ref List[Int] = ref data
ref2: ref List[Int] = ref data
assert(ref1.length == ref2.length)

# 可变借用测试
mut_data: List[Int] = [1, 2, 3]
ref: mut List[Int] = mut mut_data
ref.push(4)
assert(mut_data.length == 4)

print("Borrow check tests passed!")
```

## 相关文件

- **src/core/ownership/borrow.rs**: 借用检查器
- **src/core/ownership/errors.rs**: 错误定义
