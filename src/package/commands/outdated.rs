//! `yaoxiang outdated` command - Check for outdated dependencies (RFC-014 Phase 3)

use std::path::Path;

use semver::Version;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::source::git::{GitRef, GitSource};
use crate::package::source::resolver::{parse_version, parse_version_req};
use crate::util::i18n::{t, t_simple, current_lang, MSG};

/// 单条依赖的过时检查结果
#[derive(Debug)]
pub(crate) struct OutdatedEntry {
    /// 依赖名
    pub name: String,
    /// lock 中的当前版本
    pub current: String,
    /// 检查结论
    pub status: OutdatedStatus,
}

/// 检查结论
#[derive(Debug)]
pub(crate) enum OutdatedStatus {
    /// 有更高版本可用
    Outdated { latest: String },
    /// 已是最新
    UpToDate,
    /// 钉住（tag/branch/rev）不参与比较
    Pinned,
    /// 检查失败（离线、无 tag、版本不可解析等）
    Failed(String),
}

/// 检查项目依赖是否有更新版本
///
/// 仅默认分支的 git 依赖参与比较（按 semver 择优对比最新可用 tag）；
/// tag/branch/rev 钉住的依赖报 pinned；path 与 registry 依赖跳过。
pub fn exec_in(project_dir: &Path) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;
    let lock = LockFile::load(project_dir)?;

    let mut specs: Vec<DependencySpec> = DependencySpec::parse_all(&manifest.dependencies);
    specs.extend(DependencySpec::parse_all(&manifest.dev_dependencies));

    if specs.is_empty() {
        println!("{}", t_simple(MSG::PackageNoDeps, current_lang()));
        return Ok(());
    }

    let entries = check_entries(&specs, &lock)?;

    let lang = current_lang();
    let outdated: Vec<&OutdatedEntry> = entries
        .iter()
        .filter(|e| matches!(e.status, OutdatedStatus::Outdated { .. }))
        .collect();

    let mut reported = false;
    if !outdated.is_empty() {
        println!(
            "{}",
            t(
                MSG::PackageOutdatedFound,
                lang,
                Some(&[&outdated.len().to_string()])
            )
        );
        for entry in &outdated {
            if let OutdatedStatus::Outdated { latest } = &entry.status {
                println!(
                    "{}",
                    t(
                        MSG::PackageOutdatedRow,
                        lang,
                        Some(&[&entry.name, &entry.current, latest])
                    )
                );
            }
        }
        reported = true;
    }

    for entry in &entries {
        match &entry.status {
            OutdatedStatus::Pinned => {
                println!(
                    "{}",
                    t(
                        MSG::PackageOutdatedPinned,
                        lang,
                        Some(&[&entry.name, &entry.current])
                    )
                );
                reported = true;
            }
            OutdatedStatus::Failed(reason) => {
                println!(
                    "{}",
                    t(
                        MSG::PackageOutdatedFailed,
                        lang,
                        Some(&[&entry.name, &entry.current, reason])
                    )
                );
                reported = true;
            }
            _ => {}
        }
    }

    if !reported {
        println!("{}", t_simple(MSG::PackageOutdatedNone, lang));
    }

    Ok(())
}

/// 对依赖列表执行过时检查（纯逻辑，供测试）
pub(crate) fn check_entries(
    specs: &[DependencySpec],
    lock: &LockFile,
) -> PackageResult<Vec<OutdatedEntry>> {
    let mut entries = Vec::new();

    for spec in specs {
        // path / registry 依赖不参与
        let Some(git_url) = spec.git.as_ref() else {
            continue;
        };
        // 未安装（不在 lock）的依赖不参与
        let Some(locked) = lock.package.get(&spec.name) else {
            continue;
        };

        let (base_url, git_ref) = GitSource::parse_git_url(git_url);
        if !matches!(git_ref, GitRef::DefaultBranch) {
            entries.push(OutdatedEntry {
                name: spec.name.clone(),
                current: locked.version.clone(),
                status: OutdatedStatus::Pinned,
            });
            continue;
        }

        let status = check_default_branch(&base_url, &spec.version, &locked.version);
        entries.push(OutdatedEntry {
            name: spec.name.clone(),
            current: locked.version.clone(),
            status,
        });
    }

    Ok(entries)
}

/// 默认分支依赖：按 semver 择优对比最新可用 tag
fn check_default_branch(
    base_url: &str,
    version_req: &str,
    locked_version: &str,
) -> OutdatedStatus {
    let req = match parse_version_req(version_req) {
        Ok(req) => req,
        Err(e) => return OutdatedStatus::Failed(e.to_string()),
    };

    let tags = match GitSource::new().list_tags(base_url) {
        Ok(tags) => tags,
        Err(e) => return OutdatedStatus::Failed(e.to_string()),
    };

    let mut best: Option<Version> = None;
    for tag in &tags {
        let vstr = tag.strip_prefix('v').unwrap_or(tag);
        let Ok(v) = parse_version(vstr) else {
            continue;
        };
        if req.matches(&v) && best.as_ref().is_none_or(|b| v > *b) {
            best = Some(v);
        }
    }

    let Some(latest) = best else {
        return OutdatedStatus::Failed("no matching tags".to_string());
    };

    match parse_version(locked_version) {
        Ok(current) if latest > current => OutdatedStatus::Outdated {
            latest: latest.to_string(),
        },
        Ok(_) => OutdatedStatus::UpToDate,
        Err(_) => OutdatedStatus::Failed(format!("invalid locked version '{}'", locked_version)),
    }
}

/// Check all dependencies in the current project
pub fn exec() -> PackageResult<()> {
    exec_in(&std::env::current_dir()?)
}
