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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string_version() {
        let value = toml::Value::String("1.0.0".to_string());
        let spec = DependencySpec::parse("foo", &value);
        assert_eq!(spec.name, "foo");
        assert_eq!(spec.version, "1.0.0");
        assert!(spec.git.is_none());
        assert!(spec.path.is_none());
    }

    #[test]
    fn test_parse_table_version() {
        let toml_str = r#"version = "1.0.0""#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let spec = DependencySpec::parse("foo", &value);
        assert_eq!(spec.name, "foo");
        assert_eq!(spec.version, "1.0.0");
    }

    #[test]
    fn test_parse_table_with_git() {
        let toml_str = r#"
version = "1.0.0"
git = "https://github.com/example/foo"
"#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let spec = DependencySpec::parse("foo", &value);
        assert_eq!(spec.version, "1.0.0");
        assert_eq!(spec.git.as_deref(), Some("https://github.com/example/foo"));
    }

    #[test]
    fn test_parse_table_with_path() {
        let toml_str = r#"
version = "0.1.0"
path = "../local-dep"
"#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let spec = DependencySpec::parse("local", &value);
        assert_eq!(spec.path.as_deref(), Some("../local-dep"));
    }

    #[test]
    fn test_parse_all() {
        let mut deps = BTreeMap::new();
        deps.insert("foo".to_string(), toml::Value::String("1.0.0".to_string()));
        deps.insert("bar".to_string(), toml::Value::String("2.0.0".to_string()));

        let specs = DependencySpec::parse_all(&deps);
        assert_eq!(specs.len(), 2);
    }

    #[test]
    fn test_to_toml_value_simple() {
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: None,
            path: None,
        };
        let value = spec.to_toml_value();
        assert_eq!(value, toml::Value::String("1.0.0".to_string()));
    }

    #[test]
    fn test_to_toml_value_with_git() {
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: Some("https://github.com/example/foo".to_string()),
            path: None,
        };
        let value = spec.to_toml_value();
        assert!(value.is_table());
        let table = value.as_table().unwrap();
        assert_eq!(table["version"].as_str(), Some("1.0.0"));
        assert!(table.contains_key("git"));
    }

    #[test]
    fn test_round_trip() {
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: None,
            path: None,
        };
        let value = spec.to_toml_value();
        let parsed = DependencySpec::parse("foo", &value);
        assert_eq!(spec, parsed);
    }
}
