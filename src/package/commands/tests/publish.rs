//! `yaoxiang publish` 测试 — 基于 RFC-014a §publish 流程 + RFC-014c 6d
//!
//! 覆盖：发布前校验（description 必填，014a 校验清单 1）、裸 publish 报错
//! 指路（决议 1/2：官方 Registry 无限期后置）、6d workspace 引用在打包时
//! 物化（磁盘 manifest 不动）、`--no-test` 跳过发布前测试。
//!
//! 注：单测不触发真实 test runner——`run_test_command` 以 `current_exe()`
//! 为解释器，单测形态下那是 libtest harness（过滤器误匹配导致自递归）；
//! runner 路径的真实覆盖在 tests/integration/cli_e2e.rs（真二进制）。

use std::path::{Path, PathBuf};

use crate::package::commands::publish::{exec_in, PublishArgs};
use crate::package::error::PackageError;
use crate::package::yxpkg;

fn write(
    path: &Path,
    content: &str,
) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs for fixture");
    }
    std::fs::write(path, content)
        .unwrap_or_else(|e| panic!("write fixture {}: {e}", path.display()));
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
fn test_publish_dry_run_materializes_workspace_refs() {
    // Arrange：app 以 { workspace = "core" } 引用成员
    let tmp = tempfile::tempdir().unwrap();
    let app_dir = workspace_fixture(&tmp.path().join("ws"));

    // Act
    exec_in(
        &app_dir,
        PublishArgs {
            dry_run: true,
            github: false,
            no_test: true,
        },
    )
    .unwrap();

    // Assert：归档内 manifest 是发布形态（^0.2.0、无 workspace 残留），
    // 磁盘 manifest 保持工作空间形态（替换只在归档内物化）
    let artifact = app_dir.join("target/yxpkg/app-0.1.0.yxpkg");
    assert!(artifact.is_file(), "产物应写入 target/yxpkg/");
    let out = tmp.path().join("out");
    yxpkg::unpack(&artifact, &out).unwrap();
    let published = std::fs::read_to_string(out.join("yaoxiang.toml")).unwrap();
    assert!(published.contains("core = \"^0.2.0\""), "got: {published}");
    assert!(
        !published.contains("workspace"),
        "发布形态不得残留 workspace 引用: {published}"
    );
    let on_disk = std::fs::read_to_string(app_dir.join("yaoxiang.toml")).unwrap();
    assert!(
        on_disk.contains("{ workspace = \"core\" }"),
        "磁盘 manifest 不应被改写: {on_disk}"
    );
}

#[test]
fn test_publish_dry_run_packs_plain_project() {
    // Arrange：无 workspace 引用的单包
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);

    // Act
    exec_in(
        &project,
        PublishArgs {
            dry_run: true,
            github: false,
            no_test: true,
        },
    )
    .unwrap();

    // Assert：归档内 manifest 原样保留
    let artifact = project.join("target/yxpkg/demo-1.0.0.yxpkg");
    let out = tmp.path().join("out");
    yxpkg::unpack(&artifact, &out).unwrap();
    let published = std::fs::read_to_string(out.join("yaoxiang.toml")).unwrap();
    assert!(
        published.contains("name = \"demo\""),
        "无引用时 manifest 原样打包: {published}"
    );
}

#[test]
fn test_publish_requires_description() {
    // Arrange：缺 description（014a 发布前校验 1）
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    write(
        &project.join("yaoxiang.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
    );
    write(&project.join("src/lib.yx"), "pub x = 1\n");

    // Act
    let err = exec_in(
        &project,
        PublishArgs {
            dry_run: true,
            github: false,
            no_test: true,
        },
    )
    .unwrap_err();

    // Assert
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("description")),
        "got: {err}"
    );
}

#[test]
fn test_bare_publish_reports_registry_deferred() {
    // Arrange：无渠道裸 publish（决议 1/2）
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);

    // Act
    let err = exec_in(
        &project,
        PublishArgs {
            dry_run: false,
            github: false,
            no_test: false,
        },
    )
    .unwrap_err();

    // Assert：报 RegistryDeferred 且不产生打包产物
    assert!(matches!(err, PackageError::RegistryDeferred), "got: {err}");
    assert!(
        !project.join("target").exists(),
        "裸 publish 不应产生打包产物"
    );
}

#[test]
fn test_publish_no_test_skips_failing_tests() {
    // Arrange：测试必然失败 + --no-test
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);
    write(&project.join("tests/broken.yx"), "let ??? broken\n");

    // Act
    exec_in(
        &project,
        PublishArgs {
            dry_run: true,
            github: false,
            no_test: true,
        },
    )
    .unwrap();

    // Assert：跳过测试照常打包
    assert!(
        project.join("target/yxpkg/demo-1.0.0.yxpkg").is_file(),
        "--no-test 应跳过测试照常打包"
    );
}
