//! 依赖记录 Σ 的记录模型（RFC-027 §14 / SPEC §8.3）。
//!
//! 声明期一遍（`TypeChecker::classify_record`）把记录字段分成 data / evidence
//! 两类，落成这里的 `RecordModel`：字段量词（`Some(Quantity::Zero)` = witness）、
//! 构造点义务（`FieldObligation`）、导出槽（`EvidenceSlot`）与声明期索引
//! （`field_path`）。**唯一写入点**是声明期一遍；消费面只有三处——声明期自己、
//! 构造义务（refined walk）、IR 生成（量词传播，经 `TypeCheckResult::record_models`）。
//!
//! 本模块只放**数据与纯函数**；需要 `TypeChecker` 环境的分类逻辑在同级
//! `checker/record_model.rs`。

use crate::frontend::core::types::const_data::ConstExpr;
use crate::frontend::core::types::mono::{MonoType, StructFieldInfo};

/// 一条 evidence 字段要证明的命题。
#[derive(Debug, Clone)]
pub struct Proposition {
    /// 命题的主体类型（`Refined::base`；`Assert` 取 `Void`）
    pub base: MonoType,
    /// 约束项（谓词体代入实参后的`ConstExpr`，或证明函数的不透明 `Call`）
    pub constraint: ConstExpr,
}

/// 记录类型参数的 kind（与 `ast::GenericParamKind` 同域，只保留分类需要的部分）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordTypeParamKind {
    /// 类型参数（`T: Type`）
    Type,
    /// 编译期值参数（`N: Int`，参与 const 求值）
    Const,
    /// 类型族/构造器参数（`P: (T: Type) -> Type`）
    Family {
        /// 已声明的元数
        arity: usize,
    },
}

/// 记录类型参数（声明序）。
#[derive(Debug, Clone)]
pub struct RecordTypeParam {
    pub name: String,
    pub kind: RecordTypeParamKind,
}

/// 一条 evidence 字段的构造义务（RFC-027 §14.2 的子集）。
#[derive(Debug, Clone)]
pub struct FieldObligation {
    /// 记录名
    pub record: String,
    /// 字段声明序
    pub field: usize,
    /// 构造点要判定的命题（已按声明代换位记录依赖）
    pub proposition: Proposition,
    /// 命题引用的严格前序**数据**字段（声明序）
    pub depends_on: Vec<usize>,
    /// 命题引用的编译期值参数（`type_params` 下标）
    pub param_refs: Vec<usize>,
}

/// 自引用精化（admission）字段：字段是**数据字段**，但构造点要证明自引用命题
/// （RFC-027 §14.1 第 3 行 / §14.2 的 admission 义务）。值类型 = 命题的 `base`。
#[derive(Debug, Clone)]
pub struct AdmissionField {
    /// 字段声明序
    pub field: usize,
    /// 构造点要证明的命题（字段名即自引用位的实参名）
    pub proposition: Proposition,
}

/// evidence 槽（RFC-027 §14.4 `EvidenceSlot` 的 B.2 子集；B.4 再加 ref/hash）。
#[derive(Debug, Clone)]
pub struct EvidenceSlot {
    /// 字段声明序
    pub field: usize,
    /// 量词（恒为 `Some(Quantity::Zero)`；保留字段使导出面与 RFC 同形）
    pub quantity: Option<crate::ast::Quantity>,
    /// 命题；Registry 阶段（`collect_signatures`）为 `None`
    pub proposition: Option<Proposition>,
}

/// 记录的依赖记录 Σ 模型。
#[derive(Debug, Clone, Default)]
pub struct RecordModel {
    /// 记录名
    pub name: String,
    /// 类型参数（声明序）
    pub type_params: Vec<RecordTypeParam>,
    /// 字段（声明序；与 `StructType.fields` 同一表示，`quantity` 已写）
    pub fields: Vec<StructFieldInfo>,
    /// 构造义务（每条 evidence 字段一条，声明序）
    pub obligations: Vec<FieldObligation>,
    /// evidence 槽（与 `obligations` 同长同序）
    pub evidence: Vec<EvidenceSlot>,
    /// 自引用精化字段（admission；数据字段，构造点证明自引用命题）
    pub admission_fields: Vec<AdmissionField>,
    /// 字段名 → 声明序（声明期索引；运行期索引只由 `runtime_field_index` 回答）
    pub field_path: Vec<(String, usize)>,
}

