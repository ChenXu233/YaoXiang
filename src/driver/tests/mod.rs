//! driver 模块测试 — mirrors src/driver/

mod check_rootless_mono;
mod codegen_init_calls;
mod dce_wiring;
mod driver;
mod helpers;
mod import_cycle;
mod lower_type_identity;
mod lsp_semantic_db;
mod manifest_error;
mod method_def_names;
mod method_dispatch;
mod method_unused;
mod mono_cross_unit;
mod named_args_cross_unit_method;
mod named_args_import_shapes;
mod named_import_calls;
mod obligations;
mod overload_injection;
mod program;
mod project;
mod ref_pool;
mod run_rootless;
mod short_name_dispatch_pins;
mod stage;
mod unit;
mod warning_parity;
