//! 编译流水线
//!
//! 管理编译状态机、执行编译流程、处理错误恢复。

use crate::middle;
use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::ErrorCodeDefinition;
use super::{config::CompileConfig, core::typecheck};

/// 管道错误类型
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// 词法/解析错误（携带原始诊断，保留错误码与 span）
    LexParse(Diagnostic),
    /// 类型检查错误
    TypeCheck(Diagnostic),
    /// IR 生成错误
    IRGeneration(String),
    ProofExecution(Diagnostic),
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
            PipelineError::LexParse(err) => Some(err.clone()),
            PipelineError::TypeCheck(err) => Some(err.clone()),
            PipelineError::ProofExecution(err) => Some(err.clone()),
            _ => None,
        }
    }
}

/// 编译结果
#[derive(Debug, Clone, Default)]
pub struct CompilationResult {
    /// 生成的 IR
    pub ir: Option<middle::ModuleIR>,
    /// 错误数量
    pub error_count: usize,
    /// 警告数量
    pub warning_count: usize,
    /// 总耗时（毫秒）
    pub total_duration_ms: u64,
    /// 错误
    pub errors: Vec<PipelineError>,
    /// 警告诊断（#321 M2 结构化，含 severity/code/span）
    pub warnings: Vec<Diagnostic>,
}

impl CompilationResult {
    /// 创建成功结果
    pub fn success(
        ir: middle::ModuleIR,
        total_ms: u64,
        warnings: Vec<Diagnostic>,
    ) -> Self {
        Self {
            ir: Some(ir),
            error_count: 0,
            warning_count: warnings.len(),
            total_duration_ms: total_ms,
            errors: Vec::new(),
            warnings,
        }
    }

    /// 创建失败结果
    pub fn failed(
        errors: Vec<PipelineError>,
        total_ms: u64,
    ) -> Self {
        Self {
            ir: None,
            error_count: errors.len(),
            warning_count: 0,
            total_duration_ms: total_ms,
            errors,
            warnings: Vec::new(),
        }
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.error_count == 0 && self.ir.is_some()
    }
}

use std::fmt;

/// 编译流水线
pub struct Pipeline {
    /// 配置
    config: CompileConfig,
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
            .field("config", &self.config)
            .finish()
    }
}

impl Pipeline {
    /// 创建新流水线
    pub fn new(config: CompileConfig) -> Self {
        Self { config }
    }

    /// 获取配置
    #[inline]
    pub fn config(&self) -> &CompileConfig {
        &self.config
    }

