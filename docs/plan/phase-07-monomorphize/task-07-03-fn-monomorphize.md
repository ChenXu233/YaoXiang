# Task 7.3: 函数单态化

> **优先级**: P0
> **状态**: ⏳ 待实现
> **依赖**: task-07-01, task-07-02

## 功能描述

将泛型函数实例化为具体函数。

## 函数单态化示例

```yaoxiang
# 泛型函数
map:[T, U](List[T], (T) -> U) -> List[U] = (list, f) => {
    match list {
        List(head, tail) => List(f(head), map(tail, f)),
        empty => empty,
    }
}

# 单态化后
map_int_string:(List[Int], (Int) -> String) -> List[String] = (list, f) => {
    match list {
        List(head, tail) => List(f(head), map_int_string(tail, f)),
        empty => empty,
    }
}
```

## 函数单态化算法

```rust
impl MonoState {
    /// 单态化泛型函数
    pub fn monomorphize_fn(
        &mut self,
        generic_fn: GenericFunctionId,
        type_args: &[MonoType],
    ) -> Result<FunctionId, MonoError> {
        let key = InstanceKey::new(generic_fn, type_args.to_vec());

        // 检查缓存
        if let Some(id) = self.fn_instances.get(&key) {
            return Ok(*id);
        }

        // 创建单态化函数
        let mono_fn = self.instantiate_fn(generic_fn, type_args)?;

        // 缓存并返回
        self.fn_instances.insert(key, mono_fn.id);
        Ok(mono_fn.id)
    }

    /// 实例化函数
    fn instantiate_fn(
        &self,
        generic_fn: GenericFunctionId,
        type_args: &[MonoType],
    ) -> Result<MonoFunction, MonoError> {
        // 替换函数签名中的泛型参数
        let mono_signature = self.substitute_signature(&generic_fn.signature, type_args)?;

        // 替换函数体中的泛型参数
        let mono_body = self.substitute_body(&generic_fn.body, type_args)?;

        // 递归处理函数调用
        let mono_body = self.process_calls(mono_body, type_args)?;

        Ok(MonoFunction {
            id: self.next_fn_id(),
            name: format!("{}_{}", generic_fn.name, key.instance_name()),
            signature: mono_signature,
            body: mono_body,
        })
    }

    /// 替换签名中的泛型参数
    fn substitute_signature(&self, sig: &FnSignature, args: &[MonoType]) -> Result<FnSignature, MonoError> {
        Ok(FnSignature {
            params: sig.params.iter()
                .map(|(name, ty)| (name.clone(), self.substitute_type(ty, args)))
                .collect(),
            return_type: Box::new(self.substitute_type(&sig.return_type, args)?),
            is_async: sig.is_async,
        })
    }
}
```

## 相关文件

- **functions.rs**: 函数单态化逻辑
- **instantiate.rs**: Instantiator
