//! vendor/lock 一致性与 lock 优先解析测试 — RFC-014 §项目模式（2026-09-15 决议）
//!
//! 覆盖:
//! - 一致性三查：lock 缺条目 / vendor 缺目录 / 版本要求不满足
//! - path 依赖与 std 接口目录跳过；无 vendor（未 install）不判死
//! - resolve_in_vendor_with_lock：lock 优先 / lock 无条目回退最高版本 /
//!   lock 指向缺失版本时解析失败（留给一致性检查报错）
//! - ensure 级语义经由 consistency::check_vendor_lock_consistency 纯函数验证

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::frontend::module::consistency::{check_vendor_lock_consistency, lock_versions};
use super::resolve_in_vendor_with_lock;

/// 项目骨架：yaoxiang.toml + 可选依赖表（TOML 正文直接内联）
fn setup_project(extra: &str) -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    fs::write(
        root.join("yaoxiang.toml"),
        format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n{extra}"),
    )
    .unwrap();
    (tmp, root)
}

fn write_vendor_pkg(
    root: &Path,
    name: &str,
    version: &str,
) {
    let dir = root
        .join(".yaoxiang")
        .join("vendor")
        .join(format!("{name}-{version}"));
    fs::create_dir_all(dir.join("src").join(name)).unwrap();
    fs::write(
        dir.join("src").join(name).join("mod.yx"),
        "f = () => 1
",
    )
    .unwrap();
}

fn write_lock(
    root: &Path,
    entries: &[(&str, &str)],
) {
    let mut body = String::from("version = 1\n");
    for (name, version) in entries {
        body.push_str(&format!(
            "\n[package.{name}]\nversion = \"{version}\"\nsource = \"git\"\n"
        ));
    }
    fs::write(root.join("yaoxiang.lock"), body).unwrap();
}

// === 一致性检查 ===

#[test]
fn test_consistent_project_passes() {
    let (_tmp, root) = setup_project("[dependencies]\nfoo = \"^1.0\"\n");
    write_vendor_pkg(&root, "foo", "1.2.0");
    write_lock(&root, &[("foo", "1.2.0")]);

    let report = check_vendor_lock_consistency(&root).unwrap();
    assert!(report.is_consistent(), "{:?}", report.describe());
}

#[test]
fn test_missing_in_lock_detected() {
    let (_tmp, root) = setup_project("[dependencies]\nfoo = \"^1.0\"\n");
    write_vendor_pkg(&root, "foo", "1.2.0");
    write_lock(&root, &[]); // lock 空条目

    let report = check_vendor_lock_consistency(&root).unwrap();
    assert_eq!(report.missing_in_lock, vec!["foo".to_string()]);
    assert!(!report.is_consistent());
}

#[test]
fn test_missing_in_vendor_detected() {
    let (_tmp, root) = setup_project("[dependencies]\nfoo = \"^1.0\"\n");
    // vendor 目录存在（vendor 模式），但 foo-1.2.0 未解包
    fs::create_dir_all(root.join(".yaoxiang").join("vendor")).unwrap();
    // lock 锁了 1.2.0 但 vendor 没解包
    write_lock(&root, &[("foo", "1.2.0")]);

    let report = check_vendor_lock_consistency(&root).unwrap();
    assert_eq!(
        report.missing_in_vendor,
        vec![("foo".to_string(), "1.2.0".to_string())]
    );
}

#[test]
fn test_requirement_mismatch_detected() {
    let (_tmp, root) = setup_project("[dependencies]\nfoo = \"^2.0\"\n");
    write_vendor_pkg(&root, "foo", "1.2.0");
    write_lock(&root, &[("foo", "1.2.0")]); // lock 版本不满足 ^2.0

    let report = check_vendor_lock_consistency(&root).unwrap();
    assert_eq!(
        report.requirement_mismatch,
        vec![("foo".to_string(), "^2.0".to_string(), "1.2.0".to_string())]
    );
}

#[test]
fn test_path_deps_skipped() {
    let (_tmp, root) =
        setup_project("[dependencies]\nlocal = { version = \"1.0.0\", path = \"./local-src\" }\n");
    write_lock(&root, &[]); // path 依赖不进 lock 也不判缺

    let report = check_vendor_lock_consistency(&root).unwrap();
    assert!(report.is_consistent(), "{:?}", report.describe());
}

#[test]
fn test_no_vendor_dir_is_consistent() {
    // 未 install 的项目（无 vendor）：不判死，保持零配置可用
    let (_tmp, root) = setup_project("[dependencies]\nfoo = \"^1.0\"\n");

    let report = check_vendor_lock_consistency(&root).unwrap();
    assert!(report.is_consistent());
}

// === lock 优先解析 ===

