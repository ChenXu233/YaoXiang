//! Package management CLI commands

pub mod add;
pub mod init;
pub mod install;
pub mod list;
pub mod rm;
pub mod update;

#[cfg(test)]
mod tests;

/// 保存 manifest 并同步 lock 文件（add/rm 共享序列）
pub(crate) fn save_manifest_and_update_lock(
    manifest: &crate::package::manifest::PackageManifest,
    project_dir: &std::path::Path,
) -> crate::package::PackageResult<()> {
    manifest.save(project_dir)?;
    let mut lock = crate::package::lock::LockFile::load(project_dir)?;
    let mut all_deps = manifest.dependencies.clone();
    all_deps.extend(manifest.dev_dependencies.clone());
    lock.update_from_dependencies(&all_deps);
    lock.save(project_dir)?;
    Ok(())
}
