//! 跨模块单态化
//!
//! 管理所有模块的单态化，支持跨模块泛型实例化
//! 核心策略：
//! 1. 全局实例缓存：相同类型参数只实例化一次
//! 2. 模块状态追踪：每个模块独立追踪其泛型定义和实例化状态
//! 3. 跨模块实例化：在定义模块中实例化，使用模块引用结果

use crate::frontend::core::parser::ast::Type;
use crate::frontend::core::type_system::ConstValue;
use crate::frontend::typecheck::{EnumType, MonoType, StructType};
use crate::middle::core::ir::{BasicBlock, FunctionIR, Instruction, ModuleIR};
use crate::middle::passes::module::{ModuleGraph, ModuleId};
use crate::middle::passes::mono::module_state::{
    ExportedGeneric, FunctionInstanceKey, GenericFunctionKey, GenericTypeKey, ModuleMonoState,
    TypeInstanceKey,
};
use std::collections::HashMap;

/// 全局实例缓存键
///
/// 用于在全局范围内唯一标识一个实例
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlobalInstanceKey {
    /// 泛型名称
    pub name: String,
    /// 类型参数
    pub type_args: Vec<MonoType>,
}

impl GlobalInstanceKey {
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
    ) -> Self {
        GlobalInstanceKey { name, type_args }
    }
}

/// 缓存的实例信息
#[derive(Debug, Clone)]
pub struct CachedInstance {
    /// 实例ID（函数或类型）
    pub instance_id: String,
    /// 定义模块ID
    pub defining_module: ModuleId,
    /// 是否已生成IR
    pub ir_generated: bool,
}

/// 跨模块单态化器
///
/// 管理所有模块的泛型定义和跨模块实例化
#[derive(Debug)]
pub struct CrossModuleMonomorphizer {
    /// 模块依赖图
    module_graph: ModuleGraph,

    /// 模块单态化状态
    module_states: HashMap<ModuleId, ModuleMonoState>,

    /// 全局实例缓存：避免重复实例化
    global_instance_cache: HashMap<GlobalInstanceKey, CachedInstance>,

    /// 模块泛型导出表：module_id -> (name -> ExportedGeneric)
    module_exports: HashMap<ModuleId, HashMap<String, ExportedGeneric>>,

    /// 泛型定义位置表：(module_id, name) -> GenericInfo
    generic_definitions: HashMap<(ModuleId, String), GenericInfo>,
}

/// 泛型定义信息
#[derive(Debug, Clone)]
pub struct GenericInfo {
    /// 泛型名称
    pub name: String,
    /// 类型参数列表
    pub type_params: Vec<String>,
    /// 是否为函数
    pub is_function: bool,
}

impl CrossModuleMonomorphizer {
    /// 创建新的跨模块单态化器
    pub fn new() -> Self {
        CrossModuleMonomorphizer {
            module_graph: ModuleGraph::new(),
            module_states: HashMap::new(),
            global_instance_cache: HashMap::new(),
            module_exports: HashMap::new(),
            generic_definitions: HashMap::new(),
        }
    }

    /// 注册模块
    ///
    /// 添加模块到依赖图，创建单态化状态
    pub fn register_module(
        &mut self,
        _module_id: ModuleId,
        source_path: std::path::PathBuf,
    ) -> ModuleId {
        let actual_id = self.module_graph.add_module(source_path);

        let module_name = self
            .module_graph
            .get_module_name(actual_id)
            .unwrap_or("unknown")
            .to_string();

        self.module_states.insert(
            actual_id,
            ModuleMonoState::new(actual_id, module_name.clone()),
        );

        self.module_exports.insert(actual_id, HashMap::new());

        actual_id
    }

    /// 添加模块依赖
    pub fn add_dependency(
        &mut self,
        from: ModuleId,
        to: ModuleId,
        is_public: bool,
    ) -> Result<(), crate::middle::passes::module::ModuleGraphError> {
        self.module_graph.add_dependency(from, to, is_public)
    }

