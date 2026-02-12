//! 编译流水线
//!
//! 管理编译状态机、执行编译流程、处理错误恢复。

use crate::middle;
use crate::util::span::SourceFile;
use crate::util::diagnostic::Diagnostic;
use super::{config::CompileConfig, events::*, typecheck};

/// 管道错误类型
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// 词法/解析错误
    LexParse(String),
    /// 类型检查错误
    TypeCheck(Diagnostic),
    /// IR 生成错误
    IRGeneration(String),
}

impl fmt::Display for PipelineError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            PipelineError::LexParse(msg) => write!(f, "{}", msg),
            PipelineError::TypeCheck(err) => write!(f, "{}", err.message),
            PipelineError::IRGeneration(msg) => write!(f, "{}", msg),
        }
    }
}

impl PipelineError {
    /// 获取诊断信息（如果是类型检查错误）
    pub fn diagnostic(&self) -> Option<Diagnostic> {
        match self {
            PipelineError::TypeCheck(err) => Some(err.clone()),
            _ => None,
        }
    }
}
use std::path::{Path, PathBuf};

/// 流水线状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    /// 空闲状态
    Idle,
    /// 词法分析中
    Lexing,
    /// 语法分析中
    Parsing,
    /// 类型检查中
    TypeChecking,
    /// IR 生成中
    IRGenerating,
    /// 编译完成
    Completed,
    /// 编译失败
    Failed,
    /// 被取消
    Cancelled,
}

