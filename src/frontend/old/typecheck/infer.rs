//! 表达式类型推断
//!
//! 使用 Hindley-Milner 算法推断表达式的类型

#![allow(clippy::result_large_err)]

use super::super::lexer::tokens::Literal;
use super::super::parser::ast::{self, BinOp, UnOp};
use super::errors::{TypeError, TypeResult};
use super::types::{MonoType, PolyType, SendSyncSolver, TypeConstraintSolver};
use crate::util::span::Span;
use crate::tlog;
use crate::util::i18n::MSG;
use std::collections::HashMap;

/// 类型推断器
///
/// 负责推断表达式的类型并收集类型约束
#[derive(Debug)]
pub struct TypeInferrer<'a> {
    /// 类型约束求解器
    solver: &'a mut TypeConstraintSolver,
    /// Send/Sync 约束求解器
    send_sync_solver: SendSyncSolver,
    /// 变量环境栈：每一层是一个作用域
    scopes: Vec<HashMap<String, PolyType>>,
    /// 循环标签栈（用于 break/continue）
    loop_labels: Vec<String>,
    /// 当前函数的返回类型（用于 return 语句检查）
    pub current_return_type: Option<MonoType>,
    /// 当前函数是否需要 Send 约束（spawn 函数）
    current_fn_requires_send: bool,
    /// 当前函数的泛型参数列表（用于约束传播）
    current_fn_type_params: Vec<MonoType>,
}

impl<'a> TypeInferrer<'a> {
    /// 创建新的类型推断器
    pub fn new(solver: &'a mut TypeConstraintSolver) -> Self {
        TypeInferrer {
            solver,
            send_sync_solver: SendSyncSolver::new(),
            scopes: vec![HashMap::new()], // Global scope
            loop_labels: Vec::new(),
            current_return_type: None,
            current_fn_requires_send: false,
            current_fn_type_params: Vec::new(),
        }
    }

