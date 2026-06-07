#![allow(clippy::result_large_err)]

//! 表达式类型推断
//!
//! 实现各种表达式的类型推断。
//! 使用统一的 ScopeManager 管理变量作用域。

use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::frontend::core::parser::ast::{BinOp, UnOp};
use crate::frontend::core::types::base::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::typecheck::overload;
use crate::frontend::core::typecheck::traits::solver::TraitSolver;
use std::collections::{HashMap, HashSet};

use super::capture::{self, CaptureInfo};
use super::scope::ScopeManager;

/// 空的 Native 签名表（默认值）
static EMPTY_SIGNATURES: std::sync::LazyLock<HashMap<String, MonoType>> =
    std::sync::LazyLock::new(HashMap::new);

/// 表达式类型推断器
///
/// 使用统一的 ScopeManager 管理变量作用域，
/// 不再维护独立的作用域栈。
pub struct ExpressionInferrer<'a> {
    /// 共享的作用域管理器
    scope: &'a mut ScopeManager,
    /// 约束求解器
    solver: &'a mut TypeConstraintSolver,
    /// 当前活跃的循环标签
    loop_labels: Vec<String>,
    /// 重载候选存储引用
    overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
    /// Native 函数签名引用
    native_signatures: &'a HashMap<String, MonoType>,
    /// 当前函数的 Result 错误类型（若为 None，则不允许使用 `?`）
    result_err: Option<MonoType>,
    /// 当前函数的预期返回类型（用于 return 语句的类型检查）
    expected_return_type: Option<MonoType>,
    /// 方法绑定表: "Type.method" -> MonoType(Fn)
    /// 用于方法调用语法糖解析: p.draw(screen) → Point.draw(p, screen)
    method_bindings: &'a HashMap<String, MonoType>,
    /// 闭包捕获分析结果: Span -> CaptureInfo
    /// 用于 IR 生成阶段确定闭包的捕获模式
    capture_infos: HashMap<crate::util::span::Span, CaptureInfo>,
    /// 类型定义表: type_name -> MonoType(Struct)
    /// 用于 TypeRef → Struct 解析（字段访问等）
    type_defs: &'a HashMap<String, MonoType>,
}

impl<'a> ExpressionInferrer<'a> {
    /// 创建新的表达式推断器
    pub fn new(
        scope: &'a mut ScopeManager,
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
    ) -> Self {
        Self {
            scope,
            solver,
            loop_labels: Vec::new(),
            overload_candidates,
            native_signatures: &EMPTY_SIGNATURES,
            result_err: None,
            expected_return_type: None,
            method_bindings: &EMPTY_SIGNATURES,
            capture_infos: HashMap::new(),
            type_defs: &EMPTY_SIGNATURES,
        }
    }

    /// 创建带 native 函数签名的表达式推断器
    pub fn with_native_signatures(
        scope: &'a mut ScopeManager,
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
        native_signatures: &'a HashMap<String, MonoType>,
    ) -> Self {
        Self {
            scope,
            solver,
            loop_labels: Vec::new(),
            overload_candidates,
            native_signatures,
            result_err: None,
            expected_return_type: None,
            method_bindings: &EMPTY_SIGNATURES,
            capture_infos: HashMap::new(),
            type_defs: &EMPTY_SIGNATURES,
        }
    }

    /// 创建带 native 函数签名 + Result 错误上下文的表达式推断器
    pub fn with_native_signatures_and_result_err(
        scope: &'a mut ScopeManager,
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
        native_signatures: &'a HashMap<String, MonoType>,
        result_err: Option<MonoType>,
    ) -> Self {
        Self {
            scope,
            solver,
            loop_labels: Vec::new(),
            overload_candidates,
            native_signatures,
            result_err,
            expected_return_type: None,
            method_bindings: &EMPTY_SIGNATURES,
            capture_infos: HashMap::new(),
            type_defs: &EMPTY_SIGNATURES,
        }
    }

