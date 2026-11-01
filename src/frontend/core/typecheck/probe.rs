//! 类型环境探针的 L3 实现（RFC-039 P6 6.4 / 03 §5.4）。
//!
//! parser（L2）只持有判定的**接口**（`parser::probe::TypeEnvProbe`），本模块给出
//! 以类型环境为权威来源的**实现**：运算符接口表（约束名）、内置谓词判定、内置名
//! 解析表（`MonoType::from_builtin_name`）。依赖方向是 L3 → L2（读 L2 的接口定义），
//! 不是 L2 → L3。
//!
//! 本模块同时是**内置谓词名的单一判定口**：`Terminates` 是语言内置谓词
//! （RFC-027 §6.9，无源码声明），此前在类型检查的多处被逐点硬编码字符串比较；
//! 现在一律经 [`is_builtin_predicate_name`]。
//!
//! 同理，**可作 const 泛型参数的类型名**表（[`CONST_PARAM_TYPES`]）的唯一权威也在
//! 本模块（WBS 6.6.9 从 `parser::ast` 移回）：parser 只经
//! [`TypeEnvProbe::is_const_param_type`](crate::frontend::core::parser::probe::TypeEnvProbe)
//! 提问，不再持有副本（`check-boundary.py` B6 归零）。
//!
//! 本模块还是**「名是不是谓词」声明级判定的判定口**（WBS 3.20.9 / #522）：
//! [`TypeLevelNamespace`] 把谓词名从检查期状态（`predicate_defs` 表 / `env.vars`）
//! 提升为声明面扫描的事实，[`predicate_decl_shape`] 是声明形态匹配的全仓单点
//! （第零遍登记与 namespace scan 共用）。

// 依赖倒置：L3 实现 L2 定义的判定接口（TypeEnvProbe），读的是接口契约而不是
// parser 的行为；方向仍是「L3 实现者 → L2 接口」，不构成 L2→L3 反向依赖。
// reason: 接口契约属数据面（同 parser::ast / lexer::tokens 的豁免口径），非 parser 行为
use crate::frontend::core::parser::probe::{NameKind, TypeEnvProbe};
// 数据面引用（parser::ast 豁免口径，同上）——`TypeLevelNamespace::scan` 的参数类型
use crate::frontend::core::parser::ast::Module;
use crate::frontend::core::types::MonoType;
use std::collections::HashSet;

/// 内置谓词名（语言级封闭集，RFC-027 §6.9）。
///
/// 目前只有 `Terminates`（终止测度）。其余谓词无源码声明就不是谓词——
/// 用户声明的「返回 `Type` 的函数」由类型环境另行判定，不进本表。
pub const BUILTIN_PREDICATES: &[&str] = &["Terminates"];

/// 该名字是否为**内置谓词**（无源码声明的语言内置谓词）。
pub fn is_builtin_predicate_name(name: &str) -> bool {
    BUILTIN_PREDICATES.contains(&name)
}

/// 可作 **const 泛型参数**的类型名（`(N: Int)` 的 `Int`）。
///
/// 语义 = 「内置名表（`MonoType::from_builtin_name`）里可被 const 参数接受的**子集**」。
/// 本表是 L3 的**唯一权威**（WBS 6.6.9 从 `parser::ast` 移回此处）：parser（L2）只经
/// `TypeEnvProbe::is_const_param_type` 提问，不再持有副本。
///
/// 6.4 删掉了 10 个旧拼写（`I8` / `I16` / `I32` / `I64` / `U8` / `U16` / `U32` /
/// `U64` / `F32` / `F64`）：它们是 6.1 已删除的实现内拼写，留在表里会产生**无法 lower**
/// 的 const 参数名（Uint 族在 `MonoType` 里没有变体，位宽缩写也不是内置名）。
/// 规范名（`Int8` …）不进本表——它们自始不在 const 参数的可接受集合里，加入会拓宽
/// 泛型参数登记，属行为变更（`tools/type-tables` 的 T2 表 B 逐名对账守住这一点）。
pub const CONST_PARAM_TYPES: &[&str] = &["Int", "Bool", "Float", "Char", "String"];

