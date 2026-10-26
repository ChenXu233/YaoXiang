//! 通用模块系统
//!
//! 提供统一的模块注册、加载和解析接口。
//! 支持 std 标准库模块和用户自定义模块。
//!
//! # 模块结构
//!
//! - [`registry`] - 模块注册表（拥有项目级符号表）
//! - [`symbol`] - DefId 符号表（绑定身份 + 静态 vtable）
//! - [`resolver`] - 名字解析器（名字 → 限定名 → DefId 的唯一所有者）
//! - [`orchestrator`] - 多文件编译编排
//!
//! # 设计目标
//!
//! 1. std 模块和用户模块使用统一的接口
//! 2. 支持模块搜索路径和缓存
//! 3. 支持循环依赖检测

pub mod consistency;
pub mod orchestrator;
pub mod registry;
pub mod resolver;
pub mod roles;
pub mod symbol;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::frontend::core::typecheck::environment::MethodOverloadCandidates;
use crate::frontend::core::types::mono::MonoType;
use crate::util::span::Span;

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
    /// 真实类型（用户模块由三遍机制提取；native std 为 None，走宽松类型）
    ///
    /// ponytail: std 精度提升是独立后续任务（见后续 issue），native FFI 边界本就丢精度。
    pub mono_type: Option<MonoType>,
    /// 该导出函数的**声明期类型参数名**（按声明序），非泛型为 None。
    ///
    /// 跨模块调用（`list.len(v)`）的单态化需要这些名字：调用方的
    /// `generic_fn_type_params` 只登记本模块的函数，拿不到被调模块的声明名，
    /// 于是签名里的 `TypeRef("A")` 无人绑定、实参无法与 `&Vec(A)` unify（E1002）。
    /// 随导出一并携带可让调用方直接使用**真实名字**，无需从签名形态反推
    /// （反推无法区分类型参数与普通类型名，会误伤 `Dict(K,V)` / `File` 等）。
    ///
    /// 两条来源的层级（谁权威、何时兜底）：
    ///
    /// 1. **权威**：本字段。声明期快照按声明序给出真实名字（std 见
    ///    `crate::std::yx_sources`，用户模块见 `orchestrator::extract_module_info`），
    ///    导入方 `export_type` 据它登记短名与限定名两个键，调用点才拿得到
    ///    类型参数名（`lookup_type_params`）与实例化请求的 `generic_id`。
    /// 2. **兜底**：从签名形态反推——真实落点是 `collect_instantiation_request`
    ///    的第二条分支（`expressions.rs` 里 `let mut type_var_indices = …` 那段
    ///    位置直扫：只收「参数位直接是 TypeVar」的位），底层取变量用
    ///    `collect_type_vars_positional`。只在第 1 条拿不到名字时生效，且
    ///    **收不全**——类型参数只出现在嵌套/借用位置（`x: &T`、`Vec(T)`）时
    ///    位置直扫什么也取不到（形参位是 `Ref(TypeRef)` 而非 `TypeVar`），
    ///    故它不能替代本字段，只能服务老路径。
    pub type_params: Option<Vec<String>>,
    /// 该导出函数的**声明期形参名**（按声明序），无参或未知为 None。
    ///
    /// 命名参数调用（`list.push(item = 9, list = v)`）必须按**名字**把实参
    /// 重排到声明序。定义方的 `fn_param_names` 只在本模块的 IR 生成器实例里，
    /// 调用方拿不到（`FunctionIR.params` 只有类型没有名字）——所以随导出携带。
    pub param_names: Option<Vec<String>>,
    /// 类型定义导出载荷（仅 `ExportKind::Type`）：泛型模板 + 和类型变体表 +
    /// 接口实现登记。
    ///
    /// 类型导出曾只携带 `mono_type` 快照——导入方 typecheck 环境的三张表
    /// （`generic_type_defs` / `sum_types` / `interface_impl_registry`）全部落空，
    /// 变体构造、match 变体解构在导入方即失败。载荷随导出传播，导入方
    /// `register_use_export` 据此写入同构的三张表。
    pub type_payload: Option<TypeDefPayload>,
    /// 该导出在**定义文件**中的名源位置（WBS 4.6.1）：用户/embedded-yx 模块
    /// 由签名收集（`orchestrator::extract_module_info`）从 AST 标识符节点填
    /// `Some`，与 typecheck 定义事件（`VarInfo.definition_span`）同源同值；
    /// native std 导出无源码位置，为 `None`——导入方跨文件 `resolves_to`
    /// 据此回退到 use 行名源位置（现状语义）。
    pub definition_span: Option<Span>,
}

