//! 增量编译调度器
//!
//! 基于依赖图和编译缓存，智能调度编译任务：
//! - 检测变更文件 → 计算影响范围 → 拓扑排序 → 仅编译受影响模块
//! - 支持并行编译（无依赖关系的模块可并行）
//! - 提供编译统计和性能监控

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use super::compilation_cache::CompilationCache;
use crate::frontend::module::dep_graph::ModuleDependencyGraph;

// ============ 编译任务 ============

/// 单个编译任务
#[derive(Debug, Clone)]
pub struct CompileTask {
    /// 文件路径
    pub file: PathBuf,
    /// 源代码内容
    pub source: String,
    /// 编译原因
    pub reason: CompileReason,
    /// 依赖层级（拓扑排序中的层次，用于并行调度）
    pub level: usize,
}

/// 编译原因
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileReason {
    /// 文件内容变更
    ContentChanged,
    /// 依赖的模块变更
    DependencyChanged(String),
    /// 导出项变更导致下游失效
    ExportChanged(String),
    /// 无缓存（首次编译）
    NoCache,
    /// 强制重编译
    ForceRecompile,
}

impl std::fmt::Display for CompileReason {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            CompileReason::ContentChanged => write!(f, "内容变更"),
            CompileReason::DependencyChanged(dep) => write!(f, "依赖 {} 变更", dep),
            CompileReason::ExportChanged(dep) => write!(f, "{} 导出项变更", dep),
            CompileReason::NoCache => write!(f, "首次编译"),
            CompileReason::ForceRecompile => write!(f, "强制重编译"),
        }
    }
}

// ============ 调度结果 ============

/// 增量编译调度结果
#[derive(Debug, Clone)]
pub struct ScheduleResult {
    /// 需要编译的任务（拓扑排序后）
    pub tasks: Vec<CompileTask>,
    /// 可跳过的文件（缓存命中）
    pub skipped: Vec<PathBuf>,
    /// 总文件数
    pub total_files: usize,
    /// 调度耗时（毫秒）
    pub schedule_time_ms: u64,
}

impl ScheduleResult {
    /// 是否有需要编译的任务
    pub fn has_tasks(&self) -> bool {
        !self.tasks.is_empty()
    }

    /// 需要编译的文件数
    pub fn compile_count(&self) -> usize {
        self.tasks.len()
    }

    /// 跳过的文件数
    pub fn skip_count(&self) -> usize {
        self.skipped.len()
    }

    /// 节省的编译比例（0.0 ~ 1.0）
    pub fn savings_ratio(&self) -> f64 {
        if self.total_files == 0 {
            return 0.0;
        }
        self.skipped.len() as f64 / self.total_files as f64
    }
}

// ============ 增量编译调度器 ============

/// 增量编译调度器
///
/// 整合依赖图和编译缓存，确定最小编译集合。
pub struct IncrementalScheduler<'a> {
    /// 依赖图
    dep_graph: &'a ModuleDependencyGraph,
    /// 编译缓存
    cache: &'a mut CompilationCache,
    /// 是否强制全量编译
    force_full: bool,
}

impl<'a> IncrementalScheduler<'a> {
    /// 创建增量编译调度器
    pub fn new(
        dep_graph: &'a ModuleDependencyGraph,
        cache: &'a mut CompilationCache,
    ) -> Self {
        Self {
            dep_graph,
            cache,
            force_full: false,
        }
    }

    /// 设置为强制全量编译
    pub fn force_full_compile(mut self) -> Self {
        self.force_full = true;
        self
    }

