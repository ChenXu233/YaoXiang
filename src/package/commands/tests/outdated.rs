//! 测试 `yaoxiang outdated` 命令
//!
//! 覆盖:
//! - path 依赖与未安装依赖跳过
//! - 默认分支依赖：有新 tag → Outdated
//! - 默认分支依赖：已是最新 → UpToDate
//! - tag 钉住依赖 → Pinned
//! - 无 tag 仓库 → Failed
//! - exec_in 冒烟（不报错）

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use crate::package::commands::init;
use crate::package::commands::outdated::{check_entries, exec_in};
use crate::package::dependency::DependencySpec;
use crate::package::lock::LockFile;

/// 在目录中创建带两个 tag 的本地 git 仓库
fn make_git_repo(dir: &Path) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let git = |args: &[&str]| {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(dir);
        // 测试 git 环境隔离：config 写路径与开发仓库彻底断开
        cmd.env("GIT_CONFIG_GLOBAL", dir.join("test-global-gitconfig"));
        cmd.env("GIT_CONFIG_SYSTEM", dir.join("test-system-gitconfig"));
        cmd.env("GIT_CONFIG_NOSYSTEM", "1");
        cmd.arg("-c")
            .arg("user.name=test")
            .arg("-c")
            .arg("user.email=t@t.local");
        cmd.args(args);
        let out = cmd.output().expect("git 可用");
        assert!(
            out.status.success(),
            "git {:?} 失败: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    };

    git(&["init", "-b", "main"]);
    git(&["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join("lib.yx"), "pub fn one() { 1 }").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-m", "one"]);
    git(&["tag", "v1.0.0"]);
    std::fs::write(dir.join("lib.yx"), "pub fn two() { 2 }").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-m", "two"]);
    git(&["tag", "v1.1.0"]);

    dir.to_path_buf()
}

fn spec(
    name: &str,
    version: &str,
    url: Option<&str>,
) -> DependencySpec {
    DependencySpec {
        name: name.to_string(),
        version: version.to_string(),
        git: url.map(|u| u.to_string()),
        path: None,
    }
}

fn setup_project() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    (tmp, project_dir)
}

#[test]
fn test_outdated_skips_path_and_uninstalled() {
    let specs = vec![
        spec("local", "1.0.0", None), // 无 git 字段 → path/registry 类，跳过
        spec("ghost", "1.0.0", Some("https://github.com/u/repo")),
    ];
    let mut lock = LockFile::new();
    // ghost 未登记 lock → 跳过
    lock.lock_dependency_full("local", "1.0.0", "path", None);

    let entries = check_entries(&specs, &lock).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn test_outdated_detects_newer_tag() {
    let tmp = TempDir::new().unwrap();
    let repo = make_git_repo(&tmp.path().join("repo"));

    let specs = vec![spec("fixture", ">=1.0.0", Some(repo.to_str().unwrap()))];
    let mut lock = LockFile::new();
    lock.lock_dependency_full("fixture", "1.0.0", "git", None);

    let entries = check_entries(&specs, &lock).unwrap();
    assert_eq!(entries.len(), 1);
    match &entries[0].status {
        crate::package::commands::outdated::OutdatedStatus::Outdated { latest } => {
            assert_eq!(latest, "1.1.0");
        }
        other => panic!("期望 Outdated，实际 {:?}", other),
    }
}

#[test]
fn test_outdated_up_to_date() {
    let tmp = TempDir::new().unwrap();
    let repo = make_git_repo(&tmp.path().join("repo"));

    let specs = vec![spec("fixture", "*", Some(repo.to_str().unwrap()))];
    let mut lock = LockFile::new();
    lock.lock_dependency_full("fixture", "1.1.0", "git", None);

    let entries = check_entries(&specs, &lock).unwrap();
    assert!(matches!(
        entries[0].status,
        crate::package::commands::outdated::OutdatedStatus::UpToDate
    ));
}

#[test]
fn test_outdated_tag_pinned() {
    let tmp = TempDir::new().unwrap();
    let repo = make_git_repo(&tmp.path().join("repo"));

    let url = format!("{}?tag=v1.0.0", repo.to_str().unwrap());
    let specs = vec![spec("fixture", "1.0.0", Some(&url))];
    let mut lock = LockFile::new();
    lock.lock_dependency_full("fixture", "1.0.0", "git", None);

    let entries = check_entries(&specs, &lock).unwrap();
    assert!(matches!(
        entries[0].status,
        crate::package::commands::outdated::OutdatedStatus::Pinned
    ));
}

#[test]
fn test_outdated_no_tags_fails() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("plain");
    std::fs::create_dir_all(&dir).unwrap();
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap()
    };
    assert!(git(&["init", "-b", "main"]).status.success());
    std::fs::write(dir.join("lib.yx"), "x").unwrap();
    assert!(git(&["add", "-A"]).status.success());
    assert!(Command::new("git")
        .arg("-C")
        .arg(&dir)
        .arg("-c")
        .arg("user.name=t")
        .arg("-c")
        .arg("user.email=t@t")
        .args(["commit", "-m", "x"])
        .output()
        .unwrap()
        .status
        .success());

    let specs = vec![spec("plain", "*", Some(dir.to_str().unwrap()))];
    let mut lock = LockFile::new();
    lock.lock_dependency_full("plain", "0.0.0", "git", None);

    let entries = check_entries(&specs, &lock).unwrap();
    assert!(matches!(
        entries[0].status,
        crate::package::commands::outdated::OutdatedStatus::Failed(_)
    ));
}

#[test]
fn test_outdated_exec_smoke() {
    let (_tmp, project_dir) = setup_project();
    exec_in(&project_dir).unwrap();
}
