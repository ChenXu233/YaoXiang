# Task 7.8: 错误处理

> **优先级**: P2
> **状态**: ⏳ 待实现
> **依赖**: task-07-01 ~ task-07-07

## 功能描述

单态化过程中的错误诊断和报告。

## 错误类型

```rust
enum MonoError {
    /// 泛型参数数量不匹配
    GenericArgCountMismatch {
        expected: usize,
        actual: usize,
        generic_name: String,
    },

    /// 类型参数不满足约束
    TypeConstraintViolation {
        type_param: String,
        actual_type: MonoType,
        expected_constraint: String,
    },

    /// 无限实例化（递归类型）
    InfiniteInstantiation {
        generic_name: String,
        type_args: Vec<MonoType>,
        depth: usize,
    },

    /// 跨模块类型不可见
    TypeNotVisible {
        type_name: String,
        module: ModuleId,
        definition_module: ModuleId,
    },

    /// 实例数量超出限制
    InstanceLimitExceeded {
        generic_name: String,
        current_count: usize,
        limit: usize,
    },

    /// Send/Sync 约束失败
    SendSyncViolation {
        type_name: MonoType,
        context: String,
        required_constraint: String,
    },
}
```

## 错误报告

```rust
impl MonoError {
    /// 生成人类可读的错误信息
    pub fn report(&self, source_map: &SourceMap) -> Diagnostic {
        match self {
            MonoError::GenericArgCountMismatch { expected, actual, generic_name } => {
                Diagnostic::error()
                    .with_message(format!(
                        "generic `{}` expects {} type arguments, but {} were given",
                        generic_name, expected, actual
                    ))
                    .with_code("E0000")
            }
            MonoError::TypeConstraintViolation { type_param, actual_type, expected_constraint } => {
                Diagnostic::error()
                    .with_message(format!(
                        "type `{}` does not satisfy the `{}` constraint",
                        actual_type, expected_constraint
                    ))
                    .with_note(format!("type parameter `{}` has this constraint", type_param))
            }
            MonoError::InfiniteInstantiation { generic_name, type_args, depth } => {
                Diagnostic::error()
                    .with_message(format!(
                        "infinite instantiation detected for `{}`",
                        generic_name
                    ))
                    .with_note(format!("recursion depth: {}", depth))
                    .with_help("consider adding a type constraint to break the recursion")
            }
            _ => Diagnostic::error().with_message(format!("{:?}", self)),
        }
    }
}
```

## 错误恢复

```rust
impl MonoState {
    /// 单态化并处理错误
    pub fn monomorphize_with_error_handling(
        &mut self,
        generic_id: GenericId,
        type_args: &[MonoType],
    ) -> Result<Id, MonoError> {
        // 设置超时/深度限制
        let depth = self.instantiation_depth.insert(0);

        let result = std::panic::catch_unwind(|| {
            self.monomorphize(generic_id, type_args)
        });

        self.instantiation_depth.remove();

        match result {
            Ok(Ok(id)) => Ok(id),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(MonoError::InfiniteInstantiation {
                generic_name: generic_id.name(),
                type_args: type_args.to_vec(),
                depth: depth + 1,
            }),
        }
    }
}
```

## 相关文件

- **error.rs**: MonoError 定义
- **diagnostics.rs**: 错误报告