#[test]
fn test_resolve_lock_precise_selection() {
    let tmp = TempDir::new().unwrap();
    write_vendor_pkg(tmp.path(), "foo", "1.0.0");
    write_vendor_pkg(tmp.path(), "foo", "2.0.0");

    // lock 锁 1.0.0：不取最高版本
    let resolved = resolve_in_vendor_with_lock("foo", tmp.path(), Some("1.0.0")).unwrap();
    assert!(
        resolved.to_string_lossy().contains("foo-1.0.0"),
        "unexpected: {:?}",
        resolved
    );
}

#[test]
fn test_resolve_falls_back_to_highest_without_lock() {
    let tmp = TempDir::new().unwrap();
    write_vendor_pkg(tmp.path(), "foo", "1.0.0");
    write_vendor_pkg(tmp.path(), "foo", "2.0.0");

    let resolved = resolve_in_vendor_with_lock("foo", tmp.path(), None).unwrap();
    assert!(resolved.to_string_lossy().contains("foo-2.0.0"));
}

#[test]
fn test_resolve_fails_when_locked_version_absent() {
    let tmp = TempDir::new().unwrap();
    write_vendor_pkg(tmp.path(), "foo", "1.0.0");

    // lock 指向缺失版本：解析失败（一致性检查会先报出 missing_in_vendor）
    assert!(resolve_in_vendor_with_lock("foo", tmp.path(), Some("9.9.9")).is_none());
}

#[test]
fn test_lock_versions_bulk_read() {
    let (_tmp, root) = setup_project("");
    write_lock(&root, &[("foo", "1.2.0"), ("bar", "0.3.0")]);

    let versions = lock_versions(&root).unwrap();
    assert_eq!(versions.get("foo").map(|s| s.as_str()), Some("1.2.0"));
    assert_eq!(versions.get("bar").map(|s| s.as_str()), Some("0.3.0"));
    assert!(!versions.contains_key("ghost"));
}

// === 端到端：E5001 install 提示与一致性预检 ===

#[test]
fn test_e5001_install_hint_when_pkg_missing_from_vendor() {
    // Arrange - vendor 模式项目，use 一个 lock/vendor 中不存在的包
    let (_tmp, root) = setup_project("[dependencies]\nother = \"^1.0\"\n");
    // vendor/lock 一致（一致性预检放行）；use 的 data 包不在 vendor 中
    write_vendor_pkg(&root, "other", "1.0.0");
    write_lock(&root, &[("other", "1.0.0")]);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src").join("main.yx"),
        "use data.json;\n\nmain = { 0 }\n",
    )
    .unwrap();

    // Act
    let result = super::super::check_project(&root.join("src").join("main.yx"));

    // Assert - E5001 的 help 提到 yaoxiang install
    match result {
        Ok(files) => {
            let joined: String = files
                .iter()
                .flat_map(|(_, diags)| diags.iter())
                .map(|d| format!("{}\n{}", d.message, d.help))
                .collect();
            assert!(
                joined.contains("yaoxiang install"),
                "expected install hint, got: {joined}"
            );
        }
        Err(e) => panic!("expected E5001 diagnostics, got error: {e}"),
    }
}

#[test]
fn test_vendor_lock_inconsistent_blocks_check() {
    // Arrange - lock 缺条目（manifest 声明了 foo 但 lock 空）
    let (_tmp, root) = setup_project("[dependencies]\nfoo = \"^1.0\"\n");
    fs::create_dir_all(root.join(".yaoxiang").join("vendor")).unwrap();
    write_lock(&root, &[]);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.yx"), "main = { 0 }\n").unwrap();

    // Act
    let result = super::super::check_project(&root.join("src").join("main.yx"));

    // Assert - 前置一致性错误（不进 typecheck）
    let Err(err) = result else {
        panic!("expected VendorLockInconsistent error")
    };
    let msg = err.to_string();
    assert!(msg.contains("yaoxiang.lock"), "unexpected: {msg}");
    assert!(msg.contains("yaoxiang install"), "unexpected: {msg}");
}

#[test]
fn test_consistent_vendor_project_checks_clean() {
    // Arrange - vendor/lock 一致，use 已安装依赖
    let (_tmp, root) = setup_project("[dependencies]\nfoo = \"^1.0\"\n");
    write_vendor_pkg(&root, "foo", "1.2.0");
    write_lock(&root, &[("foo", "1.2.0")]);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src").join("main.yx"),
        "use foo;\n\nmain = { foo.f() }\n",
    )
    .unwrap();

    // Act
    let result = super::super::check_project(&root.join("src").join("main.yx"));

    // Assert - 无 E5001、无一一致性错误
    match result {
        Ok(files) => {
            let errs: Vec<String> = files
                .iter()
                .flat_map(|(_, diags)| diags.iter())
                .filter(|d| d.message.contains("E5001") || d.message.contains("未找到"))
                .map(|d| d.message.clone())
                .collect();
            assert!(errs.is_empty(), "unexpected E5001: {errs:?}");
        }
        Err(e) => panic!("expected clean check, got error: {e}"),
    }
}
