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

// 依赖倒置：L3 实现 L2 定义的判定接口（TypeEnvProbe），读的是接口契约而不是
// parser 的行为；方向仍是「L3 实现者 → L2 接口」，不构成 L2→L3 反向依赖。
// reason: 接口契约属数据面（同 parser::ast / lexer::tokens 的豁免口径），非 parser 行为
use crate::frontend::core::parser::probe::{NameKind, TypeEnvProbe};
use crate::frontend::core::types::MonoType;

/// 内置谓词名（语言级封闭集，RFC-027 §6.9）。
///
/// 目前只有 `Terminates`（终止测度）。其余谓词无源码声明就不是谓词——
/// 用户声明的「返回 `Type` 的函数」由类型环境另行判定，不进本表。
pub const BUILTIN_PREDICATES: &[&str] = &["Terminates"];

/// 该名字是否为**内置谓词**（无源码声明的语言内置谓词）。
pub fn is_builtin_predicate_name(name: &str) -> bool {
    BUILTIN_PREDICATES.contains(&name)
}

/// 该名字是否为**约束名**（运算符接口，见 `operator_interfaces::SPECS`）。
///
/// 窄入口：调用方（parser 的探针实现、IR 生成的约束形参判定）只问「是不是
/// 约束名」，不直接摸运算符接口表——表是 RFC-011b 的内部表示，不是对外判据。
pub fn is_constraint_name(name: &str) -> bool {
    super::operator_interfaces::spec(name).is_some()
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

    /// 本实现以 `MonoType::from_builtin_name` 为内置名权威表，故能给出
    /// [`NameKind::Builtin`]——const 参数判定的双知识（内置名 ∩ const 可接受集合）
    /// 在全强度生效。
    fn knows_builtin_types(&self) -> bool {
        true
    }
}
