//! 类型检查器模块
//!
//! 实现 YaoXiang 语言的类型检查器，支持：
//! - Hindley-Milner 类型推断
//! - 泛型函数和泛型类型
//! - 完整的错误收集
//! - RFC-004/010/011 支撑

use std::collections::{HashMap, HashSet};

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
// 注意：不导出 BinOp/UnOp 以避免与 parser 冲突
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
    pub vars: HashMap<String, crate::frontend::core::type_system::PolyType>,
    pub types: HashMap<String, crate::frontend::core::type_system::PolyType>,
    pub solver: crate::frontend::core::type_system::TypeConstraintSolver,
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
        poly: crate::frontend::core::type_system::PolyType,
    ) {
        self.vars.insert(name, poly);
    }

    /// 获取变量类型
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&crate::frontend::core::type_system::PolyType> {
        self.vars.get(name)
    }

    /// 获取求解器
    pub fn solver(&mut self) -> &mut crate::frontend::core::type_system::TypeConstraintSolver {
        &mut self.solver
    }

    /// 添加类型定义
    pub fn add_type(
        &mut self,
        name: String,
        poly: crate::frontend::core::type_system::PolyType,
    ) {
        self.types.insert(name, poly);
    }

    /// 获取类型定义
    pub fn get_type(
        &self,
        name: &str,
    ) -> Option<&crate::frontend::core::type_system::PolyType> {
        self.types.get(name)
    }
}

/// 类型检查器
///
/// 负责检查模块、函数和语句的类型正确性
pub struct TypeChecker {
    /// 当前环境
    env: TypeEnvironment,
    /// 已检查的函数签名（用于递归检测）
    checked_functions: HashMap<String, bool>,
    /// 当前函数的返回类型
    current_return_type: Option<MonoType>,
    /// 泛型函数缓存
    generic_cache: HashMap<String, HashMap<String, PolyType>>,
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

