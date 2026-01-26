//! 依赖类型系统 (RFC-011 Phase 5)
//!
//! 实现 Vector、List 等依赖类型和依赖类型检查

use super::{
    TypeLevelError, TypeLevelValue, TypeFamily, ConditionalType, TypeLevelComputer,
    ConditionalTypeChecker,
};
use crate::frontend::typecheck::types::MonoType;
use std::fmt;

/// 依赖类型变体
#[derive(Debug, Clone, PartialEq)]
pub enum DependentVariant {
    /// 空列表（长度为0）
    Nil,
    /// 构造函数（长度为 N+1）
    Cons {
        head: Box<DependentType>,
        tail: Box<DependentType>,
    },
}

/// 依赖类型
#[derive(Debug, Clone, PartialEq)]
pub enum DependentType {
    /// Vector[T, N]：长度为 N 的 T 类型向量
    Vector {
        elem_type: Box<DependentType>,
        length: super::Nat,
    },
    /// List[T]：T 类型列表
    List { elem_type: Box<DependentType> },
    /// Option[T]：T 类型选项
    Option { elem_type: Box<DependentType> },
    /// 函数类型（依赖参数）
    Fn {
        params: Vec<DependentType>,
        return_type: Box<DependentType>,
    },
    /// 元组类型
    Tuple { elements: Vec<DependentType> },
    /// 类型引用
    TypeRef {
        name: String,
        args: Vec<DependentType>,
    },
    /// 构造的变体
    Variant(DependentVariant),
    /// 基本类型
    Base(MonoType),
    /// 依赖变量
    Var { name: String, index: usize },
}

impl DependentType {
    /// 创建 Vector 类型
    pub fn vector<T, N>(
        elem_type: T,
        length: N,
    ) -> Self
    where
        T: Into<DependentType>,
        N: Into<super::Nat>,
    {
        DependentType::Vector {
            elem_type: Box::new(elem_type.into()),
            length: length.into(),
        }
    }

    /// 创建 List 类型
    pub fn list<T>(elem_type: T) -> Self
    where
        T: Into<DependentType>,
    {
        DependentType::List {
            elem_type: Box::new(elem_type.into()),
        }
    }

    /// 创建 Option 类型
    pub fn option<T>(elem_type: T) -> Self
    where
        T: Into<DependentType>,
    {
        DependentType::Option {
            elem_type: Box::new(elem_type.into()),
        }
    }

    /// 创建函数类型
    pub fn fn_type<T>(
        params: Vec<T>,
        return_type: T,
    ) -> Self
    where
        T: Into<DependentType>,
    {
        DependentType::Fn {
            params: params.into_iter().map(|p| p.into()).collect(),
            return_type: Box::new(return_type.into()),
        }
    }

    /// 创建元组类型
    pub fn tuple<T>(elements: Vec<T>) -> Self
    where
        T: Into<DependentType>,
    {
        DependentType::Tuple {
            elements: elements.into_iter().map(|e| e.into()).collect(),
        }
    }

    /// 创建类型引用
    pub fn type_ref(
        name: &str,
        args: Vec<DependentType>,
    ) -> Self {
        DependentType::TypeRef {
            name: name.to_string(),
            args,
        }
    }

    /// 创建 Nil 变体
    pub fn nil() -> Self {
        DependentType::Variant(DependentVariant::Nil)
    }

    /// 创建 Cons 变体
    pub fn cons<H, T>(
        head: H,
        tail: T,
    ) -> Self
    where
        H: Into<DependentType>,
        T: Into<DependentType>,
    {
        DependentType::Variant(DependentVariant::Cons {
            head: Box::new(head.into()),
            tail: Box::new(tail.into()),
        })
    }

    /// 创建基本类型
    pub fn base(ty: MonoType) -> Self {
        DependentType::Base(ty)
    }

    /// 创建变量
    pub fn var(
        name: &str,
        index: usize,
    ) -> Self {
        DependentType::Var {
            name: name.to_string(),
            index,
        }
    }

    /// 获取元素的类型
    pub fn elem_type(&self) -> Option<&DependentType> {
        match self {
            DependentType::Vector { elem_type, .. } => Some(elem_type),
            DependentType::List { elem_type, .. } => Some(elem_type),
            DependentType::Option { elem_type, .. } => Some(elem_type),
            _ => None,
        }
    }

    /// 获取长度（如果适用）
    pub fn length(&self) -> Option<super::Nat> {
        match self {
            DependentType::Vector { length, .. } => Some(length.clone()),
            _ => None,
        }
    }

