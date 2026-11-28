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
    pub release_plan: crate::frontend::core::sema::layers::ownership::ReleasePlan,
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
    /// （operator_interfaces.rs / inference/expressions/expr.rs），跨阶段零读取
    /// （src/middle 0 命中，src/lsp 0 命中）。3.4.9 用户裁决甲改判：将来 LSP
    /// 真需要该表时按 WBS 4.6.x 新登记，此处不再声明镜像。
    pub interface_impl_registry: HashMap<String, Vec<super::environment::InterfaceImplEntry>>,
    /// RFC-010 和类型登记表（类型名 → 变体定义，声明序）
    pub sum_types: HashMap<String, Vec<super::environment::SumVariantDef>>,
    /// RFC-010 和类型的类型参数名（类型名 → [T, E]，声明序）——ir_gen
    /// 载荷类型替换用（变体 params 里的 TypeRef(名) 按位绑定 scrutinee 实参）
    pub sum_type_param_names: HashMap<String, Vec<String>>,
    /// 类型身份 → 本地键（[`crate::frontend::core::sema::TypeEnvironment`]
    /// 派生索引的只读快照；唯一写者是 `TypeEnvironment::add_type`）。
    ///
    /// 跨阶段消费者（lower）据它把「本地键表」投影成**身份可查**的视图：
    /// 同一编译集里的同名类型（a.T / b.T）因此各按自己的身份取字段/变体，
    /// 不从名字串反解模块（D64；§2.5 执行口径「身份不得从名字串反解」）。
    pub type_identity_index: HashMap<crate::frontend::core::types::identity::Nominal, String>,
    /// RFC-011b 运算符派发点（span 键控；ir_gen 据此把原生指令换为方法调用）
    pub operator_dispatches: Vec<super::operator_interfaces::OperatorDispatch>,
    /// RFC-011a §3：重载决议表（**调用点 `PlanId` 键控**，RFC-039 D20/WBS 4.5.2；
    /// ir_gen 据此把方法调用命名为决议出的混编名——同名多候选时裸名会撞函数表）
    pub overload_resolutions: OverloadResolutions,
    /// RFC-011b `?` 表达式的 Try 结算结果（span 键控）：实现**类型身份**（module +
    /// 定义名；跨模块同名 Try 类型的四方法调用名按定义模块限定，单文件投影为裸名，
    /// D64 显示口径）+ 成功载荷 + 残余载荷。三样同出一次结算：ir_gen 据此命名四方法
    /// 并落它们的结果类型（接收者是任意表达式时，这几样都别无来处）。
    pub try_expr_impls: super::operator_interfaces::TryExprImpls,
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
    /// 依赖记录 Σ 的记录模型（类型名 → 模型；typecheck 声明期产出 → IR 生成消费）。
    /// B.2 起生产路径产出 witness 字段，IR 的 `TypeDeclField.quantity` 与构造
    /// 参数/默认值填充都从这张表读量词；消费端不得自行重算分类。
    pub record_models: HashMap<String, super::record_model::RecordModel>,
    /// 方法定义的混编 IR 名（RFC-011a §3 / WBS 7c.3）。
    ///
    /// 键 = 方法声明语句的 [`PlanId`]（AST 结构身份；`DefId` 在 lowering 尾部
    /// `assign_defs` 才分配，typecheck 侧不存在）。typecheck 在登记重载候选的
    /// 同一处产出裸名或 `#N` 混编名（与 `overload_resolutions` 的候选名同源），
    /// IR 生成按声明身份只读取名，不再按定义遍历顺序二次推导。
    ///
    /// 消费缺失即 `E3005`（`lower/stmt.rs` 的 `generate_method_ir`），无静默回退。
    pub method_def_ir_names: HashMap<PlanId, String>,
    /// 调用点结果类型（**调用点 `PlanId` 键控**，RFC-039 D20；ir_gen 的
    /// `expr_value_type` 按同一节点的同一身份回查）。
    ///
    /// 唯一产出点是 `infer_call_expr` 的返回处（`inference/expressions/expr.rs`
    /// 的 Call 臂）：限定名 / native / 方法 / 泛型调用的**最终**类型只在 sema
    /// 手里，IR 侧按被调名反查原生签名取不到，值表遂落显式未知 `TypeRef("?")`。
    /// 落表前先 zonk；仍含推断变量（`TypeVar`）或身份未分配
    /// （[`PlanId::UNPLANNED`]）的条目不落——IR 宁可显式未知，也不写未收敛变量。
    pub expr_result_types: HashMap<PlanId, MonoType>,
}

