//! `yaoxiang add` command - Add a dependency to the project

use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::util::i18n::{t, current_lang, MSG};

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

    let lang = current_lang();
    if dev {
        manifest.add_dev_dependency(name, version);
        println!(
            "{}",
            t(
                MSG::PackageDevDepAdded,
                lang,
                Some(&[&name.to_string(), &version.to_string()])
            )
        );
    } else {
        manifest.add_dependency(name, version);
        println!(
            "{}",
            t(
                MSG::PackageDepAdded,
                lang,
                Some(&[&name.to_string(), &version.to_string()])
            )
        );
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
