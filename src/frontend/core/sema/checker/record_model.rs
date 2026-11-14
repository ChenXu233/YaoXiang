//! 声明期一遍：把记录字段分类成 data / evidence（RFC-027 §14.1 / SPEC §8.3.1）。
//!
//! 三步判据：**space → position → category**。
//! - space：字段类型是否落在命题空间（谓词应用 / 证明函数应用 / 证明空间类型族）；
//! - position：命题的自由名是否只引用**严格前序数据字段**、还是只有自身（admission）；
//! - category：evidence ⇒ `Some(Quantity::Zero)`；admission / data ⇒ `None`。
//!
//! 本遍是 `RecordModel` 的**唯一写入点**；[crate::frontend::core::sema::record_model::apply_quantities]
//! 把结论写进 `poly`（`env.types` / `env.vars` / `generic_type_defs` 因此同源）。
//! **不改写字段的 `MonoType`**——`StructType` 只写 `quantity`，命题只进模型。
//!
//! B.2a 边界：泛型记录不分类（B.2b 待 D0 裁决，见 09a 契约 §一）。

use super::TypeChecker;
use crate::ast::{GenericParam, GenericParamKind, StructField, Type as AstType};
use crate::frontend::core::sema::record_model::{
    AdmissionField, EvidenceSlot, FieldObligation, Proposition, RecordModel, RecordRejection,
    RecordTypeParam, RecordTypeParamKind, record_rejection_diagnostic,
};
use crate::frontend::core::types::const_data::ConstExpr;
use crate::frontend::core::types::eval::const_eval::const_expr_free_names;
use crate::frontend::core::types::eval::dependent_types::SortSpace;
use crate::frontend::core::types::mono::{MonoType, StructFieldInfo};
use crate::util::span::Span;

/// 位置判据的结论。
enum FieldPosition {
    /// evidence：命题只引用严格前序数据字段（空集 = 只引用编译期值参数/常量）
    Evidence {
        depends_on: Vec<usize>,
        param_refs: Vec<usize>,
    },
    /// admission：自引用精化，按数据字段（RFC-027 §14.1 第 3 行）
    Admission,
}

impl TypeChecker {
    /// 声明期一遍。
    ///
    /// 返回 `Some(model)` 表示本记录已分类；`None` 表示不在 B.2a 面
    /// （非记录体 / 泛型记录）。非法位置在这里就地报诊断并返回 `None`
    /// （编译失败，量词不写，后续阶段不运行）。
    pub(super) fn classify_record(
        &mut self,
        name: &str,
        definition: &AstType,
        generic_params: &[GenericParam],
        span: Span,
    ) -> Option<RecordModel> {
        let fields: Vec<&StructField> = match definition {
            AstType::Struct { .. } => definition.struct_fields(),
            _ => return None,
        };
        // B.2a 边界：泛型记录的分类与实例化义务归 B.2b（D0 未裁）。
        if !generic_params.is_empty() {
            return None;
        }

        let type_param_names: Vec<String> = generic_params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        let mut model = RecordModel {
            name: name.to_string(),
            type_params: generic_params
                .iter()
                .map(|param| RecordTypeParam {
                    name: param.name.clone(),
                    kind: match param.kind {
                        GenericParamKind::Type => RecordTypeParamKind::Type,
                        GenericParamKind::Const { .. } => RecordTypeParamKind::Const,
                    },
                })
                .collect(),
            fields: Vec::with_capacity(fields.len()),
            obligations: Vec::new(),
            evidence: Vec::new(),
            admission_fields: Vec::new(),
            field_path: fields
                .iter()
                .enumerate()
                .map(|(index, field)| (field.name.clone(), index))
                .collect(),
        };

        let mut quantities: Vec<Option<crate::ast::Quantity>> = vec![None; fields.len()];
        let mut rejected = false;
        for (index, field) in fields.iter().enumerate() {
            // 直驱表示层输入（B.1 契约的直驱通道）：parser 恒写 None，只有表示层
            // 判据会把字段预标成 witness。保留该读数、不产义务——生产路径永远
            // 走下面的分类分支。
            let preset = field.quantity;
            let mut quantity = preset;
            let proposition = if preset.is_some() {
                None
            } else {
                self.proof_proposition(&field.ty)
            };
            // 不是命题形态时，仍让既有解析器对**谓词/证明函数应用**发声——
            // 字段位的非法实参（元数不符 / 实参不可转换）原先静默接受，T09 的
            // 判据是「既有码（E1092/E1093）在字段位发射」；解析器只对这类头报码，
            // 其余字段类型（Array/Vec/记录）静默返回，不引入新诊断。
            if proposition.is_none() && matches!(&field.ty, AstType::Generic { .. }) {
                let mut field_diags = Vec::new();
                let _ = self
                    .resolve_type_annotation(&MonoType::from(field.ty.clone()), &mut field_diags);
                for diagnostic in field_diags {
                    self.add_error(diagnostic);
                }
            }
            if let Some(proposition) = proposition {
                match self.classify_field_position(
                    name,
                    field,
                    index,
                    &fields,
                    &quantities,
                    &proposition,
                    &type_param_names,
                ) {
                    Ok(FieldPosition::Admission) => {
                        model.admission_fields.push(AdmissionField {
                            field: index,
                            proposition: proposition.clone(),
                        });
                    }
                    Ok(FieldPosition::Evidence {
                        depends_on,
                        param_refs,
                    }) => {
                        if field.default.is_some() {
                            rejected = true;
                            self.add_error(record_rejection_diagnostic(
                                name,
                                &RecordRejection::DefaultOnEvidence {
                                    field: field.name.clone(),
                                },
                                span,
                            ));
                            continue;
                        }
                        quantity = Some(crate::ast::Quantity::Zero);
                        model.evidence.push(EvidenceSlot {
                            field: index,
                            quantity,
                            proposition: Some(proposition.clone()),
                        });
                        model.obligations.push(FieldObligation {
                            record: name.to_string(),
                            field: index,
                            proposition,
                            depends_on,
                            param_refs,
                        });
                    }
                    Err(rejection) => {
                        rejected = true;
                        self.add_error(record_rejection_diagnostic(name, &rejection, span));
                    }
                }
            }
            quantities[index] = quantity;
            // admission 字段的值类型是命题的 base（自引用精化不是值类型）
            let value_ty = model
                .admission_fields
                .iter()
                .find(|admission| admission.field == index)
                .map(|admission| admission.proposition.base.clone())
                .unwrap_or_else(|| MonoType::from(field.ty.clone()));
            model.fields.push(StructFieldInfo {
                name: field.name.clone(),
                ty: value_ty,
                quantity,
                is_mut: field.is_mut,
                has_default: field.default.is_some(),
            });
        }

        if rejected {
            return None;
        }
        // 直驱预标的 witness 字段：补 evidence 槽（无命题），使 5A / E1114 的
        // 判据在表示层测试里仍从模型读数（生产路径由上面的分类分支填满）。
        for (index, field) in fields.iter().enumerate() {
            if field.quantity == Some(crate::ast::Quantity::Zero)
                && !model.evidence.iter().any(|slot| slot.field == index)
            {
                model.evidence.push(EvidenceSlot {
                    field: index,
                    quantity: Some(crate::ast::Quantity::Zero),
                    proposition: None,
                });
                model.evidence.sort_by_key(|slot| slot.field);
            }
        }
        Some(model)
    }

