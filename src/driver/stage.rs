//! 编译阶段枚举 —— 02-stage-contract §目标设计 1 / RFC-039 L1 编排层。
//!
//! `Stage` 是**编译期可穷举**的枚举，禁止运行期注册：`Driver::dispatch`
//! 对它的穷尽 `match` 让「新增阶段必须被编排层处理」成为编译期事实
//! （与 opcode 表同构的强制机制，RFC-039 路由表 B）。
//!
//! `Stage::ALL` 按**真实数据依赖的拓扑序**排列：
//! `Monomorphization` 消费 IR 产物（`Monomorphizer::monomorphize(&ir, …)`），
//! 故排在 `IrGeneration` **之后**——02 文档阶段表初稿把这一次序写反，
//! 实施时按数据流修正（02 §目标设计 1 修订注记）。

/// 编译阶段（12 变体）。
///
/// 变体含义与各入口的实际接线现状见 02-stage-contract §现状
/// 「六条编译路径、十个入口函数」与「11 处阶段覆盖不一致」。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    /// 供应商一致性核对（RFC-014 §项目模式：vendor 与 yaoxiang.lock 一致性）
    VendorConsistency,
    /// 文件发现（项目根扫描 .yx，产出编译单元集合）
    Discovery,
    /// 模块注册表构建（签名收集 → ModuleRegistry）
    Registry,
    /// 角色分类（Script/Bin/Lib/Test/Internal，RFC-029f）
    RoleClassification,
    /// 词法 + 语法分析（lex → parse）
    Parsing,
    /// 类型检查（含内嵌证明层：termination/ownership/predicate）
    Typecheck,
    /// 死代码族分析（W1001/W1002 + W1003 未使用导入 + W1006 模块遮蔽）
    DeadCodeAnalysis,
    /// 证明函数编译期执行（RFC-027 Phase 2.5）
    ProofExecution,
    /// 全局槽位分配（跨文件不相交槽位区间，T5）
    GlobalSlotAlloc,
    /// AST → ModuleIR（含单文件路径的嵌入 std 合并）
    IrGeneration,
    /// 单态化（消费 IR 产物与聚合的实例化请求）
    Monomorphization,
    /// 跨模块链接 / IR 合并（含入口 main 校验，#388）
    Linking,
}

/// 阶段作用域：逐编译单元执行，还是项目级恰好执行一次。
///
/// `Project` 作用域的阶段在 `dispatch` 中被结构性地保证只跑一次——
/// 「这个阶段该不该每个文件调一次」不再需要人推理（02 §目标设计 1）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum StageScope {
    /// 逐编译单元（模块）执行一次
    PerModule,
    /// 项目级执行一次
    Project,
}

impl Stage {
    /// 全部阶段，按数据依赖拓扑序。
    ///
    /// 新增变体时必须同步本数组——`test_stage_all_covers_every_variant`
    /// 与 `Program::stages()` 的覆盖断言共同保证不漏。
    pub const ALL: &'static [Stage] = &[
        Stage::VendorConsistency,
        Stage::Discovery,
        Stage::Registry,
        Stage::RoleClassification,
        Stage::Parsing,
        Stage::Typecheck,
        Stage::DeadCodeAnalysis,
        Stage::ProofExecution,
        Stage::GlobalSlotAlloc,
        Stage::IrGeneration,
        Stage::Monomorphization,
        Stage::Linking,
    ];

    /// 本阶段的作用域（02 §目标设计 1 表格）。
    pub fn scope(self) -> StageScope {
        match self {
            Stage::VendorConsistency
            | Stage::Discovery
            | Stage::Registry
            | Stage::RoleClassification
            | Stage::DeadCodeAnalysis
            | Stage::GlobalSlotAlloc
            | Stage::Monomorphization
            | Stage::Linking => StageScope::Project,
            Stage::Parsing | Stage::Typecheck | Stage::ProofExecution | Stage::IrGeneration => {
                StageScope::PerModule
            }
        }
    }

    /// 诊断文案用的短名（Skipped 诊断：「阶段 Y 因上游阶段 X 失败未执行」）。
    pub fn slug(self) -> &'static str {
        match self {
            Stage::VendorConsistency => "vendor-consistency",
            Stage::Discovery => "discovery",
            Stage::Registry => "registry",
            Stage::RoleClassification => "role-classification",
            Stage::Parsing => "parsing",
            Stage::Typecheck => "typecheck",
            Stage::DeadCodeAnalysis => "dead-code-analysis",
            Stage::ProofExecution => "proof-execution",
            Stage::GlobalSlotAlloc => "global-slot-alloc",
            Stage::IrGeneration => "ir-generation",
            Stage::Monomorphization => "monomorphization",
            Stage::Linking => "linking",
        }
    }
}
