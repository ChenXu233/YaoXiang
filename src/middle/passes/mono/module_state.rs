//! 模块单态化状态
//!
//! 管理单个模块的泛型定义和实例化状态

use crate::frontend::core::parser::ast::Type;
use crate::frontend::typecheck::{MonoType, StructType};
use crate::middle::core::ir::FunctionIR;
use crate::middle::passes::module::ModuleId;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// 泛型函数键
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericFunctionKey {
    pub name: String,
    pub type_params: Vec<String>,
}

impl GenericFunctionKey {
    pub fn new(
        name: String,
        type_params: Vec<String>,
    ) -> Self {
        Self { name, type_params }
    }
}

impl Hash for GenericFunctionKey {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for tp in &self.type_params {
            tp.hash(state);
        }
    }
}

/// 泛型类型键
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericTypeKey {
    pub name: String,
    pub type_params: Vec<String>,
}

impl GenericTypeKey {
    pub fn new(
        name: String,
        type_params: Vec<String>,
    ) -> Self {
        Self { name, type_params }
    }
}

impl Hash for GenericTypeKey {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for tp in &self.type_params {
            tp.hash(state);
        }
    }
}

/// 导出的泛型项
#[derive(Debug, Clone)]
pub struct ExportedGeneric {
    pub name: String,
    pub type_params: Vec<String>,
    pub is_function: bool,
}

impl ExportedGeneric {
    pub fn new(
        name: String,
        type_params: Vec<String>,
        is_function: bool,
    ) -> Self {
        Self {
            name,
            type_params,
            is_function,
        }
    }
}

/// 函数实例键
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInstanceKey {
    pub name: String,
    pub type_args: Vec<MonoType>,
}

impl FunctionInstanceKey {
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
    ) -> Self {
        Self { name, type_args }
    }

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

impl Hash for FunctionInstanceKey {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for ty in &self.type_args {
            ty.type_name().hash(state);
        }
    }
}

/// 类型实例键
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInstanceKey {
    pub name: String,
    pub type_args: Vec<MonoType>,
}

impl TypeInstanceKey {
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
    ) -> Self {
        Self { name, type_args }
    }

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

impl Hash for TypeInstanceKey {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for ty in &self.type_args {
            ty.type_name().hash(state);
        }
    }
}

/// 导入键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportKey {
    pub module_id: ModuleId,
    pub name: String,
}

/// 导入的泛型
#[derive(Debug, Clone)]
pub struct ImportedGeneric {
    pub source_module: ModuleId,
    pub name: String,
    pub type_params: Vec<String>,
    pub is_function: bool,
}

/// 模块单态化状态
#[derive(Debug, Clone)]
pub struct ModuleMonoState {
    /// 模块ID
    pub module_id: ModuleId,
    /// 模块名
    pub module_name: String,
    /// 泛型函数
    pub generic_functions: HashMap<GenericFunctionKey, FunctionIR>,
    /// 泛型类型
    pub generic_types: HashMap<GenericTypeKey, MonoType>,
    /// 实例化函数
    pub instantiated_functions: HashMap<FunctionInstanceKey, FunctionIR>,
    /// 实例化类型
    pub instantiated_types: HashMap<TypeInstanceKey, MonoType>,
    /// 导出泛型
    pub exported_generics: HashMap<String, ExportedGeneric>,
    /// 导入
    pub imports: Vec<ImportKey>,
    /// 导入的泛型
    pub imported_generics: HashMap<ImportKey, ImportedGeneric>,
}

impl ModuleMonoState {
    pub fn new(
        module_id: ModuleId,
        module_name: String,
    ) -> Self {
        Self {
            module_id,
            module_name,
            generic_functions: HashMap::new(),
            generic_types: HashMap::new(),
            instantiated_functions: HashMap::new(),
            instantiated_types: HashMap::new(),
            exported_generics: HashMap::new(),
            imports: Vec::new(),
            imported_generics: HashMap::new(),
        }
    }

    /// 收集泛型函数
    pub fn collect_generic_functions(
        &mut self,
        functions: &[FunctionIR],
    ) {
        for func in functions {
            let type_params = Self::extract_type_params(func);
            // 只收集泛型函数（type_params 不为空）
            if !type_params.is_empty() {
                let key = GenericFunctionKey::new(func.name.clone(), type_params);
                self.generic_functions.insert(key, func.clone());
            }
        }
    }

    /// 收集泛型类型
    pub fn collect_generic_types(
        &mut self,
        types: &[MonoType],
    ) {
        for ty in types {
            let type_params = Self::extract_type_params_from_mono_type(ty);
            let name = Self::get_type_name_from_mono_type(ty);
            let key = GenericTypeKey::new(name, type_params);
            self.generic_types.insert(key, ty.clone());
        }
    }

