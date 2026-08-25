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

    /// 添加参数（lambda 参数，lambda 体可继承）
    pub fn add_param(
        &mut self,
        name: String,
        poly: PolyType,
        is_mut: bool,
    ) {
        self.scope
            .add_param(name, poly, is_mut, crate::util::span::Span::default());
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
        self.scope.exit_block();

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
        self.scope.enter_block();
    }

    /// 退出当前作用域
    pub fn exit_scope(&mut self) {
        self.scope.exit_block();
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
            crate::frontend::core::lexer::tokens::Literal::String(_) => MonoType::make_string(),
            crate::frontend::core::lexer::tokens::Literal::Void => MonoType::Void,
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
                } else if left.is_string() && right.is_string() {
                    Ok(MonoType::make_string())
                } else if left.is_list() && right.is_list() {
                    let left_elem = &left.generic_args().expect("List args")[0];
                    let right_elem = &right.generic_args().expect("List args")[0];
                    let _ = self.solver.unify(left_elem, right_elem);
                    let elem_ty = self.solver.resolve_type(left_elem);
                    Ok(MonoType::make_list(elem_ty))
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
                Ok(MonoType::Generic {
                    name: "Range".into(),
                    args: vec![elem_ty],
                })
            }
            // #285: 位运算/移位仅限 Int（SPEC §2.2 级 7/8）
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                if let (MonoType::Int(_), MonoType::Int(_)) = (left, right) {
                    Ok(left.clone())
                } else {
                    Err(ErrorCodeDefinition::type_mismatch("Int", &format!("{}", left)).build())
                }
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
            MonoType::Generic { name, args } if name == "Range" && args.len() == 1 => {
                Self::collect_type_var_indices(&args[0], out)
            }
            MonoType::Union(types) | MonoType::Intersection(types) => {
                for t in types {
                    Self::collect_type_var_indices(t, out);
                }
            }
            MonoType::Ref { inner, .. } => Self::collect_type_var_indices(inner, out),
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
        _arg_types: &[MonoType],
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

        // 收集「参数位直接是 TypeVar」的位置（如 identity 的 x: T、twice 的 x: T）。
        // 不能收集嵌套 TypeVar 的参数位：twice(f: (T) -> T, x: T) 的 f 位是 fn(T)->T，
        // 解析后是 fn(int64)->int64，用它当 type_arg 会让 T 绑到错误类型（#255）。
        let mut type_var_indices = HashSet::new();
        for (i, param) in original_params.iter().enumerate() {
            if matches!(self.solver.resolve_type(param), MonoType::TypeVar(_)) {
                type_var_indices.insert(i);
            }
        }

        // 没有 TypeVar → 不是泛型函数调用（除非是 MetaType 构造器，交给类型特化路径）
        if type_var_indices.is_empty() {
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
            // 只收集 TypeVar 参数位对应的具体类型（去重）。
            // 不能收集所有具体参数类型：twice(x => x+1, 5) 的参位含 fn(int64)->int64 与
            // int64，但 T 只有一个（int64）——全收集会让 type_args 长度与 type_params 不匹配、
            // 特化失败（#255）。
            let mut type_args = Vec::new();
            let mut seen = HashSet::new();
            for &idx in &type_var_indices {
                if let Some(resolved) = resolved_params.get(idx) {
                    let resolved = self.solver.resolve_type(resolved);
                    if !matches!(resolved, MonoType::TypeVar(_)) {
                        let key = format!("{}", resolved);
                        if seen.insert(key) {
                            type_args.push(resolved);
                        }
                    }
                }
            }

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
                Ok(MonoType::make_tuple(types?))
            }

            // 列表
            crate::frontend::core::parser::ast::Expr::List(elems, _) => {
                if elems.is_empty() {
                    let elem_ty = self.solver.new_var();
                    Ok(MonoType::make_list(elem_ty))
                } else {
                    let mut iter = elems.iter();
                    let first = iter.next().expect("non-empty list must have first element");
                    let mut elem_ty = self.infer_expr(first)?;
                    for e in iter {
                        let ty = self.infer_expr(e)?;
                        let _ = self.solver.unify(&elem_ty, &ty);
                        elem_ty = self.solver.resolve_type(&elem_ty);
                    }
                    Ok(MonoType::make_list(elem_ty))
                }
            }

            // 字典
            crate::frontend::core::parser::ast::Expr::Dict(pairs, _) => {
                if pairs.is_empty() {
                    let key_ty = self.solver.new_var();
                    let value_ty = self.solver.new_var();
                    Ok(MonoType::make_dict(key_ty, value_ty))
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
                    Ok(MonoType::make_dict(
                        key_ty.unwrap_or_else(|| self.solver.new_var()),
                        value_ty.unwrap_or_else(|| self.solver.new_var()),
                    ))
                }
            }

            // 下标访问
            // #299 §3: membership 谓词 `elem in container` → Bool
            // 右操作数：List/Array/Dict(键)/Set/Tuple/String 子串/Range 区间
            crate::frontend::core::parser::ast::Expr::In {
                elem, container, ..
            } => {
                let _elem_ty = self.infer_expr(elem)?;
                let container_ty = self.infer_expr(container)?;
                // Range 字面量（1..10）类型是 Generic{"Range"}，区间检查合法
                // #300 决策4：Set 除名——无运行时表示（HeapValue/std.set 均不存在），
                // 待真实需求出现时照 Dict 模式补全
                match &container_ty {
                    MonoType::Generic { name, .. }
                        if matches!(
                            name.as_str(),
                            "List" | "Array" | "Dict" | "Tuple" | "Range"
                        ) =>
                    {
                        Ok(MonoType::Bool)
                    }
                    MonoType::Generic { name, .. } if name == "String" => Ok(MonoType::Bool),
                    _ => Ok(MonoType::Bool),
                }
            }
            crate::frontend::core::parser::ast::Expr::Index {
                expr: container,
                index,
                ..
            } => {
                let container_ty = self.infer_expr(container)?;
                match container_ty {
                    MonoType::Generic { name, args } if name == "List" => Ok(args[0].clone()),
                    MonoType::Generic { name, args } if name == "Dict" => Ok(args[1].clone()),
                    MonoType::Generic { name, args } if name == "Tuple" => {
                        if let crate::frontend::core::parser::ast::Expr::Lit(
                            crate::frontend::core::lexer::tokens::Literal::Int(i),
                            _,
                        ) = index.as_ref()
                        {
                            if *i >= 0 && (*i as usize) < args.len() {
                                Ok(args[*i as usize].clone())
                            } else {
                                Err(ErrorCodeDefinition::index_out_of_bounds(
                                    args.len(),
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
                func,
                args,
                named_args,
                span,
                ..
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

                // 两层调用：Container(Int)(42, 43) —— func 是泛型类型构造调用，
                // 内层已完成实例化（func_ty 是具体 Struct），外层实参是构造参数。
                if let crate::frontend::core::parser::ast::Expr::Call {
                    func: inner_func, ..
                } = &**func
                {
                    if let crate::frontend::core::parser::ast::Expr::Var(inner_name, _) =
                        &**inner_func
                    {
                        if self.generic_type_defs.contains_key(inner_name) {
                            if let MonoType::Struct(st) = &func_ty {
                                // 构造参数 arity（同普通 struct #271#1：有默认值字段可省略）
                                let total = st.fields.len();
                                let required = st.field_has_default.iter().filter(|&&d| !d).count();
                                let provided = arg_types.len();
                                // 空构造 X(参数)()：字段取默认值/零值（RFC §9.3 模式），合法
                                if provided == 0 && named_args.is_empty() {
                                    return Ok(func_ty.clone());
                                }
                                if named_args.is_empty() {
                                    if provided < required || provided > total {
                                        return Err(ErrorCodeDefinition::argument_count_mismatch(
                                            inner_name, total, provided,
                                        )
                                        .at(*span)
                                        .build());
                                    }
                                } else {
                                    // 命名参数：必需字段（无默认值）必须全部提供
                                    let provided_names: std::collections::HashSet<&str> =
                                        named_args.iter().map(|(n, _)| n.as_str()).collect();
                                    let missing: Vec<&str> = st
                                        .fields
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| !st.field_has_default[*i])
                                        .map(|(_, (n, _))| n.as_str())
                                        .filter(|n| !provided_names.contains(n))
                                        .collect();
                                    if !missing.is_empty() {
                                        return Err(ErrorCodeDefinition::type_mismatch(
                                            &format!(
                                                "{} constructor missing required field(s): {}",
                                                inner_name,
                                                missing.join(", ")
                                            ),
                                            &format!("provided {}", provided_names.len()),
                                        )
                                        .at(*span)
                                        .build());
                                    }
                                }
                                // 位置实参与字段类型一致性（#286 同款：实例化后字段已具体）
                                for (i, (_, field_ty)) in st.fields.iter().enumerate() {
                                    if i >= provided {
                                        break;
                                    }
                                    let Some(arg_ty) = arg_types.get(i) else {
                                        break;
                                    };
                                    if self.solver.unify(field_ty, arg_ty).is_err() {
                                        return Err(ErrorCodeDefinition::type_mismatch(
                                            &format!("{}", field_ty),
                                            &format!("{}", arg_ty),
                                        )
                                        .at(*span)
                                        .build());
                                    }
                                }
                                return Ok(func_ty.clone());
                            }
                        }
                    }
                }

                // 泛型类型构造调用分派（SPEC type-system.md §4.3）：
                // 实参自左向右逐位匹配类型声明参数（Type 位收类型实参，const 位收字面量）。
                //   - 全匹配 → 类型构造实例化
                //   - 部分匹配（至少一位匹配上）→ 按类型构造报错：逐位检查，先报第一个错误位
                //   - 完全匹配不上 → 构造参数（自动生成的构造函数）：位置式按字段顺序填，
                //     类型参数从元素类型自动解包；const 位无法自动解包时报错。
                if let crate::frontend::core::parser::ast::Expr::Var(fn_name, _) = &**func {
                    if let Some(generic_def) = self.generic_type_defs.get(fn_name).cloned() {
                        if let crate::frontend::core::types::MonoType::Struct(struct_body) =
                            &func_ty
                        {
                            let type_param_count = generic_def.type_param_names.len();
                            let const_param_count = generic_def.poly.const_binders.len();
                            let total_params = type_param_count + const_param_count;

                            // 位匹配判定：Type 位收 MetaType 实参，const 位收字面量实参。
                            // 部分匹配 = 存在能填某个声明参数位的实参（MetaType 或
                            // const 位可收的字面量）；全匹配 = 位置对齐逐位吻合。
                            let meta_count = arg_types
                                .iter()
                                .filter(|a| matches!(a, MonoType::MetaType { .. }))
                                .count();
                            let lit_count = args
                                .iter()
                                .filter(|a| extract_const_value_from_expr(a).is_some())
                                .count();
                            let all_matched = args.len() == total_params
                                && (0..type_param_count)
                                    .all(|i| matches!(arg_types[i], MonoType::MetaType { .. }))
                                && (type_param_count..total_params)
                                    .all(|i| extract_const_value_from_expr(&args[i]).is_some());
                            let any_matched =
                                meta_count > 0 || (const_param_count > 0 && lit_count > 0);

                            if all_matched {
                                // === 类型构造（全匹配）：SafeArray(Int, 3) / Container(Int) ===
                                let mut full_args = arg_types.clone();
                                // 类型实参解包：表达式位置的类型名 infer 成 MetaType 空壳
                                // （不存具体类型名），从 AST 实参名提取具体类型。
                                for i in 0..type_param_count {
                                    if matches!(full_args[i], MonoType::MetaType { .. }) {
                                        if let Some(concrete) = concrete_type_from_expr_arg(
                                            &args[i],
                                            self.type_defs,
                                            self.generic_type_defs,
                                        ) {
                                            full_args[i] = concrete;
                                        }
                                    }
                                }
                                // const 参数需要 MonoType::Literal，而非 MonoType::Int
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
                                return crate::frontend::core::typecheck::TypeEnvironment::instantiate_generic_type(
                                    &generic_def,
                                    &full_args,
                                );
                            }

                            if any_matched {
                                // === 部分匹配 → 一层处理：逐位检查，先报第一个错误位 ===
                                // （Matrix(42)：位0 T←42 不匹配 → 报位0，即使位1 Rows 可匹配）
                                let check_len = total_params.min(args.len());
                                for i in 0..check_len {
                                    let ok = if i < type_param_count {
                                        matches!(arg_types[i], MonoType::MetaType { .. })
                                    } else {
                                        extract_const_value_from_expr(&args[i]).is_some()
                                    };
                                    if !ok {
                                        let pname = if i < type_param_count {
                                            generic_def.type_param_names[i].clone()
                                        } else {
                                            generic_def.poly.const_binders[i - type_param_count]
                                                .name
                                                .clone()
                                        };
                                        let expected = if i < type_param_count {
                                            "类型实参".to_string()
                                        } else {
                                            "编译期常量".to_string()
                                        };
                                        return Err(ErrorCodeDefinition::type_mismatch(
                                            &format!("{}（{}）", pname, expected),
                                            &format!("{}", arg_types[i]),
                                        )
                                        .at(*span)
                                        .build());
                                    }
                                }
                                // 已匹配的位全部正确：缺参或超参
                                return Err(ErrorCodeDefinition::argument_count_mismatch(
                                    fn_name,
                                    total_params,
                                    args.len(),
                                )
                                .at(*span)
                                .build());
                            }

                            // === 完全匹配不上 → 构造参数（自动生成的构造函数）===
                            // 位置式按字段顺序填；类型参数从元素自动解包。
                            // const 位无法从元素解包 → 必须显式两层 Matrix(Int, 3, 4)(...)
                            if const_param_count > 0 {
                                return Err(ErrorCodeDefinition::type_mismatch(
                                    &format!(
                                        "{}（显式类型构造参数，如 {}(类型, ...)(构造参数)",
                                        fn_name, fn_name
                                    ),
                                    "构造参数值（编译期值参数无法从元素自动解包）",
                                )
                                .at(*span)
                                .build());
                            }

                            // === 值构造（二层，无 const）：Container(42, 43) ===
                            // #287：arity = 构造参数数（有默认值字段可省略），
                            // 类型参数从字段值类型推断。
                            {
                                let total = struct_body.fields.len();
                                let required = struct_body
                                    .field_has_default
                                    .iter()
                                    .filter(|&&d| !d)
                                    .count();
                                let provided = arg_types.len();
                                if provided < required || provided > total {
                                    return Err(ErrorCodeDefinition::argument_count_mismatch(
                                        fn_name, total, provided,
                                    )
                                    .at(*span)
                                    .build());
                                }

                                // 类型参数推断：TypeRef(param) → 独立 fresh TypeVar（同一参数共享）
                                // → unify 字段类型与实参类型 → resolve 出类型参数具体值。
                                let mut param_vars: HashMap<String, MonoType> = HashMap::new();
                                for pname in &generic_def.type_param_names {
                                    param_vars.insert(pname.clone(), self.solver.new_var());
                                }
                                for (i, (_, field_ty)) in struct_body.fields.iter().enumerate() {
                                    if i >= provided {
                                        break;
                                    }
                                    let Some(arg_ty) = arg_types.get(i) else {
                                        break;
                                    };
                                    let subst =
                                        substitute_type_params_with_vars(field_ty, &param_vars);
                                    if self.solver.unify(&subst, arg_ty).is_err() {
                                        return Err(ErrorCodeDefinition::type_mismatch(
                                            &format!("{}", subst),
                                            &format!("{}", arg_ty),
                                        )
                                        .at(*span)
                                        .build());
                                    }
                                }
                                let type_args: Vec<MonoType> = generic_def
                                    .type_param_names
                                    .iter()
                                    .map(|p| {
                                        param_vars
                                            .get(p)
                                            .map(|v| self.solver.resolve_type(v))
                                            .unwrap_or_else(|| MonoType::TypeRef(p.clone()))
                                    })
                                    .collect();
                                return crate::frontend::core::typecheck::TypeEnvironment::instantiate_generic_type(
                                    &generic_def,
                                    &type_args,
                                );
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
                // #271#1：统一参数个数检查（构造器缺参/超参、函数缺参/超参）。
                // 泛型类型构造器（List(1,2,3) 值构造）豁免——其参数语义是类型参数+值参数，
                // 不适用字段计数（RFC-011），在 match 前拦下。
                if let crate::frontend::core::parser::ast::Expr::Var(fn_name, _) = &**func {
                    // 豁免：泛型类型构造器（List(1,2,3) 值构造是 RFC-011 语义，非字段计数）；
                    // native 函数（std 可选参数 ?msg / 变参 ...args，签名 params.len() 不可靠，
                    // assert(1>0) 合法但 params 有 2 项——原 Fn 分支对数量不等静默跳过，
                    // 正是这种宽容路径）。
                    if !self.generic_type_defs.contains_key(fn_name)
                        && !self.native_signatures.contains_key(fn_name)
                    {
                        let provided = arg_types.len();
                        match &mono_func_ty {
                            MonoType::Struct(st) => {
                                // 普通 struct 构造器：Point(1.0, 2.0)。
                                // 有默认值的字段可省略 → 必需参数数 = 无默认值字段数。
                                let total = st.fields.len();
                                let required = st.field_has_default.iter().filter(|&&d| !d).count();
                                if named_args.is_empty() {
                                    // 位置参数：Point(5) 缺参 / Point(5,6,7) 超参
                                    if provided < required || provided > total {
                                        return Err(ErrorCodeDefinition::argument_count_mismatch(
                                            &st.name, total, provided,
                                        )
                                        .at(*span)
                                        .build());
                                    }
                                } else {
                                    // 命名参数：Point(x=6) 缺必需字段 → 静默 0（#271#1）。
                                    // 检查必需字段（无默认值）是否全部提供。
                                    let provided_names: std::collections::HashSet<&str> =
                                        named_args.iter().map(|(n, _)| n.as_str()).collect();
                                    let missing: Vec<&str> = st
                                        .fields
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| !st.field_has_default[*i])
                                        .map(|(_, (n, _))| n.as_str())
                                        .filter(|n| !provided_names.contains(n))
                                        .collect();
                                    if !missing.is_empty() {
                                        let msg = format!(
                                            "{} constructor missing required field(s): {}",
                                            st.name,
                                            missing.join(", ")
                                        );
                                        return Err(ErrorCodeDefinition::type_mismatch(
                                            &msg,
                                            &format!("provided {}", provided_names.len()),
                                        )
                                        .at(*span)
                                        .build());
                                    }
                                }
                            }
                            MonoType::Fn { params, .. }
                                // 普通函数调用：add(5) 缺参 → E6007 运行时错（晚且误导）；
                                // add(1,2,3) 超参静默丢弃。拦为编译期 E1010。
                                // 仅当 params 非空时检查：lambda/块函数绑定（mk: (Int,Int)->Int
                                // = (x,y)=>x+y）在 scope 里参数类型丢失（params 为空），
                                // 计数不可靠，跳过避免误伤（#271 记 lambda 绑定参数丢失）。
                                if named_args.is_empty()
                                    && !params.is_empty()
                                    && provided != params.len()
                                => {
                                    return Err(ErrorCodeDefinition::argument_count_mismatch(
                                        fn_name,
                                        params.len(),
                                        provided,
                                    )
                                    .at(*span)
                                    .build());
                                }
                            _ => {}
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
                                // resolve 参数类型：TypeRef("Int") → Int(64) 等
                                let mut resolved_param = self.solver.resolve_type(param_ty);
                                // Int -> Float 扩展转换是允许的
                                if matches!(
                                    (&actual_arg, &resolved_param),
                                    (MonoType::Int(_), MonoType::Float(_))
                                ) {
                                    continue;
                                }
                                // TypeRef: 先 solver.resolve 解析内置类型（Int/Float 等），
                                // 再 type_defs 解析用户自定义类型；都不匹配则跳过
                                if let MonoType::TypeRef(name) = &resolved_param {
                                    resolved_param = match self.type_defs.get(name) {
                                        Some(def_ty) => self.solver.resolve_type(def_ty),
                                        None => continue,
                                    };
                                }
                                if self.solver.unify(&actual_arg, &resolved_param).is_err() {
                                    return Err(ErrorCodeDefinition::type_mismatch(
                                        &format!("{}", resolved_param),
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
                        // 类型构造器：Point(1.0, 2.0) 或 List(Int) 单态化后的结果。
                        // 参数个数检查已在 #271#1 前置块完成（泛型构造器豁免）。
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
                else_if_branches,
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

                self.scope.enter_block();
                let then_result = self.infer_block(then_branch, true, None);
                self.scope.exit_block();
                let _then_ty = then_result?;

                for (else_if_cond, else_if_block) in else_if_branches {
                    let else_if_cond_ty = self.infer_expr(else_if_cond)?;
                    if else_if_cond_ty != MonoType::Bool {
                        return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                            "{}",
                            else_if_cond_ty
                        ))
                        .build());
                    }
                    self.scope.enter_block();
                    let else_if_result = self.infer_block(else_if_block, true, None);
                    self.scope.exit_block();
                    let _ = else_if_result?;
                }

                if let Some(else_block) = else_branch {
                    self.scope.enter_block();
                    let else_result = self.infer_block(else_block, true, None);
                    self.scope.exit_block();
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

                self.scope.enter_block();
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
                    MonoType::Generic { name, args } if name == "List" => args[0].clone(),
                    MonoType::Generic { name, args } if name == "Range" && args.len() == 1 => {
                        args[0].clone()
                    }
                    MonoType::Generic { name, .. } if name == "String" => MonoType::Char,
                    MonoType::Generic { name, .. } if name == "Tuple" => self.solver.new_var(),
                    MonoType::Generic { name, args } if name == "Dict" => {
                        MonoType::make_tuple(vec![args[0].clone(), args[1].clone()])
                    }
                    _ => self.solver.new_var(),
                };

                self.enter_loop(label.as_deref());

                self.scope.enter_block();
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
                self.scope.enter_fn();
                let result: Result<()> = (|| {
                    for param in params {
                        let param_ty = self.solver.new_var();
                        self.add_param(param.name.clone(), PolyType::mono(param_ty), param.is_mut);
                    }

                    let ret_mono: MonoType =
                        return_type.clone().map_or(MonoType::Void, |t| t.into());
                    // RFC-001: Result-returning functions implicitly wrap the final value in Ok(...),
                    // so the body type is the Ok type (not Result[T, E]).
                    let expected_body_ty = match &ret_mono {
                        m if m.is_result() => {
                            let args = m.generic_args().unwrap();
                            args[0].clone()
                        }
                        _ => ret_mono.clone(),
                    };

                    // Enter a new `Result` context for this function body.
                    let saved_result_err = self.result_err.take();
                    self.result_err = match &ret_mono {
                        m if m.is_result() => {
                            let args = m.generic_args().unwrap();
                            Some(args[1].clone())
                        }
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
                self.scope.exit_fn();
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
                self.scope.enter_fn();
                // #295：三链模型——enter_fn 推新局部层，外层函数局部变量不在链上（闭包不捕获），
                // 参数链跨边界累积可见（柯里化固化）。
                for param in params {
                    let param_ty = self.solver.new_var();
                    self.add_param(param.name.clone(), PolyType::mono(param_ty), param.is_mut);
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

                self.scope.exit_fn();
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
                let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

                if let Err(_e) = self.solver.unify(&inner_ty, &expected_result) {
                    let resolved = self.solver.resolve_type(&inner_ty);
                    if resolved.is_result() {
                        let err = &resolved.generic_args().expect("Result args")[1];
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
                Ok(MonoType::Generic {
                    name: "Arc".into(),
                    args: vec![expr_ty],
                })
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

                self.scope.enter_block();
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

                self.scope.exit_block();

                Ok(MonoType::make_list(elem_ty))
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
                Ok(MonoType::make_string())
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
                    MonoType::Generic { name, args } if name == "List" => args[0].clone(),
                    MonoType::Generic { name, args } if name == "Range" && args.len() == 1 => {
                        args[0].clone()
                    }
                    MonoType::Generic { name, .. } if name == "String" => MonoType::Char,
                    MonoType::Generic { name, .. } if name == "Tuple" => self.solver.new_var(),
                    MonoType::Generic { name, args } if name == "Dict" => {
                        MonoType::make_tuple(vec![args[0].clone(), args[1].clone()])
                    }
                    _ => self.solver.new_var(),
                };

                // 2. 进入循环作用域，注册迭代变量
                self.enter_loop(None);
                self.scope.enter_block();
                let body_ty = self
                    .try_add_var(var.clone(), PolyType::mono(element_type), *span, *var_mut)
                    .and_then(|_| self.infer_block(body, true, None));

                self.promote_loop_vars_to_parent_scope();

                match body_ty {
                    Ok(ty) => {
                        // spawn for 返回 List(T)，T 是循环体返回类型
                        if matches!(ty, MonoType::Void) {
                            Ok(MonoType::make_list(MonoType::Void))
                        } else {
                            Ok(MonoType::make_list(ty))
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
            crate::frontend::core::parser::ast::StmtKind::Assign {
                target,
                type_annotation,
                value,
                is_mut,
                span: stmt_span,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    _ => return Ok(()),
                };
                // 如果 value 是 Lambda，走函数推断
                if let Some(v) = value {
                    if let Expr::Lambda { params, .. } = v.as_ref() {
                        let param_types: Vec<MonoType> =
                            params.iter().map(|_| self.solver.new_var()).collect();
                        let return_type = type_annotation
                            .as_ref()
                            .map_or(MonoType::Void, |t| t.clone().into());
                        let fn_type = MonoType::Fn {
                            params: param_types,
                            return_type: Box::new(return_type),
                        };
                        self.try_add_var(
                            name.clone(),
                            PolyType::mono(fn_type),
                            *stmt_span,
                            *is_mut,
                        )?;
                        return Ok(());
                    }
                    if let Expr::Block(..) = v.as_ref() {
                        let fn_type = MonoType::Fn {
                            params: vec![],
                            return_type: Box::new(
                                type_annotation
                                    .as_ref()
                                    .map_or(MonoType::Void, |t| t.clone().into()),
                            ),
                        };
                        self.try_add_var(
                            name.clone(),
                            PolyType::mono(fn_type),
                            *stmt_span,
                            *is_mut,
                        )?;
                        return Ok(());
                    }
                }
                // 普通变量
                let init_ty = if let Some(expr) = value {
                    self.infer_expr(expr)?
                } else {
                    type_annotation
                        .as_ref()
                        .map_or_else(|| self.solver.new_var(), |t| t.clone().into())
                };
                if self.scope.var_in_any_scope(&name) {
                    if self.scope.var_in_current_scope(&name) {
                        if self.scope.var_is_moved(&name).unwrap_or(false) {
                            self.scope.remove_var(&name);
                        } else {
                            return Err(ErrorCodeDefinition::duplicate_definition(&name)
                                .at(*stmt_span)
                                .build());
                        }
                    } else {
                        if self.scope.var_is_moved(&name).unwrap_or(false) {
                            // 外层变量已 moved：在当前作用域重新声明
                        } else if !*is_mut {
                            if !self.scope.var_is_mutable(&name).unwrap_or(false) {
                                return Err(ErrorCodeDefinition::immutable_assignment(&name)
                                    .at(*stmt_span)
                                    .build());
                            }
                            self.assign_var(&name, init_ty);
                            return Ok(());
                        }
                    }
                }
                self.try_add_var(name.clone(), PolyType::mono(init_ty), *stmt_span, *is_mut)?;
                Ok(())
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

/// #287: 将泛型构造器字段类型中的 TypeRef(类型参数名) 替换为对应 TypeVar，
/// 供 unify 推断类型参数的具体值。非参数 TypeRef 原样保留。
fn substitute_type_params_with_vars(
    ty: &MonoType,
    param_vars: &HashMap<String, MonoType>,
) -> MonoType {
    match ty {
        MonoType::TypeRef(name) => param_vars.get(name).cloned().unwrap_or_else(|| ty.clone()),
        MonoType::Struct(s) => MonoType::Struct(crate::frontend::core::types::mono::StructType {
            fields: s
                .fields
                .iter()
                .map(|(n, t)| (n.clone(), substitute_type_params_with_vars(t, param_vars)))
                .collect(),
            ..s.clone()
        }),
        MonoType::Generic { name, args } => MonoType::Generic {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| substitute_type_params_with_vars(a, param_vars))
                .collect(),
        },
        MonoType::Fn {
            params,
            return_type,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|p| substitute_type_params_with_vars(p, param_vars))
                .collect(),
            return_type: Box::new(substitute_type_params_with_vars(return_type, param_vars)),
        },
        MonoType::Ref { mutable, inner } => MonoType::Ref {
            mutable: *mutable,
            inner: Box::new(substitute_type_params_with_vars(inner, param_vars)),
        },
        MonoType::Union(v) => MonoType::Union(
            v.iter()
                .map(|t| substitute_type_params_with_vars(t, param_vars))
                .collect(),
        ),
        MonoType::Intersection(v) => MonoType::Intersection(
            v.iter()
                .map(|t| substitute_type_params_with_vars(t, param_vars))
                .collect(),
        ),
        _ => ty.clone(),
    }
}

/// 从类型构造实参表达式提取具体类型：类型名在表达式位置 infer 成 MetaType 空壳
/// （不存具体类型名），实例化泛型类型前需从 AST 名称解包成具体 MonoType。
fn concrete_type_from_expr_arg(
    expr: &crate::frontend::core::parser::ast::Expr,
    type_defs: &HashMap<String, MonoType>,
    generic_type_defs: &HashMap<
        String,
        crate::frontend::core::typecheck::environment::GenericTypeDef,
    >,
) -> Option<MonoType> {
    match expr {
        crate::frontend::core::parser::ast::Expr::Var(name, _) => {
            if is_builtin_type_name(name) {
                MonoType::from_builtin_name(name)
            } else if let Some(def_ty) = type_defs.get(name) {
                Some(def_ty.clone())
            } else if generic_type_defs.contains_key(name) {
                // 泛型类型名（未实例化引用，如 Container(Container) 的类型实参位）
                Some(MonoType::TypeRef(name.clone()))
            } else {
                None
            }
        }
        _ => None,
    }
}
