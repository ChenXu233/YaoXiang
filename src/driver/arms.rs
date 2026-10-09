//! 阶段臂 —— `impl Driver` 的第二部分（02 §目标设计 3/4）。
//!
//! 12 个 `Stage` 变体的执行体全部在此（4.2.9 自 mod.rs 拆出——1766 行
//! 超仓库 1500 行拆分阈值）。臂只读 `Program`、读写 `State`；跳过
//! 由 run 循环按拓扑判定，臂自身不决定跳过。跨形态的臂按 ProgramKind
//! 分形（形状选择即形态契约，各形注释标注现状出处）。

use std::collections::HashSet;
use std::path::Path;

use crate::frontend::core::lexer;
use crate::frontend::core::parser::{self, Module};
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::{self};
use crate::frontend::core::typecheck::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::module::orchestrator::{self, OrchestratorError};
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::roles::{self, FileRole};
use crate::frontend::module::ModuleSource;
use crate::frontend::pipeline::PipelineError;
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::middle::passes::mono::Monomorphizer;
use crate::middle::ModuleIR;
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::util::span::Span;

use super::helpers::{
    allocate_global_slots, ast_has_main, collect_project_refs, is_bin_role, link_module_irs,
    merge_embedded_std_ir, role_context,
};
use super::state::{DriverError, SkipReason, SkippedStage, State, UnitState};
use super::{Aggregation, Driver, ProgramKind, Stage, Unit};