/// 类型定义导出载荷（见 [`Export::type_payload`]）。
#[derive(Debug, Clone)]
pub struct TypeDefPayload {
    /// 泛型模板（声明序类型参数名 + poly），非泛型类型为 None
    pub type_def: Option<crate::frontend::core::typecheck::environment::GenericTypeDef>,
    /// 和类型变体（声明序即 variant_id）；非和类型为空
    pub sum_variants: Vec<crate::frontend::core::typecheck::environment::SumVariantDef>,
    /// 类型体内接口应用（`Try(Result(T, E), T, E)` / `Equal` 等）：
    /// （接口名，实现登记）列表。实参引用声明类型参数的抽象条目原样携带
    /// （与定义方待决队列同源），具体条目与 finalize 产物同构。
    pub interface_impls: Vec<(
        String,
        crate::frontend::core::typecheck::environment::InterfaceImplEntry,
    )>,
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
    /// 方法绑定（"Type.method" -> 函数类型）。
    /// RFC-029：跨文件方法调用解析——导入一个类型时也带来它的方法签名。
    pub method_bindings: HashMap<String, MonoType>,
    /// 方法**重载候选**表（"Type.method" -> 候选组，定义侧声明序）。
    /// RFC-029f 改进 4 / WBS 4.9.7（issue #464）：`method_bindings` 是单值表
    /// （同名多定义互相覆盖），跨单元的方法调用决议需要**逐候选的有序表**；
    /// 本字段是它跨单元通路的载体（模块键 → 候选组的选择性合并见
    /// [`registry::ModuleRegistry::merge_method_overloads`]）。
    ///
    /// 由签名收集一趟产出（`orchestrator::extract_module_info`），与
    /// `method_bindings` 同一处导出；std 模块的构造路径不填此表（候选面
    /// 当前只覆盖用户单元，WBS 4.9.7 的登记边界）。
    pub method_overloads: HashMap<String, MethodOverloadCandidates>,
    /// **纯 yx 实现**的导出名集（RFC-036 §4）：该模块里由嵌入 YaoXiang 源
    /// （`crate::std::yx_sources` 的 `STD_YX_FILES`）**定义**的导出。
    ///
    /// native 导出不进本集合，且一个模块可能两者兼有——`std.result` 就是
    /// native（`is_ok`/`unwrap` 等工具族）与纯 yx（`Result` 类型 + Try 方法）
    /// 的混合面。因此「导出是不是代码」只能**按名字逐项**问（#493），模块
    /// 粒度或 `ModuleSource` 粒度都会判错：嵌入模块同样带 `ModuleSource::Std`。
    ///
    /// 写入点唯一：`registry::ModuleRegistry::register_std_modules` 在把嵌入
    /// 模块导出并入表面时逐个标记；读取点唯一：`ModuleRegistry::is_yx_std_export`。
    yx_defined_exports: HashSet<String>,
    /// 子模块名称列表
    pub submodules: Vec<String>,
    /// 模块的**定义文件**（WBS 4.6.1）：Registry 臂按 `DiscoveredFile` 填
    /// `Some`（含 vendor 依赖——vendor 文件同样是磁盘真实文件）；
    /// std 虚拟模块为 `None`。跨文件 `resolves_to` 的文件身份与 4.9.4 的
    /// use 项路径解析都以此为唯一来源。
    pub definition_file: Option<PathBuf>,
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
            yx_defined_exports: HashSet::new(),
            method_bindings: HashMap::new(),
            method_overloads: HashMap::new(),
            submodules: Vec::new(),
            definition_file: None,
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

    /// 登记一个**纯 yx 实现**的导出名（见 `ModuleInfo` 的 `yx_defined_exports`）。
    ///
    /// 只应由嵌入 std 模块的注册路径调用——集合的语义是「定义来自嵌入 yx 源」，
    /// 不是「我猜它是函数」。
    pub fn mark_yx_defined_export(
        &mut self,
        name: &str,
    ) {
        self.yx_defined_exports.insert(name.to_string());
    }

    /// 该导出是否**纯 yx 实现**（导出粒度；native 导出返回 false）。
    pub fn is_yx_defined_export(
        &self,
        name: &str,
    ) -> bool {
        self.yx_defined_exports.contains(name)
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
