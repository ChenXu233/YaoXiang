//! 语句和函数类型检查
//!
//! 实现语句的类型检查和函数定义的类型验证

use super::errors::{TypeError, TypeResult};
use super::infer::TypeInferrer;
use super::types::{MonoType, PolyType, TypeConstraintSolver};
use super::super::parser::ast;
use super::super::lexer::tokens::Literal;
use crate::middle;
use crate::util::span::Span;
use std::collections::HashMap;

/// 类型检查器
///
/// 负责检查模块、函数和语句的类型正确性
#[derive(Debug)]
pub struct TypeChecker<'a> {
    /// 类型推断器
    inferrer: TypeInferrer<'a>,
    /// 已检查的函数签名（用于递归检测）
    checked_functions: HashMap<String, bool>,
    /// 当前函数的返回类型
    current_return_type: Option<MonoType>,
    /// 泛型函数缓存
    generic_cache: HashMap<String, HashMap<String, PolyType>>,
    /// 收集的错误
    errors: Vec<TypeError>,
}

impl<'a> TypeChecker<'a> {
    /// 创建新的类型检查器
    pub fn new(solver: &'a mut TypeConstraintSolver) -> Self {
        TypeChecker {
            inferrer: TypeInferrer::new(solver),
            checked_functions: HashMap::new(),
            current_return_type: None,
            generic_cache: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// 获取错误列表
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 添加错误
    fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }

    // =========================================================================
    // 模块检查
    // =========================================================================

    /// 检查整个模块
    pub fn check_module(&mut self, module: &ast::Module) -> Result<middle::ModuleIR, Vec<TypeError>> {
        // 首先收集所有类型定义
        for stmt in &module.items {
            if let ast::StmtKind::TypeDef { name, definition } = &stmt.kind {
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
        self.inferrer.solver().solve().map_err(|e| {
            e.into_iter()
                .map(|e| TypeError::TypeMismatch {
                    expected: e.error.left,
                    found: e.error.right,
                    span: e.span,
                })
                .collect::<Vec<_>>()
        })?;

        // 如果有错误，返回所有错误
        if self.has_errors() {
            return Err(self.errors.clone());
        }

        // 生成 IR（简化版本）
        self.generate_module_ir(module)
    }

    /// 添加类型定义
    fn add_type_definition(&mut self, name: &str, definition: &ast::Type, _span: Span) {
        let poly = PolyType::mono(MonoType::from(definition.clone()));
        self.inferrer.add_var(name.to_string(), poly);
    }

    // =========================================================================
    // 语句检查
    // =========================================================================

    /// 检查语句
    pub fn check_stmt(&mut self, stmt: &ast::Stmt) -> TypeResult<()> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                if let ast::Expr::FnDef {
                    name,
                    params,
                    return_type,
                    body,
                    is_async,
                    span: _,
                } = expr.as_ref()
                {
                    self.check_fn_def(name, params, return_type.as_ref(), body, *is_async)?;
                    Ok(())
                } else {
                    self.inferrer.infer_expr(expr)?;
                    Ok(())
                }
            }
            ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                is_mut: _,
            } => self.check_var(name, type_annotation.as_ref(), initializer.as_deref(), stmt.span),
            ast::StmtKind::For { var, iterable, body, .. } => {
                // 类型检查 for 循环
                // 1. 推断 iterable 的类型（应该是可迭代的）
                let _iter_ty = self.inferrer.infer_expr(iterable)?;

                // 2. 在循环体中，var 的类型取决于 iterable 的元素类型
                // 暂时添加一个类型变量
                let var_ty = self.inferrer.solver().new_var();
                let poly = PolyType::mono(var_ty);
                self.inferrer.add_var(var.clone(), poly);

                // 3. 类型检查循环体
                let _body_ty = self.inferrer.infer_block(body)?;
                Ok(())
            }
            ast::StmtKind::TypeDef { name, definition } => {
                self.check_type_def(name, definition, stmt.span)
            }
            ast::StmtKind::Module { name, items } => self.check_module_alias(name, items, stmt.span),
            ast::StmtKind::Use {
                path,
                items,
                alias,
            } => self.check_use(path, items.as_deref(), alias.as_deref(), stmt.span),
        }
    }

    /// 检查变量声明: `name[: type] [= expr]`
    fn check_var(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        initializer: Option<&ast::Expr>,
        span: Span,
    ) -> TypeResult<()> {
        if let Some(init) = initializer {
            let init_ty = self.inferrer.infer_expr(init)?;

            if let Some(ann) = type_annotation {
                let ann_ty = MonoType::from(ann.clone());
                self.inferrer
                    .solver()
                    .add_constraint(init_ty.clone(), ann_ty, span);
            }

            // 泛化 initializer 的类型
            let poly = self.inferrer.solver().generalize(&init_ty);
            self.inferrer.add_var(name.to_string(), poly);
        } else if let Some(ann) = type_annotation {
            // 没有初始化时，使用类型注解
            let ty = MonoType::from(ann.clone());
            self.inferrer.add_var(name.to_string(), PolyType::mono(ty));
        } else {
            // 没有任何信息，创建新类型变量
            let ty = self.inferrer.solver().new_var();
            self.inferrer.add_var(name.to_string(), PolyType::mono(ty));
        }

        Ok(())
    }

    /// 检查类型定义
    fn check_type_def(&mut self, name: &str, definition: &ast::Type, _span: Span) -> TypeResult<()> {
        let ty = MonoType::from(definition.clone());
        self.inferrer
            .add_var(name.to_string(), PolyType::mono(ty));
        Ok(())
    }

    /// 检查模块别名
    fn check_module_alias(
        &mut self,
        _name: &str,
        _items: &[ast::Stmt],
        _span: Span,
    ) -> TypeResult<()> {
        // TODO: 实现模块别名检查
        Ok(())
    }

    /// 检查 use 语句
    fn check_use(
        &mut self,
        _path: &str,
        _items: Option<&[String]>,
        _alias: Option<&str>,
        _span: Span,
    ) -> TypeResult<()> {
        // TODO: 实现 use 语句检查
        Ok(())
    }

    // =========================================================================
    // 函数检查
    // =========================================================================

    /// 检查函数定义
    pub fn check_fn_def(
        &mut self,
        name: &str,
        params: &[ast::Param],
        return_type: Option<&ast::Type>,
        body: &ast::Block,
        is_async: bool,
    ) -> TypeResult<middle::FunctionIR> {
        // 保存当前返回类型
        let prev_return_type = self.current_return_type.take();

        // 创建函数类型
        let param_types: Vec<MonoType> = params
            .iter()
            .map(|p| {
                if let Some(ty) = &p.ty {
                    MonoType::from(ty.clone())
                } else {
                    self.inferrer.solver().new_var()
                }
            })
            .collect();

        let return_ty = return_type
            .map(|t| MonoType::from(t.clone()))
            .unwrap_or_else(|| self.inferrer.solver().new_var());

        // 设置当前返回类型
        self.current_return_type = Some(return_ty.clone());

        // 泛型参数处理
        let _generic_params: Vec<MonoType> = params
            .iter()
            .filter_map(|p| p.ty.as_ref())
            .filter_map(|t| {
                if let ast::Type::Generic { name: _, args: _ } = t {
                    Some(MonoType::from(t.clone()))
                } else {
                    None
                }
            })
            .collect();

        // 进入函数体作用域
        self.inferrer.enter_scope();

        // 添加参数到作用域
        for (param, param_ty) in params.iter().zip(param_types.iter()) {
            // 如果参数有泛型类型，需要泛化
            let poly = self.inferrer.solver().generalize(param_ty);
            self.inferrer.add_var(param.name.clone(), poly);
        }

        // 推断函数体
        let body_ty = self.inferrer.infer_block(body)?;

        // 检查返回类型
        if let Some(expected) = &self.current_return_type {
            self.inferrer
                .solver()
                .add_constraint(body_ty, expected.clone(), body.span);
        }

        // 退出函数体作用域
        self.inferrer.exit_scope();

        // 恢复之前的返回类型
        self.current_return_type = prev_return_type;

        // 生成函数 IR
        let fn_ir = middle::FunctionIR {
            name: name.to_string(),
            params: param_types,
            return_type: return_ty,
            is_async,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: 0,
        };

        Ok(fn_ir)
    }

    /// 检查函数调用
    fn check_fn_call(
        &mut self,
        func: &ast::Expr,
        args: &[ast::Expr],
        span: Span,
    ) -> TypeResult<MonoType> {
        self.inferrer.infer_call(func, args, span)
    }

    // =========================================================================
    // IR 生成
    // =========================================================================

    /// 生成模块 IR
    fn generate_module_ir(&self, _module: &ast::Module) -> Result<middle::ModuleIR, Vec<TypeError>> {
        Ok(middle::ModuleIR {
            types: Vec::new(),
            constants: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
        })
    }
}

