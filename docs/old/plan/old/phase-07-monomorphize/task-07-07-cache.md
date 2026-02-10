# Task 7.7: 实例缓存

> **优先级**: P1
> **状态**: ⏳ 待实现
> **依赖**: task-07-03, task-07-04, task-07-05

## 功能描述

缓存已生成的泛型实例，避免重复编译。

## 缓存结构

```rust
struct InstanceCache {
    /// 函数实例缓存
    fn_cache: HashMap<InstanceKey, FunctionId>,
    /// 类型实例缓存
    type_cache: HashMap<InstanceKey, TypeId>,
    /// Send/Sync 约束缓存
    constraint_cache: HashMap<MonoType, (bool, bool)>,
    /// 磁盘缓存目录
    cache_dir: PathBuf,
    /// 缓存版本（用于失效）
    version: u32,
}

struct DiskCache {
    /// 函数实例映射
    fn_map: HashMap<String, InstanceKey>,
    /// 类型实例映射
    type_map: HashMap<String, InstanceKey>,
    /// 序列化的实例数据
    instances: HashMap<String, Vec<u8>>,
}
```

## 缓存操作

```rust
impl InstanceCache {
    /// 获取或创建函数实例
    pub fn get_or_create_fn(
        &mut self,
        key: &InstanceKey,
        instantiator: impl FnOnce() -> Result<FunctionId, MonoError>,
    ) -> Result<FunctionId, MonoError> {
        // 检查内存缓存
        if let Some(id) = self.fn_cache.get(key) {
            return Ok(*id);
        }

        // 检查磁盘缓存
        if let Some(id) = self.load_from_disk(key)? {
            self.fn_cache.insert(key.clone(), id);
            return Ok(id);
        }

        // 创建新实例
        let id = instantiator()?;
        self.fn_cache.insert(key.clone(), id);
        self.save_to_disk(key, id)?;

        Ok(id)
    }

    /// 保存缓存到磁盘
    fn save_to_disk(&self, key: &InstanceKey, id: FunctionId) -> io::Result<()> {
        let path = self.cache_dir.join("fn").join(key.instance_name());
        let data = bincode::serialize(&(key, id))?;
        std::fs::write(path, data)
    }

    /// 从磁盘加载缓存
    fn load_from_disk(&self, key: &InstanceKey) -> Result<Option<FunctionId>, MonoError> {
        let path = self.cache_dir.join("fn").join(key.instance_name());
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read(path)?;
        let (_, id): (InstanceKey, FunctionId) = bincode::deserialize(&data)?;
        Ok(Some(id))
    }

    /// 使缓存失效
    pub fn invalidate(&mut self) {
        self.fn_cache.clear();
        self.type_cache.clear();
        self.constraint_cache.clear();
        self.clear_disk_cache();
    }
}
```

## 缓存失效策略

| 触发条件 | 失效范围 |
|----------|----------|
| 类型定义改变 | 所有相关实例 |
| 函数签名改变 | 所有相关实例 |
| 编译器版本改变 | 所有缓存 |
| 手动触发 | 可指定范围 |

## 相关文件

- **cache.rs**: InstanceCache
- **disk_cache.rs**: 磁盘缓存
