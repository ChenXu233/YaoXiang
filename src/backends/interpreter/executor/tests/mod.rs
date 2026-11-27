//! 解释器执行器测试入口
//!
//! 包含 debug、execute、float_compare、logical_not、shift_semantics、
//! comparison_type_mismatch 与 stack_frames 的测试模块。

mod comparison_type_mismatch;
mod debug;
mod execute;
mod float_compare;
mod logical_not;
mod shift_semantics;
mod stack_frames;
mod stack_guard;
mod task_error_fidelity;
