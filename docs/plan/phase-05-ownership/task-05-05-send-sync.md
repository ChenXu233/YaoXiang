# Task 5.5: Send/Sync 约束检查

> **优先级**: P1
> **状态**: 🔄 待实现
> **模块**: `src/core/lifetime/send_sync.rs`

## 功能描述

检查类型是否满足 Send/Sync 约束，确保并发安全：

- **Send**: 类型可以安全地跨线程**传输**（值传递）
- **Sync**: 类型可以安全地跨线程**共享引用**（共享访问）

> **注意**：YaoXiang 优先使用值传递，Sync 很少需要。

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
# ✅ Sync 类型（可以跨线程共享 Arc）
type Point = Point(x: Int, y: Int)

shared_point: Arc[Point] = Arc.new(Point(1, 2))  # ✅ 可以在线程间共享

# ⚠️ 注意：YaoXiang 很少需要共享引用
# 优先使用值传递 + clone()
```

## 自动实现规则

```rust
// 基本类型自动实现 Send + Sync
impl Send for Int {}
impl Sync for Int {}
impl Send for Float {}
impl Sync for Float {}
impl Send for Bool {}
impl Sync for Bool {}

// 结构体：如果所有字段都是 Send，则自动 Send
impl<T: Send> Send for Point<T> {}

// Sync 派生规则
impl<T: Sync> Sync for Point<T> {}

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
    /// 检查类型是否 Send
    fn is_send(&self, ty: &Type) -> bool {
        match ty {
            Type::Primitive(_) => true,
            Type::Struct(def_id) => {
                // 结构体自动 Send 如果所有字段都是 Send
                let fields = self.struct_fields(*def_id);
                fields.iter().all(|f| self.is_send(&f.ty))
            }
            Type::Tuple(types) => {
                types.iter().all(|t| self.is_send(t))
            }
            Type::Array { elem, .. } => self.is_send(elem),
            Type::Box(inner) => self.is_send(inner),
            Type::Arc(inner) => self.is_send(inner),
            Type::Rc(_) => false,  // Rc 不是 Send
            Type::RefCell(_) => false,
            Type::Mutex(inner) => self.is_send(inner),
            _ => false,
        }
    }

    /// 检查类型是否 Sync
    fn is_sync(&self, ty: &Type) -> bool {
        match ty {
            Type::Primitive(_) => true,
            Type::Struct(def_id) => {
                let fields = self.struct_fields(*def_id);
                fields.iter().all(|f| self.is_sync(&f.ty))
            }
            Type::Tuple(types) => {
                types.iter().all(|t| self.is_sync(t))
            }
            Type::Array { elem, .. } => self.is_sync(elem),
            Type::Box(inner) => self.is_send(inner) && self.is_sync(inner),
            Type::Arc(inner) => self.is_send(inner) && self.is_sync(inner),
            Type::Rc(_) => false,
            Type::RefCell(_) => false,
            Type::Mutex(inner) => self.is_send(inner),
            _ => false,
        }
    }

    /// 验证 spawn 表达式的安全性
    fn verify_spawn(&self, spawn: &SpawnExpr) -> Result<(), SendSyncError> {
        // 检查闭包参数
        for param in &spawn.closure.params {
            if !self.is_send(&param.ty) {
                return Err(SendSyncError::NonSendParameter {
                    param: param.name.clone(),
                    ty: param.ty.clone(),
                    span: param.span,
                });
            }
        }

        // 检查返回值
        if !self.is_send(&spawn.return_type) {
            return Err(SendSyncError::NonSendReturn {
                ty: spawn.return_type.clone(),
                span: spawn.return_span,
            });
        }

        // 检查闭包捕获的变量
        for captured in &spawn.closure.captured_vars {
            if !self.is_send(&captured.ty) {
                return Err(SendSyncError::NonSendCaptured {
                    value: captured.name.clone(),
                    ty: captured.ty.clone(),
                    span: captured.span,
                });
            }
        }

        Ok(())
    }

    /// 验证 channel 发送
    fn verify_channel_send(&self, send: &ChannelSend) -> Result<(), SendSyncError> {
        let value_ty = &send.value.ty;

        if !self.is_send(value_ty) {
            return Err(SendSyncError::NonSendValue {
                ty: value_ty.clone(),
                span: send.value.span,
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
        span: Span,
    },
    NonSendReturn {
        ty: Type,
        span: Span,
    },
    NonSendValue {
        ty: Type,
        span: Span,
    },
    NonSendCaptured {
        value: String,
        ty: Type,
        span: Span,
    },
    NonSyncShared {
        value: String,
        ty: Type,
        span: Span,
    },
}
```

## 标准库类型约束表

| 类型 | Send | Sync | 说明 |
|------|:----:|:----:|------|
| `Int`, `Float`, `Bool` | ✅ | ✅ | 原类型 |
| `String` | ✅ | ✅ | UTF-8 字符串 |
| `Box[T]` | ✅ | ❌ | 需要 T: Sync 才能 Sync |
| `Rc[T]` | ❌ | ❌ | 单线程引用计数 |
| `Arc[T]` | ✅ | ✅ | 原子引用计数（ref 关键字） |
| `Weak[T]` | ❌ | ✅ | 弱引用，不增加计数 |
| `RefCell[T]` | ❌ | ❌ | 运行时借用 |
| `Mutex[T]` | ✅ | ✅ | 线程安全互斥 |

## 与 RFC-009 v7 对照

| RFC-009 设计 | 实现状态 |
|-------------|---------|
| Send 约束检查 | ✅ 已实现 |
| Sync 约束检查 | ✅ 已实现 |
| spawn 参数/返回值检查 | ✅ 已实现 |
| 闭包捕获 Send 检查 | ✅ 已实现 |
| channel 发送 Send 检查 | ✅ 已实现 |

## 验收测试

```yaoxiang
# test_send_sync.yx

# === Send 测试 ===
type Point = Point(x: Int, y: Int)
spawn do_work(Point(1, 2))  # ✅ Point 是 Send

# === Sync 测试 ===
data: Point = Point(1, 2)
shared: Arc[Point] = ref data  # ✅ Arc[Point] 是 Sync
assert(shared.x == 1)

# === Arc 测试（线程安全引用）===
shared_count: Arc[Int] = Arc.new(0)
spawn increment(shared_count)  # ✅ Arc 是 Send + Sync

# === Rc 测试（应该编译错误）===
# type NonSend = NonSend(rc: Rc[Int])
# spawn do_work(NonSend(Rc.new(1)))  # ❌ Rc 不是 Send

# === RefCell 测试（应该编译错误）===
# type NonSync = NonSync(cell: RefCell[Int])
# shared: ref NonSync = ref NonSync(RefCell.new(0))  # ❌ RefCell 不是 Sync

print("Send/Sync tests passed!")
```

## 相关文件

- **src/core/ownership/send_sync.rs**: Send/Sync 检查器
- **src/core/ownership/errors.rs**: 错误定义
- **src/core/ownership/mod.rs**: 所有权检查器主模块
