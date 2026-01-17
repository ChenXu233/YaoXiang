# Task 7.4: Send/Sync 特化

> **优先级**: P0
> **状态**: ⏳ 待实现
> **依赖**: task-07-02, task-07-03

## 功能描述

根据 Send/Sync 约束生成特化版本。

## Send/Sync 特化示例

```yaoxiang
# 泛型类型
type Box[T] = Box(value: T)

# Send/Sync 派生规则
# Box[T]: Send ⇐ T: Send
# Box[T]: Sync ⇐ T: Sync

# 特化生成
# Box[ref Int] → Send + Sync（Arc 线程安全）
# Box[*Int] → 不 Send，不 Sync（裸指针）
```

## Send/Sync 推导算法

```rust
impl MonoState {
    /// 推导 MonoType 的 Send 约束
    pub fn is_send(&mut self, ty: &MonoType) -> bool {
        if let Some(result) = self.constraint_cache.get(ty) {
            return result.0;
        }

        let result = match ty {
            MonoType::Concrete(id) => {
                let type_info = self.type_table.get(*id);
                type_info.is_send
            }
            MonoType::Instance(generic_id, args) => {
                // 泛型类型的 Send 推导
                self.derive_send(generic_id, args)
            }
            MonoType::Fn { params, return_type, .. } => {
                // 函数类型的 Send 要求所有参数和返回值都 Send
                params.iter().all(|p| self.is_send(p)) && self.is_send(return_type)
            }
            MonoType::Ref(inner) => {
                // ref T 总是 Send + Sync（Arc）
                true
            }
        };

        self.constraint_cache.insert(ty.clone(), (result, result));
        result
    }

    /// 推导泛型类型的 Send 约束
    fn derive_send(&self, generic_id: GenericId, args: &[MonoType]) -> bool {
        match generic_id {
            GenericId::Box => {
                // Box[T]: Send ⇐ T: Send
                args.iter().all(|t| self.is_send(t))
            }
            GenericId::Vec => {
                // Vec[T]: Send ⇐ T: Send
                args.iter().all(|t| self.is_send(t))
            }
            GenericId::Mutex => {
                // Mutex[T]: Send ⇐ T: Send（内部可变性）
                args.iter().all(|t| self.is_send(t))
            }
            GenericId::Arc => {
                // Arc[T]: Send ⇐ T: Send
                args.iter().all(|t| self.is_send(t))
            }
        }
    }
}
```

## 特化版本生成

```rust
/// 根据 Send/Sync 约束生成特化版本
impl MonoState {
    /// 为跨线程使用生成特化版本
    pub fn specialize_for_send(
        &mut self,
        generic_fn: GenericFunctionId,
        type_args: &[MonoType],
    ) -> Result<FunctionId, MonoError> {
        // 检查是否需要 Send 特化
        let needs_send = type_args.iter().any(|t| !self.is_send(t));

        if needs_send {
            // 需要生成 Send 版本（可能需要 Arc 包装）
            self.generate_send_wrapper(generic_fn, type_args)
        } else {
            // 直接使用原版本
            self.monomorphize_fn(generic_fn, type_args)
        }
    }
}
```

## 相关文件

- **constraints.rs**: Send/Sync 推导
- **specialize.rs**: 特化逻辑
