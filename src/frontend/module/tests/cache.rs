//! Cache 模块测试
//!
//! 测试模块缓存功能，包括：
//! - 缓存存取
//! - 缓存失效
//! - 缓存统计
//! - 开发/发布模式

use std::path::PathBuf;

use crate::frontend::module::cache::{CacheMode, ModuleCache};
use crate::frontend::module::loader::ModuleInfo;
use crate::frontend::module::ModuleSource;
use tempfile::TempDir;

fn make_module(path: &str) -> ModuleInfo {
    ModuleInfo::new(path.to_string(), ModuleSource::User)
}

#[test]
fn test_cache_put_and_get() {
    let cache = ModuleCache::new(CacheMode::Compile);
    let module = make_module("my_module");

    cache.put("my_module", module.clone(), None);
    let cached = cache.get("my_module", None);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().path, "my_module");
}

#[test]
fn test_cache_miss() {
    let cache = ModuleCache::new(CacheMode::Compile);
    assert!(cache.get("nonexistent", None).is_none());
}

#[test]
fn test_cache_invalidate() {
    let cache = ModuleCache::new(CacheMode::Compile);
    cache.put("my_module", make_module("my_module"), None);

    cache.invalidate("my_module");
    assert!(cache.get("my_module", None).is_none());
}

#[test]
fn test_cache_clear() {
    let cache = ModuleCache::new(CacheMode::Compile);
    cache.put("mod_a", make_module("mod_a"), None);
    cache.put("mod_b", make_module("mod_b"), None);

    cache.clear();
    assert!(cache.get("mod_a", None).is_none());
    assert!(cache.get("mod_b", None).is_none());
}

#[test]
fn test_cache_stats() {
    let cache = ModuleCache::new(CacheMode::Compile);
    cache.put("mod_a", make_module("mod_a"), None);

    cache.get("mod_a", None); // hit
    cache.get("mod_b", None); // miss

    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.entries, 1);
    assert!((stats.hit_rate() - 50.0).abs() < f64::EPSILON);
}

#[test]
fn test_development_mode_invalidation() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.yx");
    std::fs::write(&file_path, "original content").unwrap();

    let cache = ModuleCache::new(CacheMode::Development);
    cache.put("test_mod", make_module("test_mod"), Some(&file_path));

    // 文件未修改 → 缓存命中
    let result = cache.get("test_mod", Some(&file_path));
    assert!(result.is_some());

    // 修改文件 → 缓存失效
    std::fs::write(&file_path, "modified content").unwrap();
    let result = cache.get("test_mod", Some(&file_path));
    assert!(result.is_none());
}

#[test]
fn test_release_mode_no_check() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.yx");
    std::fs::write(&file_path, "original content").unwrap();

    let cache = ModuleCache::new(CacheMode::Release);
    cache.put("test_mod", make_module("test_mod"), Some(&file_path));

    // 即使文件修改，Release 模式也不检查
    std::fs::write(&file_path, "modified content").unwrap();
    let result = cache.get("test_mod", Some(&file_path));
    assert!(result.is_some());
}

#[test]
fn test_invalidate_by_file() {
    let dir = TempDir::new().unwrap();
    let file_a = dir.path().join("a.yx");
    let file_b = dir.path().join("b.yx");
    std::fs::write(&file_a, "content a").unwrap();
    std::fs::write(&file_b, "content b").unwrap();

    let cache = ModuleCache::new(CacheMode::Compile);
    cache.put("mod_a", make_module("mod_a"), Some(&file_a));
    cache.put("mod_b", make_module("mod_b"), Some(&file_b));

    // 失效 file_a 相关的缓存
    cache.invalidate_by_file(&file_a);
    assert!(cache.get("mod_a", None).is_none());
    assert!(cache.get("mod_b", None).is_some());
}

#[test]
fn test_fnv1a_hash_consistency() {
    use crate::frontend::module::cache::fnv1a_hash;

    let data = b"hello world";
    let hash1 = fnv1a_hash(data);
    let hash2 = fnv1a_hash(data);
    assert_eq!(hash1, hash2);

    let different = b"hello world!";
    let hash3 = fnv1a_hash(different);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_cached_modules_list() {
    let cache = ModuleCache::new(CacheMode::Compile);
    cache.put("alpha", make_module("alpha"), None);
    cache.put("beta", make_module("beta"), None);

    let mut modules = cache.cached_modules();
    modules.sort();
    assert_eq!(modules, vec!["alpha", "beta"]);
}
