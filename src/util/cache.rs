//! 文档缓存系统
//!
//! 为 LSP 和增量编译提供文档级缓存管理。
//!
//! # 设计
//!
//! - `DocumentCache` 管理单个文档的缓存状态
//! - `DocumentStore` 管理所有打开文档的缓存
//! - 使用内容哈希快速检测文档是否变化
//! - 缓存 AST 避免重复解析

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::frontend::core::parser::ast::Module;

/// 文档缓存
///
/// 管理单个文档的版本、内容和缓存的 AST。
/// LSP 的 `textDocument/didOpen` 和 `textDocument/didChange` 使用此结构
/// 跟踪文档状态。
#[derive(Debug)]
pub struct DocumentCache {
    /// LSP 文档版本号
    version: u32,
    /// 当前文档内容
    content: String,
    /// 内容哈希（用于快速比较文档是否变化）
    content_hash: u64,
    /// 缓存的 AST（解析后的模块，None 表示需要重新解析）
    ast: Option<Module>,
    /// 文件路径（URI）
    file_path: String,
    /// 是否有未保存的修改
    dirty: bool,
}

impl DocumentCache {
    /// 创建新的文档缓存
    pub fn new(
        file_path: String,
        content: String,
        version: u32,
    ) -> Self {
        let content_hash = Self::compute_hash(&content);
        Self {
            version,
            content,
            content_hash,
            ast: None,
            file_path,
            dirty: false,
        }
    }

    /// 获取文档版本号
    pub fn version(&self) -> u32 {
        self.version
    }

    /// 获取文档内容
    pub fn content(&self) -> &str {
        &self.content
    }

    /// 获取内容哈希
    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    /// 获取文件路径
    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    /// 获取缓存的 AST
    pub fn ast(&self) -> Option<&Module> {
        self.ast.as_ref()
    }

    /// 设置缓存的 AST
    pub fn set_ast(
        &mut self,
        ast: Module,
    ) {
        self.ast = Some(ast);
    }

    /// 清除缓存的 AST（标记需要重新解析）
    pub fn invalidate_ast(&mut self) {
        self.ast = None;
    }

    /// 检查是否有缓存的 AST
    pub fn has_cached_ast(&self) -> bool {
        self.ast.is_some()
    }

    /// 检查是否有未保存的修改
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 更新文档内容
    ///
    /// 如果内容实际发生变化（哈希不同），则：
    /// 1. 更新内容和哈希
    /// 2. 递增版本号
    /// 3. 使 AST 缓存失效
    /// 4. 标记为 dirty
    ///
    /// 返回 `true` 表示内容确实发生了变化。
    pub fn update(
        &mut self,
        new_content: String,
        new_version: u32,
    ) -> bool {
        let new_hash = Self::compute_hash(&new_content);
        if new_hash == self.content_hash && self.content == new_content {
            // 内容未变化，仅更新版本号
            self.version = new_version;
            return false;
        }

        self.content = new_content;
        self.content_hash = new_hash;
        self.version = new_version;
        self.ast = None; // 使 AST 缓存失效
        self.dirty = true;
        true
    }

    /// 标记为已保存
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// 检查内容是否与给定内容相同（通过哈希快速比较）
    pub fn content_matches(
        &self,
        other_content: &str,
    ) -> bool {
        let other_hash = Self::compute_hash(other_content);
        self.content_hash == other_hash && self.content == other_content
    }

    /// 获取文档行数
    pub fn line_count(&self) -> usize {
        self.content.lines().count().max(1)
    }

    /// 计算字符串的哈希值
    fn compute_hash(content: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

/// 文档存储
///
/// 管理所有打开的文档的缓存，提供 LSP 需要的文档管理功能。
/// 使用文件路径（URI）作为 key。
#[derive(Debug, Default)]
pub struct DocumentStore {
    /// 已打开的文档: file_path → DocumentCache
    documents: HashMap<String, DocumentCache>,
    /// 最大缓存文档数（超过时清理最旧的）
    max_documents: usize,
}

impl DocumentStore {
    /// 创建新的文档存储
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            max_documents: 128, // 默认最多缓存 128 个文档
        }
    }

    /// 创建带容量限制的文档存储
    pub fn with_capacity(max_documents: usize) -> Self {
        Self {
            documents: HashMap::new(),
            max_documents,
        }
    }

    /// 打开文档（创建缓存）
    ///
    /// 对应 LSP 的 `textDocument/didOpen`
    pub fn open(
        &mut self,
        file_path: String,
        content: String,
        version: u32,
    ) {
        let cache = DocumentCache::new(file_path.clone(), content, version);
        self.documents.insert(file_path, cache);

        // 如果超过容量限制，清理
        if self.documents.len() > self.max_documents {
            self.cleanup();
        }
    }

    /// 更新文档内容
    ///
    /// 对应 LSP 的 `textDocument/didChange`
    /// 返回 `true` 表示内容发生了变化
    pub fn update(
        &mut self,
        file_path: &str,
        content: String,
        version: u32,
    ) -> bool {
        if let Some(cache) = self.documents.get_mut(file_path) {
            cache.update(content, version)
        } else {
            // 文档未打开，自动打开
            self.open(file_path.to_string(), content, version);
            true
        }
    }

