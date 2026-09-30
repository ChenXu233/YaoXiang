//! cargo 构建策略测试 — 基于 RFC-014b §cargo 策略详解
//!
//! 覆盖：`[build.cargo]` 拼命令（features/target 档位校验）、平台覆盖合并
//! （当前平台 `cargo-features` 追加进基础 features，去重）、产物两层分离
//! （scratch 经 CARGO_TARGET_DIR 隔离、FFI 产物复制进 build/native/<triple>/）。

use crate::package::build::{self, TrustDecision};
use crate::package::error::PackageError;
use crate::package::manifest::{BuildConfig, CargoBuildConfig, PackageManifest, PlatformOverrides};

use super::write;

fn load(pkg: &std::path::Path) -> PackageManifest {
    PackageManifest::load(pkg).expect("load fixture manifest")
}

#[test]
fn test_cargo_strategy_builds_real_crate_and_copies_artifacts() {
    // Arrange：最小 cdylib crate（真实编译）
    let tmp = tempfile::tempdir().unwrap();
    let pkg = tmp.path().join("native-demo-1.0.0");
    write(
        &pkg.join("yaoxiang.toml"),
        "[package]\nname = \"native-demo\"\nversion = \"1.0.0\"\n\n[build]\nstrategy = \"cargo\"\n\n[build.cargo]\ntarget = \"release\"\n",
    );
    write(
        &pkg.join("Cargo.toml"),
        "[package]\nname = \"native_demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n",
    );
    write(
        &pkg.join("src/lib.rs"),
        "#[no_mangle]\npub extern \"C\" fn yx_demo_add(a: i32, b: i32) -> i32 { a + b }\n",
    );
    let scratch = tmp.path().join(".yaoxiang").join("build");

    // Act
    let outcome = crate::package::runtime::drive(build::run_install_build(
        &pkg,
        &load(&pkg),
        &scratch,
        None,
        &TrustDecision::default(),
    ))
    .unwrap();

    // Assert：产物落 build/native/<triple>/，scratch 不污染包目录
    assert_eq!(outcome.via, "cargo");
    let native = outcome.native_dir.expect("应有产物目录");
    assert!(
        native.ends_with(format!("build/native/{}", build::current_triple()).as_str()),
        "产物目录应为 build/native/<triple>: {:?}",
        native
    );
    let has_lib = std::fs::read_dir(&native).unwrap().flatten().any(|e| {
        let n = e.file_name().to_string_lossy().to_string();
        n.ends_with(".dll") || n.ends_with(".so") || n.ends_with(".dylib")
    });
    assert!(has_lib, "cdylib 产物应复制进 {:?}", native);
    assert!(!pkg.join("target").exists(), "cargo scratch 不得污染包目录");
    assert!(
        scratch.join("cargo").join("native-demo-1.0.0").exists(),
        "scratch 应位于项目 .yaoxiang/build/cargo/<pkg>/"
    );
}

#[test]
fn test_cargo_features_merge_platform_overrides() {
    // Arrange：基础 features + 当前平台覆盖（含重复项）
    let mut config = BuildConfig {
        cargo: Some(CargoBuildConfig {
            features: vec!["ffi".to_string()],
            target: None,
        }),
        ..Default::default()
    };
    config.platforms.insert(
        build::current_triple(),
        PlatformOverrides {
            cargo_features: vec!["ffi".to_string(), "platform-extra".to_string()],
        },
    );

    // Act
    let merged = build::cargo::compose_features(&config);

    // Assert：追加 + 去重（RFC：--features ffi,linux-ffi 形态）
    assert_eq!(
        merged,
        vec!["ffi".to_string(), "platform-extra".to_string()]
    );
}

#[test]
fn test_cargo_target_rejects_unknown_profile() {
    // Arrange：target 只支持 release/debug（RFC-014b §cargo 策略详解）
    let tmp = tempfile::tempdir().unwrap();
    let pkg = tmp.path().join("p-0.1.0");
    write(
        &pkg.join("yaoxiang.toml"),
        "[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nstrategy = \"cargo\"\n\n[build.cargo]\ntarget = \"nightly\"\n",
    );

    // Act
    let err = crate::package::runtime::drive(build::run_install_build(
        &pkg,
        &load(&pkg),
        &tmp.path().join("scratch"),
        None,
        &TrustDecision::default(),
    ))
    .unwrap_err();

    // Assert
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("release")),
        "报错应说明合法档位: {err}"
    );
}
