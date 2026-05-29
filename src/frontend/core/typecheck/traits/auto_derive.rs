//! RFC-010/011 标准库 Derive 支持
//!
//! 提供编译器内置的标准库 trait 自动派生机制：
//! - Record 类型自动派生 Clone, Equal, Debug 等
//! - 字段全实现某 trait → Record 自动实现该 trait
//! - 显式定义可覆盖自动派生
//!
//! RFC-010 风格：
//! ```yaoxiang
//! # 标准库定义 Clone trait
//! type Clone = {
//!     clone: (self: Self) -> Self,
//! }
//!
//! # 用户显式定义（覆盖自动派生）
//! Point.clone: (self: Point) -> Point = {
//!     return Point(self.x, self.y)
//! }
//! ```

use std::collections::HashMap;
use crate::frontend::core::parser::ast::{Type, StructField};
use crate::frontend::core::types::base::MonoType;
use crate::frontend::core::types::base::{TraitTable, TraitImplementation};

/// RFC-011 定义的标准库 traits（接口类型）
pub const BUILTIN_DERIVES: &[&str] = &[
    "Clone", // 可克隆
    "Equal", // 可相等比较（合并了 PartialEq + Eq）
    "Debug", // 可调试打印
    "Send",  // 可发送（跨线程）
    "Sync",  // 可同步（跨线程共享）
];

/// 检查某 trait 是否为内置可派生
pub fn is_builtin_derive(trait_name: &str) -> bool {
    BUILTIN_DERIVES.contains(&trait_name)
}

/// 检查类型是否为 primitive 类型（不可再分）
pub fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "Int" | "Float" | "Bool" | "String" | "Void" | "Char"
    )
}

/// 检查 Record 是否可以自动派生某 trait
///
/// 规则：Record 的所有字段都实现了该 trait，则 Record 自动实现该 trait
/// 递归处理所有类型变体（Name、Generic、Tuple、Fn 等）
pub fn can_auto_derive(
    trait_table: &TraitTable,
    trait_name: &str,
    fields: &[StructField],
) -> bool {
    for field in fields {
        if !field_type_satisfies(trait_table, trait_name, &field.ty) {
            return false;
        }
    }
    true
}

/// 检查 MonoType 结构体是否可以自动派生某 trait
///
/// 与 `can_auto_derive` 类似，但直接操作 MonoType 字段（用于类型检查阶段）
pub fn can_auto_derive_for_monotype(
    trait_table: &TraitTable,
    trait_name: &str,
    struct_ty: &crate::frontend::core::types::base::mono::StructType,
) -> bool {
    for (_, field_ty) in &struct_ty.fields {
        if !mono_type_satisfies(trait_table, trait_name, field_ty) {
            return false;
        }
    }
    true
}

/// 递归检查 MonoType 是否满足某 trait
///
/// - 基本类型 → 直接查 trait_table
/// - Struct → 检查所有字段
/// - List/Set/Arc → 检查内部类型
/// - Dict → 检查 key 和 value 类型
/// - Tuple → 检查所有元素
/// - TypeRef → 查 trait_table
/// - Fn → 保守返回 false（函数类型一般不可派生）
/// - 其他 → 保守返回 false
fn mono_type_satisfies(
    trait_table: &TraitTable,
    trait_name: &str,
    ty: &MonoType,
) -> bool {
    match ty {
        // 基本类型
        MonoType::Int(_)
        | MonoType::Float(_)
        | MonoType::Bool
        | MonoType::Char
        | MonoType::String
        | MonoType::Bytes
        | MonoType::Void => trait_table.has_impl(trait_name, &ty.type_name()),

        // 结构体 → 递归检查所有字段
        MonoType::Struct(s) => s
            .fields
            .iter()
            .all(|(_, f)| mono_type_satisfies(trait_table, trait_name, f)),

        // 枚举 → 变体仅含名称（无关联数据），视为满足
        MonoType::Enum(_) => true,

        // 列表/集合 → 检查内部类型
        MonoType::List(inner) | MonoType::Set(inner) => {
            mono_type_satisfies(trait_table, trait_name, inner)
        }

        // 字典 → 检查 key 和 value
        MonoType::Dict(key, val) => {
            mono_type_satisfies(trait_table, trait_name, key)
                && mono_type_satisfies(trait_table, trait_name, val)
        }

        // 元组 → 检查所有元素
        MonoType::Tuple(elems) => elems
            .iter()
            .all(|e| mono_type_satisfies(trait_table, trait_name, e)),

        // Arc → 检查内部类型
        MonoType::Arc(inner) => mono_type_satisfies(trait_table, trait_name, inner),

        // Option → 检查内部类型
        MonoType::Option(inner) => mono_type_satisfies(trait_table, trait_name, inner),

        // Result → 检查两个内部类型
        MonoType::Result(ok, err) => {
            mono_type_satisfies(trait_table, trait_name, ok)
                && mono_type_satisfies(trait_table, trait_name, err)
        }

        // 类型引用 → 查 trait_table
        MonoType::TypeRef(name) => trait_table.has_impl(trait_name, name),

        // 函数类型 → 保守返回 false
        MonoType::Fn { .. } => false,

        // 其他类型 → 保守返回 false
        _ => false,
    }
}

