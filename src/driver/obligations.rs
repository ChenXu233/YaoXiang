//! 义务账本：跨阶段契约字段（义务）的产出 / 交付 / 结算
//! —— 02-stage-contract §2「义务账本：`Obligations` + `assert_drained()`」
//! / RFC-039 D5（两步走：4.3.1 观测期 W 级 → 4.3.2 同码升 E，已完成）。
//!
//! # 为什么需要它
//!
//! `TypeCheckResult` 里有一批字段的契约是「由某阶段产出 → 由另一阶段消费」
//!（条数以 `Obligation::ALL` / 02 §义务字段表为准，本模块不写死计数）
//! （02 §义务字段表）。编译流程把**整份结果**交给下游，却没有任何机制保证
//! 下游真的读了某个字段——`proof_calls` 曾在 orchestrator 四入口被静默丢弃
//! （02 §正确性漏洞证据链）。本模块把「交付」变成显式标记，并在阶段表尾部
//! 一次结算：义务非空而没有交付标记 → 产 W1090。
//!
//! # 结算语义（02 §2 三档表 + S1）
//!
//! | 情形 | 判据 | 处置 |
//! | --- | --- | --- |
//! | 义务为空 | 字段空 | 通过（空义务不报） |
//! | 消费者阶段不在本 ProgramKind 阶段表 | `ProgramKind::stages()`（S1，2026-10-13 用户裁决） | 通过（不判欠：Lsp 无 IrGeneration/Monomorphization，Embedded 无 Monomorphization） |
//! | 消费者阶段被跳过 | `SkipReason::UpstreamFailed` / `ConfigDisabled` | 通过（跳过不是丢弃） |
//! | 有交付标记 | 交付点调用 `deliver` | 通过（已交付） |
//! | 其余（非空 + 消费者阶段已运行 + 无标记） | — | **产 W1090**（4.3.1 W 级 → 4.3.2 **error 级**，D5 终态） |
//!
//! # 红名单（当前为空）与非义务字段登记
//!
//! 红名单条目（[`Obligation::red_list`]）是已核实的「跨阶段消费者为零」字段：
//! 它们没有可交付点，**不进用户可见诊断流**（裁定丙），其「不静默」由三处承担：
//! 1. **审计面**：[`DriverOutcome.obligations.findings`] 每次运行带出（含 WBS id 与证据）；
//! 2. **静态门禁**：`scripts/ci/check-obligations.py`（WBS 9.2）逐条对账 + **陈旧检测**
//!    （红名单字段长出消费者即红）+ `--selftest` 反例/正例夹具（含 stale-red-list 形态）；
//! 3. **文档注记**：02-stage-contract §2 实施修订注记逐条登记。
//!
//! **2026-10-13 清账收口：红名单 3 → 0**（09-WBS 三条全部落地）。
//! - `variant_ctor_calls` → 3.4.4 **已清账**（删字段：IR 侧 `detect_variant_ctor_call`
//!   就地形态检测等价兑现其职责，跨层 span 键控会引入失配）；
//! - `method_overload_ir_names` → 3.4.5 **已清账**（删 `TypeCheckResult` 上的跨阶段镜像，
//!   在相位候选名表与 `overload_resolutions` 保留）；
//! - `interface_impl_registry` → 3.4.9 **已清账**（用户裁决甲：改判为**树内事实**——
//!   它在 typecheck 树内有真实在相位消费（`src/frontend/core/typecheck/operator_interfaces.rs:623` 运算符查询、
//!   `src/frontend/core/typecheck/inference/expressions.rs` 七处约束求解查询、
//!   `src/frontend/core/typecheck/checker.rs:214-217` 只读访问器），
//!   而跨阶段读取为零（`src/middle` 0 命中）；删掉类型注释里「镜像供 middle 端/LSP
//!   消费」的**声明**，保留字段本身与在相位消费。将来 LSP 真需要该表，按 4.6.x
//!   新登记走，不在本轮造消费者）。
//!
//! 红名单为空后，字段普查由 [`NON_OBLIGATION_FIELDS`] 与非义务登记一起闭合：
//! `TypeCheckResult` 的字段集必须恰好等于「义务清单 ∪ 非义务登记」，两端都不许
//! 有未登记字段（门禁判据 1 的反向 containment）。
//!
//! **唯一调用点**：`Driver::run` 阶段表尾部、`into_outcome()` 产出
//! `CompilationResult` 之前（02 §2；`State::assert_drained`）。
//!
//! # 交付标记与静态门禁（两半互补）
//!
//! 交付标记是**声明**：谁在哪个交付点把义务交给消费者。它与下游真实读取之间
//! 可能漂移（标记了但下游不读）——运行期看不见这种漂移，**漂移由静态门禁
//! `scripts/ci/check-obligations.py`（WBS 9.2）拦截**：它把「交付标记 ↔ 该字段
//! 在消费端真被读到」做成硬判据，并对红名单做**陈旧检测**（红名单字段若长出
//! 消费者，门禁必须红）。
//!
//! # 级别（D5 两步走 → 终态）
//!
//! **4.3.1（观测期）**：只产 W 级（2026-10-13 用户裁决，D5 的观测期）。
//! **4.3.2（终态，2026-10-13 用户裁决甲）**：**同码升 E**——码号仍是 `W1090`，
//! 发射级别由显式 `.severity(Severity::Error)` 抬到 error（不新增 E 码：同一事实
//! 两个码即平行表示）。触发即进错误通道、改退出码（D59 的 D 类破坏项，如实登记）；
//! 健康态零发射，故对既有语料与用户程序零影响。
//! `span_exempt` 的理由见 `src/util/diagnostic/codes/w1xxx.rs` 的 W1090 条目。

