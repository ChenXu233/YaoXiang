#![allow(clippy::result_large_err)]

//! RFC-011 类型边界检查
//!
//! 检查泛型类型边界和约束
//! 支持鸭子类型：检查类型是否满足接口要求的所有方法（包括方法绑定）

use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition, Result};
use crate::frontend::core::typecheck::proof::verdict::DisproofKind;
use crate::frontend::core::typecheck::proof::verdict::DisproofModel;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::types::const_data::ConstVarDef;
use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::TraitTable;
use crate::frontend::core::types::eval::const_eval::ConstGenericEval;
use crate::frontend::core::types::ConstValue;
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::util::span::Span;

/// 约束检查错误
#[derive(Debug, Clone)]
pub struct ConstraintCheckError {
    pub type_name: String,
    pub constraint_name: String,
    pub reason: String,
    pub span: Span,
}

/// 边界检查器
pub struct BoundsChecker {
    trait_table: TraitTable,
}

impl Default for BoundsChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundsChecker {
    /// 创建新的边界检查器
    pub fn new() -> Self {
        Self {
            trait_table: TraitTable::with_std(),
        }
    }

    /// 设置 trait 表
    pub fn set_trait_table(
        &mut self,
        table: TraitTable,
    ) {
        self.trait_table = table;
    }

    /// 获取 trait 表引用
    pub fn trait_table(&self) -> &TraitTable {
        &self.trait_table
    }

    /// 检查特质边界
    pub fn check_trait_bounds(
        &self,
        ty: &MonoType,
        bounds: &[String],
    ) -> Result<()> {
        for bound in bounds {
            if !self.trait_table.satisfies(bound, ty) {
                // 尝试自动派生（仅对结构体类型）
                if let MonoType::Struct(s) = ty {
                    if self.trait_table.can_auto_derive_for_monotype(bound, s) {
                        continue;
                    }
                }
                return Err(ErrorCodeDefinition::trait_bound_not_satisfied(
                    &format!("{}", ty),
                    bound,
                )
                .build());
            }
        }
        Ok(())
    }

    /// 检查 Const 边界（快慢路径模型）
    ///
    /// Layer 1: 类型匹配 fast path — 直接验证 ConstKind::matches
    /// Layer 2: 值约束求值 — ConstGenericEval 求值约束表达式
    pub fn check_const_bounds(
        &self,
        const_binders: &[ConstVarDef],
        const_args: &[MonoType],
    ) -> ProofResult {
        // Layer 1: 类型匹配（fast path）
        if let Err(diag) = validate_const_args(const_binders, const_args) {
            return ProofResult::Disproved(DisproofModel {
                kind: DisproofKind::TypeMismatch,
                assignments: Vec::new(),
                constraint: diag.message,
                span: diag.span,
                predicate_span: None,
            });
        }

        // Layer 2: 值约束求值
        for (binder, arg) in const_binders.iter().zip(const_args.iter()) {
            for constraint in &binder.constraints {
                let mut eval = ConstGenericEval::new();
                if let MonoType::Literal { value, .. } = arg {
                    eval.bind_var(binder.name.clone(), value.clone());
                }
                match eval.eval(constraint) {
                    Ok(ConstValue::Bool(true)) => continue,
                    Ok(ConstValue::Bool(false)) => {
                        return ProofResult::Disproved(DisproofModel {
                            kind: DisproofKind::PredicateViolation,
                            assignments: Vec::new(),
                            constraint: format!("const 参数 `{}` 不满足约束", binder.name),
                            span: None,
                            predicate_span: None,
                        });
                    }
                    Ok(_) | Err(_) => {
                        return ProofResult::Disproved(DisproofModel {
                            kind: DisproofKind::PredicateViolation,
                            assignments: Vec::new(),
                            constraint: format!("无法验证 const 参数 `{}` 的约束", binder.name),
                            span: None,
                            predicate_span: None,
                        });
                    }
                }
            }
        }

        ProofResult::Proved
    }

    /// 检查泛型参数边界
    pub fn check_generic_bounds(
        &self,
        ty: &MonoType,
        trait_bounds: &[String],
        const_binders: &[ConstVarDef],
        const_args: &[MonoType],
    ) -> Result<()> {
        self.check_trait_bounds(ty, trait_bounds)?;
        let result = self.check_const_bounds(const_binders, const_args);
        result.into_result()
    }