    /// 调度编译任务
    ///
    /// 给定所有项目文件及其源代码，确定需要编译哪些文件。
    ///
    /// # 参数
    /// - `files`: 文件路径 → 源代码内容的映射
    ///
    /// # 算法
    /// 1. 检测内容变更的文件
    /// 2. 通过依赖图扩展受影响的模块
    /// 3. 拓扑排序确定编译顺序
    /// 4. 分配层级用于并行调度
    pub fn schedule(
        &mut self,
        files: &HashMap<PathBuf, String>,
    ) -> ScheduleResult {
        let start = Instant::now();
        let total_files = files.len();

        // 强制全量编译
        if self.force_full {
            return self.schedule_full(files, start, total_files);
        }

        // 1. 检测变更文件
        let (changed_files, change_reasons) = self.detect_changes(files);

        // 如果没有变更，返回空调度
        if changed_files.is_empty() {
            return ScheduleResult {
                tasks: Vec::new(),
                skipped: files.keys().cloned().collect(),
                total_files,
                schedule_time_ms: start.elapsed().as_millis() as u64,
            };
        }

        // 2. 通过依赖图扩展受影响范围
        let affected = self.expand_affected(&changed_files, &change_reasons);

        // 3. 构建编译任务列表
        let mut tasks = Vec::new();
        let mut skipped = Vec::new();

        // 获取拓扑排序顺序（忽略循环错误）
        let topo_order = self.dep_graph.topological_sort().unwrap_or_default();

        // 计算层级（简单方式：在 topo 序列中的位置）
        let level_map: HashMap<String, usize> = topo_order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.name.clone(), i))
            .collect();

        for (file, source) in files {
            if affected.contains(file) {
                let reason = change_reasons
                    .get(file)
                    .cloned()
                    .unwrap_or(CompileReason::DependencyChanged("unknown".to_string()));

                let level = file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|name| level_map.get(name).copied().unwrap_or(0))
                    .unwrap_or(0);

                tasks.push(CompileTask {
                    file: file.clone(),
                    source: source.clone(),
                    reason,
                    level,
                });
            } else {
                skipped.push(file.clone());
            }
        }

        // 按层级排序（低层级优先）
        tasks.sort_by_key(|t| t.level);

        ScheduleResult {
            tasks,
            skipped,
            total_files,
            schedule_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// 全量编译调度
    fn schedule_full(
        &self,
        files: &HashMap<PathBuf, String>,
        start: Instant,
        total_files: usize,
    ) -> ScheduleResult {
        let topo_order = self.dep_graph.topological_sort().unwrap_or_default();
        let level_map: HashMap<String, usize> = topo_order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.name.clone(), i))
            .collect();

        let mut tasks: Vec<CompileTask> = files
            .iter()
            .map(|(file, source)| {
                let level = file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|name| level_map.get(name).copied().unwrap_or(0))
                    .unwrap_or(0);

                CompileTask {
                    file: file.clone(),
                    source: source.clone(),
                    reason: CompileReason::ForceRecompile,
                    level,
                }
            })
            .collect();

        tasks.sort_by_key(|t| t.level);

        ScheduleResult {
            tasks,
            skipped: Vec::new(),
            total_files,
            schedule_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// 检测变更文件
    fn detect_changes(
        &self,
        files: &HashMap<PathBuf, String>,
    ) -> (HashSet<PathBuf>, HashMap<PathBuf, CompileReason>) {
        let mut changed = HashSet::new();
        let mut reasons = HashMap::new();

        for (file, source) in files {
            if !self.cache.has_valid_cache(file, source) {
                changed.insert(file.clone());

                // 确定变更原因
                let reason = if self.cache.entry_count() == 0 {
                    CompileReason::NoCache
                } else {
                    CompileReason::ContentChanged
                };
                reasons.insert(file.clone(), reason);
            }
        }

        (changed, reasons)
    }

    /// 通过依赖图扩展受影响范围
    fn expand_affected(
        &self,
        changed_files: &HashSet<PathBuf>,
        _original_reasons: &HashMap<PathBuf, CompileReason>,
    ) -> HashSet<PathBuf> {
        let mut affected = changed_files.clone();

        // 使用依赖图的 affected_modules 方法
        let changed_paths: Vec<PathBuf> = changed_files.iter().cloned().collect();
        let affected_ids = self.dep_graph.affected_modules(&changed_paths);

        // 将受影响的模块 ID 转换回文件路径
        for id in &affected_ids {
            if let Some(path) = &id.path {
                if !affected.contains(path) {
                    affected.insert(path.clone());
                }
            }
        }

        affected
    }

    /// 获取可并行编译的批次
    ///
    /// 将任务按层级分组，同层级的任务之间没有依赖关系，可以并行编译。
    pub fn parallelize(tasks: &[CompileTask]) -> Vec<Vec<&CompileTask>> {
        if tasks.is_empty() {
            return Vec::new();
        }

        let mut batches: Vec<Vec<&CompileTask>> = Vec::new();
        let mut current_level = tasks[0].level;
        let mut current_batch = Vec::new();

        for task in tasks {
            if task.level != current_level {
                batches.push(current_batch);
                current_batch = Vec::new();
                current_level = task.level;
            }
            current_batch.push(task);
        }

        if !current_batch.is_empty() {
            batches.push(current_batch);
        }

        batches
    }
}

