//! `yaoxiang list` command - List project dependencies

use std::path::Path;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::manifest::PackageManifest;
use crate::util::i18n::t_simple;
use crate::util::i18n::MSG;

/// List all dependencies in the project at the given directory
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;

    let dep_specs = DependencySpec::parse_all(&manifest.dependencies);
    let dev_dep_specs = DependencySpec::parse_all(&manifest.dev_dependencies);

    println!("{} v{}\n", manifest.package.name, manifest.package.version);

    if dep_specs.is_empty() && dev_dep_specs.is_empty() {
        println!(
            "{}",
            t_simple(MSG::PackageNoDeps, crate::util::i18n::current_lang())
        );
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
