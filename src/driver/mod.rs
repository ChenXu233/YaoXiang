//! L1 编排层：统一 Driver 与阶段契约 —— 02-stage-contract / RFC-039。
//!
//! 依赖方向红线（02 §详细设计）：`driver`（L1）依赖 `frontend`（L2）/
//! `middle`（L3）/ `backends`（L4）的接口；**L2/L3/L4 不得反向
//! `use crate::driver`**，由 check-module-boundary 门禁拦截。
//!
//! # Driver（02 §目标设计 3/4）
//!
//! `Driver` 是 `Program` 的唯一解释器：按 `Program::stages()` 的拓扑序
//! 逐阶段 `dispatch`（穷尽 `match`——新增 `Stage` 变体不处理即编译失败，
//! 与 opcode 表同构的强制机制）。跳过由**拓扑**判定而非阶段返回值
//! （02 §4）：三类原因（上游失败 / 无义务 / 配置未启用）记入
//! `DriverOutcome.skipped`。
//!
//! **4.1.3 边界**：只有 SingleFile / WasmPlayground 形态用到的六臂已接线
//! （Parsing / Typecheck / DeadCodeAnalysis / ProofExecution /
//! IrGeneration / Monomorphization）；其余六臂返回
//! `DriverError::StageNotWired`，随 4.2 入口合并逐臂落地。Skipped 在本
//! 阶段只进内部记录、**不外发诊断**（外发随 4.3 义务账本，02 §4 文案
//! 表）——4.1.3 的验收是单文件诊断集与退出码逐字节相同（C2），任何
//! 新增对外诊断都会打破它。
//!
//! 与 02 草图的两处偏差（语义等价，登记备查）：
//! - 02 §4 的 `StageOutcome` 三态不由阶段臂返回，而由 `State` 的失败
//!   标记 + `Aggregation` 门控表达（Continue = 无错误无警告 /
//!   Warn = 产警告 / Abort = 产错误并触发 `should_abort`），省去每臂
//!   的样板返回；
//! - `Driver` 无 `config` 字段——配置唯一来源是 `Program.config`
//!   （02 §5：避免 Driver 持可变全局状态），一次 run 的可变状态全部
//!   在 `State` 内。

pub mod program;
pub mod stage;
pub mod unit;

pub use program::{Aggregation, Program, ProgramKind};
pub use stage::{Stage, StageScope};
pub use unit::Unit;

use std::fmt;

use crate::frontend::core::lexer;
use crate::frontend::core::parser::{self, Module};
use crate::frontend::core::typecheck::{self, TypeCheckResult};
use crate::frontend::core::typecheck::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::pipeline::{CompilationResult, PipelineError};
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::middle::passes::mono::Monomorphizer;
use crate::middle::ModuleIR;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

/// 统一 Driver（02 §3）。
///
/// 无状态：配置在 `Program` 上，一次编译的可变状态在 `State` 内。
#[derive(Debug, Default)]
pub struct Driver;

impl Driver {
    /// 创建 Driver。
    pub fn new() -> Self {
        Self
    }

    /// 解释执行一份 `Program`：按阶段表拓扑序 dispatch，直到表尾或
    /// `Aggregation::FailFast` 下首个失败阶段（02 §3 草图的 break）。
    pub fn run(
        &mut self,
        program: Program,
    ) -> Result<DriverOutcome, DriverError> {
        if program.units.is_empty() {
            // 无编译单元的程序是调用方 bug——显式错误，不许 vacuous success
            return Err(DriverError::EmptyUnits);
        }
        let mut state = State::new(program);
        // stages() 返回 &'static [Stage]（来自 ProgramKind 的预定义表），
        // 不持有对 state 的借用。
        let stages = state.program.stages();
        for &stage in stages {
            // 跳过由拓扑决定，不由阶段返回值决定（02 §4）
            if let Some(upstream) = state.first_failed_upstream(stage) {
                state.record_skipped(stage, SkipReason::UpstreamFailed(upstream));
                continue;
            }
            self.dispatch(stage, &mut state)?;
            if state.should_abort() {
                break;
            }
        }
        Ok(state.into_outcome())
    }

