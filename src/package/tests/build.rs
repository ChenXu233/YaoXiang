//! 构建系统测试（RFC-014b Phase 5a/5b：[build] 解析、策略枚举、平台三元组、依赖预检）

use std::collections::BTreeMap;

use crate::package::build::{self, requirements, BuildStrategy, TrustDecision};
use crate::package::error::PackageError;
use crate::package::manifest::PackageManifest;

fn write(
    path: &std::path::Path,
    content: &str,
) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

#[test]
fn parses_full_build_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write(
        &dir.join("yaoxiang.toml"),
        r#"[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"
headers = ["include/sqlite3.h"]

[build.cargo]
features = ["ffi"]
target = "release"

[build.requirements]
cargo = ">= 1.70"

[build.platforms]
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }

[binaries]
"x86_64-pc-windows-msvc" = { url = "releases/download/v1.0.0/foo-win.tar.gz", sha256 = "abc123" }
"aarch64-apple-darwin" = { url = "https://example.com/foo-mac.tar.gz" }
"#,
    );
    let manifest = PackageManifest::load(dir).unwrap();
    let build = manifest.build.as_ref().expect("[build] 段应被解析");

    assert_eq!(build.parsed_strategy().unwrap(), BuildStrategy::Cargo);
    assert_eq!(build.headers, vec!["include/sqlite3.h"]);
    let cargo = build.cargo.as_ref().unwrap();
    assert_eq!(cargo.features, vec!["ffi"]);
    assert_eq!(cargo.target.as_deref(), Some("release"));
    assert_eq!(build.requirements.get("cargo").unwrap(), ">= 1.70");
    assert_eq!(
        build
            .platforms
            .get("x86_64-pc-windows-msvc")
            .unwrap()
            .cargo_features,
        vec!["win-ffi"]
    );

    assert_eq!(manifest.binaries.len(), 2);
    let win = manifest.binaries.get("x86_64-pc-windows-msvc").unwrap();
    assert_eq!(win.sha256.as_deref(), Some("abc123"));
    let mac = manifest.binaries.get("aarch64-apple-darwin").unwrap();
    assert_eq!(mac.sha256, None, "sha256 可缺省（该平台预编译路径不可用）");
}

#[test]
fn no_build_section_defaults_to_none() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        &tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"pure\"\nversion = \"0.1.0\"\n",
    );
    let manifest = PackageManifest::load(tmp.path()).unwrap();
    assert!(manifest.build.is_none());
    assert!(manifest.binaries.is_empty());
}

#[test]
fn unknown_strategy_is_hard_error() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        &tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n\n[build]\nstrategy = \"make\"\n",
    );
    let manifest = PackageManifest::load(tmp.path()).unwrap();
    let build = manifest.build.as_ref().unwrap();
    let err = build.parsed_strategy().unwrap_err();
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("none / cargo / cmake / custom")),
        "got: {err}"
    );
}

#[test]
fn current_triple_shape() {
    let triple = build::current_triple();
    let arch = std::env::consts::ARCH;
    assert!(triple.starts_with(arch), "got: {triple}");
    match std::env::consts::OS {
        "windows" => {
            assert!(
                triple.ends_with("-pc-windows-msvc") || triple.ends_with("-pc-windows-gnu"),
                "got: {triple}"
            );
        }
        "linux" => {
            assert!(
                triple.ends_with("-unknown-linux-gnu") || triple.ends_with("-unknown-linux-musl"),
                "got: {triple}"
            );
        }
        "macos" => assert!(triple.ends_with("-apple-darwin"), "got: {triple}"),
        _ => {}
    }
}

#[test]
fn tool_req_padding_and_matching() {
    // RFC 示例两段操作数 → 补齐三段
    let req = requirements::parse_tool_req(">= 1.70").unwrap();
    assert!(req.matches(&semver::Version::parse("1.88.0").unwrap()));
    assert!(!req.matches(&semver::Version::parse("1.69.0").unwrap()));

    let multi = requirements::parse_tool_req(">=3.20, <4.0").unwrap();
    assert!(multi.matches(&semver::Version::parse("3.29.6").unwrap()));
    assert!(!multi.matches(&semver::Version::parse("4.1.0").unwrap()));

    // 裸版本 = 精确
    let exact = requirements::parse_tool_req("3.29").unwrap();
    assert!(exact.matches(&semver::Version::parse("3.29.0").unwrap()));
    assert!(!exact.matches(&semver::Version::parse("3.30.0").unwrap()));
}

