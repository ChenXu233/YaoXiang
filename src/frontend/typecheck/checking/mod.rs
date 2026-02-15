//! 类型检查模块
//!
//! 负责检查模块、函数和语句的类型正确性

pub mod assignment;
pub mod bounds;
pub mod compatibility;
pub mod subtyping;

// 重新导出
pub use subtyping::SubtypeChecker;
pub use assignment::AssignmentChecker;
pub use compatibility::CompatibilityChecker;
pub use bounds::BoundsChecker;

pub use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::ErrorCodeDefinition;

use std::collections::HashMap;
use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::parser::ast::{Stmt, Expr, Param, Block};

/// 函数体检查器
///
/// 负责检查函数体中的语句和表达式的类型正确性
pub struct BodyChecker {
    /// 约束求解器
    solver: TypeConstraintSolver,
    /// 变量环境
    vars: HashMap<String, PolyType>,
    /// 已检查的函数
    checked_functions: HashMap<String, bool>,
    /// 重载候选存储
    overload_candidates:
        HashMap<String, Vec<crate::frontend::typecheck::overload::OverloadCandidate>>,
}

impl BodyChecker {
    /// 创建新的函数体检查器
    pub fn new(solver: &mut TypeConstraintSolver) -> Self {
        Self {
            solver: solver.clone(),
            vars: HashMap::new(),
            checked_functions: HashMap::new(),
            overload_candidates: HashMap::new(),
        }
    }

