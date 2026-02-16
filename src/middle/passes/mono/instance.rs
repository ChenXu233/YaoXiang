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
    /// 参数类型列表（用于区分重载）
    pub param_types: Vec<MonoType>,
    /// 类型参数
    pub type_args: Vec<MonoType>,
}

impl SpecializationKey {
    /// 创建新的缓存键（非重载函数）
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
    ) -> Self {
        SpecializationKey {
            name,
            param_types: Vec::new(),
            type_args,
        }
    }

    /// 创建重载函数的缓存键
    pub fn new_overload(
        name: String,
        param_types: Vec<MonoType>,
        type_args: Vec<MonoType>,
    ) -> Self {
        SpecializationKey {
            name,
            param_types,
            type_args,
        }
    }

    /// 生成字符串键
    pub fn as_string(&self) -> String {
        let param_str = if self.param_types.is_empty() {
            String::new()
        } else {
            let params: Vec<String> = self.param_types.iter().map(|t| t.type_name()).collect();
            format!("({})", params.join(", "))
        };

        let type_str = if self.type_args.is_empty() {
            String::new()
        } else {
            let args: Vec<String> = self.type_args.iter().map(|t| t.type_name()).collect();
            format!("<{}>", args.join(","))
        };

        format!("{}{}{}", self.name, param_str, type_str)
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
        self.name == other.name
            && self.param_types == other.param_types
            && self.type_args == other.type_args
    }
}

impl Eq for SpecializationKey {}

impl Hash for SpecializationKey {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for ty in &self.param_types {
            type_name_hash(ty, state);
        }
        for ty in &self.type_args {
            type_name_hash(ty, state);
        }
    }
}

// ==================== 辅助函数 ====================

/// 计算 MonoType 的哈希值
#[allow(clippy::only_used_in_recursion)]
fn type_name_hash<H: Hasher>(
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
                type_name_hash(t, state);
            }
        }
        MonoType::List(t) => {
            "list".hash(state);
            type_name_hash(t, state);
        }
        MonoType::Dict(k, v) => {
            "dict".hash(state);
            type_name_hash(k, state);
            type_name_hash(v, state);
        }
        MonoType::Set(t) => {
            "set".hash(state);
            type_name_hash(t, state);
        }
        MonoType::Fn { .. } => "fn".hash(state),
        MonoType::Range { elem_type } => {
            "range".hash(state);
            type_name_hash(elem_type, state);
        }
        MonoType::TypeVar(v) => format!("var{}", v.index()).hash(state),
        MonoType::TypeRef(n) => n.hash(state),
        // 联合类型和交集类型使用 TypeRef 的哈希方式
        MonoType::Union(types) => {
            "union".hash(state);
            for t in types {
                type_name_hash(t, state);
            }
        }
        MonoType::Intersection(types) => {
            "intersection".hash(state);
            for t in types {
                type_name_hash(t, state);
            }
        }
        MonoType::Arc(t) => {
            "arc".hash(state);
            type_name_hash(t, state);
        }
        MonoType::Weak(t) => {
            "weak".hash(state);
            type_name_hash(t, state);
        }
        MonoType::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => {
            "assoc_type".hash(state);
            type_name_hash(host_type, state);
            assoc_name.hash(state);
            for t in assoc_args {
                type_name_hash(t, state);
            }
        }
        MonoType::Literal {
            name,
            base_type,
            value,
        } => {
            "literal".hash(state);
            name.hash(state);
            type_name_hash(base_type, state);
            // 哈希常量值
            match value {
                crate::frontend::core::type_system::ConstValue::Int(n) => {
                    format!("int:{}", n).hash(state);
                }
                crate::frontend::core::type_system::ConstValue::Bool(b) => {
                    format!("bool:{}", b).hash(state);
                }
                crate::frontend::core::type_system::ConstValue::Float(f) => {
                    format!("float:{}", f.to_bits()).hash(state);
                }
            }
        }
        MonoType::MetaType {
            universe_level,
            type_params,
        } => {
            "meta_type".hash(state);
            universe_level.hash(state);
            for p in type_params {
                p.hash(state);
            }
        }
    }
}

/// 泛型函数ID
///
/// 用于唯一标识一个泛型函数或重载函数
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericFunctionId {
    /// 函数名称
    name: String,
    /// 参数类型列表（用于区分重载的函数）
    /// 空列表表示非重载函数
    param_types: Vec<MonoType>,
    /// 泛型参数列表（用于泛型实例化）
    type_params: Vec<String>,
}

impl GenericFunctionId {
    /// 创建新的泛型函数ID（无参数类型，用于泛型）
    pub fn new(
        name: String,
        type_params: Vec<String>,
    ) -> Self {
        GenericFunctionId {
            name,
            param_types: Vec::new(),
            type_params,
        }
    }

    /// 创建新的重载函数ID（带参数类型）
    pub fn new_overload(
        name: String,
        param_types: Vec<MonoType>,
        type_params: Vec<String>,
    ) -> Self {
        GenericFunctionId {
            name,
            param_types,
            type_params,
        }
    }