#[test]
fn requirements_check_reports_missing_tool_with_hint() {
    let mut reqs = BTreeMap::new();
    reqs.insert(
        "yx-definitely-not-a-real-tool".to_string(),
        ">= 1.0".to_string(),
    );
    let err = requirements::check(&reqs).unwrap_err();
    match err {
        PackageError::InvalidManifest(msg) => {
            assert!(msg.contains("构建依赖不满足"), "got: {msg}");
            assert!(msg.contains("不在 PATH"), "got: {msg}");
            assert!(msg.contains("Install:"), "应有安装指引: {msg}");
        }
        other => panic!("expected InvalidManifest, got {other}"),
    }
}

/// CI/dev 环境必有 cargo：真实核对走通（含两段要求补齐）
#[test]
fn requirements_check_cargo_satisfied() {
    let mut reqs = BTreeMap::new();
    reqs.insert("cargo".to_string(), ">= 1.70".to_string());
    requirements::check(&reqs).unwrap();
}

#[test]
fn requirements_check_reports_version_mismatch() {
    let mut reqs = BTreeMap::new();
    reqs.insert("cargo".to_string(), ">= 999.0".to_string());
    let err = requirements::check(&reqs).unwrap_err();
    match err {
        PackageError::InvalidManifest(msg) => {
            assert!(msg.contains("版本不满足"), "got: {msg}");
            assert!(msg.contains("https://rustup.rs"), "got: {msg}");
        }
        other => panic!("expected InvalidManifest, got {other}"),
    }
}

/// 无 [build] 无 [binaries] 的包零构建直过
#[test]
fn install_build_skips_plain_packages() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        &tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"pure\"\nversion = \"0.1.0\"\n",
    );
    let manifest = PackageManifest::load(tmp.path()).unwrap();
    let outcome = build::run_install_build(
        tmp.path(),
        &manifest,
        &tmp.path().join("scratch"),
        &TrustDecision::default(),
    )
    .unwrap();
    assert_eq!(outcome.via, "no-build");
    assert!(outcome.native_dir.is_none());
}

/// 未落地分支如实报错（随 5d/5e 提交逐一替换）
#[test]
fn pending_phases_report_clear_errors() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        &tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n\n[build]\nheaders = [\"a.h\"]\n",
    );
    let manifest = PackageManifest::load(tmp.path()).unwrap();
    let err = build::run_install_build(
        tmp.path(),
        &manifest,
        &tmp.path().join("scratch"),
        &TrustDecision::default(),
    )
    .unwrap_err();
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("yx-bindgen")),
        "got: {err}"
    );
}

/// cargo 策略端到端：真实编译一个最小 cdylib crate，产物落 build/native/<triple>/
#[test]
fn cargo_strategy_builds_real_crate_and_copies_artifacts() {
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

    let manifest = PackageManifest::load(&pkg).unwrap();
    let scratch = tmp.path().join(".yaoxiang").join("build");
    let outcome =
        build::run_install_build(&pkg, &manifest, &scratch, &TrustDecision::default()).unwrap();
    assert_eq!(outcome.via, "cargo");
    let native = outcome.native_dir.expect("应有产物目录");
    assert!(native.ends_with(format!("build/native/{}", build::current_triple()).as_str()));
    // 该平台对应扩展名的库文件已复制
    let has_lib = std::fs::read_dir(&native).unwrap().flatten().any(|e| {
        let n = e.file_name().to_string_lossy().to_string();
        n.ends_with(".dll") || n.ends_with(".so") || n.ends_with(".dylib")
    });
    assert!(has_lib, "cdylib 产物应复制进 {:?}", native);

    // scratch 不落包目录（target/ 隔离到项目 .yaoxiang/build/cargo/）
    assert!(!pkg.join("target").exists(), "cargo scratch 不得污染包目录");
    assert!(scratch.join("cargo").join("native-demo-1.0.0").exists());
}

/// 平台覆盖合并：当前平台的 cargo-features 追加进基础 features（去重）
#[test]
fn cargo_features_merge_platform_overrides() {
    use crate::package::manifest::{BuildConfig, CargoBuildConfig, PlatformOverrides};
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
    let merged = build::cargo::compose_features(&config);
    assert_eq!(
        merged,
        vec!["ffi".to_string(), "platform-extra".to_string()]
    );
}

#[test]
fn cargo_target_validation() {
    let tmp = tempfile::tempdir().unwrap();
    let pkg = tmp.path().join("p-0.1.0");
    write(
        &pkg.join("yaoxiang.toml"),
        "[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nstrategy = \"cargo\"\n\n[build.cargo]\ntarget = \"nightly\"\n",
    );
    let manifest = PackageManifest::load(&pkg).unwrap();
    let err = build::run_install_build(
        &pkg,
        &manifest,
        &tmp.path().join("scratch"),
        &TrustDecision::default(),
    )
    .unwrap_err();
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("release")),
        "got: {err}"
    );
}
