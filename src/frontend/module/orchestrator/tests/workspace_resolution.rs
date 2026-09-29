//! 工作空间成员解析测试 — RFC-014c 6c（成员引用 / 严格可见性 / path 依赖）
//!
//! 严格可见性（pnpm 纪律）：成员的 use 只解析自己声明的依赖——共享 vendor
//! 里别人声明的包不可见；成员引用解析到被引用成员包根；path 依赖按路径解析。

use std::fs;

use tempfile::TempDir;

use super::super::check_project;

/// 两成员工作空间：core + utils（utils 的 src 提供 pub 值）
fn setup_ws() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    fs::write(
        root.join("yaoxiang.toml"),
        "[workspace.members]\ncore = \"packages/core/yaoxiang.toml\"\nutils = \"packages/utils/yaoxiang.toml\"\n",
    )
    .unwrap();
    for (name, version) in [("core", "0.1.0"), ("utils", "0.2.0")] {
        let dir = root.join("packages").join(name);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("yaoxiang.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n"),
        )
        .unwrap();
    }
    fs::write(root.join("packages/utils/src/lib.yx"), "two: Int = 2\n").unwrap();
    (tmp, root)
}

/// 手放一个 vendor 条目（发现只读文件，无需真实 git 下载）
fn plant_vendor(
    root: &std::path::Path,
    name: &str,
    version: &str,
) {
    let dir = root
        .join(".yaoxiang")
        .join("vendor")
        .join(format!("{name}-{version}"));
    fs::create_dir_all(dir.join("src").join(name)).unwrap();
    fs::write(dir.join("src").join(name).join("mod.yx"), "v: Int = 42\n").unwrap();
    fs::write(
        dir.join("yaoxiang.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n"),
    )
    .unwrap();
    fs::create_dir_all(root.join(".yaoxiang").join("vendor")).unwrap();
}

fn write_lock(
    root: &std::path::Path,
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

fn entry_src(
    root: &std::path::Path,
    member: &str,
    code: &str,
) {
    fs::write(
        root.join("packages")
            .join(member)
            .join("src")
            .join("main.yx"),
        code,
    )
    .unwrap();
}

fn has_e5001(files: &[(std::path::PathBuf, Vec<crate::util::diagnostic::Diagnostic>)]) -> bool {
    files
        .iter()
        .flat_map(|(_, d)| d.iter())
        .any(|d| d.code == "E5001")
}

#[test]
fn test_member_ref_resolves_to_member_root() {
    let (_tmp, root) = setup_ws();
    // core 引用 utils（key 引用，成员 [package].name = "utils"）
    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\n\n[dependencies]\nutils = { workspace = \"utils\" }\n",
    )
    .unwrap();
    entry_src(
        &root,
        "core",
        "use utils;\n\nmain = () => { print(utils.two) }\n",
    );

    let files = check_project(&root.join("packages/core/src/main.yx")).unwrap();
    assert!(!has_e5001(&files), "成员引用应解析到 utils 包根");
}

#[test]
fn test_strict_visibility_undeclared_vendor_pkg_not_resolvable() {
    let (_tmp, root) = setup_ws();
    plant_vendor(&root, "regex", "1.5.0");
    write_lock(&root, &[("regex", "1.5.0")]);

    // core 未声明 regex → 即使躺在共享 vendor 里也不可见（严格可见性）
    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    entry_src(&root, "core", "use regex;\n\nmain = () => { 0 }\n");
    let files = check_project(&root.join("packages/core/src/main.yx")).unwrap();
    assert!(
        has_e5001(&files),
        "未声明的 vendor 包必须不可见（防幽灵依赖）"
    );

    // 声明后即可解析
    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\n\n[dependencies]\nregex = \"^1.0\"\n",
    )
    .unwrap();
    let files = check_project(&root.join("packages/core/src/main.yx")).unwrap();
    assert!(!has_e5001(&files), "声明后应从共享 vendor 解析");
}

#[test]
fn test_other_members_vendor_deps_not_visible_by_default() {
    // utils 声明 regex；core 没声明——utils 的依赖对 core 不可见（幽灵依赖防线）
    let (_tmp, root) = setup_ws();
    plant_vendor(&root, "regex", "1.5.0");
    write_lock(&root, &[("regex", "1.5.0")]);

    fs::write(
        root.join("packages/utils/yaoxiang.toml"),
        "[package]\nname = \"utils\"\nversion = \"0.2.0\"\n\n[dependencies]\nregex = \"^1.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    entry_src(&root, "core", "use regex;\n\nmain = () => { 0 }\n");

    let files = check_project(&root.join("packages/core/src/main.yx")).unwrap();
    assert!(has_e5001(&files), "其他成员的依赖对本成员不可见");
}

#[test]
fn test_path_dep_resolves_by_path() {
    let (_tmp, root) = setup_ws();
    // path 依赖在成员外（相对成员根展开）
    let dep_root = root.join("libs/bar");
    fs::create_dir_all(dep_root.join("src")).unwrap();
    fs::write(dep_root.join("src/lib.yx"), "one: Int = 1\n").unwrap();
    fs::write(
        dep_root.join("yaoxiang.toml"),
        "[package]\nname = \"bar\"\nversion = \"0.3.0\"\n",
    )
    .unwrap();

    fs::write(
        root.join("packages/core/yaoxiang.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\n\n[dependencies]\nbar = { version = \"0.3.0\", path = \"../../libs/bar\" }\n",
    )
    .unwrap();
    entry_src(
        &root,
        "core",
        "use bar;\n\nmain = () => { print(bar.one) }\n",
    );

    let files = check_project(&root.join("packages/core/src/main.yx")).unwrap();
    assert!(
        !has_e5001(&files),
        "path 依赖应按路径解析（总纲：本地模块延伸）"
    );
}
