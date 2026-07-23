//! Trait 数据定义与求解
//!
//! 核心 trait 系统：数据结构 + 标准库初始化 + trait 满足性检查 + 自动派生。

use std::collections::HashMap;
use crate::frontend::core::types::mono::MonoType;

// ============================================================================
// 数据结构
// ============================================================================

/// Trait 方法签名
#[derive(Debug, Clone)]
pub struct TraitMethodSignature {
    pub name: String,
    pub params: Vec<MonoType>,
    pub return_type: MonoType,
    pub is_static: bool,
}

/// Trait 定义
#[derive(Debug, Clone)]
pub struct TraitDefinition {
    pub name: String,
    pub methods: HashMap<String, TraitMethodSignature>,
    pub parent_traits: Vec<String>,
    pub generic_params: Vec<String>,
    pub span: Option<crate::util::span::Span>,
    pub is_marker: bool,
}

/// Trait 边界（用于泛型约束）
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TraitBound {
    pub trait_name: String,
    pub self_type: MonoType,
}

pub type TraitBounds = Vec<TraitBound>;

/// Trait 实现
#[derive(Debug, Clone)]
pub struct TraitImplementation {
    pub trait_name: String,
    pub for_type_name: String,
    pub methods: HashMap<String, MonoType>,
}

// ============================================================================
// TraitTable
// ============================================================================

/// Trait 表 - 存储所有已解析的 Trait 定义和实现
#[derive(Debug, Clone, Default)]
pub struct TraitTable {
    traits: HashMap<String, TraitDefinition>,
    implementations: HashMap<(String, String), TraitImplementation>,
}

