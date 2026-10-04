//! 依赖下载器
//!
//! 提供统一的依赖下载接口，协调不同来源的下载。

use std::collections::BTreeMap;
use std::path::Path;

use crate::package::build::TrustDecision;
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
pub async fn fetch_all(
    project_dir: &Path,
    deps: &BTreeMap<String, toml::Value>,
    lock: &mut LockFile,
    trust: &TrustDecision,
) -> PackageResult<FetchResult> {
    let manager = VendorManager::new(project_dir).with_trust(trust.clone());
    manager.ensure_vendor_dir()?;

    let specs = DependencySpec::parse_all(deps);
    // 相对 path 以项目根为基准解析成绝对路径：工作空间模式 project_dir 是
    // ws 根（成员的 `./foo-src` 相对根成立），普通项目 project_dir 即项目
    // 目录。此前相对路径按进程 CWD 解析，依赖 exec_in 的调用巧合才正确。
    let specs: Vec<DependencySpec> = specs
        .into_iter()
        .map(|mut s| {
            if let Some(p) = &s.path {
                if Path::new(p).is_relative() {
                    s.path = Some(project_dir.join(p).to_string_lossy().into_owned());
                }
            }
            s
        })
        .collect();
    let mut result = FetchResult {
        installed: Vec::new(),
        skipped: Vec::new(),
        failed: Vec::new(),
    };

    for spec in &specs {
        // 注册表依赖（无 git/path）：RFC-014a 尚未实现——明确失败，
        // 不再写锁文件、不再打印"已安装"（旧行为：静默跳过 + 锁文件记账）
        if spec.git.is_none() && spec.path.is_none() {
            result.failed.push((
                spec.name.clone(),
                "registry source not implemented (RFC-014a); \
                 use a git or path dependency"
                    .to_string(),
            ));
            continue;
        }

        // git/github 依赖：lock 完整性校验通过 → 跳过。path 依赖不走此
        // 快路径：来源在本机随时可改，每次 install 重做落盘复制，保证
        // vendor 与工作副本一致（#411：复制进 vendor 的定案语义）。
        if spec.path.is_none() {
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
        }

        match manager.install_dependency(spec).await {
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
