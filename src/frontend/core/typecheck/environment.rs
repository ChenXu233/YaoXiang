//! 类型环境模块
//!
//! 管理类型检查过程中的所有状态信息

use std::collections::{HashMap, HashSet};

use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::types::eval::const_eval::ConstFunction;

use super::passes::overload;
use super::types::ImportInfo;

/// 类型错误收集器
pub type TypeErrorCollector = crate::util::diagnostic::ErrorCollector;

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

/// RFC-011a §5.3: 实现证明——某类型完整实现了某接口的编译期凭证。
/// 纯编译期概念，运行时擦除（RFC-011a §6.4）；阶段 3 动态分发的类型收集
/// 将以此为依据枚举某接口的全部实现类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplementationProof {
    /// 实现类型名（如 "Dog"）
    pub type_name: String,
    /// 接口名（如 "Animal"）
    pub interface_name: String,
    /// 已验证签名匹配的方法名列表
    pub methods: Vec<String>,
}

/// RFC-011b: 接口实现登记表条目。
///
/// `ImplementationProof` 不带类型实参，无法区分同一接口的不同实例化
/// （`Add(Int, Float, Float)` 与 `Add(Point, Point, Point)`）；运算符查询
/// 与约束求解需要实参维度，故另立此表，与 proof 在实例化检查通过时
/// 同步写入（check_interface_instantiation）。
#[derive(Debug, Clone)]
pub struct InterfaceImplEntry {
    /// 实现类型头名（用户条目如 "Point"；native 条目为 "Int" 等基本类型名）
    pub impl_type: String,
    /// 接口类型实参，与接口形参表一一对应（如 [Point, Point, Point]）
    pub args: Vec<MonoType>,
    /// 已验证的方法名
    pub methods: Vec<String>,
    /// 核心默认登记：原生指令路径，无用户方法绑定（运算符代码生成不走方法调用）
    pub native: bool,
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
///
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
    /// RFC-011a: 已通过的接口实现证明（编译期，运行时擦除）
    pub implementation_proofs: Vec<ImplementationProof>,
    /// RFC-011b: 接口实现登记表（接口名 → 实例化条目，带类型实参维度）。
    /// 运算符查询与约束求解的唯一判据；不经普通名字解析（§名字与登记）。
    pub interface_impl_registry: HashMap<String, Vec<InterfaceImplEntry>>,
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

