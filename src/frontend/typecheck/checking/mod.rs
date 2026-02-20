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
    /// 变量环境（作用域栈）
    scopes: Vec<HashMap<String, PolyType>>,
    /// 已检查的函数
    checked_functions: HashMap<String, bool>,
    /// 重载候选存储
    overload_candidates:
        HashMap<String, Vec<crate::frontend::typecheck::overload::OverloadCandidate>>,
    /// Native 函数签名表
    native_signatures: HashMap<String, MonoType>,
    /// 是否在顶层作用域（模块级，非函数内部）
    is_top_level: bool,
}

impl BodyChecker {
    /// 创建新的函数体检查器
    pub fn new(solver: &mut TypeConstraintSolver) -> Self {
        Self {
            solver: solver.clone(),
            scopes: vec![HashMap::new()],
            checked_functions: HashMap::new(),
            overload_candidates: HashMap::new(),
            native_signatures: HashMap::new(),
            is_top_level: true, // 默认在顶层
        }
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
        self.scopes.last_mut().unwrap().insert(name, poly);
    }

    /// 获取变量（从最内层作用域开始查找）
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

    /// 获取所有变量（从所有作用域，内层覆盖外层）
    pub fn vars(&self) -> HashMap<String, PolyType> {
        let mut result = HashMap::new();
        for scope in &self.scopes {
            for (name, poly) in scope {
                result.insert(name.clone(), poly.clone());
            }
        }
        result
    }