    /// 运行完整编译流程
    pub fn run(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> CompilationResult {
        let start_time = crate::util::time_compat::Instant::now();

        // 执行各阶段
        let lex_result = self.run_lexing(source_name, source);
        if !lex_result.is_success() {
            return CompilationResult::failed(
                lex_result
                    .errors
                    .into_iter()
                    .map(PipelineError::LexParse)
                    .collect(),
                start_time.elapsed().as_millis() as u64,
            );
        }

        let parse_result = self.run_parsing(source_name, &lex_result.tokens);
        if !parse_result.is_success() {
            return CompilationResult::failed(
                parse_result
                    .errors
                    .into_iter()
                    .map(PipelineError::LexParse)
                    .collect(),
                start_time.elapsed().as_millis() as u64,
            );
        }

        let typecheck_result = self.run_typecheck(source_name, source, &parse_result.ast);
        if !typecheck_result.is_success() {
            return CompilationResult::failed(
                typecheck_result
                    .errors
                    .into_iter()
                    .map(PipelineError::TypeCheck)
                    .collect(),
                start_time.elapsed().as_millis() as u64,
            );
        }

        // RFC-027 Phase 2.5: 证明函数执行循环
        // 在类型检查通过后、IR 生成前，执行编译期证明函数
        //（共享实现：orchestrator 各入口同调，02-stage-contract §漏洞修复）
        if !typecheck_result.type_result.proof_calls().is_empty() {
            let errors = super::proof_execution::execute_proof_calls(
                typecheck_result.type_result.proof_calls(),
                &parse_result.ast,
                &typecheck_result.type_result,
            );
            if !errors.is_empty() {
                return CompilationResult::failed(
                    errors
                        .into_iter()
                        .map(PipelineError::ProofExecution)
                        .collect(),
                    start_time.elapsed().as_millis() as u64,
                );
            }
        }

        let ir_result = self.run_ir_generation(
            source_name,
            source,
            &parse_result.ast,
            &typecheck_result.type_result,
        );

        let total_ms = start_time.elapsed().as_millis() as u64;

        if ir_result.is_success() {
            // 收集所有警告（来自 typecheck 阶段）
            let warnings = typecheck_result.warnings;
            CompilationResult::success(ir_result.ir.unwrap(), total_ms, warnings)
        } else {
            // IR 生成错误被归类为类型检查错误
            let pipeline_errors: Vec<PipelineError> = ir_result
                .errors
                .into_iter()
                .map(PipelineError::TypeCheck)
                .collect();
            CompilationResult::failed(pipeline_errors, total_ms)
        }
    }

    /// 词法分析阶段
    fn run_lexing(
        &mut self,
        _source_name: &str,
        source: &str,
    ) -> LexResult {
        match super::core::lexer::tokenize(source) {
            Ok(tokens) => LexResult::success(tokens),
            Err(e) => LexResult::failed(vec![e.to_diagnostic()]),
        }
    }

    /// 语法分析阶段
    fn run_parsing(
        &mut self,
        _source_name: &str,
        tokens: &[super::core::lexer::Token],
    ) -> ParseResult {
        match super::core::parser::parse(tokens) {
            result if result.has_errors => {
                let error_msg = result.errors.into_iter().next().unwrap_or_else(|| {
                    crate::util::diagnostic::ErrorCodeDefinition::unexpected_token("unknown")
                        .at(crate::util::span::Span::dummy())
                        .build()
                });

                ParseResult::failed(vec![error_msg])
            }
            result => ParseResult::success(result.module),
        }
    }

    /// 类型检查阶段
    fn run_typecheck(
        &mut self,
        _source_name: &str,
        _source: &str,
        ast: &super::core::parser::Module,
    ) -> TypecheckResult {
        let mut type_result = typecheck::check_module(ast, &mut None);
        let has_errors = !type_result.diagnostics.is_empty();
        let errors = std::mem::take(&mut type_result.diagnostics);

        // 死代码族警告（#321）：私有死代码（分析器）+ 未使用导入（typecheck 检出）。
        // 与错误分离——混入 errors 会被按错误计数，破坏非阻断契约。
        let mut warnings = Vec::new();
        if self.config.dead_code.enabled && !has_errors {
            warnings.extend(self.run_dead_code_analysis(ast));
            warnings.extend(std::mem::take(&mut type_result.warnings));
        }

        TypecheckResult {
            type_result,
            errors,
            warnings,
        }
    }

    /// 死代码分析阶段
    fn run_dead_code_analysis(
        &mut self,
        ast: &super::core::parser::Module,
    ) -> Vec<Diagnostic> {
        use crate::frontend::core::typecheck::passes::dead_code::DeadCodeAnalyzer;

        let mut analyzer = DeadCodeAnalyzer::new();
        let warnings = analyzer.analyze(ast);

        // 结构化警告诊断（severity 由 builder 按 W 前缀推导，#321 M2）
        analyzer.to_diagnostics(&warnings)
    }

    /// IR 生成阶段
    fn run_ir_generation(
        &mut self,
        _source_name: &str,
        _source: &str,
        ast: &super::core::parser::Module,
        type_result: &typecheck::TypeCheckResult,
    ) -> IRResult {
        // 单文件模式：入口与嵌入 std 模块（std.test 等，RFC-036 §4）共用同一 registry
        // （共享 SymbolTable），使嵌入函数的 DefId 与入口调用点解析到的一致，避免跨表
        // DefId 撞车错分发（#94）。直接构造 generator 而非 middle::generate_ir 以复用 registry。
        let registry = crate::frontend::module::registry::ModuleRegistry::with_std();
        let mut ir = {
            let mut generator = middle::core::ir_gen::AstToIrGenerator::new_with_type_result(
                type_result,
                registry.clone(),
                None,
            );
            match generator.generate_module_ir(ast) {
                Ok(ir) => ir,
                Err(errors) => return IRResult::failed(errors),
            }
        };
        // 嵌入模块编译为独立 ModuleIR 并合并——入口调用按 `short_to_qualified_map`
        // 命名成 `std.test.assert_eq`，合并后函数体可解析。
        if let Err(e) = merge_embedded_std_ir(_source, &mut ir, &registry) {
            return IRResult::failed(vec![e]);
        }
        // 单态化（根据配置决定是否启用）
        if self.config.mono.enabled && !type_result.instantiation_requests.is_empty() {
            let mut mono =
                middle::passes::mono::Monomorphizer::with_max_depth(self.config.mono.max_depth)
                    .with_max_instantiations(self.config.mono.max_instantiations);
            match mono.monomorphize(&ir, &type_result.instantiation_requests) {
                Ok(mono_ir) => ir = mono_ir,
                Err(diag) => return IRResult::failed(vec![diag]),
            }
        }

        IRResult::success(ir)
    }
}

/// 单文件模式：扫描入口源码中的 `use std.X`，把命中的嵌入 std 模块（std.test）
/// 编译为独立 ModuleIR 并合并进入口 IR。
///
/// 词法扫描失败返回 Ok（真正的错误由 parse 阶段报告）。`registry` 与入口 IR 生成
/// 共用（共享 SymbolTable），保证 DefId 一致（#94）。
fn merge_embedded_std_ir(
    source: &str,
    ir: &mut crate::middle::ModuleIR,
    registry: &crate::frontend::module::registry::ModuleRegistry,
) -> Result<(), Diagnostic> {
    let mut use_paths = crate::frontend::module::orchestrator::scan_use_paths(source);
    // `for x in xs` 的脱糖会调用迭代协议 `std.list.iter/has_next/next`，
    // 该协议现由纯 yx 模块提供（`src/std/list.yx`，D5 硬切换）。
    // 源码未写 `use std.list` 时也必须纳入编译单元，否则 for 循环运行时
    // 报「Native function not found: std.list.iter」。
    if !use_paths.iter().any(|p| p == "std.list") {
        use_paths.push("std.list".to_string());
    }
    for use_path in use_paths {
        if use_path == "std"
            || !use_path.starts_with("std.")
            || crate::std::yx_sources::embedded_std_source(&use_path).is_none()
        {
            continue;
        }
        // 嵌入 std 模块：编译独立 IR 并合并（去重——入口可能多处 use 同一模块）
        let embedded = match crate::frontend::module::orchestrator::compile_embedded_module(
            &use_path, registry,
        ) {
            Ok(m) => m,
            Err(e) => {
                // #322 M3：E_INTERNAL 伪码收敛为注册码 E8001
                return Err(ErrorCodeDefinition::internal_error(&format!(
                    "嵌入 std 模块 {use_path} 编译失败: {e}"
                ))
                .build());
            }
        };
        for func in embedded.functions {
            if !ir.functions.iter().any(|f| f.name == func.name) {
                ir.functions.push(func);
            }
        }
    }
    Ok(())
}

/// 词法分析结果
struct LexResult {
    tokens: Vec<super::core::lexer::Token>,
    errors: Vec<Diagnostic>,
}

impl LexResult {
    fn success(tokens: Vec<super::core::lexer::Token>) -> Self {
        Self {
            tokens,
            errors: Vec::new(),
        }
    }

    fn failed(errors: Vec<Diagnostic>) -> Self {
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
    errors: Vec<Diagnostic>,
}

impl ParseResult {
    fn success(ast: super::core::parser::Module) -> Self {
        Self {
            ast,
            errors: Vec::new(),
        }
    }

    fn failed(errors: Vec<Diagnostic>) -> Self {
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
    warnings: Vec<Diagnostic>,
}
impl TypecheckResult {
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