    /// 获取求解器引用（可变）
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        self.solver
    }

    /// 获取求解器引用（不可变）
    pub fn solver_ref(&self) -> &TypeConstraintSolver {
        self.solver
    }

    /// 获取 Send/Sync 约束求解器
    pub fn send_sync_solver(&mut self) -> &mut SendSyncSolver {
        &mut self.send_sync_solver
    }

    /// 检查类型是否满足 Send 约束
    pub fn is_send(
        &self,
        ty: &MonoType,
    ) -> bool {
        self.send_sync_solver.is_send(ty)
    }

    /// 检查类型是否满足 Sync 约束
    pub fn is_sync(
        &self,
        ty: &MonoType,
    ) -> bool {
        self.send_sync_solver.is_sync(ty)
    }

    /// 添加 Send 约束
    pub fn add_send_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.send_sync_solver.add_send_constraint(ty);
    }

    /// 添加 Sync 约束
    pub fn add_sync_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.send_sync_solver.add_sync_constraint(ty);
    }

    /// 标记当前函数需要 Send 约束（用于 spawn 函数）
    pub fn mark_current_fn_requires_send(&mut self) {
        self.current_fn_requires_send = true;
    }

    /// 检查当前函数是否需要 Send 约束
    pub fn current_fn_requires_send(&self) -> bool {
        self.current_fn_requires_send
    }

    /// 设置当前函数的泛型参数
    pub fn set_current_fn_type_params(
        &mut self,
        params: Vec<MonoType>,
    ) {
        self.current_fn_type_params = params;
    }

    /// 获取当前函数的泛型参数
    pub fn current_fn_type_params(&self) -> &[MonoType] {
        &self.current_fn_type_params
    }

    /// 检查泛型参数是否满足 Send 约束
    ///
    /// 对于 spawn 函数，所有泛型参数都必须满足 Send 约束
    pub fn check_send_for_generic_params(&self) -> Vec<(MonoType, &'static str)> {
        if !self.current_fn_requires_send {
            return Vec::new();
        }

        let mut errors = Vec::new();
        for ty in &self.current_fn_type_params {
            if !self.is_send(ty) {
                errors.push((ty.clone(), "not Send"));
            }
        }
        errors
    }

    /// 检查作用域中的变量是否满足 Send 约束
    ///
    /// 用于 spawn 函数检查闭包捕获的变量
    pub fn check_scope_for_send(
        &mut self,
        required_vars: &[String],
    ) -> Vec<(String, MonoType, &'static str)> {
        if !self.current_fn_requires_send {
            return Vec::new();
        }

        let mut errors = Vec::new();
        for var_name in required_vars {
            // 在所有作用域中查找变量
            for scope in self.scopes.iter().rev() {
                if let Some(poly) = scope.get(var_name) {
                    let ty = self.solver.instantiate(poly);
                    if !self.is_send(&ty) {
                        errors.push((var_name.clone(), ty, "not Send"));
                    }
                    break;
                }
            }
        }
        errors
    }

    /// 为所有作用域中的自由变量添加 Send 约束
    ///
    /// 当推断 spawn 函数时，闭包捕获的所有变量都必须满足 Send 约束
    pub fn add_send_constraint_to_captured_vars(&mut self) {
        if !self.current_fn_requires_send {
            return;
        }

        // 遍历所有作用域中的变量
        for scope in self.scopes.iter() {
            for (_, poly) in scope.iter() {
                // 对多态类型中的泛型变量添加 Send 约束
                for binder in &poly.type_binders {
                    self.send_sync_solver
                        .add_send_constraint(&MonoType::TypeVar(*binder));
                }
            }
        }
    }

    // =========================================================================
    // 表达式类型推断
    // =========================================================================

    /// 推断表达式的类型
    #[allow(clippy::result_large_err)]
    pub fn infer_expr(
        &mut self,
        expr: &ast::Expr,
    ) -> TypeResult<MonoType> {
        match &expr {
            ast::Expr::Lit(lit, span) => self.infer_literal(lit, *span),
            ast::Expr::Var(name, span) => self.infer_var(name, *span),
            ast::Expr::BinOp {
                op,
                left,
                right,
                span,
            } => self.infer_binop(op, left, right, *span),
            ast::Expr::UnOp { op, expr, span } => self.infer_unop(op, expr, *span),
            ast::Expr::Call { func, args, span } => self.infer_call(func, args, *span),
            ast::Expr::FnDef {
                name: _,
                params,
                return_type,
                body,
                is_async: _,
                span,
            } => self.infer_fn_def_expr(params, return_type.as_ref(), body, *span),
            ast::Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                span,
            } => self.infer_if(
                condition,
                then_branch,
                elif_branches,
                else_branch.as_deref(),
                *span,
            ),
            ast::Expr::Match { expr, arms, span } => self.infer_match(expr, arms, *span),
            ast::Expr::While {
                condition,
                body,
                label: _,
                span,
            } => self.infer_while(condition, body, None, *span),
            ast::Expr::For {
                var,
                iterable,
                body,
                label: _,
                span,
            } => self.infer_for(var, iterable, body, *span),
            ast::Expr::Block(block) => self.infer_block(block, true, None),
            ast::Expr::Return(expr, span) => self.infer_return(expr.as_deref(), *span),
            ast::Expr::Break(label, span) => self.infer_break(label.as_deref(), *span),
            ast::Expr::Continue(label, span) => self.infer_continue(label.as_deref(), *span),
            ast::Expr::Cast {
                expr,
                target_type,
                span,
            } => self.infer_cast(expr, target_type, *span),
            ast::Expr::Tuple(exprs, span) => self.infer_tuple(exprs, *span),
            ast::Expr::List(exprs, span) => self.infer_list(exprs, *span),
            ast::Expr::Dict(pairs, span) => self.infer_dict(pairs, *span),
            ast::Expr::Index { expr, index, span } => self.infer_index(expr, index, *span),
            ast::Expr::FieldAccess { expr, field, span } => {
                self.infer_field_access(expr, field, *span)
            }
            ast::Expr::ListComp { .. } => unimplemented!("List comprehension type inference"),
            ast::Expr::Try { expr, span } => self.infer_try(expr, *span),
            ast::Expr::Ref { expr, span } => self.infer_ref(expr, *span),
        }
    }

    /// 推断字面量的类型
    #[allow(clippy::result_large_err)]
    fn infer_literal(
        &mut self,
        lit: &Literal,
        _span: Span,
    ) -> TypeResult<MonoType> {
        let ty = match lit {
            Literal::Int(_) => MonoType::Int(64),
            Literal::Float(_) => MonoType::Float(64),
            Literal::Bool(_) => MonoType::Bool,
            Literal::Char(_) => MonoType::Char,
            Literal::String(_) => MonoType::String,
        };
        Ok(ty)
    }

    /// 推断变量的类型
    fn infer_var(
        &mut self,
        name: &str,
        span: Span,
    ) -> TypeResult<MonoType> {
        // 查找变量
        let poly = self.get_var(name).cloned();

        if let Some(poly) = poly {
            // 实例化多态类型
            let ty = self.solver.instantiate(&poly);
            // 解析类型变量绑定以避免无限递归
            let resolved = self.solver.resolve_type(&ty);
            Ok(resolved)
        } else {
            Err(TypeError::UnknownVariable {
                name: name.to_string(),
                span,
            })
        }
    }

    /// 推断二元运算的类型
    fn infer_binop(
        &mut self,
        op: &BinOp,
        left: &ast::Expr,
        right: &ast::Expr,
        span: Span,
    ) -> TypeResult<MonoType> {
        let left_ty = self.infer_expr(left)?;
        let right_ty = self.infer_expr(right)?;

        // 解析类型变量绑定
        let left_ty = self.solver.resolve_type(&left_ty);
        let right_ty = self.solver.resolve_type(&right_ty);

        match op {
            // 算术运算
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                // 数值类型检查
                let num_ty = self.solver.new_var();
                self.solver
                    .add_constraint(left_ty.clone(), num_ty.clone(), span);
                self.solver
                    .add_constraint(right_ty.clone(), num_ty.clone(), span);
                Ok(num_ty)
            }

            // 比较运算
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                // 两边类型必须相等
                self.solver.add_constraint(left_ty, right_ty, span);
                Ok(MonoType::Bool)
            }

            // 逻辑运算
            BinOp::And | BinOp::Or => {
                // 两边必须是布尔类型
                self.solver.add_constraint(left_ty, MonoType::Bool, span);
                self.solver.add_constraint(right_ty, MonoType::Bool, span);
                Ok(MonoType::Bool)
            }

            // 赋值运算
            BinOp::Assign => {
                // 赋值表达式的类型是 Unit（void）
                self.solver.add_constraint(left_ty, right_ty, span);
                Ok(MonoType::Void)
            }

            // 范围运算
            BinOp::Range => {
                // 左右两边必须是相同类型
                self.solver
                    .add_constraint(left_ty.clone(), right_ty.clone(), span);
                // 返回一个范围类型
                Ok(MonoType::Range {
                    elem_type: Box::new(left_ty),
                })
            }
        }
    }

    /// 推断一元运算的类型
    fn infer_unop(
        &mut self,
        op: &UnOp,
        expr: &ast::Expr,
        span: Span,
    ) -> TypeResult<MonoType> {
        let expr_ty = self.infer_expr(expr)?;

        match op {
            UnOp::Neg | UnOp::Pos => {
                // 数值类型
                let num_ty = self.solver.new_var();
                self.solver.add_constraint(expr_ty, num_ty.clone(), span);
                Ok(num_ty)
            }
            UnOp::Not => {
                // 布尔类型
                self.solver.add_constraint(expr_ty, MonoType::Bool, span);
                Ok(MonoType::Bool)
            }
        }
    }

    /// 推断函数调用的类型
    pub fn infer_call(
        &mut self,
        func: &ast::Expr,
        args: &[ast::Expr],
        span: Span,
    ) -> TypeResult<MonoType> {
        // 检查是否是构造函数调用：标识符对应结构体类型
        if let ast::Expr::Var(func_name, _) = func {
            // 查找标识符对应的类型
            if let Some(poly) = self.get_var(func_name).cloned() {
                let mono = self.solver.instantiate(&poly);
                let resolved = self.solver.resolve_type(&mono);
                // 如果是结构体类型，直接返回该类型（构造函数调用）
                if let MonoType::Struct(_) = resolved {
                    tlog!(debug, MSG::DebugStructTypeConstructorCall, func_name);
                    // 检查参数数量是否匹配（这里简化处理，实际应该检查字段数量）
                    // 为每个参数添加类型约束
                    for arg in args {
                        let _arg_ty = self.infer_expr(arg)?;
                    }
                    return Ok(resolved);
                }
            }
        }

        // 检查是否是类型级方法调用：Type.method(args)
        // 这种语法允许直接通过类型名调用方法，无需显式传入 self
        if let ast::Expr::FieldAccess { expr, field, .. } = func {
            // 检查 expr 是否为类型名（变量）
            if let ast::Expr::Var(type_name, _) = &**expr {
                // 查找类型定义
                if let Some(poly) = self.get_var(type_name).cloned() {
                    let mono = self.solver.instantiate(&poly);
                    let resolved = self.solver.resolve_type(&mono);

                    // 如果是结构体类型，查找方法
                    if let MonoType::Struct(ref struct_type) = resolved {
                        // 从方法表中查找方法
                        if let Some(method) = struct_type.methods.get(field) {
                            let method_type = self.solver.instantiate(method);

                            // 方法类型应该是 Fn，我们需要处理参数绑定
                            if let MonoType::Fn {
                                params,
                                return_type,
                                ..
                            } = method_type
                            {
                                // 自动绑定机制：
                                // 1. 如果方法签名参数数量 = 调用参数数量 + 1，说明第一个是 self，自动绑定
                                // 2. 如果参数数量相等，尝试找到第一个匹配 self 类型的参数，自动绑定
                                // 3. 如果参数数量不匹配，报错

                                let params_len = params.len();
                                let args_len = args.len();

                                #[allow(unused_assignments)]
                                let mut params_to_bind = Vec::new();

                                if params_len == args_len + 1 {
                                    // 情况1：第一个参数是 self，自动绑定
                                    params_to_bind = params[1..].to_vec();
                                } else if params_len == args_len {
                                    // 情况2：参数数量相等，检查是否有可自动绑定的参数
                                    let mut self_param_index = None;
                                    for (i, param) in params.iter().enumerate() {
                                        if self.solver.unify(param, &resolved).is_ok() {
                                            self_param_index = Some(i);
                                            break;
                                        }
                                    }

                                    if let Some(idx) = self_param_index {
                                        // 找到了可自动绑定的参数
                                        let mut new_params = params.clone();
                                        new_params.remove(idx);
                                        params_to_bind = new_params;
                                    } else {
                                        // 没有可自动绑定的参数，按普通方式处理
                                        params_to_bind = params.clone();
                                    }
                                } else {
                                    // 参数数量不匹配
                                    return Err(TypeError::CallError {
                                        message: format!(
                                            "Method {}.{} expects {} arguments, but {} provided",
                                            type_name, field, params_len, args_len
                                        ),
                                        span,
                                    });
                                }

                                // 为每个参数添加类型约束
                                for (arg, param_ty) in args.iter().zip(params_to_bind.iter()) {
                                    let arg_ty = self.infer_expr(arg)?;
                                    self.solver.add_constraint(arg_ty, param_ty.clone(), span);
                                }

                                return Ok(*return_type.clone());
                            }
                        }
                    }
                }
            }

            // 如果不是类型级方法调用，则按普通字段访问处理
            // 首先推断字段的类型
            let field_ty = self.infer_field_access(expr, field, span)?;

            // 如果字段类型是函数，则处理方法调用
            // 创建类型变量用于参数和返回值
            let param_tys: Vec<MonoType> = args.iter().map(|_| self.solver.new_var()).collect();
            let return_ty = self.solver.new_var();

            // 构建函数类型约束
            let expected_fn_ty = MonoType::Fn {
                params: param_tys.clone(),
                return_type: Box::new(return_ty.clone()),
                is_async: false,
            };

            self.solver.add_constraint(field_ty, expected_fn_ty, span);

            // 为每个参数添加类型约束
            for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                let arg_ty = self.infer_expr(arg)?;
                self.solver.add_constraint(arg_ty, param_ty.clone(), span);
            }

            return Ok(return_ty);
        }

        // 普通函数调用
        let func_ty = self.infer_expr(func)?;

        // 创建类型变量用于参数和返回值
        let param_tys: Vec<MonoType> = args.iter().map(|_| self.solver.new_var()).collect();

        let return_ty = self.solver.new_var();

        // 构建函数类型约束
        let expected_fn_ty = MonoType::Fn {
            params: param_tys.clone(),
            return_type: Box::new(return_ty.clone()),
            is_async: false,
        };

        self.solver.add_constraint(func_ty, expected_fn_ty, span);

        // 为每个参数添加类型约束
        // 修复：增强函数调用时的参数类型匹配验证
        for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
            let arg_ty = self.infer_expr(arg)?;
            // 添加更严格的参数类型约束
            self.solver.add_constraint(arg_ty, param_ty.clone(), span);

            // 可选：添加额外的检查以确保类型真正兼容
            // 这里可以添加更复杂的类型兼容性检查
        }

        Ok(return_ty)
    }

    /// 推断函数定义表达式的类型
    fn infer_fn_def_expr(
        &mut self,
        params: &[ast::Param],
        return_type: Option<&ast::Type>,
        body: &ast::Block,
        _span: Span,
    ) -> TypeResult<MonoType> {
        let param_types: Vec<MonoType> = params
            .iter()
            .map(|p| {
                if let Some(ty) = &p.ty {
                    MonoType::from(ty.clone())
                } else {
                    self.solver.new_var()
                }
            })
            .collect();

        let return_ty = if let Some(ty) = return_type {
            MonoType::from(ty.clone())
        } else {
            self.solver.new_var()
        };

        // 保存当前返回类型
        let prev_return_type = self.current_return_type.clone();
        self.current_return_type = Some(return_ty.clone());

        self.enter_scope();
        for (param, param_ty) in params.iter().zip(param_types.iter()) {
            self.add_var(param.name.clone(), PolyType::mono(param_ty.clone()));
        }

        let body_ty = self.infer_block(body, false, None)?;

        // 处理隐式返回
        if let Some(expr) = &body.expr {
            // 如果最后表达式是 `return`，则该 return 已在 infer_return 中
            // 对当前返回类型添加了约束，因此不需要（也不应）再将
            // 块的类型与返回类型约束在一起 — 否则会把 `Void` 约束到返回类型。
            if !matches!(expr.as_ref(), ast::Expr::Return(..)) {
                // 如果最后有表达式（且不是 return），它的类型必须匹配返回类型
                self.solver
                    .add_constraint(body_ty, return_ty.clone(), body.span);
            }
        } else {
            // 如果没有最后表达式（隐式 Void）
            // 检查是否以 return 语句结尾
            let ends_with_return = if let Some(last) = body.stmts.last() {
                if let ast::StmtKind::Expr(e) = &last.kind {
                    matches!(e.as_ref(), ast::Expr::Return(..))
                } else {
                    false
                }
            } else {
                false
            };

            // 如果没有显式 return，隐式返回 Void
            if !ends_with_return {
                self.solver
                    .add_constraint(MonoType::Void, return_ty.clone(), body.span);
            }
        }

        self.exit_scope();
        self.current_return_type = prev_return_type;

        Ok(MonoType::Fn {
            params: param_types,
            return_type: Box::new(return_ty),
            is_async: false,
        })
    }

    /// 推断 if 表达式的类型
    fn infer_if(
        &mut self,
        condition: &ast::Expr,
        then_branch: &ast::Block,
        elif_branches: &[(Box<ast::Expr>, Box<ast::Block>)],
        else_branch: Option<&ast::Block>,
        span: Span,
    ) -> TypeResult<MonoType> {
        // 条件必须是布尔类型
        let cond_ty = self.infer_expr(condition)?;
        self.solver.add_constraint(cond_ty, MonoType::Bool, span);

        // 进入作用域（用于处理分支中可能添加的变量）
        self.enter_scope();

        // 推断各分支的类型
        let then_ty = self.infer_block(then_branch, false, None)?;

        // 处理 elif 分支
        let mut current_ty = then_ty;
        for (elif_cond, elif_body) in elif_branches {
            let elif_cond_ty = self.infer_expr(elif_cond)?;
            self.solver
                .add_constraint(elif_cond_ty, MonoType::Bool, span);

            let elif_body_ty = self.infer_block(elif_body, false, None)?;

            // 所有分支类型必须一致
            self.solver
                .add_constraint(current_ty.clone(), elif_body_ty.clone(), span);
            current_ty = elif_body_ty;
        }

        // 处理 else 分支
        if let Some(else_body) = else_branch {
            let else_ty = self.infer_block(else_body, false, None)?;
            self.solver
                .add_constraint(current_ty.clone(), else_ty, span);
        }

        // 退出作用域
        self.exit_scope();

        Ok(current_ty)
    }

    /// 推断 match 表达式的类型
    fn infer_match(
        &mut self,
        expr: &ast::Expr,
        arms: &[ast::MatchArm],
        _span: Span,
    ) -> TypeResult<MonoType> {
        let expr_ty = self.infer_expr(expr)?;

        // 所有 match arm 的类型必须一致
        let result_ty = self.solver.new_var();

        for arm in arms {
            self.enter_scope();
            let pat_ty = self.infer_pattern(&arm.pattern, Some(&expr_ty), arm.span)?;
            self.solver
                .add_constraint(pat_ty, expr_ty.clone(), arm.span);

            let body_ty = self.infer_expr(&arm.body)?;
            self.solver
                .add_constraint(body_ty, result_ty.clone(), arm.span);
            self.exit_scope();
        }

        Ok(result_ty)
    }

    /// 推断模式类型
    ///
    /// # Arguments
    /// * `pattern` - 要推断的模式
    /// * `expected` - 期望的类型（用于约束模式类型与被匹配值类型一致）
    /// * `span` - 模式的位置信息（用于错误报告）
    pub fn infer_pattern(
        &mut self,
        pattern: &ast::Pattern,
        expected: Option<&MonoType>,
        span: Span,
    ) -> TypeResult<MonoType> {
        match pattern {
            ast::Pattern::Wildcard => Ok(self.solver.new_var()),
            ast::Pattern::Identifier(name) => {
                // 绑定变量到新类型变量
                let ty = self.solver.new_var();
                self.add_var(name.clone(), PolyType::mono(ty.clone()));
                Ok(ty)
            }
            ast::Pattern::Literal(lit) => self.infer_literal(lit, span),
            ast::Pattern::Tuple(patterns) => {
                let elem_tys: Vec<_> = patterns
                    .iter()
                    .map(|p| self.infer_pattern(p, expected, span))
                    .collect::<Result<_, _>>()?;
                Ok(MonoType::Tuple(elem_tys))
            }
            ast::Pattern::Struct { name, fields } => {
                // 从类型环境获取结构体类型定义
                let poly_clone = self.get_var(name).cloned();
                let struct_ty: Option<super::types::StructType> = if let Some(poly) = poly_clone {
                    // 实例化多态类型获取具体类型
                    let instantiated = self.solver.instantiate(&poly);
                    match instantiated {
                        MonoType::Struct(s) => Some(s),
                        _ => None,
                    }
                } else {
                    None
                };

                if let Some(struct_def) = &struct_ty {
                    // 结构体定义存在，验证字段匹配
                    // 创建字段类型的映射
                    let field_types: HashMap<_, _> = struct_def
                        .fields
                        .iter()
                        .map(|(n, t)| (n.clone(), t.clone()))
                        .collect();

                    // 收集所有字段模式的期望类型
                    let field_expected_types: Vec<_> = fields
                        .iter()
                        .map(|(field_name, _)| field_types.get(field_name).cloned())
                        .collect();

                    // 推断每个模式字段
                    for ((field_name, field_pattern), expected_field_ty) in
                        fields.iter().zip(field_expected_types.iter())
                    {
                        if let Some(expected_ty) = expected_field_ty {
                            // 推断字段模式类型
                            let _field_pat_ty =
                                self.infer_pattern(field_pattern, Some(expected_ty), span)?;
                        } else {
                            // 字段不存在于结构体定义
                            return Err(TypeError::UnknownField {
                                struct_name: name.clone(),
                                field_name: field_name.clone(),
                                span,
                            });
                        }
                    }

                    // 返回结构体类型
                    Ok(MonoType::Struct(struct_def.clone()))
                } else {
                    // 结构体定义不存在，尝试从 expected 类型推断
                    if let Some(expected_ty) = expected {
                        match expected_ty {
                            MonoType::Struct(s) => {
                                // 从 expected 类型获取字段
                                let field_types: HashMap<_, _> = s
                                    .fields
                                    .iter()
                                    .map(|(n, t)| (n.clone(), t.clone()))
                                    .collect();

                                // 收集所有字段模式的期望类型
                                let field_expected_types: Vec<_> = fields
                                    .iter()
                                    .map(|(field_name, _)| field_types.get(field_name).cloned())
                                    .collect();

                                // 推断每个模式字段
                                for ((field_name, field_pattern), expected_field_ty) in
                                    fields.iter().zip(field_expected_types.iter())
                                {
                                    if let Some(expected_ty) = expected_field_ty {
                                        let _field_pat_ty = self.infer_pattern(
                                            field_pattern,
                                            Some(expected_ty),
                                            span,
                                        )?;
                                    } else {
                                        return Err(TypeError::UnknownField {
                                            struct_name: name.clone(),
                                            field_name: field_name.clone(),
                                            span,
                                        });
                                    }
                                }
                                Ok(expected_ty.clone())
                            }
                            _ => {
                                // expected 不是结构体类型，创建类型变量
                                let ty = self.solver.new_var();
                                self.solver
                                    .add_constraint(ty.clone(), expected_ty.clone(), span);
                                Ok(ty)
                            }
                        }
                    } else {
                        // 没有 expected 类型，创建新类型变量
                        Ok(self.solver.new_var())
                    }
                }
            }
            ast::Pattern::Union {
                name: _,
                variant: _,
                pattern: _,
            } => {
                // 简化处理：返回新类型变量
                let ty = self.solver.new_var();
                if let Some(expected_ty) = expected {
                    self.solver
                        .add_constraint(ty.clone(), expected_ty.clone(), span);
                }
                Ok(ty)
            }
            ast::Pattern::Or(patterns) => {
                if let Some(first) = patterns.first() {
                    let first_ty = self.infer_pattern(first, expected, span)?;
                    for pattern in patterns.iter().skip(1) {
                        let pattern_ty = self.infer_pattern(pattern, expected, span)?;
                        self.solver
                            .add_constraint(first_ty.clone(), pattern_ty, span);
                    }
                    Ok(first_ty)
                } else {
                    Ok(self.solver.new_var())
                }
            }
            ast::Pattern::Guard { pattern, condition } => {
                let pattern_ty = self.infer_pattern(pattern, expected, span)?;
                let _cond_ty = self.infer_expr(condition)?;
                Ok(pattern_ty)
            }
        }
    }

    /// 推断 while 循环的类型
    fn infer_while(
        &mut self,
        condition: &ast::Expr,
        body: &ast::Block,
        label: Option<&str>,
        span: Span,
    ) -> TypeResult<MonoType> {
        // 预扫描循环体中的变量声明和赋值，确保在推断条件之前变量已被添加到作用域
        // 这对于像 `while i < n { i = i + 1; }` 这样的代码是必要的
        // 注意：这里显式管理作用域，因为 while 循环需要在其 body 被推断之前添加变量

        // 进入循环体作用域
        self.enter_scope();

        // 直接为循环体中发现的变量添加到当前作用域
        for stmt in &body.stmts {
            match &stmt.kind {
                ast::StmtKind::Var { name, .. } => {
                    // 带类型注解的变量声明：StmtKind::Var
                    let ty = self.solver.new_var();
                    let poly = PolyType::mono(ty);
                    self.add_var(name.clone(), poly);
                }
                ast::StmtKind::Expr(expr) => {
                    // 不带类型注解的赋值：StmtKind::Expr(BinOp::Assign(...))
                    if let ast::Expr::BinOp { op, left, .. } = expr.as_ref() {
                        if *op == BinOp::Assign {
                            if let ast::Expr::Var(name, _) = left.as_ref() {
                                let ty = self.solver.new_var();
                                let poly = PolyType::mono(ty);
                                self.add_var(name.clone(), poly);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // 条件必须是布尔类型
        let cond_ty = self.infer_expr(condition)?;

        self.solver.add_constraint(cond_ty, MonoType::Bool, span);

        // 推断循环体（注意：这里 manage_scope = false，因为我们已经管理了作用域）
        // 重要：如果 body.expr 是 While 表达式，infer_expr 会调用 infer_while
        // 但 infer_while 会再次 enter_scope，导致无限递归
        // 所以我们需要在调用 infer_while 之前手动处理这个问题
        if let Some(l) = label {
            self.loop_labels.push(l.to_string());
        }

        // 检查 body.expr 是否是 While 或 For 循环（可能导致无限递归）
        // 如果是，我们直接设置 Void 返回类型，避免进一步推断
        let _body_ty = if let Some(expr) = &body.expr {
            match expr.as_ref() {
                ast::Expr::While { .. } | ast::Expr::For { .. } => {
                    // 避免无限递归，直接返回 Void
                    MonoType::Void
                }
                _ => self.infer_expr(expr)?,
            }
        } else {
            MonoType::Void
        };

        if label.is_some() {
            self.loop_labels.pop();
        }

        // 退出循环体作用域
        self.exit_scope();

        // while 表达式返回 Void
        Ok(MonoType::Void)
    }

    /// 推断 for 循环的类型
    fn infer_for(
        &mut self,
        var: &str,
        iterable: &ast::Expr,
        body: &ast::Block,
        span: Span,
    ) -> TypeResult<MonoType> {
        // 推断可迭代对象的类型
        let iter_ty = self.infer_expr(iterable)?;

        // 获取元素类型
        let elem_ty = self.solver.new_var();

        // 支持 Range 和 List 类型作为可迭代对象
        match &iter_ty {
            MonoType::Range { elem_type } => {
                // Range 类型：元素类型由 Range 决定
                self.solver
                    .add_constraint(elem_ty.clone(), *elem_type.clone(), span);
            }
            MonoType::List(list_elem) => {
                // List 类型：元素类型由 List 决定
                self.solver
                    .add_constraint(elem_ty.clone(), *list_elem.clone(), span);
            }
            _ => {
                // 其他类型：假设是 List，元素类型用 elem_ty
                let expected_iter_ty = MonoType::List(Box::new(elem_ty.clone()));
                self.solver.add_constraint(iter_ty, expected_iter_ty, span);
            }
        }

        // 在循环体内绑定迭代变量
        // 注意：infer_block 会自动管理作用域，所以这里不再调用 enter_scope/exit_scope
        self.add_var(var.to_string(), PolyType::mono(elem_ty));
        let _body_ty = self.infer_block(body, true, None)?;

        Ok(MonoType::Void)
    }

    /// 推断代码块类型
    ///
    /// # Arguments
    /// * `block` - 要推断的代码块
    /// * `manage_scope` - 是否管理作用域（进入/退出作用域）
    /// * `expected_type` - 期望的类型（如果有）
    pub fn infer_block(
        &mut self,
        block: &ast::Block,
        manage_scope: bool,
        expected_type: Option<&MonoType>,
    ) -> TypeResult<MonoType> {
        if manage_scope {
            self.enter_scope();
        }

        // 阶段2修复：改进块类型推断，更好地处理表达式块和语句块
        let has_last_expr = block.expr.is_some();
        let mut is_pure_statement_block = !has_last_expr;

        // 检查语句
        for stmt in &block.stmts {
            match &stmt.kind {
                ast::StmtKind::Expr(expr) => {
                    // 检查是否是赋值表达式：BinOp::Assign(Var(name), value)
                    // 如果是，先将变量添加到作用域，再推断类型
                    if let ast::Expr::BinOp {
                        op: BinOp::Assign,
                        left,
                        ..
                    } = expr.as_ref()
                    {
                        if let ast::Expr::Var(name, _) = left.as_ref() {
                            let ty = self.solver.new_var();
                            let poly = PolyType::mono(ty);
                            self.add_var(name.clone(), poly);
                        }
                    }

                    // 阶段2修复：检查是否是 Return 语句，这会影响块的返回语义
                    if let ast::Expr::Return(_, _) = expr.as_ref() {
                        // 如果有显式 return 语句，标记为非纯语句块
                        is_pure_statement_block = false;
                    }

                    // Expr 可能包含 While, For, Return, Break, Continue 等
                    let _ty = self.infer_expr(expr)?;
                }
                ast::StmtKind::Var {
                    name,
                    type_annotation,
                    initializer,
                    is_mut: _,
                } => {
                    self.infer_var_decl(
                        name,
                        type_annotation.as_ref(),
                        initializer.as_deref(),
                        block.span,
                    )?;
                }
                ast::StmtKind::For {
                    var,
                    iterable,
                    body,
                    label: _,
                } => {
                    // 推断 for 循环
                    self.infer_for(var, iterable, body, block.span)?;
                }
                ast::StmtKind::Fn {
                    name,
                    type_annotation,
                    params,
                    body: (stmts, expr),
                } => {
                    // 处理嵌套函数定义
                    // 构建函数体的 Block
                    let fn_body = ast::Block {
                        stmts: stmts.clone(),
                        expr: expr.clone(),
                        span: stmt.span,
                    };

                    // 从类型注解中提取返回类型
                    let return_type =
                        if let Some(ast::Type::Fn { return_type, .. }) = type_annotation {
                            Some(return_type.as_ref().clone())
                        } else {
                            None
                        };

                    // 推断函数类型
                    let fn_ty =
                        self.infer_fn_def_expr(params, return_type.as_ref(), &fn_body, stmt.span)?;

                    // 将函数添加到当前作用域
                    let poly = PolyType::mono(fn_ty);
                    self.add_var(name.clone(), poly);
                }
                // TypeDef, Use 等已在 check_stmt 中处理
                // While, Return, Break, Continue 作为 Expr 的一部分处理
                _ => {}
            }
        }

        // 阶段2修复：更精确的返回类型推断
        let ty = if let Some(expr) = &block.expr {
            // 有最后表达式：这是表达式块，返回表达式的类型
            self.infer_expr(expr)?
        } else if is_pure_statement_block {
            // 纯语句块（没有最后表达式，也没有显式 return）
            // 阶段2修复：正确返回 Void
            MonoType::Void
        } else {
            // 非纯语句块（可能有显式 return 语句）
            // 这种情况比较复杂，保守返回 Void
            MonoType::Void
        };

        if manage_scope {
            self.exit_scope();
        }

        if let Some(expected) = expected_type {
            self.solver
                .add_constraint(ty.clone(), expected.clone(), block.span);
        }

        Ok(ty)
    }

    /// 推断变量声明: `name[: type] [= expr]`
    pub(crate) fn infer_var_decl(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        initializer: Option<&ast::Expr>,
        span: Span,
    ) -> TypeResult<()> {
        if let Some(init) = initializer {
            let init_ty = self.infer_expr(init)?;

            if let Some(ann) = type_annotation {
                let ann_ty = MonoType::from(ann.clone());
                let resolved_ann_ty = self.resolve_type_ref(&ann_ty);
                self.solver
                    .add_constraint(init_ty.clone(), resolved_ann_ty, span);
            }

            // 泛化 initializer 的类型
            let poly = self.solver.generalize(&init_ty);
            self.add_var(name.to_string(), poly);
        } else if let Some(ann) = type_annotation {
            // 没有初始化时，使用解析后的类型
            let ty = MonoType::from(ann.clone());
            let resolved_ty = self.resolve_type_ref(&ty);
            self.add_var(name.to_string(), PolyType::mono(resolved_ty));
        } else {
            // 没有任何信息，创建新类型变量
            let ty = self.solver.new_var();
            self.add_var(name.to_string(), PolyType::mono(ty));
        }

        Ok(())
    }

    /// 推断 return 表达式
    fn infer_return(
        &mut self,
        expr: Option<&ast::Expr>,
        span: Span,
    ) -> TypeResult<MonoType> {
        if let Some(e) = expr {
            let ty = self.infer_expr(e)?;
            if let Some(ret_ty) = &self.current_return_type {
                self.solver.add_constraint(ty, ret_ty.clone(), span);
            }
        } else if let Some(ret_ty) = &self.current_return_type {
            self.solver
                .add_constraint(MonoType::Void, ret_ty.clone(), span);
        }
        Ok(MonoType::Void)
    }

    /// 推断 break 表达式
    fn infer_break(
        &mut self,
        label: Option<&str>,
        span: Span,
    ) -> TypeResult<MonoType> {
        if let Some(l) = label {
            if !self.loop_labels.contains(&l.to_string()) {
                return Err(TypeError::UnknownLabel {
                    name: l.to_string(),
                    span,
                });
            }
        }
        Ok(MonoType::Void)
    }

    /// 推断 continue 表达式
    fn infer_continue(
        &mut self,
        label: Option<&str>,
        span: Span,
    ) -> TypeResult<MonoType> {
        if let Some(l) = label {
            if !self.loop_labels.contains(&l.to_string()) {
                return Err(TypeError::UnknownLabel {
                    name: l.to_string(),
                    span,
                });
            }
        }
        Ok(MonoType::Void)
    }

    /// 推断类型转换
    fn infer_cast(
        &mut self,
        expr: &ast::Expr,
        target_type: &ast::Type,
        _span: Span,
    ) -> TypeResult<MonoType> {
        let _expr_ty = self.infer_expr(expr)?;
        let target = MonoType::from(target_type.clone());
        Ok(target)
    }

    /// 推断元组类型
    fn infer_tuple(
        &mut self,
        exprs: &[ast::Expr],
        _span: Span,
    ) -> TypeResult<MonoType> {
        let elem_tys: Result<Vec<_>, _> = exprs.iter().map(|e| self.infer_expr(e)).collect();
        Ok(MonoType::Tuple(elem_tys?))
    }

    /// 推断列表类型
    fn infer_list(
        &mut self,
        exprs: &[ast::Expr],
        span: Span,
    ) -> TypeResult<MonoType> {
        if exprs.is_empty() {
            // 空列表，创建类型变量
            let elem_ty = self.solver.new_var();
            return Ok(MonoType::List(Box::new(elem_ty)));
        }

        // 推断第一个元素的类型
        let first_ty = self.infer_expr(&exprs[0])?;

        // 确保所有元素类型一致
        for expr in exprs.iter().skip(1) {
            let ty = self.infer_expr(expr)?;
            self.solver.add_constraint(first_ty.clone(), ty, span);
        }

        Ok(MonoType::List(Box::new(first_ty)))
    }

    /// 推断字典类型
    fn infer_dict(
        &mut self,
        pairs: &[(ast::Expr, ast::Expr)],
        span: Span,
    ) -> TypeResult<MonoType> {
        if pairs.is_empty() {
            // 空字典，创建类型变量
            let key_ty = self.solver.new_var();
            let value_ty = self.solver.new_var();
            return Ok(MonoType::Dict(Box::new(key_ty), Box::new(value_ty)));
        }

        // 推断第一个键值对的类型
        let (first_key, first_value) = &pairs[0];
        let key_ty = self.infer_expr(first_key)?;
        let value_ty = self.infer_expr(first_value)?;

        // 确保所有键值对类型一致
        for (key, value) in pairs.iter().skip(1) {
            let k_ty = self.infer_expr(key)?;
            let v_ty = self.infer_expr(value)?;
            self.solver.add_constraint(key_ty.clone(), k_ty, span);
            self.solver.add_constraint(value_ty.clone(), v_ty, span);
        }

        Ok(MonoType::Dict(Box::new(key_ty), Box::new(value_ty)))
    }

    /// 推断下标访问类型
    fn infer_index(
        &mut self,
        expr: &ast::Expr,
        index: &ast::Expr,
        span: Span,
    ) -> TypeResult<MonoType> {
        let expr_ty = self.infer_expr(expr)?;
        let _index_ty = self.infer_expr(index)?;

        // 推断元素类型
        let elem_ty = match &expr_ty {
            MonoType::List(t) => *t.clone(),
            MonoType::Dict(k, _) => *k.clone(),
            MonoType::String => MonoType::Char,
            MonoType::Tuple(types) => {
                // 静态下标检查
                if let ast::Expr::Lit(Literal::Int(i), _) = index {
                    if *i >= 0 && (*i as usize) < types.len() {
                        types[*i as usize].clone()
                    } else {
                        return Err(TypeError::IndexOutOfBounds {
                            index: *i,
                            size: types.len(),
                            span,
                        });
                    }
                } else {
                    self.solver.new_var()
                }
            }
            _ => {
                return Err(TypeError::UnsupportedOp {
                    op: "index".to_string(),
                    span,
                });
            }
        };

        Ok(elem_ty)
    }

    /// 推断字段访问类型
    fn infer_field_access(
        &mut self,
        expr: &ast::Expr,
        field: &str,
        span: Span,
    ) -> TypeResult<MonoType> {
        let expr_ty = self.infer_expr(expr)?;

        match &expr_ty {
            MonoType::Struct(s) => {
                // 首先检查字段
                for (name, ty) in &s.fields {
                    if name == field {
                        return Ok(ty.clone());
                    }
                }

                // 然后检查方法
                if let Some(method) = s.methods.get(field) {
                    // 方法返回 MonoType::Fn，我们返回整个函数类型
                    let method_type = self.solver().instantiate(method);
                    match method_type {
                        MonoType::Fn { .. } => {
                            return Ok(method_type);
                        }
                        _ => {
                            return Err(TypeError::CallError {
                                message: format!("{} is not a method", field),
                                span,
                            });
                        }
                    }
                }

                Err(TypeError::UnknownField {
                    struct_name: s.name.clone(),
                    field_name: field.to_string(),
                    span,
                })
            }
            _ => Err(TypeError::UnsupportedOp {
                op: "field access".to_string(),
                span,
            }),
        }
    }

    /// 推断 try 运算符（错误传播）`expr?`
    ///
    /// `?` 运算符返回成功类型的值：
    /// - Result<T, E> -> T
    /// - Option<T> -> T
    fn infer_try(
        &mut self,
        expr: &ast::Expr,
        _span: Span,
    ) -> TypeResult<MonoType> {
        let expr_ty = self.infer_expr(expr)?;

        // 尝试解包 Result 或 Option 类型
        match &expr_ty {
            MonoType::Enum(e) if e.name == "Result" => {
                // Result<T, E> -> 返回 T (泛型，需要创建类型变量)
                Ok(self.solver.new_var())
            }
            MonoType::Enum(e) if e.name == "Option" => {
                // Option<T> -> 返回 T (泛型，需要创建类型变量)
                Ok(self.solver.new_var())
            }
            MonoType::Struct(s) if s.name == "Result" => {
                // Result 结构体
                if let Some((_, ok_ty)) = s.fields.iter().find(|(n, _)| n == "value") {
                    Ok(ok_ty.clone())
                } else {
                    Ok(self.solver.new_var())
                }
            }
            MonoType::Struct(s) if s.name == "Option" => {
                // Option 结构体
                if let Some((_, some_ty)) = s.fields.iter().find(|(n, _)| n == "value") {
                    Ok(some_ty.clone())
                } else {
                    Ok(self.solver.new_var())
                }
            }
            _ => {
                // 不是 Result/Option，创建一个新类型变量
                // 运行时错误会通过模式匹配检测
                Ok(self.solver.new_var())
            }
        }
    }

    /// 推断 ref 表达式的类型：`ref expr` 返回 Arc<T>
    fn infer_ref(
        &mut self,
        expr: &ast::Expr,
        _span: Span,
    ) -> TypeResult<MonoType> {
        // 推断内部表达式的类型
        let inner_ty = self.infer_expr(expr)?;
        // ref 创建 Arc，返回 Arc<T>
        Ok(MonoType::Arc(Box::new(inner_ty)))
    }

    // =========================================================================
    // 作用域管理
    // =========================================================================

    /// 进入新作用域
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 退出作用域
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    // =========================================================================
    // 工具方法
    // =========================================================================

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        let _scope_count = self.scopes.len();
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, poly);
        }
    }

    /// 获取变量类型
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        for scope in self.scopes.iter().rev() {
            if let Some(poly) = scope.get(name) {
                return Some(poly);
            }
        }
        None
    }

    /// 获取变量类型（可变引用）
    pub fn get_var_mut(
        &mut self,
        name: &str,
    ) -> Option<&mut PolyType> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(poly) = scope.get_mut(name) {
                return Some(poly);
            }
        }
        None
    }

    /// 解析类型引用，将 TypeRef 转换为实际类型
    /// 例如：TypeRef("Point") -> 如果 Point 是结构体，返回 Struct 类型
    pub fn resolve_type_ref(
        &mut self,
        ty: &MonoType,
    ) -> MonoType {
        match ty {
            MonoType::TypeRef(name) => {
                // 查找类型定义
                if let Some(poly) = self.get_var(name).cloned() {
                    let mono = self.solver.instantiate(&poly);
                    let resolved = self.solver.resolve_type(&mono);
                    // 修复：增强类型解析逻辑，确保 TypeRef 被正确转换
                    // 如果解析后的类型是结构体且有名称，返回它
                    // 否则保持原样
                    match resolved {
                        MonoType::Struct(_) => resolved,
                        _ => {
                            // 如果不是结构体，尝试进一步解析
                            // 避免将未解析的 TypeRef 传递给调用者
                            if resolved == *ty {
                                // 如果解析后还是 TypeRef，说明可能有循环引用或未找到定义
                                // 在这种情况下，保持原始的 TypeRef 以便后续错误报告
                                ty.clone()
                            } else {
                                resolved
                            }
                        }
                    }
                } else {
                    // 未找到类型定义，保持原样
                    ty.clone()
                }
            }
            _ => ty.clone(),
        }
    }

    /// 获取所有变量绑定（用于 IR 生成）
    ///
    /// 合并所有作用域的绑定，返回完整的符号表
    pub fn get_all_bindings(&self) -> HashMap<String, PolyType> {
        let mut all_bindings = HashMap::new();
        // 从内层到外层合并，外层覆盖内层同名绑定
        for scope in self.scopes.iter() {
            for (name, poly) in scope.iter() {
                all_bindings.insert(name.clone(), poly.clone());
            }
        }
        all_bindings
    }
}
