//! 类型环境模块
//!
//! 管理类型检查过程中的所有状态信息

use std::collections::{HashMap, HashSet};

use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::types::eval::const_eval::ConstFunction;

use super::passes::overload;
use super::types::ImportInfo;

/// 类型错误收集器
pub type TypeErrorCollector = crate::util::diagnostic::ErrorCollector<super::Diagnostic>;

/// 泛型类型定义模板
///
/// 存储泛型类型构造器的模板信息，用于类型实例化展开。
/// 例如 `List: (T: Type) -> Type = { data: Array(T), length: Int }` 中：
/// - type_param_names = ["T"]
/// - poly = PolyType { type_binders: [("T", Type)], body: Struct { ... } }
#[derive(Debug, Clone)]
pub struct GenericTypeDef {
    /// 多态类型（type_binders + const_binders + body）
    pub poly: PolyType,
    /// 类型参数名，按声明顺序，与 poly.type_binders 一一对应
    /// 用于实例化时匹配模板体中的 TypeRef 占位符
    pub type_param_names: Vec<String>,
}

/// 类型环境
///
/// 存储类型检查过程中的所有状态信息：
/// - 变量绑定
/// - 类型定义
/// - 约束求解器
/// - 错误收集
/// - 导入/导出信息
/// - 方法绑定
/// - Trait 表
/// - Native 函数签名
/// base 的语义归属：类型空间 / 类型值空间 / 未知（issue #180 F 组）。
///
/// typechecker 据此把 `X.字段 = 右值` 分流到类型空间或类型值空间，
/// 不靠语法形式 / 大小写，查类型表与变量表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseKind {
    /// base 是类型（在类型表）→ 类型空间
    TypeSpace,
    /// base 是变量/实例（在变量表）→ 类型值空间
    ValueSpace,
    /// 既非类型也非已知变量
    Unknown,
}

#[derive(Debug, Default)]
pub struct TypeEnvironment {
    pub vars: HashMap<String, PolyType>,
    pub types: HashMap<String, PolyType>,
    pub solver: TypeConstraintSolver,
    pub errors: TypeErrorCollector,
    /// 导入追踪 - 模块导入信息
    /// 包含源模块ID用于访问控制
    pub imports: Vec<ImportInfo>,
    /// 当前模块的导出项
    pub exports: HashSet<String>,
    /// 方法绑定关系: "Type.method" -> FunctionType
    /// 用于存储显式绑定和 pub 自动绑定
    pub method_bindings: HashMap<String, MonoType>,
    /// 模块名称
    pub module_name: String,
    /// 重载候选存储: 函数名 -> 多个重载版本
    /// 用于支持函数重载解析
    pub overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>>,
    /// Trait 表：存储所有已解析的 Trait 定义和实现
    pub trait_table: crate::frontend::core::types::TraitTable,
    /// Native 函数签名表：存储已注册的 native 函数类型签名
    /// Key: 函数名（如 "std.io.println"），Value: 函数类型
    pub native_signatures: HashMap<String, MonoType>,
    /// 模块注册表 - 提供统一的模块查询接口
    pub module_registry: crate::frontend::module::registry::ModuleRegistry,
    /// Const 函数表 - 存储编译期常量函数
    /// 用于值依赖类型的编译期求值
    pub const_functions: HashMap<String, ConstFunction>,
    /// 泛型类型定义模板表
    /// 存储泛型类型构造器的模板，用于 List(Int) → { data: Array(Int), length: Int } 的展开
    pub generic_type_defs: HashMap<String, GenericTypeDef>,
    /// 编译期谓词定义表
    /// 存储已注册的编译期谓词模板，供 PredicateResolver 使用
    pub predicate_defs:
        HashMap<String, crate::frontend::core::typecheck::predicate_resolver::PredicateDef>,
}

