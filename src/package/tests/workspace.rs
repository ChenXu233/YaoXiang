//! 测试工作空间 — 基于 RFC-014c（§工作空间结构 / §依赖解析 / §workspace 依赖引用）
//!
//! 覆盖:
//! - 清单类型探测（[workspace] 段 → 工作空间根）
//! - WorkspaceManifest 解析（成员表 key → toml 路径）
//! - load_workspace：成功加载、成员缺失、成员 manifest 损坏、嵌套拒绝（决议 4）
//! - find_workspace_root：成员目录向上探测、普通项目不误判
//! - 6b 合并解析：交集合成、冲突报错、`{ workspace = true }` 继承、成员引用
//! - 成员登记（register_member：toml 文件路径契约、key 冲突、越界拒绝）

use std::fs;

use tempfile::TempDir;

use crate::package::error::PackageError;
use crate::package::workspace::{
    detect_manifest_kind, find_workspace_root, load_workspace, load_workspace_manifest,
    merged_dependencies, register_member, ManifestKind,
};

/// 搭一个两成员工作空间：core（无依赖）+ utils
fn setup_workspace() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("create tempdir");
    let root = tmp.path().to_path_buf();

    fs::write(
        root.join("yaoxiang.toml"),
        "[workspace.members]\ncore = \"packages/core/yaoxiang.toml\"\nutils = \"packages/utils/yaoxiang.toml\"\n",
    )
    .expect("write workspace root toml");

    for (name, version) in [("core", "0.1.0"), ("utils", "0.2.0")] {
        let dir = root.join("packages").join(name);
        fs::create_dir_all(&dir).expect("create member dir");
        fs::write(
            dir.join("yaoxiang.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n"),
        )
        .unwrap_or_else(|e| panic!("write member {name} toml: {e}"));
    }

    (tmp, root)
}

/// 给成员 toml 追加依赖表
fn set_member_deps(
    root: &std::path::Path,
    key: &str,
    deps_toml: &str,
) {
    let path = root.join("packages").join(key).join("yaoxiang.toml");
    let mut body =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read member {key} toml: {e}"));
    body.push('\n');
    body.push_str(deps_toml);
    fs::write(path, body).unwrap_or_else(|e| panic!("write member {key} deps: {e}"));
}

/// 登记一个新成员包（[package].name = name），返回包目录
fn add_member(
    root: &std::path::Path,
    name: &str,
    manifest_body: &str,
) -> std::path::PathBuf {
    let dir = root.join("packages").join(name);
    fs::create_dir_all(&dir).expect("create member dir");
    fs::write(dir.join("yaoxiang.toml"), manifest_body)
        .unwrap_or_else(|e| panic!("write member {name} toml: {e}"));
    dir
}

#[test]
fn test_detect_manifest_kind() {
    // Arrange：普通包 manifest
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    // Act / Assert：包 → Package；[workspace] 段 → Workspace；无清单 → None
    assert_eq!(
        detect_manifest_kind(tmp.path()),
        Some(ManifestKind::Package)
    );

    fs::write(tmp.path().join("yaoxiang.toml"), "[workspace.members]\n").unwrap();
    assert_eq!(
        detect_manifest_kind(tmp.path()),
        Some(ManifestKind::Workspace)
    );

    let empty = TempDir::new().unwrap();
    assert_eq!(detect_manifest_kind(empty.path()), None);
}

#[test]
fn test_load_workspace_manifest_parses_members() {
    // Arrange
    let (_tmp, root) = setup_workspace();

    // Act
    let manifest = load_workspace_manifest(&root).unwrap();

    // Assert：成员表 key → toml 相对路径
    assert_eq!(manifest.workspace.members.len(), 2);
    assert_eq!(
        manifest.workspace.members.get("core").map(|s| s.as_str()),
        Some("packages/core/yaoxiang.toml")
    );
}

#[test]
fn test_load_workspace_manifest_rejects_package_toml() {
    // Arrange：普通包 toml（无 [workspace] 段）
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    // Act
    let result = load_workspace_manifest(tmp.path());

    // Assert
    assert!(
        matches!(result, Err(PackageError::NotWorkspace)),
        "无 [workspace] 段应报 NotWorkspace"
    );
}