    /// 收集模块的泛型定义
    ///
    /// 从模块IR中提取泛型函数和类型
    pub fn collect_generics(
        &mut self,
        module_id: ModuleId,
        ir: &ModuleIR,
    ) {
        let state = match self.module_states.get_mut(&module_id) {
            Some(s) => s,
            None => return,
        };

        state.collect_generic_functions(&ir.functions);

        let mono_types: Vec<MonoType> = ir.types.iter().map(ast_type_to_mono_type).collect();
        state.collect_generic_types(&mono_types);

        for key in state.generic_functions.keys() {
            self.generic_definitions.insert(
                (module_id, key.name.clone()),
                GenericInfo {
                    name: key.name.clone(),
                    type_params: key.type_params.clone(),
                    is_function: true,
                },
            );
        }

        for key in state.generic_types.keys() {
            self.generic_definitions.insert(
                (module_id, key.name.clone()),
                GenericInfo {
                    name: key.name.clone(),
                    type_params: key.type_params.clone(),
                    is_function: false,
                },
            );
        }

        for (name, generic) in &state.exported_generics {
            if let Some(exports) = self.module_exports.get_mut(&module_id) {
                exports.insert(name.clone(), generic.clone());
            }
        }
    }

    /// 收集模块的导入
    ///
    /// 记录模块的导入关系，用于后续可见性检查
    pub fn collect_imports(
        &mut self,
        module_id: ModuleId,
        imports: &[crate::frontend::typecheck::ImportInfo],
    ) {
        let resolved_imports: Vec<_> = imports
            .iter()
            .filter_map(|import| {
                self.resolve_module_path(&import.path)
                    .map(|source_module| (source_module, import))
            })
            .collect();

        if resolved_imports.is_empty() {
            return;
        }

        let mut generic_definitions_updates: Vec<(String, GenericInfo)> = Vec::new();
        for (src_module, import) in &resolved_imports {
            if let Some(items) = &import.items {
                for item in items {
                    if let Some(generic_info) = self.find_generic_in_module(*src_module, item) {
                        generic_definitions_updates.push((item.to_string(), generic_info));
                    }
                }
            }
        }

        let state = match self.module_states.get_mut(&module_id) {
            Some(s) => s,
            None => return,
        };

        for (src_module, import) in resolved_imports {
            if let Some(items) = &import.items {
                for item in items {
                    state.record_import(
                        src_module,
                        item.clone(),
                        vec![],
                        CrossModuleMonomorphizer::is_function_name(item),
                    );
                }
            }
        }

        for (item, generic_info) in generic_definitions_updates {
            self.generic_definitions
                .insert((module_id, item), generic_info);
        }
    }

    /// 解析模块路径到模块ID
    fn resolve_module_path(
        &self,
        _path: &str,
    ) -> Option<ModuleId> {
        None
    }

    /// 检查名称是否为函数
    fn is_function_name(name: &str) -> bool {
        name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
    }

    /// 在模块中查找泛型定义
    fn find_generic_in_module(
        &self,
        module_id: ModuleId,
        name: &str,
    ) -> Option<GenericInfo> {
        let state = self.module_states.get(&module_id)?;

        for key in state.generic_functions.keys() {
            if key.name == name {
                return Some(GenericInfo {
                    name: name.to_string(),
                    type_params: key.type_params.clone(),
                    is_function: true,
                });
            }
        }

        for key in state.generic_types.keys() {
            if key.name == name {
                return Some(GenericInfo {
                    name: name.to_string(),
                    type_params: key.type_params.clone(),
                    is_function: false,
                });
            }
        }

        None
    }

    /// 查找泛型定义所在的模块
    pub fn find_generic_definition(
        &self,
        name: &str,
    ) -> Option<(ModuleId, GenericInfo)> {
        for ((module_id, _), info) in &self.generic_definitions {
            if info.name == name {
                return Some((*module_id, info.clone()));
            }
        }
        None
    }

    /// 在全局范围内实例化泛型函数
    ///
    /// 如果已实例化，直接返回缓存的实例
    /// 否则在定义模块中实例化
    pub fn instantiate_function_global(
        &mut self,
        _module_id: ModuleId,
        generic_name: &str,
        type_args: Vec<MonoType>,
    ) -> Option<FunctionIR> {
        let cache_key = GlobalInstanceKey::new(generic_name.to_string(), type_args.clone());

        if let Some(cached) = self.global_instance_cache.get(&cache_key) {
            let state = self.module_states.get(&cached.defining_module)?;
            return state
                .get_instantiated_function(generic_name, &type_args)
                .cloned();
        }

        let (defining_module, generic_info) = self.find_generic_definition(generic_name)?;
        let generic_key =
            GenericFunctionKey::new(generic_name.to_string(), generic_info.type_params.clone());

        let generic_func = {
            let state = self.module_states.get(&defining_module)?;
            state.generic_functions.get(&generic_key)?.clone()
        };

        let specialized = substitute_types_in_function(&generic_func, type_args.clone());

        let state = self.module_states.get_mut(&defining_module)?;
        let instance_key = FunctionInstanceKey::new(generic_name.to_string(), type_args.clone());
        state.register_instantiated_function(
            generic_name.to_string(),
            type_args.clone(),
            specialized.clone(),
        );

        self.global_instance_cache.insert(
            cache_key,
            CachedInstance {
                instance_id: instance_key.specialized_name(),
                defining_module,
                ir_generated: true,
            },
        );

        Some(specialized)
    }

