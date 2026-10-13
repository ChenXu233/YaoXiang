//! 类型检查结果定义
//!
//! 包含类型检查的返回值和相关类型定义

use std::collections::{HashMap, HashSet};

// 顶部 `use` 行数**必须净零**：02 §义务字段表把字段声明行钉成锚点（门禁 F4 逐行对账）。
use crate::frontend::core::types::{MonoType, PolyType, TraitTable};
use crate::util::consumption::ConsumptionLedger;
use crate::util::plan_id::PlanId;

use super::{proof::verdict::ProofFunctionCall, semantic_db};

/// 类型检查结果
#[derive(Debug, Clone, Default)]
pub struct TypeCheckResult {
    pub module_name: String,
    /// 诊断信息（空 = 无错误）
    pub diagnostics: Vec<crate::util::diagnostic::Diagnostic>,
    pub bindings: HashMap<String, PolyType>,
    /// 局部变量的类型信息（用于 IR 生成器显示错误消息）
    /// Key 是变量名，Value 是推断出的具体类型
    pub local_var_types: HashMap<String, MonoType>,
    /// 语义信息数据库（typecheck 阶段产出）
    pub semantic_db: semantic_db::SemanticDB,
    /// Trait 表（用于 IR 生成阶段查询类型是否实现特定 trait）
    pub trait_table: TraitTable,
    /// 证明函数调用（RFC-027 Phase 2.5: 需要在编译期执行的证明函数）
    ///
    /// 读取必须经 [`Self::proof_calls`] getter——消费端盘点与 P4 Obligations
    /// 迁移的收口点（02-stage-contract §义务字段，WBS 3.1.1）。
    /// 可见性 pub(super)：typecheck 树内（checker/layers/tests）共享构造权
    /// （同一生杀圈），crate 外只读 getter——#434 裁决 B 消解（pub(crate)
    /// 泄漏计数回落），全私有需改写 11 处 struct 字面量（Rust 字面量要求
    /// 含 base 填充在内的全字段可见），churn 与收益不匹配。
    pub(super) proof_calls: Vec<ProofFunctionCall>,
    /// NLL 精确释放计划（所有权检查阶段产出 → IR 生成阶段消费）
    pub release_plan: crate::frontend::core::typecheck::layers::ownership::ReleasePlan,
    /// ref 逃逸分析结果（跨 spawn 使用的 ref 变量 → 选 Arc）
    pub escaped_refs: HashSet<String>,
    /// 实例化请求列表（单态化器使用）
    pub instantiation_requests: Vec<crate::middle::passes::mono::instance::InstantiationRequest>,
    /// RFC-011a §6 存在类型强制点（具体→存在包装点，ir_gen 按 span 注入 CreateVariant）
    pub existential_coercions: Vec<super::inference::existential::ExistentialCoercion>,
    /// RFC-011a §5.3 实现证明（编译期擦除；阶段3 ir_gen 由此得每接口的变体集合）
    pub implementation_proofs: Vec<super::environment::ImplementationProof>,
    /// RFC-011b 接口实现登记表（接口名 → 带类型实参的实例化条目）。
    /// 运算符查询与约束求解的唯一判据——**树内事实**：在相位消费
    /// （operator_interfaces.rs / inference/expressions.rs），跨阶段零读取
    /// （src/middle 0 命中，src/lsp 0 命中）。3.4.9 用户裁决甲改判：将来 LSP
    /// 真需要该表时按 WBS 4.6.x 新登记，此处不再声明镜像。
    pub interface_impl_registry: HashMap<String, Vec<super::environment::InterfaceImplEntry>>,
    /// RFC-010 和类型登记表（类型名 → 变体定义，声明序）
    pub sum_types: HashMap<String, Vec<super::environment::SumVariantDef>>,
    /// RFC-010 和类型的类型参数名（类型名 → [T, E]，声明序）——ir_gen
    /// 载荷类型替换用（变体 params 里的 TypeRef(名) 按位绑定 scrutinee 实参）
    pub sum_type_param_names: HashMap<String, Vec<String>>,
    /// RFC-011b 运算符派发点（span 键控；ir_gen 据此把原生指令换为方法调用）
    pub operator_dispatches: Vec<super::operator_interfaces::OperatorDispatch>,
    /// RFC-011a §3：重载决议表（**调用点 `PlanId` 键控**，RFC-039 D20/WBS 4.5.2；
    /// ir_gen 据此把方法调用命名为决议出的混编名——同名多候选时裸名会撞函数表）
    pub overload_resolutions: OverloadResolutions,
    /// RFC-011b `?` 表达式的 Try 实现类型名（span 键控；ir_gen 据此命名
    /// 四方法调用链——接收者是任意表达式时类型名别无来处）
    pub try_expr_impls: Vec<(crate::util::span::Span, String)>,
    /// #389 match scrutinee 推断类型（match 节点 span 键控；ir_gen 的
    /// generate_match_expr_ir 按 span 回查——AST 猜测器对调用/内联构造/
    /// 字段访问形态返回 None，Union 模式会被 #330 安全网误拦 E3008）
    pub match_scrutinee_types: HashMap<crate::util::span::Span, MonoType>,
    /// 用户模块命名空间别名表（别名 → 模块限定键）。
    /// 模块解析归 typecheck 所有：由整体导入（`use lib` / `use lib as l`）登记，IR 生成直接消费。
    pub module_namespaces: HashMap<String, String>,
    /// 警告诊断（#321：未使用导入 W1003 等，Warning 级、不阻断编译）。
    /// 与 diagnostics 分离——混入会被管线按错误计数，破坏非阻断契约。
    pub warnings: Vec<crate::util::diagnostic::Diagnostic>,
}

