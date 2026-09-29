//! `yaoxiang publish` 测试（RFC-014a；6d workspace 引用替换）

use std::path::{Path, PathBuf};

use crate::package::commands::publish::{exec_in, PublishArgs};
use crate::package::error::PackageError;
use crate::package::yxpkg;

fn write(
    path: &Path,
    content: &str,
) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

/// 单包项目：含 description（发布必填）
fn sample_project(root: &Path) {
    write(
        &root.join("yaoxiang.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.0.0\"\ndescription = \"demo pkg\"\n",
    );
    write(&root.join("src/lib.yx"), "pub x = 1\n");
}

/// 工作空间 fixture：core 0.2.0 + app 0.1.0（以 workspace 引用 core）
fn workspace_fixture(root: &Path) -> PathBuf {
    write(
        &root.join("yaoxiang.toml"),
        "[workspace.members]\ncore = \"core/yaoxiang.toml\"\napp = \"app/yaoxiang.toml\"\n",
    );
    write(
        &root.join("core/yaoxiang.toml"),
        "[package]\nname = \"core\"\nversion = \"0.2.0\"\ndescription = \"core\"\n",
    );
    write(&root.join("core/src/lib.yx"), "pub c = 1\n");
    write(
        &root.join("app/yaoxiang.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\ndescription = \"app\"\n\n[dependencies]\ncore = { workspace = \"core\" }\n",
    );
    write(&root.join("app/src/lib.yx"), "use core;\npub a = core.c\n");
    root.join("app")
}

#[test]
fn dry_run_packs_with_workspace_refs_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    let app_dir = workspace_fixture(&tmp.path().join("ws"));

    exec_in(
        &app_dir,
        PublishArgs {
            dry_run: true,
            github: false,
        },
    )
    .unwrap();

    let artifact = app_dir.join("target/yxpkg/app-0.1.0.yxpkg");
    assert!(artifact.is_file(), "产物应写入 target/yxpkg/");

    // 归档内 manifest 应是发布形态：workspace 引用 → ^0.2.0
    let out = tmp.path().join("out");
    yxpkg::unpack(&artifact, &out).unwrap();
    let published = std::fs::read_to_string(out.join("yaoxiang.toml")).unwrap();
    assert!(published.contains("core = \"^0.2.0\""), "got: {published}");
    assert!(
        !published.contains("workspace"),
        "发布形态不得残留 workspace 引用: {published}"
    );

    // 磁盘 manifest 保持工作空间形态（替换只在归档内物化）
    let on_disk = std::fs::read_to_string(app_dir.join("yaoxiang.toml")).unwrap();
    assert!(
        on_disk.contains("{ workspace = \"core\" }"),
        "磁盘 manifest 不应被改写"
    );
}

#[test]
fn dry_run_packs_plain_project_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);

    exec_in(
        &project,
        PublishArgs {
            dry_run: true,
            github: false,
        },
    )
    .unwrap();
    let artifact = project.join("target/yxpkg/demo-1.0.0.yxpkg");
    let out = tmp.path().join("out");
    yxpkg::unpack(&artifact, &out).unwrap();
    let published = std::fs::read_to_string(out.join("yaoxiang.toml")).unwrap();
    assert!(published.contains("name = \"demo\""));
}

#[test]
fn description_is_required() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write(
        &project.join("yaoxiang.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
    );
    write(&project.join("src/lib.yx"), "pub x = 1\n");

    let err = exec_in(
        &project,
        PublishArgs {
            dry_run: true,
            github: false,
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("description")),
        "got: {err}"
    );
}

#[test]
fn bare_publish_reports_registry_deferred() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);

    let err = exec_in(
        &project,
        PublishArgs {
            dry_run: false,
            github: false,
        },
    )
    .unwrap_err();
    assert!(matches!(err, PackageError::RegistryDeferred), "got: {err}");
    assert!(
        !project.join("target").exists(),
        "裸 publish 不应产生打包产物"
    );
}