// =========================================================================
// 类型环境扩展
// =========================================================================

/// 扩展类型环境，支持更多操作
#[derive(Debug, Default)]
pub struct ExtendedTypeEnvironment {
    /// 变量绑定
    vars: HashMap<String, PolyType>,
    /// 类型定义
    types: HashMap<String, PolyType>,
    /// 求解器
    solver: TypeConstraintSolver,
    /// 错误列表
    errors: Vec<TypeError>,
}

impl ExtendedTypeEnvironment {
    /// 创建新的扩展类型环境
    pub fn new() -> Self {
        ExtendedTypeEnvironment {
            vars: HashMap::new(),
            types: HashMap::new(),
            solver: TypeConstraintSolver::new(),
            errors: Vec::new(),
        }
    }

    /// 添加变量
    pub fn add_var(&mut self, name: String, poly: PolyType) {
        self.vars.insert(name, poly);
    }

    /// 添加类型
    pub fn add_type(&mut self, name: String, poly: PolyType) {
        self.types.insert(name, poly);
    }

    /// 获取变量
    pub fn get_var(&self, name: &str) -> Option<&PolyType> {
        self.vars.get(name)
    }

    /// 获取类型
    pub fn get_type(&self, name: &str) -> Option<&PolyType> {
        self.types.get(name)
    }