use std::path::PathBuf;

use crate::frontend::core::typecheck::TypeCheckResult;
use crate::frontend::module::orchestrator::{is_vendor_path, OrchestratorError};
use crate::frontend::pipeline::PipelineError;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition, Severity};

use super::program::ProgramKind;
use super::stage::Stage;
use super::state::{SkipReason, SkippedStage, State};

/// W1090 的码字符串（注册单源在 `src/util/diagnostic/codes/w1xxx.rs`）。
pub const OBLIGATION_UNDRAINED_CODE: &str = "W1090";

/// 义务字段——`TypeCheckResult` 中「产出即契约」的字段（条数见 [`Obligation::ALL`]）
/// （02-stage-contract §义务字段表；定义见
/// `src/frontend/core/typecheck/types.rs:16-77`）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Obligation {
    /// `proof_calls`：typecheck → 证明执行
    ProofCalls,
    /// `release_plan`：ownership → IR 生成
    ReleasePlan,
    /// `escaped_refs`：ownership → IR 生成
    EscapedRefs,
    /// `instantiation_requests`：typecheck → 单态化
    InstantiationRequests,
    /// `existential_coercions`：typecheck → IR 生成
    ExistentialCoercions,
    /// `implementation_proofs`：typecheck → IR 生成
    ImplementationProofs,
    /// `sum_types`：typecheck → IR 生成
    SumTypes,
    /// `sum_type_param_names`：typecheck → IR 生成
    SumTypeParamNames,
    /// `operator_dispatches`：typecheck → IR 生成（span 键控）
    OperatorDispatches,
    /// `overload_resolutions`：typecheck → IR 生成（span 键控）
    OverloadResolutions,
    /// `try_expr_impls`：typecheck → IR 生成（span 键控）
    TryExprImpls,
    /// `match_scrutinee_types`：typecheck → IR 生成（span 键控）
    MatchScrutineeTypes,
    /// `module_namespaces`：typecheck → IR 生成
    ModuleNamespaces,
}

