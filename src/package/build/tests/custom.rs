//! custom 构建策略与信任门测试 — 基于 RFC-014b §build.yx 构建脚本
//! + 2026-09-15 决议 1（信任门，不可省略的下限）
//!
//! 覆盖：非交互（CI）默认拒绝、`--trust` 放行即持久化、交互确认即持久化
//! （确认后不再询问）、拒绝不留记录、脚本失败即构建失败、缺 build.yx 报错。

use crate::package::build::{custom, TrustDecision};
use crate::package::error::PackageError;

use super::write;

fn custom_pkg(
    root: &std::path::Path,
    script: &str,
) -> std::path::PathBuf {
    let pkg = root.join("evil-corp-1.0.0");
    write(
        &pkg.join("yaoxiang.toml"),
        "[package]\nname = \"evil-corp\"\nversion = \"1.0.0\"\n\n[build]\nstrategy = \"custom\"\n",
    );
    write(&pkg.join("build.yx"), script);
    pkg
}

fn temp_store() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("create trust-store tempdir");
    let path = dir.path().join("config.toml");
    (dir, path)
}

fn trust(
    flag: bool,
    interactive: bool,
    store: &std::path::Path,
) -> TrustDecision {
    TrustDecision {
        flag,
        interactive,
        store: Some(store.to_path_buf()),
    }
}

fn stored_scripts(store: &std::path::Path) -> Vec<String> {
    crate::util::config::load_user_config_from(store)
        .expect("load trust store")
        .trust
        .build_scripts
}

#[test]
fn test_custom_non_interactive_denies_by_default() {
    // Arrange：CI 形态（非交互、无 --trust、无信任记录）
    let tmp = tempfile::tempdir().unwrap();
    let pkg = custom_pkg(tmp.path(), "x = 1\n");
    let (_dir, store) = temp_store();
    let trust = trust(false, false, &store);

    // Act
    let err = custom::build(&pkg, &trust, &|_| true).unwrap_err();

    // Assert：拒绝且不留信任记录
    assert!(
        matches!(err, PackageError::AuthFailed(ref m) if m.contains("非交互")),
        "got: {err}"
    );
    assert!(
        stored_scripts(&store).is_empty(),
        "拒绝路径不得留下信任记录"
    );
}

#[test]
fn test_custom_trust_flag_executes_and_persists() {
    // Arrange：--trust 显式放行（CI 同义）
    let tmp = tempfile::tempdir().unwrap();
    let pkg = custom_pkg(tmp.path(), "x = 1\n");
    let (_dir, store) = temp_store();
    let trust = trust(true, false, &store);

    // Act：confirm 恒 false——证明没走到交互路径
    let outcome = custom::build(&pkg, &trust, &|_| false).unwrap();

    // Assert：放行且按 RFC「--trust 把信任记录持久化」落记录
    assert_eq!(outcome.via, "custom");
    assert_eq!(stored_scripts(&store), vec!["evil-corp@1.0.0".to_string()]);
}

#[test]
fn test_custom_interactive_confirm_persists_and_repasses() {
    // Arrange：交互环境首次执行
    let tmp = tempfile::tempdir().unwrap();
    let pkg = custom_pkg(tmp.path(), "x = 1\n");
    let (_dir, store) = temp_store();
    let trust = trust(false, true, &store);
    let asked = std::cell::Cell::new(0);

    // Act：首次——确认文案应含 name@version
    let outcome = custom::build(&pkg, &trust, &|q| {
        assert!(q.contains("evil-corp@1.0.0"), "确认文案应含包名版本: {q}");
        asked.set(asked.get() + 1);
        true
    })
    .unwrap();

    // Assert：询问一次、落记录；第二次已信任不再询问
    assert_eq!(outcome.via, "custom");
    assert_eq!(asked.get(), 1, "应询问一次");
    assert_eq!(stored_scripts(&store), vec!["evil-corp@1.0.0".to_string()]);
    let outcome = custom::build(&pkg, &trust, &|_| panic!("已信任的包不应再次询问")).unwrap();
    assert_eq!(outcome.via, "custom");
}

#[test]
fn test_custom_interactive_reject_does_not_persist() {
    // Arrange：交互确认但用户拒绝
    let tmp = tempfile::tempdir().unwrap();
    let pkg = custom_pkg(tmp.path(), "x = 1\n");
    let (_dir, store) = temp_store();
    let trust = trust(false, true, &store);

    // Act
    let err = custom::build(&pkg, &trust, &|_| false).unwrap_err();

    // Assert
    assert!(
        matches!(err, PackageError::AuthFailed(ref m) if m.contains("拒绝")),
        "got: {err}"
    );
    assert!(stored_scripts(&store).is_empty(), "拒绝不得落信任记录");
}

#[test]
fn test_custom_script_failure_is_build_failure() {
    // Arrange：语法非法的 build.yx
    let tmp = tempfile::tempdir().unwrap();
    let pkg = custom_pkg(tmp.path(), "let ??? broken\n");
    let (_dir, store) = temp_store();
    let trust = trust(true, false, &store);

    // Act
    let err = custom::build(&pkg, &trust, &|_| false).unwrap_err();

    // Assert：脚本失败 = 构建失败（RFC 执行模型：非 0 退出/编译错误即中止）
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("build.yx 执行失败")),
        "got: {err}"
    );
}

#[test]
fn test_custom_requires_build_script_present() {
    // Arrange：custom 策略但包根无 build.yx
    let tmp = tempfile::tempdir().unwrap();
    let pkg = tmp.path().join("evil-corp-1.0.0");
    write(
        &pkg.join("yaoxiang.toml"),
        "[package]\nname = \"evil-corp\"\nversion = \"1.0.0\"\n\n[build]\nstrategy = \"custom\"\n",
    );
    let (_dir, store) = temp_store();
    let trust = trust(true, false, &store);

    // Act
    let err = custom::build(&pkg, &trust, &|_| false).unwrap_err();

    // Assert
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("build.yx")),
        "got: {err}"
    );
}
