//! 编译器核心
//!
//! 事件驱动的编译器实现，提供细粒度的事件系统用于 IDE 和 LSP 集成。

use crate::middle;
use crate::util::i18n::{t_cur, MSG};
use thiserror::Error;
use tracing::debug;

use super::config::CompileConfig;
use super::events::*;
use super::pipeline::{Pipeline, PipelineState};

/// 编译器
///
/// 事件驱动的编译器实现，通过事件系统支持 IDE 和 LSP 集成。
///
/// # 示例
///
/// ```ignore
/// use yaoxiang::frontend::Compiler;
///
/// let mut compiler = Compiler::new();
/// let result = compiler.compile("test.yao", "let x = 42;")?;
/// ```
#[derive(Debug)]
pub struct Compiler {
    /// 编译配置
    config: CompileConfig,
    /// 编译流水线
    pipeline: Pipeline,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    /// 创建新编译器
    #[inline]
    pub fn new() -> Self {
        Self::with_config(CompileConfig::new())
    }

    /// 使用配置创建编译器
    #[inline]
    pub fn with_config(config: CompileConfig) -> Self {
        let pipeline = Pipeline::new(config.clone());
        Self { config, pipeline }
    }

    /// 获取编译配置
    #[inline]
    pub fn config(&self) -> &CompileConfig {
        &self.config
    }

    /// 获取流水线实例
    #[inline]
    pub fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    /// 获取可变的流水线实例
    #[inline]
    pub fn pipeline_mut(&mut self) -> &mut Pipeline {
        &mut self.pipeline
    }