    /// 创建带完整上下文（native 签名 + Result + 预期返回类型 + 方法绑定）的表达式推断器
    pub fn with_full_context(
        scope: &'a mut ScopeManager,
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
        native_signatures: &'a HashMap<String, MonoType>,
        result_err: Option<MonoType>,
        expected_return_type: Option<MonoType>,
        method_bindings: &'a HashMap<String, MonoType>,
    ) -> Self {
        Self {
            scope,
            solver,
            loop_labels: Vec::new(),
            overload_candidates,
            native_signatures,
            result_err,
            expected_return_type,
            method_bindings,
            capture_infos: HashMap::new(),
            type_defs: &EMPTY_SIGNATURES,
        }
    }

    /// 获取求解器引用（可变）
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        self.solver
    }

    /// 获取闭包捕获分析结果（供 IR 生成使用）
    pub fn capture_infos(&self) -> &HashMap<crate::util::span::Span, CaptureInfo> {
        &self.capture_infos
    }

    /// 设置方法绑定表
    pub fn set_method_bindings(
        &mut self,
        bindings: &'a HashMap<String, MonoType>,
    ) {
        self.method_bindings = bindings;
    }

    /// 设置类型定义表
    pub fn set_type_defs(
        &mut self,
        defs: &'a HashMap<String, MonoType>,
    ) {
        self.type_defs = defs;
    }

    /// 添加变量到当前作用域
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
        is_mut: bool,
    ) {
        self.scope.add_var(name, poly, is_mut);
    }

    /// 检查变量是否存在于任何作用域中
    pub fn var_exists_in_any_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scope.var_in_any_scope(name)
    }

    /// 尝试添加变量到当前作用域
    pub fn try_add_var(
        &mut self,
        name: String,
        poly: PolyType,
        span: crate::util::span::Span,
        is_mut: bool,
    ) -> Result<()> {
        let _ = span;
        self.scope.add_var(name, poly, is_mut);
        Ok(())
    }

    /// 检查变量是否存在于当前作用域
    pub fn var_exists_in_current_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scope.var_in_current_scope(name)
    }

    /// 获取变量（从最内层作用域开始查找）
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.scope.get_var(name)
    }

    /// 获取所有变量（从所有作用域）
    pub fn get_all_vars(&self) -> HashMap<String, PolyType> {
        self.scope.vars()
    }

    /// 变量赋值操作 - 统一处理变量赋值并写回 scope
    ///
    /// 统一变量类型并写回 scope，确保后续类型推断能获取最新类型。
    /// 如果变量不存在，则创建新变量。
    /// 这是修复 for 循环等场景类型丢失的关键方法。
    /// 关键：直接使用右侧表达式的类型（new_ty），而不是依赖 solver.resolve()。
    pub fn assign_var(
        &mut self,
        name: &str,
        new_ty: crate::frontend::core::types::base::MonoType,
    ) {
        // 直接使用右侧表达式的类型更新变量
        // 注意：new_ty 已经是解析后的正确类型（如 List<Int>），不需要额外 resolve
        self.scope.update_var(
            name,
            crate::frontend::core::types::base::PolyType::mono(new_ty),
        );
    }

    /// 退出循环作用域时，将内部声明的变量提升到外层作用域
    ///
    /// 解决循环退出后变量丢失的问题，确保 IR 生成阶段能获取变量类型。
    fn promote_loop_vars_to_parent_scope(&mut self) {
        let current_scope_vars = self.scope.current_scope_vars();

        // 退出当前 scope
        self.scope.exit_scope();

        // 将循环内声明的变量添加到外层 scope，保留可变性
        for (name, info) in current_scope_vars {
            self.scope.add_var(name, info.poly, info.is_mut);
        }
    }

    /// 进入新的作用域
    pub fn enter_scope(&mut self) {
        self.scope.enter_scope();
    }

    /// 退出当前作用域
    pub fn exit_scope(&mut self) {
        self.scope.exit_scope();
    }

    /// 获取当前作用域层级
    pub fn scope_level(&self) -> usize {
        self.scope.scope_level()
    }

    /// 进入循环并注册标签
    pub fn enter_loop(
        &mut self,
        label: Option<&str>,
    ) {
        if let Some(l) = label {
            self.loop_labels.push(l.to_string());
        }
    }

    /// 退出循环并移除标签
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
                    let _ = self.solver.unify(left_elem, right_elem);
                    let elem_ty = self.solver.resolve_type(left_elem);
                    Ok(MonoType::List(Box::new(elem_ty)))
                } else {
                    let var = self.solver.new_var();
                    Ok(var)
                }
            }
            BinOp::Mod => {
                if let (MonoType::Int(_), MonoType::Int(_)) = (left, right) {
                    Ok(left.clone())
                } else if let (MonoType::Float(_), MonoType::Float(_)) = (left, right) {
                    Ok(left.clone())
                } else {
                    let _ = self.solver.unify(left, right);
                    Ok(left.clone())
                }
            }
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
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
                let elem_ty = if left == right {
                    left.clone()
                } else {
                    let _ = self.solver.unify(left, right);
                    left.clone()
                };
                Ok(MonoType::Range {
                    elem_type: Box::new(elem_ty),
                })
            }
            BinOp::Assign => Ok(MonoType::Void),
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
                if let MonoType::TypeRef(inner) = expr {
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
                let poly = self.scope.get_var(name).cloned();
                if let Some(poly) = poly {
                    // 关键：直接使用 scope 中存储的类型！
                    // 因为 assign_var 已经将更新后的类型写入了 scope
                    // 不需要再通过 solver 解析（solver 不知道 scope 的更新）
                    Ok(poly.body)
                } else {
                    Err(ErrorCodeDefinition::unknown_variable(name)
                        .at(*span)
                        .build())
                }
            }

            // 二元运算
            crate::frontend::core::parser::ast::Expr::BinOp {
                op, left, right, ..
            } => {
                let right_ty = self.infer_expr(right)?;

                if matches!(op, BinOp::Assign) {
                    if let crate::frontend::core::parser::ast::Expr::Var(var_name, _) =
                        left.as_ref()
                    {
                        // 统一变量类型并写回 scope，确保后续类型推断正确
                        self.assign_var(var_name, right_ty);
                    }
                    return Ok(MonoType::Void);
                }

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
                    let elem_ty = self.solver.new_var();
                    Ok(MonoType::List(Box::new(elem_ty)))
                } else {
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
                    let key_ty = self.solver.new_var();
                    let value_ty = self.solver.new_var();
                    Ok(MonoType::Dict(Box::new(key_ty), Box::new(value_ty)))
                } else {
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
                            Ok(self.solver.new_var())
                        }
                    }
                    _ => Ok(self.solver.new_var()),
                }
            }

            // 字段访问
            crate::frontend::core::parser::ast::Expr::FieldAccess {
                expr: obj, field, ..
            } => {
                fn extract_namespace_path(
                    expr: &crate::frontend::core::parser::ast::Expr
                ) -> Option<String> {
                    match expr {
                        crate::frontend::core::parser::ast::Expr::Var(name, _) => {
                            Some(name.clone())
                        }
                        crate::frontend::core::parser::ast::Expr::FieldAccess {
                            expr,
                            field,
                            ..
                        } => extract_namespace_path(expr).map(|p| format!("{}.{}", p, field)),
                        _ => None,
                    }
                }

                let obj_ty = self.infer_expr(obj)?;
                let obj_ty = self.solver.resolve_type(&obj_ty);

                // 解包所有 Ref 层，用于字段/方法查找
                // 例如 &Point -> Point, &&Point -> Point
                let mut resolved = obj_ty.clone();
                while let MonoType::Ref { inner, .. } = resolved {
                    resolved = *inner;
                }
                let resolved = self.solver.resolve_type(&resolved);

                let namespace_path = extract_namespace_path(obj);
                if let Some(ns_path) = namespace_path {
                    let full_path = format!("{}.{}", ns_path, field);
                    if let Some(sig) = self.native_signatures.get(&full_path).cloned() {
                        return Ok(sig);
                    }
                    if self
                        .native_signatures
                        .keys()
                        .any(|k| k.starts_with(&full_path))
                    {
                        let fn_ty = MonoType::Fn {
                            params: vec![self.solver.new_var()],
                            return_type: Box::new(MonoType::Void),
                        };
                        return Ok(fn_ty);
                    }
                }

                match resolved {
                    MonoType::Struct(struct_type) => {
                        for (field_name, field_ty) in &struct_type.fields {
                            if field_name == field {
                                return Ok(field_ty.clone());
                            }
                        }
                        // Field not found in struct — try method lookup
                        let method_key = format!("{}.{}", struct_type.name, field);
                        if let Some(method_ty) = self.method_bindings.get(&method_key) {
                            return Ok(method_ty.clone());
                        }
                        Err(ErrorCodeDefinition::field_not_found(field, &struct_type.name).build())
                    }
                    MonoType::TypeRef(ref type_name) => {
                        // Try to resolve TypeRef → Struct via type_defs for field lookup
                        if let Some(def_ty) = self.type_defs.get(type_name) {
                            let def_ty = self.solver.resolve_type(def_ty);
                            if let MonoType::Struct(ref struct_type) = def_ty {
                                for (field_name, field_ty) in &struct_type.fields {
                                    if field_name == field {
                                        return Ok(field_ty.clone());
                                    }
                                }
                                // Field not found in resolved struct — try method lookup
                                let method_key = format!("{}.{}", struct_type.name, field);
                                if let Some(method_ty) = self.method_bindings.get(&method_key) {
                                    return Ok(method_ty.clone());
                                }
                                return Err(ErrorCodeDefinition::field_not_found(
                                    field,
                                    &struct_type.name,
                                )
                                .build());
                            }
                        }
                        // Try method lookup on TypeRef (generic type or forward reference)
                        let method_key = format!("{}.{}", type_name, field);
                        if let Some(method_ty) = self.method_bindings.get(&method_key) {
                            return Ok(method_ty.clone());
                        }
                        Err(
                            ErrorCodeDefinition::field_access_on_non_struct(&format!("{}", obj_ty))
                                .build(),
                        )
                    }
                    _ => Err(ErrorCodeDefinition::field_access_on_non_struct(&format!(
                        "{}",
                        obj_ty
                    ))
                    .build()),
                }
            }

            // 函数调用
            crate::frontend::core::parser::ast::Expr::Call {
                func, args, span, ..
            } => {
                // 检查 native("...") 表达式 — 通过 name resolution
                if let Some(native_name) =
                    crate::std::ffi::extract_native_binding_symbol(func, args)
                {
                    if let Some(sig) = self.native_signatures.get(&native_name).cloned() {
                        return Ok(sig);
                    }
                    return Ok(self.solver.new_var());
                }

                let func_ty = self.infer_expr(func)?;

                let arg_types: Vec<MonoType> = args
                    .iter()
                    .map(|arg| self.infer_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;

                // 重载解析
                if let crate::frontend::core::parser::ast::Expr::Var(ref name, _) = **func {
                    if overload::has_overloads(self.overload_candidates, name) {
                        match overload::resolve_overload_from_env(
                            self.overload_candidates,
                            name,
                            &arg_types,
                        ) {
                            Ok(candidate) => {
                                return Ok(candidate.return_type.clone());
                            }
                            Err(_e) => {
                                if let Some(generic_candidate) = overload::resolve_generic_fallback(
                                    self.overload_candidates,
                                    name,
                                    &arg_types,
                                ) {
                                    let return_type = overload::instantiate_return_type(
                                        generic_candidate,
                                        &arg_types,
                                    );
                                    return Ok(return_type);
                                }
                                return Ok(self.solver.new_var());
                            }
                        }
                    }
                }

                if let MonoType::Fn {
                    params,
                    return_type,
                    ..
                } = func_ty
                {
                    // 仅当参数数量匹配时检查类型（方法调用语法糖中 self 是隐式参数，
                    // arg_types 不包含 self，所以长度不匹配时跳过检查）
                    if arg_types.len() == params.len() {
                        for (arg_ty, param_ty) in arg_types.iter().zip(params.iter()) {
                            // Int -> Float 扩展转换是允许的
                            if matches!((arg_ty, param_ty), (MonoType::Int(_), MonoType::Float(_)))
                            {
                                continue;
                            }
                            // TypeRef 未完全解析时跳过（如用户自定义类型名）
                            if matches!(param_ty, MonoType::TypeRef(_)) {
                                continue;
                            }
                            // TypeVar 是泛型类型参数 —— 必须 unify 以推断具体类型
                            if self.solver.unify(arg_ty, param_ty).is_err() {
                                return Err(ErrorCodeDefinition::type_mismatch(
                                    &format!("{}", param_ty),
                                    &format!("{}", arg_ty),
                                )
                                .at(*span)
                                .build());
                            }
                        }
                    }
                    // 展开返回类型中被绑定的类型变量（Type 自描述推断）
                    let resolved_ret = self.solver.expand_type_shallow(&return_type);
                    return Ok(resolved_ret);
                }
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
                let cond_ty = self.infer_expr(condition)?;
                if cond_ty != MonoType::Bool {
                    return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                        "{}",
                        cond_ty
                    ))
                    .build());
                }

                self.scope.enter_scope();
                let then_result = self.infer_block(then_branch, true, None);
                self.scope.exit_scope();
                let _then_ty = then_result?;

                for (elif_cond, elif_block) in elif_branches {
                    let elif_cond_ty = self.infer_expr(elif_cond)?;
                    if elif_cond_ty != MonoType::Bool {
                        return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                            "{}",
                            elif_cond_ty
                        ))
                        .build());
                    }
                    self.scope.enter_scope();
                    let elif_result = self.infer_block(elif_block, true, None);
                    self.scope.exit_scope();
                    let _ = elif_result?;
                }

                if let Some(else_block) = else_branch {
                    self.scope.enter_scope();
                    let else_result = self.infer_block(else_block, true, None);
                    self.scope.exit_scope();
                    else_result
                } else {
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
                let cond_ty = self.infer_expr(condition)?;
                if cond_ty != MonoType::Bool {
                    return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                        "{}",
                        cond_ty
                    ))
                    .build());
                }

                self.enter_loop(label.as_deref());

                self.scope.enter_scope();
                let result = self.infer_block(body, true, None);
                // 退出循环作用域时，将内部变量提升到外层，避免变量丢失
                self.promote_loop_vars_to_parent_scope();

                self.exit_loop(label.as_deref());

                result?;
                Ok(MonoType::Void)
            }

            // For 循环
            crate::frontend::core::parser::ast::Expr::For {
                var,
                var_mut,
                iterable,
                body,
                label,
                span,
            } => {
                let iter_ty = self.infer_expr(iterable)?;

                let element_type = match &iter_ty {
                    MonoType::List(elem_ty) => *elem_ty.clone(),
                    MonoType::Range { elem_type } => *elem_type.clone(),
                    MonoType::String => MonoType::Char,
                    MonoType::Tuple(_elems) => self.solver.new_var(),
                    MonoType::Dict(key_ty, value_ty) => {
                        MonoType::Tuple(vec![*key_ty.clone(), *value_ty.clone()])
                    }
                    _ => self.solver.new_var(),
                };

                self.enter_loop(label.as_deref());

                self.scope.enter_scope();
                let result = self
                    .try_add_var(var.clone(), PolyType::mono(element_type), *span, *var_mut)
                    .and_then(|_| self.infer_block(body, true, None));

                // 退出循环作用域时，将内部变量提升到外层，避免变量丢失
                self.promote_loop_vars_to_parent_scope();

                self.exit_loop(label.as_deref());
                result
            }

            // Return 表达式
            crate::frontend::core::parser::ast::Expr::Return(expr, span) => {
                if let Some(e) = expr {
                    let ret_ty = self.infer_expr(e)?;
                    // If we know the expected return type, check that the return
                    // expression type matches it via unification.
                    if let Some(ref expected) = self.expected_return_type {
                        self.solver.unify(&ret_ty, expected).map_err(|_| {
                            ErrorCodeDefinition::type_mismatch(
                                &format!("{}", expected),
                                &format!("{}", ret_ty),
                            )
                            .at(*span)
                            .build()
                        })?;
                    }
                    Ok(ret_ty)
                } else {
                    Ok(MonoType::Void)
                }
            }

            // Break 表达式
            crate::frontend::core::parser::ast::Expr::Break(label, _) => {
                if let Some(l) = label {
                    if !self.has_label(l) {
                        return Err(ErrorCodeDefinition::unknown_label(l).build());
                    }
                }
                Ok(MonoType::Void)
            }

            // Continue 表达式
            crate::frontend::core::parser::ast::Expr::Continue(label, _) => {
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
                self.scope.enter_scope();
                let result: Result<()> = (|| {
                    for param in params {
                        let param_ty = self.solver.new_var();
                        self.scope.add_var(
                            param.name.clone(),
                            PolyType::mono(param_ty),
                            param.is_mut,
                        );
                    }

                    let ret_mono: MonoType =
                        return_type.clone().map_or(MonoType::Void, |t| t.into());
                    // RFC-001: Result-returning functions implicitly wrap the final value in Ok(...),
                    // so the body type is the Ok type (not Result[T, E]).
                    let expected_body_ty = match &ret_mono {
                        MonoType::Result(ok, _) => (**ok).clone(),
                        _ => ret_mono.clone(),
                    };

                    // Enter a new `Result` context for this function body.
                    let saved_result_err = self.result_err.take();
                    self.result_err = match &ret_mono {
                        MonoType::Result(_, err) => Some((**err).clone()),
                        _ => None,
                    };

                    // Save and set expected return type for return statement checking
                    let saved_expected_ret = self.expected_return_type.take();
                    self.expected_return_type = Some(expected_body_ty.clone());

                    let body_ty_res = self.infer_block(body, true, Some(&expected_body_ty));

                    // Restore outer contexts
                    self.expected_return_type = saved_expected_ret;
                    self.result_err = saved_result_err;

                    let body_ty = body_ty_res?;

                    if return_type.is_some() {
                        let _ = self.solver.unify(&body_ty, &expected_body_ty);
                    }

                    Ok(())
                })();
                self.scope.exit_scope();
                result?;

                let param_types: Vec<MonoType> =
                    params.iter().map(|_| self.solver.new_var()).collect();
                let return_type_box =
                    Box::new(return_type.clone().map_or(MonoType::Void, |t| t.into()));

                let fn_type = MonoType::Fn {
                    params: param_types,
                    return_type: return_type_box,
                };
                self.scope
                    .add_var(name.clone(), PolyType::mono(fn_type.clone()), false);

                Ok(fn_type)
            }

            // Lambda 表达式
            crate::frontend::core::parser::ast::Expr::Lambda {
                params, body, span, ..
            } => {
                // 在进入 Lambda 作用域之前，收集外部作用域变量信息（用于捕获分析）
                let outer_scope_vars = self.scope.vars();
                let outer_scope_names: HashSet<String> = outer_scope_vars.keys().cloned().collect();

                self.scope.enter_scope();
                for param in params {
                    let param_ty = self.solver.new_var();
                    self.scope
                        .add_var(param.name.clone(), PolyType::mono(param_ty), param.is_mut);
                }

                // Lambda is a function boundary: it must not inherit outer `Result` context.
                let saved_result_err = self.result_err.take();
                self.result_err = None;
                // Lambda is also a return type boundary
                let saved_expected_ret = self.expected_return_type.take();
                self.expected_return_type = None;
                let body_ty = self.infer_block(body, true, None);
                self.expected_return_type = saved_expected_ret;
                self.result_err = saved_result_err;

                self.scope.exit_scope();
                let body_ty = body_ty?;

                // 运行闭包捕获分析
                let lambda_expr = expr;
                let trait_solver = TraitSolver::new();
                let capture_info = capture::analyze_lambda_captures(
                    lambda_expr,
                    body,
                    &outer_scope_names,
                    &outer_scope_vars,
                    &trait_solver,
                    None, // parent 暂不可用，后续可增强
                );
                self.capture_infos.insert(*span, capture_info);

                let param_types: Vec<MonoType> =
                    params.iter().map(|_| self.solver.new_var()).collect();

                Ok(MonoType::Fn {
                    params: param_types,
                    return_type: Box::new(body_ty),
                })
            }

            // Match 表达式
            crate::frontend::core::parser::ast::Expr::Match { expr, .. } => {
                let _expr_ty = self.infer_expr(expr)?;
                Ok(self.solver.new_var())
            }

            // Try 表达式: expr?
            crate::frontend::core::parser::ast::Expr::Try { expr, span } => {
                let Some(expected_err) = self.result_err.clone() else {
                    return Err(ErrorCodeDefinition::try_only_allowed_in_result()
                        .at(*span)
                        .build());
                };

                let inner_ty = self.infer_expr(expr)?;
                let ok_ty = self.solver.new_var();
                let expected_result =
                    MonoType::Result(Box::new(ok_ty.clone()), Box::new(expected_err.clone()));

                if let Err(_e) = self.solver.unify(&inner_ty, &expected_result) {
                    let resolved = self.solver.resolve_type(&inner_ty);
                    if let MonoType::Result(_, err) = resolved {
                        return Err(ErrorCodeDefinition::try_error_type_mismatch(
                            &expected_err.to_string(),
                            &err.to_string(),
                        )
                        .at(*span)
                        .build());
                    }
                    return Err(
                        ErrorCodeDefinition::try_requires_result(&resolved.to_string())
                            .at(*span)
                            .build(),
                    );
                }

                Ok(ok_ty)
            }

            // Ref 表达式
            crate::frontend::core::parser::ast::Expr::Ref { expr, .. } => {
                let expr_ty = self.infer_expr(expr)?;
                Ok(MonoType::Arc(Box::new(expr_ty)))
            }

            // Unsafe 块
            crate::frontend::core::parser::ast::Expr::Unsafe { body, .. } => {
                if let Some(last_expr) = &body.expr {
                    self.infer_expr(last_expr)
                } else {
                    Ok(MonoType::Void)
                }
            }

            // spawn 块：spawn { ... }
            crate::frontend::core::parser::ast::Expr::Spawn { body, .. } => {
                self.infer_block(body, true, None)
            }

            // ListComp 表达式
            crate::frontend::core::parser::ast::Expr::ListComp {
                element,
                var,
                iterable,
                condition,
                ..
            } => {
                let _iter_ty = self.infer_expr(iterable)?;

                self.scope.enter_scope();
                self.scope
                    .add_var(var.clone(), PolyType::mono(MonoType::Char), false);

                let elem_ty = if let Some(cond) = condition {
                    let _cond_ty = self.infer_expr(cond)?;
                    self.infer_expr(element)?
                } else {
                    self.infer_expr(element)?
                };

                self.scope.exit_scope();

                Ok(MonoType::List(Box::new(elem_ty)))
            }

            // RFC-012: F-string 类型推断
            // f-string 总是返回 String 类型
            crate::frontend::core::parser::ast::Expr::FString { segments, .. } => {
                // 验证每个插值表达式的类型
                for segment in segments {
                    if let crate::frontend::core::parser::ast::FStringSegment::Interpolation {
                        expr,
                        ..
                    } = segment
                    {
                        let _expr_ty = self.infer_expr(expr)?;
                        // 所有类型都支持转换为 String（通过 format()）
                    }
                }
                Ok(MonoType::String)
            }

            // 错误恢复占位符：返回新类型变量，不会导致 panic
            crate::frontend::core::parser::ast::Expr::Error(span) => {
                Err(ErrorCodeDefinition::invalid_syntax("缺失表达式")
                    .at(*span)
                    .build())
            }

            // 借用表达式：&expr 或 &mut expr
            // TODO: 详细类型检查将在后续任务中实现
            crate::frontend::core::parser::ast::Expr::Borrow {
                mutable,
                expr: inner,
                ..
            } => {
                let inner_ty = self.infer_expr(inner)?;
                Ok(MonoType::Ref {
                    mutable: *mutable,
                    inner: Box::new(inner_ty),
                })
            }
        }
    }

    /// 推断代码块的类型
    ///
    /// 语义（RFC-010）：
    /// - `{}` 块的值 = 块内 `return expr` 的 `expr` 的值
    /// - 没有 `return` = Void
    /// - 尾随表达式不影响块的返回类型
    pub fn infer_block(
        &mut self,
        block: &crate::frontend::core::parser::ast::Block,
        _allow_unit: bool,
        _expected_type: Option<&MonoType>,
    ) -> Result<MonoType> {
        let mut return_type: Option<MonoType> = None;

        for stmt in &block.stmts {
            // 检查语句是否包含 return 表达式
            if let crate::frontend::core::parser::ast::StmtKind::Expr(ref expr_stmt) = stmt.kind {
                if let Some(ty) = self.collect_return_type(expr_stmt)? {
                    return_type = Some(ty);
                }
            }
            self.infer_stmt(stmt)?;
        }

        // 块的类型 = return 的类型，没有 return 则 Void
        Ok(return_type.unwrap_or(MonoType::Void))
    }

    /// 递归收集表达式中的 return 类型
    /// 如果表达式是 `return expr`，返回 expr 的类型
    /// 如果表达式包含子块（if/for/spawn 等），递归收集子块中的 return 类型
    fn collect_return_type(
        &mut self,
        expr: &crate::frontend::core::parser::ast::Expr,
    ) -> Result<Option<MonoType>> {
        match expr {
            crate::frontend::core::parser::ast::Expr::Return(Some(ret_expr), _) => {
                let ty = self.infer_expr(ret_expr)?;
                Ok(Some(ty))
            }
            crate::frontend::core::parser::ast::Expr::Return(None, _) => Ok(Some(MonoType::Void)),
            _ => Ok(None),
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
                is_mut,
                ..
            } => {
                let init_ty = if let Some(expr) = initializer {
                    self.infer_expr(expr)?
                } else {
                    type_annotation
                        .as_ref()
                        .map_or_else(|| self.solver.new_var(), |t| t.clone().into())
                };

                if self.scope.var_in_any_scope(name) {
                    if self.scope.var_in_current_scope(name) {
                        // 当前作用域已有此变量
                        if self.scope.var_is_moved(name).unwrap_or(false) {
                            // 已 moved 的变量：视为"未找到"，重新声明
                            // 先移除旧的 moved 绑定，再添加新绑定
                            self.scope.remove_var(name);
                            // 继续到下面的 try_add_var
                        } else {
                            return Err(ErrorCodeDefinition::duplicate_definition(name)
                                .at(stmt.span)
                                .build());
                        }
                    } else {
                        // 外层作用域有此变量
                        if self.scope.var_is_moved(name).unwrap_or(false) {
                            // 外层变量已 moved：在当前作用域重新声明
                            // 继续到下面的 try_add_var
                        } else if !*is_mut {
                            // 非 mut 的 Var 在外部作用域存在同名变量时，是赋值操作
                            // 需要检查外层变量是否可变
                            if !self.scope.var_is_mutable(name).unwrap_or(false) {
                                return Err(ErrorCodeDefinition::immutable_assignment(name)
                                    .at(stmt.span)
                                    .build());
                            }
                            return Ok(());
                        }
                        // mut 声明允许遮蔽外层变量（与 StatementChecker.check_var_stmt 行为一致）
                    }
                }
                self.try_add_var(name.clone(), PolyType::mono(init_ty), stmt.span, *is_mut)?;
                Ok(())
            }
            crate::frontend::core::parser::ast::StmtKind::Binding {
                name,
                params,
                body: (stmts, expr),
                type_annotation,
                ..
            } => {
                let param_types: Vec<MonoType> =
                    params.iter().map(|_| self.solver.new_var()).collect();

                let return_type = type_annotation
                    .as_ref()
                    .map_or(MonoType::Void, |t| t.clone().into());

                let fn_type = MonoType::Fn {
                    params: param_types.clone(),
                    return_type: Box::new(return_type.clone()),
                };

                self.scope
                    .add_var(name.clone(), PolyType::mono(fn_type), false);

                self.scope.enter_scope();
                let result: Result<()> = (|| {
                    for (param, param_ty) in params.iter().zip(param_types.iter()) {
                        self.scope.add_var(
                            param.name.clone(),
                            PolyType::mono(param_ty.clone()),
                            param.is_mut,
                        );
                    }

                    let block = crate::frontend::core::parser::ast::Block {
                        stmts: stmts.clone(),
                        expr: expr.clone(),
                        span: stmt.span,
                    };
                    let _ = self.infer_block(&block, true, Some(&return_type))?;

                    Ok(())
                })();
                self.scope.exit_scope();
                result
            }
            _ => Ok(()),
        }
    }
}

/// 向后兼容：ExprInferrer 是 ExpressionInferrer 的类型别名
pub type ExprInferrer<'a> = ExpressionInferrer<'a>;
