//! 核心中间表示测试模块
//!
//! 包含 bytecode、ir 等核心模块的单元测试。

pub mod alias_fn_value;
pub mod batch_method_bind;
pub mod bytecode;
pub mod corpus;
pub mod def_assign;
pub mod ir_build;
pub mod local_slots;
pub mod overload_resolutions;
pub mod release_plan_spans;
pub mod snapshot;
pub mod verify;
