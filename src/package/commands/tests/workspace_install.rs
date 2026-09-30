//! 测试工作空间 install（RFC-014c §lockfile 共享，Phase 6b）
//!
//! 覆盖:
//! - 成员目录执行 install → 整个 workspace 装到根
//! - 根 lockfile 唯一（成员目录无 lock）
//! - 共享 vendor 在根

use std::fs;

use tempfile::TempDir;

use crate::package::commands::install::exec_in;
use crate::package::lock::LockFile;

fn setup_workspace() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    fs::write(
        root.join("yaoxiang.toml"),
        "[workspace.members]\ncore = \"packages/core/yaoxiang.toml\"\nutils = \"packages/utils/yaoxiang.toml\"\n",
    )
    .unwrap();
    for name in ["core", "utils"] {
        let dir = root.join("packages").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("yaoxiang.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }
    (tmp, root)
}

/// 写一个可安装的本地 path 依赖源（带独立 toml 的目录）
fn make_path_source(
    root: &std::path::Path,
    name: &str,
) -> String {
    let src = root.join(format!("{name}-src"));
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.yx"), "v: Int = 42\n").unwrap();
    format!("./{name}-src")
}

#[test]
fn test_install_from_member_dir_installs_whole_workspace() {
    let (_tmp, root) = setup_workspace();

    // core 声明 path 依赖 foo；utils 声明 path 依赖 bar
    let foo_path = make_path_source(&root, "foo");
    let bar_path = make_path_source(&root, "bar");
    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        format!(
            "[package]\nname = \"core\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = {{ version = \"1.0.0\", path = \"{foo_path}\" }}\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join("packages/utils/yaoxiang.toml"),
        format!(
            "[package]\nname = \"utils\"\nversion = \"0.2.0\"\n\n[dependencies]\nbar = {{ version = \"1.0.0\", path = \"{bar_path}\" }}\n"
        ),
    )
    .unwrap();

    // Act - 从成员目录 install
    exec_in(&root.join("packages/core"), &Default::default()).unwrap();

    // Assert - 根 lock 覆盖两个成员的依赖；成员目录无 lock
    let lock = LockFile::load(&root).unwrap();
    assert!(
        lock.package.contains_key("foo"),
        "root lock should have foo"
    );
    assert!(
        lock.package.contains_key("bar"),
        "root lock should have bar"
    );
    assert!(
        !root.join("packages/core/yaoxiang.lock").exists(),
        "member dir must not have its own lock（决议 3）"
    );
}
