//! 类型检查器模块
//!
//! 实现 YaoXiang 语言的类型检查器，支持：
//! - Hindley-Milner 类型推断
//! - 泛型函数和泛型类型
//! - 完整的错误收集

pub mod check;
mod errors;
pub mod infer;
pub mod specialize;
mod tests;
pub mod types;

pub use check::*;
pub use errors::*;
pub use infer::*;
pub use specialize::*;
pub use types::*;

use super::parser::ast;
use crate::middle;
use crate::util::i18n::{t, t_simple, MSG};
use crate::util::logger::get_lang;
use std::collections::HashMap;
use tracing::debug;

/// 类型环境
///
/// 存储类型检查过程中的所有状态
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    /// 变量绑定
    vars: HashMap<String, PolyType>,
    /// 类型定义
    types: HashMap<String, PolyType>,
    /// 求解器
    solver: TypeConstraintSolver,
    /// 错误收集器
    errors: ErrorCollector,
}

impl TypeEnvironment {
    /// 创建新的类型环境
    pub fn new() -> Self {
        TypeEnvironment {
            vars: HashMap::new(),
            types: HashMap::new(),
            solver: TypeConstraintSolver::new(),
            errors: ErrorCollector::new(),
        }
    }

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.vars.insert(name, poly);
    }

    /// 获取变量类型
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.vars.get(name)
    }

    /// 添加类型定义
    pub fn add_type(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.types.insert(name, poly);
    }

    /// 获取类型定义
    pub fn get_type(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.types.get(name)
    }

    /// 获取求解器
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        &mut self.solver
    }

    /// 获取错误收集器
    pub fn errors(&mut self) -> &mut ErrorCollector {
        &mut self.errors
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        self.errors.has_errors()
    }

    /// 获取所有错误
    pub fn get_errors(&self) -> &[TypeError] {
        self.errors.errors()
    }
}

/// 检查模块并生成 IR
///
/// # Arguments
///
/// * `ast` - AST 模块
/// * `env` - 类型环境（可选，如果为 None 则创建新的）
///
/// # Returns
///
/// 成功返回 IR，失败返回错误列表
pub fn check_module(
    ast: &ast::Module,
    env: Option<&mut TypeEnvironment>,
) -> Result<middle::ModuleIR, Vec<TypeError>> {
    let lang = get_lang();
    let item_count = ast.items.len();
    debug!("{}", t(MSG::TypeCheckStart, lang, Some(&[&item_count])));
    let env = env.unwrap_or_else(|| {
        let mut new_env = TypeEnvironment::new();
        // 添加内置类型
        add_builtin_types(&mut new_env);
        Box::leak(Box::new(new_env))
    });

    // 复制环境变量（因为 checker 会借用 env.solver，所以需要先复制 vars）
    let vars = env.vars.clone();

    // 在内部作用域中执行类型检查，确保 checker 在访问 env 之前被 drop
    let (result, checker_errors) = {
        let mut checker = TypeChecker::new(env.solver());

        // 添加环境变量到检查器
        for (name, poly) in vars {
            checker.add_var(name, poly);
        }

        let result = checker.check_module(ast);
        let errors = checker.errors().to_vec();
        (result, errors)
    };

    // 现在可以安全访问 env
    for error in checker_errors {
        env.errors().add_error(error);
    }

    if env.has_errors() {
        Err(env.get_errors().to_vec())
    } else {
        debug!("{}", t_simple(MSG::TypeCheckComplete, lang));
        result
    }
}

/// 添加内置类型到环境
fn add_builtin_types(env: &mut TypeEnvironment) {
    // 数值类型
    env.add_type("int8".to_string(), PolyType::mono(MonoType::Int(8)));
    env.add_type("int16".to_string(), PolyType::mono(MonoType::Int(16)));
    env.add_type("int32".to_string(), PolyType::mono(MonoType::Int(32)));
    env.add_type("int64".to_string(), PolyType::mono(MonoType::Int(64)));
    env.add_type("uint8".to_string(), PolyType::mono(MonoType::Int(8)));
    env.add_type("uint16".to_string(), PolyType::mono(MonoType::Int(16)));
    env.add_type("uint32".to_string(), PolyType::mono(MonoType::Int(32)));
    env.add_type("uint64".to_string(), PolyType::mono(MonoType::Int(64)));
    env.add_type("float32".to_string(), PolyType::mono(MonoType::Float(32)));
    env.add_type("float64".to_string(), PolyType::mono(MonoType::Float(64)));

    // 其他内置类型
    env.add_type("bool".to_string(), PolyType::mono(MonoType::Bool));
    env.add_type("char".to_string(), PolyType::mono(MonoType::Char));
    env.add_type("string".to_string(), PolyType::mono(MonoType::String));
    env.add_type("bytes".to_string(), PolyType::mono(MonoType::Bytes));
    env.add_type("void".to_string(), PolyType::mono(MonoType::Void));
}

/// 检查单个表达式
///
/// 用于 REPL 或交互式环境
pub fn infer_expression(
    expr: &ast::Expr,
    env: &mut TypeEnvironment,
) -> Result<MonoType, Vec<TypeError>> {
    let mut inferrer = TypeInferrer::new(env.solver());

    let result = inferrer.infer_expr(expr);

    // 求解约束
    env.solver().solve().map_err(|e| {
        e.into_iter()
            .map(|e| TypeError::TypeMismatch {
                expected: e.error.left,
                found: e.error.right,
                span: e.span,
            })
            .collect::<Vec<_>>()
    })?;

    result.map_err(|e| vec![e])
}

/// 检查单个函数定义
pub fn check_function(
    name: &str,
    params: &[ast::Param],
    return_type: Option<&ast::Type>,
    body: &ast::Block,
    is_async: bool,
    env: &mut TypeEnvironment,
) -> Result<middle::FunctionIR, Vec<TypeError>> {
    // 在内部作用域中执行类型检查，确保 checker 在访问 env 之前被 drop
    let (result, checker_errors) = {
        let mut checker = TypeChecker::new(env.solver());
        let result = checker.check_fn_def(name, params, return_type, body, is_async, None, false);
        let errors = checker.errors().to_vec();
        (result, errors)
    };

    // 收集错误
    for error in checker_errors {
        env.errors().add_error(error);
    }

    if env.has_errors() {
        Err(env.get_errors().to_vec())
    } else {
        result.map_err(|e| vec![e])
    }
}
