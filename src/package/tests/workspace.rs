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

// === 6b：合并解析（继承 / 成员引用 / 冲突检测）===

use crate::package::workspace::merged_dependencies;

/// 给成员 toml 追加依赖表
fn set_member_deps(
    root: &std::path::Path,
    key: &str,
    deps_toml: &str,
) {
    let path = root.join("packages").join(key).join("yaoxiang.toml");
    let mut body = fs::read_to_string(&path).unwrap();
    body.push('\n');
    body.push_str(deps_toml);
    fs::write(path, body).unwrap();
}

#[test]
fn test_merge_compatible_reqs_unify_to_intersection() {
    let (_tmp, root) = setup_workspace();
    set_member_deps(&root, "core", "[dependencies]\nregex = \">=1.0, <2.0\"\n");
    set_member_deps(&root, "utils", "[dependencies]\nregex = \"^1.5\"\n");

    let ws = load_workspace(&root).unwrap();
    let merged = merged_dependencies(&ws).unwrap();
    // 交集 [1.5, 2.0)
    assert_eq!(
        merged.fetch.get("regex").map(|v| v.to_string()),
        Some("\">=1.5.0, <2.0.0\"".to_string())
    );
    assert!(merged.member_refs.is_empty());
}

#[test]
fn test_merge_conflicting_reqs_error_names_members() {
    let (_tmp, root) = setup_workspace();
    set_member_deps(&root, "core", "[dependencies]\nregex = \"^1.0\"\n");
    set_member_deps(&root, "utils", "[dependencies]\nregex = \"^2.0\"\n");

    let ws = load_workspace(&root).unwrap();
    let err = merged_dependencies(&ws).unwrap_err().to_string();
    assert!(err.contains("版本冲突"), "{err}");
    assert!(err.contains("core") && err.contains("utils"), "{err}");
}

#[test]
fn test_merge_workspace_true_inherits_from_root() {
    let (_tmp, root) = setup_workspace();
    fs::write(
        root.join("yaoxiang.toml"),
        "[workspace.members]\ncore = \"packages/core/yaoxiang.toml\"\nutils = \"packages/utils/yaoxiang.toml\"\n\n[workspace.dependencies]\nregex = \"^1.0\"\n",
    )
    .unwrap();
    set_member_deps(
        &root,
        "core",
        "[dependencies]\nregex = { workspace = true }\n",
    );
    set_member_deps(
        &root,
        "utils",
        "[dependencies]\nregex = { workspace = true }\n",
    );

    let ws = load_workspace(&root).unwrap();
    let merged = merged_dependencies(&ws).unwrap();
    assert_eq!(
        merged.fetch.get("regex").map(|v| v.to_string()),
        Some("\">=1.0.0, <2.0.0\"".to_string())
    );
}

#[test]
fn test_merge_workspace_true_missing_in_root_errors() {
    let (_tmp, root) = setup_workspace();
    set_member_deps(
        &root,
        "core",
        "[dependencies]\nregex = { workspace = true }\n",
    );

    let ws = load_workspace(&root).unwrap();
    let err = merged_dependencies(&ws).unwrap_err().to_string();
    assert!(err.contains("未定义"), "{err}");
}

#[test]
fn test_merge_workspace_true_mixed_fields_error() {
    let (_tmp, root) = setup_workspace();
    fs::write(
        root.join("yaoxiang.toml"),
        "[workspace.members]\ncore = \"packages/core/yaoxiang.toml\"\nutils = \"packages/utils/yaoxiang.toml\"\n\n[workspace.dependencies]\nregex = \"^1.0\"\n",
    )
    .unwrap();
    set_member_deps(
        &root,
        "core",
        "[dependencies]\nregex = { workspace = true, version = \"^9\" }\n",
    );

    let ws = load_workspace(&root).unwrap();
    assert!(merged_dependencies(&ws).is_err());
}

#[test]
fn test_merge_member_ref_validated_and_recorded() {
    let (_tmp, root) = setup_workspace();
    set_member_deps(
        &root,
        "core",
        "[dependencies]\nutils = { workspace = \"utils\" }\n",
    );

    let ws = load_workspace(&root).unwrap();
    let merged = merged_dependencies(&ws).unwrap();
    assert_eq!(
        merged.member_refs.get("utils").map(|s| s.as_str()),
        Some("utils")
    );
    assert!(!merged.fetch.contains_key("utils"), "成员引用不进 fetch");
}

#[test]
fn test_merge_member_ref_unknown_key_errors() {
    let (_tmp, root) = setup_workspace();
    set_member_deps(
        &root,
        "core",
        "[dependencies]\nghost = { workspace = \"ghost\" }\n",
    );

    let ws = load_workspace(&root).unwrap();
    let err = merged_dependencies(&ws).unwrap_err().to_string();
    assert!(err.contains("ghost"), "{err}");
}