/// 红名单条目：已核实的「跨阶段消费者为零」字段的登记
/// （清账任务的 WBS id + 零消费者证据指针）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RedListEntry {
    /// 负责清账的 WBS 任务 id（09-execution-wbs.md 的行）
    pub wbs: &'static str,
    /// 「消费者为零」的直接证据指针（可复核的命令 / file:line）
    pub evidence: &'static str,
}

/// `TypeCheckResult` 里**不属跨阶段义务**的字段登记（字段名 + 理由）。
///
/// 判据 1（三处对账）的第三处：`scripts/ci/check-obligations.py` 拿它与
/// [`Obligation::ALL`] 一起做**反向 containment**——`TypeCheckResult` 的字段集
/// 必须恰好等于「义务清单 ∪ 本登记」，两端都不许有未登记字段。这样
/// 「删字段忘了同步清单」与「加字段忘了登记」都必须当场红，而不是靠放宽
/// containment 绕过。
///
/// **为什么不是「接线消费端」**：这些字段要么是通道/元信息/阶段内产物，
/// 要么是**树内事实**（按设计由 typecheck 在相位消费、跨阶段零读取，
/// 如 3.4.9 改判的 `interface_impl_registry`）。给它们造一个下游消费者
/// 只会凭空引入假义务，把「按设计不需要跨阶段消费」伪装成契约缺陷。
///
/// 收编判据（与 02 §义务字段表一致）：文档注释是否声明「由 X 阶段产出 →
/// Y 阶段消费」。本表逐条写明该字段的实测消费面，供将来复核。
pub struct NonObligationField {
    /// `TypeCheckResult` 里的字段名（逐字一致）
    pub field: &'static str,
    /// 为什么它不是跨阶段义务（必须写实测消费面 / 通道归属，不许留空）
    pub reason: &'static str,
}

/// 非义务字段登记表（见 [`NonObligationField`]）。
pub const NON_OBLIGATION_FIELDS: &[NonObligationField] = &[
    NonObligationField {
        field: "module_name",
        reason: "单元身份元信息：文档注释未声明跨阶段契约，src/middle 0 命中",
    },
    NonObligationField {
        field: "diagnostics",
        reason: "错误诊断通道：各入口直接读取的错误集合，是既有通道而非「产出即契约」",
    },
    NonObligationField {
        field: "bindings",
        reason: "类型绑定表：确有跨阶段读取（src/middle/core/ir_gen.rs:735/:5034），但文档注释未声明跨阶段契约 ⇒ 未收编进 02 义务字段表；登记在此闭合字段普查",
    },
    NonObligationField {
        field: "local_var_types",
        reason: "诊断渲染用局部类型表：跨阶段读取在 src/middle/core/ir_gen.rs:852/:4543/:4794（错误消息），与 bindings 同——未声明契约",
    },
    NonObligationField {
        field: "semantic_db",
        reason: "语义信息数据库：消费者在 LSP（src/lsp/server.rs:493 world.update_semantic_db），不属编译阶段义务",
    },
    NonObligationField {
        field: "trait_table",
        reason: "trait 表：跨阶段读取在 src/middle/core/ir_gen.rs:4336/:6867（与 local_var_types 配对传参），文档注释未声明契约",
    },
    NonObligationField {
        field: "warnings",
        reason: "警告诊断通道：与 diagnostics 同类，由 driver 臂按形态门控外发",
    },
    NonObligationField {
        field: "interface_impl_registry",
        reason: "树内事实（3.4.9 用户裁决甲）：在相位消费 src/frontend/core/typecheck/operator_interfaces.rs:623、src/frontend/core/typecheck/inference/expressions.rs:1514 等七处约束求解查询、src/frontend/core/typecheck/checker.rs:214-217 只读访问器；跨阶段零读取（src/middle 0 命中）",
    },
];