    /// 检查类型是否满足约束（结构化匹配 - 鸭子类型）
    pub fn check_constraint(
        &self,
        ty: &MonoType,
        constraint: &MonoType,
        env: Option<&TypeEnvironment>,
    ) -> Result<(), ConstraintCheckError> {
        let constraint_fields = constraint.constraint_fields();

        if constraint_fields.is_empty() {
            return Ok(());
        }

        let type_name = match ty {
            MonoType::Struct(s) => Some(s.name.clone()),
            MonoType::TypeRef(name) => Some(name.clone()),
            _ => None,
        };

        let type_fn_fields: Vec<(String, &MonoType)> = match ty {
            MonoType::Struct(s) => s
                .fields
                .iter()
                .filter(|(_, ty)| matches!(ty, MonoType::Fn { .. }))
                .map(|(name, ty)| (name.clone(), ty))
                .collect(),
            _ => Vec::new(),
        };

        let method_bindings: Vec<(String, MonoType)> =
            if let (Some(env), Some(ref name)) = (env, &type_name) {
                env.method_bindings
                    .iter()
                    .filter(|(key, _)| key.starts_with(&format!("{}.", name)))
                    .map(|(key, fn_type)| {
                        let method_name = key.split('.').next_back().unwrap_or(key).to_string();
                        (method_name, fn_type.clone())
                    })
                    .collect()
            } else {
                Vec::new()
            };

        let mut missing_fields = Vec::new();
        let mut mismatched_fields = Vec::new();

        for (field_name, constraint_fn) in constraint_fields {
            let type_fn = type_fn_fields.iter().find(|(name, _)| name == &field_name);
            let method_fn = if type_fn.is_none() {
                method_bindings.iter().find(|(name, _)| name == &field_name)
            } else {
                None
            };

            match (type_fn, method_fn) {
                (Some((_, found_fn)), _) => {
                    if !Self::fn_signatures_compatible(found_fn, constraint_fn) {
                        mismatched_fields.push((
                            field_name,
                            constraint_fn.type_name(),
                            found_fn.type_name(),
                        ));
                    }
                }
                (_, Some((_, found_fn))) => {
                    if !Self::fn_signatures_compatible(found_fn, constraint_fn) {
                        mismatched_fields.push((
                            field_name,
                            constraint_fn.type_name(),
                            found_fn.type_name(),
                        ));
                    }
                }
                (None, None) => {
                    missing_fields.push(field_name);
                }
            }
        }

        if !missing_fields.is_empty() || !mismatched_fields.is_empty() {
            let constraint_name = constraint.type_name();
            let type_name = ty.type_name();

            let reason = if !missing_fields.is_empty() && !mismatched_fields.is_empty() {
                format!(
                    "missing methods: {:?}, methods with incompatible signatures: {:?}",
                    missing_fields,
                    mismatched_fields
                        .iter()
                        .map(|(n, _, _)| n)
                        .collect::<Vec<_>>()
                )
            } else if !missing_fields.is_empty() {
                format!("missing methods: {:?}", missing_fields)
            } else {
                format!(
                    "methods with incompatible signatures: {:?}",
                    mismatched_fields
                        .iter()
                        .map(|(n, e, f)| format!("{} (expected {}, found {})", n, e, f))
                        .collect::<Vec<_>>()
                )
            };

            return Err(ConstraintCheckError {
                type_name,
                constraint_name,
                reason,
                span: Span::default(),
            });
        }

        Ok(())
    }

    fn fn_signatures_compatible(
        found_fn: &MonoType,
        constraint_fn: &MonoType,
    ) -> bool {
        match (found_fn, constraint_fn) {
            (
                MonoType::Fn {
                    params: found_params,
                    return_type: found_return,
                    ..
                },
                MonoType::Fn {
                    params: constraint_params,
                    return_type: constraint_return,
                    ..
                },
            ) => {
                if found_return != constraint_return {
                    return false;
                }
                if found_params.len() == constraint_params.len() {
                    found_params == constraint_params
                } else if found_params.len() == constraint_params.len() + 1 {
                    &found_params[1..] == constraint_params
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// 验证 const 泛型参数
///
/// 遍历 const_binders 和 const_args，验证：
/// 1. 每个 const arg 必须是 MonoType::Literal（编译期已知值）
/// 2. 字面量的 ConstValue 类型匹配 ConstKind（如 Int 参数不能传 true）
pub fn validate_const_args(
    const_binders: &[ConstVarDef],
    const_args: &[MonoType],
) -> Result<(), Diagnostic> {
    use crate::util::diagnostic::ErrorCodeDefinition;

    for (binder, arg) in const_binders.iter().zip(const_args.iter()) {
        match arg {
            MonoType::Literal { value, .. } => {
                if !binder.kind.matches(value) {
                    return Err(ErrorCodeDefinition::type_mismatch(
                        binder.kind.type_name(),
                        &value.to_string(),
                    )
                    .build());
                }
            }
            _ => {
                return Err(ErrorCodeDefinition::type_mismatch(
                    "编译期字面量",
                    &format!("{:?}", arg),
                )
                .build());
            }
        }
    }
    Ok(())
}
