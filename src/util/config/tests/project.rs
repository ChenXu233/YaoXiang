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

/// 合法配置逐节解析成功（fmt / runtime / tool.test / mono 四节齐读）。
#[test]
fn test_load_project_config_valid_file_parses_all_sections() {
    // Arrange：合法 yaoxiang.toml（四节各写一个字段）
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("yaoxiang.toml");
    std::fs::write(
        &path,
        concat!(
            "[fmt]\nline_width = 40\n\n",
            "[runtime]\nmode = \"standard\"\nworkers = 2\n\n",
            "[tool.test]\npatterns = [\"custom_tests/**/*.yx\"]\n\n",
            "[mono]\nmax_depth = 150\n"
        ),
    )
    .expect("write config");

    // Act
    let config = load_project_config_from(&path).expect("合法配置必须解析成功");

    // Assert：四节字段各归位
    assert_eq!(config.fmt.line_width, Some(40), "[fmt] line_width 必须解析");
    assert_eq!(config.runtime.mode, "standard", "[runtime] mode 必须解析");
    assert_eq!(
        config.tool.test.patterns,
        vec!["custom_tests/**/*.yx".to_string()],
        "[tool.test].patterns 必须解析"
    );
    assert_eq!(config.mono.max_depth, 150, "[mono] max_depth 必须解析");
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

/// 字段类型错（[tool.test].patterns 写成字符串）= 显式 Err——落点1 机制的
/// 单元级判据：该错误穿透 PackageManifest（serde 忽略未知字段），只有
/// ProjectConfig 投影会失败。
#[test]
fn test_load_project_config_tool_test_type_error_is_error() {
    // Arrange：patterns 是字符串而非列表
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("yaoxiang.toml");
    std::fs::write(&path, "[tool.test]\npatterns = \"tests\"\n").expect("write config");

    // Act
    let result = load_project_config_from(&path);

    // Assert
    let err = result.expect_err("patterns 类型错必须报错，不得静默回退内建约定");
    assert!(
        err.to_string().contains("patterns"),
        "错误消息必须携带出错的字段名: {}",
        err
    );
}

/// 守卫（未修分支）：未知 section 与未知字段仍容忍（serde ignore 语义不动）。
#[test]
fn test_load_project_config_unknown_fields_tolerated() {
    // Arrange：未知节 + 未知字段 + 合法节混排
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("yaoxiang.toml");
    std::fs::write(
        &path,
        "[unknown_section]\nfoo = 1\n\n[runtime]\nmode = \"embedded\"\nunknown_field = \"x\"\n",
    )
    .expect("write config");

    // Act
    let config = load_project_config_from(&path).expect("未知字段必须容忍");

    // Assert：合法节照常解析
    assert_eq!(
        config.runtime.mode, "embedded",
        "未知字段旁的合法字段必须照常解析"
    );
}
