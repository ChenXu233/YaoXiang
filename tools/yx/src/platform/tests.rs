//! 平台识别与发行包命名测试（RFC-037 §平台支持 / §发布目录结构）

use crate::error::Error;
use crate::platform::{archive_ext_for, asset_name, engine_file_name_for, triple_for, yx_file_name_for};

#[test]
fn test_triple_for_maps_all_five_supported_targets() {
    // Act & Assert: RFC-037 平台表的 5 个 target 一一对应
    assert_eq!(
        triple_for("linux", "x86_64").unwrap(),
        "x86_64-unknown-linux-gnu",
        "linux x86_64 must map to gnu triple"
    );
    assert_eq!(
        triple_for("linux", "aarch64").unwrap(),
        "aarch64-unknown-linux-gnu",
        "linux aarch64 must map to gnu triple"
    );
    assert_eq!(
        triple_for("macos", "x86_64").unwrap(),
        "x86_64-apple-darwin",
        "macOS x86_64 must map to darwin triple"
    );
    assert_eq!(
        triple_for("macos", "aarch64").unwrap(),
        "aarch64-apple-darwin",
        "macOS aarch64 must map to darwin triple"
    );
    assert_eq!(
        triple_for("windows", "x86_64").unwrap(),
        "x86_64-pc-windows-msvc",
        "windows x86_64 must map to msvc triple"
    );
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
    // Act & Assert
    assert_eq!(archive_ext_for("windows"), "zip", "windows ships zip");
    assert_eq!(archive_ext_for("linux"), "tar.gz", "linux ships tar.gz");
    assert_eq!(archive_ext_for("macos"), "tar.gz", "macos ships tar.gz");
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
    // Act & Assert
    assert_eq!(
        engine_file_name_for("windows"),
        "yaoxiang-rs.exe",
        "windows engine must have .exe"
    );
    assert_eq!(engine_file_name_for("linux"), "yaoxiang-rs");
    assert_eq!(yx_file_name_for("windows"), "yx.exe");
    assert_eq!(yx_file_name_for("macos"), "yx");
}
