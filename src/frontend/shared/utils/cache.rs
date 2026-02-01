//! 编译缓存
//!
//! 为 RFC-011 提供编译缓存机制

use std::collections::HashMap;
use std::hash::Hash;

/// 编译缓存
#[derive(Debug)]
pub struct CompilationCache<K, V> {
    /// 缓存存储
    cache: HashMap<K, V>,
    /// 最大缓存大小
    max_size: usize,
}

impl<K, V> CompilationCache<K, V>
where
    K: Hash + Eq + Clone,
{
    /// 创建新的缓存
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    /// 获取缓存项
    pub fn get(
        &self,
        key: &K,
    ) -> Option<&V> {
        self.cache.get(key)
    }

    /// 插入缓存项
    pub fn insert(
        &mut self,
        key: K,
        value: V,
    ) -> Option<V> {
        if self.cache.len() >= self.max_size {
            // 简单的淘汰策略：删除第一个元素
            let first_key = self.cache.keys().next().cloned();
            if let Some(k) = first_key {
                self.cache.remove(&k);
            }
        }
        self.cache.insert(key, value)
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}
