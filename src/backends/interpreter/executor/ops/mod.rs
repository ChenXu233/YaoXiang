//! 指令族执行模块
//!
//! `execute_instr` 原先是一个 1300 余行、58 个 arm 的单函数——定位任一指令
//! 的实现需在大段代码中翻找，且不同族的逻辑（控制流、容器、调用、并发）
//! 混在一起，阅读与修改都要先建立全貌。
//!
//! 本模块把各族 arm 体搬进独立的 `exec_*` 方法，`execute_instr` 保留单层
//! `match` 只做分派。arm 体逐字搬迁，不改变执行语义；族方法标 `#[inline]`
//! 以免引入额外调用开销。

pub mod arc;
pub mod arith;
pub mod call;
pub mod container;
pub mod control;
pub mod elem;
pub mod misc;
pub mod reg;