/// 一条义务的静态清单（字段名 / 消费者阶段 / 红名单）。
///
/// **单一事实来源**：`scripts/ci/check-obligations.py` 逐条解析本表，与
/// ①`TypeCheckResult` 的字段定义、②02-stage-contract §义务字段表 对账——
/// 三者必须逐字一致，任一漂移门禁即红。
struct ObligationSpec {
    obligation: Obligation,
    /// `TypeCheckResult` 里的字段名（逐字一致）
    field: &'static str,
    /// 义务声明的消费者阶段（02 §义务字段表「消费方」列归并到 `Stage`）
    consumer: Stage,
    /// 红名单条目；`None` = 必须在交付点交付
    red_list: Option<RedListEntry>,
}

/// 义务清单（顺序 = 02 §义务字段表；条数见 [`Obligation::ALL`]）。
const INVENTORY: &[ObligationSpec] = &[
    ObligationSpec {
        obligation: Obligation::ProofCalls,
        field: "proof_calls",
        consumer: Stage::ProofExecution,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::ReleasePlan,
        field: "release_plan",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::EscapedRefs,
        field: "escaped_refs",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::InstantiationRequests,
        field: "instantiation_requests",
        consumer: Stage::Monomorphization,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::ExistentialCoercions,
        field: "existential_coercions",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::ImplementationProofs,
        field: "implementation_proofs",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::SumTypes,
        field: "sum_types",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::SumTypeParamNames,
        field: "sum_type_param_names",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::OperatorDispatches,
        field: "operator_dispatches",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::OverloadResolutions,
        field: "overload_resolutions",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::TryExprImpls,
        field: "try_expr_impls",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::MatchScrutineeTypes,
        field: "match_scrutinee_types",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
    ObligationSpec {
        obligation: Obligation::ModuleNamespaces,
        field: "module_namespaces",
        consumer: Stage::IrGeneration,
        red_list: None,
    },
];

/// IrGeneration 臂在**唯一交付点**交出的义务集
/// （`consumer_stage() == Stage::IrGeneration` 的 14 条，剔除红名单的 3 条
/// ——它们没有可交付点）。漂移由 `test_ir_generation_delivery_set_matches_consumer_stage` 钉住。
pub const IR_GENERATION_DELIVERED: &[Obligation] = &[
    Obligation::ReleasePlan,
    Obligation::EscapedRefs,
    Obligation::ExistentialCoercions,
    Obligation::ImplementationProofs,
    Obligation::SumTypes,
    Obligation::SumTypeParamNames,
    Obligation::OperatorDispatches,
    Obligation::OverloadResolutions,
    Obligation::TryExprImpls,
    Obligation::MatchScrutineeTypes,
    Obligation::ModuleNamespaces,
];

impl Obligation {
    /// 全部 13 个义务（顺序 = 02 §义务字段表；3.4.4/3.4.5/3.4.9 清账后由 16 降为 13）。
    pub const ALL: &'static [Obligation] = &[
        Obligation::ProofCalls,
        Obligation::ReleasePlan,
        Obligation::EscapedRefs,
        Obligation::InstantiationRequests,
        Obligation::ExistentialCoercions,
        Obligation::ImplementationProofs,
        Obligation::SumTypes,
        Obligation::SumTypeParamNames,
        Obligation::OperatorDispatches,
        Obligation::OverloadResolutions,
        Obligation::TryExprImpls,
        Obligation::MatchScrutineeTypes,
        Obligation::ModuleNamespaces,
    ];

    /// 本义务的静态清单条目。
    fn spec(self) -> &'static ObligationSpec {
        INVENTORY
            .iter()
            .find(|s| s.obligation == self)
            .expect("INVENTORY 必须覆盖全部 13 个义务（test_obligation_inventory_covers_every_variant 钉住）")
    }

    /// `TypeCheckResult` 里的字段名（逐字一致；静态门禁据此对账）。
    pub fn field_name(self) -> &'static str {
        self.spec().field
    }

    /// 义务声明的消费者阶段。
    pub fn consumer_stage(self) -> Stage {
        self.spec().consumer
    }

    /// 红名单条目（已核实的零跨阶段消费者字段）；`None` = 必须交付。
    pub fn red_list(self) -> Option<RedListEntry> {
        self.spec().red_list
    }

    /// 该义务在本单元的 `TypeCheckResult` 里是否**非空**（有东西要交付）。
    ///
    /// 空义务不报——这是 02 §2 三档表的第一档，也是运行期唯一能拿到的
    /// 「本次编译是否真的产出了该义务」的事实来源。
    pub fn is_produced(
        self,
        type_result: &TypeCheckResult,
    ) -> bool {
        match self {
            Obligation::ProofCalls => !type_result.proof_calls().is_empty(),
            Obligation::ReleasePlan => !type_result.release_plan.drops.is_empty(),
            Obligation::EscapedRefs => !type_result.escaped_refs.is_empty(),
            Obligation::InstantiationRequests => !type_result.instantiation_requests.is_empty(),
            Obligation::ExistentialCoercions => !type_result.existential_coercions.is_empty(),
            Obligation::ImplementationProofs => !type_result.implementation_proofs.is_empty(),
            Obligation::SumTypes => !type_result.sum_types.is_empty(),
            Obligation::SumTypeParamNames => !type_result.sum_type_param_names.is_empty(),
            Obligation::OperatorDispatches => !type_result.operator_dispatches.is_empty(),
            Obligation::OverloadResolutions => !type_result.overload_resolutions.is_empty(),
            Obligation::TryExprImpls => !type_result.try_expr_impls.is_empty(),
            Obligation::MatchScrutineeTypes => !type_result.match_scrutinee_types.is_empty(),
            Obligation::ModuleNamespaces => !type_result.module_namespaces.is_empty(),
        }
    }
}

