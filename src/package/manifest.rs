//! yaoxiang.toml manifest parsing and writing

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::package::error::{PackageError, PackageResult};

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_manifest() {
        let manifest = PackageManifest::new("test-project");
        assert_eq!(manifest.package.name, "test-project");
        assert_eq!(manifest.package.version, "0.1.0");
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.dev_dependencies.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = TempDir::new().unwrap();
        let manifest = PackageManifest::new("test-project");
        manifest.save(dir.path()).unwrap();

        let loaded = PackageManifest::load(dir.path()).unwrap();
        assert_eq!(loaded.package.name, "test-project");
        assert_eq!(loaded.package.version, "0.1.0");
    }

    #[test]
    fn test_load_not_project() {
        let dir = TempDir::new().unwrap();
        let result = PackageManifest::load(dir.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PackageError::NotProject));
    }

    #[test]
    fn test_add_dependency() {
        let mut manifest = PackageManifest::new("test");
        manifest.add_dependency("foo", "1.0.0");
        assert!(manifest.dependencies.contains_key("foo"));
        assert_eq!(
            manifest.dependencies["foo"],
            toml::Value::String("1.0.0".to_string())
        );
    }

    #[test]
    fn test_add_dev_dependency() {
        let mut manifest = PackageManifest::new("test");
        manifest.add_dev_dependency("bar", "2.0.0");
        assert!(manifest.dev_dependencies.contains_key("bar"));
    }

    #[test]
    fn test_remove_dependency() {
        let mut manifest = PackageManifest::new("test");
        manifest.add_dependency("foo", "1.0.0");
        assert!(manifest.remove_dependency("foo"));
        assert!(!manifest.dependencies.contains_key("foo"));
    }

    #[test]
    fn test_remove_nonexistent_dependency() {
        let mut manifest = PackageManifest::new("test");
        assert!(!manifest.remove_dependency("nonexistent"));
    }

    #[test]
    fn test_has_dependency() {
        let mut manifest = PackageManifest::new("test");
        manifest.add_dependency("foo", "1.0.0");
        manifest.add_dev_dependency("bar", "2.0.0");
        assert!(manifest.has_dependency("foo"));
        assert!(manifest.has_dependency("bar"));
        assert!(!manifest.has_dependency("baz"));
    }

    #[test]
    fn test_round_trip_with_dependencies() {
        let dir = TempDir::new().unwrap();
        let mut manifest = PackageManifest::new("test-project");
        manifest.add_dependency("foo", "1.0.0");
        manifest.add_dev_dependency("bar", "2.0.0");
        manifest.save(dir.path()).unwrap();

        let loaded = PackageManifest::load(dir.path()).unwrap();
        assert!(loaded.dependencies.contains_key("foo"));
        assert!(loaded.dev_dependencies.contains_key("bar"));
    }

    #[test]
    fn test_parse_toml_with_table_dependency() {
        let toml_str = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
foo = "1.0.0"
bar = { version = "2.0.0", git = "https://github.com/example/bar" }
"#;
        let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package.name, "test");
        assert!(manifest.dependencies.contains_key("foo"));
        assert!(manifest.dependencies.contains_key("bar"));
    }

    #[test]
    fn test_parse_empty_dependencies() {
        let toml_str = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.dev_dependencies.is_empty());
    }
}