    /// 订阅编译器事件
    ///
    /// 允许外部组件订阅编译器事件，用于 IDE 集成和进度显示。
    pub fn subscribe<S: EventSubscriber + 'static>(
        &self,
        subscriber: S,
    ) -> SubscriptionHandle {
        self.pipeline.subscribe(subscriber)
    }

    /// 编译源文件
    ///
    /// 对源文件进行完整的编译流程，包括词法分析、语法分析、类型检查和 IR 生成。
    ///
    /// # 参数
    ///
    /// - `source_name`: 源文件名（用于错误报告）
    /// - `source`: 源代码
    ///
    /// # 返回
    ///
    /// 成功返回 `ModuleIR`，失败返回 `CompileError`
    pub fn compile(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> Result<middle::ModuleIR, CompileError> {
        self.compile_with_source(source_name, source)
    }

    /// 编译源文件（带源名称）
    ///
    /// 与 `compile` 功能相同，但显式指定源文件名称。
    pub fn compile_with_source(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> Result<middle::ModuleIR, CompileError> {
        let source_len = source.len();
        debug!("{}", t_cur(MSG::CompilingSource, Some(&[&source_len])));

        let result = self.pipeline.run(source_name, source);

        if result.is_success() {
            Ok(result.ir.unwrap())
        } else {
            Err(CompileError::TypeError(result.errors.join("\n")))
        }
    }

    /// 只进行词法分析
    ///
    /// 对源代码进行词法分析，返回 token 列表。
    pub fn lex(
        &mut self,
        source: &str,
    ) -> Result<Vec<super::core::lexer::Token>, CompileError> {
        super::core::lexer::tokenize(source).map_err(|e| CompileError::LexError(e.to_string()))
    }

    /// 只进行语法分析
    ///
    /// 对 token 列表进行语法分析，返回 AST。
    pub fn parse(
        &mut self,
        tokens: &[super::core::lexer::Token],
    ) -> Result<super::core::parser::Module, CompileError> {
        super::core::parser::parse(tokens).map_err(|e| CompileError::ParseError(e.to_string()))
    }

    /// 只进行类型检查
    ///
    /// 对 AST 进行类型检查，返回类型检查结果。
    pub fn typecheck(
        &mut self,
        ast: &super::core::parser::Module,
    ) -> Result<super::typecheck::TypeCheckResult, Vec<super::typecheck::TypeError>> {
        super::typecheck::check_module(ast, None)
    }

    /// 生成 IR
    ///
    /// 根据 AST 和类型检查结果生成中间表示。
    pub fn generate_ir(
        &mut self,
        ast: &super::core::parser::Module,
        type_result: &super::typecheck::TypeCheckResult,
    ) -> Result<middle::ModuleIR, Vec<super::typecheck::TypeError>> {
        super::typecheck::generate_ir(ast, type_result)
    }

    /// 检查是否可以进行增量编译
    ///
    /// # 参数
    ///
    /// - `file`: 要检查的文件路径
    ///
    /// # 返回
    ///
    /// 如果可以增量编译返回 `true`
    #[inline]
    pub fn can_incremental_compile(
        &self,
        file: &std::path::Path,
    ) -> bool {
        self.pipeline.can_incremental_compile(file)
    }

    /// 获取当前编译状态
    #[inline]
    pub fn state(&self) -> PipelineState {
        self.pipeline.state()
    }

    /// 重置编译器状态
    #[inline]
    pub fn reset(&mut self) {
        self.pipeline.reset();
    }
}

/// 编译错误
///
/// 包含编译过程中可能出现的各种错误。
#[derive(Debug, Error)]
pub enum CompileError {
    /// 词法分析错误
    #[error("Lexical error: {0}")]
    LexError(String),

    /// 语法分析错误
    #[error("Parse error: {0}")]
    ParseError(String),

    /// 类型错误
    #[error("Type error: {0}")]
    TypeError(String),

    /// IR 生成错误
    #[error("IR generation error: {0}")]
    IRError(String),

    /// 取消编译
    #[error("Compilation cancelled")]
    Cancelled,

    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 编译进度信息
///
/// 用于报告编译进度。
#[derive(Debug, Clone)]
pub struct CompileProgress {
    /// 当前阶段
    pub phase: CompilationPhase,
    /// 进度百分比
    pub percentage: f64,
    /// 当前处理的行号
    pub current_line: usize,
    /// 总行数
    pub total_lines: usize,
    /// 状态消息
    pub message: String,
}

impl CompileProgress {
    /// 创建新的进度信息
    pub fn new(
        phase: CompilationPhase,
        percentage: f64,
        current_line: usize,
        total_lines: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            percentage,
            current_line,
            total_lines,
            message: message.into(),
        }
    }
}

/// 编译进度回调实现
///
/// 用于从编译器事件生成进度信息。
#[derive(Debug)]
pub struct ProgressReporter {
    last_phase: CompilationPhase,
    total_lines: usize,
}

impl ProgressReporter {
    /// 创建新的进度报告器
    pub fn new(total_lines: usize) -> Self {
        Self {
            last_phase: CompilationPhase::Full,
            total_lines,
        }
    }

    /// 从事件生成进度信息
    pub fn on_event<E: Event>(
        &self,
        event: &E,
    ) -> Option<CompileProgress> {
        match event.name() {
            "LexingStart" => Some(CompileProgress::new(
                CompilationPhase::Lexing,
                0.0,
                0,
                self.total_lines,
                "Starting lexing...",
            )),
            "LexingComplete" => Some(CompileProgress::new(
                CompilationPhase::Lexing,
                25.0,
                self.total_lines,
                self.total_lines,
                "Lexing complete",
            )),
            "ParsingStart" => Some(CompileProgress::new(
                CompilationPhase::Parsing,
                25.0,
                0,
                self.total_lines,
                "Starting parsing...",
            )),
            "ParsingComplete" => Some(CompileProgress::new(
                CompilationPhase::Parsing,
                50.0,
                self.total_lines,
                self.total_lines,
                "Parsing complete",
            )),
            "TypeCheckingStart" => Some(CompileProgress::new(
                CompilationPhase::TypeChecking,
                50.0,
                0,
                self.total_lines,
                "Starting type checking...",
            )),
            "TypeCheckingComplete" => Some(CompileProgress::new(
                CompilationPhase::TypeChecking,
                80.0,
                self.total_lines,
                self.total_lines,
                "Type checking complete",
            )),
            "IRGenerationStart" => Some(CompileProgress::new(
                CompilationPhase::IRGeneration,
                80.0,
                0,
                self.total_lines,
                "Starting IR generation...",
            )),
            "IRGenerationComplete" => Some(CompileProgress::new(
                CompilationPhase::IRGeneration,
                100.0,
                self.total_lines,
                self.total_lines,
                "IR generation complete",
            )),
            _ => None,
        }
    }
}