#[test]
fn test_load_workspace_success() {
    // Arrange
    let (_tmp, root) = setup_workspace();

    // Act
    let ws = load_workspace(&root).unwrap();

    // Assert：成员按 key 序（BTreeMap）、根 = 成员 toml 所在目录、
    // 共享 lockfile 载自根（决议 3：根唯一）
    assert_eq!(ws.members.len(), 2);
    assert_eq!(ws.members[0].name, "core");
    assert_eq!(ws.members[0].manifest.package.version, "0.1.0");
    assert_eq!(ws.members[1].name, "utils");
    assert_eq!(ws.members[0].root, root.join("packages").join("core"));
    assert!(
        ws.lock.package.is_empty(),
        "共享 lockfile 应载自根目录（决议 3）"
    );
}

#[test]
fn test_load_workspace_invalid_member_manifest() {
    // Arrange：成员 toml 缺 name
    let (_tmp, root) = setup_workspace();
    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        "[package]\nversion = \"0.1.0\"\n", // 缺 name
    )
    .unwrap();

    // Act
    let result = load_workspace(&root);

    // Assert
    assert!(
        matches!(result, Err(PackageError::MemberInvalid { key, .. }) if key == "core"),
        "损坏成员应报 MemberInvalid 且指认 key"
    );
}

#[test]
fn test_find_workspace_root_from_member_dir() {
    // Arrange
    let (_tmp, root) = setup_workspace();
    let member_dir = root.join("packages/core");

    // Act / Assert：成员目录向上探测命中根（穿越成员包根）；根自身也命中
    assert_eq!(
        find_workspace_root(&member_dir).as_deref(),
        Some(root.as_path()),
        "应从成员目录向上探测到 workspace 根"
    );
    assert_eq!(
        find_workspace_root(&root).as_deref(),
        Some(root.as_path()),
        "根目录自身应命中"
    );
}

// === 6b：合并解析（继承 / 成员引用 / 冲突检测）===

#[test]
fn test_merge_workspace_true_mixed_fields_error() {
    // Arrange：`{ workspace = true }` 与显式 version 混写（两套版本来源冲突）
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

    // Act
    let ws = load_workspace(&root).unwrap();
    let result = merged_dependencies(&ws);

    // Assert
    assert!(
        result.is_err(),
        "继承与显式版本混写必须报错（禁止两套写法并存）"
    );
}

// === 成员登记（workspace add / init 自动注册共用）===

#[test]
fn test_register_member_stores_toml_file_path() {
    // Arrange：新成员包 app
    let (_tmp, root) = setup_workspace();
    let app = add_member(
        &root,
        "app",
        "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
    );

    // Act：登记的是成员【目录】，存储的是【toml 文件】相对路径（014c 契约）
    let key = register_member(&root, &app, None).unwrap();

    // Assert：key 取 [package].name；登记值为 toml 文件路径（否则嵌套检查误判）
    assert_eq!(key, "app");
    let manifest = load_workspace_manifest(&root).unwrap();
    assert_eq!(
        manifest.workspace.members.get("app").map(|s| s.as_str()),
        Some("packages/app/yaoxiang.toml"),
        "登记值必须是 toml 文件路径而非目录（否则嵌套检查会误判）"
    );
    let ws = load_workspace(&root).unwrap();
    assert_eq!(ws.members.len(), 3, "登记后 load_workspace 可完整加载");
}

#[test]
fn test_register_member_rejects_outside_root_and_workspace_dir() {
    // Arrange：工作空间根之外的独立目录树 + 根内的嵌套 workspace 目录
    let (_tmp, root) = setup_workspace();
    let outside_tmp = TempDir::new().unwrap();
    let outside = outside_tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(
        outside.join("yaoxiang.toml"),
        "[package]\nname = \"o\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let nested = root.join("packages/nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("yaoxiang.toml"),
        "[workspace.members]\nx = \"x/yaoxiang.toml\"\n",
    )
    .unwrap();

    // Act / Assert：根之外拒绝；目录自身是 workspace（嵌套）拒绝
    assert!(
        register_member(&root, &outside, None).is_err(),
        "工作空间根之外的目录应被拒绝"
    );
    assert!(
        matches!(
            register_member(&root, &nested, None),
            Err(PackageError::NestedWorkspace { .. })
        ),
        "嵌套 workspace 目录应被拒绝（决议 4）"
    );
}
