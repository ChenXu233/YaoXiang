//! 编译期谓词正格化
//!
//! 职责：识别 TypeRef("Positive")(args) 形式的类型应用，正格化为 Refined 类型。
//! 不参与求值，不参与证明——只做"名称 → 内部表示"的翻译。

use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::types::const_data::ConstExpr;
use crate::frontend::core::typecheck::TypeEnvironment;
use std::collections::HashMap;

/// 编译期谓词定义模板
///
/// 例：Positive: (x: Int) -> Type = { x > 0 }
/// → PredicateDef { param_name: "x", param_type: Int(64), constraint: BinOp { Gt, NamedVar("x"), Lit(Int(0)) } }
#[derive(Debug, Clone)]
pub struct PredicateDef {
    /// 参数名
    pub param_name: String,
    /// 参数类型（即精化类型的基类型）
    pub param_type: MonoType,
    /// 约束体模板（含参数名引用，使用时做替换）
    pub constraint: ConstExpr,
}

/// 编译期谓词解析器
pub struct PredicateResolver;

impl PredicateResolver {
    /// 尝试将类型应用正格化为精化类型
    ///
    /// Positive(5)  → Refined { base: Int, constraint: Gt(Lit(Int(5)), Lit(Int(0))) }
    /// Positive(b)  → Refined { base: Int, constraint: Gt(NamedVar("b"), Lit(Int(0))) }
    /// 如果不是编译期谓词 → None
    pub fn try_resolve(
        env: &TypeEnvironment,
        predicate_name: &str,
        args: &[MonoType],
    ) -> Option<MonoType> {
        // 1. 查找谓词定义
        let def = env.predicate_defs.get(predicate_name)?;

        // 2. 提取实参（阶段 1 只支持单参数谓词）
        let arg = args.first()?;

        // 3. 将实参转换为 ConstExpr（用于代入约束体）
        let arg_expr = Self::mono_type_to_const_expr(arg)?;

        // 4. 代入实参到约束体模板
        let mut bindings: HashMap<String, ConstExpr> = HashMap::new();
        bindings.insert(def.param_name.clone(), arg_expr);
        let constraint = Self::substitute_in_const_expr(&def.constraint, &bindings);

        // 5. 构建 Refined 类型
        Some(MonoType::Refined {
            base: Box::new(def.param_type.clone()),
            constraint,
        })
    }

    /// 将 MonoType 转为 ConstExpr（用于实参到约束体的代入）
    fn mono_type_to_const_expr(ty: &MonoType) -> Option<ConstExpr> {
        match ty {
            // 字面量值：如 Positive(5) 中的 5
            MonoType::Literal { value, .. } => Some(ConstExpr::Lit(value.clone())),
            // 变量引用：如 Positive(b) 中的 b（作为命名变量）
            MonoType::TypeRef(name) => Some(ConstExpr::NamedVar(name.clone())),
            // 递归处理 Generic 中的参数（如 Positive(Generic { name: "x", args: [...] })
            MonoType::Generic { name: _name, args } if args.len() == 1 => {
                Self::mono_type_to_const_expr(&args[0])
            }
            _ => None,
        }
    }

    /// 在 ConstExpr 中做变量替换
    fn substitute_in_const_expr(
        expr: &ConstExpr,
        bindings: &HashMap<String, ConstExpr>,
    ) -> ConstExpr {
        match expr {
            ConstExpr::NamedVar(name) => {
                bindings.get(name).cloned().unwrap_or_else(|| expr.clone())
            }
            ConstExpr::BinOp { op, left, right } => ConstExpr::BinOp {
                op: *op,
                left: Box::new(Self::substitute_in_const_expr(left, bindings)),
                right: Box::new(Self::substitute_in_const_expr(right, bindings)),
            },
            ConstExpr::UnOp { op, expr: inner } => ConstExpr::UnOp {
                op: *op,
                expr: Box::new(Self::substitute_in_const_expr(inner, bindings)),
            },
            ConstExpr::If {
                condition,
                then_branch,
                else_branch,
            } => ConstExpr::If {
                condition: Box::new(Self::substitute_in_const_expr(condition, bindings)),
                then_branch: Box::new(Self::substitute_in_const_expr(then_branch, bindings)),
                else_branch: Box::new(Self::substitute_in_const_expr(else_branch, bindings)),
            },
            ConstExpr::Range { start, end } => ConstExpr::Range {
                start: Box::new(Self::substitute_in_const_expr(start, bindings)),
                end: Box::new(Self::substitute_in_const_expr(end, bindings)),
            },
            // Call 递归处理参数
            ConstExpr::Call { func, args } => ConstExpr::Call {
                func: func.clone(),
                args: args
                    .iter()
                    .map(|a| Self::substitute_in_const_expr(a, bindings))
                    .collect(),
            },
            // Lit, Var(ConstVar) 不变
            _ => expr.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    fn make_positive_def() -> PredicateDef {
        PredicateDef {
            param_name: "x".into(),
            param_type: MonoType::Int(64),
            constraint: ConstExpr::BinOp {
                op: BinOp::Gt,
                left: Box::new(ConstExpr::NamedVar("x".into())),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
            },
        }
    }

    #[test]
    fn test_resolve_positive_with_literal() {
        let mut env = TypeEnvironment::new();
        env.predicate_defs
            .insert("Positive".into(), make_positive_def());

        let result = PredicateResolver::try_resolve(
            &env,
            "Positive",
            &[MonoType::Literal {
                name: "5".into(),
                base_type: Box::new(MonoType::Int(64)),
                value: ConstValue::Int(5),
            }],
        );

        assert!(result.is_some());
        match result.unwrap() {
            MonoType::Refined { base, constraint } => {
                assert_eq!(*base, MonoType::Int(64));
                // 约束应是 5 > 0
                match constraint {
                    ConstExpr::BinOp { op, left, right } => {
                        assert_eq!(op, BinOp::Gt);
                        assert_eq!(*left, ConstExpr::Lit(ConstValue::Int(5)));
                        assert_eq!(*right, ConstExpr::Lit(ConstValue::Int(0)));
                    }
                    _ => panic!("Expected BinOp"),
                }
            }
            _ => panic!("Expected Refined"),
        }
    }

    #[test]
    fn test_resolve_positive_with_variable() {
        let mut env = TypeEnvironment::new();
        env.predicate_defs
            .insert("Positive".into(), make_positive_def());

        let result = PredicateResolver::try_resolve(
            &env,
            "Positive",
            &[MonoType::TypeRef("b".into())],
        );

        assert!(result.is_some());
        match result.unwrap() {
            MonoType::Refined { base, constraint } => {
                assert_eq!(*base, MonoType::Int(64));
                match constraint {
                    ConstExpr::BinOp { op, left, .. } => {
                        assert_eq!(op, BinOp::Gt);
                        assert_eq!(*left, ConstExpr::NamedVar("b".into()));
                    }
                    _ => panic!("Expected BinOp"),
                }
            }
            _ => panic!("Expected Refined"),
        }
    }

    #[test]
    fn test_resolve_unknown_predicate_returns_none() {
        let env = TypeEnvironment::new();
        let result = PredicateResolver::try_resolve(
            &env,
            "UnknownPredicate",
            &[MonoType::Int(64)],
        );
        assert!(result.is_none());
    }
}
