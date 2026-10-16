//! URI 规范化键单元测试（WBS 4.6.1）：`path_to_uri` / `db_key` 的幂等性——
//! 同一文件的客户端 URI 等价形式必须归一到同一个键，跨文件 `resolves_to`
//! 的 DefId 才稳定命中。

use crate::lsp::uri::{db_key, path_to_uri};

// 断言的是 Windows 盘符 URI 的换算（函数内 cfg!(windows) 分支），
// Linux runner 上按构造不成立——平台门控
#[cfg(windows)]
#[test]
fn test_path_to_uri_windows_drive_letter() {
    // Arrange + Act
    let uri = path_to_uri(std::path::Path::new("E:/a/b.yx"));

    // Assert
    assert_eq!(uri, "file:///E:/a/b.yx");
}

#[cfg(windows)]
#[test]
fn test_path_to_uri_percent_encodes_space() {
    // Arrange + Act - 空格必须编码（产出可直接回传客户端）
    let uri = path_to_uri(std::path::Path::new("E:/my dir/b.yx"));

    // Assert
    assert_eq!(uri, "file:///E:/my%20dir/b.yx");
}

#[cfg(windows)]
#[test]
fn test_db_key_is_idempotent_on_encoded_form() {
    // Arrange + Act - 已编码的客户端 URI 归一后必须等于自身（幂等性）
    let key = db_key("file:///E:/my%20dir/b.yx");

    // Assert
    assert_eq!(key, "file:///E:/my%20dir/b.yx");
    // 同一字符的编码形式归一到规范形（%2D 是 '-' 的编码——解码后按
    // 非保留字原样输出；等价编码形式因此归一到同一键）
    let hyphen_encoded = db_key("file:///E:/my%2Ddir/b.yx");
    assert_eq!(hyphen_encoded, "file:///E:/my-dir/b.yx");
    // 归一结果再归一仍是自身（幂等闭包）
    assert_eq!(db_key(&hyphen_encoded), hyphen_encoded);
}

// POSIX 绝对路径换算（Linux/macOS runner 的真实面）
#[cfg(not(windows))]
#[test]
fn test_path_to_uri_posix_absolute_path() {
    // Arrange + Act
    let uri = path_to_uri(std::path::Path::new("/tmp/a/b.yx"));

    // Assert
    assert_eq!(uri, "file:///tmp/a/b.yx");
    // 归一结果再归一仍是自身（幂等闭包）
    assert_eq!(db_key(&uri), uri);
}

#[test]
fn test_db_key_non_file_scheme_passthrough() {
    // Arrange + Act - std:// 等虚拟键原样保留
    let key = db_key("std://std.io");

    // Assert
    assert_eq!(key, "std://std.io");
}
