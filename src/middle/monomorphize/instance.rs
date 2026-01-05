//! 实例化请求与缓存键定义
//!
//! 单态化过程中的工作单元定义

use crate::frontend::typecheck::MonoType;
use crate::util::span::Span;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// 实例化请求
///
/// 表示对某个泛型函数应用特定类型参数的实例化需求
#[derive(Debug, Clone)]
pub struct InstantiationRequest {
    /// 泛型函数ID
    pub generic_id: GenericFunctionId,

    /// 类型参数列表
    pub type_args: Vec<MonoType>,

    /// 实例化来源（用于调试和追踪）
    pub source_location: Span,
}

impl InstantiationRequest {
    /// 创建新的实例化请求
    pub fn new(
        generic_id: GenericFunctionId,
        type_args: Vec<MonoType>,
        source_location: Span,
    ) -> Self {
        InstantiationRequest {
            generic_id,
            type_args,
            source_location,
        }
    }

    /// 获取泛型函数ID
    pub fn generic_id(&self) -> &GenericFunctionId {
        &self.generic_id
    }

    /// 获取类型参数列表
    pub fn type_args(&self) -> &[MonoType] {
        &self.type_args
    }

    /// 生成缓存键
    pub fn specialization_key(&self) -> SpecializationKey {
        SpecializationKey::new(self.generic_id.name.clone(), self.type_args.clone())
    }
}

/// 特化缓存键
///
/// 用于在缓存中唯一标识一个特化版本
#[derive(Debug, Clone)]
pub struct SpecializationKey {
    /// 函数/类型名称
    pub name: String,
    /// 类型参数
    pub type_args: Vec<MonoType>,
}

impl SpecializationKey {
    /// 创建新的缓存键
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
    ) -> Self {
        SpecializationKey { name, type_args }
    }

    /// 生成字符串键
    pub fn as_string(&self) -> String {
        if self.type_args.is_empty() {
            self.name.clone()
        } else {
            let args_str = self
                .type_args
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join(",");
            format!("{}<{}>", self.name, args_str)
        }
    }
}

impl fmt::Display for SpecializationKey {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl PartialEq for SpecializationKey {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.name == other.name && self.type_args == other.type_args
    }
}

impl Eq for SpecializationKey {}

impl Hash for SpecializationKey {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for ty in &self.type_args {
            self.type_name_hash(ty, state);
        }
    }
}

impl SpecializationKey {
    /// 辅助函数：计算类型名称的哈希值
    #[allow(clippy::only_used_in_recursion)]
    fn type_name_hash<H: Hasher>(
        &self,
        ty: &MonoType,
        state: &mut H,
    ) {
        match ty {
            MonoType::Void => "void".hash(state),
            MonoType::Bool => "bool".hash(state),
            MonoType::Int(n) => format!("int{}", n).hash(state),
            MonoType::Float(n) => format!("float{}", n).hash(state),
            MonoType::Char => "char".hash(state),
            MonoType::String => "string".hash(state),
            MonoType::Bytes => "bytes".hash(state),
            MonoType::Struct(s) => s.name.hash(state),
            MonoType::Enum(e) => e.name.hash(state),
            MonoType::Tuple(ts) => {
                "tuple".hash(state);
                for t in ts {
                    self.type_name_hash(t, state);
                }
            },
            MonoType::List(t) => {
                "list".hash(state);
                self.type_name_hash(t, state);
            },
            MonoType::Dict(k, v) => {
                "dict".hash(state);
                self.type_name_hash(k, state);
                self.type_name_hash(v, state);
            },
            MonoType::Set(t) => {
                "set".hash(state);
                self.type_name_hash(t, state);
            },
            MonoType::Fn { .. } => "fn".hash(state),
            MonoType::Range { elem_type } => {
                "range".hash(state);
                self.type_name_hash(elem_type, state);
            },
            MonoType::TypeVar(v) => format!("var{}", v.index()).hash(state),
            MonoType::TypeRef(n) => n.hash(state),
        }
    }
}

/// 泛型函数ID
///
/// 用于唯一标识一个泛型函数
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericFunctionId {
    /// 函数名称
    name: String,
    /// 泛型参数列表（用于区分重载的泛型函数）
    type_params: Vec<String>,
}

impl GenericFunctionId {
    /// 创建新的泛型函数ID
    pub fn new(
        name: String,
        type_params: Vec<String>,
    ) -> Self {
        GenericFunctionId { name, type_params }
    }

    /// 获取函数名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取泛型参数列表
    pub fn type_params(&self) -> &[String] {
        &self.type_params
    }

    /// 获取完整的签名
    pub fn signature(&self) -> String {
        if self.type_params.is_empty() {
            self.name.clone()
        } else {
            format!("{}<{}>", self.name, self.type_params.join(", "))
        }
    }
}

impl fmt::Display for GenericFunctionId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.signature())
    }
}

/// 特化函数实例
///
/// 表示一个泛型函数被特化后的具体函数
#[derive(Debug, Clone)]
pub struct FunctionInstance {
    /// 特化后的函数ID
    pub id: FunctionId,

    /// 泛型函数ID
    pub generic_id: GenericFunctionId,

    /// 使用的类型参数
    pub type_args: Vec<MonoType>,

    /// 特化后的函数IR（延迟生成）
    pub ir: Option<Arc<crate::middle::ir::FunctionIR>>,
}

impl FunctionInstance {
    /// 创建新的函数实例
    pub fn new(
        id: FunctionId,
        generic_id: GenericFunctionId,
        type_args: Vec<MonoType>,
    ) -> Self {
        FunctionInstance {
            id,
            generic_id,
            type_args,
            ir: None,
        }
    }

    /// 设置函数IR
    pub fn set_ir(
        &mut self,
        ir: crate::middle::ir::FunctionIR,
    ) {
        self.ir = Some(Arc::new(ir));
    }

    /// 获取函数IR（如果已生成）
    pub fn get_ir(&self) -> Option<&crate::middle::ir::FunctionIR> {
        self.ir.as_deref()
    }
}

/// 函数ID
///
/// 用于唯一标识一个已特化的函数
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionId {
    /// 函数名称
    name: String,
    /// 类型参数（用于生成唯一名称）
    type_args: Vec<MonoType>,
}

impl FunctionId {
    /// 创建新的函数ID
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
    ) -> Self {
        FunctionId { name, type_args }
    }

    /// 获取函数名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取完整的特化名称
    pub fn specialized_name(&self) -> String {
        if self.type_args.is_empty() {
            self.name.clone()
        } else {
            let args_str = self
                .type_args
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join("_");
            format!("{}_{}", self.name, args_str)
        }
    }
}

impl std::hash::Hash for FunctionId {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for ty in &self.type_args {
            ty.type_name().hash(state);
        }
    }
}

impl fmt::Display for FunctionId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.specialized_name())
    }
}
