//! 通用模块系统
//!
//! 提供统一的模块注册、加载和解析接口。
//! 支持 std 标准库模块和用户自定义模块。
//!
//! # 模块结构
//!
//! - [`registry`] - 模块注册表
//! - [`resolver`] - 模块路径解析
//! - [`loader`] - 模块加载器
//!
//! # 设计目标
//!
//! 1. std 模块和用户模块使用统一的接口
//! 2. 支持模块搜索路径和缓存
//! 3. 支持循环依赖检测

pub mod loader;
pub mod registry;
pub mod resolver;

use std::collections::HashMap;

/// 导出项类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    /// 函数导出
    Function,
    /// 常量导出
    Constant,
    /// 子模块导出
    SubModule,
    /// 类型导出
    Type,
}

/// 模块导出项
///
/// 描述一个模块导出的单个符号，包含名称、完整路径、类型和签名信息。
#[derive(Debug, Clone)]
pub struct Export {
    /// 短名称（如 "print"）
    pub name: String,
    /// 完整路径（如 "std.io.print"）
    pub full_path: String,
    /// 导出类型
    pub kind: ExportKind,
    /// 函数签名描述（如 "(value: Any) -> Void"）
    pub signature: String,
}

/// 模块源类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleSource {
    /// 内置标准库模块
    Std,
    /// 用户定义的模块（从文件系统加载）
    User,
    /// 来自 vendor 目录的依赖模块
    Vendor,
}

/// 模块信息
///
/// 表示一个已注册的模块，包含其导出项和元数据。
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// 模块完整路径（如 "std.io"）
    pub path: String,
    /// 模块来源
    pub source: ModuleSource,
    /// 导出项（name -> Export）
    pub exports: HashMap<String, Export>,
    /// 子模块名称列表
    pub submodules: Vec<String>,
}

impl ModuleInfo {
    /// 创建新的模块信息
    pub fn new(
        path: String,
        source: ModuleSource,
    ) -> Self {
        Self {
            path,
            source,
            exports: HashMap::new(),
            submodules: Vec::new(),
        }
    }

    /// 添加导出项
    pub fn add_export(
        &mut self,
        export: Export,
    ) {
        self.exports.insert(export.name.clone(), export);
    }

    /// 添加子模块
    pub fn add_submodule(
        &mut self,
        name: String,
    ) {
        if !self.submodules.contains(&name) {
            self.submodules.push(name);
        }
    }

    /// 获取导出项
    pub fn get_export(
        &self,
        name: &str,
    ) -> Option<&Export> {
        self.exports.get(name)
    }

    /// 检查是否有指定的导出项
    pub fn has_export(
        &self,
        name: &str,
    ) -> bool {
        self.exports.contains_key(name)
    }

    /// 获取所有导出项名称
    pub fn export_names(&self) -> Vec<&str> {
        self.exports.keys().map(|s| s.as_str()).collect()
    }

    /// 检查是否是命名空间模块（只包含子模块）
    pub fn is_namespace(&self) -> bool {
        !self.submodules.is_empty()
            && self
                .exports
                .values()
                .all(|e| e.kind == ExportKind::SubModule)
    }
}

/// 模块系统错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum ModuleError {
    /// 模块未找到
    #[error("module not found: '{path}'")]
    NotFound {
        path: String,
        searched_paths: Vec<String>,
    },

    /// 导出项未找到
    #[error("export '{name}' not found in module '{module_path}'")]
    ExportNotFound {
        name: String,
        module_path: String,
        available: Vec<String>,
    },

    /// 循环依赖
    #[error("cyclic dependency detected: {cycle}")]
    CyclicDependency { cycle: String },

    /// 无效的模块路径
    #[error("invalid module path: '{path}'")]
    InvalidPath { path: String },

    /// 重复导入
    #[error("duplicate import: '{name}'")]
    DuplicateImport { name: String },
}
