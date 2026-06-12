//! Z3 C API FFI 绑定——仅包含 YaoXiang 实际使用的函数。
//!
//! 从 z3-sys 中提取，预生成并提交，避免构建期依赖 z3.h。

#![allow(non_camel_case_types)]

use std::ffi::c_char;

// ============ 不透明类型 ============

pub type Z3_config = *mut Z3_config_struct;
pub type Z3_context = *mut Z3_context_struct;
pub type Z3_solver = *mut Z3_solver_struct;
pub type Z3_sort = *mut Z3_sort_struct;
pub type Z3_ast = *mut Z3_ast_struct;
pub type Z3_app = *mut Z3_app_struct;
pub type Z3_func_decl = *mut Z3_func_decl_struct;
pub type Z3_model = *mut Z3_model_struct;
pub type Z3_symbol = *mut Z3_symbol_struct;
pub type Z3_string = *const c_char;

#[repr(C)] pub struct Z3_config_struct { _unused: [u8; 0] }
#[repr(C)] pub struct Z3_context_struct { _unused: [u8; 0] }
#[repr(C)] pub struct Z3_solver_struct { _unused: [u8; 0] }
#[repr(C)] pub struct Z3_sort_struct { _unused: [u8; 0] }
#[repr(C)] pub struct Z3_ast_struct { _unused: [u8; 0] }
#[repr(C)] pub struct Z3_app_struct { _unused: [u8; 0] }
#[repr(C)] pub struct Z3_func_decl_struct { _unused: [u8; 0] }
#[repr(C)] pub struct Z3_model_struct { _unused: [u8; 0] }
#[repr(C)] pub struct Z3_symbol_struct { _unused: [u8; 0] }

// ============ 常量 ============

pub const Z3_L_FALSE: i32 = -1;
pub const Z3_L_UNDEF: i32 = 0;
pub const Z3_L_TRUE: i32 = 1;

// ============ 生命周期 ============

extern "C" {
    pub fn Z3_mk_config() -> Z3_config;
    pub fn Z3_del_config(c: Z3_config);
    pub fn Z3_mk_context(c: Z3_config) -> Z3_context;
    pub fn Z3_del_context(c: Z3_context);
    pub fn Z3_update_param_value(c: Z3_context, param_id: Z3_string, param_value: Z3_string);
}

// ============ Solver ============

extern "C" {
    pub fn Z3_mk_solver(c: Z3_context) -> Z3_solver;
    pub fn Z3_solver_inc_ref(c: Z3_context, s: Z3_solver);
    pub fn Z3_solver_dec_ref(c: Z3_context, s: Z3_solver);
    pub fn Z3_solver_assert(c: Z3_context, s: Z3_solver, a: Z3_ast);
    pub fn Z3_solver_check(c: Z3_context, s: Z3_solver) -> i32;
    pub fn Z3_solver_get_model(c: Z3_context, s: Z3_solver) -> Z3_model;
    #[allow(dead_code)]
    pub fn Z3_solver_get_reason_unknown(c: Z3_context, s: Z3_solver) -> Z3_string;
}

// ============ Model ============

extern "C" {
    pub fn Z3_model_inc_ref(c: Z3_context, m: Z3_model);
    pub fn Z3_model_dec_ref(c: Z3_context, m: Z3_model);
    pub fn Z3_model_get_num_consts(c: Z3_context, m: Z3_model) -> u32;
    pub fn Z3_model_get_const_decl(c: Z3_context, m: Z3_model, i: u32) -> Z3_func_decl;
    pub fn Z3_model_get_const_interp(c: Z3_context, m: Z3_model, a: Z3_func_decl) -> Z3_ast;
}

// ============ Sort ============

extern "C" {
    pub fn Z3_mk_bool_sort(c: Z3_context) -> Z3_sort;
    pub fn Z3_mk_int_sort(c: Z3_context) -> Z3_sort;
    pub fn Z3_mk_real_sort(c: Z3_context) -> Z3_sort;
}

