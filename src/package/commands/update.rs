//! `yaoxiang update` command - Update dependencies

use std::path::Path;

use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;

/// Update all dependencies in the lock file at the given directory
///
/// Re-resolves all dependency versions. For Phase 1,
/// this simply re-reads the manifest and updates the lock file.
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;

    let mut lock = LockFile::load(project_dir)?;

    // Merge all dependencies
    let mut all_deps = manifest.dependencies.clone();
    all_deps.extend(manifest.dev_dependencies.clone());

    // Force-update (re-resolve) all dependencies
    lock.force_update_from_dependencies(&all_deps);
    lock.save(project_dir)?;

    if all_deps.is_empty() {
        println!("没有依赖需要更新。");
    } else {
        println!("✓ 已更新 {} 个依赖:", all_deps.len());
        for (name, _) in &lock.package {
            println!("  {} ({})", name, lock.package[name].version);
        }
    }

    Ok(())
}

/// Update all dependencies in the current project
pub fn exec() -> PackageResult<()> {
    exec_in(&std::env::current_dir()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::commands::{add, init};
    use tempfile::TempDir;

    fn setup_project() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        init::exec_in(tmp.path(), "test-proj").unwrap();
        let project_dir = tmp.path().join("test-proj");
        (tmp, project_dir)
    }

    #[test]
    fn test_update_empty() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(lock.package.is_empty());
    }

    #[test]
    fn test_update_with_deps() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
        add::exec_in(&project_dir, "bar", Some("2.0.0"), false).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert_eq!(lock.package.len(), 2);
        assert!(lock.package.contains_key("foo"));
        assert!(lock.package.contains_key("bar"));
    }

    #[test]
    fn test_update_refreshes_versions() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

        // Manually modify the manifest to simulate version bump
        let mut manifest = PackageManifest::load(&project_dir).unwrap();
        manifest.add_dependency("foo", "2.0.0");
        manifest.save(&project_dir).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert_eq!(lock.package["foo"].version, "2.0.0");
    }
}
