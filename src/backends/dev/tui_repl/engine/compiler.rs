//! 增量编译器
//!
//! 持久化编译器实例，支持状态保持和增量编译

use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::backends::dev::tui_repl::engine::module_builder::ModuleBuilder;
use crate::backends::dev::tui_repl::engine::symbol_cache::SymbolCache;
use crate::backends::dev::tui_repl::engine::profiler::Profiler;
use crate::Result;

/// 编译结果
#[derive(Debug)]
pub struct CompilationResult {
    /// 编译是否成功
    pub success: bool,
    /// 编译耗时
    pub duration: std::time::Duration,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// 编译的语句数量
    pub statement_count: usize,
}

/// 增量编译器
pub struct IncrementalCompiler {
    /// 模块构建器
    module_builder: Arc<RwLock<ModuleBuilder>>,
    /// 符号缓存
    symbol_cache: Arc<RwLock<SymbolCache>>,
    /// 性能分析器
    profiler: Arc<RwLock<Profiler>>,
    /// 编译统计
    stats: Arc<RwLock<CompilationStats>>,
}

impl IncrementalCompiler {
    /// 创建新的增量编译器
    pub fn new() -> Result<Self> {
        let module_builder = Arc::new(RwLock::new(ModuleBuilder::new()?));
        let symbol_cache = Arc::new(RwLock::new(SymbolCache::new()));
        let profiler = Arc::new(RwLock::new(Profiler::new()));
        let stats = Arc::new(RwLock::new(CompilationStats::new()));

        Ok(Self {
            module_builder,
            symbol_cache,
            profiler,
            stats,
        })
    }

    /// 增量编译代码
    pub fn compile(
        &self,
        source: &str,
    ) -> Result<CompilationResult> {
        let start_time = Instant::now();

        // 获取模块构建器锁
        let mut builder = self.module_builder.write().map_err(|e| {
            std::io::Error::other(format!("Failed to acquire module builder lock: {}", e))
        })?;

        // 尝试增量编译
        match builder.add_statement(source) {
            Ok(()) => {
                let duration = start_time.elapsed();

                // 更新统计
                {
                    let mut stats = self.stats.write().unwrap();
                    stats.total_compilations += 1;
                    stats.successful_compilations += 1;
                    stats.total_time += duration;
                }

                // 记录性能
                {
                    let mut profiler = self.profiler.write().unwrap();
                    profiler.record_compilation(duration, source.len());
                }

                Ok(CompilationResult {
                    success: true,
                    duration,
                    error: None,
                    statement_count: builder.statement_count(),
                })
            }
            Err(e) => {
                let duration = start_time.elapsed();

                // 更新统计
                {
                    let mut stats = self.stats.write().unwrap();
                    stats.total_compilations += 1;
                    stats.failed_compilations += 1;
                    stats.total_time += duration;
                }

                Ok(CompilationResult {
                    success: false,
                    duration,
                    error: Some(format!("Compilation error: {}", e)),
                    statement_count: builder.statement_count(),
                })
            }
        }
    }

    /// 获取符号缓存
    pub fn symbol_cache(&self) -> Arc<RwLock<SymbolCache>> {
        Arc::clone(&self.symbol_cache)
    }

    /// 获取性能分析器
    pub fn profiler(&self) -> Arc<RwLock<Profiler>> {
        Arc::clone(&self.profiler)
    }

    /// 获取编译统计
    pub fn stats(&self) -> Arc<RwLock<CompilationStats>> {
        Arc::clone(&self.stats)
    }

    /// 清空所有状态
    pub fn reset(&self) -> Result<()> {
        // 重置模块构建器
        let mut builder = self.module_builder.write().map_err(|e| {
            std::io::Error::other(format!("Failed to acquire module builder lock: {}", e))
        })?;
        builder.reset()?;

        // 清空符号缓存
        {
            let mut cache = self.symbol_cache.write().unwrap();
            cache.clear();
        }

        // 重置性能分析器
        {
            let mut profiler = self.profiler.write().unwrap();
            profiler.reset();
        }

        // 重置统计
        {
            let mut stats = self.stats.write().unwrap();
            stats.reset();
        }

        Ok(())
    }

    /// 获取当前模块状态摘要
    pub fn get_module_summary(&self) -> ModuleSummary {
        let builder = self.module_builder.read().unwrap();
        let stats = self.stats.read().unwrap();

        ModuleSummary {
            statement_count: builder.statement_count(),
            symbol_count: builder.symbol_count(),
            total_compilations: stats.total_compilations,
            successful_compilations: stats.successful_compilations,
            failed_compilations: stats.failed_compilations,
            average_compilation_time: if stats.total_compilations > 0 {
                stats.total_time / stats.total_compilations as u32
            } else {
                std::time::Duration::from_secs(0)
            },
        }
    }
}

impl Default for IncrementalCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create IncrementalCompiler")
    }
}

/// 模块摘要
#[derive(Debug, Clone)]
pub struct ModuleSummary {
    pub statement_count: usize,
    pub symbol_count: usize,
    pub total_compilations: u64,
    pub successful_compilations: u64,
    pub failed_compilations: u64,
    pub average_compilation_time: std::time::Duration,
}

/// 编译统计
#[derive(Debug)]
pub struct CompilationStats {
    pub total_compilations: u64,
    pub successful_compilations: u64,
    pub failed_compilations: u64,
    pub total_time: std::time::Duration,
}

impl CompilationStats {
    fn new() -> Self {
        Self {
            total_compilations: 0,
            successful_compilations: 0,
            failed_compilations: 0,
            total_time: std::time::Duration::from_secs(0),
        }
    }

    fn reset(&mut self) {
        self.total_compilations = 0;
        self.successful_compilations = 0;
        self.failed_compilations = 0;
        self.total_time = std::time::Duration::from_secs(0);
    }
}