/// 递归检查类型是否满足某 trait
///
/// - Name → 直接查 trait_table
/// - Generic → 容器和所有类型参数都必须满足
/// - Tuple → 所有元素必须满足
/// - Fn → 保守返回 false（函数类型一般不可派生）
/// - 其他 → 保守返回 false
pub(crate) fn field_type_satisfies(
    trait_table: &TraitTable,
    trait_name: &str,
    ty: &Type,
) -> bool {
    match ty {
        // 简单类型名 → 直接查 trait_table
        Type::Name { name, .. } => trait_table.has_impl(trait_name, name),

        // 原始类型 → 用类型名查 trait_table
        Type::Int(_) => trait_table.has_impl(trait_name, "Int"),
        Type::Float(_) => trait_table.has_impl(trait_name, "Float"),
        Type::Char => trait_table.has_impl(trait_name, "Char"),
        Type::String => trait_table.has_impl(trait_name, "String"),
        Type::Bytes => trait_table.has_impl(trait_name, "Bytes"),
        Type::Bool => trait_table.has_impl(trait_name, "Bool"),
        Type::Void => trait_table.has_impl(trait_name, "Void"),

        // 泛型类型如 List(Int), Option(Point) → 容器和所有类型参数都必须满足
        Type::Generic { name, args, .. } => {
            if !trait_table.has_impl(trait_name, name) {
                return false;
            }
            args.iter()
                .all(|arg| field_type_satisfies(trait_table, trait_name, arg))
        }

        // Option(T) → 检查内部类型
        Type::Option(inner) => field_type_satisfies(trait_table, trait_name, inner),

        // Result(T, E) → 检查两个内部类型
        Type::Result(ok_ty, err_ty) => {
            field_type_satisfies(trait_table, trait_name, ok_ty)
                && field_type_satisfies(trait_table, trait_name, err_ty)
        }

        // 元组 → 检查所有元素
        Type::Tuple(elems) => elems
            .iter()
            .all(|e| field_type_satisfies(trait_table, trait_name, e)),

        // 函数类型 → 保守返回 false（一般不可 Dup）
        Type::Fn { .. } => false,

        // 其他类型 → 保守返回 false
        _ => false,
    }
}

/// 为 Record 类型生成自动派生实现
pub fn generate_auto_derive(
    type_name: &str,
    trait_name: &str,
) -> Option<TraitImplementation> {
    let mut methods = HashMap::new();

    match trait_name {
        "Clone" => {
            // Clone 方法类型: (self: Self) -> Self
            let fn_type = MonoType::Fn {
                params: vec![MonoType::TypeRef("Self".to_string())],
                return_type: Box::new(MonoType::TypeRef(type_name.to_string())),
                is_async: false,
            };
            methods.insert("clone".to_string(), fn_type);
        }
        "Equal" => {
            // Equal 方法类型: (self: Self, other: Self) -> Bool
            let fn_type = MonoType::Fn {
                params: vec![
                    MonoType::TypeRef("Self".to_string()),
                    MonoType::TypeRef("Self".to_string()),
                ],
                return_type: Box::new(MonoType::TypeRef("Bool".to_string())),
                is_async: false,
            };
            methods.insert("equal".to_string(), fn_type);
        }
        "Debug" => {
            // Debug 方法类型: (self: Self, f: Formatter) -> Void
            let fn_type = MonoType::Fn {
                params: vec![
                    MonoType::TypeRef("Self".to_string()),
                    MonoType::TypeRef("Formatter".to_string()),
                ],
                return_type: Box::new(MonoType::TypeRef("Void".to_string())),
                is_async: false,
            };
            methods.insert("debug".to_string(), fn_type);
        }
        "Send" | "Sync" => {
            // Send/Sync 是标记 trait，不需要方法
        }
        _ => return None,
    }

    Some(TraitImplementation {
        trait_name: trait_name.to_string(),
        for_type_name: type_name.to_string(),
        methods,
    })
}
