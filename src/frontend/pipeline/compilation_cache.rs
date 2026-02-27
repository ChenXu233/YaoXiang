//! 编译缓存系统
//!
//! 提供编译产物的内存缓存，加速增量编译：
//! - 基于文件内容哈希检测变更
//! - 缓存 AST、类型检查结果、IR
//! - 支持 TTL 过期和最大容量限制
//! - 支持缓存统计和监控

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::frontend::core::parser::ast::Module as AstModule;
use crate::frontend::typecheck::TypeCheckResult;
use crate::middle::ModuleIR;

// ============ FNV-1a 哈希 ============

/// FNV-1a 哈希算法
///
/// 非加密用途的快速哈希，用于内容变更检测。
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

/// 计算文件内容哈希
pub fn content_hash(content: &str) -> u64 {
    fnv1a_hash(content.as_bytes())
}

// ============ 缓存条目 ============

/// 单文件的编译缓存条目
#[derive(Debug, Clone)]
pub struct FileCompilationCache {
    /// 源代码内容哈希
    pub content_hash: u64,
    /// 缓存的 AST
    pub ast: Option<AstModule>,
    /// 缓存的类型检查结果
    pub type_result: Option<TypeCheckResult>,
    /// 缓存的 IR
    pub ir: Option<ModuleIR>,
    /// 缓存创建时间
    pub cached_at: Instant,
    /// 最后访问时间
    pub last_accessed: Instant,
    /// 命中次数
    pub hit_count: u64,
}

impl FileCompilationCache {
    /// 创建新的缓存条目
    pub fn new(content_hash: u64) -> Self {
        let now = Instant::now();
        Self {
            content_hash,
            ast: None,
            type_result: None,
            ir: None,
            cached_at: now,
            last_accessed: now,
            hit_count: 0,
        }
    }

    /// 检查缓存是否对给定内容哈希有效
    pub fn is_valid_for(
        &self,
        hash: u64,
    ) -> bool {
        self.content_hash == hash
    }

    /// 检查缓存是否过期（根据 TTL）
    pub fn is_expired(
        &self,
        ttl: Duration,
    ) -> bool {
        self.cached_at.elapsed() > ttl
    }

    /// 标记为已访问
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.hit_count += 1;
    }
}

// ============ 编译缓存 ============

/// 编译缓存统计
#[derive(Debug, Clone, Default)]
pub struct CompilationCacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数（内容变更）
    pub misses: u64,
    /// 因过期失效次数
    pub expirations: u64,
    /// 当前缓存条目数
    pub entries: usize,
    /// 总节省的编译时间估计（毫秒）
    pub saved_time_ms: u64,
}

