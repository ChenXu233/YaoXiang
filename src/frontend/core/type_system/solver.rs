//! 类型约束求解器
//!
//! 实现类型系统的核心求解算法：
//! - TypeConstraintSolver: 类型约束求解器（union-find 实现）

use super::mono::{TypeBinding, MonoType, StructType, EnumType, PolyType};
use super::constraint::TypeConstraint;
use super::error::{TypeMismatch, TypeConstraintError};
use crate::util::span::Span;
use std::collections::{HashMap, HashSet};

/// 类型约束求解器（union-find 实现）
///
/// 负责管理类型变量的绑定和约束求解
#[derive(Debug, Clone, Default)]
pub struct TypeConstraintSolver {
    /// 类型变量的绑定状态
    bindings: Vec<TypeBinding>,
    /// 收集的约束
    constraints: Vec<TypeConstraint>,
    /// 下一个类型变量的索引
    next_var: usize,
    /// 泛型变量集合（不应被实例化）
    generic_vars: HashMap<usize, usize>,
}

impl TypeConstraintSolver {
    /// 创建新的求解器
    pub fn new() -> Self {
        TypeConstraintSolver {
            bindings: Vec::new(),
            constraints: Vec::new(),
            next_var: 0,
            generic_vars: HashMap::new(),
        }
    }

    /// 重置求解器
    pub fn reset(&mut self) {
        self.bindings.clear();
        self.constraints.clear();
        self.next_var = 0;
        self.generic_vars.clear();
    }

    /// 创建新的类型变量
    pub fn new_var(&mut self) -> MonoType {
        let var = super::var::TypeVar::new(self.next_var);
        self.next_var += 1;
        self.bindings.push(TypeBinding::Unbound);
        MonoType::TypeVar(var)
    }

    /// 创建新的泛型变量
    pub fn new_generic_var(&mut self) -> super::var::TypeVar {
        let var = super::var::TypeVar::new(self.next_var);
        self.next_var += 1;
        self.bindings.push(TypeBinding::Unbound);
        self.generic_vars
            .insert(var.index(), self.generic_vars.len());
        var
    }

    /// 查找类型变量的最终绑定（路径压缩）
    pub fn find(
        &mut self,
        var: super::var::TypeVar,
    ) -> super::var::TypeVar {
        match self.bindings.get(var.index()) {
            Some(TypeBinding::Link(next)) => {
                let root = self.find(*next);
                // 路径压缩
                if let Some(binding) = self.bindings.get_mut(var.index()) {
                    *binding = TypeBinding::Link(root);
                }
                root
            }
            Some(TypeBinding::Bound(_)) => var,
            Some(TypeBinding::Unbound) => var,
            None => var,
        }
    }

    /// 获取类型变量的当前绑定
    pub fn get_binding(
        &self,
        var: super::var::TypeVar,
    ) -> Option<&TypeBinding> {
        self.bindings.get(var.index())
    }

    /// 获取类型变量的当前绑定（可变）
    pub fn get_binding_mut(
        &mut self,
        var: super::var::TypeVar,
    ) -> Option<&mut TypeBinding> {
        self.bindings.get_mut(var.index())
    }

