//! 执行状态与产物类型（4.2.9 自 mod.rs 拆出）。
//!
//! `State` 是一次编译的可变执行上下文（只对 driver 模块树可见）；
//! `DriverOutcome` / `DriverError` / `SkipReason` / `SkippedStage`
//! 是 driver 的对外类型面（经 mod.rs 重导出，路径不变）。

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;

use crate::frontend::core::parser::Module;
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::module::orchestrator::{self, OrchestratorError};
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::roles::FileRole;
use crate::frontend::pipeline::{CompilationResult, PipelineError};
use crate::middle::ModuleIR;
use crate::util::diagnostic::Diagnostic;

use super::{Aggregation, Program, ProgramKind, Stage};

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
///   契约）；Check/Lsp：`check_diagnostics`（逐文件收集通道，4.2.2/
///   4.2.3——Lsp 由调用方过滤到目标文件）；Embedded 随 4.2.4 落地。
#[derive(Debug)]
pub struct DriverOutcome {
    /// 编译结果（诊断集与 pipeline 时代逐字节相同，C2 判据）
    pub result: CompilationResult,
    /// 多文件合并 IR（Linking 产物；MultiFile 成功时 Some）/ 嵌入 std
    /// 模块的独立 ModuleIR（Embedded 成功时 Some，4.2.4）
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
pub(in crate::driver) struct State {
    /// 正在解释的程序
    pub(in crate::driver) program: Program,
    /// 逐编译单元的阶段产物（与 `program.units` 等长、按下标对应）
    pub(in crate::driver) units: Vec<UnitState>,
    /// Parsing 产物（独立存放：多文件 IrGeneration 需要全部 AST 的引用
    /// 视图——挂在 UnitState 上会与逐单元可变访问冲突；orchestrator 时代
    /// asts 同样是独立 Vec）
    pub(in crate::driver) asts: Vec<Option<Module>>,
    /// 已产出的错误诊断（FailFast 下恰为首个失败阶段的那批）
    pub(in crate::driver) errors: Vec<PipelineError>,
    /// 已产出的警告诊断
    pub(in crate::driver) warnings: Vec<Diagnostic>,
    /// 跳过记录（内部审计；4.3 前不外发）
    pub(in crate::driver) skipped: Vec<SkippedStage>,
    /// 已失败的阶段（拓扑跳过的判定依据）
    pub(in crate::driver) failed_stages: Vec<Stage>,
    /// 入口路径与模块键（Discovery 重建 units 前从占位入口单元捕获）
    pub(in crate::driver) entry_path: PathBuf,
    /// 入口模块键（Linking 的 `{entry_key}.main` 查找用）
    pub(in crate::driver) entry_key: String,
    /// 多文件结构化故障（OrchestratorError 契约；与 errors 通道互斥——
    /// 通道选择见 `uses_pipeline_channel`）
    pub(in crate::driver) failure: Option<OrchestratorError>,
    /// Discovery 产物：被 ≥1 个文件 `use` 的文件路径集（canonical 化）——
    /// RoleClassification 的 Lib 信号（RFC-029f；4.2.2）
    pub(in crate::driver) used_by: HashSet<PathBuf>,
    /// Discovery 产物：本地模块遮蔽依赖包事件（W1006，RFC-014 §项目模式）
    /// ——DeadCodeAnalysis 的 Check 形态消费（4.2.2）
    pub(in crate::driver) shadow_events: Vec<orchestrator::ShadowEvent>,
    /// VendorConsistency 产物（Typecheck 的 E5001 install 提示注入用）
    pub(in crate::driver) vendor_root: Option<PathBuf>,
    /// Registry 产物（多文件 Typecheck / IrGeneration 的共享注册表）
    pub(in crate::driver) registry: Option<ModuleRegistry>,
    /// Registry 产物（跨文件方法调用解析，RFC-029）
    pub(in crate::driver) method_bindings: Option<HashMap<String, MonoType>>,
    /// GlobalSlotAlloc 产物（限定名 → 绝对槽位号）
    pub(in crate::driver) global_layout: Option<Vec<(String, usize)>>,
    /// GlobalSlotAlloc 产物（各文件槽位基址）
    pub(in crate::driver) slot_bases: Vec<usize>,
    /// Linking 产物
    pub(in crate::driver) merged_ir: Option<ModuleIR>,
}

/// 逐编译单元的阶段产物（PerModule 阶段的执行粒度）。
#[derive(Default)]
pub(in crate::driver) struct UnitState {
    /// Typecheck 产物
    pub(in crate::driver) type_result: Option<TypeCheckResult>,
    /// typecheck 警告暂存——外发与否由 DeadCodeAnalysis 臂的门控决定
    /// （pipeline 现状耦合，见该臂注释）
    pub(in crate::driver) typecheck_warnings: Vec<Diagnostic>,
    /// 本单元 typecheck 是否产出错误（死代码门控的 `!has_errors` 半边）
    pub(in crate::driver) typecheck_failed: bool,
    /// IrGeneration（+ Monomorphization 改写）产物
    pub(in crate::driver) ir: Option<ModuleIR>,
    /// RoleClassification 产物（RFC-029f 角色；DeadCodeAnalysis 的消费端，4.2.2）
    pub(in crate::driver) role: Option<FileRole>,
    /// 逐文件诊断收集（Check 通道；RoleClassification / Typecheck /
    /// DeadCodeAnalysis / ProofExecution 四臂按拓扑序追加——文件内顺序
    /// 即阶段序，2026-10-09 用户裁决的顺序归一）
    pub(in crate::driver) check_diagnostics: Vec<Diagnostic>,
}

impl State {
    pub(in crate::driver) fn new(mut program: Program) -> Self {
        // 占位入口单元（多文件形态）或唯一单元（单文件形态）——Discovery
        // 臂会重建 units，入口信息先行捕获（run() 已拒绝空 units）
        let entry_path = program.units[0].path.clone();
        let entry_key = program.units[0].key.clone();
        let n = program.units.len();
        // Embedded 形态（4.2.4）：注册表来自调用方注入（阶段表无
        // Registry 阶段）；其余形态由 Registry 臂产出
        let shared_registry = program.shared_registry.take();
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
            registry: shared_registry,
            method_bindings: None,
            global_layout: None,
            slot_bases: Vec::new(),
            merged_ir: None,
        }
    }