    /// 获取函数名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取参数类型列表
    pub fn param_types(&self) -> &[MonoType] {
        &self.param_types
    }

    /// 检查是否是重载函数
    pub fn is_overload(&self) -> bool {
        !self.param_types.is_empty()
    }

    /// 获取泛型参数列表
    pub fn type_params(&self) -> &[String] {
        &self.type_params
    }

    /// 获取完整的签名
    pub fn signature(&self) -> String {
        let param_str = if self.param_types.is_empty() {
            String::new()
        } else {
            format!(
                "({})",
                self.param_types
                    .iter()
                    .map(|t| t.type_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        let generic_str = if self.type_params.is_empty() {
            String::new()
        } else {
            format!("<{}>", self.type_params.join(", "))
        };

        format!("{}{}{}", self.name, param_str, generic_str)
    }

    /// 生成特化名称（用于代码生成）
    pub fn specialization_name(&self) -> String {
        let type_args_str = if self.type_params.is_empty() {
            String::new()
        } else {
            let args: Vec<String> = self.type_params.iter().map(|p| format!("_{}", p)).collect();
            args.join("")
        };

        let param_str = if self.param_types.is_empty() {
            String::new()
        } else {
            let args: Vec<String> = self
                .param_types
                .iter()
                .map(|t| format!("_{}", t.type_name()))
                .collect();
            args.join("")
        };

        format!("{}{}{}", self.name, param_str, type_args_str)
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

/// 泛型类型ID
///
/// 用于唯一标识一个泛型类型（如 `List<T>`、`Option<T>`）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericTypeId {
    /// 类型名称
    name: String,
    /// 泛型参数列表（用于区分重载的泛型类型）
    type_params: Vec<String>,
}

impl GenericTypeId {
    /// 创建新的泛型类型ID
    pub fn new(
        name: String,
        type_params: Vec<String>,
    ) -> Self {
        GenericTypeId { name, type_params }
    }

    /// 获取类型名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取泛型参数列表
    pub fn type_params(&self) -> &[String] {
        &self.type_params
    }
}

impl fmt::Display for GenericTypeId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        if self.type_params.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}<{}>", self.name, self.type_params.join(", "))
        }
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
    pub ir: Option<Arc<crate::middle::core::ir::FunctionIR>>,
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
        ir: crate::middle::core::ir::FunctionIR,
    ) {
        self.ir = Some(Arc::new(ir));
    }

    /// 获取函数IR（如果已生成）
    pub fn get_ir(&self) -> Option<&crate::middle::core::ir::FunctionIR> {
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

    /// 获取类型参数
    pub fn type_args(&self) -> &[MonoType] {
        &self.type_args
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

/// 类型ID
///
/// 用于唯一标识一个已特化的类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeId {
    /// 类型名称
    name: String,
    /// 类型参数（用于生成唯一名称）
    type_args: Vec<MonoType>,
}

impl TypeId {
    /// 创建新的类型ID
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
    ) -> Self {
        TypeId { name, type_args }
    }

    /// 获取类型名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取类型参数
    pub fn type_args(&self) -> &[MonoType] {
        &self.type_args
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

impl fmt::Display for TypeId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.specialized_name())
    }
}

impl std::hash::Hash for TypeId {
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

/// 类型实例
///
/// 表示一个泛型类型被特化后的具体类型
#[derive(Debug, Clone)]
pub struct TypeInstance {
    /// 特化后的类型ID
    pub id: TypeId,

    /// 泛型类型ID
    pub generic_id: GenericTypeId,

    /// 使用的类型参数
    pub type_args: Vec<MonoType>,

    /// 实例化后的 MonoType（延迟生成）
    pub mono_type: Option<MonoType>,
}

impl TypeInstance {
    /// 创建新的类型实例
    pub fn new(
        id: TypeId,
        generic_id: GenericTypeId,
        type_args: Vec<MonoType>,
    ) -> Self {
        TypeInstance {
            id,
            generic_id,
            type_args,
            mono_type: None,
        }
    }

    /// 设置单态类型
    pub fn set_mono_type(
        &mut self,
        mono_type: MonoType,
    ) {
        self.mono_type = Some(mono_type);
    }

    /// 获取单态类型
    pub fn get_mono_type(&self) -> Option<&MonoType> {
        self.mono_type.as_ref()
    }
}

// ==================== 闭包单态化相关 ====================

use crate::middle::core::ir::{FunctionIR, Operand};

/// 捕获变量
///
/// 表示闭包从外部环境捕获的变量
#[derive(Debug, Clone)]
pub struct CaptureVariable {
    /// 变量名称（用于调试）
    pub name: String,
    /// 变量类型
    pub mono_type: MonoType,
    /// 捕获的值（操作数）
    pub value: Operand,
}

impl CaptureVariable {
    /// 创建新的捕获变量
    pub fn new(
        name: String,
        mono_type: MonoType,
        value: Operand,
    ) -> Self {
        CaptureVariable {
            name,
            mono_type,
            value,
        }
    }
}

/// 泛型闭包ID
///
/// 用于唯一标识一个泛型闭包
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericClosureId {
    /// 闭包名称（通常是生成闭包的函数名）
    name: String,
    /// 泛型参数列表
    type_params: Vec<String>,
    /// 捕获变量名称列表（用于调试）
    capture_names: Vec<String>,
}

impl GenericClosureId {
    /// 创建新的泛型闭包ID
    pub fn new(
        name: String,
        type_params: Vec<String>,
        capture_names: Vec<String>,
    ) -> Self {
        GenericClosureId {
            name,
            type_params,
            capture_names,
        }
    }

