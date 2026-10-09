//! 编译程序描述：`Program` / `ProgramKind` / `Aggregation`
//! —— 02-stage-contract §目标设计 3/5。
//!
//! `Program` 把「这次编译是什么形态、跑哪些阶段、诊断怎么聚合」变成
//! 一份声明式描述；`Driver` 是唯一解释它的地方。
//!
//! **关键约束**：`Program.stages` 只能来自 `ProgramKind::stages()` 的
//! 6 个预定义组合之一，**不接受调用方自由传数组**——这是防止 `Program`
//! 抽象退化成「什么都能传的自由参数」的机制，由 `test_program_stage_coverage`
//! 断言（02 §目标设计 3）。

use super::stage::Stage;
use super::unit::Unit;
use crate::frontend::config::CompileConfig;

/// 编译程序形态——六条编译路径各对应一个变体
/// （02 §现状「六条编译路径、十个入口函数」表格的归并）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ProgramKind {
    /// 单文件管线（pipeline.rs / lib::run_file / build_bytecode）
    SingleFile,
    /// 多文件编译（orchestrator::compile_project / lib::run_project）
    MultiFile,
    /// 多文件检查（orchestrator::check_project / yaoxiang check）
    Check,
    /// LSP 项目内检查（orchestrator::check_source_in_project，结果过滤到目标文件）
    Lsp,
    /// 嵌入 std 模块编译（orchestrator::compile_embedded_module，RFC-036 §4）
    Embedded,
    /// wasm playground（wasm/src/lib.rs——单文件路径的第四个调用方）
    WasmPlayground,
}

/// 诊断聚合模式（02 §目标设计 5）。
///
/// 必须是 `Program` 的字段而非 Driver 全局设置：LSP 在同一进程内既服务
/// 项目内文件也服务单文件，CLI 的 run 与 check 是两次独立调用。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Aggregation {
    /// 首个错误即中止（compile 路径）
    FailFast,
    /// 收集全部诊断（check / LSP 路径）
    CollectAll,
}

/// 各 ProgramKind 的阶段表——预定义组合的**唯一来源**。
///
/// 阶段集合按各路径的**现状行为**归并（02 §现状 11 处不一致表 + P3 止血
/// 后的 proof 接线）；统一后的一致性行为由本表声明，差异处即 02 要修的
/// 覆盖缺口。拓扑位置一律合法（皆为 `Stage::ALL` 顺序的子序列）。
const SINGLE_FILE_STAGES: &[Stage] = &[
    Stage::Parsing,
    Stage::Typecheck,
    Stage::DeadCodeAnalysis,
    Stage::ProofExecution,
    Stage::IrGeneration,
    Stage::Monomorphization,
];

const MULTI_FILE_STAGES: &[Stage] = &[
    Stage::VendorConsistency,
    Stage::Discovery,
    Stage::Parsing,
    Stage::Registry,
    Stage::Typecheck,
    Stage::ProofExecution,
    Stage::GlobalSlotAlloc,
    Stage::IrGeneration,
    Stage::Linking,
];

// 注：check_project 现状是 proof（orchestrator.rs）先于 dead_code 执行；
// 统一后按 Stage::ALL 拓扑序 dead_code 先于 proof——两者无数据依赖，
// 诊断集相同（C2 集合语义），仅文件内诊断顺序归一（02 §1 修订注记）。
const CHECK_STAGES: &[Stage] = &[
    Stage::VendorConsistency,
    Stage::Discovery,
    Stage::Parsing,
    Stage::Registry,
    Stage::RoleClassification,
    Stage::Typecheck,
    Stage::DeadCodeAnalysis,
    Stage::ProofExecution,
];

const LSP_STAGES: &[Stage] = &[
    Stage::VendorConsistency,
    Stage::Discovery,
    Stage::Parsing,
    Stage::Registry,
    Stage::Typecheck,
    Stage::ProofExecution,
];

const EMBEDDED_STAGES: &[Stage] = &[
    Stage::Parsing,
    Stage::Typecheck,
    Stage::ProofExecution,
    Stage::IrGeneration,
];

impl ProgramKind {
    /// 本形态的阶段表（6 个预定义组合之一）。
    ///
    /// `SingleFile` 与 `WasmPlayground` 共用同一条单文件管线
    /// （wasm playground 经 `Compiler::compile_with_source` 落到 pipeline）。
    pub fn stages(self) -> &'static [Stage] {
        match self {
            ProgramKind::SingleFile | ProgramKind::WasmPlayground => SINGLE_FILE_STAGES,
            ProgramKind::MultiFile => MULTI_FILE_STAGES,
            ProgramKind::Check => CHECK_STAGES,
            ProgramKind::Lsp => LSP_STAGES,
            ProgramKind::Embedded => EMBEDDED_STAGES,
        }
    }

    /// 本形态的默认聚合模式（02 §目标设计 5 现状对应表）。
    pub fn default_aggregation(self) -> Aggregation {
        match self {
            ProgramKind::SingleFile
            | ProgramKind::MultiFile
            | ProgramKind::Embedded
            | ProgramKind::WasmPlayground => Aggregation::FailFast,
            ProgramKind::Check | ProgramKind::Lsp => Aggregation::CollectAll,
        }
    }
}

/// 一次编译的完整声明式描述。
///
/// `stages` 字段**私有**：只能经 `Program::new` 从 `kind` 派生，
/// 调用方无法传入自定义阶段数组（02 §目标设计 3 的防退化约束）。
pub struct Program {
    /// 编译形态
    pub kind: ProgramKind,
    /// 编译单元（单文件 1 个 / 多文件 N 个）
    pub units: Vec<Unit>,
    /// 阶段表——只能来自 `ProgramKind::stages()`
    stages: &'static [Stage],
    /// 诊断聚合模式
    pub aggregation: Aggregation,
    /// 编译配置（死代码/单态化门控等）
    pub config: CompileConfig,
}

impl Program {
    /// 以默认聚合模式构造。
    pub fn new(
        kind: ProgramKind,
        units: Vec<Unit>,
        config: CompileConfig,
    ) -> Self {
        Self {
            kind,
            units,
            stages: kind.stages(),
            aggregation: kind.default_aggregation(),
            config,
        }
    }

    /// 覆盖聚合模式（LSP 单文件分支：SingleFile 形态 + CollectAll）。
    pub fn with_aggregation(
        mut self,
        aggregation: Aggregation,
    ) -> Self {
        self.aggregation = aggregation;
        self
    }

    /// 阶段表（来自 `ProgramKind::stages()` 的预定义组合）。
    pub fn stages(&self) -> &'static [Stage] {
        self.stages
    }
}
