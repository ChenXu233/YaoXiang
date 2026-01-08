# Task 5.4: Send/Sync 约束检查

> **优先级**: P0
> **状态**: 🔄 待实现

## 功能描述

检查类型是否满足 Send/Sync 约束，确保并发安全：

- **Send**: 类型可以安全地跨线程传输（移动所有权）
- **Sync**: 类型可以安全地跨线程共享（共享引用）

## Send/Sync 规则

### Send 约束

```yaoxiang
# ✅ Send 类型（可以 spawn）
type Point = Point(x: Int, y: Int)  # Int 是 Send

spawn process_point(Point(1, 2))    # ✅ Point 可以跨线程传输

# ❌ 非 Send 类型
type NonSend = NonSend(rc: Rc[Int])  # Rc 不是 Send

spawn process_non_send(NonSend(rc))  # ❌ 编译错误！
```

### Sync 约束

```yaoxiang
# ✅ Sync 类型（可以跨线程共享）
type Point = Point(x: Int, y: Int)  # ref Point 是 Sync

shared_point: ref Point = ref Point(1, 2)  # ✅ 可以在线程间共享

# ❌ 非 Sync 类型
type NonSync = NonSync(cell: RefCell[Int])

shared_non_sync: ref NonSync = ref NonSync(cell)  # ❌ 编译错误！
```

## Send/Sync 自动实现

```rust
// 基本类型自动实现 Send + Sync
impl Send for Int {}
impl Sync for Int {}
impl Send for Float {}
impl Sync for Float {}

// 结构体：如果所有字段都是 Send，则自动 Send
impl<T: Send> Send for Point<T> {}

// 引用：&T 自动实现 Sync（如果 T: Sync）
impl<T: Sync> Sync for &T {}

// Box: Send（如果 T: Send）
impl<T: Send> Send for Box<T> {}

// Arc: Send + Sync（如果 T: Send + Sync）
impl<T: Send + Sync> Send for Arc<T> {}
impl<T: Send + Sync> Sync for Arc<T> {}

// Rc: 既不是 Send 也不是 Sync
impl<T> !Send for Rc<T> {}
impl<T> !Sync for Rc<T> {}
```

## 检查算法

```rust
struct SendSyncChecker {
    /// Send 类型集合
    send_types: HashSet<TypeId>,
    /// Sync 类型集合
    sync_types: HashSet<TypeId>,
    /// 发现的约束错误
    errors: Vec<SendSyncError>,
}

impl SendSyncChecker {
    /// 检查函数参数是否 Send
    fn check_send(&self, ty: &Type) -> bool {
        match ty {
            Type::Primitive(_) => true,
            Type::Struct(fields) => {
                fields.iter().all(|f| self.check_send(&f.ty))
            }
            Type::Box(inner) => self.check_send(inner),
            Type::Arc(inner) => self.check_send(inner),
            Type::Rc(_) => false,  // Rc 不是 Send
            Type::Ref(inner) => self.check_send(inner),
            _ => false,
        }
    }

    /// 检查类型是否 Sync
    fn check_sync(&self, ty: &Type) -> bool {
        match ty {
            Type::Primitive(_) => true,
            Type::Struct(fields) => {
                fields.iter().all(|f| self.check_sync(&f.ty))
            }
            Type::Ref(inner) => self.check_sync(inner),
            Type::Arc(inner) => self.check_sync(inner),
            Type::Mutex(inner) => self.check_send(inner),
            Type::Rc(_) => false,
            Type::RefCell(_) => false,
            _ => false,
        }
    }

    /// 验证 spawn 表达式的安全性
    fn verify_spawn(&self, spawn: &SpawnExpr) -> Result<(), SendSyncError> {
        // 检查闭包参数
        for param in &spawn.closure.params {
            if !self.check_send(&param.ty) {
                return Err(SendSyncError::NonSendParameter {
                    param: param.name.clone(),
                    ty: param.ty.clone(),
                });
            }
        }

        // 检查返回值
        if !self.check_send(&spawn.return_type) {
            return Err(SendSyncError::NonSendReturn {
                ty: spawn.return_type.clone(),
            });
        }

        Ok(())
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone)]
pub enum SendSyncError {
    NonSendParameter {
        param: String,
        ty: Type,
    },
    NonSendReturn {
        ty: Type,
    },
    NonSyncShared {
        value: ValueId,
        ty: Type,
    },
    NonSendInClosure {
        captured: ValueId,
        ty: Type,
    },
}
```

## 验收测试

```yaoxiang
# test_send_sync.yx

# Send 测试
type Point = Point(x: Int, y: Int)
spawn do_work(Point(1, 2))  # ✅ Point 是 Send

# Sync 测试
data: Point = Point(1, 2)
shared: ref Point = ref data  # ✅ ref Point 是 Sync
assert(shared.x == 1)

# Arc 测试（线程安全引用）
shared_count: Arc[Int] = Arc.new(0)
spawn increment(shared_count)  # ✅ Arc 是 Send + Sync

# Rc 测试（应该编译错误）
# type NonSend = NonSend(rc: Rc[Int])
# spawn do_work(NonSend(Rc.new(1)))  # ❌ Rc 不是 Send

print("Send/Sync tests passed!")
```

## 相关文件

- **src/core/ownership/send_sync.rs**: Send/Sync 检查器
- **src/core/ownership/errors.rs**: 错误定义
