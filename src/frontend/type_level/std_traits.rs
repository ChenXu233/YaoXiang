//! RFC-011 标准库 Traits 定义
//!
//! 定义 YaoXiang 语言的标准库 traits（接口类型）：
//! - Clone: 可克隆
//! - Copy: 可复制（标记 trait）
//! - Debug: 可调试打印
//! - PartialEq: 可相等比较
//! - Eq: 完全相等（标记 trait）
//!
//! RFC-010 风格示例：
//! ```yaoxiang
//! # 标准库定义 Clone trait
//! type Clone = {
//!     clone: (self: Self) -> Self,
//! }
//!
//! # 用户类型实现 Clone
//! type Point = { x: Float, y: Float }
//!
//! # 自动派生：字段全实现 Clone，则 Point 自动实现 Clone
//! # Point.clone: (self: Point) -> Point = { Point(self.x, self.y) }
//! ```

use std::collections::HashMap;
use crate::frontend::core::type_system::MonoType;
use super::trait_bounds::{TraitDefinition, TraitTable, TraitImplementation};

/// RFC-011 定义的标准库 trait 列表
pub const STD_TRAITS: &[&str] = &[
    "Clone",     // 可克隆
    "Copy",      // 可复制（标记 trait）
    "Debug",     // 可调试打印
    "PartialEq", // 可相等比较
    "Eq",        // 完全相等（标记 trait）
    "Iterable",  // 可迭代（用于 for 循环）
    "Iterator",  // 迭代器
];

/// 初始化标准库 traits 到 TraitTable
pub fn init_std_traits(trait_table: &mut TraitTable) {
    // 添加 Clone trait 定义
    add_clone_trait(trait_table);

    // 添加 Copy trait 定义（标记 trait）
    add_copy_trait(trait_table);

    // 添加 Debug trait 定义
    add_debug_trait(trait_table);

    // 添加 PartialEq trait 定义
    add_partial_eq_trait(trait_table);

    // 添加 Eq trait 定义（标记 trait）
    add_eq_trait(trait_table);

    // 添加 Iterable trait 定义（用于 for 循环）
    add_iterable_trait(trait_table);

    // 添加 Iterator trait 定义
    add_iterator_trait(trait_table);
}

/// 添加 Clone trait 定义
fn add_clone_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // Clone 方法签名: clone: (self: Self) -> Self
    let clone_sig = crate::frontend::type_level::trait_bounds::TraitMethodSignature {
        name: "clone".to_string(),
        params: vec![MonoType::TypeRef("Self".to_string())],
        return_type: MonoType::TypeRef("Self".to_string()),
        is_static: false,
    };
    methods.insert("clone".to_string(), clone_sig);

    let clone_def = TraitDefinition {
        name: "Clone".to_string(),
        methods,
        parent_traits: Vec::new(),
        generic_params: Vec::new(),
        span: None,
    };

    trait_table.add_trait(clone_def);
}

/// 添加 Copy trait 定义（标记 trait，通常不需要方法）
fn add_copy_trait(trait_table: &mut TraitTable) {
    // Copy 是标记 trait，语义由编译器处理
    // 不需要显式方法
    let copy_def = TraitDefinition {
        name: "Copy".to_string(),
        methods: HashMap::new(),
        parent_traits: Vec::new(),
        generic_params: Vec::new(),
        span: None,
    };

    trait_table.add_trait(copy_def);
}

/// 添加 Debug trait 定义
fn add_debug_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // Debug 方法签名: fmt: (self: Self, f: Formatter) -> Void
    let fmt_sig = crate::frontend::type_level::trait_bounds::TraitMethodSignature {
        name: "fmt".to_string(),
        params: vec![
            MonoType::TypeRef("Self".to_string()),
            MonoType::TypeRef("Formatter".to_string()),
        ],
        return_type: MonoType::TypeRef("Void".to_string()),
        is_static: false,
    };
    methods.insert("fmt".to_string(), fmt_sig);

    let debug_def = TraitDefinition {
        name: "Debug".to_string(),
        methods,
        parent_traits: Vec::new(),
        generic_params: Vec::new(),
        span: None,
    };

    trait_table.add_trait(debug_def);
}

/// 添加 PartialEq trait 定义
fn add_partial_eq_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // PartialEq 方法签名: eq: (self: Self, other: Self) -> Bool
    let eq_sig = crate::frontend::type_level::trait_bounds::TraitMethodSignature {
        name: "eq".to_string(),
        params: vec![
            MonoType::TypeRef("Self".to_string()),
            MonoType::TypeRef("Self".to_string()),
        ],
        return_type: MonoType::TypeRef("Bool".to_string()),
        is_static: false,
    };
    methods.insert("eq".to_string(), eq_sig);

    let partial_eq_def = TraitDefinition {
        name: "PartialEq".to_string(),
        methods,
        parent_traits: Vec::new(),
        generic_params: Vec::new(),
        span: None,
    };

    trait_table.add_trait(partial_eq_def);
}

