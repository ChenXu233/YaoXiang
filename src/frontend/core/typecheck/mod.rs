//! 类型检查器模块
//!
//! 实现 YaoXiang 语言的类型检查器，支持：
//! - Hindley-Milner 类型推断
//! - 泛型函数和泛型类型
//! - 完整的错误收集
//! - RFC-004/010/011 支撑

use crate::frontend::core::parser::ast::{Module, Expr};

// ============ 子模块声明 ============

// 导入推断模块
pub mod inference;

// 导入特质模块
pub mod traits;

// 导入重载解析模块
pub mod overload;

// 导入类型求值器模块
pub mod type_eval;

// 语义信息数据库
pub mod semantic_db;

// 死代码分析器
pub mod dead_code;

// spawn 放置检查
pub mod spawn_placement;

// 类型环境
pub mod environment;

// 类型检查器
pub mod checker;

// 签名解析
pub mod signature;
// 类型定义
pub mod types;

// ============ 测试模块 ============

#[cfg(test)]
mod tests;

// ============ 类型导出 ============

// 使用 core 层的类型系统（显式导出以避免 ambiguous glob re-exports）
pub use crate::frontend::core::types::base::{
    MonoType, PolyType, TypeVar, TypeBinding, StructType, EnumType, TypeConstraint,
    TypeConstraintSolver, TypeMismatch, TypeConstraintError, ConstValue, ConstExpr, ConstKind,
    ConstVarDef, UniverseLevel,
};

// 重新导出子模块
pub use environment::*;
pub use inference::*;
pub use overload::*;
pub use type_eval::*;
pub use checker::*;
pub use signature::*;
pub use types::*;

// 导入诊断系统
pub use crate::util::diagnostic::{Diagnostic, ErrorCollector, ErrorCodeDefinition, I18nRegistry};

/// 类型推断结果
pub type TypeResult<T> = Result<T, Diagnostic>;

/// 类型错误收集器
pub type TypeErrorCollector = ErrorCollector<Diagnostic>;

// ============ 入口函数 ============

/// 检查模块
#[allow(unused_variables)]
pub fn check_module(
    ast: &Module,
    env: &mut Option<environment::TypeEnvironment>,
) -> Result<types::TypeCheckResult, Vec<Diagnostic>> {
    check_module_inner(ast, env, false)
}

/// 检查模块（收集所有错误模式）
///
/// 与 `check_module` 相同，但启用错误收集模式。
/// 类型检查器会尽可能多地收集错误，适用于 LSP 诊断。
#[allow(unused_variables)]
pub fn check_module_collect_all(
    ast: &Module,
    env: &mut Option<environment::TypeEnvironment>,
) -> Result<types::TypeCheckResult, Vec<Diagnostic>> {
    check_module_inner(ast, env, true)
}

/// 模块检查内部实现
#[allow(unused_variables)]
fn check_module_inner(
    ast: &Module,
    env: &mut Option<environment::TypeEnvironment>,
    collect_all: bool,
) -> Result<types::TypeCheckResult, Vec<Diagnostic>> {
    // 使用 TypeChecker 进行完整的模块检查
    let mut checker = checker::TypeChecker::new("main");

    // 如果提供了外部环境，将其变量和类型导入到 checker 中
    if let Some(ref mut ext_env) = env {
        for (name, poly) in &ext_env.vars {
            checker.env().add_var(name.clone(), poly.clone());
        }
        for (name, poly) in &ext_env.types {
            checker.env().add_type(name.clone(), poly.clone());
        }
    }

    // 执行模块检查
    let result = if collect_all {
        checker.check_module_collect_all(ast)?
    } else {
        checker.check_module(ast)?
    };

    // 将 exports 和 method_bindings 导回传入的环境
    if let Some(ref mut ext_env) = env {
        ext_env.exports = checker.env().exports.clone();
        ext_env.method_bindings = checker.env().method_bindings.clone();
    }

    Ok(result)
}

/// 检查单个表达式
pub fn infer_expression(
    expr: &Expr,
    env: &mut environment::TypeEnvironment,
) -> Result<MonoType, Vec<Diagnostic>> {
    // 创建共享 ScopeManager 并添加环境变量
    let mut scope = inference::ScopeManager::new();
    for (name, poly) in env.vars.clone() {
        scope.add_var(name, poly, false);
    }
    let overload_candidates_clone = env.overload_candidates.clone();
    let native_signatures_clone = env.native_signatures.clone();
    let mut inferrer = inference::ExpressionInferrer::with_native_signatures(
        &mut scope,
        env.solver(),
        &overload_candidates_clone,
        &native_signatures_clone,
    );
    inferrer.infer_expr(expr).map_err(|diag| vec![diag])
}

/// 添加内置类型到环境
pub fn add_builtin_types(env: &mut environment::TypeEnvironment) {
    env.types
        .insert("int".to_string(), PolyType::mono(MonoType::Int(32)));
    env.types
        .insert("float".to_string(), PolyType::mono(MonoType::Float(64)));
    env.types
        .insert("bool".to_string(), PolyType::mono(MonoType::Bool));
    env.types
        .insert("string".to_string(), PolyType::mono(MonoType::String));
    env.types
        .insert("void".to_string(), PolyType::mono(MonoType::Void));
    env.types
        .insert("char".to_string(), PolyType::mono(MonoType::Char));
}

/// 注册标准库 native 函数类型签名到类型环境
///
/// 这些签名用于类型检查 `native("...")` 表达式，确保调用签名匹配。
/// 通过 ModuleRegistry 自动发现所有 std 模块的 native 函数。
pub fn add_native_function_types(env: &mut environment::TypeEnvironment) {
    use crate::frontend::module::registry::ModuleRegistry;
    use crate::frontend::module::ExportKind;

    let registry = ModuleRegistry::with_std();

    // 遍历所有 std 子模块，自动注册导出的函数
    for submodule_name in registry.std_submodule_names() {
        let module_path = format!("std.{}", submodule_name);
        if let Some(module) = registry.get(&module_path) {
            for export in module.exports.values() {
                // 使用签名字符串解析出正确的函数类型
                let fn_ty = match export.kind {
                    ExportKind::Function | ExportKind::Constant => {
                        signature::parse_signature(&export.signature, env)
                    }
                    _ => continue,
                };

                // 注册完全限定名
                env.native_signatures
                    .insert(export.full_path.clone(), fn_ty.clone());

                // 注册短名称
                env.native_signatures.insert(export.name.clone(), fn_ty);
            }
        }
    }

    // 同时将这些 native 函数注册为变量，使其可在类型推断时查找
    for (name, sig) in &env.native_signatures.clone() {
        env.add_var(name.clone(), PolyType::mono(sig.clone()));
    }
}

/// 添加标准库 traits 到环境
pub fn add_std_traits(env: &mut environment::TypeEnvironment) {
    // 初始化标准库 trait 定义
    traits::std_traits::init_std_traits(&mut env.trait_table);

    // 初始化 primitive 类型的 trait 实现
    traits::std_traits::init_primitive_impls(&mut env.trait_table);
}