    /// 获取求解器
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        &mut self.solver
    }

    /// 添加变量
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.vars.insert(name, poly);
    }

    /// 获取变量
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.vars.get(name)
    }

    /// 克隆变量环境
    pub fn vars(&self) -> &HashMap<String, PolyType> {
        &self.vars
    }

    /// 检查函数定义
    pub fn check_fn_def(
        &mut self,
        name: &str,
        params: &[Param],
        body: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        // 检查是否已经检查过
        if self.checked_functions.contains_key(name) {
            return Ok(());
        }

        // 标记为已检查
        self.checked_functions.insert(name.to_string(), true);

        // 添加参数到环境
        for param in params {
            let param_ty = param
                .ty
                .as_ref()
                .map(|t| MonoType::from(t.clone()))
                .unwrap_or_else(|| self.solver.new_var());
            self.vars
                .insert(param.name.clone(), PolyType::mono(param_ty));
        }

        // 检查函数体语句
        for stmt in &body.stmts {
            self.check_stmt(stmt)?;
        }

        // 检查返回表达式
        if let Some(expr) = &body.expr {
            self.check_expr(expr)?;
        }

        Ok(())
    }

    /// 检查语句
    pub fn check_stmt(
        &mut self,
        stmt: &Stmt,
    ) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            crate::frontend::core::parser::ast::StmtKind::Expr(expr) => self.check_expr_stmt(expr),
            crate::frontend::core::parser::ast::StmtKind::Fn {
                name,
                generic_params: _,
                type_annotation,
                params,
                body: (stmts, expr),
                is_pub: _,
            } => {
                let body = Block {
                    stmts: stmts.to_vec(),
                    expr: expr.clone(),
                    span: stmt.span,
                };
                self.check_fn_stmt(
                    name,
                    type_annotation.as_ref(),
                    params,
                    stmts,
                    body,
                    stmt.span,
                )
            }
            crate::frontend::core::parser::ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                ..
            } => self.check_var_stmt(name, type_annotation.as_ref(), initializer.as_deref()),
            crate::frontend::core::parser::ast::StmtKind::For {
                var,
                iterable,
                body,
                ..
            } => self.check_for_stmt(var, iterable, body),
            crate::frontend::core::parser::ast::StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                span,
            } => {
                // 转换 Vec<...> 到 slice
                let elif_refs: Vec<(&Expr, &Block)> = elif_branches
                    .iter()
                    .map(|(e, b)| (e.as_ref(), b.as_ref()))
                    .collect();
                self.check_if_stmt(
                    condition,
                    then_branch,
                    &elif_refs,
                    else_branch.as_deref(),
                    *span,
                )
            }
            crate::frontend::core::parser::ast::StmtKind::Use { .. } => Ok(()),
            crate::frontend::core::parser::ast::StmtKind::TypeDef { .. } => Ok(()),
            _ => Ok(()),
        }
    }

    /// 检查表达式语句
    fn check_expr_stmt(
        &mut self,
        expr: &Expr,
    ) -> Result<(), Box<Diagnostic>> {
        match expr {
            Expr::FnDef {
                name, params, body, ..
            } => {
                self.check_fn_def(name, params, body)?;
                Ok(())
            }
            Expr::BinOp {
                op: crate::frontend::core::parser::ast::BinOp::Assign,
                left,
                ..
            } => {
                if let Expr::Var(name, _) = left.as_ref() {
                    let ty = self.solver.new_var();
                    self.vars.insert(name.clone(), PolyType::mono(ty));
                }
                self.check_expr(expr)?;
                Ok(())
            }
            _ => {
                self.check_expr(expr)?;
                Ok(())
            }
        }
    }

    /// 检查函数语句
    fn check_fn_stmt(
        &mut self,
        name: &str,
        type_annotation: Option<&crate::frontend::core::parser::ast::Type>,
        params: &[Param],
        _stmts: &[Stmt],
        body: Block,
        _span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        // 检查是否与结构体重名
        if let Some(existing) = self.vars.get(name) {
            if let MonoType::Struct(_) = &existing.body {
                return Err(Box::new(
                    ErrorCodeDefinition::duplicate_definition(name)
                        .at(_span)
                        .build(),
                ));
            }
        }

        // 首先添加函数参数到变量环境
        for param in params {
            let param_ty = param
                .ty
                .as_ref()
                .map(|t| MonoType::from(t.clone()))
                .unwrap_or_else(|| self.solver.new_var());
            self.vars
                .insert(param.name.clone(), PolyType::mono(param_ty));
        }

        // 将函数自身注册到变量环境中（支持嵌套函数的前向引用和递归调用）
        if let Some(crate::frontend::core::parser::ast::Type::Fn {
            params: param_types,
            return_type,
        }) = type_annotation
        {
            let fn_param_types: Vec<MonoType> = param_types
                .iter()
                .map(|t| MonoType::from(t.clone()))
                .collect();
            let fn_return_type = MonoType::from(*return_type.clone());
            let fn_type = MonoType::Fn {
                params: fn_param_types,
                return_type: Box::new(fn_return_type),
                is_async: false,
            };
            self.vars.insert(name.to_string(), PolyType::mono(fn_type));
        }

        // 处理类型注解
        if let Some(crate::frontend::core::parser::ast::Type::Fn { return_type, .. }) =
            type_annotation
        {
            let fn_def_expr = Expr::FnDef {
                name: name.to_string(),
                params: params.to_vec(),
                return_type: Some(*return_type.clone()),
                body: Box::new(body.clone()),
                is_async: false,
                span: _span,
            };
            // 通过 check_expr 验证类型约束，但不提前返回
            let _ = self.check_expr(&fn_def_expr);
        }

        // 始终通过 check_fn_def 处理函数体，以收集局部变量类型
        self.check_fn_def(name, params, &body)
    }

    /// 检查变量语句
    fn check_var_stmt(
        &mut self,
        name: &str,
        type_annotation: Option<&crate::frontend::core::parser::ast::Type>,
        initializer: Option<&Expr>,
    ) -> Result<(), Box<Diagnostic>> {
        let ty = match (initializer, type_annotation) {
            (Some(init_expr), _) => self.check_expr(init_expr)?,
            (None, Some(type_ann)) => MonoType::from(type_ann.clone()),
            (None, None) => self.solver.new_var(),
        };
        self.vars.insert(name.to_string(), PolyType::mono(ty));
        Ok(())
    }

    /// 检查 for 语句
    fn check_for_stmt(
        &mut self,
        var: &str,
        iterable: &Expr,
        body: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        let iter_ty = self.check_expr(iterable)?;
        let elem_ty = match iter_ty {
            MonoType::List(elem) => *elem,
            MonoType::String => MonoType::Char,
            _ => self.solver.new_var(),
        };
        self.vars.insert(var.to_string(), PolyType::mono(elem_ty));

        for stmt in &body.stmts {
            self.check_stmt(stmt)?;
        }
        if let Some(expr) = &body.expr {
            self.check_expr(expr)?;
        }
        Ok(())
    }

    /// 检查 if 语句
    fn check_if_stmt(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        elif_branches: &[(&Expr, &Block)],
        else_branch: Option<&Block>,
        _stmt_span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        let cond_ty = self.check_expr(condition)?;
        if cond_ty != MonoType::Bool {
            return Err(Box::new(
                ErrorCodeDefinition::type_mismatch("bool", &format!("{}", cond_ty))
                    .at(_stmt_span)
                    .build(),
            ));
        }

        self.check_block(then_branch)?;

        for (elif_cond, _) in elif_branches {
            let elif_cond_ty = self.check_expr(elif_cond)?;
            if elif_cond_ty != MonoType::Bool {
                return Err(Box::new(
                    ErrorCodeDefinition::type_mismatch("bool", &format!("{}", elif_cond_ty))
                        .at(_stmt_span)
                        .build(),
                ));
            }
        }

        for (_, elif_block) in elif_branches {
            self.check_block(elif_block)?;
        }

        if let Some(else_block) = else_branch {
            self.check_block(else_block)?;
        }

        Ok(())
    }

    /// 检查代码块
    fn check_block(
        &mut self,
        block: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        for stmt in &block.stmts {
            self.check_stmt(stmt)?;
        }
        if let Some(expr) = &block.expr {
            self.check_expr(expr)?;
        }
        Ok(())
    }

    /// 检查表达式
    pub fn check_expr(
        &mut self,
        expr: &Expr,
    ) -> Result<MonoType, Box<Diagnostic>> {
        let vars_clone = self.vars.clone();
        let overload_candidates_clone = self.overload_candidates.clone();
        let mut inferrer = crate::frontend::typecheck::inference::ExprInferrer::new(
            &mut self.solver,
            &overload_candidates_clone,
        );

        for (name, poly) in vars_clone {
            inferrer.add_var(name, poly);
        }

        inferrer.infer_expr(expr).map_err(Box::new)
    }
}
