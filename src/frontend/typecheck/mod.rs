//! 类型检查器模块
//!
//! 实现 YaoXiang 语言的类型检查器，支持：
//! - Hindley-Milner 类型推断
//! - 泛型函数和泛型类型
//! - 完整的错误收集
//! - RFC-004/010/011 支撑

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::{Module, Expr};

// 导入推断模块
pub mod inference;

// 导入检查模块
pub mod checking;

// 导入特化模块
pub mod specialization;

// 导入旧特化模块（包含 GenericSpecializer）
mod specialize;

// 导入特质模块
pub mod traits;

// 导入 GAT 模块
pub mod gat;

// 导入错误处理
pub mod errors;

// 导入测试模块
#[cfg(test)]
mod tests;

// 使用 core 层的类型系统（显式导出以避免 ambiguous glob re-exports）
pub use crate::frontend::core::type_system::{
    MonoType, PolyType, TypeVar, TypeBinding, StructType, EnumType, TypeConstraint,
    TypeConstraintSolver, SendSyncConstraint, SendSyncSolver, TypeMismatch, TypeConstraintError,
    ConstValue, ConstExpr, ConstKind, ConstVarDef,
};

// 重新导出推断、检查、特化等模块
pub use inference::*;
pub use checking::*;
pub use specialization::*;
pub use specialize::*;
pub use gat::*;
pub use errors::*;

// 类型环境
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    pub vars: HashMap<String, PolyType>,
    pub types: HashMap<String, PolyType>,
    pub solver: TypeConstraintSolver,
    pub errors: TypeErrorCollector,
    /// 导入追踪 - 模块导入信息
    pub imports: Vec<ImportInfo>,
    /// 当前模块的导出项
    pub exports: HashSet<String>,
    /// 模块名称
    pub module_name: String,
}

