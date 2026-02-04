//! RFC-010/011 标准库 Derive 支持
//!
//! 提供编译器内置的标准库 trait 自动派生机制：
//! - Record 类型自动派生 Clone, Copy, Debug 等
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
use crate::frontend::core::parser::ast::Type;
use crate::frontend::core::type_system::MonoType;
use super::trait_bounds::{TraitTable, TraitImplementation};

/// RFC-011 定义的标准库 traits（接口类型）
pub const BUILTIN_DERIVES: &[&str] = &[
    "Clone",     // 可克隆
    "Copy",      // 可复制
    "Debug",     // 可调试打印
    "PartialEq", // 可相等比较
    "Eq",        // 完全相等
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
pub fn can_auto_derive(
    trait_table: &TraitTable,
    trait_name: &str,
    fields: &[(String, Type)],
) -> bool {
    // 检查该 trait 是否为内置可派生 trait
    if !is_builtin_derive(trait_name) {
        return false;
    }

    // 检查所有字段是否都实现了该 trait
    for (_field_name, field_type) in fields {
        let field_type_name = match field_type {
            Type::Name(name) => name.clone(),
            _ => return false, // 复杂类型暂不支持自动派生
        };

        // 字段类型必须实现该 trait
        if !trait_table.has_impl(trait_name, &field_type_name) {
            return false;
        }
    }

    true
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
        "Copy" => {
            // Copy 是标记 trait，不需要方法
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
            methods.insert("fmt".to_string(), fn_type);
        }
        "PartialEq" => {
            // PartialEq 方法类型: (self: Self, other: Self) -> Bool
            let fn_type = MonoType::Fn {
                params: vec![
                    MonoType::TypeRef("Self".to_string()),
                    MonoType::TypeRef("Self".to_string()),
                ],
                return_type: Box::new(MonoType::TypeRef("Bool".to_string())),
                is_async: false,
            };
            methods.insert("eq".to_string(), fn_type);
        }
        "Eq" => {
            // Eq 是标记 trait，不需要方法
        }
        _ => return None,
    }

    Some(TraitImplementation {
        trait_name: trait_name.to_string(),
        for_type_name: type_name.to_string(),
        methods,
    })
}