/// 该名字是否可作 **const 泛型参数**的类型名（窄入口，探针与 T2 对账共用）。
///
/// 只做表成员判定：**不**在这里再叠「是不是内置名」的二次过滤——表里混进非内置名
/// 必须由对账用例/T2 报红，而不是被静默滤掉。
pub fn is_const_param_type_name(name: &str) -> bool {
    CONST_PARAM_TYPES.contains(&name)
}

/// 该名字是否为**约束名**（运算符接口，见 `operator_interfaces::SPECS`）。
///
/// 窄入口：调用方（parser 的探针实现、IR 生成的约束形参判定）只问「是不是
/// 约束名」，不直接摸运算符接口表——表是 RFC-011b 的内部表示，不是对外判据。
pub fn is_constraint_name(name: &str) -> bool {
    super::operator_interfaces::spec(name).is_some()
}

/// 谓词声明的形态识别产物——**声明形态匹配的全仓单点**
/// （WBS 3.20.9 / #522：第零遍 `collect_predicate_defs` 与
/// [`TypeLevelNamespace::scan`] 共用，shape matching 不许有第二份）。
///
/// 两种声明形态（与第零遍登记判据同源）：
/// - `TypeDefinition`：`= { x > 0 }` 被解析成类型体，约束在
///   `Type::Struct` 的 `TypeBodyItem::Expr(Type::ConstExpr)` 里（主形态）
/// - `Assign`：函数体形态，注解为 `Type::Fn` 且返回 `Type`（`MetaType`
///   与 `Name { "Type" }` 两拼写同义）
pub(crate) struct PredicateDeclShape<'a> {
    /// 被绑定的谓词名（`TypeDefinition` 的类型名 / `Assign` 的左值名）。
    pub name: &'a str,
    /// 声明注解（形参从 `Type::Fn` 链拍平；第零遍登记用，scan 不消费）。
    pub type_annotation: Option<&'a crate::frontend::core::parser::ast::Type>,
    /// 约束体（常量表达式形态；第零遍登记用，scan 不消费）。
    pub constraint: crate::frontend::core::types::const_data::ConstExpr,
}

/// 识别单条语句是不是**谓词声明**，是则给出其形态产物。
///
/// 只做形态匹配、无环境副作用；第零遍在此之上叠加「仅单参数形态登记」
/// 的收窄（`PredicateResolver::try_resolve` 的阶段 1 模型），scan 则对
/// 全部命中形态收名——收窄差异是刻意的，见 `collect_predicate_defs`。
pub(crate) fn predicate_decl_shape(
    stmt: &crate::frontend::core::parser::ast::Stmt
) -> Option<PredicateDeclShape<'_>> {
    use crate::frontend::core::parser::ast::{Expr, StmtKind, Type, TypeBodyItem};
    use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;

    match &stmt.kind {
        StmtKind::TypeDefinition {
            name,
            type_annotation,
            definition,
            ..
        } => {
            let Type::Struct { body } = definition else {
                return None;
            };
            let expr = body.iter().find_map(|item| match item {
                TypeBodyItem::Expr(Type::ConstExpr(e)) => Some(e.as_ref()),
                _ => None,
            })?;
            let c = convert_expr_to_const_expr(expr)?;
            Some(PredicateDeclShape {
                name,
                type_annotation: type_annotation.as_ref(),
                constraint: c,
            })
        }
        StmtKind::Assign {
            target,
            type_annotation: fn_annotation,
            value: Some(v),
            ..
        } => {
            let Some(Type::Fn { return_type, .. }) = fn_annotation else {
                return None;
            };
            let Expr::Var(name, _) = target.as_ref() else {
                return None;
            };
            // 返回 Type 才是谓词（`MetaType` 与 `Name{Type}` 两形态同义）
            let returns_type = matches!(return_type.as_ref(), Type::MetaType { .. })
                || matches!(return_type.as_ref(), Type::Name { name: n, .. } if n == "Type");
            if !returns_type {
                return None;
            }
            let body = super::checker::extract_fn_body(Some(v.as_ref()))?;
            let expr = body.stmts.iter().find_map(|s| match &s.kind {
                StmtKind::Expr(e) => Some(e.as_ref()),
                StmtKind::Return(Some(e)) => Some(e.as_ref()),
                _ => None,
            })?;
            let c = convert_expr_to_const_expr(expr)?;
            Some(PredicateDeclShape {
                name,
                type_annotation: fn_annotation.as_ref(),
                constraint: c,
            })
        }
        _ => None,
    }
}

