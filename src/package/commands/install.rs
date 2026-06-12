//! `yaoxiang install` command - Install dependencies

use std::path::Path;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::source::conflict;
use crate::package::vendor::fetcher;
use crate::util::i18n::{t, t_simple, current_lang, MSG};

/// Install all dependencies at the given project directory
///
/// Resolves dependencies from the manifest, downloads them to vendor directory,
/// and updates the lock file with integrity checksums.
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;

    let mut lock = LockFile::load(project_dir)?;

    // Merge all dependencies
    let mut all_deps = manifest.dependencies.clone();
    all_deps.extend(manifest.dev_dependencies.clone());

    if all_deps.is_empty() {
        println!("{}", t_simple(MSG::PackageNoDepsToInstall, current_lang()));
        return Ok(());
    }

    // 检测版本冲突
    let dep_specs = DependencySpec::parse_all(&manifest.dependencies);
    let dev_dep_specs = DependencySpec::parse_all(&manifest.dev_dependencies);
    conflict::check_conflicts(&dep_specs, &dev_dep_specs)?;

    // 使用 fetcher 下载所有依赖
    let result = fetcher::fetch_all(project_dir, &all_deps, &mut lock)?;

    // 保存更新后的锁文件
    lock.save(project_dir)?;

    // 显示结果
    let lang = current_lang();
    let total = dep_specs.len() + dev_dep_specs.len();

    println!(
        "{}",
        t(MSG::PackageDepsResolved, lang, Some(&[&total.to_string()]))
    );
    for spec in &dep_specs {
        let status = if result.installed.iter().any(|r| r.name == spec.name) {
            t_simple(MSG::PackageDepInstalled, lang)
        } else {
            t_simple(MSG::PackageDepCached, lang)
        };
        println!("  {} ({}) [{}]", spec.name, spec.version, status);
    }
    for spec in &dev_dep_specs {
        let status = if result.installed.iter().any(|r| r.name == spec.name) {
            t_simple(MSG::PackageDepInstalled, lang)
        } else {
            t_simple(MSG::PackageDepCached, lang)
        };
        println!("  {} ({}) [dev, {}]", spec.name, spec.version, status);
    }

    if !result.failed.is_empty() {
        println!(
            "\n{}",
            t(
                MSG::PackageDepsInstallFailed,
                lang,
                Some(&[&result.failed.len().to_string()])
            )
        );
        for (name, err) in &result.failed {
            println!("  {} - {}", name, err);
        }
    }

    println!("\n{}", t_simple(MSG::PackageLockUpdated, lang));

    Ok(())
}

/// Install all dependencies in the current project
pub fn exec() -> PackageResult<()> {
    exec_in(&std::env::current_dir()?)
}
