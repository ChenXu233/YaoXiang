//! 编译期谓词正格化
//!
//! 职责：识别 TypeRef("Positive")(args) 形式的类型应用，正格化为 Refined 类型。
//! 不参与求值，不参与证明——只做"名称 → 内部表示"的翻译。

use crate::ast::Module;
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::types::const_data::ConstExpr;
use crate::frontend::core::sema::TypeEnvironment;
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

impl PredicateDef {
    /// 谓词的 **proof-fn 形态**：`Fn { params: [基类型…], return_type: MetaType }`。
    ///
    /// 消费面是「剥精化取基类型」的判据源——`proof_fn_bases`（由 `env.vars`
    /// 里 MetaType 返回函数构建）与 `lookup_proof_fn_base_type`。本地面的
    /// 登记点在 `add_type_definition`（RFC-027 Phase 2.5 分支）；导入面
    /// （#524 甲案）在 `register_use_export` 据本构造登记同构变量——
    /// 两处共用同一形态构造，不各写一份。
    pub fn proof_fn_type(param_types: &[MonoType]) -> MonoType {
        MonoType::Fn {
            params: param_types.to_vec(),
            return_type: Box::new(MonoType::MetaType {
                universe_level: crate::frontend::core::types::mono::UniverseLevel::type0(),
                type_params: vec![],
            }),
        }
    }

    /// [`PredicateDef::proof_fn_type`] 的**提取器**（构造/解构同栖，#527）：
    /// 形态是 `Fn` 且返回 `MetaType`（RFC-027 Phase 2.5 登记形态）时给出
    /// 形参类型清单（声明序），否则 `None`。
    ///
    /// 消费方是 Registry 导出提取面（`orchestrator::extract_module_info` 与
    /// 嵌入 std `embedded_std_module_info` 的 TypeDefinition 臂）：从定义方
    /// `env.vars` 的**已登记形态**读出形参清单随导出携带——不在导出面
    /// 从 AST 重新推导（那会是 `add_type_definition` 形参收集判据的第二份）。
    /// 返回位只认 `MetaType`：本提取器是 Phase 2.5 登记形态的逆运算，该
    /// 形态由 `proof_fn_type` 构造、返回位恒为 `MetaType`（`TypeRef("Type")`
    /// 的放宽形态是 `lookup_proof_fn_base_type` 的消费面判定，不在此处）。
    pub fn proof_fn_params(fn_ty: &MonoType) -> Option<Vec<MonoType>> {
        match fn_ty {
            MonoType::Fn {
                params,
                return_type,
            } if matches!(return_type.as_ref(), MonoType::MetaType { .. }) => Some(params.clone()),
            _ => None,
        }
    }
}

/// 从模块 AST 构建编译期谓词定义表——**def 构建判据的全仓单源**（#524）。
///
/// 两个委托方：
/// - 第零遍（`TypeChecker::collect_predicate_defs`）：把结果并入本单元
///   `env.predicate_defs`；
/// - Registry 导出提取（`orchestrator::extract_module_info`）：把结果按名
///   附到对应 `Export` 上，随导出面携带给导入方（甲案：Export 携带 def）。
///
/// 判据（自第零遍原样提纯，不得在委托方出现第二份）：
///
/// - 声明形态识别复用 `probe::predicate_decl_shape`（WBS 3.20.9 / #522 起
///   shape matching 全仓单点）；本函数在其上叠加**登记侧收窄**：
/// - 只登记单参数形态：`PredicateResolver::try_resolve` 的阶段 1 模型即
///   单参数，登记多参数谓词会使其调用一律撞 ArityMismatch（E1093）——
///   多参数形态保持走证明函数路径（现行为）。约束体不可转为常量表达式的
///   声明不登记也不报错（它只是普通函数）。
/// - 形参无类型标注时基类型取 `Int(64)`（第零遍既有口径）。
pub fn collect_predicate_defs(module: &Module) -> HashMap<String, PredicateDef> {
    let mut defs = HashMap::new();
    for stmt in &module.items {
        let Some(decl) = crate::frontend::core::sema::probe::predicate_decl_shape(stmt) else {
            continue;
        };
        // 形参名与类型同栖在 `Type::Fn.params`（6.5.2）
        let params = decl
            .type_annotation
            .map(crate::ast::Type::fn_params_flat)
            .unwrap_or_default();
        let [param] = params.as_slice() else {
            continue;
        };
        let param_type = param
            .ty
            .as_ref()
            .map(|t| MonoType::from(t.clone()))
            .unwrap_or(MonoType::Int(64));
        defs.insert(
            decl.name.to_string(),
            PredicateDef {
                param_name: param.name.clone(),
                param_type,
                constraint: decl.constraint,
            },
        );
    }
    defs
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