/// 交付标记台账（每编译单元一份）：本单元的哪些义务已在**唯一交付点**
/// 交给了它的消费者。
///
/// 标记只记「交出去了」，不记「下游读没读」——后者由静态门禁
/// `scripts/ci/check-obligations.py` 对账（本模块头注释「两半互补」）。
#[derive(Debug, Default, Clone)]
pub(in crate::driver) struct DeliveryLedger {
    marks: Vec<Obligation>,
}

impl DeliveryLedger {
    /// 在交付点标记一条义务已交给消费者（幂等）。
    pub(in crate::driver) fn deliver(
        &mut self,
        obligation: Obligation,
    ) {
        if !self.marks.contains(&obligation) {
            self.marks.push(obligation);
        }
    }

    /// 批量标记——同一交付点交出一组义务（如 IrGeneration 臂把单元
    /// `TypeCheckResult` 交给 IR 生成器，一次交出整组（`IR_GENERATION_DELIVERED`））。
    pub(in crate::driver) fn deliver_all(
        &mut self,
        obligations: &[Obligation],
    ) {
        for obligation in obligations {
            self.deliver(*obligation);
        }
    }

    /// 本单元该义务是否已交付（结算的判定输入）。
    pub(in crate::driver) fn is_delivered(
        &self,
        obligation: Obligation,
    ) -> bool {
        self.marks.contains(&obligation)
    }
}

/// 未排空义务的一条结算发现。
#[derive(Clone, Debug)]
pub struct ObligationFinding {
    /// 义务字段
    pub obligation: Obligation,
    /// 义务所在编译单元（`State.units` 下标）
    pub unit_index: usize,
    /// 单元模块键
    pub unit_key: String,
    /// 单元源路径
    pub path: PathBuf,
    /// 未排空原因
    pub cause: UndrainedCause,
}