    /// 绑定类型变量到类型
    ///
    /// 如果类型变量已经绑定，会尝试合并
    #[allow(clippy::result_large_err)]
    pub fn bind(
        &mut self,
        var: super::var::TypeVar,
        ty: &MonoType,
    ) -> Result<(), TypeMismatch> {
        let resolved_var = self.find(var);

        // 展开类型变量链
        let ty = self.expand_type(ty);

        // 完整 occurs check：禁止无限类型（包括嵌套出现）
        if self.contains_var(&ty, resolved_var) {
            return Err(TypeMismatch {
                left: MonoType::TypeVar(resolved_var),
                right: ty,
                span: Span::default(),
            });
        }

        // 绑定
        if let Some(binding) = self.bindings.get_mut(resolved_var.index()) {
            match binding {
                TypeBinding::Unbound => {
                    *binding = TypeBinding::Bound(ty);
                    Ok(())
                }
                TypeBinding::Bound(existing) => {
                    // 已绑定，检查是否一致
                    if existing == &ty {
                        Ok(())
                    } else {
                        Err(TypeMismatch {
                            left: (*existing).clone(),
                            right: ty,
                            span: Span::default(),
                        })
                    }
                }
                TypeBinding::Link(_) => {
                    // 不应该到这里
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// 展开类型变量，获取具体类型
    fn expand_type(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(v) => {
                let root = self.find_readonly(*v);
                if let Some(TypeBinding::Bound(bound_ty)) = self.bindings.get(root.index()) {
                    self.expand_type(bound_ty)
                } else {
                    MonoType::TypeVar(root)
                }
            }
            MonoType::TypeRef(name) => self
                .resolve_builtin_type_ref(name)
                .unwrap_or_else(|| ty.clone()),
            MonoType::Struct(s) => MonoType::Struct(StructType {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.expand_type(t)))
                    .collect(),
                methods: s.methods.clone(),
                field_mutability: s.field_mutability.clone(),
            }),
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Tuple(ts) => {
                MonoType::Tuple(ts.iter().map(|t| self.expand_type(t)).collect())
            }
            MonoType::List(t) => MonoType::List(Box::new(self.expand_type(t))),
            MonoType::Dict(k, v) => {
                MonoType::Dict(Box::new(self.expand_type(k)), Box::new(self.expand_type(v)))
            }
            MonoType::Set(t) => MonoType::Set(Box::new(self.expand_type(t))),
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.expand_type(elem_type)),
            },
            MonoType::Arc(inner) => MonoType::Arc(Box::new(self.expand_type(inner))),
            MonoType::Weak(inner) => MonoType::Weak(Box::new(self.expand_type(inner))),
            MonoType::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => MonoType::AssocType {
                host_type: Box::new(self.expand_type(host_type)),
                assoc_name: assoc_name.clone(),
                assoc_args: assoc_args.iter().map(|t| self.expand_type(t)).collect(),
            },
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params.iter().map(|t| self.expand_type(t)).collect(),
                return_type: Box::new(self.expand_type(return_type)),
                is_async: *is_async,
            },
            // 联合类型展开
            MonoType::Union(types) => {
                MonoType::Union(types.iter().map(|t| self.expand_type(t)).collect())
            }
            // 交集类型展开
            MonoType::Intersection(types) => {
                MonoType::Intersection(types.iter().map(|t| self.expand_type(t)).collect())
            }
            MonoType::MetaType {
                universe_level,
                type_params,
            } => MonoType::MetaType {
                universe_level: universe_level.clone(),
                type_params: type_params.iter().map(|t| self.expand_type(t)).collect(),
            },
            MonoType::Literal {
                name,
                base_type,
                value,
            } => MonoType::Literal {
                name: name.clone(),
                base_type: Box::new(self.expand_type(base_type)),
                value: value.clone(),
            },
            _ => ty.clone(),
        }
    }

    /// 添加类型约束
    pub fn add_constraint(
        &mut self,
        left: MonoType,
        right: MonoType,
        span: Span,
    ) {
        self.constraints
            .push(TypeConstraint::new(left, right, span));
    }

    /// 求解所有约束
    ///
    /// 返回求解后的类型环境状态
    pub fn solve(&mut self) -> Result<(), Vec<TypeConstraintError>> {
        let mut errors = Vec::new();

        // 逐一求解约束
        for constraint in std::mem::take(&mut self.constraints) {
            if let Err(e) = self.unify(&constraint.left, &constraint.right) {
                errors.push(TypeConstraintError {
                    error: e,
                    span: constraint.span,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 解析类型，展开所有类型变量（需要 &mut self 以调用 find）
    pub fn resolve(
        &mut self,
        ty: &MonoType,
    ) -> MonoType {
        self.expand_type_mut(ty)
    }

    /// 解析类型，展开所有类型变量
    pub fn resolve_type(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        self.expand_type(ty)
    }

    /// 展开类型变量（可变版本，能跟随链接）
    fn expand_type_mut(
        &mut self,
        ty: &MonoType,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(v) => {
                // 使用 find 找到 root var
                let root = self.find(*v);
                // 先克隆 bound_ty 以避免借用问题
                let bound_ty = self.bindings.get(root.index()).and_then(|b| match b {
                    TypeBinding::Bound(ty) => Some(ty.clone()),
                    _ => None,
                });
                if let Some(bound_ty) = bound_ty {
                    self.expand_type_mut(&bound_ty)
                } else {
                    MonoType::TypeVar(root)
                }
            }
            MonoType::TypeRef(name) => self
                .resolve_builtin_type_ref(name)
                .unwrap_or_else(|| ty.clone()),
            MonoType::Struct(s) => MonoType::Struct(StructType {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.expand_type_mut(t)))
                    .collect(),
                methods: s.methods.clone(),
                field_mutability: s.field_mutability.clone(),
            }),
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Tuple(ts) => {
                MonoType::Tuple(ts.iter().map(|t| self.expand_type_mut(t)).collect())
            }
            MonoType::List(t) => MonoType::List(Box::new(self.expand_type_mut(t))),
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(self.expand_type_mut(k)),
                Box::new(self.expand_type_mut(v)),
            ),
            MonoType::Set(t) => MonoType::Set(Box::new(self.expand_type_mut(t))),
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.expand_type_mut(elem_type)),
            },
            MonoType::Arc(inner) => MonoType::Arc(Box::new(self.expand_type_mut(inner))),
            MonoType::Weak(inner) => MonoType::Weak(Box::new(self.expand_type_mut(inner))),
            MonoType::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => MonoType::AssocType {
                host_type: Box::new(self.expand_type_mut(host_type)),
                assoc_name: assoc_name.clone(),
                assoc_args: assoc_args.iter().map(|t| self.expand_type_mut(t)).collect(),
            },
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params.iter().map(|t| self.expand_type_mut(t)).collect(),
                return_type: Box::new(self.expand_type_mut(return_type)),
                is_async: *is_async,
            },
            MonoType::Union(types) => {
                MonoType::Union(types.iter().map(|t| self.expand_type_mut(t)).collect())
            }
            MonoType::Intersection(types) => {
                MonoType::Intersection(types.iter().map(|t| self.expand_type_mut(t)).collect())
            }
            MonoType::MetaType {
                universe_level,
                type_params,
            } => MonoType::MetaType {
                universe_level: universe_level.clone(),
                type_params: type_params
                    .iter()
                    .map(|t| self.expand_type_mut(t))
                    .collect(),
            },
            MonoType::Literal {
                name,
                base_type,
                value,
            } => MonoType::Literal {
                name: name.clone(),
                base_type: Box::new(self.expand_type_mut(base_type)),
                value: value.clone(),
            },
            _ => ty.clone(),
        }
    }

    /// Unify 两个类型
    ///
    /// 尝试将两个类型统一，返回约束或错误
    #[allow(clippy::result_large_err)]
    pub fn unify(
        &mut self,
        t1: &MonoType,
        t2: &MonoType,
    ) -> Result<(), TypeMismatch> {
        // eprintln!("DEBUG unify: t1={:?}, t2={:?}", t1, t2);
        let t1 = self.expand_type(t1);
        let t2 = self.expand_type(t2);
        // eprintln!("DEBUG unify: after expand, t1={:?}, t2={:?}", t1, t2);

        match (&t1, &t2) {
            // 类型变量 unify
            (MonoType::TypeVar(v1), MonoType::TypeVar(v2)) => {
                let v1 = self.find(*v1);
                let v2 = self.find(*v2);
                if v1 == v2 {
                    Ok(())
                } else {
                    // 建立链接
                    self.bind(v1, &MonoType::TypeVar(v2))
                }
            }
            (MonoType::TypeVar(v), _) => self.bind(*v, &t2),
            (_, MonoType::TypeVar(v)) => self.bind(*v, &t1),

            // 具体类型 unify
            (MonoType::Void, MonoType::Void) => Ok(()),
            (MonoType::Bool, MonoType::Bool) => Ok(()),
            (MonoType::Int(n1), MonoType::Int(n2)) if n1 == n2 => Ok(()),
            (MonoType::Float(n1), MonoType::Float(n2)) if n1 == n2 => Ok(()),
            (MonoType::Char, MonoType::Char) => Ok(()),
            (MonoType::String, MonoType::String) => Ok(()),
            (MonoType::Bytes, MonoType::Bytes) => Ok(()),

            // 函数类型 unify
            (
                MonoType::Fn {
                    params: p1,
                    return_type: r1,
                    is_async: a1,
                },
                MonoType::Fn {
                    params: p2,
                    return_type: r2,
                    is_async: a2,
                },
            ) => {
                if p1.len() != p2.len() || a1 != a2 {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                // unify 参数和返回类型
                for (p1, p2) in p1.iter().zip(p2.iter()) {
                    self.unify(p1, p2)?;
                }
                self.unify(r1, r2)?;
                Ok(())
            }

            // 结构体类型 unify
            (MonoType::Struct(s1), MonoType::Struct(s2)) => {
                if s1.name != s2.name
                    || s1.fields.len() != s2.fields.len()
                    || s1.field_mutability != s2.field_mutability
                {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                for ((n1, f1), (n2, f2)) in s1.fields.iter().zip(s2.fields.iter()) {
                    if n1 != n2 {
                        return Err(TypeMismatch {
                            left: t1,
                            right: t2,
                            span: Span::default(),
                        });
                    }
                    self.unify(f1, f2)?;
                }
                Ok(())
            }

            // 枚举类型 unify
            (MonoType::Enum(e1), MonoType::Enum(e2)) => {
                if e1.name != e2.name || e1.variants != e2.variants {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                Ok(())
            }

            // 元组类型 unify
            (MonoType::Tuple(ts1), MonoType::Tuple(ts2)) => {
                if ts1.len() != ts2.len() {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                for (t1, t2) in ts1.iter().zip(ts2.iter()) {
                    self.unify(t1, t2)?;
                }
                Ok(())
            }

            // 列表类型 unify
            (MonoType::List(t1), MonoType::List(t2)) => self.unify(t1, t2),

            // 字典类型 unify
            (MonoType::Dict(k1, v1), MonoType::Dict(k2, v2)) => {
                self.unify(k1, k2)?;
                self.unify(v1, v2)?;
                Ok(())
            }

            // 集合类型 unify
            (MonoType::Set(t1), MonoType::Set(t2)) => self.unify(t1, t2),

            // 类型引用 unify（仅比较名称）
            (MonoType::TypeRef(n1), MonoType::TypeRef(n2)) if n1 == n2 => Ok(()),

            // 联合类型 unify：T1 | T2 == T3 分解为 (T1 == T3) | (T2 == T3)
            // 即：检查 T3 是否是联合类型的超类型，或者 T3 是否兼容联合的每个成员
            (MonoType::Union(types1), MonoType::Union(types2)) => {
                if !self.unify_unordered(types1, types2) {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                Ok(())
            }
            (MonoType::Union(types), other) | (other, MonoType::Union(types)) => {
                // 联合类型 == 具体类型：尝试与任一成员统一
                let snapshot = self.bindings.clone();
                for member in types {
                    if self.unify(member, other).is_ok() {
                        return Ok(());
                    }
                    self.bindings = snapshot.clone();
                }
                Err(TypeMismatch {
                    left: t1,
                    right: t2,
                    span: Span::default(),
                })
            }

            // 交集类型 unify：T1 & T2 == T3 分解为 (T1 == T3) & (T2 == T3)
            // 即：检查 T3 是否同时满足 T1 和 T2 的约束
            (MonoType::Intersection(types1), MonoType::Intersection(types2)) => {
                if !self.unify_unordered(types1, types2) {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                Ok(())
            }
            (MonoType::Intersection(types), other) | (other, MonoType::Intersection(types)) => {
                // 交集类型 == 具体类型：检查具体类型是否与所有成员兼容
                for member in types {
                    self.unify(member, other)?;
                }
                Ok(())
            }

            // 不兼容类型
            _ => Err(TypeMismatch {
                left: t1,
                right: t2,
                span: Span::default(),
            }),
        }
    }

    /// 实例化多态类型
    ///
    /// 将多态类型中的泛型变量替换为新类型变量
    pub fn instantiate(
        &mut self,
        poly: &PolyType,
    ) -> MonoType {
        let substitution: HashMap<_, _> = poly
            .type_binders
            .iter()
            .map(|var| (*var, self.new_var()))
            .collect();

        self.substitute_type(&poly.body, &substitution)
    }

    /// 替换类型中的变量
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_type(
        &self,
        ty: &MonoType,
        substitution: &HashMap<super::var::TypeVar, MonoType>,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(v) => {
                if let Some(new_ty) = substitution.get(v) {
                    new_ty.clone()
                } else {
                    ty.clone()
                }
            }
            MonoType::Struct(s) => MonoType::Struct(StructType {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.substitute_type(t, substitution)))
                    .collect(),
                methods: s.methods.clone(),
                field_mutability: s.field_mutability.clone(),
            }),
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Tuple(ts) => MonoType::Tuple(
                ts.iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
            ),
            MonoType::List(t) => MonoType::List(Box::new(self.substitute_type(t, substitution))),
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(self.substitute_type(k, substitution)),
                Box::new(self.substitute_type(v, substitution)),
            ),
            MonoType::Set(t) => MonoType::Set(Box::new(self.substitute_type(t, substitution))),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
                return_type: Box::new(self.substitute_type(return_type, substitution)),
                is_async: *is_async,
            },
            // 范围类型替换
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.substitute_type(elem_type, substitution)),
            },
            // Arc 类型替换
            MonoType::Arc(inner) => {
                MonoType::Arc(Box::new(self.substitute_type(inner, substitution)))
            }
            // 关联类型替换
            MonoType::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => MonoType::AssocType {
                host_type: Box::new(self.substitute_type(host_type, substitution)),
                assoc_name: assoc_name.clone(),
                assoc_args: assoc_args
                    .iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
            },
            // 联合类型替换
            MonoType::Union(types) => MonoType::Union(
                types
                    .iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
            ),
            // 交集类型替换
            MonoType::Intersection(types) => MonoType::Intersection(
                types
                    .iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
            ),
            // 基本类型不需要替换
            _ => ty.clone(),
        }
    }

    /// 生成新类型变量的替换映射（用于多态函数调用）
    ///
    /// 将泛型参数替换为新的类型变量
    pub fn fresh_substitution(
        &mut self,
        type_binders: &[super::var::TypeVar],
    ) -> HashMap<super::var::TypeVar, MonoType> {
        type_binders
            .iter()
            .map(|var| (*var, self.new_var()))
            .collect()
    }

    /// 检查类型变量是否在作用域内
    #[allow(clippy::only_used_in_recursion)]
    pub fn contains_var(
        &self,
        ty: &MonoType,
        var: super::var::TypeVar,
    ) -> bool {
        match ty {
            MonoType::TypeVar(v) => *v == var,
            MonoType::Struct(s) => s.fields.iter().any(|(_, t)| self.contains_var(t, var)),
            MonoType::Enum(_) => false, // 枚举变体名不包含类型变量
            MonoType::Tuple(types) => types.iter().any(|t| self.contains_var(t, var)),
            MonoType::List(t) => self.contains_var(t, var),
            MonoType::Dict(k, v) => self.contains_var(k, var) || self.contains_var(v, var),
            MonoType::Set(t) => self.contains_var(t, var),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                params.iter().any(|t| self.contains_var(t, var))
                    || self.contains_var(return_type, var)
            }
            MonoType::Range { elem_type } => self.contains_var(elem_type, var),
            MonoType::Arc(inner) => self.contains_var(inner, var),
            MonoType::AssocType {
                host_type,
                assoc_args,
                ..
            } => {
                self.contains_var(host_type, var)
                    || assoc_args.iter().any(|t| self.contains_var(t, var))
            }
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types.iter().any(|t| self.contains_var(t, var))
            }
            // 基本类型和类型引用不包含类型变量
            _ => false,
        }
    }

    /// 泛化类型
    pub fn generalize(
        &mut self,
        mono_type: &MonoType,
    ) -> PolyType {
        let resolved = self.resolve(mono_type);
        let mut seen = HashSet::new();
        let mut free_vars = Vec::new();
        self.collect_generalizable_vars(&resolved, &mut seen, &mut free_vars);
        PolyType::new(free_vars, resolved)
    }

    /// 检查类型变量是否未受约束
    pub fn is_unconstrained(
        &self,
        var: super::var::TypeVar,
    ) -> bool {
        if let Some(binding) = self.bindings.get(var.index()) {
            matches!(binding, TypeBinding::Unbound)
        } else {
            true
        }
    }

    fn resolve_builtin_type_ref(
        &self,
        name: &str,
    ) -> Option<MonoType> {
        match name {
            "Int" | "int" | "int64" | "i64" => Some(MonoType::Int(64)),
            "Int32" | "int32" | "i32" => Some(MonoType::Int(32)),
            "Int16" | "int16" | "i16" => Some(MonoType::Int(16)),
            "Int8" | "int8" | "i8" => Some(MonoType::Int(8)),
            "Float" | "float" | "float64" | "f64" => Some(MonoType::Float(64)),
            "Float32" | "float32" | "f32" => Some(MonoType::Float(32)),
            "Bool" | "bool" => Some(MonoType::Bool),
            "Char" | "char" => Some(MonoType::Char),
            "String" | "string" | "str" => Some(MonoType::String),
            "Bytes" | "bytes" => Some(MonoType::Bytes),
            "Void" | "void" | "()" => Some(MonoType::Void),
            _ => None,
        }
    }

    fn find_readonly(
        &self,
        mut var: super::var::TypeVar,
    ) -> super::var::TypeVar {
        let mut steps = 0;
        while steps <= self.bindings.len() {
            match self.bindings.get(var.index()) {
                Some(TypeBinding::Link(next)) => {
                    var = *next;
                    steps += 1;
                }
                _ => return var,
            }
        }
        var
    }

    fn unify_unordered(
        &mut self,
        left: &[MonoType],
        right: &[MonoType],
    ) -> bool {
        if left.len() != right.len() {
            return false;
        }
        let mut used = vec![false; right.len()];
        self.unify_unordered_recursive(left, right, 0, &mut used)
    }

    fn unify_unordered_recursive(
        &mut self,
        left: &[MonoType],
        right: &[MonoType],
        index: usize,
        used: &mut [bool],
    ) -> bool {
        if index == left.len() {
            return true;
        }

        for right_index in 0..right.len() {
            if used[right_index] {
                continue;
            }

            let snapshot = self.bindings.clone();
            if self.unify(&left[index], &right[right_index]).is_ok() {
                used[right_index] = true;
                if self.unify_unordered_recursive(left, right, index + 1, used) {
                    return true;
                }
                used[right_index] = false;
            }
            self.bindings = snapshot;
        }

        false
    }

    fn collect_generalizable_vars(
        &self,
        ty: &MonoType,
        seen: &mut HashSet<usize>,
        out: &mut Vec<super::var::TypeVar>,
    ) {
        match ty {
            MonoType::TypeVar(v) => {
                let root = self.find_readonly(*v);
                match self.bindings.get(root.index()) {
                    Some(TypeBinding::Bound(bound)) => {
                        self.collect_generalizable_vars(bound, seen, out);
                    }
                    _ => {
                        if !self.generic_vars.contains_key(&root.index())
                            && seen.insert(root.index())
                        {
                            out.push(root);
                        }
                    }
                }
            }
            MonoType::Struct(s) => {
                for (_, field_ty) in &s.fields {
                    self.collect_generalizable_vars(field_ty, seen, out);
                }
            }
            MonoType::Tuple(types) | MonoType::Union(types) | MonoType::Intersection(types) => {
                for t in types {
                    self.collect_generalizable_vars(t, seen, out);
                }
            }
            MonoType::List(t) | MonoType::Set(t) | MonoType::Arc(t) | MonoType::Weak(t) => {
                self.collect_generalizable_vars(t, seen, out)
            }
            MonoType::Range { elem_type } => {
                self.collect_generalizable_vars(elem_type, seen, out);
            }
            MonoType::Dict(k, v) => {
                self.collect_generalizable_vars(k, seen, out);
                self.collect_generalizable_vars(v, seen, out);
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                for p in params {
                    self.collect_generalizable_vars(p, seen, out);
                }
                self.collect_generalizable_vars(return_type, seen, out);
            }
            MonoType::AssocType {
                host_type,
                assoc_args,
                ..
            } => {
                self.collect_generalizable_vars(host_type, seen, out);
                for arg in assoc_args {
                    self.collect_generalizable_vars(arg, seen, out);
                }
            }
            MonoType::MetaType { type_params, .. } => {
                for p in type_params {
                    self.collect_generalizable_vars(p, seen, out);
                }
            }
            MonoType::Literal { base_type, .. } => {
                self.collect_generalizable_vars(base_type, seen, out);
            }
            MonoType::Enum(_)
            | MonoType::TypeRef(_)
            | MonoType::Void
            | MonoType::Bool
            | MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => {}
        }
    }
}
