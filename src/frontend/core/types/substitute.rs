//! 统一的类型替换模块
//!
//! 提供通用的类型替换算法，消除各模块间的重复代码。

use crate::frontend::core::types::identity::Nominal;
use crate::frontend::core::types::mono::StructFieldInfo;
use crate::frontend::core::types::{EnumType, MonoType, StructType, TypeVar};
use std::collections::HashMap;

/// 类型替换映射（使用类型变量索引）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Substitution {
    bindings: HashMap<usize, MonoType>,
}

impl Substitution {
    /// 创建新的替换
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// 添加绑定（使用 TypeVar）
    pub fn bind(
        &mut self,
        tv: TypeVar,
        ty: MonoType,
    ) {
        self.bindings.insert(tv.index(), ty);
    }

    /// 添加绑定（使用索引）
    pub fn insert(
        &mut self,
        index: usize,
        ty: MonoType,
    ) {
        self.bindings.insert(index, ty);
    }

    /// 获取绑定
    pub fn get(
        &self,
        index: &usize,
    ) -> Option<&MonoType> {
        self.bindings.get(index)
    }

    /// 检查是否包含变量
    pub fn contains_var(
        &self,
        index: &usize,
    ) -> bool {
        self.bindings.contains_key(index)
    }

    /// 合并替换
    pub fn merge(
        &self,
        other: &Substitution,
    ) -> Substitution {
        let mut result = self.clone();
        for (k, v) in &other.bindings {
            result.bindings.insert(*k, v.clone());
        }
        result
    }

    /// 获取所有绑定的变量索引
    pub fn bound_vars(&self) -> Vec<usize> {
        self.bindings.keys().cloned().collect()
    }

    /// 获取绑定数量
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

/// 应用头部的构造器名：`TypeRef("Vec")` / `Generic{"Vec", []}` 都表示构造器 `Vec`。
///
/// 用于「构造器变量只能出现在应用头部」这条规则的绑定与替换（RFC-010 元类型
/// 层级：Type1/Type2+ 的构造器在类型实参位以**名字**传递）。
pub fn ctor_head_name(ty: &MonoType) -> Option<String> {
    match ty {
        // 构造器变量的实参是**名字**（平路径）：带模块的 TypeRef 是具体名义
        // 类型（D64），不是构造器变量，参与「按名改头部」会把身份压成裸名。
        MonoType::TypeRef(name) if name.module().is_empty() => {
            Some(name.clone().bare().to_string())
        }
        MonoType::Generic { name, args } if args.is_empty() && name.module().is_empty() => {
            Some(name.clone().bare().to_string())
        }
        _ => None,
    }
}

/// 构造器变量在**应用头部**的替换：把 `F(A)` 的头部名按 `head_map`
///（构造器参数名 → 实参构造器名）改写为真实构造器。
///
/// 共享实现在此（复审轮 1 F2 校正：此前 `sema/environment.rs` 与
/// `sema/inference/expressions/mod.rs` 的 `substitute_type_params_with_vars` 各有一份内联改名，与当时「只有一处
/// 实现」的表述不符；历史提交信息不改写，此处按事实更正）。两处调用点现均为
/// 「先做各自的 TypeRef/变量遍历，再调用本函数」。
///
/// 普通替换（按 TypeVar 或 TypeRef）覆盖不到 `MonoType::Generic` 的头部字符串，
/// 而 `F(A)` 的 F 正是声明里绑定的构造器变量——只按名在头部替换，不触碰实参位。
pub fn substitute_ctor_heads(
    ty: &MonoType,
    head_map: &HashMap<String, String>,
) -> MonoType {
    if head_map.is_empty() {
        return ty.clone();
    }
    match ty {
        MonoType::Generic { name, args } => MonoType::Generic {
            // 只改写**命中**的头部（构造器变量按名绑定到实参构造器）；未命中的
            // 头部原样保留——把未命中的身份头也重建成平路径会丢模块（D64），
            // 同一次替换里另一位的平路径构造器足以让所有头被压平。
            name: match head_map.get(name.bare()) {
                Some(replacement) => Nominal::flat(AsRef::<str>::as_ref(&(replacement.clone())))
                    .expect("type name must be bare and non-empty"),
                None => name.clone(),
            },
            args: args
                .iter()
                .map(|a| substitute_ctor_heads(a, head_map))
                .collect(),
        },
        MonoType::Fn {
            params,
            return_type,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|p| substitute_ctor_heads(p, head_map))
                .collect(),
            return_type: Box::new(substitute_ctor_heads(return_type, head_map)),
        },
        MonoType::Struct(s) => MonoType::Struct(StructType {
            fields: s
                .fields
                .iter()
                .map(|field_info| StructFieldInfo {
                    ty: substitute_ctor_heads(&field_info.ty, head_map),
                    ..field_info.clone()
                })
                .collect(),
            ..s.clone()
        }),
        MonoType::Ref { mutable, inner } => MonoType::Ref {
            mutable: *mutable,
            inner: Box::new(substitute_ctor_heads(inner, head_map)),
        },
        MonoType::Union(ts) => MonoType::Union(
            ts.iter()
                .map(|t| substitute_ctor_heads(t, head_map))
                .collect(),
        ),
        MonoType::Intersection(ts) => MonoType::Intersection(
            ts.iter()
                .map(|t| substitute_ctor_heads(t, head_map))
                .collect(),
        ),
        MonoType::MetaType {
            universe_level,
            type_params,
        } => MonoType::MetaType {
            universe_level: universe_level.clone(),
            type_params: type_params
                .iter()
                .map(|t| substitute_ctor_heads(t, head_map))
                .collect(),
        },
        MonoType::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => MonoType::AssocType {
            host_type: Box::new(substitute_ctor_heads(host_type, head_map)),
            assoc_name: assoc_name.clone(),
            assoc_args: assoc_args
                .iter()
                .map(|t| substitute_ctor_heads(t, head_map))
                .collect(),
        },
        MonoType::Literal {
            name,
            base_type,
            value,
        } => MonoType::Literal {
            name: name.clone(),
            base_type: Box::new(substitute_ctor_heads(base_type, head_map)),
            value: value.clone(),
        },
        _ => ty.clone(),
    }
}

