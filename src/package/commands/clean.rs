//! `yaoxiang clean` command - Clean build artifacts and stale vendor packages (RFC-014 Phase 3)

use std::path::Path;

use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::vendor::VendorManager;
use crate::util::i18n::{t, t_simple, current_lang, MSG};

/// 清理项目目录
///
/// - 删除构建产物目录 `.yaoxiang/build/`（RFC-014b 构建系统落地前的预留契约）
/// - 裁剪 vendor 中不再被 lock 文件引用的残留包
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    let lang = current_lang();
    let mut removed_any = false;

    let build_dir = project_dir
        .join(crate::package::vendor::VENDOR_DIR)
        .join("build");
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)?;
        println!(
            "{}",
            t(
                MSG::PackageCleanRemoved,
                lang,
                Some(&[&build_dir.display().to_string()])
            )
        );
        removed_any = true;
    }

    let lock = LockFile::load(project_dir)?;
    let keep: Vec<(String, String)> = lock
        .package
        .iter()
        .map(|(name, dep)| (name.clone(), dep.version.clone()))
        .collect();

    let manager = VendorManager::new(project_dir);
    let pruned = manager.clean(&keep)?;
    for name in &pruned {
        println!("{}", t(MSG::PackageCleanRemoved, lang, Some(&[name])));
    }
    removed_any |= !pruned.is_empty();

    if !removed_any {
        println!("{}", t_simple(MSG::PackageCleanNothing, lang));
    }

    Ok(())
}

/// Clean the current project
pub fn exec() -> PackageResult<()> {
    exec_in(&std::env::current_dir()?)
}
