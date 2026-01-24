//! 增量编译器
//!
//! 持久化编译器实例，支持状态保持和增量编译

use std::sync::{Arc, RwLock, Mutex};
use std::time::Instant;

use crate::backends::dev::tui_repl::engine::module_builder::ModuleBuilder;
use crate::backends::dev::tui_repl::engine::symbol_cache::SymbolCache;
use crate::backends::dev::tui_repl::engine::profiler::Profiler;
use crate::backends::interpreter::Interpreter;
use crate::backends::Executor;
use crate::frontend::Compiler;
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
    /// 是否需要更多输入
    pub need_more_input: bool,
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

    /// 前端编译器
    compiler: Arc<RwLock<Compiler>>,
    /// 解释器 Backend
    interpreter: Arc<RwLock<Interpreter>>,
    /// 标准输出缓冲区
    stdout_buffer: Arc<Mutex<Vec<u8>>>,
    /// 输入缓冲区
    input_buffer: Arc<RwLock<String>>,
}

impl IncrementalCompiler {
    /// 创建新的增量编译器
    pub fn new() -> Result<Self> {
        let module_builder = Arc::new(RwLock::new(ModuleBuilder::new()?));
        let symbol_cache = Arc::new(RwLock::new(SymbolCache::new()));
        let profiler = Arc::new(RwLock::new(Profiler::new()));
        let stats = Arc::new(RwLock::new(CompilationStats::new()));

        // 初始化真实编译器组件
        let compiler = Arc::new(RwLock::new(Compiler::new()));
        let mut interp = Interpreter::new();
        let stdout_buffer = Arc::new(Mutex::new(Vec::new()));
        interp.set_stdout(stdout_buffer.clone());
        let interpreter = Arc::new(RwLock::new(interp));
        let input_buffer = Arc::new(RwLock::new(String::new()));

        Ok(Self {
            module_builder,
            symbol_cache,
            profiler,
            stats,
            compiler,
            interpreter,
            stdout_buffer,
            input_buffer,
        })
    }

    /// 读取并清空 stdout
    pub fn read_stdout(&self) -> String {
        if let Ok(mut buffer) = self.stdout_buffer.lock() {
            let output = String::from_utf8_lossy(&buffer).to_string();
            buffer.clear();
            output
        } else {
            String::new()
        }
    }

    /// 检查输入是否完整
    fn is_input_complete(
        &self,
        input: &str,
    ) -> bool {
        let mut parens = 0;
        let mut braces = 0;
        let mut brackets = 0;
        let mut in_string = false;
        let mut escape = false;

        for c in input.chars() {
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
                continue;
            }
            if c == '"' {
                in_string = !in_string;
                continue;
            }
            if in_string {
                continue;
            }

            match c {
                '(' => parens += 1,
                ')' => {
                    if parens > 0 {
                        parens -= 1
                    }
                }
                '{' => braces += 1,
                '}' => {
                    if braces > 0 {
                        braces -= 1
                    }
                }
                '[' => brackets += 1,
                ']' => {
                    if brackets > 0 {
                        brackets -= 1
                    }
                }
                _ => {}
            }
        }

        parens == 0 && braces == 0 && brackets == 0 && !in_string
    }

    /// 增量编译代码
    pub fn compile(
        &self,
        input: &str,
    ) -> Result<CompilationResult> {
        let start_time = Instant::now();

        // 1. 累积输入
        let full_source = {
            let mut buf = self.input_buffer.write().unwrap();
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(input);
            buf.clone()
        };

        // 2. 检查完整性
        if !self.is_input_complete(&full_source) {
            return Ok(CompilationResult {
                success: true,
                duration: start_time.elapsed(),
                error: None,
                statement_count: 0,
                need_more_input: true,
            });
        }

        // 3. 编译与执行
        let source_to_compile = full_source.clone();

        // 实际上每次都必须重新编译整个模块，因为目前的编译器不支持增量
        // 但这里我们简单地假设 input 是一个完整的代码块

        let compile_result = {
            let mut compiler = self.compiler.write().unwrap();
            compiler.compile(&source_to_compile)
        };

        match compile_result {
            Ok(module_ir) => {
                let mut ctx = crate::middle::codegen::CodegenContext::new(module_ir);
                match ctx.generate() {
                    Ok(bytecode_file) => {
                        let bytecode_module =
                            crate::middle::bytecode::BytecodeModule::from(bytecode_file);

                        let mut run_success = true;
                        let mut run_error = None;

                        {
                            let mut interpreter = self.interpreter.write().unwrap();
                            // 执行模块
                            if let Err(e) = interpreter.execute_module(&bytecode_module) {
                                run_success = false;
                                run_error = Some(format!("Runtime Error: {:?}", e));
                            }
                        }

                        // 清空输入缓冲区
                        self.input_buffer.write().unwrap().clear();

                        // 更新统计
                        {
                            let mut stats = self.stats.write().unwrap();
                            stats.total_compilations += 1;
                            if run_success {
                                stats.successful_compilations += 1;
                            } else {
                                stats.failed_compilations += 1;
                            }
                            stats.total_time += start_time.elapsed();
                        }

                        Ok(CompilationResult {
                            success: run_success,
                            duration: start_time.elapsed(),
                            error: run_error,
                            statement_count: 1,
                            need_more_input: false,
                        })
                    }
                    Err(e) => {
                        // Codegen Error
                        self.input_buffer.write().unwrap().clear();
                        {
                            let mut stats = self.stats.write().unwrap();
                            stats.total_compilations += 1;
                            stats.failed_compilations += 1;
                        }

                        Ok(CompilationResult {
                            success: false,
                            duration: start_time.elapsed(),
                            error: Some(format!("Codegen Error: {:?}", e)),
                            statement_count: 0,
                            need_more_input: false,
                        })
                    }
                }
            }
            Err(e) => {
                // Compile Error
                self.input_buffer.write().unwrap().clear();
                {
                    let mut stats = self.stats.write().unwrap();
                    stats.total_compilations += 1;
                    stats.failed_compilations += 1;
                }

                Ok(CompilationResult {
                    success: false,
                    duration: start_time.elapsed(),
                    error: Some(format!("{}", e)),
                    statement_count: 0,
                    need_more_input: false,
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