/// 类型级命名空间——「名是不是谓词」的**声明级事实**
/// （WBS 3.20.9 / #522：判据从检查期状态提升到声明面）。
///
/// #522 的病灶：`collect_signatures`（Registry 阶段）不执行第零遍
/// `collect_predicate_defs` 且导入未并，检查期三臂判定（内置名 /
/// `predicate_defs` 表 / `env.vars` 的 MetaType 返回）对谓词判假 ⇒
/// 谓词应用实参位被误计 const 用途 ⇒ E1012 假拒绝。修复 = 判据改为
/// 对全部可见单元做**声明面扫描**的并集，任何阶段、任何调用形态都
/// 同源可见（过包含方向安全：scan 全集只会多认谓词名，多认的后果是
/// 「该排除时排除」，不会把类型引用误排除）。
///
/// 由 [`TypeLevelNamespace::scan`] 构造；`TypeChecker` 在
/// `check_module` / `check_module_collect_all` / `collect_signatures`
/// 入口经显式参数注入（无默认空集的静默路径——漏接即编译错误）。
#[derive(Clone, Debug, Default)]
pub struct TypeLevelNamespace {
    /// 内置谓词名 ∪ 各单元谓词声明名的并集。
    pub predicates: HashSet<String>,
}

impl TypeLevelNamespace {
    /// 扫描全部可见单元的谓词声明名，与内置谓词名并成命名空间。
    ///
    /// 纯函数：只读 AST，不触环境。多单元调用方（多文件项目 / Registry
    /// 面）传全部单元；单文件入口传自身模块。
    pub fn scan<'a>(units: impl IntoIterator<Item = &'a Module>) -> Self {
        let mut predicates: HashSet<String> = BUILTIN_PREDICATES
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        for module in units {
            for stmt in &module.items {
                if let Some(decl) = predicate_decl_shape(stmt) {
                    predicates.insert(decl.name.to_string());
                }
            }
        }
        Self { predicates }
    }

    /// 单单元便捷形态（= `scan(std::iter::once(module))`）。
    pub fn scan_one(module: &Module) -> Self {
        Self::scan(std::iter::once(module))
    }
}

/// 以类型环境为权威来源的探针（L3 实现）。
///
/// IR 生成、语义 token 收集等需要「名字定种」的生产路径传它；parser 的单元
/// 测试传 `parser::probe::NullProbe`。
pub struct SemanticProbe;

impl TypeEnvProbe for SemanticProbe {
    fn is_constraint(
        &self,
        name: &str,
    ) -> bool {
        is_constraint_name(name)
    }

    fn is_predicate(
        &self,
        name: &str,
    ) -> bool {
        is_builtin_predicate_name(name)
    }

    fn kind_of(
        &self,
        name: &str,
    ) -> NameKind {
        if MonoType::from_builtin_name(name).is_some() {
            NameKind::Builtin
        } else if is_constraint_name(name) {
            NameKind::Constraint
        } else if is_builtin_predicate_name(name) {
            NameKind::Predicate
        } else {
            NameKind::UserType
        }
    }

    /// const 参数可接受的类型名：本实现是这张表的唯一权威（见 [`CONST_PARAM_TYPES`]）。
    fn is_const_param_type(
        &self,
        name: &str,
    ) -> bool {
        is_const_param_type_name(name)
    }
}