impl TypeEnvironment {
    /// 创建新的类型环境
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建新的类型环境（带模块名）
    pub fn new_with_module(module_name: String) -> Self {
        Self {
            module_name,
            ..Self::default()
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

    /// 获取求解器
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        &mut self.solver
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
}

/// 类型检查器
///
/// 负责模块级类型检查编排，协调前置收集和函数体检查
pub struct TypeChecker {
    /// 当前环境
    env: TypeEnvironment,
    /// 已检查的函数签名（用于递归检测）
    checked_functions: HashMap<String, bool>,
    /// 当前函数的返回类型
    current_return_type: Option<MonoType>,
    /// 泛型函数缓存
    generic_cache: HashMap<String, HashMap<String, PolyType>>,
    /// 函数体检查器
    body_checker: Option<checking::BodyChecker>,
}

impl TypeChecker {
    /// 创建新的类型检查器
    pub fn new(module_name: &str) -> Self {
        let mut env = TypeEnvironment::new_with_module(module_name.to_string());
        add_builtin_types(&mut env);

        Self {
            env,
            checked_functions: HashMap::new(),
            current_return_type: None,
            generic_cache: HashMap::new(),
            body_checker: None,
        }
    }

    /// 获取环境引用
    pub fn env(&mut self) -> &mut TypeEnvironment {
        &mut self.env
    }

    /// 获取模块名称
    pub fn module_name(&self) -> &str {
        &self.env.module_name
    }

    /// 添加错误
    fn add_error(
        &mut self,
        error: TypeError,
    ) {
        self.env.errors.add_error(error);
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        self.env.errors.has_errors()
    }

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.env.add_var(name, poly);
    }

    /// 获取错误列表
    pub fn errors(&self) -> &[TypeError] {
        self.env.errors.errors()
    }

    /// 检查单个语句（委托给 BodyChecker）
    pub fn check_stmt(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Result<(), TypeError> {
        self.body_checker_mut().check_stmt(stmt)
    }

    /// 获取函数体检查器
    fn body_checker(&mut self) -> &mut checking::BodyChecker {
        if self.body_checker.is_none() {
            self.body_checker = Some(checking::BodyChecker::new(self.env.solver()));
        }
        self.body_checker.as_mut().unwrap()
    }

    /// 检查整个模块
    pub fn check_module(
        &mut self,
        module: &Module,
    ) -> Result<TypeCheckResult, Vec<TypeError>> {
        // 第一遍：收集所有类型定义
        for stmt in &module.items {
            if let crate::frontend::core::parser::ast::StmtKind::TypeDef { name, definition } =
                &stmt.kind
            {
                self.add_type_definition(name, definition, stmt.span);
            }
        }

        // 第二遍：收集所有函数签名（使其可被前向引用）
        for stmt in &module.items {
            self.collect_function_signature(stmt);
        }

        // 初始化函数体检查器
        let body_checker = checking::BodyChecker::new(self.env.solver());
        *self.body_checker_mut() = body_checker;

        // 将环境中的变量同步到 body_checker
        for (name, poly) in self.env.vars.clone() {
            self.body_checker_mut().add_var(name, poly);
        }

        // 第三遍：检查所有语句（包括函数体）
        for stmt in &module.items {
            if let Err(e) = self.body_checker_mut().check_stmt(stmt) {
                self.add_error(e);
            }
        }

        // 求解所有约束
        self.env.solver().solve().map_err(|e| {
            e.into_iter()
                .map(|e| TypeError::TypeMismatch {
                    expected: Box::new(e.error.left),
                    found: Box::new(e.error.right),
                    span: e.span,
                })
                .collect::<Vec<_>>()
        })?;

        // 如果有错误，返回所有错误
        if self.has_errors() {
            return Err(self.errors().to_vec());
        }

        // 构建类型检查结果
        let result = TypeCheckResult {
            module_name: self.env.module_name.clone(),
            bindings: self.env.vars.clone(),
        };

        Ok(result)
    }

    /// 获取 body_checker 的可变引用
    fn body_checker_mut(&mut self) -> &mut checking::BodyChecker {
        if self.body_checker.is_none() {
            self.body_checker = Some(checking::BodyChecker::new(self.env.solver()));
        }
        self.body_checker.as_mut().unwrap()
    }

    /// 收集函数签名（第一遍扫描）
    fn collect_function_signature(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) {
        match &stmt.kind {
            crate::frontend::core::parser::ast::StmtKind::Expr(expr) => {
                // 处理函数定义表达式
                if let crate::frontend::core::parser::ast::Expr::FnDef {
                    name,
                    params,
                    return_type,
                    is_async,
                    ..
                } = expr.as_ref()
                {
                    let fn_ty = MonoType::Fn {
                        params: params.iter().map(|p| {
                            p.ty.as_ref()
                                .map(|t| MonoType::from(t.clone()))
                                .unwrap_or_else(|| self.env.solver().new_var())
                        }).collect(),
                        return_type: Box::new(
                            return_type.as_ref()
                                .map(|t| MonoType::from(t.clone()))
                                .unwrap_or_else(|| self.env.solver().new_var())
                        ),
                        is_async: *is_async,
                    };
                    self.env.add_var(name.clone(), PolyType::mono(fn_ty));
                }
                // 处理 Lambda 赋值 (name = (params) => body)
                else if let crate::frontend::core::parser::ast::Expr::BinOp {
                    op: crate::frontend::core::parser::ast::BinOp::Assign,
                    left,
                    right,
                    ..
                } = expr.as_ref()
                {
                    if let crate::frontend::core::parser::ast::Expr::Var(name, _) = left.as_ref() {
                        if let crate::frontend::core::parser::ast::Expr::Lambda { params, .. } = right.as_ref() {
                            let fn_ty = MonoType::Fn {
                                params: params.iter().map(|p| {
                                    p.ty.as_ref()
                                        .map(|t| MonoType::from(t.clone()))
                                        .unwrap_or_else(|| self.env.solver().new_var())
                                }).collect(),
                                return_type: Box::new(self.env.solver().new_var()),
                                is_async: false,
                            };
                            self.env.add_var(name.clone(), PolyType::mono(fn_ty));
                        }
                    }
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Fn {
                name,
                type_annotation,
                params,
                ..
            } => {
                // 处理统一函数语法
                let (param_types, return_type) = if let Some(type_ann) = type_annotation {
                    if let crate::frontend::core::parser::ast::Type::Fn {
                        params: param_tys,
                        return_type,
                    } = type_ann
                    {
                        (
                            param_tys.iter().map(|t| MonoType::from(t.clone())).collect(),
                            MonoType::from(*return_type.clone()),
                        )
                    } else {
                        (
                            params.iter().map(|p| {
                                p.ty.as_ref()
                                    .map(|t| MonoType::from(t.clone()))
                                    .unwrap_or_else(|| self.env.solver().new_var())
                            }).collect(),
                            self.env.solver().new_var(),
                        )
                    }
                } else {
                    (
                        params.iter().map(|p| {
                            p.ty.as_ref()
                                .map(|t| MonoType::from(t.clone()))
                                .unwrap_or_else(|| self.env.solver().new_var())
                        }).collect(),
                        self.env.solver().new_var(),
                    )
                };

                let fn_ty = MonoType::Fn {
                    params: param_types,
                    return_type: Box::new(return_type),
                    is_async: false,
                };
                self.env.add_var(name.clone(), PolyType::mono(fn_ty));
            }
            crate::frontend::core::parser::ast::StmtKind::Use {
                path,
                items: _,
                alias: _,
            } => {
                // 处理 use std.io - 添加 print/println 函数
                if path == "std.io" {
                    let print_ty = MonoType::Fn {
                        params: vec![self.env.solver().new_var()],
                        return_type: Box::new(MonoType::Void),
                        is_async: false,
                    };
                    self.env.add_var("print".to_string(), PolyType::mono(print_ty.clone()));
                    self.env.add_var("println".to_string(), PolyType::mono(print_ty));
                }
            }
            _ => {}
        }
    }

    /// 添加类型定义
    fn add_type_definition(
        &mut self,
        name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
        _span: crate::util::span::Span,
    ) {
        let poly = PolyType::mono(MonoType::from(definition.clone()));
        self.env.add_type(name.to_string(), poly);
    }
}

/// 检查模块
#[allow(unused_variables)]
pub fn check_module(
    ast: &Module,
    env: Option<&mut TypeEnvironment>,
) -> Result<TypeCheckResult, Vec<TypeError>> {
    // 使用 TypeChecker 进行完整的模块检查
    let mut checker = TypeChecker::new("main");

    // 如果提供了外部环境，将其变量和类型导入到 checker 中
    if let Some(ext_env) = env {
        for (name, poly) in &ext_env.vars {
            checker.env().add_var(name.clone(), poly.clone());
        }
        for (name, poly) in &ext_env.types {
            checker.env().add_type(name.clone(), poly.clone());
        }
    }

    // 执行模块检查
    checker.check_module(ast)
}

/// 检查单个表达式
pub fn infer_expression(
    expr: &Expr,
    env: &mut TypeEnvironment,
) -> Result<MonoType, Vec<TypeError>> {
    // 克隆环境变量，避免借用冲突
    let vars_clone = env.vars.clone();
    let mut inferrer = crate::frontend::typecheck::inference::ExprInferrer::new(env.solver());
    // 添加环境中的变量到推断器
    for (name, poly) in vars_clone {
        inferrer.add_var(name, poly);
    }
    match inferrer.infer_expr(expr) {
        Ok(ty) => Ok(ty),
        Err(diagnostic) => {
            // 转换Diagnostic到TypeError
            let type_err = TypeError::InferenceError {
                message: diagnostic.message,
                span: diagnostic.span.unwrap_or_default(),
            };
            Err(vec![type_err])
        }
    }
}

/// 添加内置类型到环境
pub fn add_builtin_types(env: &mut TypeEnvironment) {
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

/// 类型检查结果
#[derive(Debug, Clone, Default)]
pub struct TypeCheckResult {
    pub module_name: String,
    pub bindings: HashMap<String, PolyType>,
}

/// 导入信息
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub path: String,
    pub items: Option<Vec<String>>,
    pub alias: Option<String>,
}
