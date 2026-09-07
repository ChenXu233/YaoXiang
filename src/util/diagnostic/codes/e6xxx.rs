//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

// E6xxx 族整体 span_exempt（第三段 true）：运行时诊断由 build_runtime_diagnostic
// 按栈帧 debug_map 解析位置，解析失败（如无 DebugSection 的 .42、栈溢出无帧）时
// 合法无位置——不能像编译期那样强制 .at（#324 裁决维持）。
// #327 起 debug_map 默认生成，正常运行时错误默认携带位置。
define_codes!(E6XXX, {
    // E6001 除零错误
    ("E6001", Runtime, true, division_by_zero(expr: &str) => .param("expr", expr)),
    // E6003 数组索引越界（运行时）
    ("E6003", Runtime, true, runtime_index_out_of_bounds(max: usize, index: i64) => .param("max", max.to_string()) .param("index", index.to_string())),
    // E6004 栈溢出
    ("E6004", Runtime, true, stack_overflow(limit: usize) => .param("limit", limit.to_string())),
    // E6005 断言失败
    ("E6005", Runtime, true, assertion_failed(condition: &str) => .param("condition", condition)),
    // E6006 函数未找到（运行时）
    ("E6006", Runtime, true, runtime_function_not_found(func: &str) => .param("func", func)),
    // E6007 运行时错误（通用）
    ("E6007", Runtime, true, runtime_error(message: &str) => .param("message", message)),
    // E6008 键缺失（#299 §4）
    ("E6008", Runtime, true, key_not_found(key: &str) => .param("key", key)),
});
