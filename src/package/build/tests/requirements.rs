//! 系统依赖预检测试 — 基于 RFC-014b §系统依赖检查 + 2026-09-15 决议 6
//!
//! 决议 6：Cargo 版本不兼容走 `[build.requirements]` 预检报错 + 安装指引，
//! 无额外机制。覆盖：两段版本操作数补齐（`>= 1.70`）、组合要求、裸版本
//! 精确语义、缺失工具与版本不满足的报错文案（含 Install: 指引）。

use std::collections::BTreeMap;

use crate::package::build::requirements;
use crate::package::error::PackageError;

#[test]
fn test_tool_req_pads_two_segment_operand() {
    // Act：RFC 示例两段操作数应补齐三段后匹配
    let req = requirements::parse_tool_req(">= 1.70").unwrap();

    // Assert
    assert!(
        req.matches(&semver::Version::parse("1.88.0").unwrap()),
        "1.88.0 应满足 >= 1.70"
    );
    assert!(
        !req.matches(&semver::Version::parse("1.69.0").unwrap()),
        "1.69.0 不应满足 >= 1.70"
    );
}

#[test]
fn test_tool_req_multi_comparator_intersection() {
    // Act
    let multi = requirements::parse_tool_req(">=3.20, <4.0").unwrap();

    // Assert
    assert!(
        multi.matches(&semver::Version::parse("3.29.6").unwrap()),
        "3.29.6 应落在 [3.20, 4.0)"
    );
    assert!(
        !multi.matches(&semver::Version::parse("4.1.0").unwrap()),
        "4.1.0 不应落在 [3.20, 4.0)"
    );
}

#[test]
fn test_tool_req_bare_version_is_exact() {
    // Act：裸版本 = 精确要求（与 resolver::parse_version_req 口径一致）
    let exact = requirements::parse_tool_req("3.29").unwrap();

    // Assert
    assert!(
        exact.matches(&semver::Version::parse("3.29.0").unwrap()),
        "3.29.0 应精确匹配 3.29"
    );
    assert!(
        !exact.matches(&semver::Version::parse("3.30.0").unwrap()),
        "3.30.0 不应精确匹配 3.29"
    );
}

#[test]
fn test_requirements_check_reports_missing_tool_with_hint() {
    // Arrange
    let mut reqs = BTreeMap::new();
    reqs.insert(
        "yx-definitely-not-a-real-tool".to_string(),
        ">= 1.0".to_string(),
    );

    // Act
    let err = requirements::check(&reqs).unwrap_err();

    // Assert
    match err {
        PackageError::InvalidManifest(msg) => {
            assert!(msg.contains("构建依赖不满足"), "got: {msg}");
            assert!(msg.contains("不在 PATH"), "got: {msg}");
            assert!(msg.contains("Install:"), "应有安装指引: {msg}");
        }
        other => panic!("expected InvalidManifest, got {other}"),
    }
}

#[test]
fn test_requirements_check_cargo_satisfied() {
    // Arrange：CI/dev 环境必有 cargo，真实核对走通（含两段要求补齐）
    let mut reqs = BTreeMap::new();
    reqs.insert("cargo".to_string(), ">= 1.70".to_string());

    // Act / Assert
    requirements::check(&reqs).unwrap();
}

#[test]
fn test_requirements_check_reports_version_mismatch() {
    // Arrange
    let mut reqs = BTreeMap::new();
    reqs.insert("cargo".to_string(), ">= 999.0".to_string());

    // Act
    let err = requirements::check(&reqs).unwrap_err();

    // Assert
    match err {
        PackageError::InvalidManifest(msg) => {
            assert!(msg.contains("版本不满足"), "got: {msg}");
            assert!(msg.contains("https://rustup.rs"), "got: {msg}");
        }
        other => panic!("expected InvalidManifest, got {other}"),
    }
}
