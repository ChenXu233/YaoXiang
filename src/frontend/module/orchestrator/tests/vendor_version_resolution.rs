//! vendor 版本目录解析测试 — RFC-014 §模块解析顺序 priority 2
//!
//! `.yaoxiang/vendor/<pkg>-<ver>/` 多版本并存时取版本最大者；带非数字后缀的
//! 预发布版本（如 `1.0.0-beta`）排在同数值正式版之后，不得遮蔽正式版。

use std::cmp::Ordering;
use std::path::Path;

use tempfile::TempDir;

use super::{compare_version, resolve_in_vendor};

#[test]
fn test_compare_version_release_outranks_prerelease() {
    // Assert - 同数值下正式版必须压过预发布版（semver 语义）
    assert_eq!(
        compare_version("1.0.0", "1.0.0-beta"),
        Ordering::Greater,
        "release 1.0.0 must outrank prerelease 1.0.0-beta"
    );
}

#[test]
fn test_compare_version_numeric_fields_compare_numerically() {
    // Assert - 数值字段按数值比较，不得退化为字典序（"10" < "9" 是字典序）
    assert_eq!(
        compare_version("1.10.0", "1.9.0"),
        Ordering::Greater,
        "1.10.0 must be greater than 1.9.0 numerically"
    );
}

#[test]
fn test_compare_version_identical_versions_are_equal() {
    // Assert - 完全相同的版本号必须判等
    assert_eq!(
        compare_version("2.1.0", "2.1.0"),
        Ordering::Equal,
        "identical versions must compare Equal"
    );
}

#[test]
fn test_resolve_in_vendor_prefers_release_over_prerelease() {
    // Arrange - 同包两版本并存：1.0.0（正式）与 1.0.0-beta（预发布）
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let vendor = tmp.path().join(".yaoxiang").join("vendor");
    for version in ["foo-1.0.0", "foo-1.0.0-beta"] {
        let src = vendor.join(version).join("src");
        std::fs::create_dir_all(&src).expect("Failed to create vendor layout");
        std::fs::write(src.join("foo.yx"), "value: Int = 42\n")
            .expect("Failed to write vendor module");
    }

    // Act
    let resolved = resolve_in_vendor("foo", tmp.path());

    // Assert - 必须选中正式版 1.0.0，预发布目录不得遮蔽
    let resolved = resolved.expect("vendor module must resolve");
    let expected_tail = Path::new("foo-1.0.0").join("src").join("foo.yx");
    assert!(
        resolved.ends_with(&expected_tail),
        "release 1.0.0 must win over prerelease 1.0.0-beta, got: {}",
        resolved.display()
    );
}

#[test]
fn test_resolve_in_vendor_picks_highest_numeric_version() {
    // Arrange - 同包两个正式版本：1.0.0 与 1.2.0
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let vendor = tmp.path().join(".yaoxiang").join("vendor");
    for version in ["foo-1.0.0", "foo-1.2.0"] {
        let src = vendor.join(version).join("src");
        std::fs::create_dir_all(&src).expect("Failed to create vendor layout");
        std::fs::write(src.join("foo.yx"), "value: Int = 42\n")
            .expect("Failed to write vendor module");
    }

    // Act
    let resolved = resolve_in_vendor("foo", tmp.path());

    // Assert - 降序排序必须选中 1.2.0
    let resolved = resolved.expect("vendor module must resolve");
    let expected_tail = Path::new("foo-1.2.0").join("src").join("foo.yx");
    assert!(
        resolved.ends_with(&expected_tail),
        "highest numeric version 1.2.0 must win, got: {}",
        resolved.display()
    );
}