impl ObligationFinding {
    /// 归一化键：**跨入口（run / check）诊断集一致性（C2）** 的比对键。
    ///
    /// 只取「哪份文件的哪个义务、属于哪一类原因」——证据细节（WBS id、
    /// 证据指针）不参与比对，因为它们是常量、不随入口变化。
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.path.display(),
            self.obligation.field_name(),
            self.cause.kind()
        )
    }

    /// 构造成用户可见的 W1090 诊断。
    ///
    /// **同码升 E**（2026-10-13 用户裁决甲，WBS 4.3.2 / RFC-039 D5 的终态）：
    /// 码号仍是 `W1090`（不新增 E 码——同一事实两个码即平行表示），发射级别由
    /// **显式 `.severity(Severity::Error)`** 决定（`codes/builder.rs:186-203`：
    /// 显式 severity 优先于「码以 `W` 开头 → Warning」的默认；注册表的
    /// `ErrorCategory` 不参与严重级）。后果如实登记：**一旦发射即改退出码**
    /// （D59 的 D 类破坏项）；当前健康态零发射，故对既有语料与用户程序零影响。
    pub fn to_diagnostic(&self) -> Diagnostic {
        ErrorCodeDefinition::obligation_not_drained(
            self.obligation.field_name(),
            &self.path.display().to_string(),
            &self.cause.detail(),
        )
        .severity(Severity::Error)
        .build()
    }
}

/// 义务未排空的原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndrainedCause {
    /// 消费者阶段已运行，但交付点没有标记（下游没拿到义务）。
    NotDelivered,
    /// 红名单：已核实的零跨阶段消费者字段（清账任务见 `wbs`）。
    RedList {
        /// 负责清账的 WBS 任务 id
        wbs: &'static str,
        /// 「消费者为零」的证据指针
        evidence: &'static str,
    },
}

impl UndrainedCause {
    /// 归一化原因类别（C2 键的一部分）。
    pub fn kind(self) -> &'static str {
        match self {
            UndrainedCause::NotDelivered => "not-delivered",
            UndrainedCause::RedList { .. } => "red-list",
        }
    }

    /// 诊断 detail 参数（人读的补充说明）。
    pub fn detail(self) -> String {
        match self {
            UndrainedCause::NotDelivered => {
                "消费者阶段已运行，但交付点没有标记交付（编译器内部契约缺陷）".to_string()
            }
            UndrainedCause::RedList { wbs, evidence } => {
                format!("红名单（清账任务 WBS {wbs}）：跨阶段消费者为零；证据：{evidence}")
            }
        }
    }
}

/// 一次运行的义务结算结果（`DriverOutcome` 携带；4.3.1）。
///
/// 计数满足守恒式
/// `pairs == empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()`
/// ——「结算不是空转」的可验证证据（`test_obligation_settlement_counters_are_conserved`）。
#[derive(Clone, Debug, Default)]
pub struct ObligationSettlement {
    /// 未排空的义务（空 = 全部通过）
    pub findings: Vec<ObligationFinding>,
    /// 参与结算的 (单元, 义务) 对数 = 有 typecheck 产物的单元数 × 16
    pub pairs: usize,
    /// 第一档：空义务（不报）
    pub empty: usize,
    /// S1：消费者阶段不在本 ProgramKind 阶段表（不判欠）
    pub consumer_stage_absent: usize,
    /// 跳过豁免：消费者阶段因上游失败 / 配置关闭未执行（不判欠）
    pub consumer_stage_skipped: usize,
    /// 已交付（交付标记存在）
    pub delivered: usize,
    /// 本次运行里**真的被排空**的义务种类（非空 + 有交付标记；按枚举序去重）
    /// ——D5 观测期与 4.3.2 逐字段复核的数据出口：没有它，「某义务到底有没有
    /// 被消费过」只能靠读代码推断
    pub drained_kinds: Vec<Obligation>,
    /// 实际外发成 W1090 的条数——`findings` 减去红名单与非消费方单元条目
    /// （嵌入 std 虚拟路径 / vendor 依赖包：与 W1003/W1006 的同则门控，
    /// 见 `emit_obligation_diagnostics`）。审计面始终是完整的 `findings`
    pub emitted: usize,
}