    /// 在全局范围内实例化泛型类型
    pub fn instantiate_type_global(
        &mut self,
        _module_id: ModuleId,
        generic_name: &str,
        type_args: Vec<MonoType>,
    ) -> Option<MonoType> {
        let cache_key = GlobalInstanceKey::new(generic_name.to_string(), type_args.clone());

        if let Some(cached) = self.global_instance_cache.get(&cache_key) {
            let state = self.module_states.get(&cached.defining_module)?;
            return state
                .get_instantiated_type(generic_name, &type_args)
                .cloned();
        }

        let (defining_module, generic_info) = self.find_generic_definition(generic_name)?;

        let generic_key =
            GenericTypeKey::new(generic_name.to_string(), generic_info.type_params.clone());

        let generic_type = {
            let state = self.module_states.get(&defining_module)?;
            state.generic_types.get(&generic_key)?.clone()
        };

        let specialized = substitute_type(&generic_type, &type_args, &generic_info.type_params);

        let state = self.module_states.get_mut(&defining_module)?;
        let instance_key = TypeInstanceKey::new(generic_name.to_string(), type_args.clone());
        state.register_instantiated_type(
            generic_name.to_string(),
            type_args.clone(),
            specialized.clone(),
        );

        self.global_instance_cache.insert(
            cache_key,
            CachedInstance {
                instance_id: instance_key.specialized_name(),
                defining_module,
                ir_generated: true,
            },
        );

        Some(specialized)
    }

    /// 获取模块数量
    pub fn module_count(&self) -> usize {
        self.module_graph.len()
    }

    /// 获取全局实例数量
    pub fn instance_count(&self) -> usize {
        self.global_instance_cache.len()
    }

    /// 获取模块图引用
    pub fn module_graph(&self) -> &ModuleGraph {
        &self.module_graph
    }

    /// 获取模块状态
    pub fn get_module_state(
        &self,
        module_id: ModuleId,
    ) -> Option<&ModuleMonoState> {
        self.module_states.get(&module_id)
    }

    /// 获取所有模块ID
    pub fn all_modules(&self) -> Vec<ModuleId> {
        self.module_graph.all_modules()
    }
}

impl Default for CrossModuleMonomorphizer {
    fn default() -> Self {
        Self::new()
    }
}

/// 将 AST Type 转换为 MonoType
#[allow(clippy::only_used_in_recursion)]
fn ast_type_to_mono_type(ty: &Type) -> MonoType {
    match ty {
        Type::Name(name) => MonoType::TypeRef(name.clone()),
        Type::Int(n) => MonoType::Int(*n),
        Type::Float(n) => MonoType::Float(*n),
        Type::Char => MonoType::Char,
        Type::String => MonoType::String,
        Type::Bytes => MonoType::Bytes,
        Type::Bool => MonoType::Bool,
        Type::Void => MonoType::Void,
        Type::Struct(fields) => MonoType::Struct(StructType {
            name: fields
                .first()
                .map(|f| f.name.clone())
                .unwrap_or_else(|| "Struct".to_string()),
            fields: fields
                .iter()
                .map(|f| (f.name.clone(), ast_type_to_mono_type(&f.ty)))
                .collect(),
            methods: HashMap::new(),
            field_mutability: fields.iter().map(|f| f.is_mut).collect(),
        }),
        Type::NamedStruct { name, fields } => MonoType::Struct(StructType {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|f| (f.name.clone(), ast_type_to_mono_type(&f.ty)))
                .collect(),
            methods: HashMap::new(),
            field_mutability: fields.iter().map(|f| f.is_mut).collect(),
        }),
        Type::Union(variants) => MonoType::Union(
            variants
                .iter()
                .filter_map(|(_, ty)| ty.as_ref().map(ast_type_to_mono_type))
                .collect(),
        ),
        Type::Enum(variants) => MonoType::Enum(EnumType {
            name: variants
                .first()
                .cloned()
                .unwrap_or_else(|| "Enum".to_string()),
            variants: variants.clone(),
        }),
        Type::Variant(_) => MonoType::TypeRef("Variant".to_string()),
        Type::Tuple(types) => MonoType::Tuple(types.iter().map(ast_type_to_mono_type).collect()),
        Type::Fn {
            params,
            return_type,
            ..
        } => MonoType::Fn {
            params: params.iter().map(ast_type_to_mono_type).collect(),
            return_type: Box::new(ast_type_to_mono_type(return_type)),
            is_async: false,
        },
        Type::Option(inner) => MonoType::Union(vec![ast_type_to_mono_type(inner)]),
        Type::Result(_, _) => MonoType::TypeRef("Result".to_string()),
        Type::Generic { name, args } => {
            let args_str = args
                .iter()
                .map(|t| ast_type_to_mono_type(t).type_name())
                .collect::<Vec<_>>()
                .join(",");
            MonoType::TypeRef(format!("{}<{}>", name, args_str))
        }
        Type::Sum(types) => MonoType::Union(types.iter().map(ast_type_to_mono_type).collect()),
        Type::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => MonoType::AssocType {
            host_type: Box::new(ast_type_to_mono_type(host_type)),
            assoc_name: assoc_name.clone(),
            assoc_args: assoc_args.iter().map(ast_type_to_mono_type).collect(),
        },
        Type::Literal { name, base_type } => {
            // 对于字面量类型，转换为基础类型
            // 字面量的值会在类型检查阶段处理
            let base = ast_type_to_mono_type(base_type);
            // 尝试从名称解析 ConstValue
            let value = ConstValue::from_literal_name(name).unwrap_or({
                // 如果无法解析，使用默认值
                ConstValue::Int(0)
            });
            MonoType::Literal {
                name: name.clone(),
                base_type: Box::new(base),
                value,
            }
        }
        Type::Ptr(inner) => {
            // 裸指针类型：*T
            MonoType::TypeRef(format!("*{}", ast_type_to_mono_type(inner).type_name()))
        }
        Type::MetaType { .. } => {
            // 元类型：编译期概念，无需特殊处理
            MonoType::Void
        }
    }
}

