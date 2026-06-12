//! Z3 SMT 求解器后端
//!
//! 唯一依赖 z3-sys 的文件。通过 Z3 C API 将 SMTExpr 构造为 Z3 内部 AST，
//! 执行求解，解析结果。

use std::ffi::{CStr, CString};

use super::ast::{SMTCommand, SMTExpr, SMTModel, SMTResult, SMTSort};

/// Z3 后端——封装 Z3 context 和 solver 生命周期
pub struct Z3Backend {
    ctx: z3_sys::Z3_context,
}

/// Z3 初始化/运行时错误
#[derive(Debug)]
pub enum Z3Error {
    InitFailed(String),
}

impl std::fmt::Display for Z3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Z3Error::InitFailed(msg) => write!(f, "Z3 init failed: {}", msg),
        }
    }
}

impl Z3Backend {
    /// 创建 Z3 实例。整个编译过程只调用一次。
    pub fn new() -> Result<Self, Z3Error> {
        unsafe {
            let cfg = z3_sys::Z3_mk_config();
            let ctx = z3_sys::Z3_mk_context(cfg);
            z3_sys::Z3_del_config(cfg);

            if ctx.is_null() {
                return Err(Z3Error::InitFailed("Z3_mk_context returned null".into()));
            }

            Ok(Z3Backend { ctx })
        }
    }

    /// 发送 SMT 命令序列，返回求解结果
    pub fn solve(&self, commands: &[SMTCommand], timeout_ms: u64) -> SMTResult {
        unsafe {
            let solver = z3_sys::Z3_mk_solver(self.ctx);
            z3_sys::Z3_solver_inc_ref(self.ctx, solver);

            // 设置超时
            let timeout_str = timeout_ms.to_string();
            if let Ok(timeout_param) = CString::new(timeout_str) {
                let key = CString::new("timeout").unwrap();
                z3_sys::Z3_update_param_value(self.ctx, key.as_ptr(), timeout_param.as_ptr());
            }

            // 遍历命令
            for cmd in commands {
                match cmd {
                    SMTCommand::DeclareConst(_name, _sort) => {
                        // 常量在后续 assert 中引用时自然声明
                    }
                    SMTCommand::Assert(expr) => {
                        let z3_expr = expr_to_z3(self.ctx, expr);
                        z3_sys::Z3_solver_assert(self.ctx, solver, z3_expr);
                    }
                    SMTCommand::CheckSat | SMTCommand::GetModel => {
                        // 在循环结束后统一处理
                    }
                }
            }

            // 执行 check-sat
            let result = z3_sys::Z3_solver_check(self.ctx, solver);

            let smt_result = match result {
                z3_sys::Z3_L_FALSE => {
                    // unsat → 目标在假设下成立
                    SMTResult::Unsat
                }
                z3_sys::Z3_L_TRUE => {
                    // sat → 存在反例，解析 model
                    let model = z3_sys::Z3_solver_get_model(self.ctx, solver);
                    let assignments = if !model.is_null() {
                        z3_sys::Z3_model_inc_ref(self.ctx, model);
                        let assignments = parse_model(self.ctx, model);
                        z3_sys::Z3_model_dec_ref(self.ctx, model);
                        assignments
                    } else {
                        vec![]
                    };
                    SMTResult::Sat {
                        model: SMTModel { assignments },
                    }
                }
                z3_sys::Z3_L_UNDEF => {
                    let reason = z3_sys::Z3_solver_get_reason_unknown(self.ctx, solver);
                    let reason_str = if !reason.is_null() {
                        c_str_to_string(reason)
                    } else {
                        "timeout or incomplete theory".into()
                    };
                    SMTResult::Unknown {
                        reason: reason_str,
                    }
                }
                _ => SMTResult::Unknown {
                    reason: "unexpected Z3 result code".into(),
                },
            };

            z3_sys::Z3_solver_dec_ref(self.ctx, solver);
            smt_result
        }
    }
}

impl Drop for Z3Backend {
    fn drop(&mut self) {
        unsafe {
            z3_sys::Z3_del_context(self.ctx);
        }
    }
}

/// SMTSort → Z3_sort
unsafe fn sort_to_z3(ctx: z3_sys::Z3_context, sort: &SMTSort) -> z3_sys::Z3_sort {
    match sort {
        SMTSort::Bool => z3_sys::Z3_mk_bool_sort(ctx),
        SMTSort::Int => z3_sys::Z3_mk_int_sort(ctx),
        SMTSort::Real => z3_sys::Z3_mk_real_sort(ctx),
    }
}