impl ObligationSettlement {
    /// 全部义务已排空（无发现）。
    pub fn is_drained(&self) -> bool {
        self.findings.is_empty()
    }

    /// 归一化键集合（C2 跨入口一致性比对用；升序）。
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.findings.iter().map(|f| f.key()).collect();
        keys.sort();
        keys
    }
}

/// 该单元是否是**消费方可见**的（外发诊断的门控判据）。
///
/// - 嵌入 std 模块是虚拟路径（`<std/list>`）——不存在可修的文件；
/// - vendor 依赖包内部警告不混入消费方（RFC-014 §项目模式，与 W1006/W1003 同则）。
fn consumer_visible_unit(path: &std::path::Path) -> bool {
    if path.to_str().is_some_and(|p| p.starts_with('<')) {
        return false;
    }
    !is_vendor_path(path)
}

/// 消费者阶段是否因「上游失败」或「配置关闭」被跳过（02 §4 三类原因中的两类）。
///
/// `NoObligations` **不**豁免：那一类的语义是「本阶段没有要处理的东西」——
/// 若某义务非空而消费者阶段却自认无义务，正说明账不平，必须报。
fn consumer_stage_skipped(
    skipped: &[SkippedStage],
    stage: Stage,
) -> bool {
    skipped.iter().any(|s| {
        s.stage == stage
            && matches!(
                s.reason,
                SkipReason::UpstreamFailed(_) | SkipReason::ConfigDisabled
            )
    })
}

impl State {
    /// 义务结算（02 §2 三档表 + S1 + 跳过豁免）——**唯一调用点**：
    /// `Driver::run` 阶段表尾部、`into_outcome()` 产出 `CompilationResult` 之前。
    ///
    /// 结算结果写入 `State.obligations`（随 `DriverOutcome` 外发），并按
    /// ProgramKind 走各自的诊断通道：
    /// - Check / Lsp：逐文件收集通道（`UnitState.check_diagnostics`）；
    /// - 其余形态：警告通道（`State.warnings` → `CompilationResult.warnings`）。
    pub(in crate::driver) fn assert_drained(&mut self) {
        let mut settlement = self.settle_obligations();
        settlement.emitted = self.emit_obligation_diagnostics(&settlement);
        self.obligations = settlement;
    }

    /// 结算纯函数：不产诊断、不改通道，只给结论（测试直接调用它钉判据）。
    pub(in crate::driver) fn settle_obligations(&self) -> ObligationSettlement {
        let stages = self.program.stages();
        let mut settlement = ObligationSettlement::default();
        for (unit_index, unit) in self.units.iter().enumerate() {
            // typecheck 未产物的单元（解析失败 / 类型错误 / 上游失败）没有义务
            let Some(type_result) = unit.type_result.as_ref() else {
                continue;
            };
            for &obligation in Obligation::ALL {
                settlement.pairs += 1;
                // 第一档：空义务不报
                if !obligation.is_produced(type_result) {
                    settlement.empty += 1;
                    continue;
                }
                let consumer = obligation.consumer_stage();
                // S1：消费者阶段不在本 ProgramKind 的阶段表 → 不判欠
                if !stages.contains(&consumer) {
                    settlement.consumer_stage_absent += 1;
                    continue;
                }
                // 跳过豁免：上游失败 / 配置关闭导致的跳过不是丢弃
                if consumer_stage_skipped(&self.skipped, consumer) {
                    settlement.consumer_stage_skipped += 1;
                    continue;
                }
                // 红名单：零跨阶段消费者。标记与否都报——「标记 ↔ 真实读取」
                // 的漂移由静态门禁拦（本模块头注释「两半互补」）。
                if let Some(entry) = obligation.red_list() {
                    settlement.findings.push(ObligationFinding {
                        obligation,
                        unit_index,
                        unit_key: self.program.units[unit_index].key.clone(),
                        path: self.program.units[unit_index].path.clone(),
                        cause: UndrainedCause::RedList {
                            wbs: entry.wbs,
                            evidence: entry.evidence,
                        },
                    });
                    continue;
                }
                // 交付标记存在 → 通过
                if unit.delivered.is_delivered(obligation) {
                    settlement.delivered += 1;
                    settlement.drained_kinds.push(obligation);
                    continue;
                }
                settlement.findings.push(ObligationFinding {
                    obligation,
                    unit_index,
                    unit_key: self.program.units[unit_index].key.clone(),
                    path: self.program.units[unit_index].path.clone(),
                    cause: UndrainedCause::NotDelivered,
                });
            }
        }
        settlement.drained_kinds.sort_unstable_by_key(|o| {
            Obligation::ALL
                .iter()
                .position(|candidate| candidate == o)
                .unwrap_or(usize::MAX)
        });
        settlement.drained_kinds.dedup();
        settlement
    }

