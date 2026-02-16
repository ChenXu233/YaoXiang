#![allow(clippy::result_large_err)]

//! 表达式类型推断
//!
//! 实现各种表达式的类型推断

use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::frontend::core::parser::ast::{BinOp, UnOp};
use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::typecheck::overload;
use std::collections::HashMap;

/// 表达式类型推断器
pub struct ExprInferrer<'a> {
    solver: &'a mut TypeConstraintSolver,
    /// 变量环境栈：每一层是一个作用域
    scopes: Vec<HashMap<String, PolyType>>,
    /// 当前活跃的循环标签
    loop_labels: Vec<String>,
    /// 重载候选存储引用
    overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
}

impl<'a> ExprInferrer<'a> {
    /// 创建新的表达式推断器
    pub fn new(
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
    ) -> Self {
        Self {
            solver,
            scopes: vec![HashMap::new()], // Global scope
            loop_labels: Vec::new(),
            overload_candidates,
        }
    }

    /// 获取求解器引用（可变）
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        self.solver
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

    /// 获取所有变量（从所有作用域）
    pub fn get_all_vars(&self) -> HashMap<String, PolyType> {
        let mut result = HashMap::new();
        for scope in &self.scopes {
            for (name, poly) in scope {
                if !result.contains_key(name) {
                    result.insert(name.clone(), poly.clone());
                }
            }
        }
        result
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

    /// 获取当前作用域层级
    pub fn scope_level(&self) -> usize {
        self.scopes.len()
    }

    /// 进入循环并注册标签（如果有）
    pub fn enter_loop(
        &mut self,
        label: Option<&str>,
    ) {
        if let Some(l) = label {
            self.loop_labels.push(l.to_string());
        }
    }

    /// 退出循环并移除标签（如果有）
    pub fn exit_loop(
        &mut self,
        label: Option<&str>,
    ) {
        if let Some(l) = label {
            if let Some(pos) = self.loop_labels.iter().rposition(|x| x == l) {
                self.loop_labels.remove(pos);
            }
        }
    }

    /// 检查标签是否存在
    pub fn has_label(
        &self,
        label: &str,
    ) -> bool {
        self.loop_labels.contains(&label.to_string())
    }

    /// 推断字面量表达式类型
    pub fn infer_literal(
        &mut self,
        lit: &crate::frontend::core::lexer::tokens::Literal,
    ) -> Result<MonoType> {
        let ty = match lit {
            crate::frontend::core::lexer::tokens::Literal::Int(_) => MonoType::Int(64),
            crate::frontend::core::lexer::tokens::Literal::Float(_) => MonoType::Float(64),
            crate::frontend::core::lexer::tokens::Literal::Bool(_) => MonoType::Bool,
            crate::frontend::core::lexer::tokens::Literal::Char(_) => MonoType::Char,
            crate::frontend::core::lexer::tokens::Literal::String(_) => MonoType::String,
        };
        Ok(ty)
    }

    /// 推断二元操作符表达式类型
    pub fn infer_binary(
        &mut self,
        op: &BinOp,
        left: &MonoType,
        right: &MonoType,
    ) -> Result<MonoType> {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                if let (MonoType::Int(_), MonoType::Int(_)) = (left, right) {
                    Ok(left.clone())
                } else if let (MonoType::Float(_), MonoType::Float(_)) = (left, right) {
                    Ok(left.clone())
                } else if let (MonoType::String, MonoType::String) = (left, right) {
                    Ok(MonoType::String)
                } else if let (MonoType::List(left_elem), MonoType::List(right_elem)) =
                    (left, right)
                {
                    // 列表拼接：统一元素类型，返回统一后的列表类型
                    let _ = self.solver.unify(left_elem, right_elem);
                    let elem_ty = self.solver.resolve_type(left_elem);
                    Ok(MonoType::List(Box::new(elem_ty)))
                } else {
                    // 对于字符串连接和其他类型，使用类型变量
                    let var = self.solver.new_var();
                    Ok(var)
                }
            }
            BinOp::Mod => {
                // 取模运算：两个操作数必须是相同数值类型，返回相同类型
                if let (MonoType::Int(_), MonoType::Int(_)) = (left, right) {
                    Ok(left.clone())
                } else if let (MonoType::Float(_), MonoType::Float(_)) = (left, right) {
                    Ok(left.clone())
                } else {
                    // 尝试统一两个类型（忽略错误，返回左操作数类型）
                    let _ = self.solver.unify(left, right);
                    Ok(left.clone())
                }
            }
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                // 比较运算：尝试统一两个操作数
                let _ = self.solver.unify(left, right);
                Ok(MonoType::Bool)
            }
            BinOp::And | BinOp::Or => {
                if let (MonoType::Bool, MonoType::Bool) = (left, right) {
                    Ok(MonoType::Bool)
                } else {
                    Err(ErrorCodeDefinition::logical_operand_type_mismatch(
                        &format!("{}", left),
                        &format!("{}", right),
                    )
                    .build())
                }
            }
            BinOp::Range => {
                // 范围表达式的类型是列表，元素类型与操作数相同
                let elem_ty = if left == right {
                    left.clone()
                } else {
                    // 尝试统一
                    let _ = self.solver.unify(left, right);
                    left.clone()
                };
                Ok(MonoType::List(Box::new(elem_ty)))
            }
            BinOp::Assign => {
                // 赋值表达式的类型是 Void
                // 字段可变性检查在 MutChecker 中进行
                Ok(MonoType::Void)
            }
        }
    }

    /// 推断一元操作符表达式类型
    pub fn infer_unary(
        &mut self,
        op: &UnOp,
        expr: &MonoType,
    ) -> Result<MonoType> {
        match op {
            UnOp::Neg => Ok(expr.clone()),
            UnOp::Pos => Ok(expr.clone()),
            UnOp::Not => {
                if *expr == MonoType::Bool {
                    Ok(MonoType::Bool)
                } else {
                    Err(
                        ErrorCodeDefinition::logical_not_type_mismatch(&format!("{}", expr))
                            .build(),
                    )
                }
            }
            UnOp::Deref => {
                // 解引用：*ptr 返回内部类型
                // ptr 应该是 *T 形式
                if let MonoType::TypeRef(inner) = expr {
                    // 提取内部类型（去掉 * 前缀）
                    let inner_type = inner.trim_start_matches('*').to_string();
                    Ok(MonoType::TypeRef(inner_type))
                } else {
                    Err(ErrorCodeDefinition::invalid_deref(&format!("{}", expr)).build())
                }
            }
        }
    }

    /// 推断表达式的类型
    pub fn infer_expr(
        &mut self,
        expr: &crate::frontend::core::parser::ast::Expr,
    ) -> Result<MonoType> {
        match expr {
            // 字面量
            crate::frontend::core::parser::ast::Expr::Lit(lit, _) => self.infer_literal(lit),

            // 变量
            crate::frontend::core::parser::ast::Expr::Var(name, span) => {
                let poly = self.get_var(name).cloned();
                if let Some(poly) = poly {
                    let ty = self.solver.instantiate(&poly);
                    let resolved = self.solver.resolve_type(&ty);
                    Ok(resolved)
                } else {
                    // 返回错误
                    Err(ErrorCodeDefinition::unknown_variable(name)
                        .at(*span)
                        .build())
                }
            }

            // 二元运算
            crate::frontend::core::parser::ast::Expr::BinOp {
                op, left, right, ..
            } => {
                // 先推断右操作数类型
                let right_ty = self.infer_expr(right)?;

                // 处理赋值操作：将左操作数（变量）的类型与右操作数类型统一
                if matches!(op, BinOp::Assign) {
                    if let crate::frontend::core::parser::ast::Expr::Var(var_name, _) =
                        left.as_ref()
                    {
                        // 获取变量的当前类型
                        if let Some(poly) = self.get_var(var_name).cloned() {
                            // 将变量类型与右操作数类型统一（忽略错误，因为类型不匹配会在其他地方报错）
                            let _ = self.solver.unify(&poly.body, &right_ty);
                        }
                    }
                    // 赋值表达式的类型是 Void
                    return Ok(MonoType::Void);
                }

                // 非赋值操作：正常推断
                let left_ty = self.infer_expr(left)?;
                self.infer_binary(op, &left_ty, &right_ty)
            }

            // 一元运算
            crate::frontend::core::parser::ast::Expr::UnOp { op, expr, .. } => {
                let expr_ty = self.infer_expr(expr)?;
                self.infer_unary(op, &expr_ty)
            }

            // 元组
            crate::frontend::core::parser::ast::Expr::Tuple(elems, _) => {
                let types: Result<Vec<_>> = elems.iter().map(|e| self.infer_expr(e)).collect();
                Ok(MonoType::Tuple(types?))
            }

            // 列表
            crate::frontend::core::parser::ast::Expr::List(elems, _) => {
                if elems.is_empty() {
                    // 空列表：元素类型是类型变量
                    let elem_ty = self.solver.new_var();
                    Ok(MonoType::List(Box::new(elem_ty)))
                } else {
                    // 非空列表：推断所有元素类型并统一
                    let mut iter = elems.iter();
                    let first = iter.next().expect("non-empty list must have first element");
                    let mut elem_ty = self.infer_expr(first)?;
                    for e in iter {
                        let ty = self.infer_expr(e)?;
                        let _ = self.solver.unify(&elem_ty, &ty);
                        elem_ty = self.solver.resolve_type(&elem_ty);
                    }
                    Ok(MonoType::List(Box::new(elem_ty)))
                }
            }

            // 字典
            crate::frontend::core::parser::ast::Expr::Dict(pairs, _) => {
                if pairs.is_empty() {
                    // 空字典：键值类型都是类型变量
                    let key_ty = self.solver.new_var();
                    let value_ty = self.solver.new_var();
                    Ok(MonoType::Dict(Box::new(key_ty), Box::new(value_ty)))
                } else {
                    // 非空字典
                    let mut key_ty = None;
                    let mut value_ty = None;
                    for (k, v) in pairs {
                        let k_type = self.infer_expr(k)?;
                        let v_type = self.infer_expr(v)?;
                        if key_ty.is_none() {
                            key_ty = Some(k_type);
                        }
                        if value_ty.is_none() {
                            value_ty = Some(v_type);
                        }
                    }
                    Ok(MonoType::Dict(
                        Box::new(key_ty.unwrap_or_else(|| self.solver.new_var())),
                        Box::new(value_ty.unwrap_or_else(|| self.solver.new_var())),
                    ))
                }
            }

            // 下标访问
            crate::frontend::core::parser::ast::Expr::Index {
                expr: container,
                index,
                ..
            } => {
                let container_ty = self.infer_expr(container)?;
                match container_ty {
                    MonoType::List(elem_ty) => Ok(*elem_ty),
                    MonoType::Dict(_key_ty, value_ty) => Ok(*value_ty),
                    MonoType::Tuple(types) => {
                        // 静态元组下标检查
                        if let crate::frontend::core::parser::ast::Expr::Lit(
                            crate::frontend::core::lexer::tokens::Literal::Int(i),
                            _,
                        ) = index.as_ref()
                        {
                            if *i >= 0 && (*i as usize) < types.len() {
                                Ok(types[*i as usize].clone())
                            } else {
                                Err(ErrorCodeDefinition::index_out_of_bounds(
                                    types.len(),
                                    *i as usize,
                                )
                                .build())
                            }
                        } else {
                            // 动态下标：返回类型变量
                            Ok(self.solver.new_var())
                        }
                    }
                    _ => {
                        // 动态下标：返回类型变量
                        Ok(self.solver.new_var())
                    }
                }
            }

            // 字段访问
            crate::frontend::core::parser::ast::Expr::FieldAccess {
                expr: obj, field, ..
            } => {
                let obj_ty = self.infer_expr(obj)?;
                match obj_ty {
                    MonoType::Struct(struct_type) => {
                        for (field_name, field_ty) in &struct_type.fields {
                            if field_name == field {
                                return Ok(field_ty.clone());
                            }
                        }
                        Err(ErrorCodeDefinition::field_not_found(field, &struct_type.name).build())
                    }
                    _ => Err(ErrorCodeDefinition::field_access_on_non_struct(&format!(
                        "{}",
                        obj_ty
                    ))
                    .build()),
                }
            }

            // 函数调用
            crate::frontend::core::parser::ast::Expr::Call { func, args, .. } => {
                // 1. 首先推断函数表达式
                let func_ty = self.infer_expr(func)?;

                // 2. 推断所有参数类型
                let arg_types: Vec<MonoType> = args
                    .iter()
                    .map(|arg| self.infer_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;

                // 3. 尝试重载解析
                // 检查 func 是否为标识符
                if let crate::frontend::core::parser::ast::Expr::Var(ref name, _) = **func {
                    // 尝试从重载候选解析
                    if overload::has_overloads(self.overload_candidates, name) {
                        match overload::resolve_overload_from_env(
                            self.overload_candidates,
                            name,
                            &arg_types,
                        ) {
                            Ok(candidate) => {
                                // 重载解析成功，返回匹配的返回类型
                                return Ok(candidate.return_type.clone());
                            }
                            Err(_e) => {
                                // 重载解析失败，尝试泛型 fallback
                                if let Some(generic_candidate) = overload::resolve_generic_fallback(
                                    self.overload_candidates,
                                    name,
                                    &arg_types,
                                ) {
                                    // 泛型实例化成功，返回实例化后的返回类型
                                    let return_type = overload::instantiate_return_type(
                                        generic_candidate,
                                        &arg_types,
                                    );
                                    return Ok(return_type);
                                }
                                // 无匹配，返回类型变量
                                // 重载解析失败，返回类型变量（让后续类型检查捕获错误）
                                // 这里不直接报错，因为后续可能有其他错误信息
                                return Ok(self.solver.new_var());
                            }
                        }
                    }
                }
                // 4. 非重载情况或重载失败：
                // 函数调用返回类型变量（具体类型需要在调用点确定）
                // 如果函数类型已知，使用其返回类型
                if let MonoType::Fn { return_type, .. } = func_ty {
                    return Ok(*return_type);
                }
                // 否则返回类型变量（具体类型在调用点确定）
                Ok(self.solver.new_var())
            }

            // If 表达式
            crate::frontend::core::parser::ast::Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                // 推断条件类型（必须是 Bool）
                let cond_ty = self.infer_expr(condition)?;
                if cond_ty != MonoType::Bool {
                    return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                        "{}",
                        cond_ty
                    ))
                    .build());
                }

                // 推断 then 分支
                let _then_ty = self.infer_block(then_branch, true, None)?;

                // 推断 elif 分支
                for (elif_cond, elif_block) in elif_branches {
                    let elif_cond_ty = self.infer_expr(elif_cond)?;
                    if elif_cond_ty != MonoType::Bool {
                        return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                            "{}",
                            elif_cond_ty
                        ))
                        .build());
                    }
                    let _ = self.infer_block(elif_block, true, None)?;
                }

                // 推断 else 分支（如果有）
                if let Some(else_block) = else_branch {
                    self.infer_block(else_block, true, None)
                } else {
                    // 没有 else 分支时返回 Void
                    Ok(MonoType::Void)
                }
            }

            // While 表达式
            crate::frontend::core::parser::ast::Expr::While {
                condition,
                body,
                label,
                ..
            } => {
                // 推断条件类型
                let cond_ty = self.infer_expr(condition)?;
                if cond_ty != MonoType::Bool {
                    return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                        "{}",
                        cond_ty
                    ))
                    .build());
                }

                // 注册循环标签
                self.enter_loop(label.as_deref());

                // 推断循环体
                let result = self.infer_block(body, true, None);

                // 移除循环标签
                self.exit_loop(label.as_deref());

                result?;
                // While 表达式总是返回 Void
                Ok(MonoType::Void)
            }

            // For 循环
            crate::frontend::core::parser::ast::Expr::For {
                var,
                iterable,
                body,
                label,
                ..
            } => {
                // 推断可迭代对象
                let _iter_ty = self.infer_expr(iterable)?;

                // 注册循环标签
                self.enter_loop(label.as_deref());

                // 进入循环体作用域，添加迭代变量
                self.enter_scope();
                self.add_var(var.clone(), PolyType::mono(MonoType::Char));

                // 推断循环体
                let result = self.infer_block(body, true, None);

                self.exit_scope();
                // 移除循环标签
                self.exit_loop(label.as_deref());
                result
            }

            // Return 表达式
            crate::frontend::core::parser::ast::Expr::Return(expr, _) => {
                if let Some(e) = expr {
                    let _ = self.infer_expr(e)?;
                }
                Ok(MonoType::Void)
            }

            // Break 表达式
            crate::frontend::core::parser::ast::Expr::Break(label, _) => {
                // Break 表达式总是返回 Void
                // 如果有标签，需要验证标签是否有效
                if let Some(l) = label {
                    if !self.has_label(l) {
                        return Err(ErrorCodeDefinition::unknown_label(l).build());
                    }
                }
                Ok(MonoType::Void)
            }

            // Continue 表达式
            crate::frontend::core::parser::ast::Expr::Continue(label, _) => {
                // Continue 表达式总是返回 Void
                // 如果有标签，需要验证标签是否有效
                if let Some(l) = label {
                    if !self.has_label(l) {
                        return Err(ErrorCodeDefinition::unknown_label(l).build());
                    }
                }
                Ok(MonoType::Void)
            }

            // Cast 表达式
            crate::frontend::core::parser::ast::Expr::Cast {
                expr, target_type, ..
            } => {
                let _ = self.infer_expr(expr)?;
                // 将 AST Type 转换为 MonoType
                let target_mono: MonoType = target_type.clone().into();
                Ok(target_mono)
            }

            // Block 表达式
            crate::frontend::core::parser::ast::Expr::Block(block) => {
                self.infer_block(block, true, None)
            }

            // 函数定义
            crate::frontend::core::parser::ast::Expr::FnDef {
                name,
                params,
                return_type,
                body,
                ..
            } => {
                // 为函数参数创建类型变量并添加到作用域
                self.enter_scope();
                for param in params {
                    let param_ty = self.solver.new_var();
                    self.add_var(param.name.clone(), PolyType::mono(param_ty));
                }

                // 推断函数体
                let ret_mono: MonoType = return_type.clone().map_or(MonoType::Void, |t| t.into());
                let body_ty = self.infer_block(body, true, Some(&ret_mono))?;

                // 验证返回类型
                if return_type.is_some() {
                    // 已经在上面计算了 ret_mono
                    // TODO: 检查 body_ty 是否可以赋值给 ret_mono
                    let _ = self.solver.unify(&body_ty, &ret_mono);
                }

                self.exit_scope();

                // 构建函数类型
                let param_types: Vec<MonoType> =
                    params.iter().map(|_| self.solver.new_var()).collect();
                let return_type_box =
                    Box::new(return_type.clone().map_or(MonoType::Void, |t| t.into()));

                // 将函数类型添加到作用域
                let fn_type = MonoType::Fn {
                    params: param_types,
                    return_type: return_type_box,
                    is_async: false,
                };
                self.add_var(name.clone(), PolyType::mono(fn_type.clone()));

                Ok(fn_type)
            }

            // Lambda 表达式
            crate::frontend::core::parser::ast::Expr::Lambda { params, body, .. } => {
                // 为参数创建作用域
                self.enter_scope();
                for param in params {
                    let param_ty = self.solver.new_var();
                    self.add_var(param.name.clone(), PolyType::mono(param_ty));
                }

                // 推断函数体
                let body_ty = self.infer_block(body, true, None)?;

                self.exit_scope();

                // 构建函数类型
                let param_types: Vec<MonoType> =
                    params.iter().map(|_| self.solver.new_var()).collect();

                Ok(MonoType::Fn {
                    params: param_types,
                    return_type: Box::new(body_ty),
                    is_async: false,
                })
            }

            // Match 表达式（简化实现）
            crate::frontend::core::parser::ast::Expr::Match { expr, .. } => {
                let _expr_ty = self.infer_expr(expr)?;
                // 简化：返回类型变量
                Ok(self.solver.new_var())
            }

            // Try 表达式（简化实现）
            crate::frontend::core::parser::ast::Expr::Try { expr, .. } => self.infer_expr(expr),

            // Ref 表达式
            crate::frontend::core::parser::ast::Expr::Ref { expr, .. } => {
                let expr_ty = self.infer_expr(expr)?;
                Ok(MonoType::Arc(Box::new(expr_ty)))
            }

            // Unsafe 块
            crate::frontend::core::parser::ast::Expr::Unsafe { body, .. } => {
                // 推断块内最后一个表达式的类型作为 unsafe 块的类型
                if let Some(last_expr) = &body.expr {
                    self.infer_expr(last_expr)
                } else {
                    Ok(MonoType::Void)
                }
            }

            // ListComp 表达式
            crate::frontend::core::parser::ast::Expr::ListComp {
                element,
                var,
                iterable,
                condition,
                ..
            } => {
                // 推断可迭代对象
                let _iter_ty = self.infer_expr(iterable)?;

                // 进入列表推导作用域
                self.enter_scope();
                self.add_var(var.clone(), PolyType::mono(MonoType::Char));

                // 推断元素类型
                let elem_ty = if let Some(cond) = condition {
                    let _cond_ty = self.infer_expr(cond)?;
                    self.infer_expr(element)?
                } else {
                    self.infer_expr(element)?
                };

                self.exit_scope();

                Ok(MonoType::List(Box::new(elem_ty)))
            }
        }
    }

    /// 推断代码块的类型
    pub fn infer_block(
        &mut self,
        block: &crate::frontend::core::parser::ast::Block,
        _allow_unit: bool,
        _expected_type: Option<&MonoType>,
    ) -> Result<MonoType> {
        // 推断所有语句
        for stmt in &block.stmts {
            self.infer_stmt(stmt)?;
        }

        // 推断最终表达式
        if let Some(expr) = &block.expr {
            self.infer_expr(expr)
        } else {
            Ok(MonoType::Void)
        }
    }

    /// 推断语句的类型
    pub fn infer_stmt(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Result<()> {
        match &stmt.kind {
            crate::frontend::core::parser::ast::StmtKind::Expr(expr) => {
                self.infer_expr(expr)?;
                Ok(())
            }
            crate::frontend::core::parser::ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                ..
            } => {
                // 推断初始化表达式类型
                let init_ty = if let Some(expr) = initializer {
                    self.infer_expr(expr)?
                } else {
                    type_annotation
                        .as_ref()
                        .map_or_else(|| self.solver.new_var(), |t| t.clone().into())
                };

                // 添加变量到作用域
                self.add_var(name.clone(), PolyType::mono(init_ty));
                Ok(())
            }
            crate::frontend::core::parser::ast::StmtKind::Fn {
                name,
                params,
                body: (stmts, expr),
                type_annotation,
                ..
            } => {
                // 创建函数类型
                let param_types: Vec<MonoType> =
                    params.iter().map(|_| self.solver.new_var()).collect();

                let return_type = type_annotation
                    .as_ref()
                    .map_or(MonoType::Void, |t| t.clone().into());

                let fn_type = MonoType::Fn {
                    params: param_types.clone(),
                    return_type: Box::new(return_type.clone()),
                    is_async: false,
                };

                // 添加函数到作用域
                self.add_var(name.clone(), PolyType::mono(fn_type));

                // 进入函数作用域
                self.enter_scope();

                // 添加参数到作用域
                for (param, param_ty) in params.iter().zip(param_types.iter()) {
                    self.add_var(param.name.clone(), PolyType::mono(param_ty.clone()));
                }

                // 推断函数体
                let block = crate::frontend::core::parser::ast::Block {
                    stmts: stmts.clone(),
                    expr: expr.clone(),
                    span: stmt.span,
                };
                let _ = self.infer_block(&block, true, Some(&return_type))?;

                self.exit_scope();
                Ok(())
            }
            _ => {
                // 其他语句类型暂不处理
                Ok(())
            }
        }
    }
}