/// SMTExpr → Z3_ast
unsafe fn expr_to_z3(ctx: z3_sys::Z3_context, expr: &SMTExpr) -> z3_sys::Z3_ast {
    match expr {
        SMTExpr::Atom(s) => {
            if let Ok(n) = s.parse::<i64>() {
                z3_sys::Z3_mk_int(ctx, n as i32, z3_sys::Z3_mk_int_sort(ctx))
            } else if s == "true" {
                z3_sys::Z3_mk_true(ctx)
            } else if s == "false" {
                z3_sys::Z3_mk_false(ctx)
            } else {
                // 变量引用：默认 Int 类型
                let sym = z3_sys::Z3_mk_string_symbol(ctx, CString::new(s.as_str()).unwrap().as_ptr());
                z3_sys::Z3_mk_const(ctx, sym, z3_sys::Z3_mk_int_sort(ctx))
            }
        }
        SMTExpr::App(op, args) => {
            // 使用 Z3 内置操作创建表达式
            let z3_args: Vec<z3_sys::Z3_ast> = args.iter().map(|a| expr_to_z3(ctx, a)).collect();
            let int_sort = z3_sys::Z3_mk_int_sort(ctx);

            match op.as_str() {
                // 比较运算
                ">" => z3_sys::Z3_mk_gt(ctx, z3_args[0], z3_args[1]),
                ">=" => z3_sys::Z3_mk_ge(ctx, z3_args[0], z3_args[1]),
                "<" => z3_sys::Z3_mk_lt(ctx, z3_args[0], z3_args[1]),
                "<=" => z3_sys::Z3_mk_le(ctx, z3_args[0], z3_args[1]),

                // 逻辑运算
                "and" => {
                    if z3_args.len() == 1 {
                        z3_args[0]
                    } else {
                        z3_sys::Z3_mk_and(ctx, z3_args.len() as u32, z3_args.as_ptr())
                    }
                }
                "or" => {
                    if z3_args.len() == 1 {
                        z3_args[0]
                    } else {
                        z3_sys::Z3_mk_or(ctx, z3_args.len() as u32, z3_args.as_ptr())
                    }
                }
                "not" => z3_sys::Z3_mk_not(ctx, z3_args[0]),

                // 算术运算
                "+" => z3_sys::Z3_mk_add(ctx, z3_args.len() as u32, z3_args.as_ptr()),
                "-" => z3_sys::Z3_mk_sub(ctx, z3_args.len() as u32, z3_args.as_ptr()),
                "*" => z3_sys::Z3_mk_mul(ctx, z3_args.len() as u32, z3_args.as_ptr()),
                "div" => z3_sys::Z3_mk_div(ctx, z3_args[0], z3_args[1]),
                "mod" => z3_sys::Z3_mk_mod(ctx, z3_args[0], z3_args[1]),

                // 等式
                "=" => z3_sys::Z3_mk_eq(ctx, z3_args[0], z3_args[1]),
                "distinct" => z3_sys::Z3_mk_distinct(ctx, z3_args.len() as u32, z3_args.as_ptr()),

                // 未知操作：创建未解释函数应用
                _ => {
                    let fn_sym = z3_sys::Z3_mk_string_symbol(ctx, CString::new(op.as_str()).unwrap().as_ptr());
                    let mut domain = vec![];
                    for _ in 0..z3_args.len() {
                        domain.push(int_sort);
                    }
                    let func_decl = z3_sys::Z3_mk_func_decl(
                        ctx,
                        fn_sym,
                        z3_args.len() as u32,
                        domain.as_ptr(),
                        int_sort,
                    );
                    z3_sys::Z3_mk_app(ctx, func_decl, z3_args.len() as u32, z3_args.as_ptr())
                }
            }
        }
        SMTExpr::Forall { vars, body } => {
            let z3_body = expr_to_z3(ctx, body);
            let bound: Vec<z3_sys::Z3_app> = vars
                .iter()
                .map(|(name, sort)| {
                    let sym = z3_sys::Z3_mk_string_symbol(ctx, CString::new(name.as_str()).unwrap().as_ptr());
                    let z3_sort = sort_to_z3(ctx, sort);
                    let ast = z3_sys::Z3_mk_const(ctx, sym, z3_sort);
                    z3_sys::Z3_to_app(ctx, ast)
                })
                .collect();
            z3_sys::Z3_mk_forall_const(
                ctx,
                0,
                bound.len() as u32,
                bound.as_ptr(),
                0,
                std::ptr::null(),
                z3_body,
            )
        }
        SMTExpr::Exists { vars, body } => {
            let z3_body = expr_to_z3(ctx, body);
            let bound: Vec<z3_sys::Z3_app> = vars
                .iter()
                .map(|(name, sort)| {
                    let sym = z3_sys::Z3_mk_string_symbol(ctx, CString::new(name.as_str()).unwrap().as_ptr());
                    let z3_sort = sort_to_z3(ctx, sort);
                    let ast = z3_sys::Z3_mk_const(ctx, sym, z3_sort);
                    z3_sys::Z3_to_app(ctx, ast)
                })
                .collect();
            z3_sys::Z3_mk_exists_const(
                ctx,
                0,
                bound.len() as u32,
                bound.as_ptr(),
                0,
                std::ptr::null(),
                z3_body,
            )
        }
    }
}

/// 解析 Z3 model，提取变量赋值
unsafe fn parse_model(
    ctx: z3_sys::Z3_context,
    model: z3_sys::Z3_model,
) -> Vec<(String, String)> {
    let mut assignments = Vec::new();
    let num_consts = z3_sys::Z3_model_get_num_consts(ctx, model);

    for i in 0..num_consts {
        let func_decl = z3_sys::Z3_model_get_const_decl(ctx, model, i);
        let name_sym = z3_sys::Z3_get_decl_name(ctx, func_decl);
        let name = z3_symbol_to_string(ctx, name_sym);

        let interp = z3_sys::Z3_model_get_const_interp(ctx, model, func_decl);
        if !interp.is_null() {
            let value_str = z3_sys::Z3_ast_to_string(ctx, interp);
            let value = c_str_to_string(value_str);
            assignments.push((name, value));
        }
    }

    assignments
}

// --- 辅助函数 ---

/// Z3_symbol → String
unsafe fn z3_symbol_to_string(ctx: z3_sys::Z3_context, sym: z3_sys::Z3_symbol) -> String {
    let ptr = z3_sys::Z3_get_symbol_string(ctx, sym);
    c_str_to_string(ptr)
}

/// Z3_string → String
unsafe fn c_str_to_string(ptr: z3_sys::Z3_string) -> String {
    if ptr.is_null() {
        return "(null)".into();
    }
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}
