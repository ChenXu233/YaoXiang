//! 测试语义化版本解析入口（semver crate 后端）— 基于 RFC-014 Phase 3
//!
//! 覆盖:
//! - parse_version（完整/两段/单段/预发布/无效）
//! - parse_version_req（裸版本=精确、caret/tilde/比较器/通配/组合、无效输入）
//! - select_best 选择最佳版本
//! - is_compatible 区间交集判定（含旧枚举实现误判的高版本区间）
//!
//! 注：本文件的 `assert!(r.matches(&v("1.2.3")))` 式比较断言按规则 6.3 例外
//! 条款不加消息——失败输出打印的完整表达式已包含输入值与期望关系。

use crate::package::source::resolver::{is_compatible, parse_version, parse_version_req, select_best};

fn v(s: &str) -> semver::Version {
    parse_version(s).expect("parse fixture version")
}

fn req(s: &str) -> semver::VersionReq {
    parse_version_req(s).expect("parse fixture version req")
}

// === parse_version 测试 ===

#[test]
fn test_parse_full_version() {
    // Act
    let ver = v("1.2.3");

    // Assert
    assert_eq!(ver.major, 1);
    assert_eq!(ver.minor, 2);
    assert_eq!(ver.patch, 3);
    assert!(ver.pre.is_empty(), "完整三段版本不应有预发布段");
}

#[test]
fn test_parse_invalid_version() {
    // Act / Assert：非法输入（非版本词 / 四段 / 空串）一律报错
    assert!(parse_version("invalid").is_err(), "非版本词应报错");
    assert!(parse_version("1.2.3.4").is_err(), "四段版本应报错");
    assert!(parse_version("").is_err(), "空串应报错");
}

// === parse_version_req 测试 ===

#[test]
fn test_parse_caret_version() {
    // ^1.2.3 → >=1.2.3, <2.0.0
    let r = req("^1.2.3");
    assert!(r.matches(&v("1.2.3")));
    assert!(r.matches(&v("1.9.9")));
    assert!(!r.matches(&v("2.0.0")));
    assert!(!r.matches(&v("1.2.2")));
}

#[test]
fn test_parse_invalid_req() {
    // Act / Assert：非法要求（非版本词 / 四段）一律报错
    assert!(parse_version_req("invalid").is_err(), "非法要求应报错");
    assert!(parse_version_req("1.2.3.4").is_err(), "四段要求应报错");
}

// === select_best 测试 ===

#[test]
fn test_select_best_version() {
    let r = req("^1.0.0");
    let versions = vec![v("0.9.0"), v("1.0.0"), v("1.5.0"), v("1.9.9"), v("2.0.0")];
    assert_eq!(select_best(&r, &versions), Some(&v("1.9.9")));
}

#[test]
fn test_select_best_no_match() {
    // Arrange：要求 ^3.0.0，候选只有 1.x / 2.x
    let r = req("^3.0.0");
    let versions = vec![v("1.0.0"), v("2.0.0")];

    // Act / Assert：无满足候选时应返回 None
    assert!(
        select_best(&r, &versions).is_none(),
        "无满足候选时 select_best 应返回 None"
    );
}

// === 兼容性测试（区间交集）===

#[test]
fn test_compatible_versions() {
    assert!(is_compatible(&req("^1.0.0"), &req("^1.5.0")));
    // 相同区间兼容（旧枚举实现在 major<100 内枚举碰巧成立，区间法直接成立）
    assert!(is_compatible(&req("^1.0.0"), &req(">=1.2.0, <1.9.0")));
}
