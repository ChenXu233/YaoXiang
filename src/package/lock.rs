//! yaoxiang.lock file reading and writing

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::package::error::PackageResult;

/// The lock file name
pub const LOCK_FILE: &str = "yaoxiang.lock";

/// A single locked dependency entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockedDependency {
    /// Resolved version
    pub version: String,
    /// Source (e.g., "registry", "git", "path")
    #[serde(default = "default_source", skip_serializing_if = "is_default_source")]
    pub source: String,
    /// Integrity hash (SHA-256), optional for Phase 1
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

fn default_source() -> String {
    "registry".to_string()
}

fn is_default_source(s: &str) -> bool {
    s == "registry"
}

/// Represents the complete yaoxiang.lock file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    /// Lock file format version
    #[serde(default = "default_lock_version")]
    pub version: u32,
    /// Locked dependencies (name -> locked info)
    #[serde(default)]
    pub package: BTreeMap<String, LockedDependency>,
}

fn default_lock_version() -> u32 {
    1
}

impl LockFile {
    /// Create a new empty lock file
    pub fn new() -> Self {
        LockFile {
            version: 1,
            package: BTreeMap::new(),
        }
    }

    /// Load lock file from a directory. Returns empty lock file if not found.
    pub fn load(dir: &Path) -> PackageResult<Self> {
        let path = dir.join(LOCK_FILE);
        if !path.exists() {
            return Ok(LockFile::new());
        }
        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            return Ok(LockFile::new());
        }
        let lock: LockFile = toml::from_str(&content)?;
        Ok(lock)
    }

    /// Save lock file to a directory
    pub fn save(
        &self,
        dir: &Path,
    ) -> PackageResult<()> {
        let path = dir.join(LOCK_FILE);
        let content = toml::to_string_pretty(self)?;
        let header = "# 此文件由 yaoxiang 自动生成，请勿手动编辑。\n\n";
        std::fs::write(&path, format!("{}{}", header, content))?;
        Ok(())
    }

    /// Add or update a locked dependency
    pub fn lock_dependency(
        &mut self,
        name: &str,
        version: &str,
    ) {
        self.package.insert(
            name.to_string(),
            LockedDependency {
                version: version.to_string(),
                source: "registry".to_string(),
                checksum: None,
            },
        );
    }

    /// Add or update a locked dependency with full information
    pub fn lock_dependency_full(
        &mut self,
        name: &str,
        version: &str,
        source: &str,
        checksum: Option<&str>,
    ) {
        self.package.insert(
            name.to_string(),
            LockedDependency {
                version: version.to_string(),
                source: source.to_string(),
                checksum: checksum.map(|s| s.to_string()),
            },
        );
    }

    /// Remove a locked dependency
    pub fn remove_dependency(
        &mut self,
        name: &str,
    ) -> bool {
        self.package.remove(name).is_some()
    }

    /// Update lock file from manifest dependencies.
    /// For Phase 1, resolved version = declared version (no registry lookup).
    pub fn update_from_dependencies(
        &mut self,
        deps: &BTreeMap<String, toml::Value>,
    ) {
        for (name, value) in deps {
            let version = match value {
                toml::Value::String(v) => v.clone(),
                toml::Value::Table(t) => t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.0.0")
                    .to_string(),
                _ => "0.0.0".to_string(),
            };
            // Only add if not already locked
            if !self.package.contains_key(name) {
                self.lock_dependency(name, &version);
            }
        }
        // Remove locked deps that are no longer in manifest
        let dep_names: std::collections::HashSet<&String> = deps.keys().collect();
        self.package.retain(|name, _| dep_names.contains(name));
    }

    /// Force update all dependencies (re-resolve versions)
    pub fn force_update_from_dependencies(
        &mut self,
        deps: &BTreeMap<String, toml::Value>,
    ) {
        self.package.clear();
        for (name, value) in deps {
            let version = match value {
                toml::Value::String(v) => v.clone(),
                toml::Value::Table(t) => t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.0.0")
                    .to_string(),
                _ => "0.0.0".to_string(),
            };
            self.lock_dependency(name, &version);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_lock_file() {
        let lock = LockFile::new();
        assert_eq!(lock.version, 1);
        assert!(lock.package.is_empty());
    }

    #[test]
    fn test_load_nonexistent() {
        let dir = TempDir::new().unwrap();
        let lock = LockFile::load(dir.path()).unwrap();
        assert!(lock.package.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = TempDir::new().unwrap();
        let mut lock = LockFile::new();
        lock.lock_dependency("foo", "1.0.0");
        lock.save(dir.path()).unwrap();

        let loaded = LockFile::load(dir.path()).unwrap();
        assert!(loaded.package.contains_key("foo"));
        assert_eq!(loaded.package["foo"].version, "1.0.0");
    }

    #[test]
    fn test_lock_dependency() {
        let mut lock = LockFile::new();
        lock.lock_dependency("foo", "1.2.3");
        assert_eq!(lock.package["foo"].version, "1.2.3");
        assert_eq!(lock.package["foo"].source, "registry");
    }

    #[test]
    fn test_remove_dependency() {
        let mut lock = LockFile::new();
        lock.lock_dependency("foo", "1.0.0");
        assert!(lock.remove_dependency("foo"));
        assert!(!lock.package.contains_key("foo"));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut lock = LockFile::new();
        assert!(!lock.remove_dependency("foo"));
    }

    #[test]
    fn test_update_from_dependencies() {
        let mut lock = LockFile::new();
        let mut deps = BTreeMap::new();
        deps.insert("foo".to_string(), toml::Value::String("1.0.0".to_string()));
        deps.insert("bar".to_string(), toml::Value::String("2.0.0".to_string()));

        lock.update_from_dependencies(&deps);
        assert_eq!(lock.package.len(), 2);
        assert_eq!(lock.package["foo"].version, "1.0.0");
        assert_eq!(lock.package["bar"].version, "2.0.0");
    }

    #[test]
    fn test_update_removes_stale_deps() {
        let mut lock = LockFile::new();
        lock.lock_dependency("old-dep", "0.1.0");

        let mut deps = BTreeMap::new();
        deps.insert(
            "new-dep".to_string(),
            toml::Value::String("1.0.0".to_string()),
        );

        lock.update_from_dependencies(&deps);
        assert!(!lock.package.contains_key("old-dep"));
        assert!(lock.package.contains_key("new-dep"));
    }

    #[test]
    fn test_force_update() {
        let mut lock = LockFile::new();
        lock.lock_dependency("foo", "1.0.0");

        let mut deps = BTreeMap::new();
        deps.insert("foo".to_string(), toml::Value::String("2.0.0".to_string()));

        lock.force_update_from_dependencies(&deps);
        assert_eq!(lock.package["foo"].version, "2.0.0");
    }

    #[test]
    fn test_save_contains_header() {
        let dir = TempDir::new().unwrap();
        let lock = LockFile::new();
        lock.save(dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join(LOCK_FILE)).unwrap();
        assert!(content.contains("自动生成"));
    }
}