impl Driver {
    /// `Stage::Parsing` 臂（PerModule）：lex → parse。双形态——
    /// 形状由 ProgramKind 定义（本阶段无上游产物可自适应）。
    pub(super) fn parsing(
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
    /// 229-258——FailFast 下 parse 失败只取**首个**错误对外（逐字节
    /// 判据的一部分）；CollectAll（LSP 兜底，4.2.6）收全量 parse 错误
    /// 并保留残缺 AST 继续 typecheck（编辑器哲学，4.2.3 裁决延伸）。
    pub(super) fn parsing_single_file(
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
                if matches!(program.aggregation, Aggregation::CollectAll) {
                    // CollectAll：全量 parse 错误 + 残缺 AST 继续下游——
                    // 不标阶段失败（残缺 AST 仍是可用产物；标了会触发拓扑
                    // 跳过，typecheck 永远跑不到——编辑器哲学落空）
                    errors.extend(result.errors.into_iter().map(PipelineError::LexParse));
                    *ast_slot = Some(result.module);
                    continue;
                }
                let first = result.errors.into_iter().next().unwrap_or_else(|| {
                    ErrorCodeDefinition::unexpected_token("unknown")
                        .at(Span::dummy())
                        .build()
                });
                errors.push(PipelineError::LexParse(first));
                failed_stages.push(Stage::Parsing);
                return;
            }
            *ast_slot = Some(result.module);
        }
    }

    /// 多文件形态：三形——
    /// - MultiFile：逐文件 `orchestrator::parse_file`，首错经
    ///   `OrchestratorError::Parse` 硬中止（FailFast 的正确语义：坏文件
    ///   产不出 IR；测试钉板长期有效）；
    /// - Check（WBS 4.10.1 修复，2026-10-09 用户裁决 rust 式收集语义）：
    ///   parse 失败降级为逐文件诊断收集（`parsing_check`）；
    /// - Lsp（4.2.3）：按文件来源分流（`parsing_lsp`）。
    pub(super) fn parsing_orchestrated(
        &self,
        state: &mut State,
    ) {
        match state.program.kind {
            ProgramKind::Check => return self.parsing_check(state),
            ProgramKind::Lsp => return self.parsing_lsp(state),
            _ => {}
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
    pub(super) fn parsing_check(
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

    /// Lsp 形态（4.2.3，按文件来源分流裁决，2026-10-09）：
    /// - **被编辑缓冲区**（入口单元，源码来自内存）：编辑器哲学——
    ///   parse 错误收集后**保留残缺 AST** 继续 typecheck（打字中间态
    ///   不该让补全/跳转的世界消失）；词法失败无 tokens 可恢复，
    ///   只收诊断（旧行为此处硬中止，handler 被迫退回单文件路径）；
    /// - **磁盘文件**：与 Check 同款的方案 B（收集 + 退出编译单元）——
    ///   修复旧行为里 `build_registry_from` 对无关磁盘文件硬中止、
    ///   handler 静默退回单文件路径的怪癖（两端降级策略收敛，「不报
    ///   误导性次级错误」的优点在 LSP 侧同样成立）。
    pub(super) fn parsing_lsp(
        &self,
        state: &mut State,
    ) {
        let State {
            program,
            asts,
            units,
            entry_path,
            ..
        } = state;
        for (i, ast_slot) in asts.iter_mut().enumerate() {
            let is_buffer = program.units[i].path == *entry_path;
            let source = &program.units[i].source;
            let tokens = match lexer::tokenize(source) {
                Ok(tokens) => tokens,
                Err(e) => {
                    units[i].check_diagnostics.push(e.to_diagnostic());
                    continue;
                }
            };
            let result = parser::parse(&tokens);
            if result.has_errors {
                units[i].check_diagnostics.extend(result.errors);
                // 缓冲区保留残缺 AST（编辑器哲学）；磁盘文件退出编译单元
                if is_buffer {
                    *ast_slot = Some(result.module);
                }
                continue;
            }
            *ast_slot = Some(result.module);
        }
    }

    /// `Stage::Typecheck` 臂（PerModule）：`check_module`。双形态——
    /// 环境由上游阶段产物供给（State 中有无 Registry 产物），这正是
    /// 阶段模型的本意。
    pub(super) fn typecheck(
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
    pub(super) fn typecheck_single_file(
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
            // checker 入口由 Aggregation 驱动（4.2.7 语义）：FailFast
            // 首错即返；CollectAll（LSP 兜底）收集模式
            let mut type_result = match program.aggregation {
                Aggregation::FailFast => typecheck::check_module(ast, &mut None),
                Aggregation::CollectAll => typecheck::check_module_collect_all(ast, &mut None),
            };
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
    pub(super) fn typecheck_orchestrated(
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
            entry_path,
            ..
        } = state;
        // Embedded 形态无 Registry 阶段——registry 由调用方注入
        //（shared_registry，#94）；method_bindings 仅 Registry 臂产出
        //（Embedded 现状不注入，保持空表）
        let Some(registry) = registry.as_ref() else {
            return; // 上游已失败（FailFast 下 run 循环已中止，防御）
        };
        // 逐文件收集通道（Check/Lsp）；Lsp 只检目标文件——其余单元
        // 仅为 registry 供签名，不参与 typecheck（现状契约）
        let collect_per_file = matches!(program.kind, ProgramKind::Check | ProgramKind::Lsp);
        let is_lsp = matches!(program.kind, ProgramKind::Lsp);
        let is_embedded = matches!(program.kind, ProgramKind::Embedded);
        let vendor_root = vendor_root.clone();
        for (i, us) in units.iter_mut().enumerate() {
            let Some(ast) = &asts[i] else {
                continue;
            };
            if is_lsp && program.units[i].path != *entry_path {
                continue; // LSP 只 typecheck 目标文件
            };
            // checker 名按形态保现状（Embedded 用 "<embedded>"）
            let mut checker = TypeChecker::new(if is_embedded {
                "<embedded>"
            } else {
                "<module>"
            });
            checker.env().module_registry = registry.clone();
            if let Some(method_bindings) = method_bindings {
                checker.env().method_bindings = method_bindings.clone();
            }
            // RFC-014：vendor 根注入（缺依赖包的 E5001 追加 install 提示）
            if let Some(root) = &vendor_root {
                checker.set_vendor_root(root.clone());
            }
            let mut result = match program.aggregation {
                Aggregation::FailFast => checker.check_module(ast),
                Aggregation::CollectAll => checker.check_module_collect_all(ast),
            };
            if collect_per_file {
                // 诊断逐文件收集，不标阶段失败、不中止。警告：Check
                // 暂存（DeadCodeAnalysis 臂按 in_project 门控外发）；
                // Lsp 无 DeadCodeAnalysis 阶段，警告随 typecheck 直接
                // 入通道（文件内顺序归一：typecheck 诊断 → W 码警告 →
                // proof 错误——02 §1 注记 #4 同原则延伸）
                let warnings = std::mem::take(&mut result.warnings);
                if result.diagnostics.is_empty() {
                    if is_lsp {
                        us.check_diagnostics.extend(warnings);
                    } else {
                        us.typecheck_warnings = warnings;
                    }
                    us.type_result = Some(result);
                } else {
                    us.typecheck_failed = true;
                    us.check_diagnostics.extend(result.diagnostics);
                    if is_lsp {
                        us.check_diagnostics.extend(warnings);
                    } else {
                        us.typecheck_warnings = warnings;
                    }
                }
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
    pub(super) fn dead_code_analysis(
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
    pub(super) fn dead_code_analysis_check(
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
        // 4.2.5：无项目根（standalone 单文件）时，警告面只覆盖入口文件
        // 本身（check_single_file 的单文件语义直译）；被 use 带入的邻旁
        // 文件只收错误、不警告（无项目引用池兜底，宁漏勿误）
        let in_project = |file_canon: &Path, raw_path: &Path| -> bool {
            match project_root_canon.as_deref() {
                Some(root) => file_canon.starts_with(root),
                None => raw_path == entry_path.as_path(),
            }
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
            if !in_project(&file_canon, path) {
                continue; // 警告仅对本项目文件呈现（RFC-029f；standalone 仅入口文件）
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
    pub(super) fn proof_execution(
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
                // Check/Lsp 通道（4.2.2/4.2.3）：proof 失败是逐文件收集
                // 诊断——不标阶段失败、不中止（CollectAll 语义）
                if matches!(program.kind, ProgramKind::Check | ProgramKind::Lsp) {
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

    /// `Stage::IrGeneration` 臂（PerModule）：AST → ModuleIR。多形态——
    /// 单文件做嵌入 std 合并，多文件经 `generate_ir_with_context`
    ///（共享 registry + 跨文件上下文 + 槽位基址），合并由 Linking 负责；
    /// Embedded 以空调用面产独立 IR；Check 纯检查不消费 IR。
    pub(super) fn ir_generation(
        &self,
        state: &mut State,
    ) -> Result<(), DriverError> {
        if state.uses_pipeline_channel() {
            return self.ir_generation_single_file(state);
        }
        if matches!(state.program.kind, ProgramKind::Embedded) {
            self.ir_generation_embedded(state);
            return Ok(());
        }
        if matches!(state.program.kind, ProgramKind::Check) {
            self.ir_generation_check(state);
            return Ok(());
        }
        self.ir_generation_orchestrated(state);
        Ok(())
    }

    /// Check 形态（4.2.5，用户裁决）：IR 生成在 check 里是**检查**不是
    /// 产物——逐文件收集 IR 级错误（E3019 初始化环 / E1014-E1015 命名
    /// 参数 / E3023 等，ir_gen 是这些码的唯一产生点），不中止、不消费
    /// IR。standalone check 旧路径（pipeline 全链）本就跑 IR 生成；项目
    /// check 从此补上这一层（02 不一致表第 11 行缺口）。typecheck 失败
    /// 的单元无 type_result，自然跳过（与 run 路径门控一致）。
    ///
    /// Script/Bin 分形：ir_gen 的 E3023（顶层可执行语句拒绝）以
    /// `module_key.is_none()` 为开关（ir_gen.rs:1660）。无项目根的
    /// standalone 程序是 Script 语义（顶层语句即程序主体，T4）——逐单元
    /// 传 None（与 pipeline 单文件同参）；有 manifest 的项目传单元键
    ///（Bin 语义，与 compile 路径一致）。跨文件上下文（类型/全局/槽位
    /// 基址）两种形态同样播种。
    pub(super) fn ir_generation_check(
        &self,
        state: &mut State,
    ) {
        let rootless = orchestrator::find_project_root(&state.entry_path).is_none();
        let State {
            program,
            asts,
            units,
            registry,
            global_layout,
            slot_bases,
            ..
        } = state;
        let (Some(registry), Some(global_layout)) = (registry.as_ref(), global_layout.as_ref())
        else {
            return; // 上游已失败（防御）
        };
        // 跨文件上下文：所有文件的 AST 引用视图（与 orchestrated 同构）
        let all_ast_refs: Vec<&Module> = asts.iter().filter_map(|a| a.as_ref()).collect();
        for (i, us) in units.iter_mut().enumerate() {
            let (Some(ast), Some(type_result)) = (&asts[i], &us.type_result) else {
                continue;
            };
            // 与 generate_ir_with_context 同构，仅 module_key 按 Script/Bin
            // 分形（见臂注释）
            let mut generator = AstToIrGenerator::new_with_type_result(
                type_result,
                registry.clone(),
                if rootless {
                    None
                } else {
                    Some(program.units[i].key.clone())
                },
            );
            generator.seed_cross_file_types(&all_ast_refs);
            generator.seed_cross_file_globals(global_layout);
            generator.set_global_slot_base(slot_bases[i]);
            if let Err(diags) = generator.generate_module_ir(ast) {
                // IR 级错误进逐文件收集通道（不标阶段失败、不中止）
                us.check_diagnostics.extend(diags);
            }
        }
    }

    /// Embedded 形态（4.2.4）：`generate_ir_with_context` 以空调用面
    ///（无跨文件 AST 上下文、无全局布局、槽位基址 0）+ 调用方注入的
    /// 共享 registry（#94：与入口 IR 生成共用 SymbolTable，DefId 一致）。
    /// 嵌入 std 模块是独立编译单元——无 Discovery/GlobalSlotAlloc/
    /// Linking 阶段；错误归入 `OrchestratorError::Compile`（现状契约）。
    pub(super) fn ir_generation_embedded(
        &self,
        state: &mut State,
    ) {
        debug_assert_eq!(state.program.units.len(), 1, "Embedded 程序恒为 1 单元");
        let State {
            program,
            asts,
            units,
            registry,
            failure,
            failed_stages,
            ..
        } = state;
        let Some(registry) = registry.as_ref() else {
            return; // 调用方未注入共享 registry——构造 bug，入口以 Io 兜底
        };
        let (Some(ast), Some(type_result)) = (&asts[0], &units[0].type_result) else {
            return; // 上游已失败（FailFast 下 run 循环已中止，防御）
        };
        match crate::middle::generate_ir_with_context(
            ast,
            type_result,
            &[],
            &[],
            0,
            registry,
            &program.units[0].key,
        ) {
            Ok(ir) => units[0].ir = Some(ir),
            Err(diags) => {
                let message = diags
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                *failure = Some(OrchestratorError::Compile {
                    path: program.units[0].path.display().to_string(),
                    message,
                    diagnostics: diags,
                });
                failed_stages.push(Stage::IrGeneration);
            }
        }
    }

    /// SingleFile/WasmPlayground 形态：移植自 pipeline.rs:205-227 / 302-341
    /// + `merge_embedded_std_ir`（#94 共享 registry）。IR 错误归入
    ///   `PipelineError::TypeCheck`（pipeline 现状映射）。
    pub(super) fn ir_generation_single_file(
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
    pub(super) fn ir_generation_orchestrated(
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
    pub(super) fn monomorphization(
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
    pub(super) fn vendor_consistency(
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
    pub(super) fn discovery(
        &self,
        state: &mut State,
    ) {
        // LSP 形态（4.2.3）：占位入口单元携带的是内存缓冲区源码（脏
        // 缓冲区）——重建单元集后，入口单元的源码以缓冲区为准（覆盖
        // 磁盘上的陈旧/损坏内容；磁盘旧版的 parse 错误因此不会误杀
        // 正在编辑的缓冲区）
        let lsp_buffer = if matches!(state.program.kind, ProgramKind::Lsp) {
            Some(state.program.units[0].source.clone())
        } else {
            None
        };
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
                if let Some(source) = lsp_buffer {
                    if let Some(unit) = state
                        .program
                        .units
                        .iter_mut()
                        .find(|u| u.path == state.entry_path)
                    {
                        unit.source = source;
                    }
                    // 入口单元不在发现集（病态情形）→ 丢弃覆盖，防御
                }
            }
            Err(e) => state.fail_with(Stage::Discovery, e),
        }
    }

    /// `Stage::Registry` 臂（Project）：消费全部单元的 AST 产物做签名
    /// 收集（C3 拓扑修正——本臂不再自行 parse），构建共享 ModuleRegistry
    /// 与跨文件方法绑定（RFC-029）。
    pub(super) fn registry(
        &self,
        state: &mut State,
    ) {
        let mut registry = ModuleRegistry::with_std();
        for (unit, ast) in state.program.units.iter().zip(state.asts.iter()) {
            let Some(ast) = ast else {
                continue; // 上游失败（FailFast 下 run 循环已中止，防御）
            };
            // 嵌入 std 模块（虚拟路径 `<std/...>`）跳过注册：`with_std()`
            // 已按「native + yx 表面合并」语义注册过它们（register_std_
            // modules，registry.rs:305-330）；此处再注册会把 native 半面
            //（如 std.result.is_err）整体顶掉——4.2.5 施工发现的潜伏
            // 缺陷：std.test 与 std.result 同现的 check 曾误报 E1043
            if unit.path.to_str().is_some_and(|p| p.starts_with('<')) {
                continue;
            }
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
    pub(super) fn role_classification(
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
    pub(super) fn global_slot_alloc(
        &self,
        state: &mut State,
    ) {
        let (layout, bases) = allocate_global_slots(&state.program.units, &state.asts);
        state.global_layout = Some(layout);
        state.slot_bases = bases;
    }

    /// `Stage::Linking` 臂（Project）：IR 层链接（限定名共存）+ 入口
    /// 语义校验（#388 缺 main / #357 带参 main，自 compile_project 移入）。
    pub(super) fn linking(
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