impl CompilationCacheStats {
    /// 命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// 编译缓存
///
/// 管理所有文件的编译产物缓存，支持增量编译。
#[derive(Debug)]
pub struct CompilationCache {
    /// 文件路径 → 缓存条目
    files: HashMap<PathBuf, FileCompilationCache>,
    /// URI → 缓存条目（LSP 用，URI 格式为 file:///...）
    uri_files: HashMap<String, FileCompilationCache>,
    /// 缓存过期时间
    ttl: Duration,
    /// 最大缓存条目数
    max_entries: usize,
    /// 统计信息
    stats: CompilationCacheStats,
}

impl Default for CompilationCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilationCache {
    /// 创建默认配置的编译缓存
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            uri_files: HashMap::new(),
            ttl: Duration::from_secs(24 * 60 * 60), // 24小时
            max_entries: 1000,
            stats: CompilationCacheStats::default(),
        }
    }

    /// 使用自定义配置创建编译缓存
    pub fn with_config(
        ttl_secs: u64,
        max_entries: usize,
    ) -> Self {
        Self {
            files: HashMap::new(),
            uri_files: HashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
            max_entries,
            stats: CompilationCacheStats::default(),
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> &CompilationCacheStats {
        &self.stats
    }

    /// 当前缓存条目数
    pub fn entry_count(&self) -> usize {
        self.files.len() + self.uri_files.len()
    }

    // ============ 基于路径的缓存操作 ============

    /// 检查文件是否有有效缓存
    pub fn has_valid_cache(
        &self,
        file: &Path,
        source: &str,
    ) -> bool {
        let hash = content_hash(source);
        self.files
            .get(file)
            .is_some_and(|entry| entry.is_valid_for(hash) && !entry.is_expired(self.ttl))
    }

    /// 获取文件的缓存条目（如果有效）
    pub fn get(
        &mut self,
        file: &Path,
        source: &str,
    ) -> Option<&FileCompilationCache> {
        let hash = content_hash(source);

        if let Some(entry) = self.files.get_mut(file) {
            if entry.is_valid_for(hash) && !entry.is_expired(self.ttl) {
                entry.touch();
                self.stats.hits += 1;
                // 借用规则需要重新获取
                return self.files.get(file);
            } else {
                if entry.is_expired(self.ttl) {
                    self.stats.expirations += 1;
                }
                self.stats.misses += 1;
                return None;
            }
        }

        self.stats.misses += 1;
        None
    }

    /// 存储文件的编译缓存
    pub fn store(
        &mut self,
        file: PathBuf,
        source: &str,
        ast: Option<AstModule>,
        type_result: Option<TypeCheckResult>,
        ir: Option<ModuleIR>,
    ) {
        self.evict_if_needed();

        let hash = content_hash(source);
        let mut entry = FileCompilationCache::new(hash);
        entry.ast = ast;
        entry.type_result = type_result;
        entry.ir = ir;

        self.files.insert(file, entry);
        self.stats.entries = self.files.len() + self.uri_files.len();
    }

    // ============ 基于 URI 的缓存操作（LSP 用）============

    /// 检查 URI 文件是否有有效缓存
    pub fn has_valid_cache_uri(
        &self,
        uri: &str,
        source: &str,
    ) -> bool {
        let hash = content_hash(source);
        self.uri_files
            .get(uri)
            .is_some_and(|entry| entry.is_valid_for(hash) && !entry.is_expired(self.ttl))
    }

    /// 获取 URI 文件的缓存条目
    pub fn get_uri(
        &mut self,
        uri: &str,
        source: &str,
    ) -> Option<&FileCompilationCache> {
        let hash = content_hash(source);

        if let Some(entry) = self.uri_files.get_mut(uri) {
            if entry.is_valid_for(hash) && !entry.is_expired(self.ttl) {
                entry.touch();
                self.stats.hits += 1;
                return self.uri_files.get(uri);
            } else {
                if entry.is_expired(self.ttl) {
                    self.stats.expirations += 1;
                }
                self.stats.misses += 1;
                return None;
            }
        }

        self.stats.misses += 1;
        None
    }

    /// 存储 URI 文件的编译缓存
    pub fn store_uri(
        &mut self,
        uri: String,
        source: &str,
        ast: Option<AstModule>,
        type_result: Option<TypeCheckResult>,
        ir: Option<ModuleIR>,
    ) {
        self.evict_if_needed();

        let hash = content_hash(source);
        let mut entry = FileCompilationCache::new(hash);
        entry.ast = ast;
        entry.type_result = type_result;
        entry.ir = ir;

        self.uri_files.insert(uri, entry);
        self.stats.entries = self.files.len() + self.uri_files.len();
    }

    // ============ 失效与清理 ============

    /// 使指定文件的缓存失效
    pub fn invalidate(
        &mut self,
        file: &Path,
    ) {
        self.files.remove(file);
        self.stats.entries = self.files.len() + self.uri_files.len();
    }

    /// 使指定 URI 的缓存失效
    pub fn invalidate_uri(
        &mut self,
        uri: &str,
    ) {
        self.uri_files.remove(uri);
        self.stats.entries = self.files.len() + self.uri_files.len();
    }

    /// 批量使多个文件的缓存失效
    pub fn invalidate_many(
        &mut self,
        files: &[PathBuf],
    ) {
        for file in files {
            self.files.remove(file);
        }
        self.stats.entries = self.files.len() + self.uri_files.len();
    }

    /// 清空所有缓存
    pub fn clear(&mut self) {
        self.files.clear();
        self.uri_files.clear();
        self.stats.entries = 0;
    }

    /// 如果超过容量限制，淘汰最旧的条目（LRU 策略）
    fn evict_if_needed(&mut self) {
        let total = self.files.len() + self.uri_files.len();
        if total < self.max_entries {
            return;
        }

        // 先淘汰过期条目
        self.files
            .retain(|_, entry| !entry.is_expired(self.ttl));
        self.uri_files
            .retain(|_, entry| !entry.is_expired(self.ttl));

        // 如果还超限，淘汰最久未访问的
        let total = self.files.len() + self.uri_files.len();
        if total >= self.max_entries {
            // 找最旧的 file 条目
            if let Some(oldest_key) = self
                .files
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(k, _)| k.clone())
            {
                self.files.remove(&oldest_key);
            }
        }

        self.stats.entries = self.files.len() + self.uri_files.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_consistency() {
        let h1 = content_hash("hello world");
        let h2 = content_hash("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_content_hash_different() {
        let h1 = content_hash("hello");
        let h2 = content_hash("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_empty_cache() {
        let cache = CompilationCache::new();
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn test_cache_hit() {
        let mut cache = CompilationCache::new();
        let path = PathBuf::from("/test.yx");
        let source = "x = 42";

        cache.store(path.clone(), source, None, None, None);
        assert!(cache.has_valid_cache(&path, source));

        let entry = cache.get(&path, source);
        assert!(entry.is_some());
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_miss_on_change() {
        let mut cache = CompilationCache::new();
        let path = PathBuf::from("/test.yx");
        let source_v1 = "x = 42";
        let source_v2 = "x = 99";

        cache.store(path.clone(), source_v1, None, None, None);
        assert!(cache.has_valid_cache(&path, source_v1));
        assert!(!cache.has_valid_cache(&path, source_v2));

        let entry = cache.get(&path, source_v2);
        assert!(entry.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_cache_uri() {
        let mut cache = CompilationCache::new();
        let uri = "file:///test.yx";
        let source = "x = 42";

        cache.store_uri(uri.to_string(), source, None, None, None);
        assert!(cache.has_valid_cache_uri(uri, source));

        let entry = cache.get_uri(uri, source);
        assert!(entry.is_some());
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = CompilationCache::new();
        let path = PathBuf::from("/test.yx");
        let source = "x = 42";

        cache.store(path.clone(), source, None, None, None);
        assert!(cache.has_valid_cache(&path, source));

        cache.invalidate(&path);
        assert!(!cache.has_valid_cache(&path, source));
    }

    #[test]
    fn test_cache_invalidate_uri() {
        let mut cache = CompilationCache::new();
        let uri = "file:///test.yx";
        let source = "x = 42";

        cache.store_uri(uri.to_string(), source, None, None, None);
        cache.invalidate_uri(uri);
        assert!(!cache.has_valid_cache_uri(uri, source));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = CompilationCache::new();
        cache.store(PathBuf::from("/a.yx"), "a", None, None, None);
        cache.store(PathBuf::from("/b.yx"), "b", None, None, None);
        cache.store_uri("file:///c.yx".to_string(), "c", None, None, None);

        assert_eq!(cache.entry_count(), 3);

        cache.clear();
        assert_eq!(cache.entry_count(), 0);
    }

    #[test]
    fn test_cache_batch_invalidate() {
        let mut cache = CompilationCache::new();
        let paths: Vec<PathBuf> = (0..5).map(|i| PathBuf::from(format!("/{}.yx", i))).collect();

        for (i, p) in paths.iter().enumerate() {
            cache.store(p.clone(), &format!("source_{}", i), None, None, None);
        }

        // 批量失效前 3 个
        cache.invalidate_many(&paths[..3]);
        assert!(!cache.has_valid_cache(&paths[0], "source_0"));
        assert!(!cache.has_valid_cache(&paths[1], "source_1"));
        assert!(!cache.has_valid_cache(&paths[2], "source_2"));
        assert!(cache.has_valid_cache(&paths[3], "source_3"));
        assert!(cache.has_valid_cache(&paths[4], "source_4"));
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = CompilationCache::new();
        let path = PathBuf::from("/test.yx");
        let source = "x = 42";

        cache.store(path.clone(), source, None, None, None);

        // 3 次命中
        for _ in 0..3 {
            cache.get(&path, source);
        }
        // 1 次未命中
        cache.get(&path, "changed source");

        let stats = cache.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_ttl_expiration() {
        let mut cache = CompilationCache::with_config(0, 100); // TTL = 0 秒
        let path = PathBuf::from("/test.yx");
        let source = "x = 42";

        cache.store(path.clone(), source, None, None, None);

        // TTL = 0 意味着立即过期
        // 需要等一小段时间让 elapsed > 0
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(!cache.has_valid_cache(&path, source));
    }

    #[test]
    fn test_cache_stores_compilation_data() {
        let mut cache = CompilationCache::new();
        let path = PathBuf::from("/test.yx");
        let source = "x = 42";

        let type_result = TypeCheckResult::default();

        cache.store(
            path.clone(),
            source,
            None,
            Some(type_result),
            None,
        );

        let entry = cache.get(&path, source).unwrap();
        assert!(entry.ast.is_none());
        assert!(entry.type_result.is_some());
        assert!(entry.ir.is_none());
    }
}
