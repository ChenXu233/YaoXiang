//! 编译期证明结果 —— 三值代数
//!
//! 所有编译期检查（类型等式、所有权、终止性、精化谓词）
//! 统一返回此类型。这是 RFC-027 Section 4.1 的核心数据类型。

use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::codes::ErrorCodeDefinition;
use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::const_data::ConstExpr;
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
    // 所有权相关（RFC-009a §系统谓词清单）
    BorrowConflict,
    UseAfterMove,
    UseAfterDrop,
    DoubleDrop,
    MutViolation,
    /// unsafe 解引用违规（在 unsafe 块外解引用裸指针）→ E2027
    UnsafeViolation,
    /// spawn 内 ref 循环（ref 变量间形成环形引用）→ E2029
    SpawnCycleViolation,
    /// 复制 Linear 令牌（`&mut T` 非 Dup，赋值复制被拒绝，#257）→ E2003
    LinearTokenCopy,
    /// 测度不成立（SMT 给出反例，RFC-027a §义务生成）→ E4022
    ///
    /// 只用于**递减**义务判伪。良基性「推不出下界」走 `Unproven` 而非判伪
    /// （RFC-027a:223「只给方向不拒绝」）。
    MeasureNotDecreasing,
}

/// 证明结果（6.9.3a：`Result<(), ProofFailure>` 的类型别名）
///
/// **唯一失败代数**：成功是 `()`，失败只能是 [`ProofFailure`]。6.9.3a 之前这里
/// 是一套独立三值枚举，与项层的三值并列——两套三值各丢各的信息
///（项层丢符号名、丢诊断），且判等/归一化各自就地造失败值。现在失败侧只有
/// [`ProofFailure`] 一种类型，项层归一化（`const_eval::normalize`）也返回它。
pub type ProofResult = Result<(), ProofFailure>;

/// 未解命题的**主体**（t66 / RFC-027 §12.3）：诊断必须能说清「在证什么」。
///
/// 为什么不是 `ConstExpr`：项形状装不下**类型等式**命题（`A ≡ B`）——它不是项。
/// 只用一个 `ConstExpr` 会把判等义务逼成 `None`，诊断退化成中性主语「该义务」，
/// 与 §12.3「必须携带未解命题」的字面要求不符。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnprovenSubject {
    /// 编译期项命题（如 `n > 0`、`N == 5` 的项形态）
    Term(ConstExpr),
    /// 类型等式命题 `A ≡ B`（判等/子类型义务）
    ///
    /// 两侧**装箱**：`MonoType` 是大枚举，直接内联会把 `ProofFailure` 撑大，
    /// 触发 `large_enum_variant` / `result_large_err`（clippy -D warnings）。
    TypeEquivalence(Box<MonoType>, Box<MonoType>),
}

/// 证明失败（唯一失败类型）
///
/// 两臂语义互不降格：`Disproved` = 已算出「命题为假」（可给反例），
/// `Unproven` = 在给定资源/推理规则内**算不出来**（不等于为假）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofFailure {
    /// 证伪，带反例模型
    Disproved(DisproofModel),
    /// 在给定资源内无法证明（不等于命题为假）
    Unproven {
        reason: UnprovenReason,
        /// Phase 2.5: 需要被执行的证明函数。非空时 Pipeline 执行它们。
        ///
        /// 6.9.3a 的契约草案写的是 `Unproven { reason, budget }` 两字段；实现保留
        /// 本字段，因为它是 RFC-027 Phase 2.5 的**活功能**：`checker` 从失败值里
        /// 收集这些调用交给 `execute_proof_calls` 执行，删掉即静默废功（红线「核心
        /// 功能不留待实现」）。语义与 6.9.3a 之前逐字一致。
        proof_calls: Vec<ProofFunctionCall>,
        budget: BudgetReport,
        /// **未解命题的主体**（t64/t66 / RFC-027 §12.3）：可渲染的结构化命题。
        ///
        /// `None` = 该判定点**没有**结构化命题（如单侧归约失败、终止性自带原因）；
        /// 构造点必须显式写 `None`，**不得**用哑串冒充（`Option` 就是为了让
        /// 「没有命题」与「命题是空串」在类型上可区分）。
        subject: Option<UnprovenSubject>,
        /// 该义务的位置（t64 / RFC-027 §12.3）：有则 E4023 指向它。
        ///
        /// `None` = 该义务无源码位置（如纯类型层判等）；诊断构造处回落
        /// `Span::dummy()` 并 `tracing::debug!` 记录，不静默丢失。
        span: Option<Span>,
    },
}