impl TypeEnvironment {
    /// 创建新的类型环境
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建新的类型环境（带模块名）
    pub fn new_with_module(module_name: String) -> Self {
        Self {
            module_name,
            trait_table: crate::frontend::core::types::TraitTable::default(),
            module_registry: crate::frontend::module::registry::ModuleRegistry::with_std(),
            ..Self::default()
        }
    }

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.vars.insert(name, poly);
    }

    /// 添加函数绑定（支持方法绑定）
    ///
    /// 统一处理 Binding 的注册：
    /// - 如果 `type_name` 存在，则为方法绑定，同时注册到 vars 和 method_bindings
    /// - 否则仅注册到 vars
    ///
    /// 这确保了 Binding 被正确转换为 MonoType::Fn 并注册到环境
    pub fn add_fn_binding(
        &mut self,
        name: &str,
        type_name: Option<&str>,
        fn_type: MonoType,
    ) {
        // 注册到 vars
        self.vars
            .insert(name.to_string(), PolyType::mono(fn_type.clone()));

        // 如果有 type_name，注册为方法绑定
        if let Some(ty) = type_name {
            self.add_method_binding(ty, name, fn_type);
        }
    }

    /// 自动绑定 pub 函数到类型
    ///
    /// 根据第一个参数的类型自动将函数绑定到该类型。
    /// 例如: pub distance: (self: Point, other: Point) -> Float 自动绑定为 Point.distance
    ///
    /// - 如果第一个参数类型是 TypeRef 且该类型在当前模块定义，则绑定
    /// - 否则不做任何操作
    pub fn auto_bind_to_type(
        &mut self,
        fn_name: &str,
        param_types: &[MonoType],
        fn_type: MonoType,
    ) {
        if param_types.is_empty() {
            return;
        }

        // 获取第一个参数的类型名称
        let first_param_ty = &param_types[0];
        if let MonoType::TypeRef(type_name) = first_param_ty {
            // 检查该类型是否在当前模块中定义
            if self.types.contains_key(type_name) {
                // 绑定方法到类型
                self.add_method_binding(type_name, fn_name, fn_type);
            }
        }
    }

    /// 获取变量类型
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.vars.get(name)
    }

    /// 获取求解器
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        &mut self.solver
    }

    /// 添加类型定义
    pub fn add_type(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.types.insert(name, poly);
    }

    /// 获取类型定义
    pub fn get_type(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.types.get(name)
    }

    /// 语义解析 base：是类型还是值（issue #180 F 组）。
    /// 不靠语法形式 / 大小写——查类型表（类型空间）与变量表（类型值空间）。
    pub fn resolve_base_kind(
        &self,
        name: &str,
    ) -> BaseKind {
        if self.types.contains_key(name) {
            BaseKind::TypeSpace
        } else if self.vars.contains_key(name) {
            BaseKind::ValueSpace
        } else {
            BaseKind::Unknown
        }
    }

    /// 添加泛型类型定义模板
    ///
    /// 记录泛型类型构造器的模板信息，用于后续的类型实例化展开。
    pub fn add_generic_type_def(
        &mut self,
        name: String,
        def: GenericTypeDef,
    ) {
        self.generic_type_defs.insert(name, def);
    }

    /// 实例化泛型类型（静态方法，StatementChecker 也可调用）
    ///
    /// Layer 1: 类型匹配 — 验证 const 参数类型与 const_binders 的类型声明匹配
    /// Layer 2: 值约束求值 — 求值约束表达式（const 泛型约束）
    pub fn instantiate_generic_type(
        def: &GenericTypeDef,
        args: &[MonoType],
    ) -> Result<MonoType, crate::util::diagnostic::Diagnostic> {
        use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

        let type_arg_count = def.type_param_names.len();
        let const_arg_count = def.poly.const_binders.len();

        if args.len() != type_arg_count + const_arg_count {
            return Err(ErrorCodeDefinition::argument_count_mismatch(
                "generic type",
                type_arg_count + const_arg_count,
                args.len(),
            )
            .build());
        }

        let type_args = &args[..type_arg_count];
        let const_args = &args[type_arg_count..];

        let body = Self::replace_type_params(&def.poly.body, &def.type_param_names, type_args);

        // const 验证（Layer 1 + Layer 2）
        if !const_args.is_empty() {
            // Layer 1: 类型匹配
            crate::frontend::core::typecheck::inference::bounds::validate_const_args(
                &def.poly.const_binders,
                const_args,
            )?;

            // Layer 2: 值约束求值
            let checker = crate::frontend::core::typecheck::inference::bounds::BoundsChecker::new();
            let result = checker.check_const_bounds(&def.poly.const_binders, const_args);
            match result {
                crate::frontend::core::typecheck::proof::verdict::ProofResult::Proved => {}
                crate::frontend::core::typecheck::proof::verdict::ProofResult::Disproved(_) => {
                    let var_info = def
                        .poly
                        .const_binders
                        .iter()
                        .zip(const_args.iter())
                        .filter_map(|(b, a)| {
                            if let crate::frontend::core::types::MonoType::Literal {
                                value, ..
                            } = a
                            {
                                Some(format!("{} = {}", b.name, value))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(Diagnostic::error(
                        "E1062".to_string(),
                        format!("const 泛型约束失败 ({})", var_info),
                        "修改 const 参数值使其满足约束".to_string(),
                        None,
                    ));
                }
                crate::frontend::core::typecheck::proof::verdict::ProofResult::Unproven {
                    ..
                } => {
                    // 约束无法求值 — 允许编译继续
                }
            }
        }

        Ok(Self::resolve_type_refs(&body))
    }

    /// 按名称查找并实例化泛型类型（TypeEnvironment 便捷方法）
    pub fn instantiate_generic_type_by_name(
        &self,
        name: &str,
        args: &[MonoType],
    ) -> Result<MonoType, crate::util::diagnostic::Diagnostic> {
        let def = self.generic_type_defs.get(name).ok_or_else(|| {
            crate::util::diagnostic::ErrorCodeDefinition::unknown_type(name).build()
        })?;
        Self::instantiate_generic_type(def, args)
    }

    /// 替换类型参数：将 TypeRef(param_name) 替换为具体的类型实参
    fn replace_type_params(
        ty: &MonoType,
        param_names: &[String],
        args: &[MonoType],
    ) -> MonoType {
        match ty {
            MonoType::TypeRef(name) => {
                // Check if this TypeRef is a type parameter name
                if let Some(pos) = param_names.iter().position(|p| p == name) {
                    if let Some(replacement) = args.get(pos) {
                        return replacement.clone();
                    }
                }
                ty.clone()
            }
            MonoType::Struct(s) => {
                let new_fields: Vec<(String, MonoType)> = s
                    .fields
                    .iter()
                    .map(|(name, field_ty)| {
                        (
                            name.clone(),
                            Self::replace_type_params(field_ty, param_names, args),
                        )
                    })
                    .collect();
                MonoType::Struct(crate::frontend::core::types::mono::StructType {
                    name: s.name.clone(),
                    fields: new_fields,
                    methods: s.methods.clone(),
                    field_mutability: s.field_mutability.clone(),
                    field_has_default: s.field_has_default.clone(),
                    interfaces: s.interfaces.clone(),
                })
            }
            MonoType::List(elem) => {
                let new_elem = Self::replace_type_params(elem, param_names, args);
                MonoType::List(Box::new(new_elem))
            }
            MonoType::Option(elem) => {
                let new_elem = Self::replace_type_params(elem, param_names, args);
                MonoType::Option(Box::new(new_elem))
            }
            MonoType::Result(ok, err) => {
                let new_ok = Self::replace_type_params(ok, param_names, args);
                let new_err = Self::replace_type_params(err, param_names, args);
                MonoType::Result(Box::new(new_ok), Box::new(new_err))
            }
            MonoType::Tuple(elems) => {
                let new_elems: Vec<MonoType> = elems
                    .iter()
                    .map(|e| Self::replace_type_params(e, param_names, args))
                    .collect();
                MonoType::Tuple(new_elems)
            }
            MonoType::Dict(k, v) => {
                let new_k = Self::replace_type_params(k, param_names, args);
                let new_v = Self::replace_type_params(v, param_names, args);
                MonoType::Dict(Box::new(new_k), Box::new(new_v))
            }
            MonoType::Set(elem) => {
                let new_elem = Self::replace_type_params(elem, param_names, args);
                MonoType::Set(Box::new(new_elem))
            }
            MonoType::Fn {
                params,
                return_type,
            } => {
                let new_params: Vec<MonoType> = params
                    .iter()
                    .map(|p| Self::replace_type_params(p, param_names, args))
                    .collect();
                let new_ret = Self::replace_type_params(return_type, param_names, args);
                MonoType::Fn {
                    params: new_params,
                    return_type: Box::new(new_ret),
                }
            }
            MonoType::Arc(elem) => {
                let new_elem = Self::replace_type_params(elem, param_names, args);
                MonoType::Arc(Box::new(new_elem))
            }
            MonoType::Range { elem_type } => {
                let new_elem = Self::replace_type_params(elem_type, param_names, args);
                MonoType::Range {
                    elem_type: Box::new(new_elem),
                }
            }
            MonoType::Generic {
                name,
                args: generic_args,
            } => {
                let new_args = generic_args
                    .iter()
                    .map(|a| Self::replace_type_params(a, param_names, args))
                    .collect();
                MonoType::Generic {
                    name: name.clone(),
                    args: new_args,
                }
            }
            _ => ty.clone(),
        }
    }

    /// 解析 TypeRef 中的内置类型名
    /// TypeRef("Int") → Int(64), TypeRef("Float") → Float(64), 等等。
    fn resolve_type_refs(ty: &MonoType) -> MonoType {
        match ty {
            MonoType::TypeRef(name) => {
                MonoType::from_builtin_name(name).unwrap_or_else(|| ty.clone())
            }
            MonoType::Struct(s) => {
                let new_fields: Vec<(String, MonoType)> = s
                    .fields
                    .iter()
                    .map(|(name, field_ty)| (name.clone(), Self::resolve_type_refs(field_ty)))
                    .collect();
                MonoType::Struct(crate::frontend::core::types::mono::StructType {
                    name: s.name.clone(),
                    fields: new_fields,
                    methods: s.methods.clone(),
                    field_mutability: s.field_mutability.clone(),
                    field_has_default: s.field_has_default.clone(),
                    interfaces: s.interfaces.clone(),
                })
            }
            MonoType::List(elem) => MonoType::List(Box::new(Self::resolve_type_refs(elem))),
            MonoType::Option(elem) => MonoType::Option(Box::new(Self::resolve_type_refs(elem))),
            MonoType::Result(ok, err) => MonoType::Result(
                Box::new(Self::resolve_type_refs(ok)),
                Box::new(Self::resolve_type_refs(err)),
            ),
            MonoType::Tuple(elems) => {
                MonoType::Tuple(elems.iter().map(Self::resolve_type_refs).collect())
            }
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(Self::resolve_type_refs(k)),
                Box::new(Self::resolve_type_refs(v)),
            ),
            MonoType::Set(elem) => MonoType::Set(Box::new(Self::resolve_type_refs(elem))),
            MonoType::Generic { name, args } => {
                let new_args = args.iter().map(Self::resolve_type_refs).collect();
                MonoType::Generic {
                    name: name.clone(),
                    args: new_args,
                }
            }
            MonoType::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params.iter().map(Self::resolve_type_refs).collect(),
                return_type: Box::new(Self::resolve_type_refs(return_type)),
            },
            _ => ty.clone(),
        }
    }

    /// 添加方法绑定（类型空间登记 chokepoint，issue #180 F 组）
    ///
    /// 类型空间：写入该类型的 `StructType.methods`（结构化存储）。
    /// 兼容镜像：迁移期保留平表 `method_bindings`，供未迁移读侧使用（P4 删除）。
    /// 例如: Point.distance = distance → Point 的 StructType.methods["distance"] = fn_type
    pub fn add_method_binding(
        &mut self,
        type_name: &str,
        method_name: &str,
        fn_type: MonoType,
    ) {
        // 类型空间：StructType.methods
        if let Some(poly) = self.types.get_mut(type_name) {
            if let MonoType::Struct(ref mut st) = poly.body {
                st.methods
                    .insert(method_name.to_string(), PolyType::mono(fn_type.clone()));
            }
        }
        // 兼容镜像（P4 删除）
        let key = format!("{}.{}", type_name, method_name);
        self.method_bindings.insert(key.clone(), fn_type);
        // 方法绑定也导出
        self.exports.insert(key);
    }

    /// 获取方法绑定
    pub fn get_method_binding(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<&MonoType> {
        let key = format!("{}.{}", type_name, method_name);
        self.method_bindings.get(&key)
    }

    /// 添加导出项
    pub fn add_export(
        &mut self,
        name: &str,
    ) {
        self.exports.insert(name.to_string());
    }

    /// 检查是否是导出项
    pub fn is_exported(
        &self,
        name: &str,
    ) -> bool {
        self.exports.contains(name)
    }

    /// 检查名称是否可见（可从当前模块访问）
    ///
    /// 一个名称在以下情况下可见：
    /// 1. 在当前模块中定义
    /// 2. 被当前模块导出
    /// 3. 从导入了该名称的模块导入
    pub fn is_visible(
        &self,
        name: &str,
    ) -> bool {
        // 当前模块定义的变量总是可见的
        if self.vars.contains_key(name) {
            return true;
        }
        // 当前模块定义的类型总是可见的
        if self.types.contains_key(name) {
            return true;
        }
        // 当前模块导出的内容可见
        if self.exports.contains(name) {
            return true;
        }
        false
    }

    // ============ Trait 相关方法 ============

    /// 添加 Trait 定义
    pub fn add_trait(
        &mut self,
        definition: crate::frontend::core::types::TraitDefinition,
    ) {
        self.trait_table.add_trait(definition);
    }

    /// 获取 Trait 定义
    pub fn get_trait(
        &self,
        name: &str,
    ) -> Option<&crate::frontend::core::types::TraitDefinition> {
        self.trait_table.get_trait(name)
    }

    /// 检查 Trait 是否已定义
    pub fn has_trait(
        &self,
        name: &str,
    ) -> bool {
        self.trait_table.has_trait(name)
    }

    /// 添加 Trait 实现
    ///
    /// 返回 `true` 表示新插入，`false` 表示已存在（冲突）
    pub fn add_trait_impl(
        &mut self,
        impl_: crate::frontend::core::types::TraitImplementation,
    ) -> bool {
        self.trait_table.add_impl(impl_)
    }

    /// 检查类型是否实现了 Trait
    pub fn has_trait_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> bool {
        self.trait_table.has_impl(trait_name, for_type)
    }

    /// 获取 Trait 实现
    pub fn get_trait_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> Option<&crate::frontend::core::types::TraitImplementation> {
        self.trait_table.get_impl(trait_name, for_type)
    }

    /// 注册 native 函数签名
    pub fn add_native_signature(
        &mut self,
        name: &str,
        sig: MonoType,
    ) {
        self.native_signatures.insert(name.to_string(), sig);
    }

    /// 获取 native 函数签名
    pub fn get_native_signature(
        &self,
        name: &str,
    ) -> Option<&MonoType> {
        self.native_signatures.get(name)
    }

    /// 检查是否是已注册的 native 函数
    pub fn is_native_function(
        &self,
        name: &str,
    ) -> bool {
        self.native_signatures.contains_key(name)
    }

    /// 注册 const 函数
    /// 用于值依赖类型的编译期求值
    pub fn add_const_function(
        &mut self,
        name: String,
        func: ConstFunction,
    ) {
        self.const_functions.insert(name, func);
    }

    /// 获取 const 函数
    pub fn get_const_function(
        &self,
        name: &str,
    ) -> Option<&ConstFunction> {
        self.const_functions.get(name)
    }

    /// 检查是否是 const 函数
    pub fn is_const_function(
        &self,
        name: &str,
    ) -> bool {
        self.const_functions.contains_key(name)
    }
}
