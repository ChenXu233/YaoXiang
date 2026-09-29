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
///
/// RFC-014c：目录处于工作空间内（含成员目录）时，安装对象是**整个工作空间**
/// ——合并全部成员依赖到共享 lockfile、下载到根 vendor（2026-09-15 决议 3：
/// 根 lockfile 唯一）。
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    // 工作空间优先：成员/根目录内的 install 都落到根
    if let Some(ws_root) = crate::package::workspace::find_workspace_root(project_dir) {
        return exec_workspace(&ws_root);
    }

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
    // Phase 3.5：Source 层 async 化，命令层 block_on 驱动（无运行时，
    // Phase 4 接 reqwest 时此处换真执行器即可）
    let result =
        futures::executor::block_on(fetcher::fetch_all(project_dir, &all_deps, &mut lock))?;

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
        // 安装失败必须非零退出（此前打印失败但返回 Ok，脚本无从判定）
        return Err(
            crate::package::error::PackageError::DependencyInstallFailed(
                result
                    .failed
                    .iter()
                    .map(|(name, err)| format!("{}: {}", name, err))
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        );
    }

    println!("\n{}", t_simple(MSG::PackageLockUpdated, lang));

    Ok(())
}

/// 安装整个工作空间的依赖（RFC-014c §lockfile 共享）
///
/// 合并全部成员依赖（含继承展开与交集合成）→ 下载到根 vendor → 登记根
/// lockfile；成员引用（`{ workspace = "<key>" }`）不下载，直接以
/// `source = "workspace"` 登记根 lock（模块解析由 6c 接管）。
fn exec_workspace(ws_root: &Path) -> PackageResult<()> {
    let ws = crate::package::workspace::load_workspace(ws_root)?;
    let merged = crate::package::workspace::merged_dependencies(&ws)?;

    if merged.fetch.is_empty() && merged.member_refs.is_empty() {
        println!("{}", t_simple(MSG::PackageNoDepsToInstall, current_lang()));
        return Ok(());
    }

    let mut lock = ws.lock;
    let result =
        futures::executor::block_on(fetcher::fetch_all(ws_root, &merged.fetch, &mut lock))?;

    // 成员引用：校验通过的 key 直接登记（不进 vendor）
    for (name, key) in &merged.member_refs {
        let version = ws
            .members
            .iter()
            .find(|m| &m.name == key)
            .map(|m| m.manifest.package.version.clone())
            .unwrap_or_default();
        lock.lock_dependency_full(name, &version, "workspace", None);
    }

    lock.save(ws_root)?;

    let lang = current_lang();
    println!(
        "{}",
        t(
            MSG::PackageDepsResolved,
            lang,
            Some(&[&(merged.fetch.len().to_string())])
        )
    );
    for resolved in &result.installed {
        println!(
            "  {} ({}) [{}]",
            resolved.name,
            resolved.version,
            t_simple(MSG::PackageDepInstalled, lang)
        );
    }
    for (name, version) in &result.skipped {
        println!(
            "  {} ({}) [{}]",
            name,
            version,
            t_simple(MSG::PackageDepCached, lang)
        );
    }
    for (name, key) in &merged.member_refs {
        println!("  {name} (workspace:{key})");
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
        return Err(
            crate::package::error::PackageError::DependencyInstallFailed(
                result
                    .failed
                    .iter()
                    .map(|(name, err)| format!("{}: {}", name, err))
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        );
    }

    println!("\n{}", t_simple(MSG::PackageLockUpdated, lang));
    Ok(())
}

/// Install all dependencies in the current project
pub fn exec() -> PackageResult<()> {
    exec_in(&std::env::current_dir()?)
}
