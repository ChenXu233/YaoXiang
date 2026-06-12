//! `yaoxiang update` command - Update dependencies

use std::path::Path;

use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::vendor::fetcher;
use crate::package::vendor::VendorManager;
use crate::util::i18n::{t, t_simple, current_lang, MSG};

/// Update all dependencies in the lock file at the given directory
///
/// Re-resolves all dependency versions, downloads updated packages,
/// and refreshes the lock file.
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;

    let mut lock = LockFile::new(); // 清空锁文件，强制重新解析所有依赖

    // Merge all dependencies
    let mut all_deps = manifest.dependencies.clone();
    all_deps.extend(manifest.dev_dependencies.clone());

    if all_deps.is_empty() {
        println!("{}", t_simple(MSG::PackageNoDepsToUpdate, current_lang()));
        return Ok(());
    }

    // 清理 vendor 目录中的旧版本
    let manager = VendorManager::new(project_dir);
    if let Ok(installed) = manager.list_installed() {
        for (name, version) in &installed {
            let _ = manager.uninstall_dependency(name, version);
        }
    }

    // 使用 fetcher 重新下载所有依赖
    let result = fetcher::fetch_all(project_dir, &all_deps, &mut lock)?;

    // 保存更新后的锁文件
    lock.save(project_dir)?;

    let lang = current_lang();
    if result.installed.is_empty() && result.skipped.is_empty() {
        println!("{}", t_simple(MSG::PackageNoDepsToUpdate, lang));
    } else {
        let total = result.installed.len() + result.skipped.len();
        println!(
            "{}",
            t(MSG::PackageDepsUpdated, lang, Some(&[&total.to_string()]))
        );
        for resolved in &result.installed {
            println!("  {} ({})", resolved.name, resolved.version);
        }
        for (name, version) in &result.skipped {
            println!(
                "  {} ({}) [{}]",
                name,
                version,
                t_simple(MSG::PackageDepCached, lang)
            );
        }
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

    Ok(())
}

/// Update a specific dependency by name
pub fn exec_single_in(
    project_dir: &Path,
    name: &str,
) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;
    let mut lock = LockFile::load(project_dir)?;

    // 查找指定依赖
    let dep_value = manifest
        .dependencies
        .get(name)
        .or_else(|| manifest.dev_dependencies.get(name))
        .ok_or_else(|| crate::package::error::PackageError::DependencyNotFound(name.to_string()))?;

    // 删除旧版本
    if let Some(locked) = lock.package.get(name) {
        let manager = VendorManager::new(project_dir);
        let _ = manager.uninstall_dependency(name, &locked.version);
    }

    // 移除锁文件中的条目，强制重新解析
    lock.remove_dependency(name);

    // 重新安装单个依赖
    let spec = crate::package::dependency::DependencySpec::parse(name, dep_value);
    let source = crate::package::source::select_source(&spec);
    let resolved_version = source
        .resolve(&spec)
        .unwrap_or_else(|_| spec.version.clone());

    // 根据来源类型处理
    let lang = current_lang();
    if spec.git.is_some() {
        let manager = VendorManager::new(project_dir);
        match manager.install_dependency(&spec) {
            Ok(resolved) => {
                lock.lock_dependency_full(
                    &resolved.name,
                    &resolved.version,
                    &resolved.source_kind.to_string(),
                    resolved.checksum.as_deref(),
                );
                println!(
                    "{}",
                    t(
                        MSG::PackageDepsUpdated,
                        lang,
                        Some(&[&name.to_string(), &resolved.version.to_string()])
                    )
                );
            }
            Err(e) => {
                println!(
                    "{}",
                    t(
                        MSG::PackageUpdateFailed,
                        lang,
                        Some(&[&name.to_string(), &e.to_string()])
                    )
                );
            }
        }
    } else if spec.path.is_some() {
        lock.lock_dependency_full(name, &resolved_version, "path", None);
        println!(
            "{}",
            t(
                MSG::PackageAlreadyUpToDate,
                lang,
                Some(&[&name.to_string(), &resolved_version.to_string()])
            )
        );
    } else {
        lock.lock_dependency_full(name, &resolved_version, "registry", None);
        println!(
            "{}",
            t(
                MSG::PackageDepsUpdated,
                lang,
                Some(&[&name.to_string(), &resolved_version.to_string()])
            )
        );
    }

    lock.save(project_dir)?;

    Ok(())
}

/// Update all dependencies in the current project
pub fn exec() -> PackageResult<()> {
    exec_in(&std::env::current_dir()?)
}
