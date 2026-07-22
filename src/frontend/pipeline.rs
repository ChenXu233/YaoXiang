//! 编译流水线
//!
//! 管理编译状态机、执行编译流程、处理错误恢复。

pub mod compilation_cache;
pub mod incremental_scheduler;

use crate::middle;
use crate::util::span::SourceFile;
use crate::util::diagnostic::Diagnostic;
use super::{config::CompileConfig, core::typecheck};

use compilation_cache::CompilationCache;
use incremental_scheduler::IncrementalStats;

/// 管道错误类型
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// 词法/解析错误
    LexParse(String),
    /// 类型检查错误
    TypeCheck(Diagnostic),
    /// IR 生成错误
    IRGeneration(String),
    /// 证明函数执行错误（RFC-027 Phase 2.5）
    ProofExecution(String),
}

impl fmt::Display for PipelineError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            PipelineError::LexParse(msg) => write!(f, "{}", msg),
            PipelineError::TypeCheck(err) => write!(f, "{}", err),
            PipelineError::IRGeneration(msg) => write!(f, "{}", msg),
            PipelineError::ProofExecution(msg) => write!(f, "{}", msg),
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

/// 编译阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationPhase {
    /// 词法分析
    Lexing,
    /// 语法分析
    Parsing,
    /// 类型检查
    TypeChecking,
    /// IR 生成
    IRGeneration,
    /// 证明函数执行（RFC-027 Phase 2.5）
    ProofExecution,
    /// 完整编译
    Full,
}

