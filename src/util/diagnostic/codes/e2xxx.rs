//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E2XXX, {
    // E2001 变量不在作用域中
    ("E2001", Semantic, false, variable_not_in_scope(name: &str) => .param("name", name)),
    // E2002 重复定义
    ("E2002", Semantic, false, duplicate_definition(name: &str) => .param("name", name)),
    // E2003 所有权约束违反
    ("E2003", Semantic, false, ownership_violation(reason: &str) => .param("reason", reason)),
    // E2010 不可变赋值
    ("E2010", Semantic, false, immutable_assignment(name: &str) => .param("name", name)),
    // E2011 使用未初始化变量
    ("E2011", Semantic, false, uninitialized_variable(name: &str) => .param("name", name)),
    // E2012 可变性冲突
    ("E2012", Semantic, false, mutability_conflict() => ),
    // E2013 变量遮蔽
    ("E2013", Semantic, false, variable_shadowing(name: &str) => .param("name", name)),
    // E2014 使用已移动的变量
    ("E2014", Semantic, false, use_after_move(name: &str) => .param("name", name)),
    // E2016 不可变赋值（所有权检查器用）
    ("E2016", Semantic, false, immutable_assign(name: &str) => .param("name", name)),
    // E2018 可变/不可变借用冲突
    ("E2018", Semantic, false, mutable_immutable_borrow_conflict(name: &str) => .param("name", name)),
    // E2019 双重释放
    ("E2019", Semantic, false, double_drop(name: &str) => .param("name", name)),
    // E2020 释放后使用
    ("E2020", Semantic, false, use_after_drop(name: &str) => .param("name", name)),
    // E2027 unsafe 解引用
    ("E2027", Semantic, false, unsafe_deref() => ),
    // E2090 签名解析失败（通用）
    ("E2090", Semantic, false, invalid_signature(reason: &str) => .param("reason", reason)),
    // E2091 未知类型
    ("E2091", Semantic, false, invalid_signature_unknown_type(type_name: &str) => .param("type_name", type_name)),
    // E2092 缺少箭头
    ("E2092", Semantic, false, invalid_signature_missing_arrow() => ),
    // E2093 重复参数名
    ("E2093", Semantic, false, invalid_signature_duplicate_param(name: &str) => .param("name", name)),
    // E2094 泛型参数遮蔽
    ("E2094", Semantic, false, invalid_signature_generic_shadows(name: &str) => .param("name", name)),
    // E2095 参数名遮蔽泛型
    ("E2095", Semantic, false, invalid_signature_param_shadows_generic(name: &str) => .param("name", name)),
    // E2029 spawn 内 ref 循环
    ("E2029", Semantic, false, spawn_ref_cycle(cycle: &str) => .param("cycle", cycle)),
});
