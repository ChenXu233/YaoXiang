//! 编译期证明结果 —— 三值代数
//!
//! 所有编译期检查（类型等式、所有权、终止性、精化谓词）
//! 统一返回此类型。这是 RFC-027 Section 4.1 的核心数据类型。

use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::codes::ErrorCodeDefinition;
use crate::util::span::Span;

/// TypeChecker 发出的信号：这个证明函数需要被编译期执行
///
/// RFC-027 §4.2: 编译器无法自动证明约束时，
/// 程序员显式引用的证明函数在编译期被执行并验证。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofFunctionCall {
    /// 函数名（如 "Sorted"、"IsEven"）
    pub func_name: String,
    /// 实参——编译期已知的具体值
    pub args: Vec<crate::frontend::core::types::const_data::ConstValue>,
}

/// 反例类型 — 决定错误码和诊断模板
///
/// 消除调用方的 if/else——数据结构本身就区分了情况。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisproofKind {
    /// 精化谓词违反（直接求值 false 或 SMT 反例）→ E4018
    PredicateViolation,
    /// 类型等式不成立（两方归约后值不等）→ E4019
    TypeMismatch,
}

/// 证明结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofResult {
    /// 证明成立
    Proved,
    /// 证伪，带反例模型
    Disproved(DisproofModel),
    /// 在给定资源内无法证明（不等于命题为假）
    Unproven {
        reason: UnprovenReason,
        /// Phase 2.5: 需要被执行的证明函数。非空时 Pipeline 执行它们。
        proof_calls: Vec<ProofFunctionCall>,
        budget: BudgetReport,
    },
}

/// 反例模型
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisproofModel {
    /// 反例类型 — 决定错误码
    pub kind: DisproofKind,
    /// 变量名 → 使命题为假的具体值
    pub assignments: Vec<(String, String)>,
    /// 约束/等式的文本描述（如 "x > 0"、"Int == Float"）
    pub constraint: String,
    /// 违反位置
    pub span: Option<Span>,
    /// 谓词定义位置（仅 PredicateViolation 时填入）
    pub predicate_span: Option<Span>,
}

impl DisproofModel {
    /// 将反例模型转换为诊断信息
    ///
    /// 根据 `kind` 选择错误码和 i18n 模板，
    /// 构造带 Span 的完整 Diagnostic。
    pub fn into_diagnostic(self) -> Diagnostic {
        match self.kind {
            DisproofKind::PredicateViolation => {
                let counterexample = if self.assignments.is_empty() {
                    "  (no variable assignments)".to_string()
                } else {
                    self.assignments
                        .iter()
                        .map(|(k, v)| format!("  {} = {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                let mut builder = ErrorCodeDefinition::refinement_violated(&self.constraint)
                    .param("counterexample", &counterexample);

                if let Some(span) = self.span {
                    builder = builder.at(span);
                }

                builder.build()
            }
            DisproofKind::TypeMismatch => {
                let (expected, found) = if self.assignments.len() >= 2 {
                    (self.assignments[0].1.clone(), self.assignments[1].1.clone())
                } else if self.assignments.len() == 1 {
                    (self.constraint.clone(), self.assignments[0].1.clone())
                } else {
                    (String::new(), String::new())
                };

                let mut builder = ErrorCodeDefinition::type_mismatch_in_proof(&expected, &found);

                if let Some(span) = self.span {
                    builder = builder.at(span);
                }

                builder.build()
            }
        }
    }
}

/// 无法证明的原因
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnprovenReason {
    /// 存在符号变量
    Symbolic(String),
    /// 超出推理规则
    BeyondKernel(String),
    /// 超出预算
    BudgetExceeded,
    /// Phase 2.5: 需要程序员提供的证明函数
    ProofFunctionRequired,
}

/// 求解预算报告
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetReport {
    pub steps_used: u32,
    pub steps_limit: u32,
}

impl ProofResult {
    /// 是否证明通过
    pub fn is_proved(&self) -> bool {
        matches!(self, Self::Proved)
    }

    /// 转换为 Result（用于 Result 风格错误处理）
    pub fn into_result(self) -> Result<(), Diagnostic> {
        match self {
            Self::Proved => Ok(()),
            Self::Disproved(model) => Err(model.into_diagnostic()),
            Self::Unproven { reason, .. } => Err(Diagnostic::error(
                "E8001".to_string(),
                format!("无法证明: {:?}", reason),
                String::new(),
                None,
            )),
        }
    }
}