impl std::fmt::Display for CompilationPhase {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            CompilationPhase::Lexing => write!(f, "lexing"),
            CompilationPhase::Parsing => write!(f, "parsing"),
            CompilationPhase::TypeChecking => write!(f, "type checking"),
            CompilationPhase::IRGeneration => write!(f, "IR generation"),
            CompilationPhase::ProofExecution => write!(f, "proof execution"),
            CompilationPhase::Full => write!(f, "full compilation"),
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
    /// 证明函数执行中（RFC-027 Phase 2.5）
    ProofExecuting,
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
            PipelineState::ProofExecuting => write!(f, "proof executing"),
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
        warnings: Vec<String>,
    ) -> Self {
        Self {
            state: PipelineState::Completed,
            ir: Some(ir),
            error_count: 0,
            warning_count: warnings.len(),
            phase_durations: durations,
            total_duration_ms: total_ms,
            errors: Vec::new(),
            warnings,
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
    /// 缓存目录（用于增量编译）
    cache_dir: Option<PathBuf>,
    /// 编译缓存（内存）
    compilation_cache: CompilationCache,
    /// 增量编译统计
    incremental_stats: IncrementalStats,
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
        let cache = CompilationCache::with_config(
            config.incremental.cache_ttl,
            (config.incremental.max_cache_size / 1024) as usize, // 粗略估计条目数
        );
        Self {
            state: PipelineState::Idle,
            config,
            cache_dir: None,
            compilation_cache: cache,
            incremental_stats: IncrementalStats::default(),
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

    /// 设置缓存目录
    #[inline]
    pub fn set_cache_dir(
        &mut self,
        dir: PathBuf,
    ) {
        self.cache_dir = Some(dir);
    }

    /// 运行完整编译流程
    pub fn run(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> CompilationResult {
        let start_time = crate::util::time_compat::Instant::now();
        let mut phase_durations = Vec::new();

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

        // RFC-027 Phase 2.5: 证明函数执行循环
        // 在类型检查通过后、IR 生成前，执行编译期证明函数
        if !typecheck_result.type_result.proof_calls.is_empty() {
            let proof_result = self.run_proof_execution(
                &typecheck_result.type_result.proof_calls,
                &parse_result.ast,
                &typecheck_result.type_result,
                &mut phase_durations,
            );
            if !proof_result.is_success() {
                return CompilationResult::failed(
                    proof_result
                        .errors
                        .into_iter()
                        .map(PipelineError::ProofExecution)
                        .collect(),
                    phase_durations,
                    start_time.elapsed().as_millis() as u64,
                );
            }
        }

        let ir_result = self.run_ir_generation(
            source_name,
            source,
            &parse_result.ast,
            &typecheck_result.type_result,
            &mut phase_durations,
        );

        let total_ms = start_time.elapsed().as_millis() as u64;

        if ir_result.is_success() {
            // 收集所有警告（来自 typecheck 阶段）
            let warnings = typecheck_result.warnings;
            CompilationResult::success(ir_result.ir.unwrap(), phase_durations, total_ms, warnings)
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
        _source_name: &str,
        source: &str,
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> LexResult {
        let start = crate::util::time_compat::Instant::now();
        self.state = PipelineState::Lexing;

        let tokens = match super::core::lexer::tokenize(source) {
            Ok(tokens) => tokens,
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::Lexing, duration));

                return LexResult::failed(vec![e.to_string()]);
            }
        };

        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::Lexing, duration));

        LexResult::success(tokens)
    }

    /// 语法分析阶段
    fn run_parsing(
        &mut self,
        _source_name: &str,
        tokens: &[super::core::lexer::Token],
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> ParseResult {
        let start = crate::util::time_compat::Instant::now();
        self.state = PipelineState::Parsing;

        let ast = match super::core::parser::parse(tokens) {
            result if result.has_errors => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::Parsing, duration));

                let error_msg = result
                    .errors
                    .into_iter()
                    .next()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Unknown parse error".to_string());

                return ParseResult::failed(vec![error_msg]);
            }
            result => result.module,
        };

        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::Parsing, duration));

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
        let start = crate::util::time_compat::Instant::now();
        self.state = PipelineState::TypeChecking;

        // 预留：用于后续增量编译的诊断格式化
        let _source_file = SourceFile::new(source_name.to_string(), source.to_string());
        let _ = _source_file;

        let mut type_result = typecheck::check_module(ast, &mut None);
        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::TypeChecking, duration));
        let has_errors = !type_result.diagnostics.is_empty();
        let errors = std::mem::take(&mut type_result.diagnostics);

        // 执行死代码分析（根据配置决定是否启用）
        let warnings = if self.config.dead_code.enabled && !has_errors {
            self.run_dead_code_analysis(source_name, ast, &type_result.semantic_db)
        } else {
            Vec::new()
        };

        TypecheckResult {
            type_result,
            errors,
            warnings,
        }
    }

    /// 死代码分析阶段
    fn run_dead_code_analysis(
        &mut self,
        _source_name: &str,
        ast: &super::core::parser::Module,
        semantic_db: &typecheck::semantic_db::SemanticDB,
    ) -> Vec<String> {
        use crate::frontend::core::typecheck::passes::dead_code::DeadCodeAnalyzer;

        let mut analyzer = DeadCodeAnalyzer::new();
        let warnings = analyzer.analyze(ast, semantic_db);

        // 渲染警告消息
        warnings
            .iter()
            .map(|w| format!("warning [{}]: {} at {:?}", w.code, w.message, w.span))
            .collect()
    }

    /// 证明函数执行阶段（RFC-027 Phase 2.5）
    ///
    /// 类型检查后、IR 生成前，执行编译期证明函数。
    /// 每个证明函数被编译为字节码并在解释器中执行，返回 bool 结果。
    /// 任何返回 false 的证明函数都会导致编译失败。
    fn run_proof_execution(
        &mut self,
        proof_calls: &[typecheck::proof::verdict::ProofFunctionCall],
        ast: &super::core::parser::ast::Module,
        type_result: &typecheck::TypeCheckResult,
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> ProofExecResult {
        let start = crate::util::time_compat::Instant::now();
        self.state = PipelineState::ProofExecuting;

        let mut failed_proofs = Vec::new();
        let mut errors = Vec::new();

        for call in proof_calls {
            match execute_single_proof_fn(call, ast, type_result) {
                Ok(true) => {
                    // 证明通过，继续
                }
                Ok(false) => {
                    failed_proofs.push(call.func_name.clone());
                    errors.push(format!(
                        "证明函数 '{}' 返回 false，约束不满足（参数: {:?}）",
                        call.func_name, call.args,
                    ));
                }
                Err(e) => {
                    failed_proofs.push(call.func_name.clone());
                    errors.push(format!("证明函数 '{}' 执行失败: {}", call.func_name, e,));
                }
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::ProofExecution, duration));

        if failed_proofs.is_empty() {
            ProofExecResult::success()
        } else {
            ProofExecResult::failed(failed_proofs, errors)
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
        let start = crate::util::time_compat::Instant::now();
        self.state = PipelineState::IRGenerating;

        // 预留：用于后续增量编译的诊断格式化
        let _source_file = SourceFile::new(source_name.to_string(), source.to_string());
        let _ = _source_file;

        match middle::generate_ir(ast, type_result) {
            Ok(mut ir) => {
                // 单态化（根据配置决定是否启用）
                if self.config.mono.enabled && !type_result.instantiation_requests.is_empty() {
                    let mut mono = middle::passes::mono::Monomorphizer::with_max_depth(
                        self.config.mono.max_depth,
                    );
                    match mono.monomorphize(&ir, &type_result.instantiation_requests) {
                        Ok(mono_ir) => ir = mono_ir,
                        Err(diag) => return IRResult::failed(vec![diag]),
                    }
                }

                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::IRGeneration, duration));

                IRResult::success(ir)
            }
            Err(errors) => {
                let duration = start.elapsed().as_millis() as u64;
                phase_durations.push((CompilationPhase::IRGeneration, duration));

                // IR 生成错误被归类为类型检查错误（因为它们源于类型检查）
                IRResult::failed(errors)
            }
        }
    }

    /// 检查是否可以进行增量编译
    pub fn can_incremental_compile(
        &self,
        file: &Path,
        source: &str,
    ) -> bool {
        if !self.config.incremental.enabled {
            return false;
        }

        self.compilation_cache.has_valid_cache(file, source)
    }

    /// 获取缓存的编译结果
    pub fn get_cached_result(
        &mut self,
        file: &Path,
        source: &str,
    ) -> Option<CompilationResult> {
        if !self.config.incremental.enabled {
            return None;
        }

        let entry = self.compilation_cache.get(file, source)?;
        let ir = entry.ir.clone()?;

        Some(CompilationResult::success(ir, Vec::new(), 0, Vec::new()))
    }

    /// 获取编译缓存的引用
    pub fn compilation_cache(&self) -> &CompilationCache {
        &self.compilation_cache
    }

    /// 获取编译缓存的可变引用
    pub fn compilation_cache_mut(&mut self) -> &mut CompilationCache {
        &mut self.compilation_cache
    }

    /// 获取增量编译统计
    pub fn incremental_stats(&self) -> &IncrementalStats {
        &self.incremental_stats
    }

    /// 运行编译并缓存结果
    pub fn run_and_cache(
        &mut self,
        source_name: &str,
        source: &str,
        file: PathBuf,
    ) -> CompilationResult {
        let result = self.run(source_name, source);

        // 缓存编译产物
        if result.is_success() {
            self.compilation_cache.store(
                file,
                source,
                None, // AST 不在最终结果中（已被消耗）
                None, // TypeCheckResult 不在最终结果中
                result.ir.clone(),
            );
        }

        result
    }

    /// 清空编译缓存
    pub fn clear_cache(&mut self) {
        self.compilation_cache.clear();
    }

    /// 重置流水线状态
    pub fn reset(&mut self) {
        self.state = PipelineState::Idle;
    }
}

