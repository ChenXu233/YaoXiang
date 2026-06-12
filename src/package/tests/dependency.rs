//! 测试 `DependencySpec` 的解析与序列化功能
//!
//! 覆盖:
//! - 字符串版本解析
//! - 表格形式解析（含 git/path 字段）
//! - `parse_all` 批量解析
//! - `to_toml_value` 序列化与往返一致性

use std::collections::BTreeMap;

use crate::package::dependency::DependencySpec;

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
