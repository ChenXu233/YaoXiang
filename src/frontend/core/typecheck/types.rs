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