/// 执行单个证明函数（RFC-027 Phase 2.5）
///
/// 完整的编译→执行管线：AST 查找 → IR 生成 → 字节码编译 → 解释器执行
pub(crate) fn execute_single_proof_fn(
    call: &typecheck::proof::verdict::ProofFunctionCall,
    ast: &super::core::parser::ast::Module,
    type_result: &typecheck::TypeCheckResult,
) -> Result<bool, String> {
    use crate::backends::common::value::from_const_value;
    use crate::backends::common::RuntimeValue;
    use crate::backends::interpreter::Interpreter;
    use crate::backends::Executor;
    use crate::frontend::core::parser::ast::StmtKind;
    use crate::middle;

    // 1. 在 AST 中查找函数定义
    let (params, body_stmts, type_ann) = ast
        .items
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StmtKind::Binding {
                name,
                type_name,
                type_annotation,
                params,
                body,
                ..
            } if *name == call.func_name && type_name.is_none() => {
                Some((params.clone(), body.clone(), type_annotation.clone()))
            }
            _ => None,
        })
        .ok_or_else(|| format!("证明函数 '{}' 未在 AST 中找到", call.func_name))?;

    // 2. IR 生成：单个函数 → FunctionIR
    let mut ir_gen = middle::core::ir_gen::AstToIrGenerator::new_with_type_result(type_result);
    let mut constants: Vec<middle::core::ir::ConstValue> = Vec::new();
    let func_ir = ir_gen
        .generate_function_ir(
            &call.func_name,
            type_ann.as_ref(),
            &params,
            &body_stmts,
            &mut constants,
            None,
        )
        .map_err(|e| format!("证明函数 '{}' IR 生成失败: {}", call.func_name, e))?;

    let func_ir = func_ir.ok_or_else(|| {
        format!(
            "证明函数 '{}' 是 native 函数，不能编译期执行",
            call.func_name
        )
    })?;

    // 3. 构造最小 ModuleIR（仅含一个函数）
    let module_ir = middle::ModuleIR {
        functions: vec![func_ir],
        ..Default::default()
    };

    // 4. 字节码编译
    let mut codegen = middle::passes::codegen::CodegenContext::new(module_ir);
    let bytecode_file = codegen
        .generate()
        .map_err(|e| format!("证明函数 '{}' 字节码编译失败: {}", call.func_name, e))?;

    let mut bytecode_module = crate::middle::core::bytecode::BytecodeModule::from(bytecode_file);
    // 证明函数没有 main 入口，不应自动执行 entry_point（会把参数为空的函数执行一遍）
    bytecode_module.entry_point = None;

    // 5. ConstValue → RuntimeValue
    let args: Vec<RuntimeValue> = call.args.iter().map(from_const_value).collect();

    // 6. 解释器执行
    let mut interpreter = Interpreter::new();
    // 先通过 execute_module 加载常量池和函数表
    interpreter
        .execute_module(&bytecode_module)
        .map_err(|e| format!("证明函数 '{}' 模块加载失败: {}", call.func_name, e))?;
    // 然后通过 call_function_by_id 执行具体函数（函数已在 execute_module 中注册）
    let func_id = bytecode_module
        .functions
        .iter()
        .position(|f| f.name == call.func_name)
        .ok_or_else(|| format!("证明函数 '{}' 在模块中未找到", call.func_name))?;
    let result = interpreter
        .call_function_by_id(
            crate::backends::common::value::FunctionId(func_id as u32),
            &args,
        )
        .map_err(|e| format!("证明函数 '{}' 执行失败: {}", call.func_name, e))?;

    // 7. 提取 bool
    match result {
        RuntimeValue::Bool(b) => Ok(b),
        other => Err(format!(
            "证明函数 '{}' 必须返回 Bool，实际返回: {:?}",
            call.func_name, other
        )),
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
    warnings: Vec<String>,
}

#[allow(dead_code)]
impl TypecheckResult {
    fn success(
        type_result: typecheck::TypeCheckResult,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            type_result,
            errors: Vec::new(),
            warnings,
        }
    }

    fn failed(errors: Vec<Diagnostic>) -> Self {
        Self {
            type_result: typecheck::TypeCheckResult::default(),
            errors,
            warnings: Vec::new(),
        }
    }

    #[allow(dead_code)]
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

/// 证明函数执行结果
struct ProofExecResult {
    /// 执行失败的证明函数名
    #[allow(dead_code)] // Phase 2.5: 将用于更详细的错误报告
    failed_proofs: Vec<String>,
    errors: Vec<String>,
}

impl ProofExecResult {
    fn success() -> Self {
        Self {
            failed_proofs: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn failed(
        failed_proofs: Vec<String>,
        errors: Vec<String>,
    ) -> Self {
        Self {
            failed_proofs,
            errors,
        }
    }

    fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}