/// 添加 Eq trait 定义（标记 trait）
fn add_eq_trait(trait_table: &mut TraitTable) {
    // Eq 是标记 trait，继承自 PartialEq
    // 语义上要求类型是等价关系（自反、传递、对称）
    let eq_def = TraitDefinition {
        name: "Eq".to_string(),
        methods: HashMap::new(),
        parent_traits: vec!["PartialEq".to_string()],
        generic_params: Vec::new(),
        span: None,
    };

    trait_table.add_trait(eq_def);
}

/// 为 primitive 类型添加标准库 trait 实现
pub fn init_primitive_impls(trait_table: &mut TraitTable) {
    // 为 Int 添加 Clone, Copy, PartialEq, Debug 实现
    add_primitive_impl(trait_table, "Int", "Clone");
    add_primitive_impl(trait_table, "Int", "Copy");
    add_primitive_impl(trait_table, "Int", "PartialEq");
    add_primitive_impl(trait_table, "Int", "Debug");

    // 为 Float 添加 Clone, Copy, PartialEq, Debug 实现
    add_primitive_impl(trait_table, "Float", "Clone");
    add_primitive_impl(trait_table, "Float", "Copy");
    add_primitive_impl(trait_table, "Float", "PartialEq");
    add_primitive_impl(trait_table, "Float", "Debug");

    // 为 Bool 添加 Clone, Copy, PartialEq, Debug 实现
    add_primitive_impl(trait_table, "Bool", "Clone");
    add_primitive_impl(trait_table, "Bool", "Copy");
    add_primitive_impl(trait_table, "Bool", "PartialEq");
    add_primitive_impl(trait_table, "Bool", "Debug");

    // 为 String 添加 Clone, PartialEq, Debug 实现
    add_primitive_impl(trait_table, "String", "Clone");
    add_primitive_impl(trait_table, "String", "PartialEq");
    add_primitive_impl(trait_table, "String", "Debug");
}

/// 为 primitive 类型添加 trait 实现
fn add_primitive_impl(
    trait_table: &mut TraitTable,
    type_name: &str,
    trait_name: &str,
) {
    let mut methods = HashMap::new();

    match trait_name {
        "Clone" => {
            // Clone 方法: 返回 self
            let fn_type = MonoType::Fn {
                params: vec![MonoType::TypeRef("Self".to_string())],
                return_type: Box::new(MonoType::TypeRef("Self".to_string())),
                is_async: false,
            };
            methods.insert("clone".to_string(), fn_type);
        }
        "Copy" => {
            // Copy 是标记 trait，不需要方法
        }
        "PartialEq" => {
            // eq 方法: 比较两个值
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
        "Debug" => {
            // fmt 方法: 格式化输出
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
        "Eq" => {
            // Eq 是标记 trait，不需要方法
        }
        _ => {}
    }

    let impl_ = TraitImplementation {
        trait_name: trait_name.to_string(),
        for_type_name: type_name.to_string(),
        methods,
    };

    trait_table.add_impl(impl_);
}

/// 检查是否为 primitive 类型
pub fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "Int" | "Float" | "Bool" | "String" | "Void" | "Char"
    )
}

/// 获取所有标准库 trait 名称
pub fn std_trait_names() -> &'static [&'static str] {
    STD_TRAITS
}

// ============================================================================
// 迭代器协议 Traits
// ============================================================================

/// 添加 Iterable trait 定义（用于 for 循环）
fn add_iterable_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // Iterable::iter 方法: iter: (self: &Self) -> Iterator<T>
    // 返回迭代器类型，这里使用 TypeRef 表示，由具体类型参数决定
    let iter_sig = crate::frontend::type_level::trait_bounds::TraitMethodSignature {
        name: "iter".to_string(),
        params: vec![MonoType::TypeRef("Self".to_string())],
        // 返回类型使用泛型占位符，在实现时具体化
        return_type: MonoType::TypeRef("Iterator".to_string()),
        is_static: false,
    };
    methods.insert("iter".to_string(), iter_sig);

    let iterable_def = TraitDefinition {
        name: "Iterable".to_string(),
        methods,
        parent_traits: Vec::new(),
        generic_params: vec!["T".to_string()], // 元素类型参数
        span: None,
    };

    trait_table.add_trait(iterable_def);
}

/// 添加 Iterator trait 定义
fn add_iterator_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // Iterator::next 方法: next: (&mut self) -> Option<T>
    let next_sig = crate::frontend::type_level::trait_bounds::TraitMethodSignature {
        name: "next".to_string(),
        params: vec![MonoType::TypeRef("Self".to_string())],
        // 返回 Option<T>，Option 是内置类型
        return_type: MonoType::TypeRef("Option".to_string()),
        is_static: false,
    };
    methods.insert("next".to_string(), next_sig);

    let iterator_def = TraitDefinition {
        name: "Iterator".to_string(),
        methods,
        parent_traits: Vec::new(),
        generic_params: vec!["T".to_string()], // 元素类型参数
        span: None,
    };

    trait_table.add_trait(iterator_def);
}
