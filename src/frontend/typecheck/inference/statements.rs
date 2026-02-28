#![allow(clippy::result_large_err)]

//! 语句检查器
//!
//! 合并原 checking/BodyChecker 和 inference/StmtInferrer
//! 使用统一的 ScopeManager 管理变量作用域

use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use std::collections::HashMap;
use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::parser::ast::{Stmt, Expr, Param, Block};

use super::scope::ScopeManager;

/// 语句检查器
///
/// 负责检查函数体中的语句和表达式的类型正确性。
/// 使用统一的 ScopeManager 实现作用域管理。
///
/// ## 错误收集模式
///
/// 支持两种错误处理模式：
/// - **短路模式**（默认）：遇到错误立即返回，保持向后兼容
/// - **收集模式**：收集所有错误后统一返回，用于 LSP 支持
///
/// 通过 `set_collect_all_errors(true)` 启用收集模式。
pub struct StatementChecker {
    /// 约束求解器
    solver: TypeConstraintSolver,
    /// 统一作用域管理器
    scope: ScopeManager,
    /// 已检查的函数
    checked_functions: HashMap<String, bool>,
    /// 重载候选存储
    overload_candidates:
        HashMap<String, Vec<crate::frontend::typecheck::overload::OverloadCandidate>>,
    /// Native 函数签名表
    native_signatures: HashMap<String, MonoType>,
    /// 是否在顶层作用域（模块级，非函数内部）
    is_top_level: bool,
    /// 累积的错误（收集模式下使用）
    collected_errors: Vec<Diagnostic>,
    /// 是否启用错误收集模式（收集所有错误而非短路返回）
    collect_all_errors: bool,
}

impl StatementChecker {
    /// 创建新的语句检查器
    pub fn new(solver: &mut TypeConstraintSolver) -> Self {
        Self {
            solver: solver.clone(),
            scope: ScopeManager::new(),
            checked_functions: HashMap::new(),
            overload_candidates: HashMap::new(),
            native_signatures: HashMap::new(),
            is_top_level: true,
            collected_errors: Vec::new(),
            collect_all_errors: false,
        }
    }

    /// 设置是否启用错误收集模式
    ///
    /// 启用后，类型检查不会在遇到第一个错误时短路返回，
    /// 而是尽可能多地收集错误，最终统一返回。
    /// 这对于 LSP 诊断非常重要，因为用户需要看到所有错误。
    pub fn set_collect_all_errors(
        &mut self,
        collect: bool,
    ) {
        self.collect_all_errors = collect;
    }

    /// 获取是否启用了错误收集模式
    pub fn is_collect_all_errors(&self) -> bool {
        self.collect_all_errors
    }

    /// 获取累积的错误
    pub fn collected_errors(&self) -> &[Diagnostic] {
        &self.collected_errors
    }