/// 类型替换（用于函数）
fn substitute_types_in_function(
    generic_func: &FunctionIR,
    type_args: Vec<MonoType>,
) -> FunctionIR {
    let type_param_map: HashMap<usize, MonoType> = generic_func
        .params
        .iter()
        .enumerate()
        .filter_map(|(idx, ty)| {
            if let MonoType::TypeVar(tv) = ty {
                if idx < type_args.len() {
                    return Some((tv.index(), type_args[idx].clone()));
                }
            }
            None
        })
        .collect();

    let new_params: Vec<MonoType> = generic_func
        .params
        .iter()
        .map(|ty| substitute_single_type(ty, &type_param_map))
        .collect();

    let new_return_type = substitute_single_type(&generic_func.return_type, &type_param_map);

    let new_locals: Vec<MonoType> = generic_func
        .locals
        .iter()
        .map(|ty| substitute_single_type(ty, &type_param_map))
        .collect();

    let new_blocks: Vec<BasicBlock> = generic_func
        .blocks
        .iter()
        .map(|block| substitute_block(block, &type_param_map))
        .collect();

    FunctionIR {
        name: generate_specialized_name(&generic_func.name, &type_args),
        params: new_params,
        return_type: new_return_type,
        is_async: generic_func.is_async,
        locals: new_locals,
        blocks: new_blocks,
        entry: generic_func.entry,
    }
}

/// 类型替换（用于类型）
fn substitute_type(
    generic_type: &MonoType,
    type_args: &[MonoType],
    type_params: &[String],
) -> MonoType {
    let type_map: HashMap<String, MonoType> = type_params
        .iter()
        .enumerate()
        .filter_map(|(idx, param)| {
            if idx < type_args.len() {
                Some((param.clone(), type_args[idx].clone()))
            } else {
                None
            }
        })
        .collect();

    substitute_type_with_map(generic_type, &type_map)
}