    /// 转换到具体类型
    pub fn to_mono_type(
        &self,
        computer: &mut TypeLevelComputer,
    ) -> Result<MonoType, TypeLevelError> {
        match self {
            DependentType::Base(ty) => Ok(ty.clone()),
            DependentType::Vector { elem_type, length } => {
                // Vector[T, N] 转换为一个类型引用
                Ok(MonoType::TypeRef(format!(
                    "Vector[{}, {}]",
                    elem_type, length
                )))
            }
            DependentType::List { elem_type } => {
                Ok(MonoType::TypeRef(format!("List[{}]", elem_type)))
            }
            DependentType::Option { elem_type } => {
                Ok(MonoType::TypeRef(format!("Option[{}]", elem_type)))
            }
            DependentType::Fn {
                params,
                return_type,
            } => {
                let param_types: Result<Vec<_>, _> =
                    params.iter().map(|p| p.to_mono_type(computer)).collect();
                let return_type = Box::new(return_type.to_mono_type(computer)?);
                Ok(MonoType::Fn {
                    params: param_types?,
                    return_type,
                    is_async: false,
                })
            }
            DependentType::Tuple { elements } => {
                let element_types: Result<Vec<_>, _> =
                    elements.iter().map(|e| e.to_mono_type(computer)).collect();
                Ok(MonoType::Tuple(element_types?))
            }
            DependentType::TypeRef { name, args: _ } => {
                // 类型族应用
                match name.as_str() {
                    "Vector" | "List" | "Option" => Ok(MonoType::TypeRef(name.clone())),
                    _ => Ok(MonoType::TypeRef(name.clone())),
                }
            }
            DependentType::Variant(variant) => match variant {
                DependentVariant::Nil => Ok(MonoType::TypeRef("Nil".to_string())),
                DependentVariant::Cons { .. } => Ok(MonoType::TypeRef("Cons".to_string())),
            },
            DependentType::Var { index, .. } => {
                if let Some(value) = computer.var_mapping.get(index) {
                    match value {
                        TypeLevelValue::Type(ty) => Ok(ty.clone()),
                        TypeLevelValue::Nat(_n) => Ok(MonoType::Int(64)), // 或者其他表示
                        _ => Err(TypeLevelError::DependentTypeError {
                            reason: format!("Cannot convert {:?} to MonoType", value),
                            span: crate::util::span::Span::default(),
                        }),
                    }
                } else {
                    Err(TypeLevelError::DependentTypeError {
                        reason: format!("Unbound dependent type variable {}", index),
                        span: crate::util::span::Span::default(),
                    })
                }
            }
        }
    }

    /// 获取类型族表示
    pub fn to_type_family(&self) -> Result<TypeFamily, TypeLevelError> {
        match self {
            DependentType::Base(ty) => Ok(TypeFamily::concrete(ty.clone())),
            DependentType::Vector {
                elem_type,
                length: _,
            } => {
                let elem_family = elem_type.to_type_family()?;
                Ok(TypeFamily::list(elem_family))
            }
            DependentType::List { elem_type } => {
                let elem_family = elem_type.to_type_family()?;
                Ok(TypeFamily::list(elem_family))
            }
            DependentType::Option { elem_type } => {
                let elem_family = elem_type.to_type_family()?;
                Ok(TypeFamily::option(elem_family))
            }
            DependentType::Fn {
                params,
                return_type,
            } => {
                let param_families: Result<Vec<_>, _> =
                    params.iter().map(|p| p.to_type_family()).collect();
                let return_family = return_type.to_type_family()?;
                Ok(TypeFamily::fn_type(param_families?, return_family))
            }
            DependentType::Tuple { elements } => {
                let element_families: Result<Vec<_>, _> =
                    elements.iter().map(|e| e.to_type_family()).collect();
                Ok(TypeFamily::tuple(element_families?))
            }
            DependentType::TypeRef { name, .. } => {
                Ok(TypeFamily::concrete(MonoType::TypeRef(name.clone())))
            }
            DependentType::Variant(_) => {
                Ok(TypeFamily::concrete(MonoType::TypeRef("List".to_string())))
            }
            DependentType::Var { name, index } => Ok(TypeFamily::Var {
                name: name.clone(),
                index: *index,
            }),
        }
    }

    /// 检查是否为空
    pub fn is_nil(&self) -> bool {
        matches!(self, DependentType::Variant(DependentVariant::Nil))
    }

    /// 检查是否为 Cons
    pub fn is_cons(&self) -> bool {
        matches!(self, DependentType::Variant(DependentVariant::Cons { .. }))
    }

