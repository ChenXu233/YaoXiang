# Task 7.2: 类型单态化

> **优先级**: P0
> **状态**: ⏳ 待实现
> **依赖**: task-07-01

## 功能描述

将泛型类型实例化为具体类型。

## 类型单态化示例

```yaoxiang
# 泛型类型定义
type Option[T] = some(T) | none
type List[T] = List(head: T, tail: List[T]) | empty

# 单态化后
type Option_Int = some(Int) | none
type Option_String = some(String) | none
type List_Int = List(head: Int, tail: List_Int) | empty
type List_String = List(head: String, tail: List_String) | empty
```

## 类型单态化算法

```rust
impl MonoState {
    /// 单态化泛型类型
    pub fn monomorphize_type(&mut self, generic_type: GenericTypeId, args: &[MonoType]) -> Result<TypeId, MonoError> {
        let key = InstanceKey::new(generic_type, args.to_vec());

        // 检查缓存
        if let Some(id) = self.type_instances.get(&key) {
            return Ok(*id);
        }

        // 创建单态化类型
        let mono_type = self.instantiate_type(generic_type, args)?;

        // 缓存并返回
        self.type_instances.insert(key, mono_type.id);
        Ok(mono_type.id)
    }

    /// 实例化类型
    fn instantiate_type(&self, generic_id: GenericTypeId, args: &[MonoType]) -> Result<MonoType, MonoError> {
        match generic_id {
            GenericTypeId::Enum(name, variants) => {
                // 单态化每个变体
                let mono_variants = variants.iter()
                    .map(|v| self.monomorphize_variant(v, args))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(MonoType::Enum(name.clone(), mono_variants))
            }
            GenericTypeId::Struct(name, fields) => {
                // 单态化每个字段
                let mono_fields = fields.iter()
                    .map(|(f, t)| (f.clone(), self.substitute_type(t, args)))
                    .collect();
                Ok(MonoType::Struct(name.clone(), mono_fields))
            }
        }
    }

    /// 替换类型中的泛型参数
    fn substitute_type(&self, ty: &MonoType, args: &[MonoType]) -> MonoType {
        match ty {
            MonoType::GenericParam(index) => args[*index].clone(),
            MonoType::Instance(generic_id, type_args) => {
                let substituted = type_args.iter()
                    .map(|t| self.substitute_type(t, args))
                    .collect();
                MonoType::Instance(*generic_id, substituted)
            }
            _ => ty.clone(),
        }
    }
}
```

## 相关文件

- **types.rs**: 类型单态化逻辑
- **instantiate.rs**: Instantiator
