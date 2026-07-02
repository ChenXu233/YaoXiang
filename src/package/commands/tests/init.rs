use std::fs;

use crate::package::commands::init::{exec_in, exec_here, InitOptions};
use crate::package::error::PackageError;
use crate::package::manifest::PackageManifest;
use tempfile::TempDir;

fn default_opts() -> InitOptions {
    InitOptions { lib: false }
}

fn lib_opts() -> InitOptions {
    InitOptions { lib: true }
}

#[test]
fn test_init_creates_project() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &default_opts(), "test-project").unwrap();

    let project_path = tmp.path().join("test-project");
    assert!(project_path.join("yaoxiang.toml").exists());
    assert!(project_path.join("yaoxiang.lock").exists());
    assert!(project_path.join("src/main.yx").exists());
    assert!(project_path.join(".gitignore").exists());
    assert!(project_path.join("tests").is_dir());
    assert!(project_path.join(".yaoxiang/std/io.yx").exists());
}

#[test]
fn test_init_manifest_content() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &default_opts(), "my-app").unwrap();

    let manifest = PackageManifest::load(&tmp.path().join("my-app")).unwrap();
    assert_eq!(manifest.package.name, "my-app");
    assert_eq!(manifest.package.version, "0.1.0");
}

#[test]
fn test_init_main_yx_content() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &default_opts(), "my-app").unwrap();

    let content = fs::read_to_string(tmp.path().join("my-app/src/main.yx")).unwrap();
    assert!(content.contains("my-app"));
    assert!(content.contains("main ="));
}

#[test]
fn test_init_existing_project_fails() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &default_opts(), "existing").unwrap();

    let result = exec_in(tmp.path(), &default_opts(), "existing");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PackageError::ProjectExists(_)
    ));
}

#[test]
fn test_new_lib_creates_lib_yx() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &lib_opts(), "my-lib").unwrap();

    let project_path = tmp.path().join("my-lib");
    assert!(project_path.join("src/lib.yx").exists());
    // lib project should NOT have main.yx
    assert!(!project_path.join("src/main.yx").exists());
}

#[test]
fn test_new_lib_content_no_main() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &lib_opts(), "my-lib").unwrap();

    let content = fs::read_to_string(tmp.path().join("my-lib/src/lib.yx")).unwrap();
    assert!(content.contains("my-lib"));
    assert!(content.contains("\u5e93\u9879\u76ee"));
    assert!(!content.contains("main ="));
}

#[test]
fn test_init_here_creates_in_current_dir() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my-here");
    fs::create_dir(&project_dir).unwrap();

    let _guard = std::env::set_current_dir(&project_dir);
    exec_here(&default_opts()).unwrap();

    assert!(project_dir.join("yaoxiang.toml").exists());
    assert!(project_dir.join("src/main.yx").exists());
    assert!(project_dir.join("tests").is_dir());
}

#[test]
fn test_init_here_existing_project_fails() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my-here");
    fs::create_dir(&project_dir).unwrap();

    let _guard = std::env::set_current_dir(&project_dir);
    exec_here(&default_opts()).unwrap();

    let result = exec_here(&default_opts());
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PackageError::ProjectExists(_)
    ));
}

#[test]
fn test_init_here_skips_existing_files() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my-here");
    fs::create_dir(&project_dir).unwrap();
    fs::create_dir_all(project_dir.join("src")).unwrap();

    let main_path = project_dir.join("src").join("main.yx");
    fs::write(&main_path, "// existing content").unwrap();

    let _guard = std::env::set_current_dir(&project_dir);
    exec_here(&default_opts()).unwrap();

    let content = fs::read_to_string(&main_path).unwrap();
    assert_eq!(content, "// existing content");
    assert!(project_dir.join("yaoxiang.toml").exists());
}

#[test]
fn test_new_and_init_name_equivalent() {
    let tmp_new = TempDir::new().unwrap();
    let tmp_init = TempDir::new().unwrap();

    let opts = default_opts();
    exec_in(tmp_new.path(), &opts, "eq-test").unwrap();
    exec_in(tmp_init.path(), &opts, "eq-test").unwrap();

    let new_toml = fs::read_to_string(tmp_new.path().join("eq-test/yaoxiang.toml")).unwrap();
    let init_toml = fs::read_to_string(tmp_init.path().join("eq-test/yaoxiang.toml")).unwrap();
    assert_eq!(new_toml, init_toml);

    let new_main = fs::read_to_string(tmp_new.path().join("eq-test/src/main.yx")).unwrap();
    let init_main = fs::read_to_string(tmp_init.path().join("eq-test/src/main.yx")).unwrap();
    assert_eq!(new_main, init_main);
}