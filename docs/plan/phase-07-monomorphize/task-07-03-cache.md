# Task 7.3: 实例缓存

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

缓存已生成的泛型实例，避免重复编译。

## 缓存策略

```rust
struct InstanceCache {
    /// 函数实例缓存
    fn_cache: HashMap<InstanceKey, FunctionId>,
    /// 类型实例缓存
    type_cache: HashMap<InstanceKey, TypeId>,
    /// 缓存文件路径
    cache_dir: PathBuf,
}
```

## 缓存管理

```rust
impl InstanceCache {
    /// 获取或创建实例
    pub fn get_or_create(&mut self, key: &InstanceKey) -> Result<Id, MonoError> {
        if let Some(id) = self.fn_cache.get(key) {
            return Ok(*id);
        }
        let id = self.instantiate(key)?;
        self.fn_cache.insert(key.clone(), id);
        Ok(id)
    }

    /// 保存缓存到磁盘
    pub fn save(&self) -> io::Result<()> { ... }

    /// 从磁盘加载缓存
    pub fn load(&mut self) -> io::Result<()> { ... }
}
```

## 相关文件

- **cache.rs**: InstanceCache