#[test]
fn test_merge_dev_deps_included() {
    let (_tmp, root) = setup_workspace();
    set_member_deps(&root, "core", "[dev-dependencies]\nregex = \"^1.0\"\n");
    set_member_deps(&root, "utils", "[dev-dependencies]\nregex = \"^1.2\"\n");

    let ws = load_workspace(&root).unwrap();
    let merged = merged_dependencies(&ws).unwrap();
    // dev 依赖同样并入共享 lock（测试可复现）
    assert!(merged.fetch.contains_key("regex"));
}

#[test]
fn test_merge_same_git_dep_intersects() {
    let (_tmp, root) = setup_workspace();
    let url = "https://example.com/u/repo";
    set_member_deps(
        &root,
        "core",
        &format!("[dependencies]\nrepo = {{ version = \">=1.0\", git = \"{url}\" }}\n"),
    );
    set_member_deps(
        &root,
        "utils",
        &format!("[dependencies]\nrepo = {{ version = \"^1.2\", git = \"{url}\" }}\n"),
    );

    let ws = load_workspace(&root).unwrap();
    let merged = merged_dependencies(&ws).unwrap();
    assert!(merged.fetch.contains_key("repo"));
}

#[test]
fn test_merge_different_git_urls_conflict() {
    let (_tmp, root) = setup_workspace();
    set_member_deps(
        &root,
        "core",
        "[dependencies]\nrepo = { version = \"*\", git = \"https://example.com/u/a\" }\n",
    );
    set_member_deps(
        &root,
        "utils",
        "[dependencies]\nrepo = { version = \"*\", git = \"https://example.com/u/b\" }\n",
    );

    let ws = load_workspace(&root).unwrap();
    let err = merged_dependencies(&ws).unwrap_err().to_string();
    assert!(err.contains("不同的 git 仓库"), "{err}");
}

// === 成员登记（workspace add / init 自动注册共用）===

use crate::package::workspace::register_member;

#[test]
fn test_register_member_stores_toml_file_path() {
    let (_tmp, root) = setup_workspace();
    fs::create_dir_all(root.join("packages/app/src")).unwrap();
    fs::write(
        root.join("packages/app/yaoxiang.toml"),
        "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();

    // 登记的是成员【目录】，存储的是【toml 文件】相对路径（014c 契约）
    let key = register_member(&root, &root.join("packages/app"), None).unwrap();
    assert_eq!(key, "app");

    let manifest = load_workspace_manifest(&root).unwrap();
    assert_eq!(
        manifest.workspace.members.get("app").map(|s| s.as_str()),
        Some("packages/app/yaoxiang.toml"),
        "登记值必须是 toml 文件路径而非目录（否则嵌套检查会误判）"
    );
    // 登记后 load_workspace 可完整加载
    let ws = load_workspace(&root).unwrap();
    assert_eq!(ws.members.len(), 3);
}

#[test]
fn test_register_member_key_override_and_conflicts() {
    let (_tmp, root) = setup_workspace();
    fs::create_dir_all(root.join("packages/app")).unwrap();
    fs::write(
        root.join("packages/app/yaoxiang.toml"),
        "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();

    // --as 覆盖 key
    let key = register_member(&root, &root.join("packages/app"), Some("core2")).unwrap();
    assert_eq!(key, "core2");

    // key 冲突
    fs::create_dir_all(root.join("packages/other")).unwrap();
    fs::write(
        root.join("packages/other/yaoxiang.toml"),
        "[package]\nname = \"other\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    assert!(register_member(&root, &root.join("packages/other"), Some("core2")).is_err());

    // 同路径重复登记
    assert!(register_member(&root, &root.join("packages/app"), None).is_err());
}

#[test]
fn test_register_member_rejects_outside_root_and_workspace_dir() {
    let (_tmp, root) = setup_workspace();

    // 工作空间根之外（独立目录树）
    let outside_tmp = TempDir::new().unwrap();
    let outside = outside_tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(
        outside.join("yaoxiang.toml"),
        "[package]\nname = \"o\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert!(register_member(&root, &outside, None).is_err());

    // 目录自身是 workspace（嵌套）
    let nested = root.join("packages/nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("yaoxiang.toml"),
        "[workspace.members]\nx = \"x/yaoxiang.toml\"\n",
    )
    .unwrap();
    assert!(matches!(
        register_member(&root, &nested, None),
        Err(PackageError::NestedWorkspace { .. })
    ));
}