    /// space + 命题：字段类型落在命题空间时返回要证明的命题。
    fn proof_proposition(
        &self,
        ty: &AstType,
    ) -> Option<Proposition> {
        let AstType::Generic {
            name: head, args, ..
        } = ty
        else {
            return None;
        };
        // 1. 谓词 / 证明函数应用（AST 无损代入）
        if let Some(MonoType::Refined { base, constraint }) =
            self.refine_predicate_from_ast_args(head, args)
        {
            return Some(Proposition {
                base: *base,
                constraint,
            });
        }
        // 2. 证明空间的类型族（Assert / IsTrue 等；登记数据，不硬编码名字）
        if let Some(SortSpace::Proof) = self.dependent_type_env.type_family_space(head) {
            let constraint = self.ast_type_to_const_expr(args.first()?)?;
            return Some(Proposition {
                base: MonoType::Void,
                constraint,
            });
        }
        // 3. 内建谓词（Terminates）：不透明 Call，构造点判不了（E2031）
        if crate::frontend::core::sema::probe::is_builtin_predicate_name(head) {
            let args: Option<Vec<ConstExpr>> = args
                .iter()
                .map(|arg| self.ast_type_to_const_expr(arg))
                .collect();
            return Some(Proposition {
                base: MonoType::Void,
                constraint: ConstExpr::Call {
                    func: head.clone(),
                    args: args?,
                },
            });
        }
        None
    }

    /// position：命题自由名相对本记录字段的位置判据。
    #[allow(clippy::too_many_arguments)]
    fn classify_field_position(
        &self,
        record: &str,
        field: &StructField,
        index: usize,
        all_fields: &[&StructField],
        quantities: &[Option<crate::ast::Quantity>],
        proposition: &Proposition,
        type_param_names: &[String],
    ) -> Result<FieldPosition, RecordRejection> {
        let mut depends_on: Vec<usize> = Vec::new();
        let mut param_refs: Vec<usize> = Vec::new();
        let mut self_ref = false;
        for name in const_expr_free_names(&proposition.constraint) {
            if let Some(target) = all_fields
                .iter()
                .position(|candidate| candidate.name == name)
            {
                if target == index {
                    self_ref = true;
                } else if target < index {
                    if matches!(quantities[target], Some(crate::ast::Quantity::Zero)) {
                        return Err(RecordRejection::EvidenceReference {
                            field: field.name.clone(),
                            other: all_fields[target].name.clone(),
                        });
                    }
                    if !depends_on.contains(&target) {
                        depends_on.push(target);
                    }
                } else {
                    return Err(RecordRejection::ForwardReference {
                        field: field.name.clone(),
                        other: all_fields[target].name.clone(),
                    });
                }
            } else if let Some(param) = type_param_names.iter().position(|known| *known == name) {
                if !param_refs.contains(&param) {
                    param_refs.push(param);
                }
            }
            // 其余自由名 = 模块级编译期常量 / 不透明名（构造点判定时再结算）
        }
        let _ = record;
        if self_ref {
            if depends_on.is_empty() {
                Ok(FieldPosition::Admission)
            } else {
                Err(RecordRejection::SelfMixed {
                    field: field.name.clone(),
                })
            }
        } else {
            Ok(FieldPosition::Evidence {
                depends_on,
                param_refs,
            })
        }
    }
}
