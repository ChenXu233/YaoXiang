//! 依赖下载器
//!
//! 提供统一的依赖下载接口，协调不同来源的下载。

use std::collections::BTreeMap;
use std::path::Path;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::source::ResolvedPackage;
use crate::package::vendor::VendorManager;

/// 下载结果
#[derive(Debug)]
pub struct FetchResult {
    /// 成功安装的包
    pub installed: Vec<ResolvedPackage>,
    /// 已存在（跳过）的包
    pub skipped: Vec<(String, String)>,
    /// 安装失败的包
    pub failed: Vec<(String, String)>,
}

/// 批量下载依赖
///
/// 从 manifest 的依赖列表下载所有依赖到 vendor 目录，并更新锁文件。
pub fn fetch_all(
    project_dir: &Path,
    deps: &BTreeMap<String, toml::Value>,
    lock: &mut LockFile,
) -> PackageResult<FetchResult> {
    let manager = VendorManager::new(project_dir);
    manager.ensure_vendor_dir()?;

    let specs = DependencySpec::parse_all(deps);
    let mut result = FetchResult {
        installed: Vec::new(),
        skipped: Vec::new(),
        failed: Vec::new(),
    };

    for spec in &specs {
        // 跳过本地依赖（不需要下载）
        if spec.path.is_some() && spec.git.is_none() {
            // 本地依赖只需解析，不下载
            lock.lock_dependency_full(&spec.name, &spec.version, "path", None);
            result
                .skipped
                .push((spec.name.clone(), spec.version.clone()));
            continue;
        }

        // 注册表依赖（无 git/path）在 Phase 3 前仅记录到锁文件
        if spec.git.is_none() && spec.path.is_none() {
            let source = crate::package::source::select_source(spec);
            let resolved_version = source
                .resolve(spec)
                .unwrap_or_else(|_| spec.version.clone());
            lock.lock_dependency_full(&spec.name, &resolved_version, "registry", None);
            result.skipped.push((spec.name.clone(), resolved_version));
            continue;
        }

        // 检查锁文件中是否已有此依赖且完整性校验通过
        if let Some(locked) = lock.package.get(&spec.name) {
            if let Some(ref checksum) = locked.checksum {
                if manager
                    .verify_integrity(&spec.name, &locked.version, checksum)
                    .unwrap_or(false)
                {
                    result
                        .skipped
                        .push((spec.name.clone(), locked.version.clone()));
                    continue;
                }
            }
        }

        match manager.install_dependency(spec) {
            Ok(resolved) => {
                let source_kind_str = resolved.source_kind.to_string();
                lock.lock_dependency_full(
                    &resolved.name,
                    &resolved.version,
                    &source_kind_str,
                    resolved.checksum.as_deref(),
                );
                result.installed.push(resolved);
            }
            Err(e) => {
                result.failed.push((spec.name.clone(), e.to_string()));
            }
        }
    }

    // 删除锁文件中不再需要的依赖
    let dep_names: std::collections::HashSet<String> =
        specs.iter().map(|s| s.name.clone()).collect();
    lock.package.retain(|name, _| dep_names.contains(name));

    Ok(result)
}