/// 类型表示层身份解析：把**平路径**名义名按导入面解析成 (module, 定义名) 身份。
///
/// 用途（RFC-039 D63/D64）：类型构造点从 AST 写出名得到的是平路径
/// （`Nominal::flat`）；身份的唯一解析实现是导入面
/// [`crate::frontend::core::sema::import_face::ImportFace::type_identity`]，
/// 它知道本单元声明 / 花括号导入 / std 已声明类型，也只把**已声明类型**
/// 解析成身份（幻名、类型族、内建名、类型参数保持平路径）。
///
/// 幂等：已经带模块的身份原样保留——否则把 a.Result 传进一个只看得见
/// std.result 的模块就会被改写成 std.result.Result（身份被猜测覆盖）。
/// 平路径才查表；查不到保持平路径（D64：module="" 即内建/参数/合成名）。
///
/// 与 [`substitute_ctor_heads`] 的分工：那是泛型应用头部的**改名**；本函数
/// 不改名，只把平路径升格为身份（同名不同模块因此可分）。与
/// [`MonoType::substitute`] 的分工：那是类型**变量**替换；本函数按名义
/// 引用语义结构递归全部变体。
pub fn resolve_nominal_identities(
    ty: &MonoType,
    resolve: &impl Fn(&str) -> Option<Nominal>,
) -> MonoType {
    let resolve_face = |name: &Nominal| -> Nominal {
        if !name.module().is_empty() || name.bare().is_empty() {
            return name.clone();
        }
        resolve(name.bare()).unwrap_or_else(|| name.clone())
    };
    match ty {
        MonoType::TypeRef(name) => MonoType::TypeRef(resolve_face(name)),
        MonoType::Fn {
            params,
            return_type,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|p| resolve_nominal_identities(p, resolve))
                .collect(),
            return_type: Box::new(resolve_nominal_identities(return_type, resolve)),
        },
        MonoType::Generic { name, args } => MonoType::Generic {
            name: resolve_face(name),
            args: args
                .iter()
                .map(|a| resolve_nominal_identities(a, resolve))
                .collect(),
        },
        MonoType::Struct(s) => MonoType::Struct(StructType {
            name: resolve_face(&s.name),
            fields: s
                .fields
                .iter()
                .map(|field_info| StructFieldInfo {
                    ty: resolve_nominal_identities(&field_info.ty, resolve),
                    ..field_info.clone()
                })
                .collect(),
            // 接口实现名是名义名位（语义与 TypeRef 引用体同）。
            interfaces: s.interfaces.iter().map(resolve_face).collect(),
            ..s.clone()
        }),
        MonoType::Enum(e) => MonoType::Enum(EnumType {
            name: resolve_face(&e.name),
            ..e.clone()
        }),
        MonoType::Ref { mutable, inner } => MonoType::Ref {
            mutable: *mutable,
            inner: Box::new(resolve_nominal_identities(inner, resolve)),
        },
        MonoType::Union(types) => MonoType::Union(
            types
                .iter()
                .map(|t| resolve_nominal_identities(t, resolve))
                .collect(),
        ),
        MonoType::Intersection(types) => MonoType::Intersection(
            types
                .iter()
                .map(|t| resolve_nominal_identities(t, resolve))
                .collect(),
        ),
        MonoType::MetaType {
            universe_level,
            type_params,
        } => MonoType::MetaType {
            universe_level: universe_level.clone(),
            type_params: type_params
                .iter()
                .map(|t| resolve_nominal_identities(t, resolve))
                .collect(),
        },
        MonoType::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => MonoType::AssocType {
            host_type: Box::new(resolve_nominal_identities(host_type, resolve)),
            assoc_name: assoc_name.clone(),
            assoc_args: assoc_args
                .iter()
                .map(|t| resolve_nominal_identities(t, resolve))
                .collect(),
        },
        MonoType::Literal {
            name,
            base_type,
            value,
        } => MonoType::Literal {
            name: name.clone(),
            base_type: Box::new(resolve_nominal_identities(base_type, resolve)),
            value: value.clone(),
        },
        MonoType::Refined { base, constraint } => MonoType::Refined {
            base: Box::new(resolve_nominal_identities(base, resolve)),
            constraint: constraint.clone(),
        },
        MonoType::DepFn {
            params,
            return_type,
        } => MonoType::DepFn {
            params: params
                .iter()
                .map(|dep_param| crate::frontend::core::types::mono::DepParam {
                    name: dep_param.name.clone(),
                    ty: resolve_nominal_identities(&dep_param.ty, resolve),
                })
                .collect(),
            return_type: Box::new(resolve_nominal_identities(return_type, resolve)),
        },
        // 无名义引用位的变体——逐名显式穷尽、无兜底臂（3.24.3 t3 F1）：
        // 新增变体时编译器在此指路，不再静默吞掉。TypeVar 由求解器管理，
        // 原语（Never/Void/Bool/Int/Float/Char）与 FFI 形态无名义引用位。
        MonoType::Never | MonoType::Void | MonoType::Bool | MonoType::Char => ty.clone(),
        MonoType::Int(width) => MonoType::Int(*width),
        MonoType::Float(width) => MonoType::Float(*width),
        MonoType::TypeVar(tv) => MonoType::TypeVar(*tv),
        MonoType::LibraryRef { mechanism, lib } => MonoType::LibraryRef {
            mechanism: mechanism.clone(),
            lib: lib.clone(),
        },
        MonoType::ExternRef {
            mechanism,
            lib,
            symbol,
        } => MonoType::ExternRef {
            mechanism: mechanism.clone(),
            lib: lib.clone(),
            symbol: symbol.clone(),
        },
    }
}

