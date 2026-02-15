//! `yaoxiang list` command - List project dependencies

use std::path::Path;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::manifest::PackageManifest;

/// List all dependencies in the project at the given directory
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;

    let dep_specs = DependencySpec::parse_all(&manifest.dependencies);
    let dev_dep_specs = DependencySpec::parse_all(&manifest.dev_dependencies);

    println!("{} v{}\n", manifest.package.name, manifest.package.version);

    if dep_specs.is_empty() && dev_dep_specs.is_empty() {
        println!("没有依赖。");
        return Ok(());
    }

    if !dep_specs.is_empty() {
        println!("[dependencies]");
        for spec in &dep_specs {
            let extra = format_extra(spec);
            println!("  {} = \"{}\"{}", spec.name, spec.version, extra);
        }
    }

    if !dev_dep_specs.is_empty() {
        if !dep_specs.is_empty() {
            println!();
        }
        println!("[dev-dependencies]");
        for spec in &dev_dep_specs {
            let extra = format_extra(spec);
            println!("  {} = \"{}\"{}", spec.name, spec.version, extra);
        }
    }

    Ok(())
}

/// List all dependencies in the current project
pub fn exec() -> PackageResult<()> {
    exec_in(&std::env::current_dir()?)
}

fn format_extra(spec: &DependencySpec) -> String {
    let mut parts = Vec::new();
    if let Some(ref git) = spec.git {
        parts.push(format!("git: {}", git));
    }
    if let Some(ref path) = spec.path {
        parts.push(format!("path: {}", path));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::commands::{add, init};
    use tempfile::TempDir;

    fn setup_project() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        init::exec_in(tmp.path(), "test-proj").unwrap();
        let project_dir = tmp.path().join("test-proj");
        (tmp, project_dir)
    }

    #[test]
    fn test_list_empty() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir).unwrap(); // Should not error
    }

    #[test]
    fn test_list_with_deps() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
        add::exec_in(&project_dir, "bar", Some("2.0.0"), true).unwrap();
        exec_in(&project_dir).unwrap(); // Should not error
    }

    #[test]
    fn test_format_extra_empty() {
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: None,
            path: None,
        };
        assert_eq!(format_extra(&spec), "");
    }

    #[test]
    fn test_format_extra_with_git() {
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: Some("https://github.com/example/foo".to_string()),
            path: None,
        };
        let extra = format_extra(&spec);
        assert!(extra.contains("git:"));
    }
}