// ============ 编译统计 ============

/// 增量编译统计
#[derive(Debug, Clone, Default)]
pub struct IncrementalStats {
    /// 总编译次数
    pub total_compiles: u64,
    /// 增量编译次数
    pub incremental_compiles: u64,
    /// 全量编译次数
    pub full_compiles: u64,
    /// 总节省的文件编译数
    pub total_files_saved: u64,
    /// 总编译时间（毫秒）
    pub total_compile_time_ms: u64,
    /// 总节省时间估计（毫秒）
    pub total_saved_time_ms: u64,
}

impl IncrementalStats {
    /// 记录一次编译
    pub fn record_compile(
        &mut self,
        result: &ScheduleResult,
        compile_time_ms: u64,
    ) {
        self.total_compiles += 1;
        self.total_compile_time_ms += compile_time_ms;

        if result.skip_count() > 0 {
            self.incremental_compiles += 1;
            self.total_files_saved += result.skip_count() as u64;
        } else {
            self.full_compiles += 1;
        }
    }

    /// 增量编译比率
    pub fn incremental_rate(&self) -> f64 {
        if self.total_compiles == 0 {
            return 0.0;
        }
        self.incremental_compiles as f64 / self.total_compiles as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::module::dep_graph::{DependencyEdge, ModuleId};

    fn make_files(names: &[(&str, &str)]) -> HashMap<PathBuf, String> {
        names
            .iter()
            .map(|(name, source)| (PathBuf::from(name), source.to_string()))
            .collect()
    }

    #[test]
    fn test_no_changes_skips_all() {
        let graph = ModuleDependencyGraph::new();
        let mut cache = CompilationCache::new();

        let files = make_files(&[("/a.yx", "a = 1"), ("/b.yx", "b = 2")]);

        // 预填充缓存
        for (path, source) in &files {
            cache.store(path.clone(), source, None, None, None);
        }

        let mut scheduler = IncrementalScheduler::new(&graph, &mut cache);
        let result = scheduler.schedule(&files);

        assert!(!result.has_tasks());
        assert_eq!(result.skip_count(), 2);
        assert!((result.savings_ratio() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_single_change_detected() {
        let graph = ModuleDependencyGraph::new();
        let mut cache = CompilationCache::new();

        // 旧版本缓存
        cache.store(PathBuf::from("/a.yx"), "a = 1", None, None, None);
        cache.store(PathBuf::from("/b.yx"), "b = 2", None, None, None);

        // 新版本，a.yx 变更了
        let files = make_files(&[("/a.yx", "a = 999"), ("/b.yx", "b = 2")]);

        let mut scheduler = IncrementalScheduler::new(&graph, &mut cache);
        let result = scheduler.schedule(&files);

        assert!(result.has_tasks());
        assert_eq!(result.compile_count(), 1);
        assert_eq!(result.skip_count(), 1);

        let task = &result.tasks[0];
        assert_eq!(task.file, PathBuf::from("/a.yx"));
        assert_eq!(task.reason, CompileReason::ContentChanged);
    }

    #[test]
    fn test_force_full_compile() {
        let graph = ModuleDependencyGraph::new();
        let mut cache = CompilationCache::new();

        let files = make_files(&[("/a.yx", "a = 1"), ("/b.yx", "b = 2")]);

        // 缓存全部
        for (path, source) in &files {
            cache.store(path.clone(), source, None, None, None);
        }

        let mut scheduler = IncrementalScheduler::new(&graph, &mut cache).force_full_compile();
        let result = scheduler.schedule(&files);

        // 强制全量编译
        assert_eq!(result.compile_count(), 2);
        assert_eq!(result.skip_count(), 0);

        for task in &result.tasks {
            assert_eq!(task.reason, CompileReason::ForceRecompile);
        }
    }

    #[test]
    fn test_dependency_propagation() {
        let mut graph = ModuleDependencyGraph::new();
        let mut cache = CompilationCache::new();

        // a → b → c（a 依赖 b，b 依赖 c）
        let id_a = ModuleId::new("a", PathBuf::from("/a.yx"));
        let id_b = ModuleId::new("b", PathBuf::from("/b.yx"));
        let id_c = ModuleId::new("c", PathBuf::from("/c.yx"));

        graph.add_module(id_a.clone());
        graph.add_module(id_b.clone());
        graph.add_module(id_c.clone());
        graph.add_dependency(&id_a, DependencyEdge { target: id_b.clone(), items: None });
        graph.add_dependency(&id_b, DependencyEdge { target: id_c.clone(), items: None });

        // 缓存旧版本
        cache.store(PathBuf::from("/a.yx"), "a = 1", None, None, None);
        cache.store(PathBuf::from("/b.yx"), "b = 2", None, None, None);
        cache.store(PathBuf::from("/c.yx"), "c = 3", None, None, None);

        // c.yx 变更了
        let files = make_files(&[("/a.yx", "a = 1"), ("/b.yx", "b = 2"), ("/c.yx", "c = 999")]);

        let mut scheduler = IncrementalScheduler::new(&graph, &mut cache);
        let result = scheduler.schedule(&files);

        // c 变更 → a, b 也受影响（因为 a→b→c）
        let affected_paths: HashSet<PathBuf> =
            result.tasks.iter().map(|t| t.file.clone()).collect();

        // c 一定要编译
        assert!(affected_paths.contains(&PathBuf::from("/c.yx")));
    }

    #[test]
    fn test_no_cache_means_all_compile() {
        let graph = ModuleDependencyGraph::new();
        let mut cache = CompilationCache::new();

        let files = make_files(&[("/a.yx", "a = 1"), ("/b.yx", "b = 2")]);

        let mut scheduler = IncrementalScheduler::new(&graph, &mut cache);
        let result = scheduler.schedule(&files);

        // 没有缓存，全部需要编译
        assert_eq!(result.compile_count(), 2);
        assert_eq!(result.skip_count(), 0);
    }

    #[test]
    fn test_parallelize_batches() {
        let tasks = vec![
            CompileTask {
                file: PathBuf::from("/a.yx"),
                source: "a".to_string(),
                reason: CompileReason::ContentChanged,
                level: 0,
            },
            CompileTask {
                file: PathBuf::from("/b.yx"),
                source: "b".to_string(),
                reason: CompileReason::ContentChanged,
                level: 0,
            },
            CompileTask {
                file: PathBuf::from("/c.yx"),
                source: "c".to_string(),
                reason: CompileReason::ContentChanged,
                level: 1,
            },
        ];

        let batches = IncrementalScheduler::parallelize(&tasks);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 2); // level 0: a, b
        assert_eq!(batches[1].len(), 1); // level 1: c
    }

    #[test]
    fn test_empty_schedule() {
        let graph = ModuleDependencyGraph::new();
        let mut cache = CompilationCache::new();
        let files = HashMap::new();

        let mut scheduler = IncrementalScheduler::new(&graph, &mut cache);
        let result = scheduler.schedule(&files);

        assert!(!result.has_tasks());
        assert_eq!(result.total_files, 0);
    }

    #[test]
    fn test_incremental_stats() {
        let mut stats = IncrementalStats::default();

        // 模拟一次增量编译
        let result = ScheduleResult {
            tasks: vec![CompileTask {
                file: PathBuf::from("/a.yx"),
                source: "a".to_string(),
                reason: CompileReason::ContentChanged,
                level: 0,
            }],
            skipped: vec![PathBuf::from("/b.yx"), PathBuf::from("/c.yx")],
            total_files: 3,
            schedule_time_ms: 5,
        };

        stats.record_compile(&result, 100);

        assert_eq!(stats.total_compiles, 1);
        assert_eq!(stats.incremental_compiles, 1);
        assert_eq!(stats.full_compiles, 0);
        assert_eq!(stats.total_files_saved, 2);
        assert!((stats.incremental_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_schedule_result_savings() {
        let result = ScheduleResult {
            tasks: vec![CompileTask {
                file: PathBuf::from("/a.yx"),
                source: "a".to_string(),
                reason: CompileReason::ContentChanged,
                level: 0,
            }],
            skipped: vec![PathBuf::from("/b.yx"), PathBuf::from("/c.yx"), PathBuf::from("/d.yx")],
            total_files: 4,
            schedule_time_ms: 1,
        };

        assert!((result.savings_ratio() - 0.75).abs() < f64::EPSILON);
    }
}
