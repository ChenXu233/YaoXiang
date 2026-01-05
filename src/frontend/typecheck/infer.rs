//! 表达式类型推断
//!
//! 使用 Hindley-Milner 算法推断表达式的类型

#![allow(clippy::result_large_err)]

use super::super::lexer::tokens::Literal;
use super::super::parser::ast::{self, BinOp, UnOp};
use super::errors::{TypeError, TypeResult};
use super::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::util::span::Span;
use std::collections::HashMap;

/// 类型推断器
///
/// 负责推断表达式的类型并收集类型约束
#[derive(Debug)]
pub struct TypeInferrer<'a> {
    /// 类型约束求解器
    solver: &'a mut TypeConstraintSolver,
    /// 变量环境栈：每一层是一个作用域
    scopes: Vec<HashMap<String, PolyType>>,
    /// 循环标签栈（用于 break/continue）
    loop_labels: Vec<String>,
    /// 当前函数的返回类型（用于 return 语句检查）
    pub current_return_type: Option<MonoType>,
}

impl<'a> TypeInferrer<'a> {
    /// 创建新的类型推断器
    pub fn new(solver: &'a mut TypeConstraintSolver) -> Self {
        TypeInferrer {
            solver,
            scopes: vec![HashMap::new()], // Global scope
            loop_labels: Vec::new(),
            current_return_type: None,
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
            Ok(ty)
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
        // 推断函数表达式的类型
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
        for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
            let arg_ty = self.infer_expr(arg)?;
            self.solver.add_constraint(arg_ty, param_ty.clone(), span);
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

        self.enter_scope();
        for (param, param_ty) in params.iter().zip(param_types.iter()) {
            self.add_var(param.name.clone(), PolyType::mono(param_ty.clone()));
        }
        let _body_ty = self.infer_block(body, false, None)?;
        self.exit_scope();

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

        // 推断各分支的类型
        let then_ty = self.infer_block(then_branch, true, None)?;

        // 处理 elif 分支
        let mut current_ty = then_ty;
        for (elif_cond, elif_body) in elif_branches {
            let elif_cond_ty = self.infer_expr(elif_cond)?;
            self.solver
                .add_constraint(elif_cond_ty, MonoType::Bool, span);

            let elif_body_ty = self.infer_block(elif_body, true, None)?;

            // 所有分支类型必须一致
            self.solver
                .add_constraint(current_ty.clone(), elif_body_ty.clone(), span);
            current_ty = elif_body_ty;
        }

        // 处理 else 分支
        if let Some(else_body) = else_branch {
            let else_ty = self.infer_block(else_body, true, None)?;
            self.solver
                .add_constraint(current_ty.clone(), else_ty, span);
        }

        Ok(current_ty)
    }

    /// 推断 match 表达式的类型
    fn infer_match(
        &mut self,
        expr: &ast::Expr,
        arms: &[ast::MatchArm],
        _span: Span,
    ) -> TypeResult<MonoType> {
        let _expr_ty = self.infer_expr(expr)?;

        // 所有 match arm 的类型必须一致
        let result_ty = self.solver.new_var();

        for arm in arms {
            let arm_ty = self.infer_pattern(&arm.pattern)?;
            self.solver
                .add_constraint(arm_ty, result_ty.clone(), arm.span);
        }

        Ok(result_ty)
    }

    /// 推断模式类型
    pub fn infer_pattern(
        &mut self,
        pattern: &ast::Pattern,
    ) -> TypeResult<MonoType> {
        match pattern {
            ast::Pattern::Wildcard => Ok(self.solver.new_var()),
            ast::Pattern::Identifier(name) => {
                // 绑定变量到新类型变量
                let ty = self.solver.new_var();
                self.add_var(name.clone(), PolyType::mono(ty.clone()));
                Ok(ty)
            }
            ast::Pattern::Literal(lit) => self.infer_literal(lit, Span::default()),
            ast::Pattern::Tuple(patterns) => {
                let elem_tys: Vec<_> = patterns
                    .iter()
                    .map(|p| self.infer_pattern(p))
                    .collect::<Result<_, _>>()?;
                Ok(MonoType::Tuple(elem_tys))
            }
            ast::Pattern::Struct { name: _, fields: _ } => {
                // 简化处理：返回新类型变量
                Ok(self.solver.new_var())
            }
            ast::Pattern::Union {
                name: _,
                variant: _,
                pattern: _,
            } => {
                // 简化处理：返回新类型变量
                Ok(self.solver.new_var())
            }
            ast::Pattern::Or(patterns) => {
                if let Some(first) = patterns.first() {
                    let first_ty = self.infer_pattern(first)?;
                    for pattern in patterns.iter().skip(1) {
                        let pattern_ty = self.infer_pattern(pattern)?;
                        self.solver
                            .add_constraint(first_ty.clone(), pattern_ty, Span::default());
                    }
                    Ok(first_ty)
                } else {
                    Ok(self.solver.new_var())
                }
            }
            ast::Pattern::Guard { pattern, condition } => {
                let pattern_ty = self.infer_pattern(pattern)?;
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
        // 条件必须是布尔类型
        let cond_ty = self.infer_expr(condition)?;

        self.solver.add_constraint(cond_ty, MonoType::Bool, span);

        // 推断循环体
        // 注意：infer_block 会自动管理作用域，所以这里不再调用 enter_scope/exit_scope
        if let Some(l) = label {
            self.loop_labels.push(l.to_string());
        }
        let _body_ty = self.infer_block(body, true, None)?;
        if label.is_some() {
            self.loop_labels.pop();
        }

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

        // 检查语句
        for stmt in &block.stmts {
            match &stmt.kind {
                ast::StmtKind::Expr(expr) => {
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
                // Fn, TypeDef, Use 等已在 check_stmt 中处理
                // While, Return, Break, Continue 作为 Expr 的一部分处理
                _ => {}
            }
        }

        // 返回最后表达式的类型或 Void
        let ty = if let Some(expr) = &block.expr {
            self.infer_expr(expr)?
        } else {
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
                self.solver.add_constraint(init_ty.clone(), ann_ty, span);
            }

            // 泛化 initializer 的类型
            let poly = self.solver.generalize(&init_ty);
            self.add_var(name.to_string(), poly);
        } else if let Some(ann) = type_annotation {
            // 没有初始化时，创建未绑定类型变量
            let ty = MonoType::from(ann.clone());
            self.add_var(name.to_string(), PolyType::mono(ty));
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
                for (name, ty) in &s.fields {
                    if name == field {
                        return Ok(ty.clone());
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
}