/// 类型替换器
#[derive(Debug, Clone, Default)]
pub struct Substituter;

impl Substituter {
    /// 创建新的替换器
    pub fn new() -> Self {
        Self
    }

    /// 替换单个类型变量
    pub fn substitute_var(
        &self,
        ty: &MonoType,
        var: &TypeVar,
        replacement: &MonoType,
    ) -> MonoType {
        let mut lookup = |tv: &TypeVar| {
            if tv == var {
                Some(replacement.clone())
            } else {
                None
            }
        };
        self.substitute_internal(ty, &mut lookup)
    }

    /// 批量替换（使用 Substitution）
    pub fn substitute(
        &self,
        ty: &MonoType,
        sub: &Substitution,
    ) -> MonoType {
        let mut lookup = |tv: &TypeVar| sub.get(&tv.index()).cloned();
        self.substitute_internal(ty, &mut lookup)
    }

    /// 批量替换（使用 HashMap<usize, MonoType>）
    pub fn substitute_with_map(
        &self,
        ty: &MonoType,
        substitutions: &HashMap<usize, MonoType>,
    ) -> MonoType {
        let mut lookup = |tv: &TypeVar| substitutions.get(&tv.index()).cloned();
        self.substitute_internal(ty, &mut lookup)
    }

    /// 泛型参数替换（按索引顺序）
    pub fn substitute_generic_params(
        &self,
        ty: &MonoType,
        args: &[MonoType],
    ) -> MonoType {
        let mut lookup = |tv: &TypeVar| {
            let index = tv.index();
            if index < args.len() {
                Some(args[index].clone())
            } else {
                None
            }
        };
        self.substitute_internal(ty, &mut lookup)
    }