impl TraitTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取类型在 trait 注册中使用的 key
    ///
    /// `type_name()` 返回 "int32"、"float64" 等，但 trait 注册用 "Int"、"Float"。
    fn trait_key(ty: &MonoType) -> String {
        match ty {
            MonoType::Int(_) => "Int".to_string(),
            MonoType::Float(_) => "Float".to_string(),
            MonoType::Bool => "Bool".to_string(),
            MonoType::Char => "Char".to_string(),
            MonoType::String => "String".to_string(),
            MonoType::Bytes => "Bytes".to_string(),
            MonoType::Void => "Void".to_string(),
            MonoType::Never => "Never".to_string(),
            _ => ty.type_name(),
        }
    }

    // ---- 基础 CRUD ----

    pub fn add_trait(
        &mut self,
        definition: TraitDefinition,
    ) {
        self.traits.insert(definition.name.clone(), definition);
    }

    pub fn get_trait(
        &self,
        name: &str,
    ) -> Option<&TraitDefinition> {
        self.traits.get(name)
    }

    pub fn has_trait(
        &self,
        name: &str,
    ) -> bool {
        self.traits.contains_key(name)
    }

    pub fn has_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> bool {
        self.implementations
            .contains_key(&(trait_name.to_string(), for_type.to_string()))
    }

    pub fn get_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> Option<&TraitImplementation> {
        self.implementations
            .get(&(trait_name.to_string(), for_type.to_string()))
    }

    /// 添加 Trait 实现
    ///
    /// 返回 `true` 表示新插入，`false` 表示已存在（不覆盖）
    pub fn add_impl(
        &mut self,
        impl_: TraitImplementation,
    ) -> bool {
        let key = (impl_.trait_name.clone(), impl_.for_type_name.clone());
        if self.implementations.contains_key(&key) {
            return false;
        }
        self.implementations.insert(key, impl_);
        true
    }

    pub fn get_method_impl(
        &self,
        trait_name: &str,
        for_type: &str,
        method_name: &str,
    ) -> Option<&MonoType> {
        self.implementations
            .get(&(trait_name.to_string(), for_type.to_string()))
            .and_then(|impl_| impl_.methods.get(method_name))
    }

    pub fn trait_names(&self) -> impl Iterator<Item = &String> {
        self.traits.keys()
    }

    /// 获取所有已注册的 trait 实现
    pub fn all_implementations(
        &self
    ) -> impl Iterator<Item = (&(String, String), &TraitImplementation)> {
        self.implementations.iter()
    }

    // ---- trait 满足性检查 ----

    /// 检查 MonoType 是否满足某 trait（统一入口）
    ///
    /// 1. 先查 TraitTable 已注册的实现
    /// 2. 复合类型（Struct/Tuple/List 等）递归检查内部类型
    pub fn satisfies(
        &self,
        trait_name: &str,
        ty: &MonoType,
    ) -> bool {
        // 已注册实现
        if self.has_impl(trait_name, &Self::trait_key(ty)) {
            return true;
        }

        match ty {
            // 基本类型：已注册则 true，否则 false
            MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Bool
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes
            | MonoType::Void => false,

            // 结构体：所有字段都满足
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.satisfies(trait_name, f)),

            // 枚举：变体仅含名称（无关联数据），视为满足
            MonoType::Enum(_) => true,

            // 元组：所有元素都满足 → 元组满足
            MonoType::Tuple(elems) => elems.iter().all(|e| self.satisfies(trait_name, e)),

            // Arc：引用计数，Clone/Dup 自动满足
            MonoType::Arc(_) => matches!(trait_name, "Clone" | "Dup"),

            // &T（不可变引用令牌）：Dup 和 Clone 都满足
            MonoType::Ref { mutable: false, .. } => matches!(trait_name, "Clone" | "Dup"),

            // TypeRef：查表
            MonoType::TypeRef(name) => self.has_impl(trait_name, name),

            // List、Dict、Set、Option、Result、Fn、&mut T、其他：不自动满足
            _ => false,
        }
    }

    /// 检查 AST Type 是否满足某 trait（用于自动派生阶段，操作 AST 而非 MonoType）
    pub fn satisfies_ast_type(
        &self,
        trait_name: &str,
        ty: &crate::frontend::core::parser::ast::Type,
    ) -> bool {
        use crate::frontend::core::parser::ast::Type;
        match ty {
            Type::Name { name, .. } => self.has_impl(trait_name, name),
            Type::Int(_) => self.has_impl(trait_name, "Int"),
            Type::Float(_) => self.has_impl(trait_name, "Float"),
            Type::Char => self.has_impl(trait_name, "Char"),
            Type::String => self.has_impl(trait_name, "String"),
            Type::Bytes => self.has_impl(trait_name, "Bytes"),
            Type::Bool => self.has_impl(trait_name, "Bool"),
            Type::Void => self.has_impl(trait_name, "Void"),
            Type::Generic { name, args, .. } => {
                self.has_impl(trait_name, name)
                    && args
                        .iter()
                        .all(|arg| self.satisfies_ast_type(trait_name, arg))
            }
            Type::Option(inner) => self.satisfies_ast_type(trait_name, inner),
            Type::Result(ok, err) => {
                self.satisfies_ast_type(trait_name, ok) && self.satisfies_ast_type(trait_name, err)
            }
            Type::Tuple(elems) => elems.iter().all(|e| self.satisfies_ast_type(trait_name, e)),
            Type::Fn { .. } => false,
            _ => false,
        }
    }

    // ---- 自动派生 ----

    /// 内置可派生 trait 列表
    pub const BUILTIN_DERIVES: &[&str] = &["Clone", "Equal", "Debug"];

    /// 检查 Record 的所有字段是否都满足某 trait（AST 层面）
    pub fn can_auto_derive(
        &self,
        trait_name: &str,
        fields: &[crate::frontend::core::parser::ast::StructField],
    ) -> bool {
        fields
            .iter()
            .all(|f| self.satisfies_ast_type(trait_name, &f.ty))
    }

    /// 检查 MonoType 结构体是否可以自动派生某 trait
    pub fn can_auto_derive_for_monotype(
        &self,
        trait_name: &str,
        struct_ty: &crate::frontend::core::types::mono::StructType,
    ) -> bool {
        struct_ty
            .fields
            .iter()
            .all(|(_, f)| self.satisfies(trait_name, f))
    }

    /// 为 Record 类型生成自动派生实现
    pub fn generate_auto_derive(
        type_name: &str,
        trait_name: &str,
    ) -> Option<TraitImplementation> {
        let mut methods = HashMap::new();
        match trait_name {
            "Clone" => {
                methods.insert(
                    "clone".to_string(),
                    MonoType::Fn {
                        params: vec![MonoType::TypeRef("Self".to_string())],
                        return_type: Box::new(MonoType::TypeRef(type_name.to_string())),
                    },
                );
            }
            "Equal" => {
                methods.insert(
                    "equal".to_string(),
                    MonoType::Fn {
                        params: vec![
                            MonoType::TypeRef("Self".to_string()),
                            MonoType::TypeRef("Self".to_string()),
                        ],
                        return_type: Box::new(MonoType::Bool),
                    },
                );
            }
            "Debug" => {
                methods.insert(
                    "debug".to_string(),
                    MonoType::Fn {
                        params: vec![
                            MonoType::TypeRef("Self".to_string()),
                            MonoType::TypeRef("Formatter".to_string()),
                        ],
                        return_type: Box::new(MonoType::Void),
                    },
                );
            }
            _ => return None,
        }
        Some(TraitImplementation {
            trait_name: trait_name.to_string(),
            for_type_name: type_name.to_string(),
            methods,
        })
    }

    // ---- 类型属性 ----

    /// 原语值类型：编译器内置值复制，不属于 Dup
    pub fn is_primitive_value_type(ty: &MonoType) -> bool {
        matches!(
            ty,
            MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool | MonoType::Char
        )
    }

    // ---- 标准库初始化 ----

    /// 创建包含标准库 trait 定义和原语实现的 TraitTable
    pub fn with_std() -> Self {
        let mut table = Self::new();
        table.init_std_traits();
        table.init_primitive_impls();
        table
    }

    fn init_std_traits(&mut self) {
        // Clone
        self.add_trait(TraitDefinition {
            name: "Clone".to_string(),
            methods: HashMap::from([(
                "clone".to_string(),
                TraitMethodSignature {
                    name: "clone".to_string(),
                    params: vec![MonoType::TypeRef("Self".to_string())],
                    return_type: MonoType::TypeRef("Self".to_string()),
                    is_static: false,
                },
            )]),
            parent_traits: Vec::new(),
            generic_params: Vec::new(),
            span: None,
            is_marker: false,
        });

        // Dup（标记 trait）
        self.add_trait(TraitDefinition {
            name: "Dup".to_string(),
            methods: HashMap::new(),
            parent_traits: Vec::new(),
            generic_params: vec![],
            span: None,
            is_marker: true,
        });

        // Equal
        self.add_trait(TraitDefinition {
            name: "Equal".to_string(),
            methods: HashMap::from([(
                "equal".to_string(),
                TraitMethodSignature {
                    name: "equal".to_string(),
                    params: vec![
                        MonoType::TypeRef("Self".to_string()),
                        MonoType::TypeRef("Self".to_string()),
                    ],
                    return_type: MonoType::Bool,
                    is_static: false,
                },
            )]),
            parent_traits: Vec::new(),
            generic_params: Vec::new(),
            span: None,
            is_marker: false,
        });

        // Debug
        self.add_trait(TraitDefinition {
            name: "Debug".to_string(),
            methods: HashMap::from([(
                "debug".to_string(),
                TraitMethodSignature {
                    name: "debug".to_string(),
                    params: vec![
                        MonoType::TypeRef("Self".to_string()),
                        MonoType::TypeRef("Formatter".to_string()),
                    ],
                    return_type: MonoType::Void,
                    is_static: false,
                },
            )]),
            parent_traits: Vec::new(),
            generic_params: Vec::new(),
            span: None,
            is_marker: false,
        });

        // Iterator
        self.add_trait(TraitDefinition {
            name: "Iterator".to_string(),
            methods: HashMap::from([(
                "next".to_string(),
                TraitMethodSignature {
                    name: "next".to_string(),
                    params: vec![MonoType::TypeRef("Self".to_string())],
                    return_type: MonoType::TypeRef("Option".to_string()),
                    is_static: false,
                },
            )]),
            parent_traits: Vec::new(),
            generic_params: vec!["T".to_string()],
            span: None,
            is_marker: false,
        });

        // Resource（标记 trait）
        self.add_trait(TraitDefinition {
            name: "Resource".to_string(),
            methods: HashMap::new(),
            parent_traits: Vec::new(),
            generic_params: vec![],
            span: None,
            is_marker: true,
        });

        // 内置资源类型
        for type_name in &["FilePath", "HttpUrl", "DBUrl", "Console"] {
            self.add_impl(TraitImplementation {
                trait_name: "Resource".to_string(),
                for_type_name: type_name.to_string(),
                methods: HashMap::new(),
            });
        }
    }

    fn init_primitive_impls(&mut self) {
        let clone_fn = || -> HashMap<String, MonoType> {
            HashMap::from([(
                "clone".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::TypeRef("Self".to_string())],
                    return_type: Box::new(MonoType::TypeRef("Self".to_string())),
                },
            )])
        };
        let equal_fn = || -> HashMap<String, MonoType> {
            HashMap::from([(
                "equal".to_string(),
                MonoType::Fn {
                    params: vec![
                        MonoType::TypeRef("Self".to_string()),
                        MonoType::TypeRef("Self".to_string()),
                    ],
                    return_type: Box::new(MonoType::Bool),
                },
            )])
        };
        let debug_fn = || -> HashMap<String, MonoType> {
            HashMap::from([(
                "debug".to_string(),
                MonoType::Fn {
                    params: vec![
                        MonoType::TypeRef("Self".to_string()),
                        MonoType::TypeRef("Formatter".to_string()),
                    ],
                    return_type: Box::new(MonoType::Void),
                },
            )])
        };

        // 值类型：Clone + Equal + Debug（不是 Dup）
        for type_name in &["Int", "Float", "Bool", "Char"] {
            self.add_impl(TraitImplementation {
                trait_name: "Clone".into(),
                for_type_name: (*type_name).into(),
                methods: clone_fn(),
            });
            self.add_impl(TraitImplementation {
                trait_name: "Equal".into(),
                for_type_name: (*type_name).into(),
                methods: equal_fn(),
            });
            self.add_impl(TraitImplementation {
                trait_name: "Debug".into(),
                for_type_name: (*type_name).into(),
                methods: debug_fn(),
            });
        }

        // String: Clone + Dup + Equal + Debug
        self.add_impl(TraitImplementation {
            trait_name: "Clone".into(),
            for_type_name: "String".into(),
            methods: clone_fn(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Dup".into(),
            for_type_name: "String".into(),
            methods: HashMap::new(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Equal".into(),
            for_type_name: "String".into(),
            methods: equal_fn(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Debug".into(),
            for_type_name: "String".into(),
            methods: debug_fn(),
        });

        // Bytes: Clone + Dup + Debug（不是 Equal）
        self.add_impl(TraitImplementation {
            trait_name: "Clone".into(),
            for_type_name: "Bytes".into(),
            methods: clone_fn(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Dup".into(),
            for_type_name: "Bytes".into(),
            methods: HashMap::new(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Debug".into(),
            for_type_name: "Bytes".into(),
            methods: debug_fn(),
        });

        // Void: Dup + Equal + Debug（不是 Clone）
        self.add_impl(TraitImplementation {
            trait_name: "Dup".into(),
            for_type_name: "Void".into(),
            methods: HashMap::new(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Equal".into(),
            for_type_name: "Void".into(),
            methods: equal_fn(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Debug".into(),
            for_type_name: "Void".into(),
            methods: debug_fn(),
        });

        // Never: Dup + Equal + Debug（底部类型，无实例可构造）
        self.add_impl(TraitImplementation {
            trait_name: "Dup".into(),
            for_type_name: "Never".into(),
            methods: HashMap::new(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Equal".into(),
            for_type_name: "Never".into(),
            methods: equal_fn(),
        });
        self.add_impl(TraitImplementation {
            trait_name: "Debug".into(),
            for_type_name: "Never".into(),
            methods: debug_fn(),
        });
    }
}
