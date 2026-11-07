//! 项目级配置加载（`load_project_config_from`）单测 — #520 甲案（RFC-015
//! §向后兼容性「配置解析失败时显示友好错误，提示具体行号和错误原因」）。
//!
//! 判据：文件不存在 = 合法默认（单文件模式语义不动）；存在但读不了/解析
//! 不了 = 显式 Err（toml 错误携带行号列号），不再静默回退默认。

use crate::util::config::{load_project_config_from, ConfigError};

/// 守卫（未修分支）：文件不存在 = 合法默认，不得报错。
#[test]
fn test_load_project_config_missing_file_returns_default() {
    // Arrange：不存在的路径
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("yaoxiang.toml");

    // Act
    let result = load_project_config_from(&path);

    // Assert：Ok 且为默认（runtime.mode = embedded）
    let config = result.expect("无配置文件是合法默认，不得报错");
    assert_eq!(
        config.runtime.mode, "embedded",
        "无配置文件必须取内建默认 runtime.mode"
    );
}

/// 存在但非法 TOML = 显式 Err，错误消息携带 toml 行号（RFC-015 判据）。
#[test]
fn test_load_project_config_broken_toml_is_error_with_line_info() {
    // Arrange：非法 TOML（表头未闭合）
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("yaoxiang.toml");
    std::fs::write(&path, "[mono\nmax_depth = 150\n").expect("write config");

    // Act
    let result = load_project_config_from(&path);

    // Assert：ParseError 且 Display 带行号信息
    let err = result.expect_err("存在但非法 TOML 必须报错，不得静默回退默认");
    assert!(
        matches!(err, ConfigError::ParseError(_)),
        "非法 TOML 必须归 ParseError: {err:?}"
    );
    let display = err.to_string();
    assert!(
        display.contains("TOML parse error") && display.contains("line"),
        "错误消息必须携带 toml 行号信息（RFC-015:282）: {display}"
    );
}
