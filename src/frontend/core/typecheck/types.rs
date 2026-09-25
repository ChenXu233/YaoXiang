//! 类型检查结果定义
//!
//! 包含类型检查的返回值和相关类型定义

use std::collections::{HashMap, HashSet};

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::PolyType;
use crate::frontend::core::types::TraitTable;

use super::semantic_db;
use super::proof::verdict::ProofFunctionCall;

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
    pub proof_calls: Vec<ProofFunctionCall>,
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
    /// 运算符查询与约束求解的唯一判据；镜像供 middle 端/LSP 消费。
    pub interface_impl_registry: HashMap<String, Vec<super::environment::InterfaceImplEntry>>,
    /// RFC-010 和类型登记表（类型名 → 变体定义，声明序）
    pub sum_types: HashMap<String, Vec<super::environment::SumVariantDef>>,
    /// RFC-010 变体构造调用点（span 键控；ir_gen 生成 CreateVariant）
    pub variant_ctor_calls: Vec<super::environment::VariantCtorCall>,
    /// RFC-011b 运算符派发点（span 键控；ir_gen 据此把原生指令换为方法调用）
    pub operator_dispatches: Vec<super::operator_interfaces::OperatorDispatch>,
    /// RFC-011b `?` 表达式的 Try 实现类型名（span 键控；ir_gen 据此命名
    /// 四方法调用链——接收者是任意表达式时类型名别无来处）
    pub try_expr_impls: Vec<(crate::util::span::Span, String)>,
    /// 用户模块命名空间别名表（别名 → 模块限定键）。
    /// 模块解析归 typecheck 所有：由整体导入（`use lib` / `use lib as l`）登记，IR 生成直接消费。
    pub module_namespaces: HashMap<String, String>,
    /// 警告诊断（#321：未使用导入 W1003 等，Warning 级、不阻断编译）。
    /// 与 diagnostics 分离——混入会被管线按错误计数，破坏非阻断契约。
    pub warnings: Vec<crate::util::diagnostic::Diagnostic>,
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
