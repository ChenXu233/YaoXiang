//! 工作空间（workspace）支持（RFC-014c）
//!
//! 根 toml 只做三件事：声明成员（`[workspace.members]`，key → 成员 toml
//! 路径）、提供共享 lockfile 与共享 vendor。根不定义 dependencies；每个
//! 成员完全自包含（2026-09-15 决议 2/3：无 workspace 级 build、无成员
//! lockfile；决议 4：无嵌套 workspace；循环依赖按 RFC-029 决策不允许）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::package::error::{PackageError, PackageResult};
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;

/// 工作空间根 manifest（RFC-014c：独立类型，不复用 PackageManifest）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    /// `[workspace]` 段
    pub workspace: WorkspaceConfig,
}

/// `[workspace]` 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// 成员表：key（工作空间内稳定唯一标识）→ 成员 toml 路径（相对根目录）
    #[serde(default)]
    pub members: BTreeMap<String, String>,
}

/// 工作空间成员
#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    /// `[workspace.members]` 的 key（workspace 依赖引用 `{ workspace = key }`
    /// 的目标；不是成员的 `[package].name`——RFC-014c：key 由工作空间控制）
    pub name: String,
    /// 成员包根目录（成员 toml 所在目录）
    pub root: PathBuf,
    /// 成员 manifest
    pub manifest: PackageManifest,
}

/// 已加载的工作空间
#[derive(Debug, Clone)]
pub struct Workspace {
    /// 工作空间根目录
    pub root: PathBuf,
    /// 根 manifest
    pub manifest: WorkspaceManifest,
    /// 成员（按 `[workspace.members]` key 序）
    pub members: Vec<WorkspaceMember>,
    /// 共享 lockfile（根目录唯一，2026-09-15 决议 3）
    pub lock: LockFile,
}

/// toml 清单类型探测（RFC-014c：有 `[workspace]` 段 → 工作空间根，否则包）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestKind {
    /// 包 manifest（成员或独立项目）
    Package,
    /// 工作空间根 manifest
    Workspace,
}

/// 探测目录下 yaoxiang.toml 的类型；文件不存在返回 None
pub fn detect_manifest_kind(dir: &Path) -> Option<ManifestKind> {
    let path = dir.join(crate::package::manifest::MANIFEST_FILE);
    let content = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&content).ok()?;
    if value.get("workspace").is_some() {
        Some(ManifestKind::Workspace)
    } else {
        Some(ManifestKind::Package)
    }
}

/// 从目录加载工作空间根 manifest；无 `[workspace]` 段报 [`PackageError::NotWorkspace`]
pub fn load_workspace_manifest(dir: &Path) -> PackageResult<WorkspaceManifest> {
    let path = dir.join(crate::package::manifest::MANIFEST_FILE);
    if !path.exists() {
        return Err(PackageError::NotWorkspace);
    }
    let content = std::fs::read_to_string(&path)?;
    let manifest: WorkspaceManifest =
        toml::from_str(&content).map_err(|_| PackageError::NotWorkspace)?;
    Ok(manifest)
}

/// 加载完整工作空间：根 manifest + 全部成员 + 共享 lockfile
///
/// 校验（全部硬错误）：
/// - 成员 toml 存在且可解析
/// - 成员自身不得含 `[workspace]` 段（2026-09-15 决议 4：无嵌套）
pub fn load_workspace(root_dir: &Path) -> PackageResult<Workspace> {
    let manifest = load_workspace_manifest(root_dir)?;
    let mut members = Vec::new();

    for (key, toml_path) in &manifest.workspace.members {
        let member_manifest_path = root_dir.join(toml_path);
        if !member_manifest_path.exists() {
            return Err(PackageError::MemberMissing {
                key: key.clone(),
                path: toml_path.clone(),
            });
        }
        let member_root = member_manifest_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| root_dir.to_path_buf());

        // 无嵌套 workspace（决议 4）：成员 toml 含 [workspace] 段即拒绝
        if detect_manifest_kind(&member_root) == Some(ManifestKind::Workspace) {
            return Err(PackageError::NestedWorkspace {
                key: key.clone(),
                path: toml_path.clone(),
            });
        }

        let manifest =
            PackageManifest::load(&member_root).map_err(|e| PackageError::MemberInvalid {
                key: key.clone(),
                reason: e.to_string(),
            })?;

        members.push(WorkspaceMember {
            name: key.clone(),
            root: member_root,
            manifest,
        });
    }

    let lock = LockFile::load(root_dir)?;

    Ok(Workspace {
        root: root_dir.to_path_buf(),
        manifest,
        members,
        lock,
    })
}

/// 从起始目录向上找最近的工作空间根（含 `[workspace]` 段的 yaoxiang.toml）。
///
/// 沿途遇到普通项目根（无 [workspace] 的 yaoxiang.toml）不算命中——工作空间
/// 根与成员包根是两种清单；成员目录向上穿越自身包根继续找是合法形态
/// （成员包根在 workspace 根下）。
pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        if d.join(crate::package::manifest::MANIFEST_FILE).exists()
            && detect_manifest_kind(&d) == Some(ManifestKind::Workspace)
        {
            return Some(d);
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    None
}
