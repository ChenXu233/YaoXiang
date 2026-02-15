//! `yaoxiang install` command - Install dependencies

use std::path::Path;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::source::conflict;
use crate::package::vendor::fetcher;

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
        println!("没有依赖需要安装。");
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
    let total = dep_specs.len() + dev_dep_specs.len();

    println!("✓ 已解析 {} 个依赖:", total);
    for spec in &dep_specs {
        let status = if result.installed.iter().any(|r| r.name == spec.name) {
            "已安装"
        } else {
            "已缓存"
        };
        println!("  {} ({}) [{}]", spec.name, spec.version, status);
    }
    for spec in &dev_dep_specs {
        let status = if result.installed.iter().any(|r| r.name == spec.name) {
            "已安装"
        } else {
            "已缓存"
        };
        println!("  {} ({}) [dev, {}]", spec.name, spec.version, status);
    }

    if !result.failed.is_empty() {
        println!("\n⚠ {} 个依赖安装失败:", result.failed.len());
        for (name, err) in &result.failed {
            println!("  {} - {}", name, err);
        }
    }

    println!("\n已更新 yaoxiang.lock");

    Ok(())
}

/// Install all dependencies in the current project
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
    fn test_install_empty() {
        let (_tmp, project_dir) = setup_project();
        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(lock.package.is_empty());
    }

    #[test]
    fn test_install_with_deps() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
        add::exec_in(&project_dir, "bar", Some("2.0.0"), true).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(lock.package.contains_key("foo"));
        assert!(lock.package.contains_key("bar"));
    }

    #[test]
    fn test_install_updates_lock_correctly() {
        let (_tmp, project_dir) = setup_project();
        add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert_eq!(lock.package["foo"].version, "1.0.0");
    }

    #[test]
    fn test_install_local_dependency() {
        let (_tmp, project_dir) = setup_project();

        // 创建本地依赖目录
        let local_dep_dir = project_dir.join("local-dep");
        std::fs::create_dir_all(&local_dep_dir).unwrap();
        std::fs::write(local_dep_dir.join("lib.yx"), "export x = 42").unwrap();

        // 添加本地依赖
        let mut manifest = PackageManifest::load(&project_dir).unwrap();
        let mut dep_table = toml::map::Map::new();
        dep_table.insert(
            "version".to_string(),
            toml::Value::String("0.1.0".to_string()),
        );
        dep_table.insert(
            "path".to_string(),
            toml::Value::String("./local-dep".to_string()),
        );
        manifest
            .dependencies
            .insert("local-dep".to_string(), toml::Value::Table(dep_table));
        manifest.save(&project_dir).unwrap();

        exec_in(&project_dir).unwrap();

        let lock = LockFile::load(&project_dir).unwrap();
        assert!(lock.package.contains_key("local-dep"));
        assert_eq!(lock.package["local-dep"].source, "path");
    }
}