impl std::fmt::Display for PipelineState {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            PipelineState::Idle => write!(f, "idle"),
            PipelineState::Lexing => write!(f, "lexing"),
            PipelineState::Parsing => write!(f, "parsing"),
            PipelineState::TypeChecking => write!(f, "type checking"),
            PipelineState::IRGenerating => write!(f, "IR generating"),
            PipelineState::Completed => write!(f, "completed"),
            PipelineState::Failed => write!(f, "failed"),
            PipelineState::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// 编译结果
#[derive(Debug, Clone)]
pub struct CompilationResult {
    /// 最终状态
    pub state: PipelineState,
    /// 生成的 IR
    pub ir: Option<middle::ModuleIR>,
    /// 错误数量
    pub error_count: usize,
    /// 警告数量
    pub warning_count: usize,
    /// 各阶段耗时（毫秒）
    pub phase_durations: Vec<(CompilationPhase, u64)>,
    /// 总耗时（毫秒）
    pub total_duration_ms: u64,
    /// 错误
    pub errors: Vec<PipelineError>,
    /// 警告消息
    pub warnings: Vec<String>,
}

impl Default for CompilationResult {
    fn default() -> Self {
        Self {
            state: PipelineState::Idle,
            ir: None,
            error_count: 0,
            warning_count: 0,
            phase_durations: Vec::new(),
            total_duration_ms: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl CompilationResult {
    /// 创建成功结果
    pub fn success(
        ir: middle::ModuleIR,
        durations: Vec<(CompilationPhase, u64)>,
        total_ms: u64,
    ) -> Self {
        Self {
            state: PipelineState::Completed,
            ir: Some(ir),
            error_count: 0,
            warning_count: 0,
            phase_durations: durations,
            total_duration_ms: total_ms,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// 创建失败结果
    pub fn failed(
        errors: Vec<PipelineError>,
        durations: Vec<(CompilationPhase, u64)>,
        total_ms: u64,
    ) -> Self {
        Self {
            state: PipelineState::Failed,
            ir: None,
            error_count: errors.len(),
            warning_count: 0,
            phase_durations: durations,
            total_duration_ms: total_ms,
            errors,
            warnings: Vec::new(),
        }
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.state == PipelineState::Completed && self.error_count == 0
    }
}

/// 编译进度回调
pub trait ProgressCallback: Send + Sync {
    fn on_progress(
        &self,
        state: PipelineState,
        progress: f64,
    );
}

use std::fmt;

/// 编译流水线
pub struct Pipeline {
    /// 当前状态
    state: PipelineState,
    /// 配置
    config: CompileConfig,
    /// 事件总线
    event_bus: EventBus,
    /// 缓存目录（用于增量编译）
    cache_dir: Option<PathBuf>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new(CompileConfig::default())
    }
}

impl fmt::Debug for Pipeline {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("Pipeline")
            .field("state", &self.state)
            .field("config", &self.config)
            .finish()
    }
}

impl Pipeline {
    /// 创建新流水线
    pub fn new(config: CompileConfig) -> Self {
        Self {
            state: PipelineState::Idle,
            config,
            event_bus: EventBus::new(),
            cache_dir: None,
        }
    }

    /// 创建带事件总线的流水线
    pub fn with_event_bus(
        config: CompileConfig,
        event_bus: EventBus,
    ) -> Self {
        Self {
            state: PipelineState::Idle,
            config,
            event_bus,
            cache_dir: None,
        }
    }

    /// 获取当前状态
    #[inline]
    pub fn state(&self) -> PipelineState {
        self.state
    }

    /// 获取配置
    #[inline]
    pub fn config(&self) -> &CompileConfig {
        &self.config
    }

    /// 获取事件总线
    #[inline]
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// 获取可变事件总线
    #[inline]
    pub fn event_bus_mut(&mut self) -> &mut EventBus {
        &mut self.event_bus
    }

    /// 设置缓存目录
    #[inline]
    pub fn set_cache_dir(
        &mut self,
        dir: PathBuf,
    ) {
        self.cache_dir = Some(dir);
    }

    /// 订阅事件
    pub fn subscribe<S: EventSubscriber + 'static>(
        &self,
        subscriber: S,
    ) -> SubscriptionHandle {
        self.event_bus.subscribe(subscriber)
    }

    /// 运行完整编译流程
    pub fn run(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> CompilationResult {
        let start_time = std::time::Instant::now();
        let mut phase_durations = Vec::new();

        // 发射编译开始事件
        self.event_bus.emit(CompilationStart::new(
            source_name,
            source.len(),
            self.config.incremental.enabled,
        ));

        // 执行各阶段
        let lex_result = self.run_lexing(source_name, source, &mut phase_durations);
        if !lex_result.is_success() {
            return CompilationResult::failed(
                lex_result
                    .errors
                    .into_iter()
                    .map(PipelineError::LexParse)
                    .collect(),
                phase_durations,
                start_time.elapsed().as_millis() as u64,
            );
        }

        let parse_result = self.run_parsing(source_name, &lex_result.tokens, &mut phase_durations);
        if !parse_result.is_success() {
            return CompilationResult::failed(
                parse_result
                    .errors
                    .into_iter()
                    .map(PipelineError::LexParse)
                    .collect(),
                phase_durations,
                start_time.elapsed().as_millis() as u64,
            );
        }

        let typecheck_result =
            self.run_typecheck(source_name, source, &parse_result.ast, &mut phase_durations);
        if !typecheck_result.is_success() {
            return CompilationResult::failed(
                typecheck_result
                    .errors
                    .into_iter()
                    .map(PipelineError::TypeCheck)
                    .collect(),
                phase_durations,
                start_time.elapsed().as_millis() as u64,
            );
        }

        let ir_result = self.run_ir_generation(
            source_name,
            source,
            &parse_result.ast,
            &typecheck_result.type_result,
            &mut phase_durations,
        );

        let total_ms = start_time.elapsed().as_millis() as u64;

        // 发射编译完成事件
        self.event_bus.emit(CompilationComplete::new(
            ir_result.is_success(),
            total_ms,
            phase_durations.clone(),
        ));

        if ir_result.is_success() {
            CompilationResult::success(ir_result.ir.unwrap(), phase_durations, total_ms)
        } else {
            // IR 生成错误被归类为类型检查错误
            let pipeline_errors: Vec<PipelineError> = ir_result
                .errors
                .into_iter()
                .map(PipelineError::TypeCheck)
                .collect();
            CompilationResult::failed(pipeline_errors, phase_durations, total_ms)
        }
    }

    /// 词法分析阶段
    fn run_lexing(
        &mut self,
        source_name: &str,
        source: &str,
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> LexResult {
        let start = std::time::Instant::now();
        self.state = PipelineState::Lexing;

        self.event_bus
            .emit(LexingStart::new(source_name, source.len()));

        let tokens = match super::core::lexer::tokenize(source) {
            Ok(tokens) => tokens,
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::Lexing, duration));

                self.event_bus.emit(LexingComplete::new(0, duration));
                self.event_bus.emit(ErrorOccurred::new(
                    e.to_string(),
                    "E0100",
                    ErrorLevel::Error,
                ));

                return LexResult::failed(vec![e.to_string()]);
            }
        };

        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::Lexing, duration));

        self.event_bus
            .emit(LexingComplete::new(tokens.len(), duration));

        LexResult::success(tokens)
    }

