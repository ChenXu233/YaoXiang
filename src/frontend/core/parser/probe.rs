//! 类型环境探针：parser 数据流的去类型依赖（RFC-039 P6 6.4 / 03 §5.4）。
//!
//! parser 需要回答三类「这个名字是什么」的问题：**约束名**（运算符接口，
//! `(T: Add)` 的 `Add`）、**编译期谓词名**（`Terminates(n)` 的 `Terminates`）、
//! **内置类型名**（`Int` / `Bool` / …）。这些事实的权威来源都在类型环境
//! （L3：运算符接口表、谓词判定、`MonoType::from_builtin_name`），而 parser
//! 属 L2，不得反向依赖 L3（`scripts/ci/check-boundary.py` 的 B2）。故本模块
//! 只定义**判定的接口**，实现由调用方注入（依赖倒置）：
//!
//! - [`NullProbe`]：全 false，**无类型环境**入口与 parser 单元测试的基线；
//! - L3 的语义探针 `SemanticProbe`（`typecheck::probe`）：以运算符接口表 / 内置谓词
//!   判定 / 内置名表 / const 参数名表为准——9 个生产入口一律注入它（WBS 6.6.9）。
//!
//! **L2 不再持有任何名字表**（WBS 6.6.9）：6.4 曾在 parser 里留过两张封闭集
//! （`CONSTRAINT_NAMES` / `PREDICATE_NAMES`）与一张 const 参数名表
//! （`ast::CONST_PARAM_TYPES`），当作「默认入口拿不到 L3 环境」的兜底。三者都是
//! L3 知识的**第二份副本**（`check-boundary.py` B6 的两个命中点），已全部删除：
//! 默认入口只带 [`NullProbe`]（= parser 不出类型知识），生产入口显式注入语义探针。
//!
//! 注入点是 [`ParserState`](super::ParserState) 的 `probe` 字段：parser 内所有
//! 「名字定种」判定一律经 `state.probe()`，parser 自身不再持有类型/谓词名字面量。

/// 名字的语义定种（03 §5.3 的 `NameKind`）。
///
/// **与 03 §5.3 的差异（本批刻意不做的部分）**：03 §5.3 的目标形态是给
/// `Type::Name` 增加 `kind` 字段、在 `Type::Name` 构造点一次定死。本批
/// （WBS 6.4）只引入**定种枚举 + 探针查询**，不动 `Type::Name` 的结构：
/// 字段化会牵动全部生产/测试构造点，且要求反向桥 `mono_to_ast_type` 保真地
/// 回写 `kind`，IR 快照与语料基线随之再生——与本批 C3 判据（零漂移）冲突。
/// 当前定种信息只有一个消费者（`extract_generic_param_names` 的 const 参数
/// 判定），字段化留待出现第二个消费者时再评估。
///
/// **`TypeVar` 未设变体**：类型变量的身份由**声明位**结构性决定（`(T: Type)`
/// 的 MetaType 标注 / 类型级函数标注），不由名字决定；不设该变体以免出现
/// 无消费者的死变体（6.3 的变体纪律）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    /// 内置类型名：交给 `MonoType::from_builtin_name` 解析。
    Builtin,
    /// 约束名（运算符接口 / 约束）。
    Constraint,
    /// 编译期谓词。
    Predicate,
    /// 用户声明的类型名，或无法定种。
    UserType,
}

/// 类型环境探针：parser 向 L3 提问的唯一接口。
///
/// 三个方法都带默认实现（全 false / `UserType`），实现者只覆写自己知道的那部分。
pub trait TypeEnvProbe {
    /// 该名字是否为**约束名**（运算符接口，`(T: Add)` 的 `Add`）。
    ///
    /// 约束形参在类型位、不占运行时参数位；判错会让约束形参落成值参数。
    fn is_constraint(
        &self,
        _name: &str,
    ) -> bool {
        false
    }

    /// 该名字是否为**编译期谓词**（`Terminates`，或返回 `Type` 的函数声明）。
    ///
    /// 谓词应用的**实参位是值**（`Terminates(b)` 的 `b`），不是类型引用；
    /// 判错会把形参 `b` 误当 const 泛型参数从 lambda 形参里剔除（RFC-027a）。
    fn is_predicate(
        &self,
        _name: &str,
    ) -> bool {
        false
    }

    /// 名字定种（[`NameKind`]）。默认不定种（[`NameKind::UserType`]）。
    ///
    /// 内置名的权威表（`from_builtin_name`）在 L3，故 L2 的实现不覆写本方法。
    fn kind_of(
        &self,
        _name: &str,
    ) -> NameKind {
        NameKind::UserType
    }

    /// 该名字是否为**可作 const 泛型参数**的类型名（`(N: Int)` 的 `Int`）。
    ///
    /// 权威集合在 L3（`typecheck::probe::CONST_PARAM_TYPES`，语义 =「内置名表里
    /// 可被 const 参数接受的子集」）：parser 只提问、不建表。默认 `false` =
    /// 「本探针不提供这条知识」，调用方不能据此反推「该名字一定不是」——它只意味着
    /// parser 不会把它登记成 const 泛型参数。
    ///
    /// **历史口径（已删除）**：6.4 修复轮曾在 L2 留一张 `CONST_PARAM_TYPES` 单表，
    /// 供不持有内置名表的探针回退（避免默认入口漏判）。那张表是 L3 知识的**第二份
    /// 副本**（`check-boundary.py` B6 的命中点之一），WBS 6.6.9 已把它移回 L3、
    /// 回退分支一并删除——parser 的内置封闭集清零。
    fn is_const_param_type(
        &self,
        _name: &str,
    ) -> bool {
        false
    }
}

/// 全 false 探针：单元测试与「无类型环境」入口的基线。
pub struct NullProbe;

impl TypeEnvProbe for NullProbe {}
