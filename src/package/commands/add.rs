//! `yaoxiang add` command - Add a dependency to the project

use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::package::manifest::PackageManifest;
use crate::util::i18n::{t, current_lang, MSG};

/// Add a dependency to a project at the given directory
///
/// # Arguments
/// - `project_dir`: project root directory
/// - `name`: dependency package name
/// - `version`: version requirement string (defaults to "*" if None)
/// - `git`: git repository URL (explicit source; RFC-014a 决议：Registry
///   上线前添加依赖须显式来源)
/// - `path`: local path (explicit source; 与 `git` 互斥)
/// - `dev`: if true, add as dev-dependency
pub fn exec_in(
    project_dir: &Path,
    name: &str,
    version: Option<&str>,
    git: Option<&str>,
    path: Option<&str>,
    dev: bool,
) -> PackageResult<()> {
    let mut manifest = PackageManifest::load(project_dir)?;

    let version = version.unwrap_or("*");

    // Check if dependency already exists
    if manifest.has_dependency(name) {
        return Err(PackageError::DependencyAlreadyExists(name.to_string()));
    }

    let value = match (git, path) {
        (None, None) => toml::Value::String(version.to_string()),
        (Some(url), None) => {
            let mut table = toml::map::Map::new();
            table.insert(
                "version".to_string(),
                toml::Value::String(version.to_string()),
            );
            table.insert("git".to_string(), toml::Value::String(url.to_string()));
            toml::Value::Table(table)
        }
        (None, Some(p)) => {
            let mut table = toml::map::Map::new();
            table.insert(
                "version".to_string(),
                toml::Value::String(version.to_string()),
            );
            table.insert("path".to_string(), toml::Value::String(p.to_string()));
            toml::Value::Table(table)
        }
        // CLI 层 conflicts_with 已互斥；此处防御直接调用
        (Some(_), Some(_)) => {
            return Err(PackageError::InvalidManifest(
                "git 与 path 来源互斥，只能指定其一".to_string(),
            ));
        }
    };

    let lang = current_lang();
    if dev {
        manifest.dev_dependencies.insert(name.to_string(), value);
        println!(
            "{}",
            t(
                MSG::PackageDevDepAdded,
                lang,
                Some(&[&name.to_string(), &version.to_string()])
            )
        );
    } else {
        manifest.dependencies.insert(name.to_string(), value);
        println!(
            "{}",
            t(
                MSG::PackageDepAdded,
                lang,
                Some(&[&name.to_string(), &version.to_string()])
            )
        );
    }

    super::save_manifest_and_update_lock(&manifest, project_dir)?;

    Ok(())
}

/// Add a dependency to the current project
pub fn exec(
    name: &str,
    version: Option<&str>,
    git: Option<&str>,
    path: Option<&str>,
    dev: bool,
) -> PackageResult<()> {
    exec_in(&std::env::current_dir()?, name, version, git, path, dev)
}
