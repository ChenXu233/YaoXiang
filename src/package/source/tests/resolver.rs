//! 测试语义化版本解析器
//!
//! 覆盖:
//! - SemVer 解析（完整/两段/单段/预发布版本）
//! - 版本 Display 输出
//! - 版本排序
//! - 无效版本解析错误
//! - VersionReq 解析（caret/tilde/exact/wildcard/gte/gt/lte/lt/compound）
//! - VersionReq 匹配检查
//! - VersionReq Display 输出
//! - select_best 选择最佳版本
//! - 版本兼容性检查

use crate::package::source::resolver::{SemVer, VersionReq};

// === SemVer 解析测试 ===

#[test]
fn test_parse_full_version() {
    let v = SemVer::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.pre, None);
}

#[test]
fn test_parse_two_part_version() {
    let v = SemVer::parse("1.2").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 0);
}

#[test]
fn test_parse_single_part_version() {
    let v = SemVer::parse("1").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 0);
}

#[test]
fn test_parse_prerelease_version() {
    let v = SemVer::parse("1.0.0-alpha").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 0);
    assert_eq!(v.pre, Some("alpha".to_string()));
}

#[test]
fn test_version_display() {
    assert_eq!(SemVer::new(1, 2, 3).to_string(), "1.2.3");
    assert_eq!(SemVer::with_pre(1, 0, 0, "beta").to_string(), "1.0.0-beta");
}

#[test]
fn test_version_ordering() {
    assert!(SemVer::new(1, 0, 0) < SemVer::new(2, 0, 0));
    assert!(SemVer::new(1, 0, 0) < SemVer::new(1, 1, 0));
    assert!(SemVer::new(1, 0, 0) < SemVer::new(1, 0, 1));
    assert!(SemVer::with_pre(1, 0, 0, "alpha") < SemVer::new(1, 0, 0));
}

#[test]
fn test_parse_invalid_version() {
    assert!(SemVer::parse("invalid").is_err());
    assert!(SemVer::parse("1.2.3.4").is_err());
}

// === VersionReq 解析测试 ===

#[test]
fn test_parse_caret_version() {
    // ^1.2.3 → >=1.2.3, <2.0.0
    let req = VersionReq::parse("^1.2.3").unwrap();
    assert!(req.matches(&SemVer::new(1, 2, 3)));
    assert!(req.matches(&SemVer::new(1, 9, 9)));
    assert!(!req.matches(&SemVer::new(2, 0, 0)));
    assert!(!req.matches(&SemVer::new(1, 2, 2)));
}

#[test]
fn test_parse_caret_zero_major() {
    // ^0.2.3 → >=0.2.3, <0.3.0
    let req = VersionReq::parse("^0.2.3").unwrap();
    assert!(req.matches(&SemVer::new(0, 2, 3)));
    assert!(req.matches(&SemVer::new(0, 2, 9)));
    assert!(!req.matches(&SemVer::new(0, 3, 0)));
}

#[test]
fn test_parse_tilde_version() {
    // ~1.2.3 → >=1.2.3, <1.3.0
    let req = VersionReq::parse("~1.2.3").unwrap();
    assert!(req.matches(&SemVer::new(1, 2, 3)));
    assert!(req.matches(&SemVer::new(1, 2, 9)));
    assert!(!req.matches(&SemVer::new(1, 3, 0)));
    assert!(!req.matches(&SemVer::new(1, 2, 2)));
}

#[test]
fn test_parse_exact_version() {
    let req = VersionReq::parse("1.0.0").unwrap();
    assert!(req.matches(&SemVer::new(1, 0, 0)));
    assert!(!req.matches(&SemVer::new(1, 0, 1)));
    assert!(!req.matches(&SemVer::new(0, 9, 9)));
}

#[test]
fn test_parse_wildcard() {
    let req = VersionReq::parse("*").unwrap();
    assert!(req.matches(&SemVer::new(0, 0, 0)));
    assert!(req.matches(&SemVer::new(99, 99, 99)));
}

#[test]
fn test_parse_gte() {
    let req = VersionReq::parse(">=1.0.0").unwrap();
    assert!(req.matches(&SemVer::new(1, 0, 0)));
    assert!(req.matches(&SemVer::new(2, 0, 0)));
    assert!(!req.matches(&SemVer::new(0, 9, 9)));
}

#[test]
fn test_parse_gt() {
    let req = VersionReq::parse(">1.0.0").unwrap();
    assert!(!req.matches(&SemVer::new(1, 0, 0)));
    assert!(req.matches(&SemVer::new(1, 0, 1)));
}

#[test]
fn test_parse_lte() {
    let req = VersionReq::parse("<=1.0.0").unwrap();
    assert!(req.matches(&SemVer::new(1, 0, 0)));
    assert!(req.matches(&SemVer::new(0, 9, 9)));
    assert!(!req.matches(&SemVer::new(1, 0, 1)));
}

#[test]
fn test_parse_lt() {
    let req = VersionReq::parse("<1.0.0").unwrap();
    assert!(!req.matches(&SemVer::new(1, 0, 0)));
    assert!(req.matches(&SemVer::new(0, 9, 9)));
}

#[test]
fn test_parse_compound() {
    // >=1.2.3, <2.0.0
    let req = VersionReq::parse(">=1.2.3, <2.0.0").unwrap();
    assert!(req.matches(&SemVer::new(1, 2, 3)));
    assert!(req.matches(&SemVer::new(1, 9, 9)));
    assert!(!req.matches(&SemVer::new(2, 0, 0)));
    assert!(!req.matches(&SemVer::new(1, 2, 2)));
}

#[test]
fn test_version_req_display() {
    let req = VersionReq::parse("*").unwrap();
    assert_eq!(req.to_string(), "*");

    let req = VersionReq::parse("^1.0.0").unwrap();
    assert_eq!(req.to_string(), ">=1.0.0, <2.0.0");
}

// === select_best 测试 ===

#[test]
fn test_select_best_version() {
    let req = VersionReq::parse("^1.0.0").unwrap();
    let versions = vec![
        SemVer::new(0, 9, 0),
        SemVer::new(1, 0, 0),
        SemVer::new(1, 5, 0),
        SemVer::new(1, 9, 9),
        SemVer::new(2, 0, 0),
    ];
    let best = req.select_best(&versions).unwrap();
    assert_eq!(*best, SemVer::new(1, 9, 9));
}

#[test]
fn test_select_best_no_match() {
    let req = VersionReq::parse("^3.0.0").unwrap();
    let versions = vec![SemVer::new(1, 0, 0), SemVer::new(2, 0, 0)];
    assert!(req.select_best(&versions).is_none());
}

// === 兼容性测试 ===

#[test]
fn test_compatible_versions() {
    let req1 = VersionReq::parse("^1.0.0").unwrap();
    let req2 = VersionReq::parse("^1.5.0").unwrap();
    assert!(req1.is_compatible(&req2));
}

#[test]
fn test_incompatible_versions() {
    let req1 = VersionReq::parse("^1.0.0").unwrap();
    let req2 = VersionReq::parse("^2.0.0").unwrap();
    assert!(!req1.is_compatible(&req2));
}

#[test]
fn test_wildcard_compatible_with_anything() {
    let req1 = VersionReq::parse("*").unwrap();
    let req2 = VersionReq::parse("^1.0.0").unwrap();
    assert!(req1.is_compatible(&req2));
}
