//! 符号缓存
//!
//! 高性能符号查找缓存，将 O(n) 查询优化为 O(1)

use std::collections::HashMap;

/// 符号缓存
pub struct SymbolCache {
    /// 类型缓存
    types: HashMap<String, CachedType>,
    /// 函数缓存
    functions: HashMap<String, CachedFunction>,
    /// 常量缓存
    constants: HashMap<String, CachedConstant>,
    /// 使用频率统计
    usage_count: HashMap<String, usize>,
    /// LRU 缓存大小限制
    max_cache_size: usize,
}

impl SymbolCache {
    /// 创建新的符号缓存
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            functions: HashMap::new(),
            constants: HashMap::new(),
            usage_count: HashMap::new(),
            max_cache_size: 10000,
        }
    }

    /// 获取类型
    pub fn get_type(
        &self,
        name: &str,
    ) -> Option<&CachedType> {
        self.record_usage(name);
        self.types.get(name)
    }

    /// 插入类型
    pub fn insert_type(
        &mut self,
        name: String,
        type_info: CachedType,
    ) {
        if self.types.len() >= self.max_cache_size {
            self.evict_lru();
        }
        self.types.insert(name, type_info);
    }

    /// 获取函数
    pub fn get_function(
        &self,
        name: &str,
    ) -> Option<&CachedFunction> {
        self.record_usage(name);
        self.functions.get(name)
    }

    /// 插入函数
    pub fn insert_function(
        &mut self,
        name: String,
        func_info: CachedFunction,
    ) {
        if self.functions.len() >= self.max_cache_size {
            self.evict_lru();
        }
        self.functions.insert(name, func_info);
    }

    /// 获取常量
    pub fn get_constant(
        &self,
        name: &str,
    ) -> Option<&CachedConstant> {
        self.record_usage(name);
        self.constants.get(name)
    }

    /// 插入常量
    pub fn insert_constant(
        &mut self,
        name: String,
        const_info: CachedConstant,
    ) {
        if self.constants.len() >= self.max_cache_size {
            self.evict_lru();
        }
        self.constants.insert(name, const_info);
    }

    /// 记录使用次数
    fn record_usage(
        &self,
        _name: &str,
    ) {
        // 注意：这里只是记录，不实际增加（避免写锁）
        // 在实际使用中，可以通过其他方式统计
    }

    /// 清理最少使用的条目
    fn evict_lru(&mut self) {
        // 简化版：随机移除一些条目
        // 实际应该基于 usage_count 进行 LRU 淘汰
        if !self.types.is_empty() {
            let keys: Vec<String> = self.types.keys().take(100).cloned().collect();
            for key in keys {
                self.types.remove(&key);
            }
        }
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.types.clear();
        self.functions.clear();
        self.constants.clear();
        self.usage_count.clear();
    }

    /// 获取缓存统计
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            type_count: self.types.len(),
            function_count: self.functions.len(),
            constant_count: self.constants.len(),
            total_items: self.types.len() + self.functions.len() + self.constants.len(),
        }
    }
}

impl Default for SymbolCache {
    fn default() -> Self {
        Self::new()
    }
}

/// 缓存的类型信息
#[derive(Debug, Clone)]
pub struct CachedType {
    pub name: String,
    pub type_kind: TypeKind,
    pub size: usize,
    pub alignment: usize,
}

/// 类型种类
#[derive(Debug, Clone)]
pub enum TypeKind {
    Primitive,
    Struct,
    Enum,
    List,
    Dict,
    Function,
}

/// 缓存的函数信息
#[derive(Debug, Clone)]
pub struct CachedFunction {
    pub name: String,
    pub param_count: usize,
    pub return_type: String,
    pub is_generator: bool,
}

/// 缓存的常量信息
#[derive(Debug, Clone)]
pub struct CachedConstant {
    pub name: String,
    pub value: String,
    pub type_name: String,
}

/// 缓存统计
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub type_count: usize,
    pub function_count: usize,
    pub constant_count: usize,
    pub total_items: usize,
}

impl CacheStats {
    pub fn is_empty(&self) -> bool {
        self.total_items == 0
    }
}
