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
