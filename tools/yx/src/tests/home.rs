//! 安装根解析测试（RFC-037 §安装器支持：YAOXIANG_HOME 覆盖约定）

use crate::error::Error;
use crate::home::yaoxiang_home_from;

#[test]
fn test_env_home_overrides_user_home() {
    // Act
    let home = yaoxiang_home_from(Some("/opt/yx-home"), Some("/Users/dev")).unwrap();

    // Assert: YAOXIANG_HOME 优先（服务 CI 与容器场景）
    assert_eq!(
        home,
        std::path::Path::new("/opt/yx-home"),
        "YAOXIANG_HOME must take precedence over user home"
    );
}

#[test]
fn test_empty_env_home_falls_back_to_user_profile() {
    // Act: 空串视为未设置
    let home = yaoxiang_home_from(Some(""), Some("/Users/dev")).unwrap();

    // Assert
    assert_eq!(
        home,
        std::path::Path::new("/Users/dev/.yaoxiang"),
        "empty YAOXIANG_HOME must fall back to ~/.yaoxiang"
    );
}

#[test]
fn test_missing_both_homes_reports_no_home() {
    // Act
    let home = yaoxiang_home_from(None, None);

    // Assert
    assert!(
        matches!(home, Err(Error::NoHome)),
        "no env and no user home must yield NoHome, got {home:?}"
    );
}
