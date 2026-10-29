//! 类型环境探针：parser 数据流的去类型依赖（RFC-039 P6 6.4 / 03 §5.4）。
//!
//! parser 需要回答三类「这个名字是什么」的问题：**约束名**（运算符接口，
//! `(T: Add)` 的 `Add`）、**编译期谓词名**（`Terminates(n)` 的 `Terminates`）、
//! **内置类型名**（`Int` / `Bool` / …）。这些事实的权威来源都在类型环境
//! （L3：运算符接口表、谓词判定、`MonoType::from_builtin_name`），而 parser
//! 属 L2，不得反向依赖 L3（`scripts/ci/check-boundary.py` 的 B2）。故本模块
//! 只定义**判定的接口**，实现由调用方注入（依赖倒置）：
//!
//! - [`NullProbe`]：全 false，单元测试与「无类型环境」入口的基线；
//! - [`BuiltinProbe`]：编译器内置封闭集的镜像（见其文档与「残留」说明）；
//! - L3 的语义探针 `SemanticProbe`：以运算符接口表 / 谓词判定 / 内置名表为准。
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
    /// 内置名的权威表（`from_builtin_name`）在 L3，故 L2 的默认实现不覆写本方法。
    fn kind_of(
        &self,
        _name: &str,
    ) -> NameKind {
        NameKind::UserType
    }
}

/// 全 false 探针：单元测试与「无类型环境」入口的基线。
pub struct NullProbe;

impl TypeEnvProbe for NullProbe {}

/// 编译器**内置封闭集**探针：运算符接口名与内置谓词名的语言级常量表。
///
/// **为什么 L2 还留一份**：`parse()` 有 9 个生产入口（driver 的三个解析臂、
/// 前端编排的两处逐文件解析、前端门面、`validate`、库入口、LSP 补全、
/// `src/std/` 的源码解析），其中 `src/std/` 属本批硬验收的**零差异**面，
/// 本批无法把它们全部改成注入形态。故默认探针保留这张封闭集表，判定结果与
/// 6.4 之前**逐字一致**；一旦 L1 在各入口注入 L3 语义探针，本类型即可退化为
/// 测试基线并删除。
///
/// **不是第二份权威表**：`CONSTRAINT_NAMES` 与 L3 运算符接口表的 7 名逐名
/// 对账、`PREDICATE_NAMES` 与 L3 的内置谓词判定双向对账，由 L3 侧的对账
/// 用例守着（漂移即测试红）。
pub struct BuiltinProbe;

/// 运算符接口名（约束名）封闭集，与 L3 运算符接口表逐名对账。
pub const CONSTRAINT_NAMES: &[&str] = &[
    "Add", "Subtract", "Multiply", "Divide", "Modulo", "Equal", "Index",
];

/// 内置谓词名封闭集，与 L3 的内置谓词判定双向对账。
pub const PREDICATE_NAMES: &[&str] = &["Terminates"];

impl TypeEnvProbe for BuiltinProbe {
    fn is_constraint(
        &self,
        name: &str,
    ) -> bool {
        CONSTRAINT_NAMES.contains(&name)
    }

    fn is_predicate(
        &self,
        name: &str,
    ) -> bool {
        PREDICATE_NAMES.contains(&name)
    }

    /// 默认探针只能按自己的封闭集定种（内置类型名表在 L3，不在此处）。
    fn kind_of(
        &self,
        name: &str,
    ) -> NameKind {
        if self.is_predicate(name) {
            NameKind::Predicate
        } else if self.is_constraint(name) {
            NameKind::Constraint
        } else {
            NameKind::UserType
        }
    }
}