    /// 穷尽 dispatch——新增 `Stage` 变体时本 match 编译失败（02 §1 的
    /// 强制机制）。六个未接线臂随 4.2 入口合并落地；以显式错误而非
    /// `todo!()` 占位（红线）。
    fn dispatch(
        &self,
        stage: Stage,
        state: &mut State,
    ) -> Result<(), DriverError> {
        match stage {
            Stage::VendorConsistency
            | Stage::Discovery
            | Stage::Registry
            | Stage::RoleClassification
            | Stage::GlobalSlotAlloc
            | Stage::Linking => Err(DriverError::StageNotWired(stage)),
            Stage::Parsing => {
                self.parsing(state);
                Ok(())
            }
            Stage::Typecheck => {
                self.typecheck(state);
                Ok(())
            }
            Stage::DeadCodeAnalysis => {
                self.dead_code_analysis(state);
                Ok(())
            }
            Stage::ProofExecution => {
                self.proof_execution(state);
                Ok(())
            }
            Stage::IrGeneration => self.ir_generation(state),
            Stage::Monomorphization => self.monomorphization(state),
        }
    }

    /// `Stage::Parsing` 臂（PerModule）：lex → parse。
    ///
    /// 移植自 pipeline.rs:149-171 / 229-258——parse 失败只取**首个**错误
    /// 对外（逐字节判据的一部分）。
    fn parsing(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            units,
            errors,
            failed_stages,
            ..
        } = state;
        for (i, us) in units.iter_mut().enumerate() {
            let source = &program.units[i].source;
            let tokens = match lexer::tokenize(source) {
                Ok(tokens) => tokens,
                Err(e) => {
                    errors.push(PipelineError::LexParse(e.to_diagnostic()));
                    failed_stages.push(Stage::Parsing);
                    if matches!(program.aggregation, Aggregation::FailFast) {
                        return;
                    }
                    continue;
                }
            };
            let result = parser::parse(&tokens);
            if result.has_errors {
                let first = result.errors.into_iter().next().unwrap_or_else(|| {
                    ErrorCodeDefinition::unexpected_token("unknown")
                        .at(Span::dummy())
                        .build()
                });
                errors.push(PipelineError::LexParse(first));
                failed_stages.push(Stage::Parsing);
                if matches!(program.aggregation, Aggregation::FailFast) {
                    return;
                }
                continue;
            }
            us.ast = Some(result.module);
        }
    }

    /// `Stage::Typecheck` 臂（PerModule）：`check_module`。
    ///
    /// 移植自 pipeline.rs:173-183 / 262-285。警告**暂存**而非立即外发——
    /// 是否外发由 DeadCodeAnalysis 臂的门控决定（忠实复刻 pipeline 的
    /// 耦合：`dead_code.enabled == false` 时 typecheck 警告一并丢弃）。
    fn typecheck(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            units,
            errors,
            failed_stages,
            ..
        } = state;
        for us in units.iter_mut() {
            let Some(ast) = &us.ast else {
                continue; // CollectAll 下 parse 失败的单元（4.1.3 不可达）
            };
            let mut type_result = typecheck::check_module(ast, &mut None);
            let diagnostics = std::mem::take(&mut type_result.diagnostics);
            if !diagnostics.is_empty() {
                us.typecheck_failed = true;
                errors.extend(diagnostics.into_iter().map(PipelineError::TypeCheck));
                failed_stages.push(Stage::Typecheck);
                if matches!(program.aggregation, Aggregation::FailFast) {
                    return;
                }
                continue;
            }
            us.typecheck_warnings = std::mem::take(&mut type_result.warnings);
            us.type_result = Some(type_result);
        }
    }

    /// `Stage::DeadCodeAnalysis` 臂（Project 作用域，臂内逐单元）。
    ///
    /// 移植自 pipeline.rs:275-278 / 288-299。门控
    /// `dead_code.enabled && !has_errors` 原样保留；开关关闭时连暂存的
    /// typecheck 警告一并丢弃（现状耦合），并记录 ConfigDisabled 跳过。
    /// 警告顺序 = 死代码警告在前、typecheck 警告在后（与 pipeline 一致）。
    fn dead_code_analysis(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            units,
            warnings,
            skipped,
            ..
        } = state;
        if !program.config.dead_code.enabled {
            for us in units.iter_mut() {
                us.typecheck_warnings.clear();
            }
            skipped.push(SkippedStage {
                stage: Stage::DeadCodeAnalysis,
                reason: SkipReason::ConfigDisabled,
            });
            return;
        }
        for us in units.iter_mut() {
            let Some(ast) = &us.ast else {
                continue;
            };
            if us.typecheck_failed {
                continue;
            }
            let mut analyzer = DeadCodeAnalyzer::new();
            let found = analyzer.analyze(ast);
            // 结构化警告诊断（severity 由 builder 按 W 前缀推导，#321 M2）
            warnings.extend(analyzer.to_diagnostics(&found));
            warnings.extend(std::mem::take(&mut us.typecheck_warnings));
        }
    }

    /// `Stage::ProofExecution` 臂（PerModule）：RFC-027 Phase 2.5 编译期
    /// 证明函数执行。
    ///
    /// 移植自 pipeline.rs:185-203。义务为空 = 正常跳过（02 §4：Info 级，
    /// 不计警告数——4.1.3 仅内部记录）。实现调
    /// `frontend::proof_execution::execute_proof_calls`（编排层共享原语；
    /// orchestrator 四入口的同函数调用随 4.2 一并迁入本臂）。
    fn proof_execution(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            units,
            errors,
            skipped,
            failed_stages,
            ..
        } = state;
        let mut any_obligation = false;
        for us in units.iter_mut() {
            let (Some(ast), Some(type_result)) = (&us.ast, &us.type_result) else {
                continue;
            };
            if us.typecheck_failed || type_result.proof_calls().is_empty() {
                continue;
            }
            any_obligation = true;
            let stage_errors = crate::frontend::proof_execution::execute_proof_calls(
                type_result.proof_calls(),
                ast,
                type_result,
            );
            if !stage_errors.is_empty() {
                errors.extend(stage_errors.into_iter().map(PipelineError::ProofExecution));
                failed_stages.push(Stage::ProofExecution);
                if matches!(program.aggregation, Aggregation::FailFast) {
                    return;
                }
            }
        }
        if !any_obligation {
            skipped.push(SkippedStage {
                stage: Stage::ProofExecution,
                reason: SkipReason::NoObligations,
            });
        }
    }

    /// `Stage::IrGeneration` 臂（PerModule）：AST → ModuleIR + 嵌入 std
    /// 合并（#94 共享 registry）。
    ///
    /// 移植自 pipeline.rs:205-227 / 302-341 + `merge_embedded_std_ir`。
    /// IR 错误归入 `PipelineError::TypeCheck`（pipeline 现状映射）。
    /// 本臂是单文件形态（嵌入 std 合并逐单元做）；多文件的共享 registry /
    /// 槽位分配形态随 4.2 / 3.4.8 落地，多单元显式报错而非静默错跑。
    fn ir_generation(
        &self,
        state: &mut State,
    ) -> Result<(), DriverError> {
        if state.units.len() != 1 {
            return Err(DriverError::MultiUnitNotWired(Stage::IrGeneration));
        }
        let State {
            program,
            units,
            errors,
            failed_stages,
            ..
        } = state;
        let us = &mut units[0];
        let (Some(ast), Some(type_result)) = (&us.ast, &us.type_result) else {
            return Ok(()); // 上游已失败（CollectAll；4.1.3 不可达）
        };
        // 单文件模式：入口与嵌入 std 模块（std.test 等，RFC-036 §4）共用同一
        // registry（共享 SymbolTable），使嵌入函数的 DefId 与入口调用点解析到
        // 的一致，避免跨表 DefId 撞车错分发（#94）。
        let registry = ModuleRegistry::with_std();
        let mut ir = {
            let mut generator =
                AstToIrGenerator::new_with_type_result(type_result, registry.clone(), None);
            match generator.generate_module_ir(ast) {
                Ok(ir) => ir,
                Err(errs) => {
                    errors.extend(errs.into_iter().map(PipelineError::TypeCheck));
                    failed_stages.push(Stage::IrGeneration);
                    return Ok(());
                }
            }
        };
        if let Err(e) = merge_embedded_std_ir(&program.units[0].source, &mut ir, &registry) {
            errors.push(PipelineError::TypeCheck(e));
            failed_stages.push(Stage::IrGeneration);
            return Ok(());
        }
        us.ir = Some(ir);
        Ok(())
    }

    /// `Stage::Monomorphization` 臂（Project 作用域）：消费 IR 产物与
    /// 聚合的实例化请求。
    ///
    /// 移植自 pipeline.rs:329-338。两道门控原样保留：配置关闭 →
    /// ConfigDisabled；请求为空 → NoObligations。错误归入 TypeCheck
    /// （pipeline 现状：mono 失败经 IRResult 映射为类型检查错误）。
    fn monomorphization(
        &self,
        state: &mut State,
    ) -> Result<(), DriverError> {
        let (max_depth, max_instantiations) = (
            state.program.config.mono.max_depth,
            state.program.config.mono.max_instantiations,
        );
        if !state.program.config.mono.enabled {
            state.record_skipped(Stage::Monomorphization, SkipReason::ConfigDisabled);
            return Ok(());
        }
        if state.units.len() != 1 {
            return Err(DriverError::MultiUnitNotWired(Stage::Monomorphization));
        }
        let State {
            units,
            errors,
            skipped,
            failed_stages,
            ..
        } = state;
        let us = &mut units[0];
        let (Some(type_result), Some(ir)) = (&us.type_result, &mut us.ir) else {
            return Ok(()); // 上游已失败（CollectAll；4.1.3 不可达）
        };
        if type_result.instantiation_requests.is_empty() {
            skipped.push(SkippedStage {
                stage: Stage::Monomorphization,
                reason: SkipReason::NoObligations,
            });
            return Ok(());
        }
        let mut mono =
            Monomorphizer::with_max_depth(max_depth).with_max_instantiations(max_instantiations);
        match mono.monomorphize(ir, &type_result.instantiation_requests) {
            Ok(mono_ir) => *ir = mono_ir,
            Err(diag) => {
                errors.push(PipelineError::TypeCheck(diag));
                failed_stages.push(Stage::Monomorphization);
            }
        }
        Ok(())
    }
}

