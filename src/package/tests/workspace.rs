//! 测试工作空间（RFC-014c Phase 6a）
//!
//! 覆盖:
//! - 清单类型探测（[workspace] 段 → 工作空间根）
//! - WorkspaceManifest 解析（成员表 key → toml 路径）
//! - load_workspace：成功加载、成员缺失、成员 manifest 损坏、嵌套拒绝
//! - find_workspace_root：成员目录向上探测、普通项目不误判
//! - 决议 3：根 lockfile 唯一（Workspace.lock 载自根）

use std::fs;

use tempfile::TempDir;

use crate::package::error::PackageError;
use crate::package::workspace::{
    detect_manifest_kind, find_workspace_root, load_workspace, load_workspace_manifest,
    ManifestKind,
};

/// 搭一个两成员工作空间：core（无依赖）+ utils
fn setup_workspace() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    fs::write(
        root.join("yaoxiang.toml"),
        "[workspace.members]\ncore = \"packages/core/yaoxiang.toml\"\nutils = \"packages/utils/yaoxiang.toml\"\n",
    )
    .unwrap();

    for (name, version) in [("core", "0.1.0"), ("utils", "0.2.0")] {
        let dir = root.join("packages").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("yaoxiang.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n"),
        )
        .unwrap();
    }

    (tmp, root)
}

#[test]
fn test_detect_manifest_kind() {
    let tmp = TempDir::new().unwrap();

    fs::write(
        tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert_eq!(
        detect_manifest_kind(tmp.path()),
        Some(ManifestKind::Package)
    );

    fs::write(tmp.path().join("yaoxiang.toml"), "[workspace.members]\n").unwrap();
    assert_eq!(
        detect_manifest_kind(tmp.path()),
        Some(ManifestKind::Workspace)
    );

    // 空目录无清单
    let empty = TempDir::new().unwrap();
    assert_eq!(detect_manifest_kind(empty.path()), None);
}

#[test]
fn test_load_workspace_manifest_parses_members() {
    let (_tmp, root) = setup_workspace();
    let manifest = load_workspace_manifest(&root).unwrap();
    assert_eq!(manifest.workspace.members.len(), 2);
    assert_eq!(
        manifest.workspace.members.get("core").map(|s| s.as_str()),
        Some("packages/core/yaoxiang.toml")
    );
}

#[test]
fn test_load_workspace_manifest_rejects_package_toml() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert!(matches!(
        load_workspace_manifest(tmp.path()),
        Err(PackageError::NotWorkspace)
    ));
}

#[test]
fn test_load_workspace_success() {
    let (_tmp, root) = setup_workspace();
    let ws = load_workspace(&root).unwrap();

    assert_eq!(ws.members.len(), 2);
    // 成员按 key 序（BTreeMap）
    assert_eq!(ws.members[0].name, "core");
    assert_eq!(ws.members[0].manifest.package.version, "0.1.0");
    assert_eq!(ws.members[1].name, "utils");
    // 成员根 = 成员 toml 所在目录
    assert_eq!(ws.members[0].root, root.join("packages").join("core"));
    // 共享 lockfile 载自根（决议 3：根唯一）
    assert!(ws.lock.package.is_empty());
}

#[test]
fn test_load_workspace_missing_member() {
    let (_tmp, root) = setup_workspace();
    fs::remove_dir_all(root.join("packages/utils")).unwrap();
    fs::write(
        root.join("yaoxiang.toml"),
        "[workspace.members]\nutils = \"packages/utils/yaoxiang.toml\"\n",
    )
    .unwrap();

    match load_workspace(&root) {
        Err(PackageError::MemberMissing { key, path }) => {
            assert_eq!(key, "utils");
            assert_eq!(path, "packages/utils/yaoxiang.toml");
        }
        other => panic!("expected MemberMissing, got {other:?}"),
    }
}

#[test]
fn test_load_workspace_invalid_member_manifest() {
    let (_tmp, root) = setup_workspace();
    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        "[package]\nversion = \"0.1.0\"\n", // 缺 name
    )
    .unwrap();

    assert!(matches!(
        load_workspace(&root),
        Err(PackageError::MemberInvalid { key, .. }) if key == "core"
    ));
}

#[test]
fn test_load_workspace_rejects_nested_workspace() {
    let (_tmp, root) = setup_workspace();
    // 成员自身含 [workspace] 段 → 决议 4 拒绝
    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\n\n[workspace.members]\ninner = \"x/yaoxiang.toml\"\n",
    )
    .unwrap();

    match load_workspace(&root) {
        Err(PackageError::NestedWorkspace { key, .. }) => assert_eq!(key, "core"),
        other => panic!("expected NestedWorkspace, got {other:?}"),
    }
}

#[test]
fn test_find_workspace_root_from_member_dir() {
    let (_tmp, root) = setup_workspace();
    let member_dir = root.join("packages/core");

    // 从成员目录向上探测命中 workspace 根（穿越成员包根）
    assert_eq!(
        find_workspace_root(&member_dir).as_deref(),
        Some(root.as_path())
    );
    // 从根自身命中
    assert_eq!(find_workspace_root(&root).as_deref(), Some(root.as_path()));
}

#[test]
fn test_find_workspace_root_none_for_plain_project() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    // 普通项目不是工作空间
    assert_eq!(find_workspace_root(tmp.path()), None);
}
