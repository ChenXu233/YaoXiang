//! yaoxiang.lock file reading and writing

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::package::error::PackageResult;
use crate::util::i18n::{t_simple, current_lang, MSG};

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
        let header = format!(
            "{}\n\n",
            t_simple(MSG::PackageLockGenerated, current_lang())
        );
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
