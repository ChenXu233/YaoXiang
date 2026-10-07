//! L1 编排层：统一 Driver 与阶段契约 —— 02-stage-contract / RFC-039。
//!
//! 依赖方向红线（02 §详细设计）：`driver`（L1）依赖 `frontend`（L2）/
//! `middle`（L3）/ `backends`（L4）的接口；**L2/L3/L4 不得反向
//! `use crate::driver`**，由 check-module-boundary 门禁拦截。

pub mod stage;

pub use stage::{Stage, StageScope};

#[cfg(test)]
mod tests;
