//! `yaoxiang install` command - Install dependencies

use std::path::Path;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;

/// Install all dependencies at the given project directory
///
/// Resolves dependencies from the manifest and updates the lock file.
/// For Phase 1, this performs local resolution only (no registry download).
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;

    let mut lock = LockFile::load(project_dir)?;

    // Merge all dependencies
    let mut all_deps = manifest.dependencies.clone();
    all_deps.extend(manifest.dev_dependencies.clone());

    // Update lock from dependencies
    lock.update_from_dependencies(&all_deps);
    lock.save(project_dir)?;

    // Parse and display dependency information
    let dep_specs = DependencySpec::parse_all(&manifest.dependencies);
    let dev_dep_specs = DependencySpec::parse_all(&manifest.dev_dependencies);

    let total = dep_specs.len() + dev_dep_specs.len();
    if total == 0 {
        println!("没有依赖需要安装。");
    } else {
        println!("✓ 已解析 {} 个依赖:", total);
        for spec in &dep_specs {
            println!("  {} ({})", spec.name, spec.version);
        }
        for spec in &dev_dep_specs {
            println!("  {} ({}) [dev]", spec.name, spec.version);
        }
        println!("\n已更新 yaoxiang.lock");
    }

    Ok(())
}

/// Install all dependencies in the current project
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
    fn test_install_empty() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(lock.package.is_empty());
    }

    #[test]
    fn test_install_with_deps() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
        add::exec_in(&project_dir, "bar", Some("2.0.0"), true).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(lock.package.contains_key("foo"));
        assert!(lock.package.contains_key("bar"));
    }

    #[test]
    fn test_install_updates_lock_correctly() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert_eq!(lock.package["foo"].version, "1.0.0");
    }
}