    /// 取出所有累积的错误（消耗）
    pub fn drain_collected_errors(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.collected_errors)
    }

    /// 检查是否有累积的错误
    pub fn has_collected_errors(&self) -> bool {
        !self.collected_errors.is_empty()
    }

    /// 收集错误（在收集模式下添加到列表，否则忽略）
    fn collect_error(
        &mut self,
        error: Diagnostic,
    ) {
        self.collected_errors.push(error);
    }

    /// 设置 native 函数签名表
    pub fn set_native_signatures(
        &mut self,
        signatures: HashMap<String, MonoType>,
    ) {
        self.native_signatures = signatures;
    }

    /// 获取求解器
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        &mut self.solver
    }

    /// 添加变量到当前作用域
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.scope.add_var(name, poly);
    }

    /// 获取变量（从最内层作用域开始查找）
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.scope.get_var(name)
    }

    /// 获取所有变量（从所有作用域，内层覆盖外层）
    pub fn vars(&self) -> HashMap<String, PolyType> {
        self.scope.vars()
    }

    /// 检查变量是否存在于任何作用域中
    pub fn var_exists_in_any_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scope.var_in_any_scope(name)
    }

    /// 检查变量是否存在于当前作用域
    pub fn var_exists_in_current_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scope.var_in_current_scope(name)
    }

    /// 统一变量类型并更新（用于赋值操作）
    fn unify_and_update_var(
        &mut self,
        name: &str,
        new_ty: MonoType,
    ) {
        let existing_poly = self.scope.get_var(name).unwrap().clone();
        let _ = self.solver.unify(&existing_poly.body, &new_ty);
        self.scope.update_var(name, existing_poly);
    }

    /// 进入新的作用域
    pub fn enter_scope(&mut self) {
        self.scope.enter_scope();
    }

    /// 退出当前作用域
    pub fn exit_scope(&mut self) {
        self.scope.exit_scope();
    }

    /// 检查函数定义
    ///
    /// 在收集模式下，遇到错误不会短路返回，而是继续检查后续语句，
    /// 尽可能收集所有错误。
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

        // 保存当前顶层状态，进入函数后不再是顶层
        let was_top_level = self.is_top_level;
        self.is_top_level = false;

        // 创建函数作用域
        self.scope.enter_scope();

        // 添加参数到函数作用域
        for param in params {
            let param_ty = param
                .ty
                .as_ref()
                .map(|t| MonoType::from(t.clone()))
                .unwrap_or_else(|| self.solver.new_var());
            self.scope
                .add_var(param.name.clone(), PolyType::mono(param_ty));
        }

        if self.collect_all_errors {
            // 收集模式：收集所有错误，不短路
            let mut first_err = None;
            for stmt in &body.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }

            // 检查返回表达式
            if let Some(expr) = &body.expr {
                if let Err(e) = self.check_expr(expr) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }

            // 退出函数作用域
            self.scope.exit_scope();
            self.is_top_level = was_top_level;

            match first_err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        } else {
            // 短路模式：遇到第一个错误立即返回
            let mut err = None;
            for stmt in &body.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    err = Some(e);
                    break;
                }
            }

            // 检查返回表达式
            if err.is_none() {
                if let Some(expr) = &body.expr {
                    if let Err(e) = self.check_expr(expr) {
                        err = Some(e);
                    }
                }
            }

            // 退出函数作用域
            self.scope.exit_scope();
            self.is_top_level = was_top_level;

            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
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
                var_mut,
                iterable,
                body,
                ..
            } => self.check_for_stmt(var, *var_mut, iterable, body),
            crate::frontend::core::parser::ast::StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                span,
            } => {
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
            crate::frontend::core::parser::ast::StmtKind::TypeDef {
                name, definition, ..
            } => self.check_type_def(name, definition, stmt.span),
            // 错误恢复占位符：报告错误但不 panic
            crate::frontend::core::parser::ast::StmtKind::Error(span) => Err(Box::new(
                ErrorCodeDefinition::invalid_syntax("缺失语句")
                    .at(*span)
                    .build(),
            )),
            _ => Ok(()),
        }
    }

    /// 检查类型定义
    fn check_type_def(
        &mut self,
        name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
        span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        use crate::frontend::core::parser::ast::Type;

        match definition {
            Type::Struct {
                fields, bindings, ..
            } => {
                for field in fields {
                    if let Some(default_expr) = &field.default {
                        self.check_field_default(field, default_expr, span)?;
                    }
                }

                for binding in bindings {
                    self.check_field_binding(name, binding, span)?;
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// 检查字段默认值类型是否与字段类型匹配
    fn check_field_default(
        &mut self,
        field: &crate::frontend::core::parser::ast::StructField,
        default_expr: &Expr,
        span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        let expected_type = MonoType::from(field.ty.clone());
        let actual_type = self.check_expr(default_expr)?;

        if self.solver.unify(&expected_type, &actual_type).is_err() {
            let is_numeric_promotion = matches!(
                (&expected_type, &actual_type),
                (MonoType::Float(_), MonoType::Int(_))
            );

            if !is_numeric_promotion {
                return Err(Box::new(
                    ErrorCodeDefinition::type_mismatch(
                        &format!("{}", expected_type),
                        &format!("{}", actual_type),
                    )
                    .at(span)
                    .build(),
                ));
            }
        }

        Ok(())
    }

    /// 检查绑定字段的有效性
    fn check_field_binding(
        &mut self,
        type_name: &str,
        binding: &crate::frontend::core::parser::ast::TypeBodyBinding,
        span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        use crate::frontend::core::parser::ast::BindingKind;

        match &binding.kind {
            BindingKind::External {
                function,
                positions,
            } => {
                if positions.is_empty() {
                    return Err(Box::new(
                        ErrorCodeDefinition::type_mismatch(
                            "at least one binding position",
                            "empty positions",
                        )
                        .at(span)
                        .build(),
                    ));
                }

                if let Some(func_poly) = self.scope.get_var(function).cloned() {
                    let func_mono = self.solver.instantiate(&func_poly);

                    if let MonoType::Fn {
                        params: param_types,
                        return_type: _,
                        is_async: _,
                    } = &func_mono
                    {
                        let param_count = param_types.len();

                        for &pos in positions {
                            if (pos as usize) >= param_count {
                                return Err(Box::new(
                                    ErrorCodeDefinition::type_mismatch(
                                        &format!(
                                            "binding position < {} (function '{}' has {} params)",
                                            param_count, function, param_count
                                        ),
                                        &format!("position {}", pos),
                                    )
                                    .at(span)
                                    .build(),
                                ));
                            }

                            let param_type = &param_types[pos as usize];
                            let binding_type = MonoType::TypeRef(type_name.to_string());
                            if self.solver.unify(&binding_type, param_type).is_err() {
                                return Err(Box::new(
                                    ErrorCodeDefinition::type_mismatch(
                                        &format!("{}", param_type),
                                        type_name,
                                    )
                                    .at(span)
                                    .build(),
                                ));
                            }
                        }
                    }
                }

                Ok(())
            }
            BindingKind::Anonymous {
                params,
                return_type: _,
                positions,
                body: _,
            } => {
                if positions.is_empty() {
                    return Err(Box::new(
                        ErrorCodeDefinition::type_mismatch(
                            "at least one binding position",
                            "empty positions",
                        )
                        .at(span)
                        .build(),
                    ));
                }

                let param_count = params.len();
                for &pos in positions {
                    if (pos as usize) >= param_count {
                        return Err(Box::new(
                            ErrorCodeDefinition::type_mismatch(
                                &format!(
                                    "binding position < {} (anonymous function has {} params)",
                                    param_count, param_count
                                ),
                                &format!("position {}", pos),
                            )
                            .at(span)
                            .build(),
                        ));
                    }

                    if let Some(param_ty) = &params[pos as usize].ty {
                        let param_mono = MonoType::from(param_ty.clone());
                        let binding_type = MonoType::TypeRef(type_name.to_string());
                        if self.solver.unify(&binding_type, &param_mono).is_err() {
                            return Err(Box::new(
                                ErrorCodeDefinition::type_mismatch(
                                    &format!("{}", param_mono),
                                    type_name,
                                )
                                .at(span)
                                .build(),
                            ));
                        }
                    }
                }

                Ok(())
            }
            BindingKind::DefaultExternal { .. } => {
                // DefaultExternal: 自动推导位置，此处无需额外检查
                Ok(())
            }
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
                right,
                ..
            } => {
                let right_ty = self.check_expr(right)?;

                if let Expr::Var(name, _) = left.as_ref() {
                    if self.scope.var_in_current_scope(name) {
                        let poly = self.scope.get_var(name).unwrap().clone();
                        let _ = self.solver.unify(&poly.body, &right_ty);
                    } else if self.scope.var_in_any_scope(name) {
                        self.unify_and_update_var(name, right_ty);
                    } else {
                        let ty = self.solver.new_var();
                        let _ = self.solver.unify(&ty, &right_ty);
                        self.scope.add_var(name.clone(), PolyType::mono(ty));
                    }
                }
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
        if let Some(existing) = self.scope.get_var(name) {
            if let MonoType::Struct(_) = &existing.body {
                return Err(Box::new(
                    ErrorCodeDefinition::duplicate_definition(name)
                        .at(_span)
                        .build(),
                ));
            }
        }

        // 将函数自身注册到变量环境中
        if let Some(type_ann) = type_annotation {
            if let crate::frontend::core::parser::ast::Type::Fn {
                params: param_types,
                return_type,
            } = type_ann
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
                self.scope
                    .add_var(name.to_string(), PolyType::mono(fn_type));
            }
        } else {
            let param_types: Vec<MonoType> = params
                .iter()
                .map(|p| {
                    p.ty.as_ref()
                        .map(|t| MonoType::from(t.clone()))
                        .unwrap_or_else(|| self.solver.new_var())
                })
                .collect();

            let fn_type = MonoType::Fn {
                params: param_types,
                return_type: Box::new(self.solver.new_var()),
                is_async: false,
            };
            self.scope
                .add_var(name.to_string(), PolyType::mono(fn_type));
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
            let _ = self.check_expr(&fn_def_expr);
        }

        self.check_fn_def(name, params, &body)
    }

    /// 检查变量语句
    fn check_var_stmt(
        &mut self,
        name: &str,
        type_annotation: Option<&crate::frontend::core::parser::ast::Type>,
        initializer: Option<&Expr>,
    ) -> Result<(), Box<Diagnostic>> {
        // 顶层变量不支持函数调用检测
        if self.is_top_level {
            if let Some(init_expr) = initializer {
                if self.contains_function_call(init_expr) {
                    return Err(Box::new(
                        ErrorCodeDefinition::top_level_function_call().build(),
                    ));
                }
            }
        }

        let ty = match (initializer, type_annotation) {
            (Some(init_expr), Some(type_ann)) => {
                let init_ty = self.check_expr(init_expr)?;
                let ann_ty = MonoType::from(type_ann.clone());
                let _ = self.solver.unify(&init_ty, &ann_ty);
                ann_ty
            }
            (Some(init_expr), None) => self.check_expr(init_expr)?,
            (None, Some(type_ann)) => MonoType::from(type_ann.clone()),
            (None, None) => self.solver.new_var(),
        };

        if self.scope.var_in_current_scope(name) {
            let existing_poly = self.scope.get_var(name).unwrap().clone();
            let _ = self.solver.unify(&existing_poly.body, &ty);
            return Ok(());
        }

        if self.scope.var_in_any_scope(name) {
            self.unify_and_update_var(name, ty);
            return Ok(());
        }

        self.scope.add_var(name.to_string(), PolyType::mono(ty));
        Ok(())
    }

    /// 检测表达式是否包含函数调用（递归）
    fn contains_function_call(
        &self,
        expr: &Expr,
    ) -> bool {
        match expr {
            Expr::Call { .. } => true,
            Expr::BinOp { left, right, .. } => {
                self.contains_function_call(left) || self.contains_function_call(right)
            }
            Expr::UnOp { expr: inner, .. } => self.contains_function_call(inner),
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.contains_function_call(condition)
                    || self.contains_function_call_block(then_branch)
                    || elif_branches.iter().any(|(cond, block)| {
                        self.contains_function_call(cond)
                            || self.contains_function_call_block(block)
                    })
                    || else_branch
                        .as_ref()
                        .map(|b| self.contains_function_call_block(b))
                        .unwrap_or(false)
            }
            Expr::Match {
                expr: match_expr,
                arms,
                ..
            } => {
                self.contains_function_call(match_expr)
                    || arms
                        .iter()
                        .any(|arm| self.contains_function_call_block(&arm.body))
            }
            Expr::For { iterable, body, .. } => {
                self.contains_function_call(iterable) || self.contains_function_call_block(body)
            }
            Expr::While {
                condition, body, ..
            } => self.contains_function_call(condition) || self.contains_function_call_block(body),
            Expr::Block(block) => self.contains_function_call_block(block),
            Expr::Lit(..) => false,
            Expr::Var(..) => false,
            Expr::FnDef { .. } => false,
            Expr::Index {
                expr: obj, index, ..
            } => self.contains_function_call(obj) || self.contains_function_call(index),
            Expr::FieldAccess { expr: obj, .. } => self.contains_function_call(obj),
            _ => false,
        }
    }

    /// 检测代码块是否包含函数调用
    fn contains_function_call_block(
        &self,
        block: &Block,
    ) -> bool {
        block
            .stmts
            .iter()
            .any(|stmt| self.contains_function_call_stmt(stmt))
            || block
                .expr
                .as_ref()
                .map(|e| self.contains_function_call(e))
                .unwrap_or(false)
    }

    /// 检测语句是否包含函数调用
    fn contains_function_call_stmt(
        &self,
        stmt: &Stmt,
    ) -> bool {
        match &stmt.kind {
            crate::frontend::core::parser::ast::StmtKind::Expr(expr) => {
                self.contains_function_call(expr)
            }
            crate::frontend::core::parser::ast::StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.contains_function_call(condition)
                    || self.contains_function_call_block(then_branch)
                    || elif_branches.iter().any(|(cond, block)| {
                        self.contains_function_call(cond)
                            || self.contains_function_call_block(block)
                    })
                    || else_branch
                        .as_ref()
                        .map(|b| self.contains_function_call_block(b))
                        .unwrap_or(false)
            }
            crate::frontend::core::parser::ast::StmtKind::For { iterable, body, .. } => {
                self.contains_function_call(iterable) || self.contains_function_call_block(body)
            }
            _ => false,
        }
    }

    /// 检查 for 语句
    ///
    /// 在收集模式下，循环体内的错误会被收集而非短路。
    fn check_for_stmt(
        &mut self,
        var: &str,
        var_mut: bool,
        iterable: &Expr,
        body: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        let iter_ty = self.check_expr(iterable)?;
        let elem_ty = match iter_ty {
            MonoType::List(elem) => *elem,
            MonoType::String => MonoType::Char,
            _ => self.solver.new_var(),
        };

        self.scope.enter_scope();

        // 遮蔽检查
        if self.scope.var_in_any_scope(var) {
            self.scope.exit_scope();
            return Err(Box::new(
                ErrorCodeDefinition::variable_shadowing(var).build(),
            ));
        }

        self.scope.add_var(var.to_string(), PolyType::mono(elem_ty));

        let _ = var_mut;

        if self.collect_all_errors {
            let mut first_err = None;
            for stmt in &body.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }
            if let Some(expr) = &body.expr {
                if let Err(e) = self.check_expr(expr) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }
            self.scope.exit_scope();
            match first_err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        } else {
            let mut err = None;
            for stmt in &body.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    err = Some(e);
                    break;
                }
            }
            if err.is_none() {
                if let Some(expr) = &body.expr {
                    if let Err(e) = self.check_expr(expr) {
                        err = Some(e);
                    }
                }
            }
            self.scope.exit_scope();
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
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

    /// 检查代码块（创建独立作用域）
    ///
    /// 在收集模式下，代码块内的错误会被收集而非短路。
    fn check_block(
        &mut self,
        block: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        self.scope.enter_scope();

        if self.collect_all_errors {
            let mut first_err = None;
            for stmt in &block.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }
            if let Some(expr) = &block.expr {
                if let Err(e) = self.check_expr(expr) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }
            self.scope.exit_scope();
            match first_err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        } else {
            let mut err = None;
            for stmt in &block.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    err = Some(e);
                    break;
                }
            }
            if err.is_none() {
                if let Some(expr) = &block.expr {
                    if let Err(e) = self.check_expr(expr) {
                        err = Some(e);
                    }
                }
            }
            self.scope.exit_scope();
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
    }

    /// 检查表达式
    ///
    /// 直接使用共享的 ScopeManager，无需拷贝变量。
    pub fn check_expr(
        &mut self,
        expr: &Expr,
    ) -> Result<MonoType, Box<Diagnostic>> {
        let mut inferrer = super::ExpressionInferrer::with_native_signatures(
            &mut self.scope,
            &mut self.solver,
            &self.overload_candidates,
            &self.native_signatures,
        );

        inferrer.infer_expr(expr).map_err(Box::new)
    }
}