    /// 添加错误
    pub fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }

    /// 获取错误
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// 是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// 检查字面量类型
pub fn infer_literal_type(lit: &Literal) -> MonoType {
    match lit {
        Literal::Int(_) => MonoType::Int(64),
        Literal::Float(_) => MonoType::Float(64),
        Literal::Bool(_) => MonoType::Bool,
        Literal::Char(_) => MonoType::Char,
        Literal::String(_) => MonoType::String,
    }
}

/// 获取二元运算的结果类型
pub fn binop_result_type(op: &ast::BinOp, left: &MonoType, right: &MonoType) -> Option<MonoType> {
    match op {
        ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul | ast::BinOp::Div | ast::BinOp::Mod => {
            if left == right && left.is_numeric() {
                Some(left.clone())
            } else {
                None
            }
        }
        ast::BinOp::Eq | ast::BinOp::Neq | ast::BinOp::Lt | ast::BinOp::Le | ast::BinOp::Gt | ast::BinOp::Ge => {
            if left == right {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
        ast::BinOp::And | ast::BinOp::Or => {
            if *left == MonoType::Bool && *right == MonoType::Bool {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
        ast::BinOp::Assign => Some(MonoType::Void),
        ast::BinOp::Range => None, // 范围运算暂时不支持类型检查
    }
}

/// 获取一元运算的结果类型
pub fn unop_result_type(op: &ast::UnOp, expr: &MonoType) -> Option<MonoType> {
    match op {
        ast::UnOp::Neg | ast::UnOp::Pos => {
            if expr.is_numeric() {
                Some(expr.clone())
            } else {
                None
            }
        }
        ast::UnOp::Not => {
            if *expr == MonoType::Bool {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
    }
}
