//! Dependency specification parsing

use std::collections::BTreeMap;

/// Represents a dependency specification
#[derive(Debug, Clone, PartialEq)]
pub struct DependencySpec {
    /// Package name
    pub name: String,
    /// Version requirement string
    pub version: String,
    /// Optional git URL
    pub git: Option<String>,
    /// Optional local path
    pub path: Option<String>,
}

impl DependencySpec {
    /// Parse a dependency from a TOML value
    ///
    /// Supports two forms:
    /// - String: `"1.0.0"` -> version dependency
    /// - Table: `{ version = "1.0.0", git = "..." }` -> detailed dependency
    pub fn parse(
        name: &str,
        value: &toml::Value,
    ) -> Self {
        match value {
            toml::Value::String(version) => DependencySpec {
                name: name.to_string(),
                version: version.clone(),
                git: None,
                path: None,
            },
            toml::Value::Table(table) => {
                let version = table
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .to_string();
                let git = table
                    .get("git")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let path = table
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                DependencySpec {
                    name: name.to_string(),
                    version,
                    git,
                    path,
                }
            }
            _ => DependencySpec {
                name: name.to_string(),
                version: "*".to_string(),
                git: None,
                path: None,
            },
        }
    }

    /// Parse all dependencies from a BTreeMap (as stored in manifest)
    pub fn parse_all(deps: &BTreeMap<String, toml::Value>) -> Vec<DependencySpec> {
        deps.iter()
            .map(|(name, value)| DependencySpec::parse(name, value))
            .collect()
    }

    /// Convert back to a TOML value
    pub fn to_toml_value(&self) -> toml::Value {
        if self.git.is_none() && self.path.is_none() {
            // Simple string form
            toml::Value::String(self.version.clone())
        } else {
            // Table form
            let mut table = toml::map::Map::new();
            table.insert(
                "version".to_string(),
                toml::Value::String(self.version.clone()),
            );
            if let Some(ref git) = self.git {
                table.insert("git".to_string(), toml::Value::String(git.clone()));
            }
            if let Some(ref path) = self.path {
                table.insert("path".to_string(), toml::Value::String(path.clone()));
            }
            toml::Value::Table(table)
        }
    }
}
