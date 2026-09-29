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
    /// 共享依赖声明（2026-09-29 修订，Cargo/uv 先例）：版本的唯一权威来源，
    /// 成员以 `{ workspace = true }` 逐条继承。升共享版本只改根一处。
    #[serde(default)]
    pub dependencies: BTreeMap<String, toml::Value>,
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

/// 合并解析产物
#[derive(Debug, Default)]
pub struct MergedDependencies {
    /// 需要下载的外部依赖（已含继承展开、交集合成后的要求）
    pub fetch: BTreeMap<String, toml::Value>,
    /// 成员引用：依赖名 → `[workspace.members]` key（6c 接路径解析；
    /// 6b 只校验 key 存在并登记 lock）
    pub member_refs: BTreeMap<String, String>,
}

/// 合并全部成员的依赖（RFC-014c §依赖解析 + 2026-09-29 修订）
///
/// 规则：
/// - 收集每个成员的 `[dependencies]` 与 `[dev-dependencies]`（测试可复现，
///   与 Cargo.lock 覆盖 dev 同理）
/// - `{ workspace = true }` → 从根 `[workspace.dependencies]` 继承要求；
///   根未声明该包 → 错误；与其它字段混用 → 错误
/// - `{ workspace = "<key>" }` → 成员引用：key 必须存在，登记 member_refs
/// - 同名包各成员要求求交集（同 git 依赖的 base URL 必须一致）；空交集
///   → 冲突错误，列出冲突来源（成员名 + 要求原文）
pub fn merged_dependencies(ws: &Workspace) -> PackageResult<MergedDependencies> {
    // (成员名, 原始 value) 聚合，同名包对齐检查
    let mut aggregated: BTreeMap<String, Vec<(&str, &toml::Value)>> = BTreeMap::new();
    let mut member_refs: BTreeMap<String, String> = BTreeMap::new();

    for member in &ws.members {
        let deps = member
            .manifest
            .dependencies
            .iter()
            .chain(member.manifest.dev_dependencies.iter());
        for (name, value) in deps {
            // workspace 引用/继承形态识别
            if let toml::Value::Table(table) = value {
                match table.get("workspace") {
                    Some(toml::Value::Boolean(true)) => {
                        if table.len() > 1 {
                            return Err(PackageError::InvalidManifest(format!(
                                "成员 '{}' 的依赖 '{}'：workspace = true 不能与其他字段混用",
                                member.name, name
                            )));
                        }
                        let inherited = ws.manifest.workspace.dependencies.get(name).ok_or_else(|| {
                            PackageError::InvalidManifest(format!(
                                "成员 '{}' 的依赖 '{}' 声明 workspace = true，但根 [workspace.dependencies] 未定义它",
                                member.name, name
                            ))
                        })?;
                        aggregated
                            .entry(name.clone())
                            .or_default()
                            .push((&member.name, inherited));
                        continue;
                    }
                    Some(toml::Value::String(key)) => {
                        if !table.contains_key("version") && table.len() > 1 {
                            return Err(PackageError::InvalidManifest(format!(
                                "成员 '{}' 的依赖 '{}'：workspace 引用不能与其他字段混用",
                                member.name, name
                            )));
                        }
                        if !ws.manifest.workspace.members.contains_key(key) {
                            return Err(PackageError::InvalidManifest(format!(
                                "成员 '{}' 的依赖 '{}' 引用了不存在的 workspace 成员 '{key}'",
                                member.name, name
                            )));
                        }
                        if member_refs.contains_key(name) {
                            return Err(PackageError::InvalidManifest(format!(
                                "依赖 '{name}' 被多个成员以 workspace 引用声明（重复）"
                            )));
                        }
                        member_refs.insert(name.clone(), key.clone());
                        continue;
                    }
                    _ => {}
                }
            }
            aggregated
                .entry(name.clone())
                .or_default()
                .push((&member.name, value));
        }
    }

    // 交集合成与冲突检测
    let mut fetch = BTreeMap::new();
    for (name, entries) in &aggregated {
        // 同名 git 依赖 base URL 必须一致（不同仓库视为不同包）
        let urls: std::collections::BTreeSet<Option<String>> = entries
            .iter()
            .map(|(_, v)| {
                crate::package::dependency::DependencySpec::parse(name, v)
                    .git
                    .map(|u| crate::package::source::git::GitSource::parse_git_url(&u).0)
            })
            .collect();
        if urls.len() > 1 {
            return Err(PackageError::InvalidManifest(format!(
                "依赖 '{name}' 在不同成员中指向不同的 git 仓库，版本无法统一"
            )));
        }

        let mut reqs = Vec::new();
        let mut origin_desc = Vec::new();
        let mut first_value: Option<&toml::Value> = None;
        let mut has_path = false;
        for (member_name, value) in entries {
            let spec = crate::package::dependency::DependencySpec::parse(name, value);
            if first_value.is_none() {
                first_value = Some(value);
            }
            // path 依赖不经版本统一（成员本地实现），跳过交集
            if spec.path.is_some() {
                has_path = true;
                continue;
            }
            match crate::package::source::resolver::parse_version_req(&spec.version) {
                Ok(req) => {
                    reqs.push(req);
                    origin_desc.push(format!("{} 要求 {}", member_name, spec.version));
                }
                Err(e) => {
                    return Err(PackageError::InvalidManifest(format!(
                        "依赖 '{}' 的版本要求无效（来自成员 {}）：{}",
                        name, member_name, e
                    )));
                }
            }
        }

        if has_path {
            // 有 path 形式声明：原样透传第一个（成员本地实现优先，不统一）
            if let Some(v) = first_value {
                fetch.insert(name.clone(), v.clone());
            }
            continue;
        }
        if first_value.is_none() {
            continue;
        }

        let Some(merged_req) = crate::package::source::resolver::intersect_reqs(&reqs) else {
            return Err(PackageError::InvalidManifest(format!(
                "依赖 '{}' 存在版本冲突：\n  {}\n请改用 {{ workspace = true }} 从根 [workspace.dependencies] 继承统一版本",
                name,
                origin_desc.join("\n  ")
            )));
        };
        let merged_string = req_to_manifest_string(&merged_req);
        // 合成要求覆写 version 字段（其余字段保留自第一个声明者）
        let mut value = first_value
            .cloned()
            .unwrap_or_else(|| toml::Value::String(merged_string.clone()));
        match &mut value {
            toml::Value::String(v) => *v = merged_string.clone(),
            toml::Value::Table(t) => {
                t.insert("version".to_string(), toml::Value::String(merged_string));
            }
            _ => {}
        }
        fetch.insert(name.clone(), value);
    }

    Ok(MergedDependencies { fetch, member_refs })
}

/// 合成要求 → manifest 里的版本串（`>=1.0, <2.0` / `=1.2.3` / `*`）
fn req_to_manifest_string(req: &semver::VersionReq) -> String {
    if req.comparators.is_empty() {
        return "*".to_string();
    }
    let mut parts = Vec::new();
    for c in &req.comparators {
        let mut v = format!("{}", c.major);
        if let Some(minor) = c.minor {
            v.push('.');
            v.push_str(&minor.to_string());
            if let Some(patch) = c.patch {
                v.push('.');
                v.push_str(&patch.to_string());
            }
        }
        let op = match c.op {
            semver::Op::Exact => "=",
            semver::Op::GreaterEq => ">=",
            semver::Op::Less => "<",
            _ => "",
        };
        parts.push(format!("{op}{v}"));
    }
    parts.join(", ")
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
