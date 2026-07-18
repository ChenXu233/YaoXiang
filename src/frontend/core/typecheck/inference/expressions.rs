#![allow(clippy::result_large_err)]

//! 表达式类型推断
//!
//! 实现各种表达式的类型推断。
//! 使用统一的 ScopeManager 管理变量作用域。

use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::frontend::core::parser::ast::{BinOp, UnOp};
use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::typecheck::passes::overload;
use crate::middle::passes::mono::instance::{GenericFunctionId, InstantiationRequest};
use std::collections::{HashMap, HashSet};

use super::scope::ScopeManager;

/// 空的 Native 签名表（默认值）
static EMPTY_SIGNATURES: std::sync::LazyLock<HashMap<String, MonoType>> =
    std::sync::LazyLock::new(HashMap::new);

static EMPTY_GENERIC_TYPE_DEFS: std::sync::LazyLock<
    HashMap<String, crate::frontend::core::typecheck::environment::GenericTypeDef>,
> = std::sync::LazyLock::new(HashMap::new);

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
    /// 类型定义表: type_name -> MonoType(Struct)
    /// 用于 TypeRef → Struct 解析（字段访问等）
    type_defs: &'a HashMap<String, MonoType>,
    /// 泛型类型定义表
    /// 用于 List(1, 2, 3) 等泛型类型构造调用的实例化
    generic_type_defs:
        &'a HashMap<String, crate::frontend::core::typecheck::environment::GenericTypeDef>,
    /// 实例化请求（收集遇到的所有泛型函数实例化需求）
    pub instantiation_requests: Vec<InstantiationRequest>,
    /// 依赖类型环境（效应查询）—— 由 StatementChecker 注入
    dep_env: Option<&'a crate::frontend::core::types::eval::dependent_types::DependentTypeEnv>,
    /// 流敏感假设集 Γ（效应注入）—— 由 StatementChecker 注入
    gamma: Option<&'a mut crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma>,
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
            type_defs: &EMPTY_SIGNATURES,
            generic_type_defs: &EMPTY_GENERIC_TYPE_DEFS,
            instantiation_requests: Vec::new(),
            dep_env: None,
            gamma: None,
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
            type_defs: &EMPTY_SIGNATURES,
            generic_type_defs: &EMPTY_GENERIC_TYPE_DEFS,
            instantiation_requests: Vec::new(),
            dep_env: None,
            gamma: None,
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
            type_defs: &EMPTY_SIGNATURES,
            generic_type_defs: &EMPTY_GENERIC_TYPE_DEFS,
            instantiation_requests: Vec::new(),
            dep_env: None,
            gamma: None,
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
            type_defs: &EMPTY_SIGNATURES,
            generic_type_defs: &EMPTY_GENERIC_TYPE_DEFS,
            instantiation_requests: Vec::new(),
            dep_env: None,
            gamma: None,
        }
    }

    /// 获取求解器引用（可变）
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        self.solver
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

    /// 设置泛型类型定义表
    pub fn set_generic_type_defs(
        &mut self,
        defs: &'a HashMap<String, crate::frontend::core::typecheck::environment::GenericTypeDef>,
    ) {
        self.generic_type_defs = defs;
    }

    /// 设置依赖类型环境（效应查询）
    pub fn set_dep_env(
        &mut self,
        dep_env: &'a crate::frontend::core::types::eval::dependent_types::DependentTypeEnv,
    ) {
        self.dep_env = Some(dep_env);
    }

    /// 设置流敏感假设集 Γ（效应注入）
    pub fn set_gamma(
        &mut self,
        gamma: &'a mut crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma,
    ) {
        self.gamma = Some(gamma);
    }

    /// 添加变量到当前作用域
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
        is_mut: bool,
    ) {
        self.scope
            .add_var(name, poly, is_mut, crate::util::span::Span::default());
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
        self.scope
            .add_var(name, poly, is_mut, crate::util::span::Span::default());
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
        new_ty: crate::frontend::core::types::MonoType,
    ) {
        // 直接使用右侧表达式的类型更新变量
        // 注意：new_ty 已经是解析后的正确类型（如 List<Int>），不需要额外 resolve
        self.scope
            .update_var(name, crate::frontend::core::types::PolyType::mono(new_ty));
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
            self.scope.add_var(
                name,
                info.poly,
                info.is_mut,
                crate::util::span::Span::default(),
            );
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

    /// 递归收集类型中的所有 TypeVar 索引
    fn collect_type_var_indices(
        ty: &MonoType,
        out: &mut HashSet<usize>,
    ) {
        match ty {
            MonoType::TypeVar(tv) => {
                out.insert(tv.index());
            }
            MonoType::List(inner) => Self::collect_type_var_indices(inner, out),
            MonoType::Tuple(types) => {
                for t in types {
                    Self::collect_type_var_indices(t, out);
                }
            }
            MonoType::Dict(k, v) => {
                Self::collect_type_var_indices(k, out);
                Self::collect_type_var_indices(v, out);
            }
            MonoType::Set(t) => Self::collect_type_var_indices(t, out),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                for p in params {
                    Self::collect_type_var_indices(p, out);
                }
                Self::collect_type_var_indices(return_type, out);
            }
            MonoType::Option(inner) => Self::collect_type_var_indices(inner, out),
            MonoType::Result(ok, err) => {
                Self::collect_type_var_indices(ok, out);
                Self::collect_type_var_indices(err, out);
            }
            MonoType::Range { elem_type } => Self::collect_type_var_indices(elem_type, out),
            MonoType::Union(types) | MonoType::Intersection(types) => {
                for t in types {
                    Self::collect_type_var_indices(t, out);
                }
            }
            MonoType::Arc(t) | MonoType::Weak(t) => Self::collect_type_var_indices(t, out),
            MonoType::Struct(s) => {
                for (_, field_ty) in &s.fields {
                    Self::collect_type_var_indices(field_ty, out);
                }
            }
            MonoType::Generic { args, .. } => {
                for a in args {
                    Self::collect_type_var_indices(a, out);
                }
            }
            MonoType::AssocType {
                host_type,
                assoc_args,
                ..
            } => {
                Self::collect_type_var_indices(host_type, out);
                for a in assoc_args {
                    Self::collect_type_var_indices(a, out);
                }
            }
            _ => {}
        }
    }

    /// 将类型中的 TypeVar 根据替换映射替换为具体类型
    ///
    /// 递归遍历类型，将遇到的 TypeVar 在 `subst` 映射中查找，
    /// 若找到替换项则替换，否则保留原 TypeVar。
    fn substitute_type_vars(
        ty: &MonoType,
        subst: &HashMap<usize, MonoType>,
    ) -> MonoType {
        use crate::frontend::core::types::substitute::{Substituter, Substitution};
        let mut sub = Substitution::new();
        for (idx, replacement) in subst {
            sub.insert(*idx, replacement.clone());
        }
        Substituter::new().substitute(ty, &sub)
    }

    /// 单态化泛型函数类型：将泛型函数类型中的类型变量统一替换为具体类型。
    ///
    /// 当调用泛型函数（如 `fn identity[T](x: T) -> T`）时，根据实参类型
    /// 推断类型变量的具体类型，返回单态化后的函数类型。
    ///
    /// 仅处理 Fn 类型；非 Fn 类型或不含 MetaType 的 Fn 类型原样返回。
    fn monomorphize(
        &mut self,
        func_ty: MonoType,
        arg_types: &[MonoType],
    ) -> MonoType {
        let MonoType::Fn {
            params,
            return_type,
        } = &func_ty
        else {
            return func_ty;
        };

        // 收集原始类型变量索引
        let mut var_indices = HashSet::new();
        Self::collect_type_var_indices(&func_ty, &mut var_indices);

        if !var_indices.is_empty() {
            // 泛型值级函数：创建新的 TypeVar 实例（每次调用独立）
            let mut subst = HashMap::new();
            for idx in var_indices {
                let fresh = self.solver.new_var();
                subst.insert(idx, fresh);
            }

            let new_params: Vec<MonoType> = params
                .iter()
                .map(|p| Self::substitute_type_vars(p, &subst))
                .collect();
            let new_return = Self::substitute_type_vars(return_type, &subst);

            // Unify 新参数与实参以推断具体类型
            if arg_types.len() == new_params.len() {
                for (arg_ty, param_ty) in arg_types.iter().zip(new_params.iter()) {
                    let _ = self.solver.unify(arg_ty, param_ty);
                }
            }

            let resolved_return = self.solver.resolve_type(&new_return);
            return MonoType::Fn {
                params: new_params,
                return_type: Box::new(resolved_return),
            };
        }

        // 检查参数中是否包含 MetaType（泛型类型构造器）
        let has_meta = params
            .iter()
            .any(|p| matches!(p, MonoType::MetaType { .. }));
        if !has_meta {
            return func_ty;
        }

        // 当没有 TypeVar 但有 MetaType 参数时，为 MetaType 创建新的 TypeVar
        // 用于处理 List(1, 2, 3) 这样的情况
        {
            // 为每个 MetaType 参数创建新的 TypeVar
            let mut subst = HashMap::new();
            for (i, param) in params.iter().enumerate() {
                if matches!(param, MonoType::MetaType { .. }) && i < arg_types.len() {
                    let fresh = self.solver.new_var();
                    let fresh_clone = fresh.clone();
                    subst.insert(i, fresh);
                    // 将新 TypeVar 与实参类型统一，以推断具体类型
                    let _ = self.solver.unify(&fresh_clone, &arg_types[i]);
                }
            }
            // 替换参数中的 MetaType 为推断出的具体类型
            let new_params: Vec<MonoType> = params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    if let Some(fresh) = subst.get(&i) {
                        self.solver.resolve_type(fresh)
                    } else {
                        p.clone()
                    }
                })
                .collect();
            let resolved_return = self.solver.resolve_type(return_type);

            // 如果返回类型是 MetaType，尝试从 generic_type_defs 中获取具体的类型
            if matches!(resolved_return, MonoType::MetaType { .. }) {
                // 尝试从函数名中获取泛型类型定义
                // 这里我们需要从 func_ty 中获取函数名，但 func_ty 是 MonoType::Fn，
                // 没有函数名信息。因此我们需要在调用时传入函数名。
                // 暂时返回一个 TypeVar，让调用者处理
                return MonoType::Fn {
                    params: new_params,
                    return_type: Box::new(self.solver.new_var()),
                };
            }

            MonoType::Fn {
                params: new_params,
                return_type: Box::new(resolved_return),
            }
        }
    }

    /// 收集泛型函数实例化请求
    ///
    /// 检测函数调用是否是泛型函数调用，如果是则构造 InstantiationRequest
    /// 并添加到实例化请求列表中。
    fn collect_instantiation_request(
        &mut self,
        func_ty: &MonoType,
        func_expr: &crate::frontend::core::parser::ast::Expr,
        arg_types: &[MonoType],
        mono_func_ty: &MonoType,
        call_span: crate::util::span::Span,
    ) {
        // 只处理 Fn 类型
        let MonoType::Fn {
            params: original_params,
            ..
        } = func_ty
        else {
            return;
        };

        // 检查原函数类型是否包含 TypeVar（即是否为泛型函数）
        let mut var_indices = HashSet::new();
        Self::collect_type_var_indices(func_ty, &mut var_indices);
        if var_indices.is_empty() {
            // 也检查 MetaType 参数（泛型类型构造器）
            let has_meta = original_params
                .iter()
                .any(|p| matches!(p, MonoType::MetaType { .. }));
            if !has_meta {
                return;
            }
        }

        // 获取函数名称（从 AST）
        let fn_name = match func_expr {
            crate::frontend::core::parser::ast::Expr::Var(ref name, _) => name.clone(),
            _ => return, // 对于非命名函数调用（如 lambda 调用），暂不收集
        };

        // 获取泛型参数名称列表
        let type_params: Vec<String> = self.lookup_type_params(&fn_name);

        // 从单态化后的函数类型中提取具体的类型参数
        if let MonoType::Fn {
            params: resolved_params,
            ..
        } = mono_func_ty
        {
            // 收集所有的具体类型（从已解析的参数中提取去重后的类型）
            let type_args: Vec<MonoType> =
                self.extract_concrete_type_args(resolved_params, arg_types);

            if !type_args.is_empty() {
                let generic_id = if type_params.is_empty() {
                    GenericFunctionId::new(fn_name, vec![])
                } else {
                    GenericFunctionId::new(fn_name, type_params)
                };
                let request = InstantiationRequest::new(generic_id, type_args, call_span);
                self.instantiation_requests.push(request);
            }
        }
    }

    /// 查找函数的泛型类型参数名称
    fn lookup_type_params(
        &self,
        fn_name: &str,
    ) -> Vec<String> {
        // 1. 优先从重载候选中查找（OverloadCandidate 包含 type_params）
        if let Some(candidates) = self.overload_candidates.get(fn_name) {
            for candidate in candidates {
                if candidate.is_generic {
                    return candidate.type_params.clone();
                }
            }
        }

        // 2. 从作用域中查找 PolyType
        if let Some(poly) = self.scope.get_var(fn_name) {
            // type_binders 是 TypeVar 列表，按索引顺序对应类型参数
            // 由于当前系统不存储类型参数名称，返回空列表
            // Monomorphizer 可以通过函数名匹配（name 唯一时）
            if !poly.type_binders.is_empty() {
                // 如果有 type_binders，说明是泛型函数
                // 生成占位名称（如 "T0", "T1"）以便单态化器识别
                return poly
                    .type_binders
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("T{}", i))
                    .collect();
            }
        }

        vec![]
    }

    /// 从已解析的参数类型和实参类型中提取具体的类型参数
    fn extract_concrete_type_args(
        &self,
        resolved_params: &[MonoType],
        _arg_types: &[MonoType],
    ) -> Vec<MonoType> {
        let mut type_args = Vec::new();
        let mut seen = HashSet::new();

        // 收集 params 中的具体类型（已解析的 TypeVar）
        for param in resolved_params {
            let resolved = self.solver.resolve_type(param);
            if !matches!(resolved, MonoType::TypeVar(_)) {
                let key = format!("{}", resolved);
                if seen.insert(key) {
                    type_args.push(resolved);
                }
            }
        }

        type_args
    }

    /// 推断表达式的类型
    #[allow(irrefutable_let_patterns)]
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
                } else if is_builtin_type_name(name) {
                    // 内置类型名在表达式位置 — 当作 Type 宇宙的值
                    Ok(crate::frontend::core::types::MonoType::MetaType {
                        universe_level: crate::frontend::core::types::mono::UniverseLevel::type0(),
                        type_params: Vec::new(),
                    })
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
                let func_ty = self.infer_expr(func)?;

                // LibraryRef callable rule: when calling a LibraryRef with a string literal
                // e.g. sqlite3("sqlite3_open") where sqlite3: LibraryRef
                // Returns ExternRef at compile time
                let func_ty_resolved = self.solver.resolve_type(&func_ty);
                if let MonoType::LibraryRef { mechanism, .. } = &func_ty_resolved {
                    if args.len() == 1 {
                        if let Some(sym) = extract_string_literal_from_expr(&args[0]) {
                            return Ok(MonoType::ExternRef {
                                mechanism: mechanism.clone(),
                                lib: String::new(), // filled at IR gen
                                symbol: sym,
                            });
                        }
                        return Err(ErrorCodeDefinition::type_mismatch(
                            "String",
                            &format!(
                                "{}",
                                self.infer_expr(&args[0])
                                    .unwrap_or_else(|_| self.solver.new_var())
                            ),
                        )
                        .at(*span)
                        .build());
                    }
                    return Err(ErrorCodeDefinition::argument_count_mismatch(
                        "LibraryRef callable",
                        1,
                        args.len(),
                    )
                    .at(*span)
                    .build());
                }

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

                // 单态化：处理编译期泛型参数
                let mono_func_ty = self.monomorphize(func_ty.clone(), &arg_types);

                // 收集实例化请求：检测泛型函数调用并记录
                self.collect_instantiation_request(
                    &func_ty,
                    func.as_ref(),
                    &arg_types,
                    &mono_func_ty,
                    *span,
                );

                // 泛型类型构造：当函数名在 generic_type_defs 中且 func_ty 是 Struct 时，
                // 直接使用 arg_types 调用 instantiate_generic_type（Layer 1 + Layer 2）
                if let crate::frontend::core::parser::ast::Expr::Var(fn_name, _) = &**func {
                    if let Some(generic_def) = self.generic_type_defs.get(fn_name).cloned() {
                        if let crate::frontend::core::types::MonoType::Struct(_) = &func_ty {
                            let type_param_count = generic_def.type_param_names.len();
                            let const_param_count = generic_def.poly.const_binders.len();
                            let expected_arg_count = type_param_count + const_param_count;

                            if arg_types.len() == expected_arg_count {
                                // const 参数需要 MonoType::Literal，而非 MonoType::Int
                                let mut full_args = arg_types.clone();
                                for (i, binder) in generic_def.poly.const_binders.iter().enumerate()
                                {
                                    let arg_idx = type_param_count + i;
                                    if let Some(arg) = full_args.get_mut(arg_idx) {
                                        if !matches!(arg, MonoType::Literal { .. }) {
                                            // 尝试从表达式提取字面量值
                                            if let Some(lit) = args.get(arg_idx) {
                                                if let Some(value) =
                                                    extract_const_value_from_expr(lit)
                                                {
                                                    *arg = MonoType::Literal {
                                                        name: format!("{}", value),
                                                        base_type: Box::new(arg.clone()),
                                                        value,
                                                    };
                                                }
                                            }
                                        }
                                    }
                                    let _ = binder; // 避免未使用警告
                                }
                                match crate::frontend::core::typecheck::TypeEnvironment::instantiate_generic_type(
                                    &generic_def,
                                    &full_args,
                                ) {
                                    Ok(result) => return Ok(result),
                                    Err(diag) => return Err(diag),
                                }
                            }
                        }
                    }
                }

                // 效应消费：成功调用后向流敏感 Γ 注入谓词
                // （如 std.assert(x > 0) 成功后把 x > 0 加入 Γ）
                if let crate::frontend::core::parser::ast::Expr::Var(fn_name, _) = &**func {
                    if let Some(dep_env) = self.dep_env {
                        if let Some(spec) = dep_env.get_effect_spec(fn_name) {
                            for effect in &spec.effects {
                                if let crate::frontend::core::types::eval::dependent_types::Effect::GammaAssume { predicate_arg } = effect {
                                    if let Some(arg_expr) = args.get(*predicate_arg) {
                                        if let Some(pred) = crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(arg_expr) {
                                            if let Some(gamma) = self.gamma.as_deref_mut() {
                                                gamma.inject(pred);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // 分发
                match mono_func_ty {
                    MonoType::Fn {
                        params,
                        return_type,
                        ..
                    } => {
                        // 值级函数调用
                        if arg_types.len() == params.len() {
                            for (arg_ty, param_ty) in arg_types.iter().zip(params.iter()) {
                                // 自动借用：当参数签名要求 &T 且实参是值类型时，
                                // 编译器自动创建令牌（RFC-009 §2.8）
                                let actual_arg = match (param_ty, arg_ty) {
                                    (MonoType::Ref { mutable, .. }, a)
                                        if !matches!(a, MonoType::Ref { .. }) =>
                                    {
                                        MonoType::Ref {
                                            mutable: *mutable,
                                            inner: Box::new(a.clone()),
                                        }
                                    }
                                    _ => arg_ty.clone(),
                                };
                                // Int -> Float 扩展转换是允许的
                                if matches!(
                                    (&actual_arg, param_ty),
                                    (MonoType::Int(_), MonoType::Float(_))
                                ) {
                                    continue;
                                }
                                // TypeRef 未完全解析时跳过（如用户自定义类型名）
                                if matches!(param_ty, MonoType::TypeRef(_)) {
                                    continue;
                                }
                                // TypeVar 是泛型类型参数 —— 必须 unify 以推断具体类型
                                if self.solver.unify(&actual_arg, param_ty).is_err() {
                                    return Err(ErrorCodeDefinition::type_mismatch(
                                        &format!("{}", param_ty),
                                        &format!("{}", arg_ty),
                                    )
                                    .at(*span)
                                    .build());
                                }
                            }
                        }
                        let resolved_ret = self.solver.expand_type_shallow(&return_type);
                        return Ok(resolved_ret);
                    }
                    MonoType::Struct(_) | MonoType::TypeRef(_) => {
                        // 类型构造器：Point(1.0, 2.0) 或 List(Int) 单态化后的结果
                        return Ok(mono_func_ty);
                    }
                    _ => {}
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
                            crate::util::span::Span::default(),
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
                self.scope.add_var(
                    name.clone(),
                    PolyType::mono(fn_type.clone()),
                    false,
                    crate::util::span::Span::default(),
                );

                Ok(fn_type)
            }

            // Lambda 表达式
            crate::frontend::core::parser::ast::Expr::Lambda {
                params,
                body,
                span: _span,
                ..
            } => {
                self.scope.enter_scope();
                for param in params {
                    let param_ty = self.solver.new_var();
                    self.scope.add_var(
                        param.name.clone(),
                        PolyType::mono(param_ty),
                        param.is_mut,
                        crate::util::span::Span::default(),
                    );
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
                self.infer_block(body, false, None)
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
                self.scope.add_var(
                    var.clone(),
                    PolyType::mono(MonoType::Char),
                    false,
                    crate::util::span::Span::default(),
                );

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

            // spawn for 数据并行循环（RFC-024 §2.4）
            crate::frontend::core::parser::ast::Expr::SpawnFor {
                var,
                var_mut,
                iterable,
                body,
                span,
                ..
            } => {
                // 1. 检查 iterable 类型，推导元素类型
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

                // 2. 进入循环作用域，注册迭代变量
                self.enter_loop(None);
                self.scope.enter_scope();
                let body_ty = self
                    .try_add_var(var.clone(), PolyType::mono(element_type), *span, *var_mut)
                    .and_then(|_| self.infer_block(body, true, None));

                self.promote_loop_vars_to_parent_scope();

                match body_ty {
                    Ok(ty) => {
                        // spawn for 返回 List(T)，T 是循环体返回类型
                        if matches!(ty, MonoType::Void) {
                            Ok(MonoType::List(Box::new(MonoType::Void)))
                        } else {
                            Ok(MonoType::List(Box::new(ty)))
                        }
                    }
                    Err(e) => Err(e),
                }
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
            match &stmt.kind {
                crate::frontend::core::parser::ast::StmtKind::Expr(ref expr_stmt) => {
                    if let Some(ty) = self.collect_return_type(expr_stmt)? {
                        return_type = Some(ty);
                    }
                }
                crate::frontend::core::parser::ast::StmtKind::Return(Some(ref ret_expr)) => {
                    let ty = self.infer_expr(ret_expr)?;
                    return_type = Some(ty);
                }
                _ => {}
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
                            self.assign_var(name, init_ty);
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
                body,
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

                self.scope.add_var(
                    name.clone(),
                    PolyType::mono(fn_type),
                    false,
                    crate::util::span::Span::default(),
                );

                self.scope.enter_scope();
                let result: Result<()> = (|| {
                    for (param, param_ty) in params.iter().zip(param_types.iter()) {
                        self.scope.add_var(
                            param.name.clone(),
                            PolyType::mono(param_ty.clone()),
                            param.is_mut,
                            crate::util::span::Span::default(),
                        );
                    }

                    let block = crate::frontend::core::parser::ast::Block {
                        stmts: body.clone(),
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

/// Extract a string literal from an AST expression (compile-time evaluation helper)
fn extract_string_literal_from_expr(
    expr: &crate::frontend::core::parser::ast::Expr
) -> Option<String> {
    match expr {
        crate::frontend::core::parser::ast::Expr::Lit(
            crate::frontend::core::lexer::tokens::Literal::String(s),
            _,
        ) => Some(s.clone()),
        _ => None,
    }
}
/// 从表达式提取编译期常量值（用于 const 泛型参数）
fn extract_const_value_from_expr(
    expr: &crate::frontend::core::parser::ast::Expr
) -> Option<crate::frontend::core::types::const_data::ConstValue> {
    use crate::frontend::core::types::const_data::ConstValue;
    match expr {
        crate::frontend::core::parser::ast::Expr::Lit(
            crate::frontend::core::lexer::tokens::Literal::Int(n),
            _,
        ) => Some(ConstValue::Int(*n)),
        crate::frontend::core::parser::ast::Expr::Lit(
            crate::frontend::core::lexer::tokens::Literal::Bool(b),
            _,
        ) => Some(ConstValue::Bool(*b)),
        crate::frontend::core::parser::ast::Expr::Lit(
            crate::frontend::core::lexer::tokens::Literal::Float(f),
            _,
        ) => Some(ConstValue::Float(*f as f32)),
        _ => None,
    }
}
/// 检查名称是否为内置类型名（Type 宇宙的值）
fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "Int"
            | "int"
            | "Float"
            | "float"
            | "Bool"
            | "bool"
            | "String"
            | "string"
            | "Void"
            | "void"
            | "Never"
            | "never"
            | "Char"
            | "char"
            | "Type"
    )
}
