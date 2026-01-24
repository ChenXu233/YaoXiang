//! 性能分析器
//!
//! 记录和分析编译、执行性能指标

use std::collections::BTreeMap;
use std::time::Duration;

/// 性能分析器
pub struct Profiler {
    /// 编译时间记录
    compilation_times: BTreeMap<String, Duration>,
    /// 执行时间记录
    execution_times: BTreeMap<String, Duration>,
    /// 内存使用记录
    memory_usage: BTreeMap<String, usize>,
    /// 函数调用统计
    call_counts: BTreeMap<String, usize>,
    /// 缓存命中率
    cache_hits: u64,
    cache_misses: u64,
}

impl Profiler {
    /// 创建新的性能分析器
    pub fn new() -> Self {
        Self {
            compilation_times: BTreeMap::new(),
            execution_times: BTreeMap::new(),
            memory_usage: BTreeMap::new(),
            call_counts: BTreeMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// 记录编译时间
    pub fn record_compilation(
        &mut self,
        duration: Duration,
        _source_length: usize,
    ) {
        let key = format!("compilation_{}", self.compilation_times.len());
        self.compilation_times.insert(key, duration);

        // 限制历史记录数量
        if self.compilation_times.len() > 100 {
            self.compilation_times.pop_first();
        }
    }

    /// 记录执行时间
    pub fn record_execution(
        &mut self,
        function_name: String,
        duration: Duration,
    ) {
        // 更新执行时间
        if let Some(existing) = self.execution_times.get_mut(&function_name) {
            // 累积执行时间
            *existing += duration;
        } else {
            self.execution_times.insert(function_name.clone(), duration);
        }

        // 增加调用计数
        *self.call_counts.entry(function_name).or_insert(0) += 1;

        // 限制记录数量
        if self.execution_times.len() > 1000 {
            self.execution_times.pop_first();
        }
    }

    /// 记录内存使用
    pub fn record_memory(
        &mut self,
        label: String,
        bytes: usize,
    ) {
        self.memory_usage.insert(label, bytes);

        // 限制记录数量
        if self.memory_usage.len() > 100 {
            self.memory_usage.pop_first();
        }
    }

    /// 记录缓存命中
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// 记录缓存未命中
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// 获取缓存命中率
    pub fn get_cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64 * 100.0
        }
    }

    /// 获取平均编译时间
    pub fn get_average_compilation_time(&self) -> Option<Duration> {
        if self.compilation_times.is_empty() {
            None
        } else {
            let total: Duration = self.compilation_times.values().sum();
            Some(total / self.compilation_times.len() as u32)
        }
    }

    /// 获取最慢的编译
    pub fn get_slowest_compilation(&self) -> Option<(String, Duration)> {
        self.compilation_times
            .iter()
            .max_by_key(|(_, duration)| *duration)
            .map(|(key, duration)| (key.clone(), *duration))
    }

    /// 获取最耗时的函数
    pub fn get_most_expensive_function(&self) -> Option<(String, Duration)> {
        self.execution_times
            .iter()
            .max_by_key(|(_, duration)| *duration)
            .map(|(name, duration)| (name.clone(), *duration))
    }

    /// 获取最常调用的函数
    pub fn get_most_called_function(&self) -> Option<(String, usize)> {
        self.call_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, count)| (name.clone(), *count))
    }

    /// 获取所有编译时间
    pub fn get_compilation_times(&self) -> &BTreeMap<String, Duration> {
        &self.compilation_times
    }

    /// 获取所有执行时间
    pub fn get_execution_times(&self) -> &BTreeMap<String, Duration> {
        &self.execution_times
    }

    /// 获取所有内存使用
    pub fn get_memory_usage(&self) -> &BTreeMap<String, usize> {
        &self.memory_usage
    }

    /// 获取调用计数
    pub fn get_call_counts(&self) -> &BTreeMap<String, usize> {
        &self.call_counts
    }

    /// 生成性能报告
    pub fn generate_report(&self) -> PerformanceReport {
        let avg_compile_time = self.get_average_compilation_time();
        let slowest_compile = self.get_slowest_compilation();
        let most_expensive_fn = self.get_most_expensive_function();
        let most_called_fn = self.get_most_called_function();
        let cache_hit_rate = self.get_cache_hit_rate();

        PerformanceReport {
            compilation_stats: CompilationStats {
                total_compilations: self.compilation_times.len(),
                average_time: avg_compile_time,
                slowest_time: slowest_compile.map(|(_, d)| d),
            },
            execution_stats: ExecutionStats {
                total_executions: self.execution_times.len(),
                most_expensive_function: most_expensive_fn.map(|(n, _)| n),
                most_called_function: most_called_fn.map(|(n, _)| n),
            },
            cache_stats: CacheStats {
                hits: self.cache_hits,
                misses: self.cache_misses,
                hit_rate: cache_hit_rate,
            },
            memory_stats: MemoryStats {
                peak_memory: self.memory_usage.values().max().cloned(),
                current_memory: self.memory_usage.values().last().cloned(),
            },
        }
    }

    /// 重置分析器
    pub fn reset(&mut self) {
        self.compilation_times.clear();
        self.execution_times.clear();
        self.memory_usage.clear();
        self.call_counts.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// 性能报告
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub compilation_stats: CompilationStats,
    pub execution_stats: ExecutionStats,
    pub cache_stats: CacheStats,
    pub memory_stats: MemoryStats,
}

#[derive(Debug, Clone)]
pub struct CompilationStats {
    pub total_compilations: usize,
    pub average_time: Option<Duration>,
    pub slowest_time: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_executions: usize,
    pub most_expensive_function: Option<String>,
    pub most_called_function: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub peak_memory: Option<usize>,
    pub current_memory: Option<usize>,
}
