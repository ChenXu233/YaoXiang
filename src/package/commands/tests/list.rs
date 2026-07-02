//! 测试 `yaoxiang list` 命令
//!
//! 覆盖:
//! - 无依赖时列出不报错
//! - 有依赖时列出不报错
//! - `format_extra` 辅助函数（空/git/path）

use crate::package::commands::add;
use crate::package::commands::init;
use crate::package::commands::list::{exec_in, format_extra};
use crate::package::dependency::DependencySpec;
use tempfile::TempDir;

fn setup_project() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    (tmp, project_dir)
}

#[test]
fn test_list_empty() {
    let (_tmp, project_dir) = setup_project();
    exec_in(&project_dir).unwrap(); // Should not error
}

#[test]
fn test_list_with_deps() {
    let (_tmp, project_dir) = setup_project();
    add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
    add::exec_in(&project_dir, "bar", Some("2.0.0"), true).unwrap();
    exec_in(&project_dir).unwrap(); // Should not error
}

#[test]
fn test_format_extra_empty() {
    let spec = DependencySpec {
        name: "foo".to_string(),
        version: "1.0.0".to_string(),
        git: None,
        path: None,
    };
    assert_eq!(format_extra(&spec), "");
}

#[test]
fn test_format_extra_with_git() {
    let spec = DependencySpec {
        name: "foo".to_string(),
        version: "1.0.0".to_string(),
        git: Some("https://github.com/example/foo".to_string()),
        path: None,
    };
    let extra = format_extra(&spec);
    assert!(extra.contains("git:"));
}