/// 方法级重载决议表（RFC-011a §3 / RFC-039 D20 / WBS 4.5.2）。
///
/// 键 = **调用表达式节点的 [`PlanId`]**：决议产出点（`inference/expressions.rs`
/// 的 `infer_call_expr`）与消费点（`ir_gen` 的两处方法调用命名）读的是同一个
/// 调用节点上的同一个值——不再靠「Call 的 span 两处各算一遍」的位置巧合。
///
/// 消费经 [`Self::get`]（命中即记账，台账与表同体）；结算读
/// [`Self::unconsumed`]：非空即响亮（义务账本 W1090）。未分配身份
/// （[`PlanId::UNPLANNED`]，解析器之外的合成调用）**不可被消费**——任何
/// 「合成调用进了重载决议」都会以未消费条目在结算处响亮，不会静默落回
/// 未混编名。
#[derive(Debug, Clone, Default)]
pub struct OverloadResolutions {
    /// 产出：调用点身份 → 决议出的混编 IR 名
    entries: HashMap<PlanId, String>,
    /// 消费台账（与表同体；4.5.1 释放计划共用同一实现）
    consumption: ConsumptionLedger<PlanId>,
}

impl OverloadResolutions {
    /// 产出：登记一次重载决议（typecheck 侧唯一写入点）。
    pub fn resolve(
        &mut self,
        call: PlanId,
        ir_name: String,
    ) {
        self.entries.insert(call, ir_name);
    }

    /// 由 (调用点, 混编名) 序列装配（checker 组装 `TypeCheckResult` 时用）。
    pub fn from_entries(entries: impl IntoIterator<Item = (PlanId, String)>) -> Self {
        let mut table = Self::default();
        for (call, ir_name) in entries {
            table.resolve(call, ir_name);
        }
        table
    }

    /// **唯一消费入口**：按调用点身份取混编 IR 名，命中即记账。
    ///
    /// 未分配身份（合成调用）一律 `None`：不消费、也不冒充消费——它若真的
    /// 进了决议表，结算会把它作为未消费条目报出来（响亮）。
    pub fn get(
        &self,
        call: PlanId,
    ) -> Option<&str> {
        if !call.is_planned() {
            return None;
        }
        let ir_name = self.entries.get(&call)?;
        self.consumption.mark(call);
        Some(ir_name.as_str())
    }

    /// 未消费的产出键（**结算判据：必须为空**；升序，诊断可比）。
    pub fn unconsumed(&self) -> Vec<PlanId> {
        let mut ids = self.consumption.unconsumed(self.entries.keys().copied());
        ids.sort_unstable();
        ids
    }

    /// 是否没有决议条目（空义务不报的判据）。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 决议条目数（审计面）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 标记消费侧已完整跑完一轮（IR 生成成功返回时调用；幂等）。
    pub fn mark_consumption_completed(&self) {
        self.consumption.mark_completed();
    }

    /// 消费侧是否完整跑完一轮（IR 生成成功返回）。
    pub fn consumption_completed(&self) -> bool {
        self.consumption.is_completed()
    }

    /// 已记账的消费条数（审计面计数）。
    pub fn consumed_len(&self) -> usize {
        self.consumption.len()
    }
}

impl TypeCheckResult {
    /// 读取待执行的证明函数义务（RFC-027 Phase 2.5）。
    ///
    /// 唯一对外读取路径：pipeline.rs 与 orchestrator 各入口的消费端经此取得
    /// 义务并执行（02-stage-contract §正确性漏洞——义务产出后必须有人消费）。
    pub fn proof_calls(&self) -> &[ProofFunctionCall] {
        &self.proof_calls
    }
}

/// 导入信息
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// 导入路径（如 "std.io"）
    pub path: String,
    /// 导入的具体项（如 ["print", "println"]），None 表示全部
    pub items: Option<Vec<String>>,
    /// 模块别名
    pub alias: Option<String>,
}