    /// 获取 Cons 的头部
    pub fn head(&self) -> Option<&DependentType> {
        match self {
            DependentType::Variant(DependentVariant::Cons { head, .. }) => Some(head),
            _ => None,
        }
    }

    /// 获取 Cons 的尾部
    pub fn tail(&self) -> Option<&DependentType> {
        match self {
            DependentType::Variant(DependentVariant::Cons { tail, .. }) => Some(tail),
            _ => None,
        }
    }
}

impl From<MonoType> for DependentType {
    fn from(ty: MonoType) -> Self {
        DependentType::Base(ty)
    }
}

impl fmt::Display for DependentType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            DependentType::Vector { elem_type, length } => {
                write!(f, "Vector[{}, {}]", elem_type, length)
            }
            DependentType::List { elem_type } => {
                write!(f, "List[{}]", elem_type)
            }
            DependentType::Option { elem_type } => {
                write!(f, "Option[{}]", elem_type)
            }
            DependentType::Fn {
                params,
                return_type,
            } => {
                write!(
                    f,
                    "({}) -> {}",
                    params
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    return_type
                )
            }
            DependentType::Tuple { elements } => {
                write!(
                    f,
                    "({})",
                    elements
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            DependentType::TypeRef { name, args } => {
                if args.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(
                        f,
                        "{}[{}]",
                        name,
                        args.iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            DependentType::Variant(DependentVariant::Nil) => write!(f, "Nil"),
            DependentType::Variant(DependentVariant::Cons { head, tail }) => {
                write!(f, "Cons({}, {})", head, tail)
            }
            DependentType::Base(ty) => write!(f, "{}", ty),
            DependentType::Var { name, .. } => write!(f, "{}", name),
        }
    }
}

/// 依赖类型构造器
#[derive(Debug)]
pub struct DependentTypeBuilder {
    computer: TypeLevelComputer,
}

impl DependentTypeBuilder {
    /// 创建新的依赖类型构造器
    pub fn new() -> Self {
        DependentTypeBuilder {
            computer: TypeLevelComputer::new(),
        }
    }

    /// 获取计算器
    pub fn computer(&self) -> &TypeLevelComputer {
        &self.computer
    }

    /// 获取可变计算器
    pub fn computer_mut(&mut self) -> &mut TypeLevelComputer {
        &mut self.computer
    }

    /// 构建 Vector 类型
    pub fn build_vector<T, N>(
        &mut self,
        elem_type: T,
        length: N,
    ) -> Result<MonoType, TypeLevelError>
    where
        T: Into<DependentType>,
        N: Into<super::Nat>,
    {
        let dep_type = DependentType::vector(elem_type, length);
        dep_type.to_mono_type(&mut self.computer)
    }

    /// 构建 List 类型
    pub fn build_list<T>(
        &mut self,
        elem_type: T,
    ) -> Result<MonoType, TypeLevelError>
    where
        T: Into<DependentType>,
    {
        let dep_type = DependentType::list(elem_type);
        dep_type.to_mono_type(&mut self.computer)
    }

    /// 构建 Option 类型
    pub fn build_option<T>(
        &mut self,
        elem_type: T,
    ) -> Result<MonoType, TypeLevelError>
    where
        T: Into<DependentType>,
    {
        let dep_type = DependentType::option(elem_type);
        dep_type.to_mono_type(&mut self.computer)
    }

    /// 绑定变量
    pub fn bind(
        &mut self,
        index: usize,
        value: TypeLevelValue,
    ) {
        self.computer.bind_var(index, value);
    }

    /// 取消绑定变量
    pub fn unbind(
        &mut self,
        index: usize,
    ) {
        self.computer.unbind_var(index);
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.computer.clear_cache();
    }
}

impl Default for DependentTypeBuilder {
    fn default() -> Self {
        DependentTypeBuilder::new()
    }
}

/// 依赖类型检查器
#[derive(Debug)]
pub struct DependentTypeChecker {
    builder: DependentTypeBuilder,
    checker: ConditionalTypeChecker,
}

impl DependentTypeChecker {
    /// 创建新的依赖类型检查器
    pub fn new() -> Self {
        DependentTypeChecker {
            builder: DependentTypeBuilder::new(),
            checker: ConditionalTypeChecker::new(),
        }
    }

    /// 检查依赖类型
    pub fn check(
        &mut self,
        dep_type: &DependentType,
    ) -> Result<MonoType, TypeLevelError> {
        dep_type.to_mono_type(&mut self.builder.computer)
    }

    /// 检查条件依赖类型
    pub fn check_conditional(
        &mut self,
        cond_type: &ConditionalType,
    ) -> Result<MonoType, TypeLevelError> {
        self.checker.check(cond_type)
    }

    /// 绑定变量
    pub fn bind(
        &mut self,
        index: usize,
        value: TypeLevelValue,
    ) {
        self.builder.bind(index, value.clone());
        self.checker.bind(index, value);
    }

    /// 取消绑定变量
    pub fn unbind(
        &mut self,
        index: usize,
    ) {
        self.builder.unbind(index);
        self.checker.unbind(index);
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.builder.clear_cache();
        self.checker.clear_cache();
    }

    /// 获取类型级计算器
    pub fn computer(&self) -> &TypeLevelComputer {
        self.builder.computer()
    }
}

impl Default for DependentTypeChecker {
    fn default() -> Self {
        DependentTypeChecker::new()
    }
}

/// Vector 类型操作
#[derive(Debug)]
pub struct VectorOps;

impl VectorOps {
    /// 创建空 Vector
    pub fn empty<T>() -> DependentType
    where
        T: Into<DependentType>,
    {
        let void_type = DependentType::base(MonoType::Void);
        DependentType::vector(void_type, super::Nat::Zero)
    }

    /// 创建单元素 Vector
    pub fn singleton<T>(elem: T) -> DependentType
    where
        T: Into<DependentType>,
    {
        DependentType::vector(elem.into(), super::Nat::from_usize(1))
    }

    /// 创建长度为 N 的 Vector
    pub fn of_length<T, N>(
        elem: T,
        length: N,
    ) -> DependentType
    where
        T: Into<DependentType>,
        N: Into<super::Nat>,
    {
        DependentType::vector(elem.into(), length.into())
    }

    /// Vector 连接
    pub fn concat(
        v1: &DependentType,
        v2: &DependentType,
    ) -> Option<DependentType> {
        match (v1, v2) {
            (
                DependentType::Vector {
                    elem_type: e1,
                    length: l1,
                },
                DependentType::Vector {
                    elem_type: e2,
                    length: l2,
                },
            ) => {
                if **e1 == **e2 {
                    Some(DependentType::vector(e1.as_ref().clone(), l1.add(l2)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Vector 长度
    pub fn len(v: &DependentType) -> Option<super::Nat> {
        v.length()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::typecheck::types::MonoType;
    use crate::frontend::typecheck::type_level::Nat;

    #[test]
    fn test_vector_type() {
        let mut checker = DependentTypeChecker::new();
        let vector_type = DependentType::vector(MonoType::Int(64), Nat::from_usize(5));
        let mono_type = checker.check(&vector_type).unwrap();
        assert!(matches!(mono_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_list_type() {
        let mut checker = DependentTypeChecker::new();
        let list_type = DependentType::list(MonoType::String);
        let mono_type = checker.check(&list_type).unwrap();
        assert!(matches!(mono_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_option_type() {
        let mut checker = DependentTypeChecker::new();
        let option_type = DependentType::option(MonoType::Int(32));
        let mono_type = checker.check(&option_type).unwrap();
        assert!(matches!(mono_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_vector_nil() {
        let nil_vec = DependentType::nil();
        assert!(nil_vec.is_nil());
    }

    #[test]
    fn test_vector_cons() {
        let cons_vec =
            DependentType::cons(DependentType::base(MonoType::Int(64)), DependentType::nil());
        assert!(cons_vec.is_cons());
        assert!(cons_vec.head().is_some());
        assert!(cons_vec.tail().is_some());
    }

    #[test]
    fn test_vector_concat() {
        let v1 = DependentType::vector(MonoType::Int(64), Nat::from_usize(3));
        let v2 = DependentType::vector(MonoType::Int(64), Nat::from_usize(2));
        let v_concat = VectorOps::concat(&v1, &v2).unwrap();
        assert_eq!(v_concat.length(), Some(Nat::from_usize(5)));
    }

    #[test]
    fn test_vector_len() {
        let v = DependentType::vector(MonoType::String, Nat::from_usize(10));
        assert_eq!(VectorOps::len(&v), Some(Nat::from_usize(10)));
    }

    #[test]
    fn test_dependent_type_display() {
        let vec_type = DependentType::vector(MonoType::Int(64), Nat::from_usize(5));
        let s = vec_type.to_string();
        assert!(s.contains("Vector"));
        assert!(s.contains("5"));
    }
}
