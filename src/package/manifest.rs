//! yaoxiang.toml manifest parsing and writing

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::util::config::I18nConfig;

/// The main manifest file name
pub const MANIFEST_FILE: &str = "yaoxiang.toml";

/// Represents the `[package]` section of yaoxiang.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version (semver string)
    pub version: String,
    /// Package description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Package authors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// Package license
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// Represents the complete yaoxiang.toml manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Package metadata
    pub package: PackageInfo,
    /// Runtime dependencies
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, toml::Value>,
    /// Development dependencies
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        rename = "dev-dependencies"
    )]
    pub dev_dependencies: BTreeMap<String, toml::Value>,
    /// I18n configuration (project-level overrides user-level)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i18n: Option<I18nConfig>,
}

impl PackageManifest {
    /// Create a new manifest with the given project name
    pub fn new(name: &str) -> Self {
        PackageManifest {
            package: PackageInfo {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                description: None,
                authors: Vec::new(),
                license: None,
            },
            dependencies: BTreeMap::new(),
            dev_dependencies: BTreeMap::new(),
            i18n: None,
        }
    }

    /// Load manifest from a directory containing yaoxiang.toml
    pub fn load(dir: &Path) -> PackageResult<Self> {
        let path = dir.join(MANIFEST_FILE);
        if !path.exists() {
            return Err(PackageError::NotProject);
        }
        let content = std::fs::read_to_string(&path)?;
        let manifest: PackageManifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    /// Save manifest to a directory
    pub fn save(
        &self,
        dir: &Path,
    ) -> PackageResult<()> {
        let path = dir.join(MANIFEST_FILE);
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a dependency
    pub fn add_dependency(
        &mut self,
        name: &str,
        version: &str,
    ) {
        self.dependencies
            .insert(name.to_string(), toml::Value::String(version.to_string()));
    }

    /// Add a dev dependency
    pub fn add_dev_dependency(
        &mut self,
        name: &str,
        version: &str,
    ) {
        self.dev_dependencies
            .insert(name.to_string(), toml::Value::String(version.to_string()));
    }

    /// Remove a dependency. Returns true if it was present.
    pub fn remove_dependency(
        &mut self,
        name: &str,
    ) -> bool {
        self.dependencies.remove(name).is_some()
    }

    /// Remove a dev dependency. Returns true if it was present.
    pub fn remove_dev_dependency(
        &mut self,
        name: &str,
    ) -> bool {
        self.dev_dependencies.remove(name).is_some()
    }

    /// Check if a dependency exists (in either dependencies or dev-dependencies)
    pub fn has_dependency(
        &self,
        name: &str,
    ) -> bool {
        self.dependencies.contains_key(name) || self.dev_dependencies.contains_key(name)
    }
}