    /// 提取函数的类型参数
    fn extract_type_params(func: &FunctionIR) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = HashSet::new();

        for param_ty in &func.params {
            if let MonoType::TypeVar(tv) = param_ty {
                let name = format!("T{}", tv.index());
                if seen.insert(name.clone()) {
                    type_params.push(name);
                }
            }
        }

        if let MonoType::TypeVar(tv) = &func.return_type {
            let name = format!("T{}", tv.index());
            if seen.insert(name.clone()) {
                type_params.push(name);
            }
        }

        type_params
    }

    /// 从类型提取类型参数
    fn extract_type_params_from_type(ty: &Type) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = HashSet::new();
        Self::collect_type_vars_from_type(ty, &mut type_params, &mut seen);
        type_params
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_type_vars_from_type(
        ty: &Type,
        type_params: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) {
        match ty {
            Type::Name(name) => {
                if name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    && seen.insert(name.clone())
                {
                    type_params.push(name.clone());
                }
            }
            Type::Struct(fields) | Type::NamedStruct { fields, .. } => {
                for field in fields {
                    Self::collect_type_vars_from_type(&field.ty, type_params, seen);
                }
            }
            Type::Union(variants) => {
                for (_, ty) in variants {
                    if let Some(t) = ty {
                        Self::collect_type_vars_from_type(t, type_params, seen);
                    }
                }
            }
            Type::Tuple(types) => {
                for t in types {
                    Self::collect_type_vars_from_type(t, type_params, seen);
                }
            }
            Type::List(elem) => {
                Self::collect_type_vars_from_type(elem, type_params, seen);
            }
            Type::Dict(key, value) => {
                Self::collect_type_vars_from_type(key, type_params, seen);
                Self::collect_type_vars_from_type(value, type_params, seen);
            }
            Type::Set(elem) => {
                Self::collect_type_vars_from_type(elem, type_params, seen);
            }
            Type::Fn {
                params,
                return_type,
                ..
            } => {
                for p in params {
                    Self::collect_type_vars_from_type(p, type_params, seen);
                }
                Self::collect_type_vars_from_type(return_type, type_params, seen);
            }
            Type::Option(inner) => {
                Self::collect_type_vars_from_type(inner, type_params, seen);
            }
            Type::Result(ok, err) => {
                Self::collect_type_vars_from_type(ok, type_params, seen);
                Self::collect_type_vars_from_type(err, type_params, seen);
            }
            Type::Generic { args, .. } => {
                for t in args {
                    Self::collect_type_vars_from_type(t, type_params, seen);
                }
            }
            Type::Sum(types) => {
                for t in types {
                    Self::collect_type_vars_from_type(t, type_params, seen);
                }
            }
            _ => {}
        }
    }

    /// 获取类型名称
    fn get_type_name(ty: &Type) -> String {
        match ty {
            Type::Name(name) => name.clone(),
            Type::Int(n) => format!("int{}", n),
            Type::Float(n) => format!("float{}", n),
            Type::Char => "char".to_string(),
            Type::String => "string".to_string(),
            Type::Bytes => "bytes".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Void => "void".to_string(),
            Type::Struct(fields) => fields
                .first()
                .map(|f| f.name.clone())
                .unwrap_or_else(|| "Struct".to_string()),
            Type::NamedStruct { name, .. } => name.clone(),
            Type::Union(variants) => variants
                .first()
                .map(|(n, _)| n.clone())
                .unwrap_or_else(|| "Union".to_string()),
            Type::Enum(variants) => variants
                .first()
                .cloned()
                .unwrap_or_else(|| "Enum".to_string()),
            Type::Variant(variants) => variants
                .first()
                .map(|v| v.name.clone())
                .unwrap_or_else(|| "Variant".to_string()),
            Type::Tuple(types) => format!("tuple{}", types.len()),
            Type::List(_) => "List".to_string(),
            Type::Dict(_, _) => "Dict".to_string(),
            Type::Set(_) => "Set".to_string(),
            Type::Fn { .. } => "Fn".to_string(),
            Type::Option(_) => "Option".to_string(),
            Type::Result(_, _) => "Result".to_string(),
            Type::Generic { name, .. } => name.clone(),
            Type::Sum(_) => "Sum".to_string(),
            Type::AssocType {
                host_type,
                assoc_name,
                ..
            } => {
                // 递归调用get_type_name来获取宿主类型名称
                Self::get_type_name(host_type) + "::" + assoc_name
            }
            Type::Literal { name, base_type } => {
                // 字面量类型：基础类型::名称
                Self::get_type_name(base_type) + "::" + name
            }
            Type::Ptr(inner) => {
                // 裸指针类型：*T
                format!("*{}", Self::get_type_name(inner))
            }
        }
    }

    /// 记录导入
    pub fn record_import(
        &mut self,
        source_module: ModuleId,
        name: String,
        type_params: Vec<String>,
        is_function: bool,
    ) {
        let key = ImportKey {
            module_id: source_module,
            name: name.clone(),
        };
        self.imports.push(key.clone());
        self.imported_generics.insert(
            key,
            ImportedGeneric {
                source_module,
                name,
                type_params,
                is_function,
            },
        );
    }

    /// 注册实例化函数
    pub fn register_instantiated_function(
        &mut self,
        name: String,
        type_args: Vec<MonoType>,
        func: FunctionIR,
    ) {
        let key = FunctionInstanceKey::new(name, type_args);
        self.instantiated_functions.insert(key, func);
    }

    /// 获取实例化函数
    pub fn get_instantiated_function(
        &self,
        name: &str,
        type_args: &[MonoType],
    ) -> Option<&FunctionIR> {
        let key = FunctionInstanceKey::new(name.to_string(), type_args.to_vec());
        self.instantiated_functions.get(&key)
    }

    /// 注册实例化类型
    pub fn register_instantiated_type(
        &mut self,
        name: String,
        type_args: Vec<MonoType>,
        ty: MonoType,
    ) {
        let key = TypeInstanceKey::new(name, type_args);
        self.instantiated_types.insert(key, ty);
    }

    /// 获取实例化类型
    pub fn get_instantiated_type(
        &self,
        name: &str,
        type_args: &[MonoType],
    ) -> Option<&MonoType> {
        let key = TypeInstanceKey::new(name.to_string(), type_args.to_vec());
        self.instantiated_types.get(&key)
    }

    /// 从 MonoType 提取类型参数
    fn extract_type_params_from_mono_type(ty: &MonoType) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = std::collections::HashSet::new();
        Self::collect_type_vars_from_mono_type(ty, &mut type_params, &mut seen);
        type_params
    }

    /// 递归收集 MonoType 中的类型变量
    #[allow(clippy::only_used_in_recursion)]
    fn collect_type_vars_from_mono_type(
        ty: &MonoType,
        type_params: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match ty {
            MonoType::TypeVar(tv) => {
                let name = format!("T{}", tv.index());
                if seen.insert(name.clone()) {
                    type_params.push(name);
                }
            }
            MonoType::Struct(StructType { fields, .. }) => {
                for (_, fty) in fields {
                    Self::collect_type_vars_from_mono_type(fty, type_params, seen);
                }
            }
            MonoType::Tuple(types) => {
                for t in types {
                    Self::collect_type_vars_from_mono_type(t, type_params, seen);
                }
            }
            MonoType::List(elem) => {
                Self::collect_type_vars_from_mono_type(elem, type_params, seen);
            }
            MonoType::Dict(key, value) => {
                Self::collect_type_vars_from_mono_type(key, type_params, seen);
                Self::collect_type_vars_from_mono_type(value, type_params, seen);
            }
            MonoType::Set(elem) => {
                Self::collect_type_vars_from_mono_type(elem, type_params, seen);
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                for p in params {
                    Self::collect_type_vars_from_mono_type(p, type_params, seen);
                }
                Self::collect_type_vars_from_mono_type(return_type, type_params, seen);
            }
            MonoType::Range { elem_type } => {
                Self::collect_type_vars_from_mono_type(elem_type, type_params, seen);
            }
            MonoType::Union(types) | MonoType::Intersection(types) => {
                for t in types {
                    Self::collect_type_vars_from_mono_type(t, type_params, seen);
                }
            }
            MonoType::Arc(inner) => {
                Self::collect_type_vars_from_mono_type(inner, type_params, seen);
            }
            _ => {}
        }
    }

    /// 从 MonoType 获取类型名称
    fn get_type_name_from_mono_type(ty: &MonoType) -> String {
        match ty {
            MonoType::Struct(s) => s.name.clone(),
            MonoType::Enum(e) => e.name.clone(),
            MonoType::TypeRef(n) => n.clone(),
            _ => "Unknown".to_string(),
        }
    }

    /// 获取泛型函数数量
    pub fn generic_function_count(&self) -> usize {
        self.generic_functions.len()
    }

    /// 获取泛型类型数量
    pub fn generic_type_count(&self) -> usize {
        self.generic_types.len()
    }
}