    /// 关闭文档（移除缓存）
    ///
    /// 对应 LSP 的 `textDocument/didClose`
    pub fn close(
        &mut self,
        file_path: &str,
    ) -> Option<DocumentCache> {
        self.documents.remove(file_path)
    }

    /// 获取文档缓存
    pub fn get(
        &self,
        file_path: &str,
    ) -> Option<&DocumentCache> {
        self.documents.get(file_path)
    }

    /// 获取文档缓存（可变）
    pub fn get_mut(
        &mut self,
        file_path: &str,
    ) -> Option<&mut DocumentCache> {
        self.documents.get_mut(file_path)
    }

    /// 检查文档是否已打开
    pub fn is_open(
        &self,
        file_path: &str,
    ) -> bool {
        self.documents.contains_key(file_path)
    }

    /// 获取所有已打开的文档路径
    pub fn open_files(&self) -> Vec<&str> {
        self.documents.keys().map(|s| s.as_str()).collect()
    }

    /// 获取所有已打开文档的迭代器
    ///
    /// 返回 (uri, DocumentCache) 的引用对。
    pub fn all_documents(&self) -> impl Iterator<Item = (&str, &DocumentCache)> {
        self.documents.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// 获取已打开的文档数量
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// 清理缓存（移除版本号最小的文档以腾出空间）
    fn cleanup(&mut self) {
        if self.documents.len() <= self.max_documents {
            return;
        }

        let to_remove = self.documents.len() - self.max_documents;
        let mut entries: Vec<(String, u32)> = self
            .documents
            .iter()
            .map(|(path, cache)| (path.clone(), cache.version()))
            .collect();
        entries.sort_by_key(|(_path, version)| *version);

        for (path, _) in entries.into_iter().take(to_remove) {
            self.documents.remove(&path);
        }
    }

    /// 清空所有缓存
    pub fn clear(&mut self) {
        self.documents.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_cache_new() {
        let cache = DocumentCache::new("test.yx".to_string(), "x = 42".to_string(), 1);
        assert_eq!(cache.version(), 1);
        assert_eq!(cache.content(), "x = 42");
        assert_eq!(cache.file_path(), "test.yx");
        assert!(!cache.has_cached_ast());
        assert!(!cache.is_dirty());
    }

    #[test]
    fn test_document_cache_update_changed() {
        let mut cache = DocumentCache::new("test.yx".to_string(), "x = 42".to_string(), 1);
        let changed = cache.update("x = 43".to_string(), 2);
        assert!(changed);
        assert_eq!(cache.version(), 2);
        assert_eq!(cache.content(), "x = 43");
        assert!(cache.is_dirty());
        assert!(!cache.has_cached_ast());
    }

    #[test]
    fn test_document_cache_update_unchanged() {
        let mut cache = DocumentCache::new("test.yx".to_string(), "x = 42".to_string(), 1);
        let changed = cache.update("x = 42".to_string(), 2);
        assert!(!changed);
        assert_eq!(cache.version(), 2);
    }

    #[test]
    fn test_document_cache_hash_detection() {
        let cache = DocumentCache::new("test.yx".to_string(), "hello world".to_string(), 1);
        assert!(cache.content_matches("hello world"));
        assert!(!cache.content_matches("hello world!"));
    }

    #[test]
    fn test_document_store_open_close() {
        let mut store = DocumentStore::new();
        store.open("a.yx".to_string(), "x = 1".to_string(), 1);
        store.open("b.yx".to_string(), "y = 2".to_string(), 1);
        assert_eq!(store.document_count(), 2);
        assert!(store.is_open("a.yx"));

        store.close("a.yx");
        assert_eq!(store.document_count(), 1);
        assert!(!store.is_open("a.yx"));
    }

    #[test]
    fn test_document_store_update() {
        let mut store = DocumentStore::new();
        store.open("a.yx".to_string(), "x = 1".to_string(), 1);

        let changed = store.update("a.yx", "x = 2".to_string(), 2);
        assert!(changed);
        assert_eq!(store.get("a.yx").unwrap().content(), "x = 2");
        assert_eq!(store.get("a.yx").unwrap().version(), 2);
    }

    #[test]
    fn test_document_store_cleanup() {
        let mut store = DocumentStore::with_capacity(2);
        store.open("a.yx".to_string(), "1".to_string(), 1);
        store.open("b.yx".to_string(), "2".to_string(), 2);
        store.open("c.yx".to_string(), "3".to_string(), 3);

        // 容量为 2，应该清理版本号最小的
        assert_eq!(store.document_count(), 2);
        assert!(!store.is_open("a.yx")); // 版本号最低被清理
    }

    #[test]
    fn test_document_cache_ast_invalidation() {
        let mut cache = DocumentCache::new("test.yx".to_string(), "x = 42".to_string(), 1);

        // 设置 AST
        let module = Module::default();
        cache.set_ast(module);
        assert!(cache.has_cached_ast());

        // 更新内容应该使 AST 失效
        cache.update("x = 43".to_string(), 2);
        assert!(!cache.has_cached_ast());
    }
}
