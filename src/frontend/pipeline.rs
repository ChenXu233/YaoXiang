//! 编译流水线
//!
//! 管理编译状态机、执行编译流程、处理错误恢复。

use crate::middle;
use crate::util::diagnostic::Diagnostic;
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
    /// 警告消息
    pub warnings: Vec<String>,
}

impl CompilationResult {
    /// 创建成功结果
    pub fn success(
        ir: middle::ModuleIR,
        total_ms: u64,
        warnings: Vec<String>,
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
        if !typecheck_result.type_result.proof_calls.is_empty() {
            let proof_result = self.run_proof_execution(
                &typecheck_result.type_result.proof_calls,
                &parse_result.ast,
                &typecheck_result.type_result,
            );
            if !proof_result.is_success() {
                return CompilationResult::failed(
                    proof_result
                        .errors
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
        source_name: &str,
        _source: &str,
        ast: &super::core::parser::Module,
    ) -> TypecheckResult {
        let mut type_result = typecheck::check_module(ast, &mut None);
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
    ) -> ProofExecResult {
        let mut failed_proofs = Vec::new();
        let mut errors = Vec::new();

        for call in proof_calls {
            match execute_single_proof_fn(call, ast, type_result) {
                Ok(true) => {
                    // 证明通过，继续
                }
                Ok(false) => {
                    failed_proofs.push(call.func_name.clone());
                    let msg = format!(
                        "证明函数 '{}' 返回 false，约束不满足（参数: {:?}）",
                        call.func_name, call.args,
                    );
                    let diag = Diagnostic::error(
                        "E4018".to_string(),
                        msg,
                        "检查约束条件或修改传入值".to_string(),
                        None,
                    );
                    errors.push(diag);
                }
                Err(e) => {
                    failed_proofs.push(call.func_name.clone());
                    let diag = Diagnostic::error(
                        "E4018".to_string(),
                        format!("证明函数 '{}' 执行失败: {}", call.func_name, e),
                        String::new(),
                        None,
                    );
                    errors.push(diag);
                }
            }
        }

        if failed_proofs.is_empty() {
            ProofExecResult::success()
        } else {
            ProofExecResult::failed(errors)
        }
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
                middle::passes::mono::Monomorphizer::with_max_depth(self.config.mono.max_depth);
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
    use crate::frontend::core::lexer::TokenKind;
    let Ok(tokens) = crate::frontend::core::tokenize(source) else {
        return Ok(());
    };
    let mut i = 0;
    while i < tokens.len() {
        if !matches!(tokens[i].kind, TokenKind::KwUse) {
            i += 1;
            continue;
        }
        i += 1;
        // 收集 `use std.a.b` 的模块路径（到 { 或非标识符为止）
        let mut segments: Vec<String> = Vec::new();
        while let Some(TokenKind::Identifier(name)) = tokens.get(i).map(|t| &t.kind) {
            segments.push(name.clone());
            i += 1;
            match tokens.get(i).map(|t| &t.kind) {
                Some(TokenKind::Dot)
                    if matches!(tokens.get(i + 1).map(|t| &t.kind), Some(TokenKind::LBrace)) =>
                {
                    break;
                }
                Some(TokenKind::Dot) => {
                    i += 1;
                }
                _ => break,
            }
        }
        let use_path = segments.join(".");
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
                return Err(Diagnostic::error(
                    "E_INTERNAL".to_string(),
                    format!("嵌入 std 模块 {use_path} 编译失败: {e}"),
                    "This is a compiler bug".to_string(),
                    None,
                ));
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

/// 执行单个证明函数（RFC-027 Phase 2.5）
/// RFC-027 Phase 2.5: 执行单个证明函数
///
/// 优先使用 const 求值（约束表达式），回退到 IR/字节码管线（return 形式）
pub(crate) fn execute_single_proof_fn(
    call: &typecheck::proof::verdict::ProofFunctionCall,
    ast: &super::core::parser::ast::Module,
    type_result: &typecheck::TypeCheckResult,
) -> Result<bool, String> {
    use crate::frontend::core::parser::ast::StmtKind;

    // 1. 在 AST 中查找函数定义
    let (params, body_stmts, type_ann) = ast
        .items
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation,
                value,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    _ => return None,
                };
                if name != call.func_name {
                    return None;
                }
                if let Some(v) = value {
                    if let Expr::Lambda { params, body, .. } = v.as_ref() {
                        Some((params.clone(), body.stmts.clone(), type_annotation.clone()))
                    } else if let Expr::Block(block) = v.as_ref() {
                        Some((Vec::new(), block.stmts.clone(), type_annotation.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
        .or_else(|| {
            // 同时搜索 TypeDefinition 项（类型级证明函数语法）
            ast.items.iter().find_map(|stmt| match &stmt.kind {
                StmtKind::TypeDefinition {
                    name,
                    signature_params,
                    definition,
                    ..
                } => {
                    if *name != call.func_name {
                        return None;
                    }
                    use crate::frontend::core::parser::ast::{Type, TypeBodyItem};
                    if let Type::Struct { body } = definition {
                        for item in body {
                            if let TypeBodyItem::Expr(Type::ConstExpr(expr)) = item {
                                let constraint_stmt = crate::frontend::core::parser::ast::Stmt {
                                    kind: StmtKind::Expr(expr.clone()),
                                    span: crate::util::span::Span::dummy(),
                                };
                                return Some((
                                    signature_params.clone(),
                                    vec![constraint_stmt],
                                    None,
                                ));
                            }
                        }
                    }
                    None
                }
                _ => None,
            })
        })
        .ok_or_else(|| format!("证明函数 '{}' 未在 AST 中找到", call.func_name))?;

    // 2. 提取约束表达式（Expr 或 Return 中的表达式）
    let constraint_expr = body_stmts.iter().find_map(|s| match &s.kind {
        StmtKind::Expr(e) => Some(e.as_ref().clone()),
        StmtKind::Return(Some(e)) => Some(e.as_ref().clone()),
        _ => None,
    });

    // 3. 优先 const 求值（精化约束语义：表达式是约束，不是返回值）
    if let Some(ref expr) = constraint_expr {
        if let Some(const_expr) =
            crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(expr)
        {
            let mut evaluator =
                crate::frontend::core::types::eval::const_eval::ConstGenericEval::new();
            // 绑定参数：param name → proof call arg value
            for (i, param) in params.iter().enumerate() {
                if let Some(arg) = call.args.get(i) {
                    evaluator.bind_var(param.name.clone(), arg.clone());
                }
            }
            if let Ok(result) = evaluator.eval(&const_expr) {
                match result {
                    crate::frontend::core::types::ConstValue::Bool(b) => return Ok(b),
                    other => {
                        return Err(format!(
                            "证明函数 '{}' 约束求值结果不是 Bool: {:?}",
                            call.func_name, other
                        ))
                    }
                }
            }
        }
    }

    // 4. 回退：IR 生成 → 字节码 → 解释器（支持 return 形式的复杂证明函数）
    use crate::backends::common::value::from_const_value;
    use crate::backends::common::RuntimeValue;
    use crate::backends::interpreter::Interpreter;
    use crate::backends::Executor;
    use crate::middle;

    let mut ir_gen = middle::core::ir_gen::AstToIrGenerator::new_with_type_result(
        type_result,
        crate::frontend::module::registry::ModuleRegistry::with_std(),
        None,
    );
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

    let module_ir = middle::ModuleIR {
        functions: vec![func_ir],
        ..Default::default()
    };

    let mut codegen = middle::passes::codegen::CodegenContext::new(module_ir);
    let bytecode_file = codegen
        .generate()
        .map_err(|e| format!("证明函数 '{}' 字节码编译失败: {}", call.func_name, e))?;

    let mut bytecode_module = crate::middle::core::bytecode::BytecodeModule::from(bytecode_file);
    bytecode_module.entry_point = None;

    let args: Vec<RuntimeValue> = call.args.iter().map(from_const_value).collect();

    let mut interpreter = Interpreter::new();
    interpreter
        .execute_module(&bytecode_module)
        .map_err(|e| format!("证明函数 '{}' 模块加载失败: {}", call.func_name, e))?;
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
    warnings: Vec<String>,
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

/// 证明函数执行结果
struct ProofExecResult {
    errors: Vec<Diagnostic>,
}

impl ProofExecResult {
    fn success() -> Self {
        Self { errors: Vec::new() }
    }

    fn failed(errors: Vec<Diagnostic>) -> Self {
        Self { errors }
    }

    fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}