/// 阶段跳过原因（02 §4 文案表的三类）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkipReason {
    /// 上游阶段失败（Abort 沿数据依赖传播）
    UpstreamFailed(Stage),
    /// 前置义务为空（正常跳过；02 §4：Info 级、不计入警告数）
    NoObligations,
    /// 配置未启用（如 `config.mono.enabled == false`）
    ConfigDisabled,
}

/// 一条跳过记录：阶段 + 原因。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SkippedStage {
    /// 被跳过的阶段
    pub stage: Stage,
    /// 跳过原因
    pub reason: SkipReason,
}

/// Driver 一次运行的产出。
#[derive(Debug)]
pub struct DriverOutcome {
    /// 编译结果（诊断集与 pipeline 时代逐字节相同，C2 判据）
    pub result: CompilationResult,
    /// 被跳过的阶段及原因——4.1.3 仅内部记录；对外 Skipped 诊断
    /// 随 4.3 义务账本接入（02 §4）
    pub skipped: Vec<SkippedStage>,
}

/// Driver 的结构化错误。
///
/// 4.1.3 下从 `Pipeline::run` 不可达（SingleFile 阶段表全接线、恒为
/// 1 单元），但 `Driver` 是 pub 接口：显式错误优于 panic / `todo!()`。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DriverError {
    /// 阶段臂未接线（4.2 入口合并逐臂落地）
    StageNotWired(Stage),
    /// 程序没有任何编译单元
    EmptyUnits,
    /// 该臂只支持单单元程序（多文件 IR / 单态化随 4.2 / 3.4.8 落地）
    MultiUnitNotWired(Stage),
}