    /// 记录一次跳过（拓扑判定，非阶段返回值——02 §4）。
    pub(in crate::driver) fn record_skipped(
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
    pub(in crate::driver) fn uses_pipeline_channel(&self) -> bool {
        matches!(
            self.program.kind,
            ProgramKind::SingleFile | ProgramKind::WasmPlayground
        )
    }

    /// 记录一次多文件故障（OrchestratorError 通道）并标记阶段失败。
    pub(in crate::driver) fn fail_with(
        &mut self,
        stage: Stage,
        error: OrchestratorError,
    ) {
        self.failure = Some(error);
        self.failed_stages.push(stage);
    }

    /// 聚合模式门控：FailFast 下首个失败阶段即中止（02 §3 草图 break）；
    /// 多文件故障一律中止（compile_project 现状：`?` 传播）。
    pub(in crate::driver) fn should_abort(&self) -> bool {
        if self.failure.is_some() {
            return true;
        }
        matches!(self.program.aggregation, Aggregation::FailFast) && !self.errors.is_empty()
    }

    /// 拓扑判定：本阶段声明的数据依赖中，落在本程序阶段表内且已失败的
    /// 首个上游阶段。
    pub(in crate::driver) fn first_failed_upstream(
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
    pub(in crate::driver) fn into_outcome(mut self) -> DriverOutcome {
        if let Some(error) = self.failure.take() {
            return DriverOutcome {
                result: CompilationResult::default(),
                module: None,
                failure: Some(error),
                check_diagnostics: Vec::new(),
                skipped: self.skipped,
            };
        }
        // Check/Lsp 通道（4.2.2/4.2.3）：逐文件诊断收集——每个发现文件
        // 都有条目（干净文件为空 Vec，check_project 契约；Lsp 由调用方
        // 过滤到目标文件）；其余形态恒空
        let check_diagnostics =
            if matches!(self.program.kind, ProgramKind::Check | ProgramKind::Lsp) {
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
        // Embedded 产物（4.2.4）：唯一单元的 IR 即最终产物（阶段表无
        // Linking）；MultiFile 的 module 仍由 Linking 臂写入 merged_ir
        let module = if matches!(self.program.kind, ProgramKind::Embedded) {
            self.units.first_mut().and_then(|u| u.ir.take())
        } else {
            self.merged_ir.take()
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
            module,
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