/// 单个类型替换
#[allow(clippy::only_used_in_recursion)]
fn substitute_single_type(
    ty: &MonoType,
    type_map: &HashMap<usize, MonoType>,
) -> MonoType {
    match ty {
        MonoType::TypeVar(tv) => {
            if let Some(replacement) = type_map.get(&tv.index()) {
                replacement.clone()
            } else {
                ty.clone()
            }
        }
        MonoType::List(elem) => MonoType::List(Box::new(substitute_single_type(elem, type_map))),
        MonoType::Dict(key, value) => MonoType::Dict(
            Box::new(substitute_single_type(key, type_map)),
            Box::new(substitute_single_type(value, type_map)),
        ),
        MonoType::Set(elem) => MonoType::Set(Box::new(substitute_single_type(elem, type_map))),
        MonoType::Tuple(types) => MonoType::Tuple(
            types
                .iter()
                .map(|t| substitute_single_type(t, type_map))
                .collect(),
        ),
        MonoType::Fn {
            params,
            return_type,
            is_async,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|t| substitute_single_type(t, type_map))
                .collect(),
            return_type: Box::new(substitute_single_type(return_type, type_map)),
            is_async: *is_async,
        },
        MonoType::Arc(inner) => MonoType::Arc(Box::new(substitute_single_type(inner, type_map))),
        MonoType::Range { elem_type } => MonoType::Range {
            elem_type: Box::new(substitute_single_type(elem_type, type_map)),
        },
        MonoType::Union(types) | MonoType::Intersection(types) => {
            let substituted: Vec<MonoType> = types
                .iter()
                .map(|ty| substitute_single_type(ty, type_map))
                .collect();
            if matches!(ty, MonoType::Union(_)) {
                MonoType::Union(substituted)
            } else {
                MonoType::Intersection(substituted)
            }
        }
        _ => ty.clone(),
    }
}

/// 使用名称映射替换类型
#[allow(clippy::only_used_in_recursion)]
fn substitute_type_with_map(
    ty: &MonoType,
    type_map: &HashMap<String, MonoType>,
) -> MonoType {
    match ty {
        MonoType::TypeVar(tv) => {
            let name = format!("T{}", tv.index());
            if let Some(replacement) = type_map.get(&name) {
                replacement.clone()
            } else {
                ty.clone()
            }
        }
        MonoType::Struct(struct_type) => MonoType::Struct(StructType {
            name: struct_type.name.clone(),
            fields: struct_type
                .fields
                .iter()
                .map(|(name, field_ty)| {
                    (name.clone(), substitute_type_with_map(field_ty, type_map))
                })
                .collect(),
            methods: struct_type.methods.clone(),
            field_mutability: struct_type.field_mutability.clone(),
        }),
        MonoType::List(elem) => MonoType::List(Box::new(substitute_type_with_map(elem, type_map))),
        MonoType::Dict(key, value) => MonoType::Dict(
            Box::new(substitute_type_with_map(key, type_map)),
            Box::new(substitute_type_with_map(value, type_map)),
        ),
        MonoType::Set(elem) => MonoType::Set(Box::new(substitute_type_with_map(elem, type_map))),
        MonoType::Tuple(types) => MonoType::Tuple(
            types
                .iter()
                .map(|t| substitute_type_with_map(t, type_map))
                .collect(),
        ),
        MonoType::Fn {
            params,
            return_type,
            is_async,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|t| substitute_type_with_map(t, type_map))
                .collect(),
            return_type: Box::new(substitute_type_with_map(return_type, type_map)),
            is_async: *is_async,
        },
        MonoType::Arc(inner) => MonoType::Arc(Box::new(substitute_type_with_map(inner, type_map))),
        MonoType::Range { elem_type } => MonoType::Range {
            elem_type: Box::new(substitute_type_with_map(elem_type, type_map)),
        },
        MonoType::Union(types) | MonoType::Intersection(types) => {
            let substituted: Vec<MonoType> = types
                .iter()
                .map(|ty| substitute_type_with_map(ty, type_map))
                .collect();
            if matches!(ty, MonoType::Union(_)) {
                MonoType::Union(substituted)
            } else {
                MonoType::Intersection(substituted)
            }
        }
        _ => ty.clone(),
    }
}

/// 替换基本块中的指令
fn substitute_block(
    block: &BasicBlock,
    type_map: &HashMap<usize, MonoType>,
) -> BasicBlock {
    let new_instructions: Vec<Instruction> = block
        .instructions
        .iter()
        .map(|instr| substitute_instruction(instr, type_map))
        .collect();

    BasicBlock {
        label: block.label,
        instructions: new_instructions,
        successors: block.successors.clone(),
    }
}

/// 替换指令中的类型
fn substitute_instruction(
    instr: &Instruction,
    _type_map: &HashMap<usize, MonoType>,
) -> Instruction {
    instr.clone()
}

/// 生成特化函数名称
fn generate_specialized_name(
    base_name: &str,
    type_args: &[MonoType],
) -> String {
    if type_args.is_empty() {
        base_name.to_string()
    } else {
        let args_str = type_args
            .iter()
            .map(|t| {
                t.type_name()
                    .replace("/", "_")
                    .replace("<", "_")
                    .replace(">", "_")
            })
            .collect::<Vec<_>>()
            .join("_");
        format!("{}_{}", base_name, args_str)
    }
}
