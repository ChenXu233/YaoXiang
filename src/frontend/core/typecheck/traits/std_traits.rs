//! RFC-011 标准库 Traits 定义
//!
//! 定义 YaoXiang 语言的标准库 traits（接口类型）：
//! - Clone: 可克隆
//! - Dup: 可复制（标记 trait，隐含 Clone）
//! - Equal: 可相等比较（合并了 PartialEq + Eq）
//! - Debug: 可调试打印
//! - Iterator: 迭代器
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
use crate::frontend::core::types::base::MonoType;
use crate::frontend::core::types::base::{TraitDefinition, TraitTable, TraitImplementation};

/// RFC-011 定义的标准库 trait 列表
pub const STD_TRAITS: &[&str] = &[
    "Clone",    // 可克隆
    "Dup",      // 可复制（标记 trait，隐含 Clone）
    "Equal",    // 可相等比较（合并了 PartialEq + Eq）
    "Debug",    // 可调试打印
    "Iterator", // 迭代器
];

/// 初始化标准库 traits 到 TraitTable
pub fn init_std_traits(trait_table: &mut TraitTable) {
    // 添加 Clone trait 定义
    add_clone_trait(trait_table);

    // 添加 Dup trait 定义（标记 trait，隐含 Clone）
    add_dup_trait(trait_table);

    // 添加 Equal trait 定义（合并了 PartialEq + Eq）
    add_equal_trait(trait_table);

    // 添加 Debug trait 定义
    add_debug_trait(trait_table);

    // 添加 Iterator trait 定义
    add_iterator_trait(trait_table);
}

/// 添加 Clone trait 定义
fn add_clone_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // Clone 方法签名: clone: (self: Self) -> Self
    let clone_sig = crate::frontend::core::types::base::TraitMethodSignature {
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
        is_marker: false,
    };

    trait_table.add_trait(clone_def);
}

/// 添加 Equal trait 定义（合并了 PartialEq + Eq）
fn add_equal_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // Equal 方法签名: equal: (self: Self, other: Self) -> Bool
    let equal_sig = crate::frontend::core::types::base::TraitMethodSignature {
        name: "equal".to_string(),
        params: vec![
            MonoType::TypeRef("Self".to_string()),
            MonoType::TypeRef("Self".to_string()),
        ],
        return_type: MonoType::TypeRef("Bool".to_string()),
        is_static: false,
    };
    methods.insert("equal".to_string(), equal_sig);

    let equal_def = TraitDefinition {
        name: "Equal".to_string(),
        methods,
        parent_traits: Vec::new(),
        generic_params: Vec::new(),
        span: None,
        is_marker: false,
    };

    trait_table.add_trait(equal_def);
}

/// 添加 Dup trait 定义（标记 trait，隐含 Clone）
fn add_dup_trait(trait_table: &mut TraitTable) {
    // Dup 是标记 trait，表示类型可以被隐式复制
    // 语义：隐含 Clone 约束，由编译器自动处理
    trait_table.add_trait(TraitDefinition {
        name: "Dup".to_string(),
        methods: HashMap::new(),
        parent_traits: vec!["Clone".to_string()],
        generic_params: vec![],
        span: None,
        is_marker: true,
    });
}

/// 添加 Debug trait 定义
fn add_debug_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // Debug 方法签名: debug: (self: Self, f: Formatter) -> Void
    let debug_sig = crate::frontend::core::types::base::TraitMethodSignature {
        name: "debug".to_string(),
        params: vec![
            MonoType::TypeRef("Self".to_string()),
            MonoType::TypeRef("Formatter".to_string()),
        ],
        return_type: MonoType::TypeRef("Void".to_string()),
        is_static: false,
    };
    methods.insert("debug".to_string(), debug_sig);

    let debug_def = TraitDefinition {
        name: "Debug".to_string(),
        methods,
        parent_traits: Vec::new(),
        generic_params: Vec::new(),
        span: None,
        is_marker: false,
    };

    trait_table.add_trait(debug_def);
}

/// 为 primitive 类型添加标准库 trait 实现
pub fn init_primitive_impls(trait_table: &mut TraitTable) {
    // 为 Int 添加 Clone, Dup, Equal, Debug 实现
    add_primitive_impl(trait_table, "Int", "Clone");
    add_primitive_impl(trait_table, "Int", "Dup");
    add_primitive_impl(trait_table, "Int", "Equal");
    add_primitive_impl(trait_table, "Int", "Debug");

    // 为 Float 添加 Clone, Dup, Equal, Debug 实现
    add_primitive_impl(trait_table, "Float", "Clone");
    add_primitive_impl(trait_table, "Float", "Dup");
    add_primitive_impl(trait_table, "Float", "Equal");
    add_primitive_impl(trait_table, "Float", "Debug");

    // 为 Bool 添加 Clone, Dup, Equal, Debug 实现
    add_primitive_impl(trait_table, "Bool", "Clone");
    add_primitive_impl(trait_table, "Bool", "Dup");
    add_primitive_impl(trait_table, "Bool", "Equal");
    add_primitive_impl(trait_table, "Bool", "Debug");

    // 为 Char 添加 Clone, Dup, Equal, Debug 实现
    add_primitive_impl(trait_table, "Char", "Clone");
    add_primitive_impl(trait_table, "Char", "Dup");
    add_primitive_impl(trait_table, "Char", "Equal");
    add_primitive_impl(trait_table, "Char", "Debug");

    // 为 String 添加 Clone, Dup, Equal, Debug 实现
    add_primitive_impl(trait_table, "String", "Clone");
    add_primitive_impl(trait_table, "String", "Dup");
    add_primitive_impl(trait_table, "String", "Equal");
    add_primitive_impl(trait_table, "String", "Debug");

    // 为 Bytes 添加 Clone, Dup, Debug 实现（不实现 Equal）
    add_primitive_impl(trait_table, "Bytes", "Clone");
    add_primitive_impl(trait_table, "Bytes", "Dup");
    add_primitive_impl(trait_table, "Bytes", "Debug");

    // 为 Void 添加 Equal, Debug 实现（不实现 Clone, Dup）
    add_primitive_impl(trait_table, "Void", "Equal");
    add_primitive_impl(trait_table, "Void", "Debug");
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
        "Dup" => {
            // Dup 是标记 trait，不需要方法（隐含 Clone）
        }
        "Equal" => {
            // equal 方法: 比较两个值
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
            // debug 方法: 格式化输出
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

/// 添加 Iterator trait 定义
fn add_iterator_trait(trait_table: &mut TraitTable) {
    let mut methods = HashMap::new();

    // Iterator::next 方法: next: (&mut self) -> Option<T>
    let next_sig = crate::frontend::core::types::base::TraitMethodSignature {
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
        is_marker: false,
    };

    trait_table.add_trait(iterator_def);
}