impl fmt::Display for DriverError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            DriverError::StageNotWired(stage) => write!(
                f,
                "stage {} ({stage:?}) is not wired into the Driver yet (lands with WBS 4.2)",
                stage.slug()
            ),
            DriverError::EmptyUnits => write!(f, "program has no compilation units"),
            DriverError::MultiUnitNotWired(stage) => write!(
                f,
                "stage {} ({stage:?}) supports only single-unit programs until WBS 4.2/3.4.8",
                stage.slug()
            ),
        }
    }
}

impl std::error::Error for DriverError {}

/// 一次编译的可变状态（`Driver::run` 的执行上下文）。
struct State {
    /// 正在解释的程序
    program: Program,
    /// 逐编译单元的阶段产物（与 `program.units` 等长、按下标对应）
    units: Vec<UnitState>,
    /// 已产出的错误诊断（FailFast 下恰为首个失败阶段的那批）
    errors: Vec<PipelineError>,
    /// 已产出的警告诊断
    warnings: Vec<Diagnostic>,
    /// 跳过记录（内部审计；4.3 前不外发）
    skipped: Vec<SkippedStage>,
    /// 已失败的阶段（拓扑跳过的判定依据）
    failed_stages: Vec<Stage>,
}

/// 逐编译单元的阶段产物（PerModule 阶段的执行粒度）。
#[derive(Default)]
struct UnitState {
    /// Parsing 产物
    ast: Option<Module>,
    /// Typecheck 产物
    type_result: Option<TypeCheckResult>,
    /// typecheck 警告暂存——外发与否由 DeadCodeAnalysis 臂的门控决定
    /// （pipeline 现状耦合，见该臂注释）
    typecheck_warnings: Vec<Diagnostic>,
    /// 本单元 typecheck 是否产出错误（死代码门控的 `!has_errors` 半边）
    typecheck_failed: bool,
    /// IrGeneration（+ Monomorphization 改写）产物
    ir: Option<ModuleIR>,
}

