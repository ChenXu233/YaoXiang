//! toolchain 动词的纯函数测试（RFC-037 §安装器支持）
//!
//! 覆盖：
//! - releases/latest 重定向落地 URL 的版本提取（API 不可达回退路径）
//! - 卸载前置检查的护栏语义（默认版本与项目 pin 均拒绝）

use crate::error::Error;
use crate::toolchain::{check_uninstall_allowed, compare_versions_desc, parse_tag_from_release_url};

#[test]
fn test_parse_tag_extracts_version_from_release_landing_url() {
    // Act & Assert: releases/latest 302 落地形态
    assert_eq!(
        parse_tag_from_release_url("https://github.com/ChenXu233/YaoXiang/releases/tag/v0.8.0")
            .as_deref(),
        Some("0.8.0"),
        "tag landing URL must yield normalized version"
    );
}

#[test]
fn test_parse_tag_tolerates_query_and_fragment() {
    // Act & Assert: 落地 URL 可能带追踪参数
    assert_eq!(
        parse_tag_from_release_url(
            "https://github.com/ChenXu233/YaoXiang/releases/tag/v1.2.3?utm_source=x#readme"
        )
        .as_deref(),
        Some("1.2.3"),
        "query and fragment must be stripped"
    );
}

#[test]
fn test_parse_tag_rejects_non_tag_landing_url() {
    // Act & Assert: 未发布过任何 release 时 latest 落地不是 tag 形态
    let result = parse_tag_from_release_url("https://github.com/ChenXu233/YaoXiang");
    assert!(
        result.is_none(),
        "URL without /releases/tag/ must yield None, got {result:?}"
    );
}

#[test]
fn test_check_uninstall_allowed_rejects_default_version() {
    // Act
    let result = check_uninstall_allowed("0.7.14", Some("0.7.14"), None);

    // Assert: 正在使用的默认版本不允许直接卸载
    assert!(
        matches!(result, Err(Error::IsDefault { .. })),
        "default version must be refused, got {result:?}"
    );
}

#[test]
fn test_check_uninstall_allowed_rejects_pinned_version() {
    // Act
    let result = check_uninstall_allowed("0.7.13", Some("0.7.14"), Some("0.7.13"));

    // Assert: 项目 pin 的版本卸掉会让派发直接断，必须先解 pin
    assert!(
        matches!(&result, Err(Error::Message(m)) if m.contains("pinned")),
        "pinned version must be refused with pin hint, got {result:?}"
    );
}

#[test]
fn test_check_uninstall_allowed_accepts_unrelated_version() {
    // Act
    let result = check_uninstall_allowed("0.7.13", Some("0.7.14"), Some("0.9.0"));

    // Assert: 既非默认也非 pin 的版本可以卸载
    assert!(
        result.is_ok(),
        "unrelated version must be uninstallable, got {result:?}"
    );
}

#[test]
fn test_compare_versions_desc_orders_semver_numerically() {
    // Arrange: 字符串序会把 0.10.0 排在 0.9.0 后
    let mut versions = vec!["0.9.0", "0.10.0", "0.7.14"];

    // Act
    versions.sort_by(|a, b| compare_versions_desc(a, b));

    // Assert
    assert_eq!(
        versions,
        vec!["0.10.0", "0.9.0", "0.7.14"],
        "semver must sort numerically, not lexicographically"
    );
}

#[test]
fn test_compare_versions_desc_puts_non_semver_last() {
    // Arrange: nightly 这类散目录名不属于 semver
    let mut versions = vec!["nightly", "0.7.14", "0.8.0-rc1"];

    // Act
    versions.sort_by(|a, b| compare_versions_desc(a, b));

    // Assert
    assert_eq!(
        versions,
        vec!["0.8.0-rc1", "0.7.14", "nightly"],
        "non-semver names must sort last"
    );
}
