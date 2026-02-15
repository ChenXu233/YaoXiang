//! `yaoxiang rm` command - Remove a dependency from the project

use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::util::i18n::{t, current_lang, MSG};

/// Remove a dependency from a project at the given directory
///
/// # Arguments
/// - `project_dir`: project root directory
/// - `name`: dependency package name to remove
/// - `dev`: if true, remove from dev-dependencies
pub fn exec_in(
    project_dir: &Path,
    name: &str,
    dev: bool,
) -> PackageResult<()> {
    let mut manifest = PackageManifest::load(project_dir)?;

    let removed = if dev {
        manifest.remove_dev_dependency(name)
    } else {
        manifest.remove_dependency(name)
    };

    if !removed {
        return Err(PackageError::DependencyNotFound(name.to_string()));
    }

    manifest.save(project_dir)?;

    // Update lock file
    let mut lock = LockFile::load(project_dir)?;
    let mut all_deps = manifest.dependencies.clone();
    all_deps.extend(manifest.dev_dependencies.clone());
    lock.update_from_dependencies(&all_deps);
    lock.save(project_dir)?;

    let lang = current_lang();
    if dev {
        println!(
            "{}",
            t(MSG::PackageDevDepRemoved, lang, Some(&[&name.to_string()]))
        );
    } else {
        println!(
            "{}",
            t(MSG::PackageDepRemoved, lang, Some(&[&name.to_string()]))
        );
    }

    Ok(())
}

/// Remove a dependency from the current project
pub fn exec(
    name: &str,
    dev: bool,
) -> PackageResult<()> {
    exec_in(&std::env::current_dir()?, name, dev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::commands::{add, init};
    use tempfile::TempDir;

    fn setup_project_with_deps() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        init::exec_in(tmp.path(), "test-proj").unwrap();
        let project_dir = tmp.path().join("test-proj");
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
        add::exec_in(&project_dir, "bar", Some("2.0.0"), true).unwrap();
        (tmp, project_dir)
    }

    #[test]
    fn test_rm_dependency() {
        let (_tmp, project_dir) = setup_project_with_deps();
        exec_in(&project_dir, "foo", false).unwrap();

        let manifest = PackageManifest::load(&project_dir).unwrap();
        assert!(!manifest.dependencies.contains_key("foo"));
    }

    #[test]
    fn test_rm_dev_dependency() {
        let (_tmp, project_dir) = setup_project_with_deps();
        exec_in(&project_dir, "bar", true).unwrap();

        let manifest = PackageManifest::load(&project_dir).unwrap();
        assert!(!manifest.dev_dependencies.contains_key("bar"));
    }

    #[test]
    fn test_rm_updates_lock() {
        let (_tmp, project_dir) = setup_project_with_deps();
        exec_in(&project_dir, "foo", false).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(!lock.package.contains_key("foo"));
    }

    #[test]
    fn test_rm_nonexistent_fails() {
        let (_tmp, project_dir) = setup_project_with_deps();
        let result = exec_in(&project_dir, "nonexistent", false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PackageError::DependencyNotFound(_)
        ));
    }
}