    /// 语法分析阶段
    fn run_parsing(
        &mut self,
        _source_name: &str,
        tokens: &[super::core::lexer::Token],
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> ParseResult {
        let start = std::time::Instant::now();
        self.state = PipelineState::Parsing;

        self.event_bus.emit(ParsingStart::new(tokens.len()));

        let ast = match super::core::parser::parse(tokens) {
            Ok(ast) => ast,
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::Parsing, duration));

                self.event_bus.emit(ParsingComplete::new(0, duration));
                self.event_bus.emit(ErrorOccurred::new(
                    e.to_string(),
                    "E0200",
                    ErrorLevel::Error,
                ));

                return ParseResult::failed(vec![e.to_string()]);
            }
        };

        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::Parsing, duration));

        self.event_bus
            .emit(ParsingComplete::new(ast.items.len(), duration));

        ParseResult::success(ast)
    }

    /// 类型检查阶段
    fn run_typecheck(
        &mut self,
        source_name: &str,
        source: &str,
        ast: &super::core::parser::Module,
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> TypecheckResult {
        let start = std::time::Instant::now();
        self.state = PipelineState::TypeChecking;

        self.event_bus
            .emit(TypeCheckingStart::new(source_name, ast.items.len()));

        // 预留：用于后续增量编译的诊断格式化
        let _source_file = SourceFile::new(source_name.to_string(), source.to_string());
        let _ = _source_file;

        match typecheck::check_module(ast, &mut None) {
            Ok(type_result) => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::TypeChecking, duration));

                self.event_bus.emit(TypeCheckingComplete::new(
                    type_result.bindings.len(),
                    0, // errors
                    0, // warnings
                    duration,
                ));

                TypecheckResult::success(type_result)
            }
            Err(errors) => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::TypeChecking, duration));

                // 保留原始 Diagnostic，同时发送字符串消息给事件总线
                let error_messages: Vec<String> =
                    errors.iter().map(|e| e.message.clone()).collect();

                self.event_bus.emit(TypeCheckingComplete::new(
                    0,
                    error_messages.len(),
                    0,
                    duration,
                ));

                for err in &error_messages {
                    self.event_bus.emit(ErrorOccurred::new(
                        err.clone(),
                        "E0300",
                        ErrorLevel::Error,
                    ));
                }

                TypecheckResult::failed(errors)
            }
        }
    }

    /// IR 生成阶段
    fn run_ir_generation(
        &mut self,
        source_name: &str,
        source: &str,
        ast: &super::core::parser::Module,
        type_result: &typecheck::TypeCheckResult,
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> IRResult {
        let start = std::time::Instant::now();
        self.state = PipelineState::IRGenerating;

        self.event_bus.emit(IRGenerationStart::new(ast.items.len()));

        // 预留：用于后续增量编译的诊断格式化
        let _source_file = SourceFile::new(source_name.to_string(), source.to_string());
        let _ = _source_file;

        match middle::generate_ir(ast, type_result) {
            Ok(ir) => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::IRGeneration, duration));

                self.event_bus.emit(IRGenerationComplete::new(
                    std::mem::size_of_val(&ir),
                    ast.items.len(),
                    duration,
                ));

                IRResult::success(ir)
            }
            Err(errors) => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::IRGeneration, duration));

                let error_messages: Vec<String> = errors.iter().map(|e| format!("{}", e)).collect();

                self.event_bus
                    .emit(IRGenerationComplete::new(0, 0, duration));

                for err in &error_messages {
                    self.event_bus.emit(ErrorOccurred::new(
                        err.clone(),
                        "E0400",
                        ErrorLevel::Error,
                    ));
                }
                // IR 生成错误被归类为类型检查错误（因为它们源于类型检查）
                IRResult::failed(errors)
            }
        }
    }

    /// 检查是否可以进行增量编译
    pub fn can_incremental_compile(
        &self,
        file: &Path,
    ) -> bool {
        if !self.config.incremental.enabled {
            return false;
        }

        if let Some(ref cache_dir) = self.cache_dir {
            let cache_file = cache_dir.join(file.file_stem().unwrap_or_default());
            cache_file.exists() && cache_file.is_file()
        } else {
            false
        }
    }

    /// 获取缓存的编译结果
    pub fn get_cached_result(
        &self,
        _file: &Path,
    ) -> Option<CompilationResult> {
        // TODO: 实现增量编译缓存逻辑
        None
    }

    /// 重置流水线状态
    pub fn reset(&mut self) {
        self.state = PipelineState::Idle;
    }
}

/// 词法分析结果
struct LexResult {
    tokens: Vec<super::core::lexer::Token>,
    errors: Vec<String>,
}

impl LexResult {
    fn success(tokens: Vec<super::core::lexer::Token>) -> Self {
        Self {
            tokens,
            errors: Vec::new(),
        }
    }

    fn failed(errors: Vec<String>) -> Self {
        Self {
            tokens: Vec::new(),
            errors,
        }
    }

    fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// 语法分析结果
struct ParseResult {
    ast: super::core::parser::Module,
    errors: Vec<String>,
}

impl ParseResult {
    fn success(ast: super::core::parser::Module) -> Self {
        Self {
            ast,
            errors: Vec::new(),
        }
    }

    fn failed(errors: Vec<String>) -> Self {
        Self {
            ast: super::core::parser::Module::default(),
            errors,
        }
    }

    fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// 类型检查结果
struct TypecheckResult {
    type_result: typecheck::TypeCheckResult,
    errors: Vec<Diagnostic>,
}

impl TypecheckResult {
    fn success(type_result: typecheck::TypeCheckResult) -> Self {
        Self {
            type_result,
            errors: Vec::new(),
        }
    }

    fn failed(errors: Vec<Diagnostic>) -> Self {
        Self {
            type_result: typecheck::TypeCheckResult::default(),
            errors,
        }
    }

    fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// IR 生成结果
struct IRResult {
    ir: Option<middle::ModuleIR>,
    errors: Vec<Diagnostic>,
}

impl IRResult {
    fn success(ir: middle::ModuleIR) -> Self {
        Self {
            ir: Some(ir),
            errors: Vec::new(),
        }
    }

    fn failed(errors: Vec<Diagnostic>) -> Self {
        Self { ir: None, errors }
    }

    fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}
