//! `yaoxiang add` command - Add a dependency to the project

use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;

/// Add a dependency to a project at the given directory
///
/// # Arguments
/// - `project_dir`: project root directory
/// - `name`: dependency package name
/// - `version`: version string (defaults to "*" if None)
/// - `dev`: if true, add as dev-dependency
pub fn exec_in(
    project_dir: &Path,
    name: &str,
    version: Option<&str>,
    dev: bool,
) -> PackageResult<()> {
    let mut manifest = PackageManifest::load(project_dir)?;

    let version = version.unwrap_or("*");

    // Check if dependency already exists
    if manifest.has_dependency(name) {
        return Err(PackageError::DependencyAlreadyExists(name.to_string()));
    }

    if dev {
        manifest.add_dev_dependency(name, version);
        println!("✓ 已添加开发依赖 '{}' ({})", name, version);
    } else {
        manifest.add_dependency(name, version);
        println!("✓ 已添加依赖 '{}' ({})", name, version);
    }

    manifest.save(project_dir)?;

    // Update lock file
    let mut lock = LockFile::load(project_dir)?;
    let mut all_deps = manifest.dependencies.clone();
    all_deps.extend(manifest.dev_dependencies.clone());
    lock.update_from_dependencies(&all_deps);
    lock.save(project_dir)?;

    Ok(())
}

/// Add a dependency to the current project
pub fn exec(
    name: &str,
    version: Option<&str>,
    dev: bool,
) -> PackageResult<()> {
    exec_in(&std::env::current_dir()?, name, version, dev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::commands::init;
    use tempfile::TempDir;

    fn setup_project() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        init::exec_in(tmp.path(), "test-proj").unwrap();
        let project_dir = tmp.path().join("test-proj");
        (tmp, project_dir)
    }

    #[test]
    fn test_add_dependency() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

        let manifest = PackageManifest::load(&project_dir).unwrap();
        assert!(manifest.dependencies.contains_key("foo"));
    }

    #[test]
    fn test_add_dev_dependency() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir, "bar", Some("2.0.0"), true).unwrap();

        let manifest = PackageManifest::load(&project_dir).unwrap();
        assert!(manifest.dev_dependencies.contains_key("bar"));
    }

    #[test]
    fn test_add_updates_lock() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(lock.package.contains_key("foo"));
        assert_eq!(lock.package["foo"].version, "1.0.0");
    }

    #[test]
    fn test_add_duplicate_fails() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

        let result = exec_in(&project_dir, "foo", Some("2.0.0"), false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PackageError::DependencyAlreadyExists(_)
        ));
    }

    #[test]
    fn test_add_default_version() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir, "foo", None, false).unwrap();

        let manifest = PackageManifest::load(&project_dir).unwrap();
        assert_eq!(
            manifest.dependencies["foo"],
            toml::Value::String("*".to_string())
        );
    }
}
