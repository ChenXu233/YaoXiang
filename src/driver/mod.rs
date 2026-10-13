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
//!（02 §4）：三类原因（上游失败 / 无义务 / 配置未启用）记入
//! `DriverOutcome.skipped`。
//!
//! **接线现状（4.2.4）**：12 个 `Stage` 变体全接线；六条编译路径
//!（SingleFile / WasmPlayground / MultiFile / Check / Lsp / Embedded）
//! 全部开通。
//!
//! **义务账本接线（4.3.1 / 4.3.2）**：`run` 在阶段表尾部、产出 `CompilationResult`
//! 之前调用 `State::assert_drained()`——**唯一结算点**（02 §2）。结算把
//! 「义务非空但没交付」外发为 `W1090`：4.3.1 为 W 级（RFC-039 D5 观测期），
//! **4.3.2 起同码升 E**（显式 `Severity::Error`，2026-10-13 用户裁决甲；码号不变、
//! 不新增 E 码）——触发即进错误通道、渲染 `error[W1090]`、退出码非零
//! （D59 的 D 类破坏项；健康态零发射，故既有语料与用户程序零影响）。
//! **红名单当前为空**（3.4.4/3.4.5/3.4.9 三条清账任务已全部落地）：机制保留、条目为零；
//! 字段普查由「义务清单 ∪ 非义务登记（`NON_OBLIGATION_FIELDS`）」闭合，
//! 由 `DriverOutcome.obligations.findings` 审计面 +
//! `scripts/ci/check-obligations.py` 门禁 + 02 §2 注记三处承担
//! （详见 `obligations.rs` 模块头）。`Skipped` 记录本身**仍不外发诊断**
//!（02 §4 文案表的外发归后续轮次；本轮契约明列「上游失败 / 配置关闭导致的
//! 跳过不报」），它只作义务结算的豁免判据与审计面。
//!
//! **文件布局（4.2.9 拆分——mod.rs 达 1766 行，超仓库 1500 行阈值）**：
//! - `mod.rs`：`Driver`、`run`/`dispatch` 解释器核心与模块级重导出；
//! - `state.rs`：`State`（一次编译的可变上下文）、`DriverOutcome`/
//!   `DriverError`/跳过台账类型、数据依赖表；
//! - `arms.rs`：12 个阶段臂（`impl Driver` 的第二部分，跨形态臂按
//!   ProgramKind 分形）；
//! - `helpers.rs`：臂共用的纯函数（链接、槽位分配、嵌入 std 合并、
//!   角色上下文、引用池收集）。
//!
//! 与 02 草图的两处偏差（语义等价，登记备查）：
//! - 02 §4 的 `StageOutcome` 三态不由阶段臂返回，而由 `State` 的失败
//!   标记 + `Aggregation` 门控表达（Continue = 无错误无警告 /
//!   Warn = 产警告 / Abort = 产错误并触发 `should_abort`），省去每臂
//!   的样板返回；
//! - `Driver` 无 `config` 字段——配置唯一来源是 `Program.config`
//!   （02 §5：避免 Driver 持可变全局状态），一次 run 的可变状态全部
//!   在 `State` 内。

pub mod obligations;
pub mod program;
pub mod stage;
pub mod unit;

mod arms;
mod helpers;
mod state;

pub use obligations::{
    Obligation, ObligationFinding, ObligationSettlement, OBLIGATION_UNDRAINED_CODE, RedListEntry,
    UndrainedCause,
};
pub use program::{Aggregation, Program, ProgramKind};
pub use stage::{Stage, StageScope};
pub use state::{DriverError, DriverOutcome, SkipReason, SkippedStage};
pub use unit::Unit;

use state::State;

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
    ///
    /// **阶段表尾部结算义务账本**（02 §2）：`assert_drained()` 的调用点是
    /// 唯一的一处——产出 `CompilationResult`（`into_outcome`）之前。
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
        // 义务账本结算：阶段表尾部、产出 CompilationResult 之前
        // ——02 §2 的唯一调用点（早退路径同样结算：失败的编译也有账）
        state.assert_drained();
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
}

#[cfg(test)]
mod tests;