    /// RFC-011b: 登记接口实现（完全同参的重复条目去重）
    pub fn add_interface_impl(
        &mut self,
        interface_name: &str,
        entry: InterfaceImplEntry,
    ) {
        let list = self
            .interface_impl_registry
            .entry(interface_name.to_string())
            .or_default();
        if !list
            .iter()
            .any(|e| e.impl_type == entry.impl_type && e.args == entry.args)
        {
            list.push(entry);
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
        use crate::util::diagnostic::ErrorCodeDefinition;

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
                    // #322 M3：走注册表快捷方法（i18n 模板渲染）
                    return Err(ErrorCodeDefinition::const_constraint_failed(&var_info).build());
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
    pub(crate) fn replace_type_params(
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
            MonoType::Generic { name, args } if name == "Range" && args.len() == 1 => {
                let new_elem = Self::replace_type_params(&args[0], param_names, args);
                MonoType::Generic {
                    name: "Range".into(),
                    args: vec![new_elem],
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
            // RFC-011a §3：impl 签名 = 接口签名经 Self↦具体类型替换——替换必须
            // 结构递归。此前 Ref 落入兜底原样克隆，接口 `speak: (self: &Self)` 与
            // impl `(self: &Dog)` 比较时 expected 侧内层仍是不变元 → E1099 误报。
            MonoType::Ref { mutable, inner } => MonoType::Ref {
                mutable: *mutable,
                inner: Box::new(Self::replace_type_params(inner, param_names, args)),
            },
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

    /// 添加方法绑定
    /// 例如: Point.distance = distance 存储为 "Point.distance" -> fn_type
    pub fn add_method_binding(
        &mut self,
        type_name: &str,
        method_name: &str,
        fn_type: MonoType,
    ) {
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

    /// 类型名在本环境里是否可解析（#371/#372）。
    ///
    /// **这是「合法类型名」的唯一真相源。** 此前不存在这样一个查询，各消费点
    /// 各自判断（有查 `types`、有查 `generic_type_defs`、有查 `trait_table`），
    /// 而 `MonoType::from(ast::Type)` 对 `Type::Name` 干脆不查——直接包成
    /// `TypeRef(name)`。后果是形参/字段里的错拼类型名静默变成一个「合法类型」，
    /// 该参数上的类型检查随之全部失效（任意实参都能传）。
    ///
    /// 允许集（按来源）：
    /// - **内置名**：`Int`/`String`/... —— `from_builtin_name`
    /// - **编译器级顶类型**：`Any` —— 与 `Void`/`Never` 同族，solver 特判，不需声明
    /// - **已登记类型**：`types`（本地类型定义 + `use` 导入的类型）
    /// - **泛型构造器**：`generic_type_defs`（`Vec`/`Array` + 用户泛型类型）
    /// - **接口/特质**：`trait_table`（存在类型位置，RFC-011a）
    /// - **std 导出类型**：`module_registry` 里标记为 `Type` 的导出
    ///   （`Error`/`File`/`Iterator`... —— 它们由 Rust 侧 `export!` 宏登记，
    ///   不进 `types`；此前完全无人可查，却能隐式通过）
    ///
    /// 注意：**不含**泛型参数名（`T`/`Self`）与编译期值参数名——那些是声明处
    /// 绑定的局部名字，由调用方（`TypeNameScope`）先行剔除后再查询。
    pub fn resolves_type_name(
        &self,
        name: &str,
    ) -> bool {
        if MonoType::from_builtin_name(name).is_some() {
            return true;
        }
        // `Any` 是编译器级顶类型：`from_builtin_name` 不含它，solver 直接特判
        // （`solver.rs` 的 `(TypeRef(n), _) if n == "Any" => Ok(())`）。
        // `Self` 是语言关键字：在类型体/接口方法签名里指当前类型，
        // 由接口实例化阶段替换（RFC-011a），同样没有也不可能有声明。
        if name == "Any" || name == "Self" {
            return true;
        }
        // 编译器级容器/引用类型：这些名字在类型位置由编译器直接识别
        // （`mono.rs` 的 `is_vec`/`is_arc`/`is_weak`/`is_range` 等谓词、
        // `type_name()` 的专门臂），不需要也不存在源码声明。
        // 注意：它们只在**类型位置**成立；值位置的构造器另有限制
        // （见 `is_builtin_generic_type_name`，那里只放 `Vec`/`Array`）。
        if Self::is_compiler_container_type(name) {
            return true;
        }
        if self.types.contains_key(name) || self.generic_type_defs.contains_key(name) {
            return true;
        }
        if self.has_trait(name) {
            return true;
        }
        // std 导出类型：`Error`/`File`/`Native`/`Weak`/`Arc` 等由 Rust 侧登记，
        // 只活在模块注册表里。逐个 std 子模块查同名 Type 导出。
        self.std_export_type_name(name)
    }

    /// 编译器直接识别的容器/引用类型名（类型位置合法，无源码声明）。
    ///
    /// 与 `mono.rs` 的 `is_vec`/`is_arc`/`is_weak`/`is_range` 等谓词同源——
    /// 那些谓词按名字判定，本表列出同一批名字，供名字解析查询。
    /// 新增此类内置容器时**两处都要改**（谓词 + 本表）。
    fn is_compiler_container_type(name: &str) -> bool {
        matches!(
            name,
            "Vec"
                | "Array"
                | "List"
                | "Dict"
                | "Tuple"
                | "Option"
                | "Result"
                | "Range"
                | "Bytes"
                | "Set"
                | "Arc"
                | "Weak"
        )
    }

    /// 名字是否是某个 std 子模块导出的**类型**（`ExportKind::Type`）。
    ///
    /// 兜底顺序上放在最后：它要遍历子模块，比前几项的表查询贵，
    /// 而绝大多数名字在前面就命中了。
    fn std_export_type_name(
        &self,
        name: &str,
    ) -> bool {
        use crate::frontend::module::ExportKind;
        self.module_registry
            .std_submodule_names()
            .iter()
            .any(|sub| {
                let path = format!("std.{sub}");
                self.module_registry
                    .get(&path)
                    .and_then(|m| m.get_export(name))
                    .is_some_and(|e| matches!(e.kind, ExportKind::Type))
            })
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
