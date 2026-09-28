//! vendor 与 lock 文件一致性（RFC-014 §项目模式，2026-09-15 决议）
//!
//! Node 语义：不静默自动安装。vendor 存在但与 `yaoxiang.lock` 不一致时，
//! `run`/`build`/`check` 在发现阶段前置报错并提示 `yaoxiang install`。
//!
//! 一致性判定（按 manifest 依赖逐条核对，均为硬错误）：
//! - lock 缺条目（manifest 有、lock 无）
//! - vendor 缺目录（lock 有、目录无——`<name>-<version>` 未解包）
//! - manifest 要求与 lock 版本不满足（add 后 lock 未刷新等）
//!
//! `std` 接口目录（RFC-037，非包布局）不在核对范围。

use std::path::Path;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::source::resolver::parse_version_req;

/// 一致性检查结果
#[derive(Debug, Default)]
pub struct ConsistencyReport {
    /// lock 缺条目的依赖（manifest 声明了但 lock 没锁）
    pub missing_in_lock: Vec<String>,
    /// vendor 缺目录的依赖（lock 有版本但 `<name>-<ver>/` 不存在）
    pub missing_in_vendor: Vec<(String, String)>,
    /// manifest 版本要求与 lock 版本不满足
    pub requirement_mismatch: Vec<(String, String, String)>,
}

impl ConsistencyReport {
    /// 完全一致
    pub fn is_consistent(&self) -> bool {
        self.missing_in_lock.is_empty()
            && self.missing_in_vendor.is_empty()
            && self.requirement_mismatch.is_empty()
    }

    /// 用户可读的问题清单（诊断/错误消息体）
    pub fn describe(&self) -> String {
        let mut lines = Vec::new();
        for name in &self.missing_in_lock {
            lines.push(format!("  {name}: manifest 已声明但 yaoxiang.lock 未锁定"));
        }
        for (name, version) in &self.missing_in_vendor {
            lines.push(format!("  {name}@{version}: lock 已锁定但 vendor 目录缺失"));
        }
        for (name, req, locked) in &self.requirement_mismatch {
            lines.push(format!("  {name}: manifest 要求 {req}，lock 锁定 {locked}"));
        }
        lines.join("\n")
    }
}

/// 检查项目 vendor 与 lock 的一致性（RFC-014 §项目模式规则）。
///
/// 仅在 vendor 模式（`.yaoxiang/vendor/` 存在）下执行；无 vendor（纯嵌入
/// std 的单文件/未 install 项目）直接返回一致——「manifest 有依赖但从未
/// install」由 Gap2 的 E5001 install 提示负责，不在此处判死（保持单文件
/// 与新项目的零配置可用）。
pub fn check_vendor_lock_consistency(project_root: &Path) -> PackageResult<ConsistencyReport> {
    let vendor_dir = project_root.join(".yaoxiang").join("vendor");
    if !vendor_dir.is_dir() {
        return Ok(ConsistencyReport::default());
    }

    let manifest = PackageManifest::load(project_root)?;
    let lock = LockFile::load(project_root)?;

    let mut deps = manifest.dependencies.clone();
    deps.extend(manifest.dev_dependencies.clone());

    let mut report = ConsistencyReport::default();
    for (name, value) in &deps {
        let spec = DependencySpec::parse(name, value);

        // path 依赖不经核心包源（RFC-014：视同本地模块延伸），跳过
        if spec.path.is_some() {
            continue;
        }

        let Some(locked) = lock.package.get(name) else {
            report.missing_in_lock.push(name.clone());
            continue;
        };

        if !vendor_dir
            .join(format!("{}-{}", name, locked.version))
            .is_dir()
        {
            report
                .missing_in_vendor
                .push((name.clone(), locked.version.clone()));
            continue;
        }

        // 版本要求满足性：git tag/branch/rev 钉住的依赖 lock 版本即探测
        // 版本，manifest req 已在 add/install 时择优，不重复核对；默认
        // 分支（纯 version req）按区间核对
        let pinned = spec
            .git
            .as_ref()
            .map(|url| {
                !matches!(
                    crate::package::source::git::GitSource::parse_git_url(url).1,
                    crate::package::source::git::GitRef::DefaultBranch
                )
            })
            .unwrap_or(false);
        if !pinned {
            if let Ok(req) = parse_version_req(&spec.version) {
                if let Ok(locked_ver) =
                    crate::package::source::resolver::parse_version(&locked.version)
                {
                    if !req.matches(&locked_ver) {
                        report.requirement_mismatch.push((
                            name.clone(),
                            spec.version.clone(),
                            locked.version.clone(),
                        ));
                    }
                }
            }
        }
    }

    Ok(report)
}

/// 在 lock 中查依赖的锁定版本；不存在返回 None
pub fn locked_version(
    project_root: &Path,
    name: &str,
) -> PackageResult<Option<String>> {
    let lock = LockFile::load(project_root)?;
    Ok(lock.package.get(name).map(|d| d.version.clone()))
}

/// 整表读出 lock 版本（包名 → 锁定版本）；lock 缺失/损坏返回空表。
///
/// 供发现阶段逐条 `use` 查询（vendor lock 优先解析）；一致性判定走
/// [`check_vendor_lock_consistency`]，不共用此接口。
pub fn lock_versions(
    project_root: &Path
) -> PackageResult<std::collections::HashMap<String, String>> {
    let lock = LockFile::load(project_root)?;
    Ok(lock
        .package
        .into_iter()
        .map(|(name, dep)| (name, dep.version))
        .collect())
}