    /// 把结算发现外发到本形态的诊断通道（W1090）；返回实际外发条数。
    ///
    /// **门控**（与 DeadCodeAnalysis 臂对 W1003/W1006 的同则——「vendor /
    /// 嵌入 std 的内部警告不混入消费方」）：非消费方单元（嵌入 std 的虚拟
    /// 路径 `<std/...>`、vendor 依赖包）不对外发诊断。它们的内容用户改不了，
    /// 逐单元报出来只会淹没真正要清账的红名单。
    /// 门控**不吞事实**：`DriverOutcome.obligations.findings` 始终完整
    /// （审计面），被门控的条数记在 `ObligationSettlement::emitted` 的差值里。
    ///
    /// 红名单条目（[`Obligation::red_list`]）**不外发**（裁定丙，理由见模块头
    /// 「红名单」一节）：它的可见性由审计面 / 静态门禁 / 文档注记三处承担。
    fn emit_obligation_diagnostics(
        &mut self,
        settlement: &ObligationSettlement,
    ) -> usize {
        let per_file = matches!(self.program.kind, ProgramKind::Check | ProgramKind::Lsp);
        let orchestrated = !self.uses_pipeline_channel();
        let mut emitted = 0;
        for finding in &settlement.findings {
            if finding.obligation.red_list().is_some() {
                continue;
            }
            if !consumer_visible_unit(&finding.path) {
                continue;
            }
            let diagnostic = finding.to_diagnostic();
            if per_file {
                if let Some(unit) = self.units.get_mut(finding.unit_index) {
                    unit.check_diagnostics.push(diagnostic);
                    emitted += 1;
                }
                continue;
            }
            if diagnostic.severity == Severity::Error {
                // E 级（4.3.2，D5 终态）：进**错误通道**，触发即改退出码——
                // 单文件形态走 PipelineError::TypeCheck（result 失败面），
                // 多文件/嵌入形态另记 OrchestratorError 故障（compile_project
                // 的 failure 契约；否则 result.errors 无人消费）。
                let stage = finding.obligation.consumer_stage();
                // 首个失败优先：本次运行若已因上游失败（如 zzz.yx 的类型错误）
                // 记过故障，结算**不得覆盖**它——结算只是尾部后置条件，
                // 抢走主因会把真实的首个错误从用户视野里换掉
                // （test_driver_run_multi_file_proof_error_yields_to_typecheck_error）。
                if orchestrated && self.failure.is_none() {
                    self.fail_with(
                        stage,
                        OrchestratorError::TypeCheck {
                            path: finding.path.display().to_string(),
                            message: diagnostic.to_string(),
                            diagnostics: vec![diagnostic.clone()],
                        },
                    );
                }
                self.errors.push(PipelineError::TypeCheck(diagnostic));
                emitted += 1;
                continue;
            }
            self.warnings.push(diagnostic);
            emitted += 1;
        }
        emitted
    }
}