    /// 检查整个模块
    pub fn check_module(
        &mut self,
        module: &crate::frontend::core::parser::ast::Module,
    ) -> Result<TypeCheckResult, Vec<TypeError>> {
        // 首先收集所有类型定义
        for stmt in &module.items {
            if let crate::frontend::core::parser::ast::StmtKind::TypeDef { name, definition } =
                &stmt.kind
            {
                self.add_type_definition(name, definition, stmt.span);
            }
        }

        // 然后检查所有语句
        for stmt in &module.items {
            if let Err(e) = self.check_stmt(stmt) {
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

    /// 检查语句
    pub fn check_stmt(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Result<(), TypeError> {
        match &stmt.kind {
            crate::frontend::core::parser::ast::StmtKind::Expr(expr) => {
                // 检查函数定义
                if let crate::frontend::core::parser::ast::Expr::FnDef {
                    name,
                    params,
                    return_type,
                    body,
                    is_async,
                    span: _,
                } = expr.as_ref()
                {
                    // 有返回类型标注表示有完整类型签名
                    let is_annotated = return_type.is_some();
                    self.check_fn_def(
                        name,
                        params,
                        return_type.as_ref(),
                        body,
                        *is_async,
                        None,
                        is_annotated,
                    )?;
                    Ok(())
                } else if let crate::frontend::core::parser::ast::Expr::BinOp {
                    op: crate::frontend::core::parser::ast::BinOp::Assign,
                    left,
                    right: _,
                    ..
                } = expr.as_ref()
                {
                    // 对于赋值表达式，先将变量添加到作用域，再推断类型
                    if let crate::frontend::core::parser::ast::Expr::Var(name, _) = left.as_ref() {
                        // 为变量创建类型变量并添加到作用域
                        let ty = self.env.solver().new_var();
                        let poly = PolyType::mono(ty);
                        self.env.add_var(name.clone(), poly);
                    }
                    // 推断整个赋值表达式
                    self.check_expr(expr)?;
                    Ok(())
                } else {
                    self.check_expr(expr)?;
                    Ok(())
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Fn {
                name,
                type_annotation,
                params,
                body: (stmts, expr),
            } => {
                // 检查是否与已存在的结构体类型同名
                if let Some(existing) = self.env.get_var(name) {
                    if let MonoType::Struct(_) = existing.body {
                        return Err(TypeError::UnknownVariable {
                            name: format!("'{}' is already defined as a struct type", name),
                            span: stmt.span,
                        });
                    }
                }

                // Create the function body block
                let body = crate::frontend::core::parser::ast::Block {
                    stmts: stmts.clone(),
                    expr: expr.clone(),
                    span: stmt.span,
                };

                // For unified syntax, we need to handle the type annotation specially
                // We'll convert this to a FnDef expression and delegate to the Expr handler
                #[allow(clippy::collapsible_match)]
                if let Some(ref type_annotation) = type_annotation {
                    if let crate::frontend::core::parser::ast::Type::Fn {
                        params: _,
                        return_type,
                    } = type_annotation
                    {
                        // Create a FnDef expression from the unified syntax
                        let fn_def_expr = crate::frontend::core::parser::ast::Expr::FnDef {
                            name: name.clone(),
                            params: params.clone(),
                            return_type: Some(*return_type.clone()),
                            body: Box::new(body),
                            is_async: false,
                            span: stmt.span,
                        };

                        // Delegate to the Expr handler for FnDef
                        let _ = self.check_expr(&fn_def_expr)?;
                        return Ok(());
                    }
                }

                // 如果没有类型注解，使用原有逻辑
                let is_async = false;
                let return_type = None;
                let is_annotated = false;

                self.check_fn_def(
                    name,
                    params,
                    return_type.as_ref(),
                    &body,
                    is_async,
                    None,
                    is_annotated,
                )?;
                Ok(())
            }
            _ => {
                // TODO: 实现其他语句类型的检查
                Ok(())
            }
        }
    }

    /// 检查表达式
    pub fn check_expr(
        &mut self,
        expr: &crate::frontend::core::parser::ast::Expr,
    ) -> Result<MonoType, TypeError> {
        // 克隆环境变量避免借用冲突
        let vars_clone = self.env.vars.clone();
        let mut inferrer =
            crate::frontend::typecheck::inference::ExprInferrer::new(self.env.solver());
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
                Err(type_err)
            }
        }
    }

    /// 检查函数定义
    #[allow(clippy::too_many_arguments)]
    fn check_fn_def(
        &mut self,
        name: &str,
        _params: &[crate::frontend::core::parser::ast::Param],
        _return_type: Option<&crate::frontend::core::parser::ast::Type>,
        _body: &crate::frontend::core::parser::ast::Block,
        _is_async: bool,
        _span: Option<crate::util::span::Span>,
        _is_annotated: bool,
    ) -> Result<(), TypeError> {
        // 检查递归调用
        if self.checked_functions.contains_key(name) {
            return Err(TypeError::RecursiveType {
                name: name.to_string(),
                span: crate::util::span::Span::default(),
            });
        }

        // 标记函数为已检查
        self.checked_functions.insert(name.to_string(), true);

        // TODO: 实现完整的函数类型检查逻辑
        // 这包括：
        // 1. 检查参数类型
        // 2. 检查返回类型
        // 3. 检查函数体

        // 临时实现：假设函数检查通过
        Ok(())
    }
}

/// 检查模块
#[allow(unused_variables)]
pub fn check_module(
    _ast: &crate::frontend::core::parser::ast::Module,
    env: Option<&mut TypeEnvironment>,
) -> Result<TypeCheckResult, Vec<TypeError>> {
    let env = env.unwrap_or_else(|| {
        let mut new_env = TypeEnvironment::new();
        add_builtin_types(&mut new_env);
        Box::leak(Box::new(new_env))
    });

    // TODO: 使用新架构实现模块检查
    Ok(TypeCheckResult::default())
}

/// 检查单个表达式
pub fn infer_expression(
    expr: &crate::frontend::core::parser::ast::Expr,
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
}

/// 生成 IR
pub fn generate_ir(
    _ast: &crate::frontend::core::parser::ast::Module,
    _result: &TypeCheckResult,
) -> Result<crate::middle::ModuleIR, Vec<TypeError>> {
    // TODO: 实现 IR 生成
    Ok(crate::middle::ModuleIR::default())
}

/// 类型检查结果
#[derive(Debug, Clone, Default)]
pub struct TypeCheckResult {
    pub module_name: String,
    pub bindings: std::collections::HashMap<String, crate::frontend::core::type_system::PolyType>,
}

/// 导入信息
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub path: String,
    pub items: Option<Vec<String>>,
    pub alias: Option<String>,
}
