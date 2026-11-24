//! 解释器执行器测试入口
//!
//! 包含 debug、execute、float_compare、logical_not、comparison_type_mismatch
//! 与 stack_frames 的测试模块。

mod comparison_type_mismatch;
mod debug;
mod execute;
mod float_compare;
mod logical_not;
mod stack_frames;
mod stack_guard;
mod task_error_fidelity;
