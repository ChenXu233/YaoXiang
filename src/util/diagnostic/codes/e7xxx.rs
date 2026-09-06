//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E7XXX, {
    // E7001 文件未找到
    ("E7001", Io, false, file_not_found(path: &str) => .param("path", path)),
    // E7002 权限被拒绝
    ("E7002", Io, false, permission_denied(path: &str) => .param("path", path)),
    // E7003 I/O 错误
    ("E7003", Io, false, io_error(reason: &str) => .param("reason", reason)),
    // E7004 网络错误
    ("E7004", Io, false, network_error(reason: &str) => .param("reason", reason)),
});