impl RecordModel {
    /// 字段名 → 声明序。
    pub fn field_index(
        &self,
        name: &str,
    ) -> Option<usize> {
        self.field_path
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, index)| *index)
    }

    /// 是否含 evidence 字段（5A 判据与构造义务的入口）。
    pub fn has_evidence(&self) -> bool {
        !self.evidence.is_empty()
    }

    /// 该字段是否是 witness。
    pub fn is_witness(
        &self,
        index: usize,
    ) -> bool {
        self.fields
            .get(index)
            .map(|field| matches!(field.quantity, Some(crate::ast::Quantity::Zero)))
            .unwrap_or(false)
    }

    /// 数据字段的声明序（构造参数与运行期布局的口径）。
    pub fn data_field_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.fields
            .iter()
            .enumerate()
            .filter(|(_, field)| !matches!(field.quantity, Some(crate::ast::Quantity::Zero)))
            .map(|(index, _)| index)
    }
}

/// 声明期拒绝的类别（RFC-027 §14.1 第 4 行 / §14.6 的清单）。
///
/// **码位待 D2 裁决**（契约 §十二）：唯一的码位落点是
/// [`record_rejection_diagnostic`]，裁决后只改那一个函数。
#[derive(Debug, Clone)]
pub enum RecordRejection {
    /// 命题引用后序字段（前向引用）
    ForwardReference { field: String, other: String },
    /// 命题引用另一个 evidence 字段
    EvidenceReference { field: String, other: String },
    /// 把自身当 evidence 实参又与其它字段混用
    SelfMixed { field: String },
    /// evidence 字段带默认值
    DefaultOnEvidence { field: String },
}

/// 声明期拒绝 → 诊断（**唯一码位落点**，D2 裁决后只改这里）。
///
/// 当前用契约 §三 的 fallback 映射（前向 → E1001、引用 evidence → E1114、
/// 自身混用 → E1001、默认值 → E1092）；它们不是 RFC 新码，D2 若裁新增声明期码，
/// 只替换本函数的 `match` 臂，调用点与检测逻辑零改动。
pub fn record_rejection_diagnostic(
    record: &str,
    rejection: &RecordRejection,
    span: crate::util::span::Span,
) -> crate::util::diagnostic::Diagnostic {
    use crate::util::diagnostic::ErrorCodeDefinition;
    let definition = match rejection {
        RecordRejection::ForwardReference { field, .. } => {
            ErrorCodeDefinition::unknown_variable(field)
        }
        RecordRejection::EvidenceReference { other, .. } => {
            ErrorCodeDefinition::witness_field_not_readable(record, other)
        }
        RecordRejection::SelfMixed { field } => ErrorCodeDefinition::unknown_variable(field),
        RecordRejection::DefaultOnEvidence { field } => {
            ErrorCodeDefinition::refined_arg_not_const(field)
        }
    };
    definition.at(span).build()
}

/// 把模型的量词写进类型表示（`add_type_definition` 的 `poly` 与其后的
/// `env.types`/`env.vars`/`generic_type_defs` 都从这一份结果派生）。
///
/// 只对同名 `Struct` 生效；非记录体（或字段数不符）原样返回。
pub fn apply_quantities(
    body: MonoType,
    model: Option<&RecordModel>,
) -> MonoType {
    let Some(model) = model else {
        return body;
    };
    match body {
        MonoType::Struct(mut structure) if structure.name == model.name => {
            if structure.fields.len() == model.fields.len() {
                for (field, classified) in structure.fields.iter_mut().zip(model.fields.iter()) {
                    field.quantity = classified.quantity;
                    // admission：值类型 = 命题的 base（自引用精化不是值类型）
                    field.ty = classified.ty.clone();
                }
            }
            MonoType::Struct(structure)
        }
        other => other,
    }
}
