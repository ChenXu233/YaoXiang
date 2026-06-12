//! 测试依赖冲突检测功能
//!
//! 覆盖:
//! - 无冲突的情况
//! - 同一包兼容版本要求
//! - 同一包不兼容版本要求
//! - `check_conflicts` 返回错误
//! - 通配符版本不冲突
//! - `ConflictInfo` 的 Display 输出

use crate::package::dependency::DependencySpec;
use crate::package::source::conflict::{check_conflicts, detect_conflicts, ConflictInfo, ConflictRequirement};

fn make_dep(
    name: &str,
    version: &str,
) -> DependencySpec {
    DependencySpec {
        name: name.to_string(),
        version: version.to_string(),
        git: None,
        path: None,
    }
}

#[test]
fn test_no_conflicts() {
    let deps = vec![make_dep("foo", "^1.0.0"), make_dep("bar", "^2.0.0")];
    let conflicts = detect_conflicts(&deps, &[]).unwrap();
    assert!(conflicts.is_empty());
}

#[test]
fn test_same_package_compatible() {
    let deps = vec![make_dep("foo", "^1.0.0")];
    let dev_deps = vec![make_dep("foo", "^1.5.0")];
    let conflicts = detect_conflicts(&deps, &dev_deps).unwrap();
    assert!(conflicts.is_empty());
}

#[test]
fn test_same_package_incompatible() {
    let deps = vec![make_dep("foo", "^1.0.0")];
    let dev_deps = vec![make_dep("foo", "^2.0.0")];
    let conflicts = detect_conflicts(&deps, &dev_deps).unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].package_name, "foo");
}

#[test]
fn test_check_conflicts_returns_error() {
    let deps = vec![make_dep("foo", "^1.0.0")];
    let dev_deps = vec![make_dep("foo", "^2.0.0")];
    let result = check_conflicts(&deps, &dev_deps);
    assert!(result.is_err());
}

#[test]
fn test_check_conflicts_ok() {
    let deps = vec![make_dep("foo", "^1.0.0")];
    let dev_deps = vec![];
    let result = check_conflicts(&deps, &dev_deps);
    assert!(result.is_ok());
}

#[test]
fn test_wildcard_no_conflict() {
    let deps = vec![make_dep("foo", "*")];
    let dev_deps = vec![make_dep("foo", "^1.0.0")];
    let conflicts = detect_conflicts(&deps, &dev_deps).unwrap();
    assert!(conflicts.is_empty());
}

#[test]
fn test_conflict_info_display() {
    let info = ConflictInfo {
        package_name: "foo".to_string(),
        requirements: vec![
            ConflictRequirement {
                from: "dependencies".to_string(),
                version_req: ">=1.0.0, <2.0.0".to_string(),
            },
            ConflictRequirement {
                from: "dev-dependencies".to_string(),
                version_req: ">=2.0.0, <3.0.0".to_string(),
            },
        ],
    };
    let display = info.to_string();
    assert!(display.contains("foo"));
    assert!(display.contains("版本冲突"));
}
