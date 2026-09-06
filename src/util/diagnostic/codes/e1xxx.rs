//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E1XXX, {
    // E1001 未知变量
    ("E1001", TypeCheck, false, unknown_variable(name: &str) => .param("name", name)),
    // E1002 类型不匹配
    ("E1002", TypeCheck, false, type_mismatch(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    // E1003 未知类型
    ("E1003", TypeCheck, false, unknown_type(type_: &str) => .param("type", type_)),
    // E1010 参数数量不匹配
    ("E1010", TypeCheck, false, argument_count_mismatch(func: &str, expected: usize, found: usize) => .param("func", func) .param("expected", expected.to_string()) .param("found", found.to_string())),
    // E1011 参数类型不匹配
    ("E1011", TypeCheck, false, parameter_type_mismatch(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    // E1012 返回类型不匹配
    ("E1012", TypeCheck, false, return_type_mismatch(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    // E1013 函数未找到
    ("E1013", TypeCheck, false, function_not_found(func: &str) => .param("func", func)),
    // E1020 无法推断类型
    ("E1020", TypeCheck, false, cannot_infer_type(expr: &str) => .param("expr", expr)),
    // E1021 类型推断冲突
    ("E1021", TypeCheck, false, type_inference_conflict(reason: &str) => .param("reason", reason)),
    // E1030 模式穷举不足
    ("E1030", TypeCheck, false, pattern_non_exhaustive(patterns: &str) => .param("patterns", patterns)),
    // E1031 不可达模式
    ("E1031", TypeCheck, false, unreachable_pattern(pattern: &str) => .param("pattern", pattern)),
    // E1040 不支持的操作
    ("E1040", TypeCheck, false, unsupported_operation(op: &str, type_: &str) => .param("op", op).param("type", type_)),
    // E1041 数组越界
    ("E1041", TypeCheck, false, index_out_of_bounds(max: usize, index: i64) => .param("max", max.to_string()) .param("index", index.to_string())),
    // E1042 字段未找到
    ("E1042", TypeCheck, false, field_not_found(field: &str, struct_: &str) => .param("field", field).param("struct", struct_)),
    // E1050 逻辑运算需要布尔操作数
    ("E1050", TypeCheck, false, logical_operand_type_mismatch(left: &str, right: &str) => .param("left", left).param("right", right)),
    // E1051 逻辑 NOT 需要布尔操作数
    ("E1051", TypeCheck, false, logical_not_type_mismatch(type_: &str) => .param("type", type_)),
    // E1052 不能解引用非指针类型
    ("E1052", TypeCheck, false, invalid_deref(type_: &str) => .param("type", type_)),
    // E1053 不能在非结构体类型上访问字段
    ("E1053", TypeCheck, false, field_access_on_non_struct(type_: &str) => .param("type", type_)),
    // E1054 条件必须是布尔类型
    ("E1054", TypeCheck, false, condition_type_mismatch(type_: &str) => .param("type", type_)),
    // E1055 约束类型只能在泛型上下文中使用
    ("E1055", TypeCheck, false, constraint_not_in_generic(type_: &str) => .param("type", type_)),
    // E1060 类型参数数量不匹配
    ("E1060", TypeCheck, false, type_argument_count_mismatch(expected: usize, found: usize) => .param("expected", expected.to_string()) .param("found", found.to_string())),
    // E1061 无法实例化泛型类型
    ("E1061", TypeCheck, false, cannot_instantiate_generic() => ),
    // E1062 const 泛型约束失败
    ("E1062", TypeCheck, false, const_constraint_failed(constraint: &str) => .param("constraint", constraint)),
    // E1064 绑定位置索引无效（RFC-004）
    ("E1064", TypeCheck, false, invalid_binding_position(positions: &str, total: usize) => .param("positions", positions) .param("total", total.to_string())),
    // E1071 类型定义只能在模块级
    ("E1071", TypeCheck, false, type_def_only_at_module_level(name: &str) => .param("name", name)),
    // E1081 `?` 仅允许在返回 Result 的函数内使用
    ("E1081", TypeCheck, false, try_only_allowed_in_result() => ),
    // E1082 `?` 只能用于 Result 表达式
    ("E1082", TypeCheck, false, try_requires_result(type_: &str) => .param("type", type_)),
    // E1083 `?` 的错误类型不匹配
    ("E1083", TypeCheck, false, try_error_type_mismatch(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    // E1090 彩蛋（返回占位符，由 i18n 的 zen_message 提供实际消息）
    ("E1090", TypeCheck, false, type_self_reference_easter_egg() => ),
    // E1091 泛型元类型自指错误
    ("E1091", TypeCheck, false, invalid_generic_self_reference(decl: &str) => .param("decl", decl)),
    // E1092 精化类型实参形态非法（RFC-027，#263）
    ("E1092", TypeCheck, false, refined_arg_not_const(name: &str) => .param("name", name)),
    // E1093 精化类型实参个数不匹配（RFC-027，#263）
    // E1093 精化类型参数数量不匹配
    ("E1093", TypeCheck, false, refined_arity_mismatch(name: &str, expected: usize, found: usize) => .param("name", name) .param("expected", expected.to_string()) .param("found", found.to_string())),
    // E1094 编译期值参数未在类型体引用（#297/F）
    ("E1094", TypeCheck, false, unused_const_param(param: &str, type_: &str) => .param("param", param) .param("type", type_)),
    // E1095 未知接口（RFC-011a）
    ("E1095", TypeCheck, false, unknown_interface(name: &str) => .param("name", name)),
    // E1096 接口实例化类型实参个数不匹配（RFC-011a）
    ("E1096", TypeCheck, false, interface_arity_mismatch(name: &str, expected: usize, found: usize) => .param("name", name) .param("expected", expected.to_string()) .param("found", found.to_string())),
    // E1097 接口成员与类型已有字段共享命名空间（RFC-011a §1.2）
    ("E1097", TypeCheck, false, interface_member_conflict(type_: &str, member: &str) => .param("type", type_) .param("member", member)),
    // E1098 接口方法未实现（RFC-011a 完整性检查）
    ("E1098", TypeCheck, false, interface_method_missing(type_: &str, interface: &str, method: &str) => .param("type", type_) .param("interface", interface) .param("method", method)),
    // E1099 接口方法签名不匹配（RFC-011a）
    ("E1099", TypeCheck, false, interface_method_mismatch(type_: &str, method: &str, expected: &str, found: &str) => .param("type", type_) .param("method", method) .param("expected", expected) .param("found", found)),
    // E1100 同签名接口方法重复实现（RFC-011a §3 覆盖禁止）
    ("E1100", TypeCheck, false, interface_method_duplicate(type_: &str, method: &str) => .param("type", type_) .param("method", method)),
    // E1101 具体类型未实现目标接口（RFC-011a §6.3 存在类型成员检查）
    ("E1101", TypeCheck, false, type_does_not_implement_interface(type_: &str, interface: &str) => .param("type", type_) .param("interface", interface)),
    // E1102 break/continue 出现在循环外（#311：仅 while/for 体内允许循环控制流）
    ("E1102", TypeCheck, false, break_outside_loop(keyword: &str) => .param("keyword", keyword)),
});
