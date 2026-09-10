//! yx-toolchain.toml 项目 pin 测试（RFC-037 §安装器支持）

use std::fs;
use tempfile::TempDir;

use crate::pin::{find_pin_from, normalize_version};

#[test]
fn test_find_pin_discovers_file_in_start_directory() {
    // Arrange
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("yx-toolchain.toml"),
        "toolchain = \"0.7.14\"\n",
    )
    .unwrap();

    // Act
    let pin = find_pin_from(dir.path());

    // Assert
    assert_eq!(
        pin.as_deref(),
        Some("0.7.14"),
        "pin file in start dir must be discovered"
    );
}

#[test]
fn test_find_pin_walks_up_to_parent_directory() {
    // Arrange: pin 在父目录，cwd 在子目录
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("yx-toolchain.toml"),
        "toolchain = \"0.8.0\"\n",
    )
    .unwrap();
    let child = dir.path().join("src/sub");
    fs::create_dir_all(&child).unwrap();

    // Act
    let pin = find_pin_from(&child);

    // Assert: 与 rust-toolchain.toml 同样的向上查找语义
    assert_eq!(
        pin.as_deref(),
        Some("0.8.0"),
        "pin in ancestor dir must be found from nested cwd"
    );
}

#[test]
fn test_find_pin_returns_none_without_file() {
    // Arrange
    let dir = TempDir::new().unwrap();

    // Act
    let pin = find_pin_from(dir.path());

    // Assert
    assert!(pin.is_none(), "no pin file must yield None, got {pin:?}");
}

#[test]
fn test_normalize_version_strips_v_prefix() {
    // Act & Assert: Release tag 带 v 前缀，两种写法都要接受
    assert_eq!(
        normalize_version("v0.7.14"),
        "0.7.14",
        "v-prefixed tag must normalize"
    );
    assert_eq!(
        normalize_version("0.7.14"),
        "0.7.14",
        "bare version must stay unchanged"
    );
}

#[test]
fn test_find_pin_ignores_malformed_file_with_warning() {
    // Arrange: 缺少 toolchain 键的坏 pin 文件
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("yx-toolchain.toml"), "garbage = true\n").unwrap();

    // Act
    let pin = find_pin_from(dir.path());

    // Assert: 显式按无 pin 处理（stderr 告警），不得误派发
    assert!(pin.is_none(), "malformed pin must yield None, got {pin:?}");
}
