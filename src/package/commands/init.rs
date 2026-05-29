//! `yaoxiang init` command - Initialize a new YaoXiang project

use std::fs;
use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::template::{generate_gitignore, generate_main_yx};
use crate::util::i18n::{t, current_lang, MSG};

/// Initialize a new YaoXiang project at the given base directory
///
/// Creates the following structure:
/// ```text
/// <base>/<name>/
/// ├── yaoxiang.toml
/// ├── yaoxiang.lock
/// ├── .gitignore
/// ├── .yaoxiang/
/// │   └── std/           ← 标准库接口文件（LSP 跳转用）
/// │       ├── io.yx
/// │       ├── list.yx
/// │       ├── math.yx
/// │       └── ...
/// └── src/
///     └── main.yx
/// ```
pub fn exec_in(
    base: &Path,
    name: &str,
) -> PackageResult<()> {
    let project_dir = base.join(name);

    // Check if directory already exists
    if project_dir.exists() {
        return Err(PackageError::ProjectExists(project_dir.clone()));
    }

    // Create project directory structure
    fs::create_dir_all(project_dir.join("src"))?;

    // Generate yaoxiang.toml
    let manifest = PackageManifest::new(name);
    manifest.save(&project_dir)?;

    // Generate yaoxiang.lock
    let lock = LockFile::new();
    lock.save(&project_dir)?;

    // Generate src/main.yx
    let main_content = generate_main_yx(name);
    fs::write(project_dir.join("src").join("main.yx"), main_content)?;

    // Generate .gitignore
    let gitignore_content = generate_gitignore();
    fs::write(project_dir.join(".gitignore"), gitignore_content)?;

    // Generate standard library interface files for LSP
    let std_dir = project_dir.join(".yaoxiang").join("std");
    if let Err(e) = crate::std::gen_interfaces::write_interfaces_to_dir(&std_dir) {
        // 接口文件生成失败不应阻止项目创建，仅输出警告
        eprintln!("Warning: failed to generate std interface files: {}", e);
    }

    let lang = current_lang();
    println!(
        "{}",
        t(MSG::PackageProjectCreated, lang, Some(&[&name.to_string()]))
    );
    println!("  {}/yaoxiang.toml", name);
    println!("  {}/yaoxiang.lock", name);
    println!("  {}/src/main.yx", name);
    println!("  {}/.gitignore", name);
    println!("  {}/.yaoxiang/std/", name);

    Ok(())
}

/// Initialize a new YaoXiang project in the current directory
pub fn exec(name: &str) -> PackageResult<()> {
    exec_in(&std::env::current_dir()?, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_project() {
        let tmp = TempDir::new().unwrap();
        exec_in(tmp.path(), "test-project").unwrap();

        let project_path = tmp.path().join("test-project");
        assert!(project_path.join("yaoxiang.toml").exists());
        assert!(project_path.join("yaoxiang.lock").exists());
        assert!(project_path.join("src/main.yx").exists());
        assert!(project_path.join(".gitignore").exists());
        // 标准库接口文件
        assert!(project_path.join(".yaoxiang/std/io.yx").exists());
        assert!(project_path.join(".yaoxiang/std/list.yx").exists());
        assert!(project_path.join(".yaoxiang/std/math.yx").exists());
    }

    #[test]
    fn test_init_manifest_content() {
        let tmp = TempDir::new().unwrap();
        exec_in(tmp.path(), "my-app").unwrap();

        let manifest = PackageManifest::load(&tmp.path().join("my-app")).unwrap();
        assert_eq!(manifest.package.name, "my-app");
        assert_eq!(manifest.package.version, "0.1.0");
    }

    #[test]
    fn test_init_main_yx_content() {
        let tmp = TempDir::new().unwrap();
        exec_in(tmp.path(), "my-app").unwrap();

        let content = fs::read_to_string(tmp.path().join("my-app/src/main.yx")).unwrap();
        assert!(content.contains("my-app"));
        // YaoXiang 使用 `main = {...}` 语法而非 `fn main() {}`
        assert!(content.contains("main ="));
    }

    #[test]
    fn test_init_existing_project_fails() {
        let tmp = TempDir::new().unwrap();
        exec_in(tmp.path(), "existing").unwrap();

        let result = exec_in(tmp.path(), "existing");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PackageError::ProjectExists(_)
        ));
    }
}
