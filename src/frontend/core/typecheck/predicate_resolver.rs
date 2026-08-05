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

/// 谓词/证明函数用法非法（结构化，供诊断选用错误码，#263）
///
/// 不携带自由文本——诊断文案由 i18n 模板生成，避免中英混排
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateResolveError {
    /// 实参个数与谓词参数不匹配（E1093）
    ArityMismatch { expected: usize, found: usize },
    /// 实参形态不可转换为编译期常量表达式（E1092）
    ArgNotConst,
}

/// 编译期谓词解析器
pub struct PredicateResolver;

impl PredicateResolver {
    /// 尝试将类型应用正格化为精化类型（三值结果，#263）
    ///
    /// - `None`：`predicate_name` 不是已注册的编译期谓词（调用方继续其他解析路径）
    /// - `Some(Ok(refined))`：正格化成功
    ///   - Positive(5)  → Refined { base: Int, constraint: Gt(Lit(Int(5)), Lit(Int(0))) }
    ///   - Positive(b)  → Refined { base: Int, constraint: Gt(NamedVar("b"), Lit(Int(0))) }
    /// - `Some(Err(err))`：是已注册谓词但用法非法（参数个数/实参形态）——
    ///   调用方必须发诊断，绝不静默放行（#263：精化约束不得静默丢弃）
    pub fn try_resolve(
        env: &TypeEnvironment,
        predicate_name: &str,
        args: &[MonoType],
    ) -> Option<Result<MonoType, PredicateResolveError>> {
        // 1. 查找谓词定义
        let def = env.predicate_defs.get(predicate_name)?;

        // 2. 阶段 1 谓词为单参数：个数不匹配是非法用法（#263）
        if args.len() != 1 {
            return Some(Err(PredicateResolveError::ArityMismatch {
                expected: 1,
                found: args.len(),
            }));
        }
        let arg = &args[0];

        // 3. 将实参转换为 ConstExpr（用于代入约束体）
        let arg_expr = match Self::mono_type_to_const_expr(arg) {
            Some(expr) => expr,
            // #263：已注册谓词但实参形态不可转换——不得与「不是谓词」混淆
            None => return Some(Err(PredicateResolveError::ArgNotConst)),
        };

        // 4. 代入实参到约束体模板
        let mut bindings: HashMap<String, ConstExpr> = HashMap::new();
        bindings.insert(def.param_name.clone(), arg_expr);
        let constraint = Self::substitute_in_const_expr(&def.constraint, &bindings);

        // 5. 构建 Refined 类型
        Some(Ok(MonoType::Refined {
            base: Box::new(def.param_type.clone()),
            constraint,
        }))
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