    /// 检查变量是否存在于任何作用域中
    pub fn var_exists_in_any_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scopes.iter().any(|scope| scope.contains_key(name))
    }

    /// 检查变量是否存在于当前作用域
    pub fn var_exists_in_current_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scopes.last().map_or(false, |s| s.contains_key(name))
    }

    /// 在现有作用域中更新变量（从内层到外层搜索）
    fn update_var(
        &mut self,
        name: &str,
        poly: PolyType,
    ) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), poly);
                return;
            }
        }
        // 未找到则添加到当前作用域
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), poly);
    }

    /// 进入新的作用域
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 退出当前作用域
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
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

        // 保存当前顶层状态，进入函数后不再是顶层
        let was_top_level = self.is_top_level;
        self.is_top_level = false;

        // 创建函数作用域
        self.enter_scope();

        // 添加参数到函数作用域
        for param in params {
            let param_ty = param
                .ty
                .as_ref()
                .map(|t| MonoType::from(t.clone()))
                .unwrap_or_else(|| self.solver.new_var());
            self.add_var(param.name.clone(), PolyType::mono(param_ty));
        }

        // 检查函数体语句
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
        self.exit_scope();

        // 恢复顶层状态
        self.is_top_level = was_top_level;

        match err {
            Some(e) => Err(e),
            None => Ok(()),
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
            crate::frontend::core::parser::ast::StmtKind::TypeDef {
                name, definition, ..
            } => self.check_type_def(name, definition, stmt.span),
            _ => Ok(()),
        }
    }

    /// 检查类型定义
    ///
    /// 验证：
    /// - 默认值字段的类型与字段类型一致
    /// - 绑定引用的函数和位置有效
    fn check_type_def(
        &mut self,
        name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
        span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        use crate::frontend::core::parser::ast::Type;

        match definition {
            Type::Struct { fields, bindings } => {
                // 检查默认值字段
                for field in fields {
                    if let Some(default_expr) = &field.default {
                        self.check_field_default(field, default_expr, span)?;
                    }
                }

                // 检查绑定字段（传入类型名用于位置类型匹配验证）
                for binding in bindings {
                    self.check_field_binding(name, binding, span)?;
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// 检查字段默认值类型是否与字段类型匹配
    ///
    /// 支持隐式数值提升（Int → Float），符合 RFC-010 中 `x: Float = 0` 的用法
    fn check_field_default(
        &mut self,
        field: &crate::frontend::core::parser::ast::StructField,
        default_expr: &Expr,
        span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        let expected_type = MonoType::from(field.ty.clone());
        let actual_type = self.check_expr(default_expr)?;

        // 尝试统一默认值类型与字段声明类型
        if self.solver.unify(&expected_type, &actual_type).is_err() {
            // 检查是否可以隐式数值提升（Int → Float）
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
            // 隐式提升：Int → Float 在默认值上下文中允许
        }

        Ok(())
    }

    /// 检查绑定字段的有效性
    ///
    /// 按 RFC-004 规定验证：
    /// 1. 位置索引列表非空
    /// 2. 引用的函数存在时：位置索引在参数范围内
    /// 3. 绑定位置的参数类型与当前类型兼容
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
                // 验证位置索引非空
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

                // 验证引用的函数名存在，并检查位置索引和类型匹配
                if let Some(func_poly) = self.get_var(function).cloned() {
                    let func_mono = self.solver.instantiate(&func_poly);

                    if let MonoType::Fn {
                        params: param_types,
                        return_type: _,
                        is_async: _,
                    } = &func_mono
                    {
                        let param_count = param_types.len();

                        for &pos in positions {
                            // 验证位置索引在参数范围内（RFC-004：支持从0开始的正数索引）
                            if pos >= param_count {
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

                            // 验证绑定位置的参数类型与当前类型匹配（RFC-004 类型安全）
                            let param_type = &param_types[pos];
                            let binding_type = MonoType::TypeRef(type_name.to_string());
                            if self.solver.unify(&binding_type, param_type).is_err() {
                                // 类型不匹配：绑定位置的参数类型与当前类型不兼容
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
                // 函数未在当前作用域中找到时，跳过深度检查
                // 函数可能在外层作用域或后续定义中

                Ok(())
            }
            BindingKind::Anonymous {
                params,
                return_type: _,
                positions,
                body: _,
            } => {
                // 验证位置索引非空
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

                // 验证位置索引在匿名函数参数范围内
                let param_count = params.len();
                for &pos in positions {
                    if pos >= param_count {
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

                    // 验证绑定位置的参数类型与当前类型匹配
                    if let Some(param_ty) = &params[pos].ty {
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
                // 首先推断右侧表达式的类型
                let right_ty = self.check_expr(right)?;

                // 如果左侧是变量，将其类型与右侧类型统一
                if let Expr::Var(name, _) = left.as_ref() {
                    if self.var_exists_in_current_scope(name) {
                        // 当前作用域存在，是赋值操作
                        let poly = self.get_var(name).unwrap().clone();
                        let _ = self.solver.unify(&poly.body, &right_ty);
                    } else if self.var_exists_in_any_scope(name) {
                        // 外层作用域存在，遮蔽错误
                        return Err(Box::new(
                            ErrorCodeDefinition::variable_shadowing(name).build(),
                        ));
                    } else {
                        // 变量不存在，创建新变量并与右侧类型统一
                        let ty = self.solver.new_var();
                        let _ = self.solver.unify(&ty, &right_ty);
                        self.add_var(name.clone(), PolyType::mono(ty));
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
        if let Some(existing) = self.get_var(name) {
            if let MonoType::Struct(_) = &existing.body {
                return Err(Box::new(
                    ErrorCodeDefinition::duplicate_definition(name)
                        .at(_span)
                        .build(),
                ));
            }
        }

        // 将函数自身注册到变量环境中（支持嵌套函数的前向引用和递归调用）
        // 注意：函数参数由 check_fn_def 在函数作用域内添加
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
                self.add_var(name.to_string(), PolyType::mono(fn_type));
            }
        } else {
            // 没有类型注解时，也需要添加函数到作用域
            // 从参数创建类型变量
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
            self.add_var(name.to_string(), PolyType::mono(fn_type));
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

    /// 检查变量语句（mut 声明或隐式赋值）
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
            (Some(init_expr), _) => self.check_expr(init_expr)?,
            (None, Some(type_ann)) => MonoType::from(type_ann.clone()),
            (None, None) => self.solver.new_var(),
        };

        if self.var_exists_in_current_scope(name) {
            // 当前作用域已存在 → 重新赋值（统一类型）
            // 可变性检查由 MutChecker 在 IR 阶段处理
            let existing_poly = self.get_var(name).unwrap().clone();
            let _ = self.solver.unify(&existing_poly.body, &ty);
            return Ok(());
        }

        // 仅外层作用域存在 → 遮蔽错误
        if self.var_exists_in_any_scope(name) {
            return Err(Box::new(
                ErrorCodeDefinition::variable_shadowing(name).build(),
            ));
        }

        // 不存在 → 新变量
        self.add_var(name.to_string(), PolyType::mono(ty));
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
            // 以下类型不包含函数调用
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

        // 创建 for 循环作用域
        self.enter_scope();

        // 遮蔽检查：如果变量已存在于外层作用域，报错
        if self.var_exists_in_any_scope(var) {
            self.exit_scope();
            return Err(Box::new(
                ErrorCodeDefinition::variable_shadowing(var).build(),
            ));
        }

        self.add_var(var.to_string(), PolyType::mono(elem_ty));

        // var_mut 在 IR 生成阶段使用，用于决定循环变量是否可变
        // for i in 1..5 - i 不可变
        // for mut i in 1..5 - i 可变
        let _ = var_mut;

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

        // 退出 for 循环作用域（循环变量被销毁）
        self.exit_scope();

        match err {
            Some(e) => Err(e),
            None => Ok(()),
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
    fn check_block(
        &mut self,
        block: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        self.enter_scope();

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

        self.exit_scope();

        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// 检查表达式
    pub fn check_expr(
        &mut self,
        expr: &Expr,
    ) -> Result<MonoType, Box<Diagnostic>> {
        let all_vars = self.vars();
        let overload_candidates_clone = self.overload_candidates.clone();
        let native_signatures_clone = self.native_signatures.clone();
        let mut inferrer =
            crate::frontend::typecheck::inference::ExprInferrer::with_native_signatures(
                &mut self.solver,
                &overload_candidates_clone,
                &native_signatures_clone,
            );

        for (name, poly) in all_vars {
            inferrer.add_var(name, poly);
        }

        let result = inferrer.infer_expr(expr).map_err(Box::new)?;

        // 同步所有变量的类型（包括修改过的，如赋值统一后的类型）
        for (name, poly) in inferrer.get_all_vars() {
            self.update_var(&name, poly);
        }

        Ok(result)
    }
}
