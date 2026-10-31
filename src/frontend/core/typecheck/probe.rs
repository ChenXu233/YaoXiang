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
