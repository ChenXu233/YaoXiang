//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E8XXX, {
    // E8001 内部编译器错误
    ("E8001", Internal, true, internal_error(message: &str) => .param("message", message)),
    // E8002 意外 panic
    ("E8002", Internal, false, unexpected_panic(reason: &str) => .param("reason", reason)),
    // E8003 编译器阶段错误
    ("E8003", Internal, false, compiler_phase_error(phase: &str, message: &str) => .param("phase", phase) .param("message", message)),
});
