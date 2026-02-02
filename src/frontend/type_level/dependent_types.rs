//! RFC-011 依赖类型支持
//!
//! 提供有限的依赖类型能力：
//! - 类型族: 类型依赖于类型参数
//! - 关联类型: 类型中定义的类型成员
//!
//! RFC-011 设计中，YaoXiang 的依赖类型主要用于：
//! ```yaoxiang
//! type Iterator[T] = {
//!     Item: T,           # 关联类型
//!     next: (Self) -> Option[T],
//! }
//!
//! type Container[T] = {
//!     Item: T,
//!     IteratorType: Iterator[T],  # 关联类型依赖于 T
//! }
//! ```

use crate::frontend::core::type_system::MonoType;
use std::collections::HashMap;

/// 关联类型定义
///
/// 表示类型中定义的关联类型成员
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssociatedType {
    /// 关联类型名称
    pub name: String,

    /// 关联类型的定义（可能是类型参数）
    pub definition: AssociatedTypeDef,
}

impl AssociatedType {
    /// 创建新的关联类型
    pub fn new(
        name: String,
        definition: AssociatedTypeDef,
    ) -> Self {
        Self { name, definition }
    }
}

/// 关联类型定义
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssociatedTypeDef {
    /// 直接类型
    Direct(MonoType),

    /// 类型参数引用
    TypeParam(String),

    /// 类型参数上的变换
    /// 例如: `Iterator[T]` 表示对类型参数 T 应用 Iterator
    Applied {
        /// 类型构造器名称
        constructor: String,

        /// 类型参数
        arguments: Vec<AssociatedTypeDef>,
    },
}

impl AssociatedTypeDef {
    /// 检查是否包含未绑定的类型参数
    pub fn has_unbound_params(
        &self,
        bound_params: &[String],
    ) -> bool {
        match self {
            AssociatedTypeDef::Direct(_) => false,
            AssociatedTypeDef::TypeParam(name) => !bound_params.contains(name),
            AssociatedTypeDef::Applied { arguments, .. } => arguments
                .iter()
                .any(|arg| arg.has_unbound_params(bound_params)),
        }
    }

    /// 替换类型参数
    pub fn substitute(
        &self,
        substitutions: &HashMap<String, MonoType>,
    ) -> Self {
        match self {
            AssociatedTypeDef::Direct(ty) => {
                // 替换 MonoType 中的类型引用
                if let MonoType::TypeRef(type_name) = ty {
                    if let Some(replacement) = substitutions.get(type_name) {
                        return AssociatedTypeDef::Direct(replacement.clone());
                    }
                }
                AssociatedTypeDef::Direct(ty.clone())
            }
            AssociatedTypeDef::TypeParam(name) => {
                if let Some(replacement) = substitutions.get(name) {
                    AssociatedTypeDef::Direct(replacement.clone())
                } else {
                    AssociatedTypeDef::TypeParam(name.clone())
                }
            }
            AssociatedTypeDef::Applied {
                constructor,
                arguments,
            } => AssociatedTypeDef::Applied {
                constructor: constructor.clone(),
                arguments: arguments
                    .iter()
                    .map(|arg| arg.substitute(substitutions))
                    .collect(),
            },
        }
    }

    /// 转换为 MonoType
    pub fn into_type(self) -> MonoType {
        match self {
            AssociatedTypeDef::Direct(ty) => ty,
            AssociatedTypeDef::TypeParam(_) => {
                // 类型参数转换为 TypeRef（使用名称）
                MonoType::TypeRef("Unknown".to_string())
            }
            AssociatedTypeDef::Applied { constructor, .. } => {
                // 类型应用转换为 TypeRef
                MonoType::TypeRef(constructor)
            }
        }
    }
}

/// 类型族
///
/// 表示一个依赖于类型参数的泛型类型定义
///
/// # 示例
/// ```yaoxiang
/// type Result[T, E] = Ok(T) | Err(E)  # Result 是 T 和 E 的类型族
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeFamily {
    /// 类型族名称
    pub name: String,

    /// 类型参数
    pub type_params: Vec<String>,

    /// 关联类型
    pub associated_types: Vec<AssociatedType>,

    /// 定义（类型表达式）
    pub definition: AssociatedTypeDef,
}

impl TypeFamily {
    /// 创建新的类型族
    pub fn new(
        name: String,
        type_params: Vec<String>,
        associated_types: Vec<AssociatedType>,
        definition: AssociatedTypeDef,
    ) -> Self {
        Self {
            name,
            type_params,
            associated_types,
            definition,
        }
    }

    /// 实例化类型族
    pub fn instantiate(
        &self,
        args: &[MonoType],
    ) -> Option<AssociatedTypeDef> {
        // 检查参数数量是否匹配
        if self.type_params.len() != args.len() {
            return None;
        }

        // 构建替换映射
        let substitutions: HashMap<String, MonoType> = self
            .type_params
            .iter()
            .zip(args.iter())
            .map(|(param, arg)| (param.clone(), arg.clone()))
            .collect();

        // 替换定义中的类型参数
        Some(self.definition.substitute(&substitutions))
    }

    /// 获取关联类型
    pub fn get_associated_type(
        &self,
        name: &str,
    ) -> Option<&AssociatedType> {
        self.associated_types.iter().find(|at| at.name == name)
    }

    /// 实例化关联类型
    pub fn instantiate_associated_type(
        &self,
        name: &str,
        args: &[MonoType],
    ) -> Option<MonoType> {
        let associated = self.get_associated_type(name)?;
        let substitutions: HashMap<String, MonoType> = self
            .type_params
            .iter()
            .zip(args.iter())
            .map(|(param, arg)| (param.clone(), arg.clone()))
            .collect();

        Some(associated.definition.substitute(&substitutions).into_type())
    }

    /// 获取所有类型参数
    pub fn type_params(&self) -> &[String] {
        &self.type_params
    }

    /// 获取所有关联类型
    pub fn associated_types(&self) -> &[AssociatedType] {
        &self.associated_types
    }
}

/// 依赖类型环境
///
/// 管理类型族和关联类型的注册与查找
#[derive(Debug, Clone, Default)]
pub struct DependentTypeEnv {
    /// 类型族映射
    type_families: HashMap<String, TypeFamily>,
}

impl DependentTypeEnv {
    /// 创建新的依赖类型环境
    pub fn new() -> Self {
        Self {
            type_families: HashMap::new(),
        }
    }

    /// 注册类型族
    pub fn register_type_family(
        &mut self,
        family: TypeFamily,
    ) {
        self.type_families.insert(family.name.clone(), family);
    }

    /// 查找类型族
    pub fn get_type_family(
        &self,
        name: &str,
    ) -> Option<&TypeFamily> {
        self.type_families.get(name)
    }

    /// 检查类型是否是类型族实例
    pub fn is_type_family_instance(
        &self,
        _ty: &MonoType,
    ) -> Option<&TypeFamily> {
        // 简化实现：暂不检查类型是否为类型族实例
        None
    }
}