    /// 内部实现：使用闭包进行替换
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_internal<F>(
        &self,
        ty: &MonoType,
        lookup: &mut F,
    ) -> MonoType
    where
        F: FnMut(&TypeVar) -> Option<MonoType>,
    {
        match ty {
            MonoType::TypeVar(tv) => {
                if let Some(replacement) = lookup(tv) {
                    replacement
                } else {
                    ty.clone()
                }
            }
            MonoType::Generic { name, args } => MonoType::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            },
            MonoType::Fn {
                params,
                return_type,
            } => {
                let new_params = params
                    .iter()
                    .map(|p| self.substitute_internal(p, lookup))
                    .collect();
                let new_return_type = Box::new(self.substitute_internal(return_type, lookup));
                MonoType::Fn {
                    params: new_params,
                    return_type: new_return_type,
                }
            }
            MonoType::Struct(struct_type) => {
                let new_fields = struct_type
                    .fields
                    .iter()
                    .map(|field_info| {
                        let field_ty = self.substitute_internal(&field_info.ty, lookup);
                        StructFieldInfo {
                            ty: field_ty,
                            ..field_info.clone()
                        }
                    })
                    .collect();
                MonoType::Struct(StructType {
                    name: struct_type.name.clone(),
                    fields: new_fields,
                    methods: struct_type.methods.clone(),
                    interfaces: struct_type.interfaces.clone(),
                })
            }
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Union(types) => MonoType::Union(
                types
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            ),
            MonoType::Intersection(types) => MonoType::Intersection(
                types
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            ),
            MonoType::Ref { mutable, inner } => MonoType::Ref {
                mutable: *mutable,
                inner: Box::new(self.substitute_internal(inner, lookup)),
            },
            MonoType::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => MonoType::AssocType {
                host_type: Box::new(self.substitute_internal(host_type, lookup)),
                assoc_name: assoc_name.clone(),
                assoc_args: assoc_args
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            },
            _ => ty.clone(),
        }
    }
}

/// 检查类型是否包含类型变量
#[allow(clippy::only_used_in_recursion)]
pub fn contains_type_vars(ty: &MonoType) -> bool {
    match ty {
        MonoType::TypeVar(_) => true,
        MonoType::Generic { name: _, args } => args.iter().any(contains_type_vars),
        MonoType::Fn {
            params,
            return_type,
            ..
        } => params.iter().any(contains_type_vars) || contains_type_vars(return_type),
        MonoType::Struct(struct_type) => struct_type
            .fields
            .iter()
            .any(|field| contains_type_vars(&field.ty)),
        MonoType::Union(types) | MonoType::Intersection(types) => {
            types.iter().any(contains_type_vars)
        }
        MonoType::AssocType {
            host_type,
            assoc_args,
            ..
        } => contains_type_vars(host_type) || assoc_args.iter().any(contains_type_vars),
        // 借用类型 `&T` / `&mut T`：与 `TypeConstraintSolver::contains_var` 同族，
        // 内层必须穿透——缺本臂时「含类型变量」对借用位恒为假，occurs check
        // 一类的判据在借用位整段失灵（同一缺陷的两份实现，见提交信息）。
        MonoType::Ref { inner, .. } => contains_type_vars(inner),
        _ => false,
    }
}

// ========== RFC-010: Universe Level Tests ==========
