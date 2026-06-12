//! `util::cache` 模块的单元测试
//!
//! 覆盖 `DocumentCache` 和 `DocumentStore` 的创建、更新、缓存失效、容量清理等功能。

use crate::util::cache::{DocumentCache, DocumentStore};
use crate::frontend::core::parser::ast::Module;

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
