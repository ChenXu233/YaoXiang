//! 编译期证明结果 —— 三值代数
//!
//! 所有编译期检查（类型等式、所有权、终止性、精化谓词）
//! 统一返回此类型。这是 RFC-027 Section 4.1 的核心数据类型。

use crate::util::diagnostic::Diagnostic;

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

/// 反例模型：变量名 → 使命题为假的具体值
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisproofModel {
    pub assignments: Vec<(String, String)>,
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
            Self::Disproved(model) => Err(Diagnostic::error(
                "E8001".to_string(),
                format!("反例: {:?}", model.assignments),
                String::new(),
                None,
            )),
            Self::Unproven { reason, .. } => Err(Diagnostic::error(
                "E8001".to_string(),
                format!("无法证明: {:?}", reason),
                String::new(),
                None,
            )),
        }
    }
}
