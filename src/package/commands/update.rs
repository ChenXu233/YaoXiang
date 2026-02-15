//! `yaoxiang update` command - Update dependencies

use std::path::Path;

use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::vendor::fetcher;
use crate::package::vendor::VendorManager;

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
        println!("没有依赖需要更新。");
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

    if result.installed.is_empty() && result.skipped.is_empty() {
        println!("没有依赖需要更新。");
    } else {
        let total = result.installed.len() + result.skipped.len();
        println!("✓ 已更新 {} 个依赖:", total);
        for resolved in &result.installed {
            println!("  {} ({})", resolved.name, resolved.version);
        }
        for (name, version) in &result.skipped {
            println!("  {} ({}) [本地]", name, version);
        }
    }

    if !result.failed.is_empty() {
        println!("\n⚠ {} 个依赖更新失败:", result.failed.len());
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
                println!("✓ 已更新 {} → {}", name, resolved.version);
            }
            Err(e) => {
                println!("⚠ {} 更新失败: {}", name, e);
            }
        }
    } else if spec.path.is_some() {
        lock.lock_dependency_full(name, &resolved_version, "path", None);
        println!("✓ {} ({}) 已是最新", name, resolved_version);
    } else {
        lock.lock_dependency_full(name, &resolved_version, "registry", None);
        println!("✓ 已更新 {} → {}", name, resolved_version);
    }

    lock.save(project_dir)?;

    Ok(())
}

/// Update all dependencies in the current project
pub fn exec() -> PackageResult<()> {
    exec_in(&std::env::current_dir()?)
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
    fn test_update_empty() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(lock.package.is_empty());
    }

    #[test]
    fn test_update_with_deps() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
        add::exec_in(&project_dir, "bar", Some("2.0.0"), false).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert_eq!(lock.package.len(), 2);
        assert!(lock.package.contains_key("foo"));
        assert!(lock.package.contains_key("bar"));
    }

    #[test]
    fn test_update_refreshes_versions() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

        // Manually modify the manifest to simulate version bump
        let mut manifest = PackageManifest::load(&project_dir).unwrap();
        manifest.add_dependency("foo", "2.0.0");
        manifest.save(&project_dir).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert_eq!(lock.package["foo"].version, "2.0.0");
    }

    #[test]
    fn test_update_single_dependency() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
        add::exec_in(&project_dir, "bar", Some("2.0.0"), false).unwrap();

        // 先安装
        crate::package::commands::install::exec_in(&project_dir).unwrap();

        // 修改 foo 的版本
        let mut manifest = PackageManifest::load(&project_dir).unwrap();
        manifest.add_dependency("foo", "1.1.0");
        manifest.save(&project_dir).unwrap();

        // 只更新 foo
        exec_single_in(&project_dir, "foo").unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert_eq!(lock.package["foo"].version, "1.1.0");
        // bar 不受影响
        assert!(lock.package.contains_key("bar"));
    }
}
