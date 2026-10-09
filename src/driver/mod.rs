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
//! **接线现状（4.2.2）**：12 个 `Stage` 变体全接线。SingleFile /
//! WasmPlayground 六臂、MultiFile 九臂、Check 八臂（VendorConsistency /
//! Discovery / Parsing / Registry / RoleClassification / Typecheck /
//! DeadCodeAnalysis / ProofExecution）已开通；Lsp/Embedded 入口随
//! 4.2.3/4.2.4 开通。
//! Skipped 只进内部记录、**不外发诊断**（外发随 4.3 义务账本，02 §4
//! 文案表）——zero-diff 判据（C2）要求任何新增对外诊断都必须显式裁决。
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

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::frontend::core::lexer;
use crate::frontend::core::parser::ast::{Expr, StmtKind};
use crate::frontend::core::parser::{self, Module};
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::core::typecheck::{self, TypeCheckResult};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::module::orchestrator::{self, OrchestratorError};
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::roles::{self, FileRole};
use crate::frontend::module::symbol::SymbolTable;
use crate::frontend::module::ModuleSource;
use crate::frontend::pipeline::{CompilationResult, PipelineError};
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::middle::passes::mono::Monomorphizer;
use crate::middle::ModuleIR;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

// manifest 解析依赖 package 模块（wasm32 下不编译）——角色上下文在 wasm 降级
#[cfg(not(target_arch = "wasm32"))]
use crate::package::manifest::PackageManifest;

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
    /// 强制机制）。12 变体全接线（4.2.2 收齐）。
    fn dispatch(
        &self,
        stage: Stage,
        state: &mut State,
    ) -> Result<(), DriverError> {
        match stage {
            Stage::RoleClassification => {
                self.role_classification(state);
                Ok(())
            }
            Stage::VendorConsistency => {
                self.vendor_consistency(state);
                Ok(())
            }
            Stage::Discovery => {
                self.discovery(state);
                Ok(())
            }
            Stage::Parsing => {
                self.parsing(state);
                Ok(())
            }
            Stage::Registry => {
                self.registry(state);
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
            Stage::GlobalSlotAlloc => {
                self.global_slot_alloc(state);
                Ok(())
            }
            Stage::IrGeneration => self.ir_generation(state),
            Stage::Monomorphization => self.monomorphization(state),
            Stage::Linking => {
                self.linking(state);
                Ok(())
            }
        }
    }

    /// `Stage::Parsing` 臂（PerModule）：lex → parse。双形态——
    /// 形状由 ProgramKind 定义（本阶段无上游产物可自适应）。
    fn parsing(
        &self,
        state: &mut State,
    ) {
        if state.uses_pipeline_channel() {
            self.parsing_single_file(state);
        } else {
            self.parsing_orchestrated(state);
        }
    }

    /// SingleFile/WasmPlayground 形态：移植自 pipeline.rs:149-171 /
    /// 229-258——parse 失败只取**首个**错误对外（逐字节判据的一部分）。
    fn parsing_single_file(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            asts,
            errors,
            failed_stages,
            ..
        } = state;
        for (i, ast_slot) in asts.iter_mut().enumerate() {
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
            *ast_slot = Some(result.module);
        }
    }

    /// 多文件形态：双形——
    /// - MultiFile：逐文件 `orchestrator::parse_file`，首错经
    ///   `OrchestratorError::Parse` 硬中止（FailFast 的正确语义：坏文件
    ///   产不出 IR；测试钉板长期有效）；
    /// - Check（WBS 4.10.1 修复，2026-10-09 用户裁决 rust 式收集语义）：
    ///   parse 失败降级为逐文件诊断收集（`parsing_check`）。
    fn parsing_orchestrated(
        &self,
        state: &mut State,
    ) {
        if matches!(state.program.kind, ProgramKind::Check) {
            return self.parsing_check(state);
        }
        let mut failure = None;
        for (i, ast_slot) in state.asts.iter_mut().enumerate() {
            let unit = &state.program.units[i];
            match orchestrator::parse_file(&unit.path, &unit.source) {
                Ok(ast) => *ast_slot = Some(ast),
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        if let Some(e) = failure {
            state.fail_with(Stage::Parsing, e);
        }
    }

    /// Check 形态（WBS 4.10.1）：parse 失败的文件**退出编译单元**——
    /// AST 槽保持 `None`，Registry/RoleClassification/Typecheck/
    /// DeadCodeAnalysis/ProofExecution 经既有 None 防御跳过该单元；
    /// 全量 parse 错误进该文件的收集通道（CollectAll，不止首错），
    /// 其余文件照常全阶段收集，最终由 CLI 汇总报错并非零退出
    ///（rust 式「收集所有错误再统一报告」语义）。带病 AST **不进**
    /// registry（方案 B 裁决：坏文件 = 不存在于本次编译——registry 永远
    /// 干净，导入方报模块未找到 E5001；部分签名参与解析的误导性次级
    /// 诊断由此归零）。
    fn parsing_check(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            asts,
            units,
            ..
        } = state;
        for (i, ast_slot) in asts.iter_mut().enumerate() {
            let source = &program.units[i].source;
            let tokens = match lexer::tokenize(source) {
                Ok(tokens) => tokens,
                Err(e) => {
                    // 词法失败：无 tokens 可 parse——该单元直接退出编译
                    units[i].check_diagnostics.push(e.to_diagnostic());
                    continue;
                }
            };
            let result = parser::parse(&tokens);
            if result.has_errors {
                units[i].check_diagnostics.extend(result.errors);
                continue;
            }
            *ast_slot = Some(result.module);
        }
    }

    /// `Stage::Typecheck` 臂（PerModule）：`check_module`。双形态——
    /// 环境由上游阶段产物供给（State 中有无 Registry 产物），这正是
    /// 阶段模型的本意。
    fn typecheck(
        &self,
        state: &mut State,
    ) {
        if state.uses_pipeline_channel() {
            self.typecheck_single_file(state);
        } else {
            self.typecheck_orchestrated(state);
        }
    }

    /// SingleFile/WasmPlayground 形态：移植自 pipeline.rs:173-183 /
    /// 262-285。警告**暂存**而非立即外发——是否外发由 DeadCodeAnalysis
    /// 臂的门控决定（忠实复刻 pipeline 的耦合：`dead_code.enabled ==
    /// false` 时 typecheck 警告一并丢弃）。
    fn typecheck_single_file(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            asts,
            units,
            errors,
            failed_stages,
            ..
        } = state;
        for (i, us) in units.iter_mut().enumerate() {
            let Some(ast) = &asts[i] else {
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

    /// 多文件形态（4.2.1）：移植自 compile_project 的逐文件 checker 装配
    /// （`TypeChecker::new("<module>")` + registry/method_bindings/vendor
    /// 注入）。checker 入口由 `Aggregation` 驱动二选一——02 §改动清单
    /// 4.2.7 的语义在此预置（FailFast → `check_module`，CollectAll →
    /// `check_module_collect_all`）。
    ///
    /// 双通道：MultiFile 首错经 `OrchestratorError::TypeCheck` 即返
    /// （compile_project 现状；`result.warnings` 丢弃——决策 B1）；Check
    /// （4.2.2）诊断进逐文件收集通道并**继续**下一文件（CollectAll），
    /// 警告暂存，由 DeadCodeAnalysis 臂按项目内门控外发（check_project
    /// 现状：类型失败文件的警告同样流出）。
    fn typecheck_orchestrated(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            asts,
            units,
            registry,
            method_bindings,
            vendor_root,
            failure,
            failed_stages,
            ..
        } = state;
        let (Some(registry), Some(method_bindings)) = (registry.as_ref(), method_bindings.as_ref())
        else {
            return; // 上游已失败（FailFast 下 run 循环已中止，防御）
        };
        let is_check = matches!(program.kind, ProgramKind::Check);
        let vendor_root = vendor_root.clone();
        for (i, us) in units.iter_mut().enumerate() {
            let Some(ast) = &asts[i] else {
                continue;
            };
            let mut checker = TypeChecker::new("<module>");
            checker.env().module_registry = registry.clone();
            checker.env().method_bindings = method_bindings.clone();
            // RFC-014：vendor 根注入（缺依赖包的 E5001 追加 install 提示）
            if let Some(root) = &vendor_root {
                checker.set_vendor_root(root.clone());
            }
            let mut result = match program.aggregation {
                Aggregation::FailFast => checker.check_module(ast),
                Aggregation::CollectAll => checker.check_module_collect_all(ast),
            };
            if is_check {
                // Check 通道：警告暂存（DeadCodeAnalysis 臂按 in_project
                // 门控外发）；诊断逐文件收集，不标阶段失败、不中止
                us.typecheck_warnings = std::mem::take(&mut result.warnings);
                if !result.diagnostics.is_empty() {
                    us.typecheck_failed = true;
                    us.check_diagnostics.extend(result.diagnostics);
                    continue;
                }
                us.type_result = Some(result);
                continue;
            }
            if !result.diagnostics.is_empty() {
                us.typecheck_failed = true;
                let message = result
                    .diagnostics
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                *failure = Some(OrchestratorError::TypeCheck {
                    path: program.units[i].path.display().to_string(),
                    message,
                    diagnostics: result.diagnostics,
                });
                failed_stages.push(Stage::Typecheck);
                return;
            }
            us.type_result = Some(result);
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
        if matches!(state.program.kind, ProgramKind::Check) {
            return self.dead_code_analysis_check(state);
        }
        let State {
            program,
            asts,
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
        for (i, us) in units.iter_mut().enumerate() {
            let Some(ast) = &asts[i] else {
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

    /// Check 形态（4.2.2）：移植自 check_project 主循环的逐文件警告装配。
    /// 与单文件形态的三处现状差异（即 check_project 与 pipeline 的既有
    /// 差异，警告面归一是 B1/4.9 的既定缓议）：
    /// - 无 `config.dead_code.enabled` 门控（check 不读该开关）；
    /// - 不按 `typecheck_failed` 跳过（死代码只看 AST，与类型错误并存）；
    /// - 角色感知分析器（RFC-029f：Test 不参与；Bin/Internal 按
    ///   `set_exempt_pub`、Lib/Script 按 `set_cross_file_refs` 接项目
    ///   引用池），W1006 遮蔽事件 / W1003 未使用导入 / 死代码警告仅对
    ///   **本项目文件**外发（vendor/嵌入 std 的内部警告不混入消费方）。
    fn dead_code_analysis_check(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            asts,
            units,
            shadow_events,
            entry_path,
            ..
        } = state;
        let project_root = orchestrator::find_project_root(entry_path);
        let project_root_canon = project_root
            .as_ref()
            .map(|root| root.canonicalize().unwrap_or_else(|_| root.clone()));
        let in_project = |file_canon: &Path| -> bool {
            project_root_canon
                .as_deref()
                .is_some_and(|root| file_canon.starts_with(root))
        };
        // RFC-029f Phase 2：包内引用池 = 项目根下全部 .yx 文件的标识符
        // 引用并集（含 Test——测试使用使产品绑定存活）。聚合全项目而非
        // 入口可达集：单文件 check 时引用方可能不在发现集，按可达集聚合
        // 会产生顺序敏感的误报。vendor/嵌入 std 不入池（外部包视角）
        let project_refs: HashSet<String> = project_root_canon
            .as_deref()
            .map(collect_project_refs)
            .unwrap_or_default();
        for (i, us) in units.iter_mut().enumerate() {
            let Some(ast) = &asts[i] else {
                continue; // 上游失败（FailFast 下 run 循环已中止，防御）
            };
            let path = &program.units[i].path;
            let file_canon = path.canonicalize().unwrap_or_else(|_| path.clone());
            if !in_project(&file_canon) {
                continue; // 警告仅对本项目文件呈现（RFC-029f）
            }
            // RFC-014 §项目模式：W1006 本地模块遮蔽依赖包（发现期事件，
            // 附着在做遮蔽 use 的文件上；Warning severity 不阻断）
            for (shadow_path, use_path, span, pkg) in shadow_events.iter() {
                if shadow_path == path {
                    // use 路径在模板里显示为文件式路径（data/json）
                    us.check_diagnostics.push(
                        ErrorCodeDefinition::module_shadows_dependency(
                            &use_path.replace('.', "/"),
                            pkg,
                        )
                        .at(*span)
                        .build(),
                    );
                }
            }
            // W1003 未使用导入随 W 码通道流出（Warning severity，不阻断）
            us.check_diagnostics
                .extend(std::mem::take(&mut us.typecheck_warnings));
            if matches!(us.role, Some(FileRole::Test)) {
                continue; // Test 不参与死代码判定（RFC-036 测试发现规则）
            }
            let mut analyzer = DeadCodeAnalyzer::new();
            match us.role {
                Some(FileRole::Bin | FileRole::Internal) => {
                    analyzer.set_exempt_pub(project_refs.clone());
                }
                Some(FileRole::Lib | FileRole::Script) => {
                    analyzer.set_cross_file_refs(project_refs.clone());
                }
                // Test 上面已跳过；None 防御不可达（RoleClassification 恒
                // 产角色）——不给池，退化为纯文件内可达判定
                _ => {}
            }
            let warnings = analyzer.analyze(ast);
            us.check_diagnostics
                .extend(analyzer.to_diagnostics(&warnings));
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
        let single_file = state.uses_pipeline_channel();
        let State {
            program,
            asts,
            units,
            errors,
            skipped,
            failed_stages,
            failure,
            ..
        } = state;
        let mut any_obligation = false;
        for (i, us) in units.iter_mut().enumerate() {
            let (Some(ast), Some(type_result)) = (&asts[i], &us.type_result) else {
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
                // Check 通道（4.2.2）：proof 失败是逐文件收集诊断——不标
                // 阶段失败、不中止（check_project 现状的 CollectAll 语义）
                if matches!(program.kind, ProgramKind::Check) {
                    us.check_diagnostics.extend(stage_errors);
                    continue;
                }
                if single_file {
                    errors.extend(stage_errors.into_iter().map(PipelineError::ProofExecution));
                } else {
                    // 多文件契约：proof 失败归入 TypeCheck 变体（compile_project
                    // 现状），路径指向出错文件
                    let message = stage_errors
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    *failure = Some(OrchestratorError::TypeCheck {
                        path: program.units[i].path.display().to_string(),
                        message,
                        diagnostics: stage_errors,
                    });
                }
                failed_stages.push(Stage::ProofExecution);
                if !single_file || matches!(program.aggregation, Aggregation::FailFast) {
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

    /// `Stage::IrGeneration` 臂（PerModule）：AST → ModuleIR。双形态——
    /// 单文件做嵌入 std 合并，多文件经 `generate_ir_with_context`
    /// （共享 registry + 跨文件上下文 + 槽位基址），合并由 Linking 负责。
    fn ir_generation(
        &self,
        state: &mut State,
    ) -> Result<(), DriverError> {
        if state.uses_pipeline_channel() {
            return self.ir_generation_single_file(state);
        }
        self.ir_generation_orchestrated(state);
        Ok(())
    }

    /// SingleFile/WasmPlayground 形态：移植自 pipeline.rs:205-227 / 302-341
    /// + `merge_embedded_std_ir`（#94 共享 registry）。IR 错误归入
    ///   `PipelineError::TypeCheck`（pipeline 现状映射）。
    fn ir_generation_single_file(
        &self,
        state: &mut State,
    ) -> Result<(), DriverError> {
        if state.units.len() != 1 {
            return Err(DriverError::MultiUnitNotWired(Stage::IrGeneration));
        }
        let State {
            program,
            asts,
            units,
            errors,
            failed_stages,
            ..
        } = state;
        let (Some(ast), Some(type_result)) = (&asts[0], &units[0].type_result) else {
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
        units[0].ir = Some(ir);
        Ok(())
    }

    /// 多文件形态（4.2.1）：移植自 compile_project Phase 2——预注册跨文件
    /// 上下文的 `generate_ir_with_context`，错误归入
    /// `OrchestratorError::Compile` 并首错即返。
    fn ir_generation_orchestrated(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            asts,
            units,
            registry,
            global_layout,
            slot_bases,
            failure,
            failed_stages,
            ..
        } = state;
        let (Some(registry), Some(global_layout)) = (registry.as_ref(), global_layout.as_ref())
        else {
            return; // 上游已失败（FailFast 下 run 循环已中止，防御）
        };
        // 跨文件上下文：所有文件的 AST 引用视图（结构体布局/全局名解析用）
        let all_ast_refs: Vec<&Module> = asts.iter().filter_map(|a| a.as_ref()).collect();
        for (i, us) in units.iter_mut().enumerate() {
            let (Some(ast), Some(type_result)) = (&asts[i], &us.type_result) else {
                continue;
            };
            let ir = match crate::middle::generate_ir_with_context(
                ast,
                type_result,
                &all_ast_refs,
                global_layout,
                slot_bases[i],
                registry,
                &program.units[i].key,
            ) {
                Ok(ir) => ir,
                Err(diags) => {
                    let message = diags
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    *failure = Some(OrchestratorError::Compile {
                        path: program.units[i].path.display().to_string(),
                        message,
                        diagnostics: diags,
                    });
                    failed_stages.push(Stage::IrGeneration);
                    return;
                }
            };
            us.ir = Some(ir);
        }
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

    /// `Stage::VendorConsistency` 臂（Project）：RFC-014 §项目模式
    /// vendor 与 yaoxiang.lock 一致性核对（实现留在 orchestrator——Node
    /// 语义：不静默自动安装；vendor 根供 Typecheck 注入 E5001 install 提示）。
    fn vendor_consistency(
        &self,
        state: &mut State,
    ) {
        match orchestrator::ensure_vendor_consistency(&state.entry_path) {
            Ok(root) => state.vendor_root = root,
            Err(e) => state.fail_with(Stage::VendorConsistency, e),
        }
    }

    /// `Stage::Discovery` 臂（Project）：沿 `use` BFS 发现编译单元集
    /// （#247；嵌入 std 模块以虚拟路径混入），按路径排序后**重建**程序的
    /// 单元列表——多文件 Program 构造时只有占位入口单元，本臂是唯一展开点。
    ///
    /// 附带产物（4.2.2）：`used_by` 边集（RoleClassification 的 Lib 信号）
    /// 与 W1006 遮蔽事件（DeadCodeAnalysis 的 Check 形态消费）入 State；
    /// MultiFile 形态的阶段表没有下游消费者，产物仅记录不外发。
    fn discovery(
        &self,
        state: &mut State,
    ) {
        match orchestrator::discover_with_used(&state.entry_path) {
            Ok((files, used_by, shadow_events)) => {
                state.program.units = files
                    .iter()
                    .map(|f| Unit::new(&f.module_key, &f.path, &f.source))
                    .collect();
                let n = state.program.units.len();
                state.units = (0..n).map(|_| UnitState::default()).collect();
                state.asts = (0..n).map(|_| None).collect();
                state.used_by = used_by;
                state.shadow_events = shadow_events;
            }
            Err(e) => state.fail_with(Stage::Discovery, e),
        }
    }

    /// `Stage::Registry` 臂（Project）：消费全部单元的 AST 产物做签名
    /// 收集（C3 拓扑修正——本臂不再自行 parse），构建共享 ModuleRegistry
    /// 与跨文件方法绑定（RFC-029）。
    fn registry(
        &self,
        state: &mut State,
    ) {
        let mut registry = ModuleRegistry::with_std();
        for (unit, ast) in state.program.units.iter().zip(state.asts.iter()) {
            let Some(ast) = ast else {
                continue; // 上游失败（FailFast 下 run 循环已中止，防御）
            };
            let mut info = orchestrator::extract_module_info(&unit.key, ast);
            // 来源标记：vendor 依赖目录下的文件是 Vendor（与本地 User 区分）
            if orchestrator::is_vendor_path(&unit.path) {
                info.source = ModuleSource::Vendor;
            }
            registry.register(info);
        }
        state.method_bindings = Some(registry.all_method_bindings());
        state.registry = Some(registry);
    }

    /// `Stage::RoleClassification` 臂（Project 作用域，臂内逐单元）：
    /// RFC-029f 角色分类（Script/Bin/Lib/Test/Internal），产物存
    /// `UnitState.role` 供 DeadCodeAnalysis 消费。
    ///
    /// 职责扩展（2026-10-09 用户裁决）：本臂同时承担「声明面入口一致性
    /// 校验」——manifest 声明面里的入口文件（`[[bin]].path` /
    /// `[run].main`）必须有名为 `main` 的绑定，缺失报 E3020 进逐文件
    /// 收集通道。归入本臂因为只有这里同时握着 surfaces 与 AST（compile
    /// 路径的同类校验在 Linking 臂——两边对称：握着判据的臂负责校验；
    /// check 路径没有 Linking 阶段）。
    fn role_classification(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            asts,
            units,
            used_by,
            entry_path,
            ..
        } = state;
        // 项目角色上下文：manifest 缺失或损坏 → surfaces 为 None → 全体
        // Script 态（行为与引入角色模型前一致）；wasm32 下 package 模块
        // 不编译，同样降级
        let project_root = orchestrator::find_project_root(entry_path);
        let (surfaces, test_rules) = role_context(project_root.as_deref());
        for (i, us) in units.iter_mut().enumerate() {
            let Some(ast) = &asts[i] else {
                continue; // 上游失败（FailFast 下 run 循环已中止，防御）
            };
            let path = &program.units[i].path;
            let file_canon = path.canonicalize().unwrap_or_else(|_| path.clone());
            let role = roles::classify(
                &file_canon,
                project_root.as_deref(),
                surfaces.as_ref(),
                test_rules.as_ref(),
                used_by.contains(&file_canon),
                ast_has_main(ast),
            );
            // 入口存在性（#388 定案 check 半边判据）：声明面入口必须有
            // main 绑定（值/函数皆可）；声明面之外的文件不查——Lib/
            // Internal/Test 不要求 main（被 check ≠ 被当程序跑）
            if surfaces
                .as_ref()
                .is_some_and(|s| s.bins.contains(&file_canon))
                && !ast_has_main(ast)
            {
                // 无源码位置的文件级诊断用占位 span（原 entry_span 语义）
                us.check_diagnostics.push(
                    ErrorCodeDefinition::bin_missing_main(&path.display().to_string())
                        .at(Span::default())
                        .build(),
                );
            }
            us.role = Some(role);
        }
    }

    /// `Stage::GlobalSlotAlloc` 臂（Project）：为所有文件分配不相交的
    /// 全局槽位区间（T5——引用方与定义方共享同一份布局）。
    fn global_slot_alloc(
        &self,
        state: &mut State,
    ) {
        let (layout, bases) = allocate_global_slots(&state.program.units, &state.asts);
        state.global_layout = Some(layout);
        state.slot_bases = bases;
    }

    /// `Stage::Linking` 臂（Project）：IR 层链接（限定名共存）+ 入口
    /// 语义校验（#388 缺 main / #357 带参 main，自 compile_project 移入）。
    fn linking(
        &self,
        state: &mut State,
    ) {
        let mut irs: Vec<(String, ModuleIR)> = Vec::new();
        for (i, us) in state.units.iter_mut().enumerate() {
            if let Some(ir) = us.ir.take() {
                irs.push((state.program.units[i].path.display().to_string(), ir));
            }
        }
        let merged = match link_module_irs(irs, &state.entry_key) {
            Ok(merged) => merged,
            Err(e) => {
                state.fail_with(Stage::Linking, e);
                return;
            }
        };
        // T4：入口语义（RFC-029f 角色驱动；#388 定案：按**绑定存在性**判定）。
        // Script 无入口概念（顶层代码即程序）；Bin 要求存在名为 main 的绑定
        // （值/函数皆可）；带参 main 编译期拒绝（#357：入口恒零参调用）。
        if is_bin_role(&state.entry_path) {
            let qualified_main = format!("{}.main", state.entry_key);
            let has_main_fn = merged.functions.iter().any(|f| f.name == qualified_main);
            let has_main_value = merged
                .globals
                .iter()
                .any(|g| g.name == "main" || g.name == qualified_main);
            if !has_main_fn && !has_main_value {
                let diag =
                    ErrorCodeDefinition::bin_missing_main(&state.entry_path.display().to_string())
                        .at(Span::default())
                        .build();
                state.fail_with(
                    Stage::Linking,
                    OrchestratorError::TypeCheck {
                        path: state.entry_path.display().to_string(),
                        message: diag.to_string(),
                        diagnostics: vec![diag],
                    },
                );
                return;
            }
            if let Some(main_fn) = merged.functions.iter().find(|f| f.name == qualified_main) {
                if !main_fn.params.is_empty() {
                    let found = main_fn
                        .params
                        .iter()
                        .map(|p| format!("{p}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let diag = ErrorCodeDefinition::bin_main_signature("main", "()", &found)
                        .at(Span::default())
                        .build();
                    state.fail_with(
                        Stage::Linking,
                        OrchestratorError::TypeCheck {
                            path: state.entry_path.display().to_string(),
                            message: diag.to_string(),
                            diagnostics: vec![diag],
                        },
                    );
                    return;
                }
            }
        }
        state.merged_ir = Some(merged);
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
///
/// 产物通道按 ProgramKind 分（各入口的外部契约各归其主，02 草图的单一
/// CompilationResult 无法表达 OrchestratorError 契约）：
/// - SingleFile/WasmPlayground：`result`（pipeline 契约；IR 在 `result.ir`）；
/// - MultiFile：`module`（Linking 产物）+ `failure`（OrchestratorError
///   契约）；Check：`check_diagnostics`（逐文件收集通道，4.2.2）；
///   Lsp/Embedded 的通道随 4.2.3/4.2.4 按需扩展。
#[derive(Debug)]
pub struct DriverOutcome {
    /// 编译结果（诊断集与 pipeline 时代逐字节相同，C2 判据）
    pub result: CompilationResult,
    /// 多文件合并 IR（Linking 产物；MultiFile 成功时 Some）
    pub module: Option<ModuleIR>,
    /// 多文件结构化故障（OrchestratorError 契约；失败时 Some）
    pub failure: Option<OrchestratorError>,
    /// 逐文件诊断收集（Check 契约：`Vec<(文件, 该文件诊断)>`，每个发现文件
    /// 都有条目——干净文件为空 Vec；4.2.2）
    pub check_diagnostics: Vec<(PathBuf, Vec<Diagnostic>)>,
    /// 被跳过的阶段及原因——仅内部记录；对外 Skipped 诊断随 4.3 义务账本接入（02 §4）
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
    /// Parsing 产物（独立存放：多文件 IrGeneration 需要全部 AST 的引用
    /// 视图——挂在 UnitState 上会与逐单元可变访问冲突；orchestrator 时代
    /// asts 同样是独立 Vec）
    asts: Vec<Option<Module>>,
    /// 已产出的错误诊断（FailFast 下恰为首个失败阶段的那批）
    errors: Vec<PipelineError>,
    /// 已产出的警告诊断
    warnings: Vec<Diagnostic>,
    /// 跳过记录（内部审计；4.3 前不外发）
    skipped: Vec<SkippedStage>,
    /// 已失败的阶段（拓扑跳过的判定依据）
    failed_stages: Vec<Stage>,
    /// 入口路径与模块键（Discovery 重建 units 前从占位入口单元捕获）
    entry_path: PathBuf,
    /// 入口模块键（Linking 的 `{entry_key}.main` 查找用）
    entry_key: String,
    /// 多文件结构化故障（OrchestratorError 契约；与 errors 通道互斥——
    /// 通道选择见 `uses_pipeline_channel`）
    failure: Option<OrchestratorError>,
    /// Discovery 产物：被 ≥1 个文件 `use` 的文件路径集（canonical 化）——
    /// RoleClassification 的 Lib 信号（RFC-029f；4.2.2）
    used_by: HashSet<PathBuf>,
    /// Discovery 产物：本地模块遮蔽依赖包事件（W1006，RFC-014 §项目模式）
    /// ——DeadCodeAnalysis 的 Check 形态消费（4.2.2）
    shadow_events: Vec<orchestrator::ShadowEvent>,
    /// VendorConsistency 产物（Typecheck 的 E5001 install 提示注入用）
    vendor_root: Option<PathBuf>,
    /// Registry 产物（多文件 Typecheck / IrGeneration 的共享注册表）
    registry: Option<ModuleRegistry>,
    /// Registry 产物（跨文件方法调用解析，RFC-029）
    method_bindings: Option<HashMap<String, MonoType>>,
    /// GlobalSlotAlloc 产物（限定名 → 绝对槽位号）
    global_layout: Option<Vec<(String, usize)>>,
    /// GlobalSlotAlloc 产物（各文件槽位基址）
    slot_bases: Vec<usize>,
    /// Linking 产物
    merged_ir: Option<ModuleIR>,
}

/// 逐编译单元的阶段产物（PerModule 阶段的执行粒度）。
#[derive(Default)]
struct UnitState {
    /// Typecheck 产物
    type_result: Option<TypeCheckResult>,
    /// typecheck 警告暂存——外发与否由 DeadCodeAnalysis 臂的门控决定
    /// （pipeline 现状耦合，见该臂注释）
    typecheck_warnings: Vec<Diagnostic>,
    /// 本单元 typecheck 是否产出错误（死代码门控的 `!has_errors` 半边）
    typecheck_failed: bool,
    /// IrGeneration（+ Monomorphization 改写）产物
    ir: Option<ModuleIR>,
    /// RoleClassification 产物（RFC-029f 角色；DeadCodeAnalysis 的消费端，4.2.2）
    role: Option<FileRole>,
    /// 逐文件诊断收集（Check 通道；RoleClassification / Typecheck /
    /// DeadCodeAnalysis / ProofExecution 四臂按拓扑序追加——文件内顺序
    /// 即阶段序，2026-10-09 用户裁决的顺序归一）
    check_diagnostics: Vec<Diagnostic>,
}

impl State {
    fn new(program: Program) -> Self {
        // 占位入口单元（多文件形态）或唯一单元（单文件形态）——Discovery
        // 臂会重建 units，入口信息先行捕获（run() 已拒绝空 units）
        let entry_path = program.units[0].path.clone();
        let entry_key = program.units[0].key.clone();
        let n = program.units.len();
        Self {
            program,
            units: (0..n).map(|_| UnitState::default()).collect(),
            asts: (0..n).map(|_| None).collect(),
            errors: Vec::new(),
            warnings: Vec::new(),
            skipped: Vec::new(),
            failed_stages: Vec::new(),
            entry_path,
            entry_key,
            failure: None,
            used_by: HashSet::new(),
            shadow_events: Vec::new(),
            vendor_root: None,
            registry: None,
            method_bindings: None,
            global_layout: None,
            slot_bases: Vec::new(),
            merged_ir: None,
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

    /// 结果通道选择：SingleFile/WasmPlayground 走 PipelineError 通道
    /// （pipeline 外部契约）；其余形态走 OrchestratorError 故障通道
    /// （orchestrator 外部契约，4.2.1）。Check 的诊断主体不走这两条——
    /// 逐文件收集进 `check_diagnostics`（4.2.2），故障通道只承载
    /// vendor/发现/解析三类硬中止。
    fn uses_pipeline_channel(&self) -> bool {
        matches!(
            self.program.kind,
            ProgramKind::SingleFile | ProgramKind::WasmPlayground
        )
    }

    /// 记录一次多文件故障（OrchestratorError 通道）并标记阶段失败。
    fn fail_with(
        &mut self,
        stage: Stage,
        error: OrchestratorError,
    ) {
        self.failure = Some(error);
        self.failed_stages.push(stage);
    }

    /// 聚合模式门控：FailFast 下首个失败阶段即中止（02 §3 草图 break）；
    /// 多文件故障一律中止（compile_project 现状：`?` 传播）。
    fn should_abort(&self) -> bool {
        if self.failure.is_some() {
            return true;
        }
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

    /// 收尾：按通道产出。多文件故障直接进 `failure`（OrchestratorError
    /// 契约）；单文件失败时警告按 pipeline 现状一并丢弃
    /// （`CompilationResult::failed` 语义）；`total_duration_ms` 由
    /// 调用方（`Pipeline::run`）覆写。
    fn into_outcome(mut self) -> DriverOutcome {
        if let Some(error) = self.failure.take() {
            return DriverOutcome {
                result: CompilationResult::default(),
                module: None,
                failure: Some(error),
                check_diagnostics: Vec::new(),
                skipped: self.skipped,
            };
        }
        // Check 通道（4.2.2）：逐文件诊断收集——每个发现文件都有条目
        //（干净文件为空 Vec，check_project 契约）；其余形态恒空
        let check_diagnostics = if matches!(self.program.kind, ProgramKind::Check) {
            let units_meta = &self.program.units;
            self.units
                .iter_mut()
                .enumerate()
                .map(|(i, us)| {
                    (
                        units_meta[i].path.clone(),
                        std::mem::take(&mut us.check_diagnostics),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        let single_file = self.uses_pipeline_channel();
        let result = if self.errors.is_empty() {
            // SingleFile：unit 0 的 IR 即最终产物
            let ir = if single_file {
                self.units.iter_mut().find_map(|u| u.ir.take())
            } else {
                None
            };
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
            module: self.merged_ir.take(),
            failure: None,
            check_diagnostics,
            skipped: self.skipped,
        }
    }
}

/// 阶段的数据依赖边（02 §1 拓扑序的依据；与 tests/stage.rs 的规范侧
/// 清单对账——两边独立罗列，防单方漂移）。
fn data_dependencies(stage: Stage) -> &'static [Stage] {
    match stage {
        Stage::Parsing => &[Stage::Discovery],
        Stage::Registry => &[Stage::Parsing],
        // 4.2.2 补边：消费 Discovery 的 used_by 边集（Lib 信号）
        Stage::RoleClassification => &[Stage::Discovery, Stage::Parsing],
        Stage::Typecheck => &[Stage::Parsing, Stage::Registry],
        // 4.2.2 补边：消费 Discovery 的 W1006 遮蔽事件
        Stage::DeadCodeAnalysis => &[Stage::Discovery, Stage::Parsing, Stage::RoleClassification],
        Stage::ProofExecution => &[Stage::Typecheck],
        Stage::GlobalSlotAlloc => &[Stage::Parsing],
        Stage::IrGeneration => &[Stage::Parsing, Stage::Registry, Stage::GlobalSlotAlloc],
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
    // 该协议现由纯 yx 模块提供（`src/std/list.yx`，#117 硬切换）。
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

/// 为所有文件分配不相交的全局槽位区间（T5）——自 orchestrator 移入
/// （4.2.1；入参由 owned 元组切片改为「单元 + AST 槽」引用视图，避免全量
/// AST clone——原签名里的 PathBuf 本就未被使用）。
///
/// 返回 `(限定名 → 绝对槽位号, 各文件的槽位基址)`。槽位号按发现顺序连续
/// 分配；限定名用 `{module_key}.{name}`（与 `SymbolTable::qualify` 同源），
/// 使 `use lib.{value}` 解析到 `lib.value`。只登记**真正的值绑定**
/// （非函数）——函数进函数表，不占全局槽位。
fn allocate_global_slots(
    units: &[Unit],
    asts: &[Option<Module>],
) -> (Vec<(String, usize)>, Vec<usize>) {
    let mut layout: Vec<(String, usize)> = Vec::new();
    let mut bases: Vec<usize> = Vec::with_capacity(units.len());
    let mut next_slot = 0usize;
    for (unit, ast) in units.iter().zip(asts.iter()) {
        bases.push(next_slot);
        let Some(ast) = ast else {
            continue; // 上游失败（FailFast 下 run 循环已中止，防御）
        };
        for (name, _ty) in extract_global_defs(ast) {
            layout.push((SymbolTable::qualify(&unit.key, &name), next_slot));
            next_slot += 1;
        }
    }
    (layout, bases)
}

/// 提取顶层**值绑定**（非函数）——自 orchestrator 移入（4.2.1）。
fn extract_global_defs(ast: &Module) -> Vec<(String, MonoType)> {
    let mut out = Vec::new();
    for stmt in &ast.items {
        if let StmtKind::Assign {
            target,
            type_annotation,
            value,
            ..
        } = &stmt.kind
        {
            if let Expr::Var(name, _) = target.as_ref() {
                // RFC-010a 附录D + B 方案（33f2ebbe）：Lambda → 函数；注解是
                // Fn → 函数；其余（含无注解块，内容决定类型）→ 块值。
                // 此前这里写「Block 就是函数」，把 `x: Int = { 5 }`
                // 误判为函数——与 ir_gen 的注册口径不一致，导致跨文件引用
                // `use lib.{x}` 找不到槽位（T5）。
                let is_fn =
                    Expr::block_binding_is_function(type_annotation.as_ref(), value.as_deref())
                        || matches!(value.as_deref(), Some(Expr::Lambda { .. }));
                if !is_fn {
                    let ty = type_annotation
                        .as_ref()
                        .map(|t| MonoType::from(t.clone()))
                        .unwrap_or(MonoType::Int(64));
                    out.push((name.clone(), ty));
                }
            }
        }
    }
    out
}

/// 链接多个文件的 `ModuleIR`：拼接函数/全局/FFI，合并 per-function 映射
/// ——自 orchestrator 移入（4.2.1）。
///
/// 各文件的函数已带模块限定名（`qualify_module_ir`），跨文件同名函数天然共存。
/// 仅当同一限定名出现两次（同名文件重复发现等病态情形）才报错。入口函数：
/// main 为函数绑定时设 `{entry_key}.main`；值 main 时为 None——程序体就是
/// 初始化序列（#388 定案：值 main 初始化期求值即执行）。
fn link_module_irs(
    irs: Vec<(String, ModuleIR)>,
    entry_key: &str,
) -> Result<ModuleIR, OrchestratorError> {
    let mut seen: HashMap<String, String> = HashMap::new();
    for (path, ir) in &irs {
        for func in &ir.functions {
            if let Some(prev) = seen.get(&func.name) {
                return Err(OrchestratorError::Collision {
                    name: func.name.clone(),
                    first: prev.clone(),
                    second: path.clone(),
                });
            }
            seen.insert(func.name.clone(), path.clone());
        }
    }

    let mut merged = ModuleIR {
        globals: Vec::new(),
        functions: Vec::new(),
        init: Vec::new(),
        init_locals: Vec::new(),
        init_file_ids: Vec::new(),
        ffi_libs: Vec::new(),
        ffi_bindings: Vec::new(),
        entry_function: None,
        source_files: irs.iter().map(|(p, _)| p.clone()).collect(),
        function_files: HashMap::new(),
    };
    for (i, (_, ir)) in irs.iter().enumerate() {
        for func in &ir.functions {
            merged.function_files.insert(func.name.clone(), i);
        }
    }
    for (file_idx, (_, ir)) in irs.into_iter().enumerate() {
        merged.globals.extend(ir.globals);
        merged.functions.extend(ir.functions);
        // T5：各文件的初始化序列按发现顺序拼接（被依赖模块先于入口文件）。
        // #368：每条指令记下所属文件——多文件下按它给 debug span 定 file_id，
        // 否则所有段的错误都指向同一个（错的）文件。
        merged
            .init_file_ids
            .extend(std::iter::repeat_n(file_idx, ir.init.len()));
        merged.init.extend(ir.init);
        // 不合并 init_locals：各文件槽位号从 0 起算，直接拼接会错位。
        // 多文件下顶层只有声明（可执行语句被 E3023 拒），具名局部仅出现在
        // Script 模式，故此处不需要它。
        merged.ffi_libs.extend(ir.ffi_libs);
        merged.ffi_bindings.extend(ir.ffi_bindings);
    }
    // 槽位号已由 allocate_global_slots 全局唯一，此处仅按索引稳定排序，
    // 便于调试与运行时按索引直取。
    merged.globals.sort_by_key(|g| g.index);

    // 入口函数仅在 main 为**函数**绑定时设置：值 main 的程序体就是初始化
    // 序列（求值即执行），无需入口调用——与 Script 同款执行形态（#356
    // 防双跑：初始化与入口调用绝不叠加）。
    let entry_fn = format!("{}.main", entry_key);
    if merged.functions.iter().any(|f| f.name == entry_fn) {
        merged.entry_function = Some(entry_fn);
    }

    Ok(merged)
}

/// T4：入口文件是否必须定义 `main`（Bin 要求）——自 orchestrator 移入（4.2.1）。
///
/// 判据：项目里有 manifest（`yaoxiang.toml`）。
///
/// 为何不用 `roles::classify`：那个模型回答「这个文件被谁消费」（服务于死代码
/// 分析），而这里要回答的是「这个文件能不能当程序跑」——两回事。有 manifest
/// 却没 `main` 的文件在 classify 里是 Internal（合理：它不被别的文件 use），
/// 但用户刚把它当入口跑了，此时必须有入口，否则静默什么都不做。
///
/// 对无 manifest 的单文件直跑（Script 角色）：顶层语句即程序主体，
/// 无需 main——这是「默认情况零门槛」。
fn is_bin_role(entry: &std::path::Path) -> bool {
    crate::frontend::module::orchestrator::find_project_root(entry).is_some()
}

/// 顶层是否存在 `main` 绑定（角色推断的 Bin 信号，RFC-029f）——自
/// orchestrator 移入（4.2.2；唯一消费端是 RoleClassification 臂）。
fn ast_has_main(ast: &Module) -> bool {
    ast.items.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            StmtKind::Assign { target, .. }
                if matches!(target.as_ref(), Expr::Var(name, _) if name == "main")
        )
    })
}

/// 项目角色上下文（RFC-029f）：显式声明面 + 测试发现规则——自
/// orchestrator 移入（4.2.2；唯一消费端是 RoleClassification 臂）。
/// manifest 读取解析依赖 `crate::package`——wasm32 下不编译，降级为
/// `(None, None)`（全体 Script 态、无 patterns 规则，行为与引入模型前一致）。
#[cfg(not(target_arch = "wasm32"))]
fn role_context(
    project_root: Option<&Path>
) -> (Option<roles::ExplicitSurfaces>, Option<roles::TestRules>) {
    let Some(root) = project_root else {
        return (None, None);
    };
    let Ok(src) = std::fs::read_to_string(root.join(crate::package::manifest::MANIFEST_FILE))
    else {
        return (None, None);
    };
    let surfaces = toml::from_str::<PackageManifest>(&src)
        .ok()
        .map(|manifest| roles::TargetViews {
            bins: manifest
                .bin
                .iter()
                .map(|b| b.path.clone())
                .chain(manifest.run.as_ref().and_then(|r| r.main.clone()))
                .collect(),
            libs: manifest
                .exports
                .values()
                .cloned()
                .chain(manifest.lib.as_ref().map(|l| l.path.clone()))
                .collect(),
        })
        .map(|views| roles::explicit_surfaces(&views, root));
    let test_rules = toml::from_str::<crate::util::config::ProjectConfig>(&src)
        .ok()
        .map(|config| roles::TestRules::from_config(&config.tool.test));
    (surfaces, test_rules)
}

#[cfg(target_arch = "wasm32")]
fn role_context(
    _project_root: Option<&Path>
) -> (Option<roles::ExplicitSurfaces>, Option<roles::TestRules>) {
    (None, None)
}

/// 收集项目根下全部 `.yx` 文件的标识符引用并集（RFC-029f Phase 2 引用池）
/// ——自 orchestrator 移入（4.2.2；唯一消费端是 DeadCodeAnalysis 臂的
/// Check 形态）。
///
/// 跳过 `.git`/`.yaoxiang`（vendor）/`target` 目录；词法或语法失败的文件
/// 静默跳过——引用池是容错的辅助判定，不允许它制造新的检查失败。
fn collect_project_refs(project_root: &Path) -> HashSet<String> {
    fn is_excluded_dir(dir: &Path) -> bool {
        dir.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name == ".git" || name == ".yaoxiang" || name == "target"
        })
    }

    fn collect_yx(
        dir: &Path,
        out: &mut Vec<PathBuf>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !is_excluded_dir(&path) {
                    collect_yx(&path, out);
                }
            } else if path.extension().is_some_and(|e| e == "yx") {
                out.push(path);
            }
        }
    }

    let mut refs = HashSet::new();
    let mut files = Vec::new();
    collect_yx(project_root, &mut files);
    for path in files {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(tokens) = lexer::tokenize(&source) else {
            continue;
        };
        let parsed = parser::parse(&tokens);
        refs.extend(DeadCodeAnalyzer::collect_ident_refs(&parsed.module));
    }
    refs
}

#[cfg(test)]
mod tests;
