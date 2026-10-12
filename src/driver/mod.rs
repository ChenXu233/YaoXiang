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
//! 全部开通。Skipped 只进内部记录、**不外发诊断**（外发随 4.3 义务账本，
//! 02 §4 文案表）——zero-diff 判据（C2）要求任何新增对外诊断都必须显式
//! 裁决。
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

pub mod program;
pub mod stage;
pub mod unit;

mod arms;
mod helpers;
mod state;

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
}

#[cfg(test)]
mod tests;