// ============ 表达式构造 ============

extern "C" {
    pub fn Z3_mk_true(c: Z3_context) -> Z3_ast;
    pub fn Z3_mk_false(c: Z3_context) -> Z3_ast;
    pub fn Z3_mk_int(c: Z3_context, v: i32, ty: Z3_sort) -> Z3_ast;
    pub fn Z3_mk_const(c: Z3_context, s: Z3_symbol, ty: Z3_sort) -> Z3_ast;
    pub fn Z3_mk_string_symbol(c: Z3_context, s: Z3_string) -> Z3_symbol;

    // 比较
    pub fn Z3_mk_gt(c: Z3_context, l: Z3_ast, r: Z3_ast) -> Z3_ast;
    pub fn Z3_mk_ge(c: Z3_context, l: Z3_ast, r: Z3_ast) -> Z3_ast;
    pub fn Z3_mk_lt(c: Z3_context, l: Z3_ast, r: Z3_ast) -> Z3_ast;
    pub fn Z3_mk_le(c: Z3_context, l: Z3_ast, r: Z3_ast) -> Z3_ast;

    // 逻辑
    pub fn Z3_mk_and(c: Z3_context, num_args: u32, args: *const Z3_ast) -> Z3_ast;
    pub fn Z3_mk_or(c: Z3_context, num_args: u32, args: *const Z3_ast) -> Z3_ast;
    pub fn Z3_mk_not(c: Z3_context, a: Z3_ast) -> Z3_ast;

    // 算术
    pub fn Z3_mk_add(c: Z3_context, num_args: u32, args: *const Z3_ast) -> Z3_ast;
    pub fn Z3_mk_sub(c: Z3_context, num_args: u32, args: *const Z3_ast) -> Z3_ast;
    pub fn Z3_mk_mul(c: Z3_context, num_args: u32, args: *const Z3_ast) -> Z3_ast;
    pub fn Z3_mk_div(c: Z3_context, l: Z3_ast, r: Z3_ast) -> Z3_ast;
    pub fn Z3_mk_mod(c: Z3_context, l: Z3_ast, r: Z3_ast) -> Z3_ast;

    // 等式
    pub fn Z3_mk_eq(c: Z3_context, l: Z3_ast, r: Z3_ast) -> Z3_ast;
    pub fn Z3_mk_distinct(c: Z3_context, num_args: u32, args: *const Z3_ast) -> Z3_ast;

    // 函数
    pub fn Z3_mk_func_decl(
        c: Z3_context, s: Z3_symbol, domain_size: u32, domain: *const Z3_sort, range: Z3_sort,
    ) -> Z3_func_decl;
    pub fn Z3_mk_app(
        c: Z3_context, d: Z3_func_decl, num_args: u32, args: *const Z3_ast,
    ) -> Z3_ast;

    // 量词
    pub fn Z3_mk_forall_const(
        c: Z3_context, weight: u32, num_bound: u32, bound: *const Z3_app,
        num_patterns: u32, patterns: *const Z3_ast, body: Z3_ast,
    ) -> Z3_ast;
    pub fn Z3_mk_exists_const(
        c: Z3_context, weight: u32, num_bound: u32, bound: *const Z3_app,
        num_patterns: u32, patterns: *const Z3_ast, body: Z3_ast,
    ) -> Z3_ast;

    pub fn Z3_to_app(c: Z3_context, a: Z3_ast) -> Z3_app;
}

// ============ 符号 ============

extern "C" {
    pub fn Z3_get_decl_name(c: Z3_context, d: Z3_func_decl) -> Z3_symbol;
    pub fn Z3_get_symbol_string(c: Z3_context, s: Z3_symbol) -> Z3_string;
}

// ============ 诊断 ============

extern "C" {
    pub fn Z3_ast_to_string(c: Z3_context, a: Z3_ast) -> Z3_string;
}
