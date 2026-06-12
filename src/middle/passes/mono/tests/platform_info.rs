//! 平台信息单元测试
//!
//! 测试 TargetPlatform、PlatformDetector 和平台参数检测功能。

use crate::middle::passes::mono::platform_info::{
    is_platform_param, PlatformDetector, TargetPlatform,
};

#[test]
fn test_parse_x86_64() {
    let target = "x86_64-unknown-linux-gnu";
    let info = PlatformDetector::detect_from_target(target);
    assert_eq!(info.target(), &TargetPlatform::X86_64);
    assert_eq!(info.vendor(), "unknown");
    assert_eq!(info.os(), "linux");
    assert_eq!(info.environment(), "gnu");
}

#[test]
fn test_parse_aarch64() {
    let target = "aarch64-apple-darwin";
    let info = PlatformDetector::detect_from_target(target);
    assert_eq!(info.target(), &TargetPlatform::AArch64);
    assert_eq!(info.vendor(), "apple");
}

#[test]
fn test_platform_display() {
    assert_eq!(TargetPlatform::X86_64.to_string(), "X86_64");
    assert_eq!(TargetPlatform::AArch64.to_string(), "AArch64");
}

#[test]
fn test_is_platform_param() {
    assert!(is_platform_param("P"));
    assert!(!is_platform_param("T"));
    assert!(!is_platform_param("X86_64"));
}
