//! 阶段臂 —— `impl Driver` 的第二部分（02 §目标设计 3/4）。
//!
//! 12 个 `Stage` 变体的执行体全部在此（4.2.9 自 mod.rs 拆出——1766 行
//! 超仓库 1500 行拆分阈值）。臂只读 `Program`、读写 `State`；跳过
//! 由 run 循环按拓扑判定，臂自身不决定跳过。跨形态的臂按 ProgramKind
//! 分形（形状选择即形态契约，各形注释标注现状出处）。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::frontend::core::lexer;
use crate::frontend::core::parser::{self, Module};
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::environment::MethodOverloadCandidates;
use crate::frontend::core::typecheck::TypeCheckResult;
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

use super::obligations::{Obligation, IR_GENERATION_DELIVERED};
use super::helpers::{
    aggregate_instantiation_requests, allocate_global_slots, ast_has_main,
    collect_compilation_refs, collect_package_refs, is_bin_role, link_module_irs,
    merge_embedded_std_ir, resolved_method_marks, role_context,
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
    ///
    /// 4.6.1：checker 以**单元文件路径**为模块身份（原硬编码 `"main"`，
    /// 经自由函数 `check_module_inner`）——SemanticDB 的文件身份由此
    /// 可得；CollectAll（LSP 兜底）无论诊断成败都把语义数据汇入
    /// `State.semantic_db`（染色/跳转在打字中间态不消失）。
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
            semantic_db,
            ..
        } = state;
        for (i, us) in units.iter_mut().enumerate() {
            let Some(ast) = &asts[i] else {
                continue; // CollectAll 下 parse 失败的单元（4.1.3 不可达）
            };
            // checker 入口由 Aggregation 驱动（4.2.7 语义）：FailFast
            // 首错即返；CollectAll（LSP 兜底）收集模式
            let mut checker = TypeChecker::new(&program.units[i].path.display().to_string());
            let mut type_result = match program.aggregation {
                Aggregation::FailFast => checker.check_module(ast),
                Aggregation::CollectAll => checker.check_module_collect_all(ast),
            };
            // 语义数据面（4.6.1）：仅 CollectAll 形态产出（LSP 消费）；
            // 在诊断判定**之前**取走，错误文件同样保留染色/跳转数据
            if matches!(program.aggregation, Aggregation::CollectAll) {
                semantic_db.upsert_from(std::mem::take(&mut type_result.semantic_db));
            }
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
            semantic_db,
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
            // checker 身份按形态区分：Embedded 用 "<embedded>"；其余以
            // **单元文件路径**为模块身份（原统一 "<module>"）——SemanticDB
            // 的文件身份由此可得（4.6.1；该值只消费于语义数据面与
            // TypeCheckResult.module_name 元信息，不进诊断）
            let unit_name = if is_embedded {
                "<embedded>".to_string()
            } else {
                program.units[i].path.display().to_string()
            };
            let mut checker = TypeChecker::new(&unit_name);
            checker.env().module_registry = registry.clone();
            if let Some(method_bindings) = method_bindings {
                checker.env().method_bindings = method_bindings.clone();
            }
            // RFC-029f 改进 4 / WBS 4.9.7（#464）：跨单元方法调用的**重载候选**注入。
            // 注入面 = 本单元裸类型名面**逐对**（名字 ← 它自己的提供模块）取到的候选组：
            // 只并入「键的提供模块 == 该类型名前缀在本单元 scope 里的提供模块」的候选组
            // （名字升序 ⇒ 合并顺序确定）。跨对组合（名字集合 × 模块集合）会把别的模块
            // 定义的同名类型候选塞进来 ⇒ 调用点 2 候选 ⇒ E1107 误报（修复轮实测：
            // `use a.{Temp}` + `use b.{Other}`、a 亦定义 Other、b 亦定义 Temp 时
            // 2×E1107；逐对过滤后 check/run 均退出 0）。判据与理由见本文件的
            // `visible_method_overloads`；入口单元自己的候选不在此列——它们由本单元
            // 签名收集现场登记（自有优先）。
            checker
                .env()
                .inject_method_overloads(visible_method_overloads(
                    ast,
                    &program.units[i].key,
                    registry,
                ));
            // RFC-014：vendor 根注入（缺依赖包的 E5001 追加 install 提示）
            if let Some(root) = &vendor_root {
                checker.set_vendor_root(root.clone());
            }
            let mut result = match program.aggregation {
                Aggregation::FailFast => checker.check_module(ast),
                Aggregation::CollectAll => checker.check_module_collect_all(ast),
            };
            // 语义数据面（4.6.1）：Lsp 形态的目标文件无论诊断成败都汇入
            // ——染色/跳转在打字中间态不消失；Check 无消费者不汇入
            if is_lsp {
                semantic_db.upsert_from(std::mem::take(&mut result.semantic_db));
            }
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
            // 3.4.6：typecheck 警告暂存，交 DeadCodeAnalysis 臂按项目内门控外发
            //（与单文件臂 arms.rs:277 同一落点、同一消费端）。
            us.typecheck_warnings = std::mem::take(&mut result.warnings);
            us.type_result = Some(result);
        }
        // 外部定义面（4.6.1）：Lsp 形态把**非目标文件**的顶层导出定义
        // （函数/常量，带定义文件与名源 span）注入语义数据面——跨文件
        // 跳转在定义文件尚未被编辑器打开时也能命中。定义面来自 Registry
        // 臂的签名收集（与诊断同一次编译的上游），不要求那些文件参与
        // typecheck（「Lsp 只检目标文件」的性能契约不动）。std/嵌入模块
        // 无定义文件（definition_file 为 None），天然排除。
        if is_lsp {
            use crate::frontend::core::typecheck::semantic_db::{DefId, DefinitionInfo, DefinitionKind};
            use crate::frontend::module::ExportKind;
            let mut externals = Vec::new();
            for unit in &program.units {
                if unit.path == *entry_path {
                    continue; // 目标文件自身：全量语义信息已在 typecheck 汇入
                }
                let Some(module) = registry.get(&unit.key) else {
                    continue; // 嵌入 std 等未注册单元
                };
                let Some(file) = &module.definition_file else {
                    continue;
                };
                let file_path = file.display().to_string();
                for export in module.exports.values() {
                    let Some(span) = export.definition_span else {
                        continue; // std/native 导出无定义位置
                    };
                    let kind = match export.kind {
                        ExportKind::Function => DefinitionKind::Function,
                        ExportKind::Constant => DefinitionKind::Variable,
                        // 类型/子模块导出不注入：typecheck 侧类型注册不产生
                        // 定义事件，两侧口径保持一致的空缺（4.6.1）
                        ExportKind::Type | ExportKind::SubModule => continue,
                    };
                    externals.push(DefinitionInfo {
                        def_id: DefId {
                            file_path: file_path.clone(),
                            span,
                        },
                        name: export.name.clone(),
                        kind,
                        span,
                        file_path: file_path.clone(),
                        type_info: export.mono_type.as_ref().map(|t| t.to_string()),
                        signature: None,
                    });
                }
            }
            semantic_db.set_external_definitions(externals);
        }
    }

    /// `Stage::DeadCodeAnalysis` 臂（Project 作用域，臂内逐单元）。
    ///
    /// 单文件形态（`SingleFile`/`WasmPlayground`/`Embedded`）移植自
    /// pipeline.rs:275-278 / 288-299。门控 `dead_code.enabled && !has_errors`
    /// 原样保留；开关关闭时连暂存的 typecheck 警告一并丢弃（现状耦合），
    /// 并记录 ConfigDisabled 跳过。警告顺序 = 死代码警告在前、typecheck
    /// 警告在后（与 pipeline 一致）。
    ///
    /// **编排形态（Check / Lsp / MultiFile）走同一份实现**
    /// （`dead_code_analysis_check`）：角色感知 + 引用池 + W1006 遮蔽 + W1003
    /// 未使用导入是一套判据，形态只决定 sink 落在哪条通道（见该函数）。
    /// 3.4.6 之前 MultiFile 没有本阶段——警告面在 compile 路径整块缺失。
    pub(super) fn dead_code_analysis(
        &self,
        state: &mut State,
    ) {
        if !state.uses_pipeline_channel() {
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

    /// 编排形态（Check / Lsp / MultiFile）的逐文件警告装配——**唯一实现**：
    /// 角色感知 + 引用池是一套判据，形态只决定 sink 落在哪条通道。
    ///
    /// 与单文件形态的三处差异（`SingleFile` 保留 pipeline 的旧语义）：
    /// - 无 `config.dead_code.enabled` 门控（编排形态不读该开关）；
    /// - 不按 `typecheck_failed` 跳过（死代码只看 AST，与类型错误并存）；
    /// - 角色感知分析器（RFC-029f：Test 不参与；Bin/Internal 按
    ///   `set_exempt_pub`、Lib 按 `set_cross_file_refs` 接**编译集**引用池，
    ///   Script 不接池——4.9.3），W1006 遮蔽事件 / W1003 未使用导入 /
    ///   死代码警告仅对 **本项目文件**外发（vendor/嵌入 std 的内部警告
    ///   不混入消费方）。
    ///
    /// sink 分流（3.4.6）：Check / Lsp / MultiFile 一律进 `us.check_diagnostics`
    /// ——`Diagnostic` 不携带文件身份（`util/diagnostic/error.rs` 的 span 无
    /// file 字段），逐文件载体是唯一能把 W 码渲染到定义文件的外发形态。
    /// 消费端按形态取：Check/Lsp 经 `DriverOutcome.check_diagnostics`，
    /// MultiFile 由 `compile_project` 以同一形态外发。
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
        // RFC-029f 改进 1+2（WBS 4.9.1/4.9.2 / #334）：包内引用池的**根**按判定
        // 三层解析（roles.rs 文件头：manifest 显式声明 > 入口可达性推断 >
        // RFC-036 测试约定）——manifest 声明面的**包目标**优先：
        //   roots = ExplicitSurfaces.bins ∪ libs
        //           （`[run].main` / `[[bin]]` / `[lib]` / `[exports]` 指向的文件）
        // 池 = 各 root `discover_with_used` 编译集的引用并集（身份 = (模块, 名字)：
        // 装配期已剔除每个模块自有的名字，池里留下的每个名字都表示「**别的
        // 模块**在消费它」）；**无声明面时退化为被检文件自身的编译集**。
        // 两条路径都不为找入口递归扫目录——这正是 RFC-029f 缺陷 1 的修法
        // （孤立文件、tests/ 下的测试文件不再进池）。
        //
        // 旧口径的注释理由（「聚合全项目而非入口可达集：单文件 check 时引用方
        // 可能不在发现集，按可达集聚合会产生顺序敏感的误报」）逐句处置：
        // - 成立的一半（保留）：引用方只要不在**同一个编译面**里就不该豁免，
        //   故池取**包目标**的编译面而非被检文件一个文件的发现集——
        //   `check internal.yx` 时包入口 main.yx 的消费照样救活 internal 的定义
        //   （tests/integration/cli_e2e.rs 的 Internal 用例钉的就是这条，#334）；
        // - 被取代的一半：**「全项目」不再等于「项目根目录下全部 .yx」**——
        //   目录扫描把从未参与编译的文件也算成消费者（缺陷 1），改由「包目标的
        //   编译集」表达「全项目」，扫目录这条路整体作废。
        // vendor/嵌入 std 不入池（外部包视角，保留原口径）。
        //
        // 轮 2（WBS 4.9.4/4.9.5）在本块之上把**消费判据**收紧：名字只记到
        // import 解析出的那个模块（改进 4：被 import 后真实使用才算消费），
        // 且 Test 模块的引用不贡献（改进 5）——见 helpers.rs 的 `RefPool`。
        let (surfaces, test_rules) = role_context(project_root_canon.as_deref());
        let mut package_roots: Vec<PathBuf> = surfaces
            .as_ref()
            .map(|s| s.bins.iter().chain(s.libs.iter()).cloned().collect())
            .unwrap_or_default();
        package_roots.sort();
        package_roots.dedup();
        // 退化路径要本单元的角色（Test 不贡献消费面）；roles 在 RoleClassification
        // 臂已产，这里只读不写
        let unit_roles: Vec<Option<FileRole>> = units.iter().map(|us| us.role).collect();
        // WBS 4.9.6 / #450 + 4.9.7 / #464 逐候选方法标记：从 D20 决议产物（纯读）解析，
        // 按**定义单元**归因（条目记在记录方单元；定义单元由记录方的类型名面反查，
        // 见 helpers.rs::typed_owners）
        let resolved_marks = {
            let results: Vec<Option<&TypeCheckResult>> =
                units.iter().map(|us| us.type_result.as_ref()).collect();
            resolved_method_marks(&program.units, asts.as_slice(), &results)
        };
        let pool = if package_roots.is_empty() {
            // 无声明面（含无 manifest 的 standalone 单文件）：退化为被检文件
            // 自身的编译集
            collect_compilation_refs(&program.units, asts.as_slice(), &unit_roles)
        } else {
            match collect_package_refs(
                &package_roots,
                project_root_canon.as_deref(),
                surfaces.as_ref(),
                test_rules.as_ref(),
            ) {
                Some(pool) => pool,
                // 声明面存在却一个 root 都跑不通（文件缺失 / 语法全败）——
                // 宁漏勿误：退化为被检文件自身的编译集，不用空池制造误报
                None => collect_compilation_refs(&program.units, asts.as_slice(), &unit_roles),
            }
        };
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
            // 本单元可消费的池 = **从本模块 import 出去且被真实使用**的名字
            // （改进 4 的 (module, name) 归属）+ 字段/方法位残差（AST 层归因不到
            // 模块，名字维度保守计入；方法决议归 #450 另案）
            let unit_key = program.units[i].key.clone();
            let consumed_names = || -> HashSet<String> {
                let mut names: HashSet<String> = pool
                    .consumed_from
                    .get(&unit_key)
                    .cloned()
                    .unwrap_or_default();
                names.extend(pool.field_names.iter().cloned());
                names
            };
            let mut analyzer = DeadCodeAnalyzer::new();
            // 逐候选方法标记（同名重载各自判定：未被决议命中的兄弟重载仍报 W1005）——
            // 只喂**本单元定义上**的标记；另喂「存在决议条目的名字组」，
            // 让池的名字级豁免对这类组让位（WBS 4.9.7 / #464）
            analyzer.set_resolved_methods(
                resolved_marks
                    .by_unit
                    .get(&unit_key)
                    .cloned()
                    .unwrap_or_default(),
            );
            analyzer.set_resolved_groups(resolved_marks.groups.clone());
            match us.role {
                Some(FileRole::Bin | FileRole::Internal) => {
                    analyzer.set_exempt_pub(consumed_names());
                }
                Some(FileRole::Lib) => {
                    analyzer.set_cross_file_refs(consumed_names());
                }
                // Script：不接池（RFC-029f 改进 3 / WBS 4.9.3）——单文件语义
                // 只按定义-使用图判定，项目内其它文件的引用不豁免它；
                // Test 上面已跳过；None 防御不可达（RoleClassification 恒
                // 产角色）——两者都不给池，退化为纯文件内可达判定
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
            let UnitState {
                type_result,
                typecheck_failed,
                check_diagnostics,
                delivered,
                ..
            } = us;
            let (Some(ast), Some(type_result)) = (&asts[i], type_result.as_ref()) else {
                continue;
            };
            if *typecheck_failed || type_result.proof_calls().is_empty() {
                continue;
            }
            any_obligation = true;
            // 唯一交付点（ProofExecution）：本单元证明义务交给消费者。
            // 交付点是 execute_proof_calls 调用本身——proof_calls() 是
            // TypeCheckResult 对该义务的唯一对外读取路径（types.rs:84）。
            delivered.deliver(Obligation::ProofCalls);
            let stage_errors = crate::frontend::proof_execution::execute_proof_calls(
                type_result.proof_calls(),
                ast,
                type_result,
            );
            if !stage_errors.is_empty() {
                // Check/Lsp 通道（4.2.2/4.2.3）：proof 失败是逐文件收集
                // 诊断——不标阶段失败、不中止（CollectAll 语义）
                if matches!(program.kind, ProgramKind::Check | ProgramKind::Lsp) {
                    check_diagnostics.extend(stage_errors);
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
    /// 参数 / E3023 等，ir_gen 是这些码的唯一产生点），不中止。standalone
    /// check 旧路径（pipeline 全链）本就跑 IR 生成；项目 check 从此补上
    /// 这一层（02 不一致表第 11 行缺口）。typecheck 失败的单元无
    /// type_result，自然跳过（与 run 路径门控一致）。
    ///
    /// 3.4.8 ③：生成的 IR 存入 `UnitState.ir`——Linking 的纯合并与
    /// Monomorphization 的检查形态都要消费它。IR 只驻内存（不进
    /// DriverOutcome、不写任何产物），check 的对外诊断集不因此改变。
    ///
    /// 命名与语义（3.4.8 ③ 回归修复）：本形态是**多单元**程序——逐单元 IR 一律
    /// 按单元键限定名（`Some(unit.key)`），否则合并时跨单元同名函数（用户模块之间、
    /// 或用户模块与 `<std/list>` 之间）会被 `link_module_irs` 判成「模块限定名
    /// 冲突」，把 standalone 程序整体打红（文档示例门禁 10 项实证）。
    /// 无项目根的 standalone 程序仍保持 **Script 语义**（顶层语句即程序主体，T4，
    /// E3023 不打开）——用 `with_script_mode(rootless)` 显式表达，与限定名键解耦。
    /// 跨文件上下文（类型/全局/槽位基址）两种形态同样播种。
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
            let UnitState {
                type_result,
                ir: ir_slot,
                check_diagnostics,
                delivered,
                ..
            } = us;
            let (Some(ast), Some(type_result)) = (&asts[i], type_result.as_ref()) else {
                continue;
            };
            // 唯一交付点（IrGeneration）：整份 TypeCheckResult 交给 IR 生成器
            // ——new_with_type_result 是 11 条 IrGeneration 义务的唯一交付口
            //（构造期读 8 条 + 生成期按 span 回查其余，见 obligations.rs 清单）。
            delivered.deliver_all(IR_GENERATION_DELIVERED);
            // 与 generate_ir_with_context 同构，但 Script/Bin 语义与限定名键
            // 解耦（见臂注释）：限定名恒按单元键，Script 语义由 with_script_mode 给
            let mut generator = AstToIrGenerator::new_with_type_result(
                type_result,
                registry.clone(),
                Some(program.units[i].key.clone()),
            )
            .with_script_mode(rootless);
            generator.seed_cross_file_types(&all_ast_refs);
            generator.seed_cross_file_globals(global_layout);
            generator.set_global_slot_base(slot_bases[i]);
            match generator.generate_module_ir(ast) {
                // 3.4.8 ③：Check 形态也留存单元 IR（仅内存）——它是
                // Linking 纯合并与 mono 检查形态的输入；不外发、不写盘。
                Ok(ir) => *ir_slot = Some(ir),
                // IR 级错误进逐文件收集通道（不标阶段失败、不中止）
                Err(diags) => check_diagnostics.extend(diags),
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
        let UnitState {
            type_result,
            ir: ir_slot,
            delivered,
            ..
        } = &mut units[0];
        let (Some(ast), Some(type_result)) = (&asts[0], type_result.as_ref()) else {
            return; // 上游已失败（FailFast 下 run 循环已中止，防御）
        };
        // 唯一交付点（IrGeneration，Embedded 形态）
        delivered.deliver_all(IR_GENERATION_DELIVERED);
        match crate::middle::generate_ir_with_context(
            ast,
            type_result,
            &[],
            &[],
            0,
            registry,
            &program.units[0].key,
            false, // 嵌入模块：非 Script 语义（保持现状）
        ) {
            Ok(ir) => *ir_slot = Some(ir),
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
        let UnitState {
            type_result,
            ir: ir_slot,
            delivered,
            ..
        } = &mut units[0];
        let (Some(ast), Some(type_result)) = (&asts[0], type_result.as_ref()) else {
            return Ok(()); // 上游已失败（CollectAll；4.1.3 不可达）
        };
        // 唯一交付点（IrGeneration，SingleFile/WasmPlayground 形态）
        delivered.deliver_all(IR_GENERATION_DELIVERED);
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
        *ir_slot = Some(ir);
        Ok(())
    }

    /// 多文件形态（4.2.1）：移植自 compile_project Phase 2——预注册跨文件
    /// 上下文的 `generate_ir_with_context`，错误归入
    /// `OrchestratorError::Compile` 并首错即返。
    ///
    /// 4.10.3：rootless（standalone，无项目根）时 Script 语义放行顶层可执行
    /// 语句（E3023 关闭）——run 半边与 3.4.8 ③ 的 check 半边同一开关：限定名
    /// 恒按单元键，Script/Bin 语义由 script_mode 独立承载（与限定名键解耦）。
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
        // 4.10.3：Script/Bin 语义开关（见臂注释）
        let rootless = orchestrator::find_project_root(&state.entry_path).is_none();
        // 跨文件上下文：所有文件的 AST 引用视图（结构体布局/全局名解析用）
        let all_ast_refs: Vec<&Module> = asts.iter().filter_map(|a| a.as_ref()).collect();
        for (i, us) in units.iter_mut().enumerate() {
            let UnitState {
                type_result,
                ir: ir_slot,
                delivered,
                ..
            } = us;
            let (Some(ast), Some(type_result)) = (&asts[i], type_result.as_ref()) else {
                continue;
            };
            // 唯一交付点（IrGeneration，MultiFile 形态）
            delivered.deliver_all(IR_GENERATION_DELIVERED);
            let ir = match crate::middle::generate_ir_with_context(
                ast,
                type_result,
                &all_ast_refs,
                global_layout,
                slot_bases[i],
                registry,
                &program.units[i].key,
                rootless,
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
            *ir_slot = Some(ir);
        }
    }

    /// `Stage::Monomorphization` 臂（Project 作用域）：消费 IR 产物与
    /// 聚合的实例化请求。双形态（3.4.8 ②，2026-10-10）。
    ///
    /// 移植自 pipeline.rs:329-338。两道门控原样保留：配置关闭 →
    /// ConfigDisabled；请求为空 → NoObligations。
    ///
    /// 三种形态的 IR 与请求来源不同：
    /// - SingleFile / WasmPlayground：唯一单元的 IR 与请求（阶段表在
    ///   `Stage::ALL` 拓扑下仍把 mono 排在单元 IR 之后——本形态无 Linking，
    ///   本次次序调整对它零影响）；
    /// - MultiFile / Check：**Linking 产物**（merged IR，全程序唯一完整函数
    ///   表）与聚合后的全程序请求表；`containing_fn` 在此限定化为 merged IR
    ///   的函数表键（见 `aggregate_instantiation_requests`）。Check 用它做
    ///   检查（3.4.8 ③——同 IrGeneration 的 Check 形态），MultiFile 用它做
    ///   产物。
    ///
    /// 错误通道按形态分流：单文件走 pipeline 契约
    /// （`PipelineError::TypeCheck`，mono 失败映射为类型检查错误）；多文件走
    /// orchestrator 契约（`OrchestratorError::TypeCheck`，与 Linking 的入口
    /// 校验同形）；Check 走逐文件收集通道（3.4.8 ③，检查形态）——mono 失败
    /// 不该丢掉其余文件已收集的诊断。
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
        // 形态判据用 ProgramKind（数据流形状），不用单元数——单文件项目
        // 在 MultiFile 形态下也是 1 单元，但它的 IR 在 Linking 的 merged_ir
        // 里（单元 IR 已被链接取走），按单元数分派会走进单文件分支空转。
        if !state.uses_pipeline_channel() {
            return self.monomorphization_multi_file(state, max_depth, max_instantiations);
        }
        let State {
            units,
            errors,
            skipped,
            failed_stages,
            ..
        } = state;
        let UnitState {
            type_result,
            ir,
            delivered,
            ..
        } = &mut units[0];
        let (Some(type_result), Some(ir)) = (type_result.as_ref(), ir.as_mut()) else {
            return Ok(()); // 上游已失败（CollectAll；4.1.3 不可达）
        };
        if type_result.instantiation_requests.is_empty() {
            skipped.push(SkippedStage {
                stage: Stage::Monomorphization,
                reason: SkipReason::NoObligations,
            });
            return Ok(());
        }
        // 唯一交付点（Monomorphization，单文件/WasmPlayground 形态）：
        // 本单元的实例化请求表交给单态化器
        delivered.deliver(Obligation::InstantiationRequests);
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

    /// 多文件形态的 mono（3.4.8 ②）：消费 **Linking 产物**（merged IR）。
    ///
    /// 请求表 = 各单元 typecheck 请求的并集，`containing_fn` 在聚合时
    /// 限定化（裸名 → `{module_key}.{bare}`），使 deferred 桶键与 merged IR
    /// 的函数表键同源。两道门控与单文件同形：关闭 → ConfigDisabled；
    /// 聚合后请求为空 → NoObligations（**排空**由聚合表空集判定，不再由
    /// 「单元数 != 1」硬拒）。
    fn monomorphization_multi_file(
        &self,
        state: &mut State,
        max_depth: usize,
        max_instantiations: usize,
    ) -> Result<(), DriverError> {
        let requests = std::mem::take(&mut state.aggregated_requests);

        if requests.is_empty() {
            state.record_skipped(Stage::Monomorphization, SkipReason::NoObligations);
            return Ok(());
        }
        let Some(merged) = state.merged_ir.take() else {
            // 上游 Linking 未产出 merged IR（拓扑应在失败时跳过本阶段；防御）
            state.record_skipped(Stage::Monomorphization, SkipReason::NoObligations);
            return Ok(());
        };
        let mut mono =
            Monomorphizer::with_max_depth(max_depth).with_max_instantiations(max_instantiations);
        match mono.monomorphize(&merged, &requests) {
            Ok(mono_ir) => state.merged_ir = Some(mono_ir),
            Err(diag) => {
                // Check（3.4.8 ③）：mono 失败是**检查诊断**——进逐文件收集
                // 通道（同 IrGeneration 的 Check 形态），不因它丢掉其余文件
                // 已收的诊断；标记阶段失败使下游按拓扑跳过（本阶段之后无
                // 下游，记录口径与 run 路径一致）。
                //
                // 归属入口文件：与 run 路径同口径（`OrchestratorError::
                // TypeCheck{path}` 恒为入口）——mono 诊断是程序级事实，失败
                // 请求的 span 不携带文件身份，无从逐文件归属。
                if matches!(state.program.kind, ProgramKind::Check) {
                    let entry_index = state
                        .program
                        .units
                        .iter()
                        .position(|u| u.path == state.entry_path)
                        // 防御：发现成功必含入口单元；不可达时退回首单元
                        //（归属偏差好过静默丢诊断）
                        .unwrap_or(0);
                    state.units[entry_index].check_diagnostics.push(diag);
                    state.failed_stages.push(Stage::Monomorphization);
                    return Ok(());
                }
                let message = diag.to_string();
                state.fail_with(
                    Stage::Monomorphization,
                    OrchestratorError::TypeCheck {
                        path: state.entry_path.display().to_string(),
                        message,
                        diagnostics: vec![diag],
                    },
                );
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
            // 4.6.1：模块 → 定义文件映射随注册表携带——typecheck 的导入处理
            // 据此把跨文件引用的 resolves_to 指向真实定义位置（LSP 跳转）
            info.definition_file = Some(unit.path.clone());
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
    /// 路径的同类校验在 Linking 臂——两边对称：握着判据的臂负责校验）。
    ///
    /// 3.4.8 ③（2026-10-10 裁决）：CHECK 自本步起也**有** Linking 阶段，
    /// 但它只做纯合并（不跑 Linking 的入口校验，见该臂注释）——判据仍归
    /// 本臂（声明面 = surfaces.bins），否则同一缺失在 check 里双报。
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
        // 不编译，同样降级。
        //
        // canonical 口径必须与下面 `file_canon` 一致：`roles::classify` 的
        // TestRules / 声明面都是 canonical 比较（roles.rs 的 `canonical()`），
        // 根用原始路径时 `file_canon.starts_with(root.join("tests"))` 在
        // Windows 的 `\\?\` 前缀下静默为 false——tests/ 下的文件被误判成
        // Internal（CLI 恰好因入口先 canonicalize 而掩盖了这条）
        let project_root = orchestrator::find_project_root(entry_path);
        let project_root_canon = project_root
            .as_ref()
            .map(|root| root.canonicalize().unwrap_or_else(|_| root.clone()));
        let (surfaces, test_rules) = role_context(project_root_canon.as_deref());
        for (i, us) in units.iter_mut().enumerate() {
            let Some(ast) = &asts[i] else {
                continue; // 上游失败（FailFast 下 run 循环已中止，防御）
            };
            let path = &program.units[i].path;
            let file_canon = path.canonicalize().unwrap_or_else(|_| path.clone());
            let role = roles::classify(
                &file_canon,
                project_root_canon.as_deref(),
                surfaces.as_ref(),
                test_rules.as_ref(),
                used_by.contains(&file_canon),
                ast_has_main(ast),
            );
            // 入口存在性（#388 定案 check 半边判据）：声明面入口必须有
            // main 绑定（值/函数皆可）；声明面之外的文件不查——Lib/
            // Internal/Test 不要求 main（被 check ≠ 被当程序跑）。
            //
            // 3.4.6：本臂自 MultiFile 起也参与 compile 路径，故注入必须按形态
            // 门控到 Check/Lsp——compile 路径的同类校验在 Linking 臂
            //（`bin_missing_main`，判据同 surfaces 语义下的入口存在性），
            // 不门控即同一缺失双报（与 3.4.8 ③ 把入口校验留在 Linking 的
            // 「纯合并」决定同一条理由）。
            let check_channel = matches!(program.kind, ProgramKind::Check | ProgramKind::Lsp);
            if check_channel
                && surfaces
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
    ///
    /// **Check 形态（3.4.8 ③，2026-10-10 用户裁决）只做纯合并**：先把全程序
    /// 实例化请求聚合出来（②），再把单元 IR 合并供 mono 的检查形态消费，
    /// **不跑入口校验**——声明面 bin 缺 main 在 check 里已由 RoleClassification
    /// 报 E3020，本臂再查一次就是同一缺失双报（02 §3 注记 #14 的「纯合并」）。
    /// 合并冲突（模块限定名撞名）是项目结构错误，与 compile 路径同口径走
    /// 故障通道：它没有可归属的结构化诊断（判据见 state.rs `into_outcome`
    /// 的注释——已收的逐文件诊断不会因此丢失）。
    pub(super) fn linking(
        &self,
        state: &mut State,
    ) {
        // 3.4.8 ②：聚合全程序实例化请求——必须在 take 各单元 IR **之前**
        // （限定化 `containing_fn` 要逐单元的函数名集合），产物交
        // Monomorphization 臂消费。
        //
        // t10：请求名要对齐成 IR 调用指令里的名字，故这里连同单元 AST 与 registry
        // 一起交给聚合函数——它用 ir_gen 的 `import_aliases_from`（`qualify_names`
        // 改写调用名用的同一实现）现场取别名表，驱动侧不另建表。
        {
            let unit_irs: Vec<Option<ModuleIR>> =
                state.units.iter().map(|us| us.ir.clone()).collect();
            let type_results: Vec<Option<&crate::frontend::core::typecheck::TypeCheckResult>> =
                state
                    .units
                    .iter()
                    .map(|us| us.type_result.as_ref())
                    .collect();
            let registry = state.registry.clone();
            state.aggregated_requests = aggregate_instantiation_requests(
                &state.program.units,
                &unit_irs,
                &type_results,
                &state.asts,
                registry.as_ref().expect("Registry 阶段必产注册表"),
            );
            // 唯一交付点（Monomorphization，MultiFile/Check 形态）：本臂的
            // 全程序聚合**读取了各单元的请求表**，聚合产物交 mono 臂消费
            // ——多文件形态的请求表在这里离开产出阶段，故标记落在此处。
            for us in state.units.iter_mut() {
                if us.type_result.is_some() {
                    us.delivered.deliver(Obligation::InstantiationRequests);
                }
            }
        }
        let mut irs: Vec<(String, ModuleIR)> = Vec::new();
        for (i, us) in state.units.iter_mut().enumerate() {
            if let Some(ir) = us.ir.take() {
                irs.push((state.program.units[i].path.display().to_string(), ir));
            }
        }
        let mut merged = match link_module_irs(irs, &state.entry_key) {
            Ok(merged) => merged,
            Err(e) => {
                state.fail_with(Stage::Linking, e);
                return;
            }
        };
        // #413 / 4.10.3：entry_function 按 is_bin_role 门控——Script（无项目根）
        // 无入口概念（顶层语句即程序主体），自动调 main 会与显式 main() 双跑
        //（#356）。link_module_irs 只看绑定存在性、不看角色；角色门控归本臂
        //（与下方入口校验同一判据 is_bin_role，单点）。
        if !is_bin_role(&state.entry_path) {
            merged.entry_function = None;
        }
        // T4：入口语义（RFC-029f 角色驱动；#388 定案：按**绑定存在性**判定）。
        // Script 无入口概念（顶层代码即程序）；Bin 要求存在名为 main 的绑定
        // （值/函数皆可）；带参 main 编译期拒绝（#357：入口恒零参调用）。
        //
        // Check 形态整块跳过（纯合并）：E3020 的判据归 RoleClassification
        // （双报防线，见臂注释）；E3022 属 run 路径的入口契约，check 现状不报
        // 它——本臂在 check 只合并，不给 check 添新诊断。
        //
        // 还有一层硬性理由：run 路径的入口校验以「merged IR 里有没有 main」
        // 判定，而在 Check 的收集语义下，入口自身带错（parse/typecheck 失败）
        // 就没有 IR——会被误判成「缺 main」而硬中止，把 rust 式收集退化成
        // FailFast。并入纯合并即无此形状。
        if !matches!(state.program.kind, ProgramKind::Check) && is_bin_role(&state.entry_path) {
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

/// 一个单元**能按裸类型名命名的方法候选面**（RFC-029f 改进 4 / WBS 4.9.7 / #464）：
/// 本单元 `use M.{Type}` 带进来的**每个类型名**，只取它**自己的提供模块 M** 下
/// 「键前缀 = 该名字」的候选组（名字 ← 模块**逐对**配对，不跨对组合）。
///
/// 为什么必须逐对过滤（而不是"名字集合 × 模块集合"）：候选键是**裸类型名**
/// （`{Type}.{method}`）——两个单元各自定义同名类型时键相同。若把「本单元能命名
/// 到的模块集合」整体合并后再按「键前缀 ∈ 名字集合」过滤，任一被命名到的模块定义
/// 的同键候选都会混进来：调用点因此出现 2 个签名相同的候选，按「多候选无法区分即
/// 报错」的规则报 E1107。实测（WBS 4.9.7 修复轮）：main 用 `use a.{Temp}` +
/// `use b.{Other}`，a 也定义 Other、b 也定义 Temp ⇒ 每个名字只由一个模块带进
/// 作用域、接收者类型唯一，程序仍然 2×E1107（Temp.to / Other.to 各 2 候选）。
/// 反过来命名得到的候选一个都不能少（漏并 ⇒ 决议不产生条目 ⇒ 跨单元方法回落池的
/// 名字级豁免，W1005 漏报）。
///
/// 三条收窄规则（都朝**宁漏报**方向，宁可不产生条目也不制造歧义误报）：
/// ① 只认**花括号项**形态（`use M.{Type}`）：整体导入 `use M` 把类型名挂在模块
///    命名空间下（另按导出名补一份裸名镜像），两形态混用时裸名归属由导入顺序
///    静默决定（last-wins）——这种源程序在名字层已经歧义，本函数不为它产条目；
/// ② 只认**局部名 = 目标名**（无内联别名 `as`）：决议键的前缀是调用方写的裸名，
///    别名形态下它与定义侧前缀不同名，配不上（不猜、不重命名）；
/// ③ 同一个裸类型名被**多个模块**带进作用域时整名跳过（同上：源程序已歧义）。
///
/// 本单元**自有键位不外并**：同名裸类型在本单元已有定义时，调用点解析到的就是
/// 本单元那一个（模块身份优先）；自有候选由本单元自己的签名收集现场登记。
///
/// 名字面复用既有单点：[DeadCodeAnalyzer::collect_module_names]（use 面）+
/// 注册表导出查询（[ModuleRegistry::get_exports]）——与 checker 处理
/// `use M.{Type}` 时查模块导出的**同一张表**，不另造路径解析。
fn visible_method_overloads(
    ast: &Module,
    own_module_key: &str,
    registry: &ModuleRegistry,
) -> Vec<(String, MethodOverloadCandidates)> {
    let names = DeadCodeAnalyzer::collect_module_names(ast);
    // 裸类型名 → 带它进作用域的模块键（重复 = 名字层歧义 ⇒ 整名跳过）
    let mut scope: HashMap<String, String> = HashMap::new();
    let mut conflicted: HashSet<String> = HashSet::new();
    for (local, target, use_path) in &names.imported {
        if local != target {
            continue; // ② 内联别名：调用方写的裸名与定义侧前缀不同名
        }
        // use 路径必须命中注册表模块、且该模块确实导出这个名字（未解析导入 /
        // 子模块名都不构成裸类型名）；查法与 checker 的 use 处理同表
        let Some(exports) = registry.get_exports(use_path) else {
            continue;
        };
        if !exports.contains_key(target) {
            continue;
        }
        match scope.get(target) {
            Some(existing) if existing != use_path => {
                conflicted.insert(target.clone()); // ③ 同名被多模块带进作用域
            }
            Some(_) => {}
            None => {
                scope.insert(target.clone(), use_path.clone());
            }
        }
    }
    let own_keys: HashSet<String> = registry
        .get(own_module_key)
        .map(|module| module.method_overloads.keys().cloned().collect())
        .unwrap_or_default();
    // 名字 ← 提供模块**逐对**取候选：只有「键的提供模块 == 该类型名前缀的提供模块」
    // 的候选组才进决议表（跨对组合会把别的模块的同名类型候选塞进来 ⇒ 2 候选 ⇒ E1107）。
    // 名字升序 ⇒ 合并顺序确定（不同前缀的键互不相干）。
    let mut pairs: Vec<(String, String)> = scope
        .into_iter()
        .filter(|(name, _)| !conflicted.contains(name))
        .collect();
    pairs.sort();
    let mut merged: Vec<(String, MethodOverloadCandidates)> = Vec::new();
    for (name, module) in pairs {
        for (key, group) in registry.merge_method_overloads(&[module]) {
            if own_keys.contains(&key) {
                continue; // 自有定义优先：自有候选由本单元签名收集现场登记
            }
            if key
                .split_once('.')
                .is_some_and(|(type_name, _)| type_name == name)
            {
                merged.push((key, group));
            }
        }
    }
    merged
}