impl State {
    fn new(program: Program) -> Self {
        let units = (0..program.units.len())
            .map(|_| UnitState::default())
            .collect();
        Self {
            program,
            units,
            errors: Vec::new(),
            warnings: Vec::new(),
            skipped: Vec::new(),
            failed_stages: Vec::new(),
        }
    }

    /// 记录一次跳过（拓扑判定，非阶段返回值——02 §4）。
    fn record_skipped(
        &mut self,
        stage: Stage,
        reason: SkipReason,
    ) {
        self.skipped.push(SkippedStage { stage, reason });
    }

    /// 聚合模式门控：FailFast 下首个失败阶段即中止（02 §3 草图 break）。
    fn should_abort(&self) -> bool {
        matches!(self.program.aggregation, Aggregation::FailFast) && !self.errors.is_empty()
    }

    /// 拓扑判定：本阶段声明的数据依赖中，落在本程序阶段表内且已失败的
    /// 首个上游阶段。
    fn first_failed_upstream(
        &self,
        stage: Stage,
    ) -> Option<Stage> {
        data_dependencies(stage)
            .iter()
            .find(|dep| self.program.stages().contains(dep) && self.failed_stages.contains(dep))
            .copied()
    }

    /// 收尾：产出 CompilationResult。失败时警告按 pipeline 现状一并丢弃
    /// （`CompilationResult::failed` 语义）；`total_duration_ms` 由
    /// 调用方（`Pipeline::run`）覆写。
    fn into_outcome(mut self) -> DriverOutcome {
        let result = if self.errors.is_empty() {
            // SingleFile：unit 0 的 IR 即最终产物；多单元合并（Linking）随 4.2
            let ir = self.units.iter_mut().find_map(|u| u.ir.take());
            CompilationResult {
                ir,
                error_count: 0,
                warning_count: self.warnings.len(),
                total_duration_ms: 0,
                errors: Vec::new(),
                warnings: self.warnings,
            }
        } else {
            CompilationResult::failed(self.errors, 0)
        };
        DriverOutcome {
            result,
            skipped: self.skipped,
        }
    }
}

/// 阶段的数据依赖边（02 §1 拓扑序的依据；与 tests/stage.rs 的规范侧
/// 清单对账——两边独立罗列，防单方漂移）。
fn data_dependencies(stage: Stage) -> &'static [Stage] {
    match stage {
        Stage::Registry => &[Stage::Discovery],
        Stage::Parsing => &[Stage::Registry],
        Stage::Typecheck => &[Stage::Parsing],
        Stage::ProofExecution => &[Stage::Typecheck],
        Stage::IrGeneration => &[Stage::GlobalSlotAlloc],
        Stage::Monomorphization => &[Stage::IrGeneration],
        Stage::Linking => &[Stage::IrGeneration],
        _ => &[],
    }
}

/// 单文件模式：扫描入口源码中的 `use std.X`，把命中的嵌入 std 模块
/// （std.test）编译为独立 ModuleIR 并合并进入口 IR。
///
/// 移植自 pipeline.rs:349-389。词法扫描失败返回 Ok（真正的错误由
/// parse 阶段报告）。`registry` 与入口 IR 生成共用（共享 SymbolTable），
/// 保证 DefId 一致（#94）。
fn merge_embedded_std_ir(
    source: &str,
    ir: &mut ModuleIR,
    registry: &ModuleRegistry,
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

#[cfg(test)]
mod tests;
