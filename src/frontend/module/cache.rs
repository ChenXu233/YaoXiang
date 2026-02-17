//! 模块缓存策略
//!
//! 提供多级缓存机制以优化模块加载性能：
//!
//! | 缓存级别 | 适用场景 | 实现方式 |
//! |---------|---------|---------|
//! | **编译时缓存** | 同一编译单元内 | 内存 HashMap |
//! | **开发缓存** | `yaoxiang run` 运行时 | 文件哈希 + 内存缓存 |
//! | **发布缓存** | 生产构建 | 锁定版本，不执行变更检测 |
//!
//! # 设计思路
//!
//! - 使用文件内容哈希（而非修改时间）判断是否需要重新解析
//! - 缓存粒度为单个模块（`ModuleInfo`）
//! - 线程安全：使用 `parking_lot::RwLock` 支持并发读取

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use parking_lot::RwLock;

use super::ModuleInfo;

/// 缓存条目
///
/// 包含模块信息及其关联的校验数据。
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 缓存的模块信息
    module: ModuleInfo,
    /// 源文件内容哈希（用于变更检测）
    content_hash: u64,
    /// 缓存写入时间（预留用于 TTL 淘汰策略）
    #[allow(dead_code)]
    cached_at: Instant,
    /// 源文件路径（None 表示 std 模块）
    file_path: Option<PathBuf>,
}

/// 缓存模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    /// 编译时缓存：仅内存缓存，编译结束后丢弃
    Compile,
    /// 开发缓存：监听文件变化，自动失效
    Development,
    /// 发布缓存：锁定版本，不检测变更
    Release,
}

/// 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: usize,
    /// 缓存未命中次数
    pub misses: usize,
    /// 因失效导致的重新加载次数
    pub invalidations: usize,
    /// 当前缓存条目数
    pub entries: usize,
}

impl CacheStats {
    /// 命中率（百分比）
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// 模块缓存
///
/// 线程安全的模块缓存，支持多种缓存策略。
#[derive(Debug)]
pub struct ModuleCache {
    /// 缓存条目（module_path -> CacheEntry）
    entries: RwLock<HashMap<String, CacheEntry>>,
    /// 缓存模式
    mode: CacheMode,
    /// 统计信息
    stats: RwLock<CacheStats>,
}

impl ModuleCache {
    /// 创建新的缓存
    pub fn new(mode: CacheMode) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            mode,
            stats: RwLock::new(CacheStats::default()),
        }
    }

    /// 获取缓存的模块（如果有效）
    ///
    /// 根据缓存模式决定是否需要验证：
    /// - `Compile`：直接返回缓存
    /// - `Development`：检查文件内容哈希
    /// - `Release`：直接返回缓存
    pub fn get(
        &self,
        module_path: &str,
        file_path: Option<&Path>,
    ) -> Option<ModuleInfo> {
        let entries = self.entries.read();
        if let Some(entry) = entries.get(module_path) {
            // Development 模式需要检查文件变更
            if self.mode == CacheMode::Development {
                if let (Some(cached_path), Some(check_path)) = (&entry.file_path, file_path) {
                    if cached_path == check_path {
                        // 计算当前文件哈希
                        if let Ok(current_hash) = compute_file_hash(check_path) {
                            if current_hash != entry.content_hash {
                                // 文件已变更，缓存失效
                                drop(entries);
                                self.stats.write().invalidations += 1;
                                self.stats.write().misses += 1;
                                return None;
                            }
                        }
                    }
                }
            }

            self.stats.write().hits += 1;
            Some(entry.module.clone())
        } else {
            self.stats.write().misses += 1;
            None
        }
    }

    /// 写入缓存
    pub fn put(
        &self,
        module_path: &str,
        module: ModuleInfo,
        file_path: Option<&Path>,
    ) {
        let content_hash = file_path
            .and_then(|p| compute_file_hash(p).ok())
            .unwrap_or(0);

        let entry = CacheEntry {
            module,
            content_hash,
            cached_at: Instant::now(),
            file_path: file_path.map(|p| p.to_path_buf()),
        };

        let mut entries = self.entries.write();
        entries.insert(module_path.to_string(), entry);
        drop(entries);

        self.stats.write().entries = self.entries.read().len();
    }

    /// 使指定模块的缓存失效
    pub fn invalidate(
        &self,
        module_path: &str,
    ) {
        let mut entries = self.entries.write();
        if entries.remove(module_path).is_some() {
            drop(entries);
            let mut stats = self.stats.write();
            stats.invalidations += 1;
            stats.entries = self.entries.read().len();
        }
    }

    /// 使所有依赖指定文件路径的缓存失效
    ///
    /// 当文件被修改时，失效所有来源于该文件的缓存条目。
    pub fn invalidate_by_file(
        &self,
        file_path: &Path,
    ) {
        let mut entries = self.entries.write();
        let to_remove: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| entry.file_path.as_ref().is_some_and(|p| p == file_path))
            .map(|(path, _)| path.clone())
            .collect();

        let count = to_remove.len();
        for path in to_remove {
            entries.remove(&path);
        }

        drop(entries);
        let mut stats = self.stats.write();
        stats.invalidations += count;
        stats.entries = self.entries.read().len();
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        self.entries.write().clear();
        let mut stats = self.stats.write();
        stats.entries = 0;
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// 获取缓存模式
    pub fn mode(&self) -> CacheMode {
        self.mode
    }

    /// 获取缓存中的模块路径列表
    pub fn cached_modules(&self) -> Vec<String> {
        self.entries.read().keys().cloned().collect()
    }
}

/// 计算文件内容哈希
///
/// 使用 FNV-1a 算法，性能优于加密哈希，适用于变更检测。
fn compute_file_hash(path: &Path) -> std::io::Result<u64> {
    let content = std::fs::read(path)?;
    Ok(fnv1a_hash(&content))
}

/// FNV-1a 哈希算法
///
/// 非加密用途的快速哈希，用于文件内容变更检测。
fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
