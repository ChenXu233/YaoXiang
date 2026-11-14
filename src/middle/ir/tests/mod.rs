//! IR 测试模块
//!
//! 包含 IR 构建 / 槽位 / 快照 / verify 等单元测试。
//! （字节码 IR 与 .42 序列化的单测自 D1 6.7.1b' 起随实现住在 `middle/bytecode/tests/`。）

pub mod alias_fn_value;
pub mod batch_method_bind;
pub mod corpus;
pub mod def_assign;
pub mod fn_param_slots;
pub mod generic_params;
pub mod ir_build;
pub mod local_slots;
pub mod namespace_scope;
pub mod native_fn_value;
pub mod overload_resolutions;
pub mod release_plan_spans;
pub mod snapshot;
pub mod type_decl;
mod type_decl_quantity;
pub mod verify;
mod witness_quantity;
