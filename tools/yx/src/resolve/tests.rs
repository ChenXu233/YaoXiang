//! 版本解析优先级测试（RFC-037：pin > 默认版本 > 相邻引擎）

use std::fs;
use tempfile::TempDir;

use crate::error::Error;
use crate::platform::engine_file_name;
use crate::resolve::{resolve_in, Resolution};

/// 搭一棵托管安装树：`<home>/versions/<ver>/bin/<引擎>`（空文件即可，只测存在性）。
/// 引擎名按宿主平台取——resolve_in 内部用 host 命名。
fn install_stub_version(
    home: &std::path::Path,
    version: &str,
) -> std::path::PathBuf {
    let engine = home
        .join("versions")
        .join(version)
        .join("bin")
        .join(engine_file_name());
    fs::create_dir_all(engine.parent().unwrap()).unwrap();
    fs::write(&engine, "").unwrap();
    engine
}

fn write_default(
    home: &std::path::Path,
    version: &str,
) {
    fs::write(
        home.join("settings.toml"),
        format!("default = \"{version}\"\n"),
    )
    .unwrap();
}

#[test]
fn test_resolve_pin_beats_default_version() {
    // Arrange: pin 0.7.13、默认 0.7.14，两者都已安装
    let home = TempDir::new().unwrap();
    let pin_engine = install_stub_version(home.path(), "0.7.13");
    install_stub_version(home.path(), "0.7.14");
    write_default(home.path(), "0.7.14");
    fs::write(
        home.path().join("yx-toolchain.toml"),
        "toolchain = \"0.7.13\"\n",
    )
    .unwrap();
    let cwd = home.path();

    // Act
    let resolved = resolve_in(home.path(), cwd, None).unwrap();

    // Assert: 项目 pin 优先于默认版本
    assert_eq!(
        resolved,
        Resolution::Managed {
            version: "0.7.13".into(),
            engine: pin_engine,
        },
        "project pin must win over settings default"
    );
}

#[test]
fn test_resolve_uses_default_when_no_pin() {
    // Arrange: 只有默认版本 0.7.14
    let home = TempDir::new().unwrap();
    let engine = install_stub_version(home.path(), "0.7.14");
    write_default(home.path(), "0.7.14");

    // Act
    let resolved = resolve_in(home.path(), home.path(), None).unwrap();

    // Assert
    assert_eq!(
        resolved,
        Resolution::Managed {
            version: "0.7.14".into(),
            engine,
        },
        "settings default must be used without pin"
    );
}

#[test]
fn test_resolve_falls_back_to_adjacent_engine_portably() {
    // Arrange: 无 pin、无默认版本，但 yx 与引擎相邻（解压包 bin/ 布局）
    let pkg = TempDir::new().unwrap();
    let bin = pkg.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let engine = bin.join(engine_file_name());
    fs::write(&engine, "").unwrap();

    // Act
    let resolved = resolve_in(pkg.path(), pkg.path(), Some(&bin)).unwrap();

    // Assert: 便携解压必须回退到相邻引擎，与托管安装行为一致
    assert_eq!(
        resolved,
        Resolution::Portable { engine },
        "portable layout must resolve to adjacent engine"
    );
}

#[test]
fn test_resolve_missing_pinned_version_fails_loudly() {
    // Arrange: pin 指向未安装版本
    let home = TempDir::new().unwrap();
    fs::write(
        home.path().join("yx-toolchain.toml"),
        "toolchain = \"9.9.9\"\n",
    )
    .unwrap();

    // Act
    let resolved = resolve_in(home.path(), home.path(), None);

    // Assert: 显式失败并提示安装，不静默回退
    assert!(
        matches!(resolved, Err(Error::NotInstalled { ref version }) if version == "9.9.9"),
        "missing pinned version must fail with NotInstalled, got {resolved:?}"
    );
}

#[test]
fn test_resolve_nothing_configured_reports_no_toolchain() {
    // Arrange: 空安装根
    let home = TempDir::new().unwrap();

    // Act
    let resolved = resolve_in(home.path(), home.path(), None);

    // Assert
    assert!(
        matches!(resolved, Err(Error::NoToolchain)),
        "empty home must yield NoToolchain, got {resolved:?}"
    );
}