impl ProofFailure {
    /// 失败 → 用户可见诊断
    ///
    /// 映射逻辑与 6.9.3a 之前 `ProofResult::into_result` 逐字一致（同一个
    /// `DisproofModel::into_diagnostic` / `UnprovenReason::to_diagnostic`）。
    pub fn into_diagnostic(self) -> Diagnostic {
        match self {
            Self::Disproved(model) => model.into_diagnostic(),
            Self::Unproven {
                reason,
                subject,
                span,
                budget,
                ..
            } => reason.to_diagnostic(subject.as_ref(), span, &budget),
        }
    }
}

/// `ProofResult` 的便捷方法（它是 `Result` 别名，方法只能由扩展 trait 提供）
///
/// 语义与 6.9.3a 之前同名方法一致：`is_proved` = 成功；`into_result` = 失败映射成
/// 既有诊断（映射逻辑见 [`ProofFailure::into_diagnostic`]）。
pub trait ProofResultExt {
    /// 是否证明通过
    fn is_proved(&self) -> bool;
    /// 转换为 Result（用于 Result 风格错误处理）
    fn into_result(self) -> Result<(), Diagnostic>;
}

impl ProofResultExt for ProofResult {
    fn is_proved(&self) -> bool {
        self.is_ok()
    }

    fn into_result(self) -> Result<(), Diagnostic> {
        self.map_err(ProofFailure::into_diagnostic)
    }
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
    /// 从 assignments 中提取指定 key 的值
    fn extract_var<'a>(
        assignments: &'a [(String, String)],
        key: &str,
    ) -> Option<&'a String> {
        assignments.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// 将反例模型转换为诊断信息
    ///
    /// 根据 `kind` 选择错误码和 i18n 模板，
    /// 构造带 Span 的完整 Diagnostic。
    pub fn into_diagnostic(self) -> Diagnostic {
        match self.kind {
            DisproofKind::PredicateViolation => {
                // issue #469 处 2a：反例段（标题 + 内容）整体由 locales 的
                // per-site 键渲染（`E4018.counterexample.section`），模型未绑定
                // 值时该参数为空串——模板占位符渲染为空，段整体缺席。
                let mut builder = ErrorCodeDefinition::refinement_violated(&self.constraint);
                if self.assignments.is_empty() {
                    builder = builder.param("counterexample", "");
                } else {
                    let assignments = self
                        .assignments
                        .iter()
                        .map(|(k, v)| format!("  {} = {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n");
                    builder = builder
                        .param("assignments", assignments)
                        .param_from_key("counterexample", "E4018.counterexample.section");
                }

                if let Some(span) = self.span {
                    builder = builder.at(span);
                }

                builder.build()
            }
            DisproofKind::MeasureNotDecreasing => {
                let counterexample = if self.assignments.is_empty() {
                    "（求解器未给出具体取值）".to_string()
                } else {
                    self.assignments
                        .iter()
                        .map(|(k, v)| format!("{k} = {v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let mut builder = ErrorCodeDefinition::measure_not_decreasing(&counterexample);
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
            DisproofKind::BorrowConflict => {
                let name = Self::extract_var(&self.assignments, "token")
                    .unwrap_or(&String::new())
                    .clone();
                let mut builder = ErrorCodeDefinition::mutable_immutable_borrow_conflict(&name);
                if let Some(span) = self.span {
                    builder = builder.at(span);
                }
                builder.build()
            }
            DisproofKind::UseAfterMove => {
                let name = Self::extract_var(&self.assignments, "variable")
                    .unwrap_or(&String::new())
                    .clone();
                let mut builder = ErrorCodeDefinition::use_after_move(&name);
                if let Some(span) = self.span {
                    builder = builder.at(span);
                }
                builder.build()
            }
            DisproofKind::UseAfterDrop => {
                let name = Self::extract_var(&self.assignments, "variable")
                    .unwrap_or(&String::new())
                    .clone();
                let mut builder = ErrorCodeDefinition::use_after_drop(&name);
                if let Some(span) = self.span {
                    builder = builder.at(span);
                }
                builder.build()
            }
            DisproofKind::DoubleDrop => {
                let name = Self::extract_var(&self.assignments, "variable")
                    .unwrap_or(&String::new())
                    .clone();
                let mut builder = ErrorCodeDefinition::double_drop(&name);
                if let Some(span) = self.span {
                    builder = builder.at(span);
                }
                builder.build()
            }
            DisproofKind::MutViolation => {
                let name = Self::extract_var(&self.assignments, "variable")
                    .unwrap_or(&String::new())
                    .clone();
                let mut builder = ErrorCodeDefinition::immutable_assign(&name);
                if let Some(span) = self.span {
                    builder = builder.at(span);
                }
                builder.build()
            }
            DisproofKind::UnsafeViolation => {
                let mut builder = ErrorCodeDefinition::unsafe_deref();
                if let Some(span) = self.span {
                    builder = builder.at(span);
                }
                builder.build()
            }
            DisproofKind::SpawnCycleViolation => {
                let cycle_info = self.constraint.clone();
                let mut builder = ErrorCodeDefinition::spawn_ref_cycle(&cycle_info);
                if let Some(span) = self.span {
                    builder = builder.at(span);
                }
                builder.build()
            }
            DisproofKind::LinearTokenCopy => {
                let mut builder = ErrorCodeDefinition::ownership_violation(&self.constraint);
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
    /// 循环终止性无法自动证明
    ///
    /// 这是**编译器能力边界**，不是程序错误，更不是内部故障。
    /// 必须与 `BeyondKernel` 区分：后者的文案路径会走 ICE（E8001）。
    ///
    /// 携带循环位置的 span，使 E4021 能指向具体循环（#377 的升级路径：
    /// 原 `ProofResult::Unproven` 不带位置，诊断只能无 span 输出）。
    LoopTerminationUnproven { span: crate::util::span::Span },
}

/// 求解预算报告
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetReport {
    pub steps_used: u32,
    pub steps_limit: u32,
}

impl UnprovenReason {
    /// 一般性「无法证明」的诊断（t64：专属 **E4023**）。
    ///
    /// `subject` 缺失时不伪造命题：用中性主语「该义务」占位，具体内容由
    /// `reason` 一栏承载（模板 `无法证明：{proposition}（{reason}）`）。
    ///
    /// 两类主体分别渲染：`Term(t)` ⇒ `t` 的 Display；`TypeEquivalence(a, b)` ⇒
    /// `a ≡ b`（类型等式的字面形态，RFC-027 §12.3「未解命题」可直接读出）。
    /// 位置优先用调用方给的 `span`；缺失时用 `Span::dummy()` 并记日志——
    /// E4023 的码表条目 `span_exempt=false`，不给位置会触发 span 强制（debug 下
    /// panic、release 降级 E8001），等于把能力边界又变回 ICE。
    fn unproven_general_diagnostic(
        subject: Option<&UnprovenSubject>,
        span: Option<Span>,
        reason: &str,
    ) -> Diagnostic {
        let proposition_text = match subject {
            Some(UnprovenSubject::Term(term)) => term.to_string(),
            Some(UnprovenSubject::TypeEquivalence(lhs, rhs)) => format!("{lhs} ≡ {rhs}"),
            None => {
                tracing::debug!("无法证明：该判定点没有结构化命题，用中性主语渲染");
                "该义务".to_string()
            }
        };
        let span = match span {
            Some(span) => span,
            None => {
                tracing::debug!("无法证明：该义务没有源码位置，回落 dummy span");
                Span::dummy()
            }
        };
        ErrorCodeDefinition::unproven_general(&proposition_text, reason)
            .at(span)
            .build()
    }

    /// 将「无法证明」转为用户可读诊断（**穷尽 match**，无 ICE 兜底）。
    ///
    /// 每个变体都有显式分支：`LoopTerminationUnproven` → E4021（自带 span）、
    /// `ProofFunctionRequired` → E4020、其余三条**一般性**原因（`Symbolic` /
    /// `BudgetExceeded` / `BeyondKernel`）→ **专属 E4023**（t64 / RFC-027 §12.3）。
    ///
    /// E2031 只服务**精化通道**（带真实 `assigned`/`var` 两槽）：一般义务不再借用它，
    /// 因为空串形态的 E2031 会误导用户以为问题出在精化约束上。
    fn to_diagnostic(
        &self,
        subject: Option<&UnprovenSubject>,
        span: Option<Span>,
        budget: &BudgetReport,
    ) -> Diagnostic {
        match self {
            Self::LoopTerminationUnproven { span } => {
                ErrorCodeDefinition::loop_termination_unproven()
                    .at(*span)
                    .build()
            }
            Self::ProofFunctionRequired => ErrorCodeDefinition::proof_function_required().build(),
            // 符号名：用户能看懂「依赖哪个编译期名字」，措辞不带 Rust 调试文本
            Self::Symbolic(name) => {
                Self::unproven_general_diagnostic(subject, span, &format!("含未绑定符号 `{name}`"))
            }
            // 预算：消息只放**一行摘要**（步数），完整分解进日志（RFC-027 §12.3 的形式放松）
            Self::BudgetExceeded => {
                tracing::debug!(
                    "无法证明：预算耗尽（已用 {} 步 / 上限 {} 步）",
                    budget.steps_used,
                    budget.steps_limit
                );
                Self::unproven_general_diagnostic(
                    subject,
                    span,
                    &format!("预算耗尽（{} 步）", budget.steps_used),
                )
            }
            // `BeyondKernel` 的载荷由**构造方**给出用户域措辞（6.9.3c 已把
            // `equivalence.rs` 里原先的 Rust `Debug` 串改成类型域文案）
            Self::BeyondKernel(detail) => Self::unproven_general_diagnostic(subject, span, detail),
        }
    }
}
