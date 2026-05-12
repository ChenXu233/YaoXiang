//! 类型检查结果定义
//!
//! 包含类型检查的返回值和相关类型定义

use std::collections::HashMap;

use crate::frontend::core::types::base::MonoType;
use crate::frontend::core::types::base::PolyType;

use super::semantic_db;

/// 类型检查结果
#[derive(Debug, Clone, Default)]
pub struct TypeCheckResult {
    pub module_name: String,
    pub bindings: HashMap<String, PolyType>,
    /// 局部变量的类型信息（用于 IR 生成器显示错误消息）
    /// Key 是变量名，Value 是推断出的具体类型
    pub local_var_types: HashMap<String, MonoType>,
    /// 语义信息数据库（typecheck 阶段产出）
    pub semantic_db: semantic_db::SemanticDB,
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
