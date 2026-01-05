# Task 13.3: 代码缓存

> **优先级**: P2
> **状态**: ⏳ 待实现

## 功能描述

缓存已编译的机器码，避免重复编译。

## 代码缓存

```rust
/// JIT 代码缓存
struct JITCodeCache {
    /// 缓存映射
    cache: HashMap<FunctionId, CompiledCode>,
    /// 缓存大小限制
    max_size: usize,
    /// 当前缓存大小
    current_size: usize,
    /// 内存映射
    memory_map: MemoryMapping,
}

impl JITCodeCache {
    /// 获取或编译函数
    pub fn get_or_compile(&mut self, fn_id: FunctionId, bytecode: &FunctionBytecode) -> &CompiledCode {
        if let Some(code) = self.cache.get(&fn_id) {
            return code;
        }

        let code = self.compile(bytecode);
        self.cache.insert(fn_id, code);
        code
    }

    /// 使缓存失效
    pub fn invalidate(&mut self, fn_id: FunctionId) {
        if let Some(code) = self.cache.remove(&fn_id) {
            self.current_size -= code.size();
        }
    }
}
```

## 相关文件

- **cache.rs**: JITCodeCache