/// 一次方法调用重载决议的**载荷**（WBS 4.9.10 D-3）。
///
/// 两个字段是消费端的两件不同的事，且都由 typecheck 唯一产出：
/// - `target`：**IR 函数表里真实存在的函数名**（D-8：多文件 `{模块键}.{Type}.{method}`
///   （同名重载按声明序加 `#序`），单文件裸名）——IR 生成直接用它当被调名，
///   不再按短名二次查表；
/// - `param_names`：**胜出候选**的声明期形参名（接收者居首，与候选签名同序）。
///   命名实参的按名入槽以它为准：产出点按方法身份（[`DefPath`]）直查候选组取得，
///   消费端因此既不用反解名字、也不再有第二份名字表（WBS 4.9.10 删 `method_param_info`）。
///   空表 = 声明期名字不可用（候选与形参名对不齐）——消费端不得把空表冒充「名字已核对」。
///
/// [`DefPath`]: crate::frontend::core::types::identity::DefPath
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MethodResolution {
    /// IR 最终函数名（D-8）
    pub target: String,
    /// 胜出候选的声明期形参名（空表 = 不可用）
    pub param_names: Vec<String>,
}

impl MethodResolution {
    /// 只有目标名、声明期形参名不可得——仅测试夹具与「无候选名来源」用；
    /// 生产路径一律给出两者（见 `infer_call_expr` 的产出点）。
    pub fn target_only(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            param_names: Vec::new(),
        }
    }
}

/// 方法级重载决议表（RFC-011a §3 / RFC-039 D20 / WBS 4.5.2）。
///
/// 键 = **调用表达式节点的 [`PlanId`]**：产出点（`inference/expressions/call/mod.rs` 的
/// `infer_call_expr`）与消费点（`lower/expr/call.rs` 的 `generate_call_expr_ir` 两处：
/// 有绑定 / 无绑定，经 [`Self::get`]）读的是同一个调用节点上的
/// 同一个值——不再靠「Call 的 span 两处各算一遍」的位置巧合。
///
/// 消费经 [`Self::get`]（命中即记账，台账与表同体）；结算读
/// [`Self::unconsumed`]：非空即响亮（义务账本 W1090）。未分配身份
/// （[`PlanId::UNPLANNED`]，解析器之外的合成调用）**不可被消费**——任何
/// 「合成调用进了重载决议」都会以未消费条目在结算处响亮，不会静默落回
/// 未混编名。
#[derive(Debug, Clone, Default)]
pub struct OverloadResolutions {
    /// 产出：调用点身份 → 决议载荷（最终函数名 + 声明期形参名）
    entries: HashMap<PlanId, MethodResolution>,
    /// 消费台账（与表同体；4.5.1 释放计划共用同一实现）
    consumption: ConsumptionLedger<PlanId>,
}

impl OverloadResolutions {
    /// 产出：登记一次重载决议（typecheck 侧唯一写入点）。
    pub fn resolve(
        &mut self,
        call: PlanId,
        resolution: MethodResolution,
    ) {
        self.entries.insert(call, resolution);
    }

    /// 由 (调用点, 载荷) 序列装配（checker 组装 `TypeCheckResult` 时用）。
    pub fn from_entries(entries: impl IntoIterator<Item = (PlanId, MethodResolution)>) -> Self {
        let mut table = Self::default();
        for (call, resolution) in entries {
            table.resolve(call, resolution);
        }
        table
    }

    /// **唯一消费入口**：按调用点身份取决议载荷，命中即记账。
    ///
    /// 未分配身份（合成调用）一律 `None`：不消费、也不冒充消费——它若真的
    /// 进了决议表，结算会把它作为未消费条目报出来（响亮）。
    pub fn get(
        &self,
        call: PlanId,
    ) -> Option<&MethodResolution> {
        if !call.is_planned() {
            return None;
        }
        let resolution = self.entries.get(&call)?;
        self.consumption.mark(call);
        Some(resolution)
    }

    /// **只读**枚举决议条目 `(调用点, 载荷)`——不触达 `get`、不计入消费台账
    /// （WBS 4.9.6 / #450 逐候选归因的读取面；纯读保证不掩盖「消费必达」结算）。
    pub fn iter(&self) -> impl Iterator<Item = (&PlanId, &MethodResolution)> {
        self.entries.iter()
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

    /// 消费台账（**标记与结算的唯一入口**：`mark_completed` 标记、
    /// `is_completed` 读取「消费者真的跑过」判据，见 [`ConsumptionLedger`] 文档）。
    ///
    /// #460：此前 `mark_consumption_completed` / `consumption_completed` 两个
    /// 转发在本类型与 `ReleasePlan` 各抄一份（禁令三 A），现收敛到
    /// [`ConsumptionLedger`] 单一入口。
    pub fn consumption(&self) -> &ConsumptionLedger<PlanId> {
        &self.consumption
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