    /// 获取闭包名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取泛型参数列表
    pub fn type_params(&self) -> &[String] {
        &self.type_params
    }

    /// 获取捕获变量名称
    pub fn capture_names(&self) -> &[String] {
        &self.capture_names
    }

    /// 获取完整的签名
    pub fn signature(&self) -> String {
        let captures = if self.capture_names.is_empty() {
            "".to_string()
        } else {
            format!("|[{}]|", self.capture_names.join(", "))
        };
        if self.type_params.is_empty() {
            format!("{}{}", self.name, captures)
        } else {
            format!("{}<{}>{}", self.name, self.type_params.join(", "), captures)
        }
    }
}

impl fmt::Display for GenericClosureId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.signature())
    }
}

/// 闭包ID
///
/// 用于唯一标识一个已特化的闭包
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureId {
    /// 闭包名称
    name: String,
    /// 类型参数（用于生成唯一名称）
    type_args: Vec<MonoType>,
    /// 捕获变量类型（用于生成唯一名称）
    capture_types: Vec<MonoType>,
}

impl ClosureId {
    /// 创建新的闭包ID
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
        capture_types: Vec<MonoType>,
    ) -> Self {
        ClosureId {
            name,
            type_args,
            capture_types,
        }
    }

    /// 获取闭包名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取类型参数
    pub fn type_args(&self) -> &[MonoType] {
        &self.type_args
    }

    /// 获取捕获变量类型
    pub fn capture_types(&self) -> &[MonoType] {
        &self.capture_types
    }

    /// 获取完整的特化名称
    pub fn specialized_name(&self) -> String {
        let type_suffix = if self.type_args.is_empty() {
            "".to_string()
        } else {
            let args_str = self
                .type_args
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join("_");
            format!("_{}", args_str)
        };

        let capture_suffix = if self.capture_types.is_empty() {
            "".to_string()
        } else {
            let caps_str = self
                .capture_types
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join("_");
            format!("_cap_{}", caps_str)
        };

        format!("{}{}{}", self.name, type_suffix, capture_suffix)
    }
}

impl std::hash::Hash for ClosureId {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for ty in &self.type_args {
            ty.type_name().hash(state);
        }
        for ty in &self.capture_types {
            ty.type_name().hash(state);
        }
    }
}

impl fmt::Display for ClosureId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.specialized_name())
    }
}

/// 闭包实例
///
/// 表示一个泛型闭包被特化后的具体闭包
#[derive(Debug, Clone)]
pub struct ClosureInstance {
    /// 特化后的闭包ID
    pub id: ClosureId,
    /// 泛型闭包ID
    pub generic_id: GenericClosureId,
    /// 使用的类型参数
    pub type_args: Vec<MonoType>,
    /// 捕获变量
    pub capture_vars: Vec<CaptureVariable>,
    /// 特化后的闭包体IR
    pub body_ir: FunctionIR,
}

impl ClosureInstance {
    /// 创建新的闭包实例
    pub fn new(
        id: ClosureId,
        generic_id: GenericClosureId,
        type_args: Vec<MonoType>,
        capture_vars: Vec<CaptureVariable>,
        body_ir: FunctionIR,
    ) -> Self {
        ClosureInstance {
            id,
            generic_id,
            type_args,
            capture_vars,
            body_ir,
        }
    }
}

/// 闭包特化缓存键
///
/// 用于在缓存中唯一标识一个闭包特化版本
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureSpecializationKey {
    /// 闭包名称
    pub name: String,
    /// 类型参数
    pub type_args: Vec<MonoType>,
    /// 捕获变量类型
    pub capture_types: Vec<MonoType>,
}

impl ClosureSpecializationKey {
    /// 创建新的闭包特化缓存键
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
        capture_types: Vec<MonoType>,
    ) -> Self {
        ClosureSpecializationKey {
            name,
            type_args,
            capture_types,
        }
    }

    /// 生成字符串键
    pub fn as_string(&self) -> String {
        let type_str = if self.type_args.is_empty() {
            "".to_string()
        } else {
            let args_str = self
                .type_args
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join(",");
            format!("<{}>", args_str)
        };

        let capture_str = if self.capture_types.is_empty() {
            "".to_string()
        } else {
            let caps_str = self
                .capture_types
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join(",");
            format!("|[{}]|", caps_str)
        };

        format!("{}{}{}", self.name, type_str, capture_str)
    }
}

impl Hash for ClosureSpecializationKey {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for ty in &self.type_args {
            type_name_hash(ty, state);
        }
        for ty in &self.capture_types {
            type_name_hash(ty, state);
        }
    }
}

impl fmt::Display for ClosureSpecializationKey {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}
