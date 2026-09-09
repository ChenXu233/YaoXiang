//! 平台识别与发行包命名测试（RFC-037 §平台支持 / §发布目录结构）

use crate::error::Error;
use crate::platform::{archive_ext_for, asset_name, engine_file_name_for, triple_for, yx_file_name_for};

#[test]
fn test_triple_for_maps_all_five_supported_targets() {
    // Arrange: RFC-037 平台表的 5 个 target
    const TABLE: &[(&str, &str, &str)] = &[
        ("linux", "x86_64", "x86_64-unknown-linux-gnu"),
        ("linux", "aarch64", "aarch64-unknown-linux-gnu"),
        ("macos", "x86_64", "x86_64-apple-darwin"),
        ("macos", "aarch64", "aarch64-apple-darwin"),
        ("windows", "x86_64", "x86_64-pc-windows-msvc"),
    ];

    // Act
    let mapped: Vec<&str> = TABLE
        .iter()
        .map(|(os, arch, _)| triple_for(os, arch).unwrap())
        .collect();

    // Assert: (os, arch) → triple 一一对应，不得漂移
    for ((os, arch, expected), actual) in TABLE.iter().zip(mapped.iter()) {
        assert_eq!(
            *expected, *actual,
            "triple for {os}/{arch} must match the RFC-037 platform table"
        );
    }
}

#[test]
fn test_triple_for_rejects_unsupported_platform() {
    // Act
    let result = triple_for("freebsd", "x86_64");

    // Assert: Z3 无全平台预编译，未支持平台必须明确报错而非猜一个 triple
    assert!(
        matches!(result, Err(Error::UnsupportedPlatform { .. })),
        "unsupported platform must yield UnsupportedPlatform, got {result:?}"
    );
}

#[test]
fn test_archive_ext_is_zip_only_on_windows() {
    // Arrange
    const TABLE: &[(&str, &str)] = &[("windows", "zip"), ("linux", "tar.gz"), ("macos", "tar.gz")];

    // Act
    let mapped: Vec<&str> = TABLE.iter().map(|(os, _)| archive_ext_for(os)).collect();

    // Assert: Windows 发 zip，其余发 tar.gz（package-dist.sh 约定）
    for ((os, expected), actual) in TABLE.iter().zip(mapped.iter()) {
        assert_eq!(*expected, *actual, "archive ext for {os}");
    }
}

#[test]
fn test_asset_name_follows_package_dist_convention() {
    // Act
    let asset = asset_name("0.7.14", "x86_64-unknown-linux-gnu", "linux");

    // Assert: 与 package-dist.sh 的命名一致
    assert_eq!(
        asset, "yaoxiang-0.7.14-x86_64-unknown-linux-gnu.tar.gz",
        "asset name must match package-dist.sh output"
    );
}

#[test]
fn test_binary_file_names_carry_exe_suffix_on_windows_only() {
    // Arrange
    const TABLE: &[(&str, &str, &str)] = &[
        ("windows", "yaoxiang-rs.exe", "yx.exe"),
        ("linux", "yaoxiang-rs", "yx"),
        ("macos", "yaoxiang-rs", "yx"),
    ];

    // Act
    let mapped: Vec<(&str, &str)> = TABLE
        .iter()
        .map(|(os, _, _)| (engine_file_name_for(os), yx_file_name_for(os)))
        .collect();

    // Assert: 只有 Windows 带 .exe 后缀
    for ((os, engine, yx), (engine_actual, yx_actual)) in TABLE.iter().zip(mapped.iter()) {
        assert_eq!(engine, engine_actual, "engine file name for {os}");
        assert_eq!(yx, yx_actual, "front-door file name for {os}");
    }
}
