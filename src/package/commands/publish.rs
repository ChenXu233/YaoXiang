//! `yaoxiang publish` 命令（RFC-014a）
//!
//! 官方 Registry 无限期后置（决议 1/2）：裸 `publish` 报错并指路；
//! `--dry-run` 完成「校验 → workspace 引用替换（014c 6d）→ 打包」全链；
//! `--github` 继而创建 GitHub Release 并上传 `.yxpkg` 资产。
//!
//! 发布前校验（014a）：name/version/description 必填、Release 不得重复、
//! tag 必须已存在（Cargo 同款语义：打 tag 是用户的事，publish 不代create）。

use std::path::Path;

use crate::package::cache::GlobalCache;
use crate::package::error::{PackageError, PackageResult};
use crate::package::manifest::{PackageManifest, MANIFEST_FILE};
use crate::package::source::github::GitHubClient;
use crate::package::workspace::{find_workspace_root, load_workspace};
use crate::package::yxpkg;
use crate::util::i18n::{t, t_simple, current_lang, MSG};

/// publish 参数
#[derive(Debug, Clone, Copy, Default)]
pub struct PublishArgs {
    /// 只做校验与本地打包，不发布
    pub dry_run: bool,
    /// 发布为 GitHub Release（`.yxpkg` 资产）
    pub github: bool,
}

/// 在给定目录执行 publish
pub fn exec_in(
    project_dir: &Path,
    args: PublishArgs,
) -> PackageResult<()> {
    let manifest = PackageManifest::load(project_dir)?;
    validate_for_publish(&manifest)?;

    if !args.dry_run && !args.github {
        eprintln!(
            "{}",
            t_simple(MSG::PackagePublishRegistryDeferred, current_lang())
        );
        return Err(PackageError::RegistryDeferred);
    }

    // 6d：打包时物化 workspace 引用替换（磁盘 manifest 不动）
    let manifest_toml = publishing_manifest_toml(project_dir, &manifest)?;
    let artifact = project_dir
        .join("target")
        .join("yxpkg")
        .join(yxpkg::artifact_name(
            &manifest.package.name,
            &manifest.package.version,
        ));
    yxpkg::pack_with_manifest(project_dir, &artifact, &manifest_toml)?;
    let sha = crate::package::vendor::checksum::compute_file_checksum(&artifact)?;

    let lang = current_lang();
    println!(
        "{}",
        t(
            MSG::PackagePublishPacked,
            lang,
            Some(&[&artifact.display().to_string(), &sha]),
        )
    );

    if args.dry_run {
        return Ok(());
    }

    // --github
    // token 预检：缺凭据时在首次网络请求前给出可读错误
    // （client 构造时自读 `$YX_GITHUB_TOKEN` 挂到请求上）
    if std::env::var("YX_GITHUB_TOKEN")
        .ok()
        .filter(|s| !s.is_empty())
        .is_none()
    {
        return Err(PackageError::AuthFailed(t_simple(
            MSG::PackagePublishTokenMissing,
            current_lang(),
        )));
    }
    let (owner, repo) = resolve_repo(project_dir, &manifest)?;
    let client = GitHubClient::new(GlobalCache::from_config()?);
    let name = &manifest.package.name;
    let version = &manifest.package.version;
    let description = manifest.package.description.as_deref().unwrap_or_default();
    let url = crate::package::runtime::drive(publish_github_flow(
        &client,
        &owner,
        &repo,
        name,
        version,
        description,
        &artifact,
    ))?;
    println!(
        "{}",
        t(MSG::PackagePublishGitHubDone, current_lang(), Some(&[&url]))
    );
    Ok(())
}

/// 在当前目录执行 publish
pub fn exec(args: PublishArgs) -> PackageResult<()> {
    exec_in(&std::env::current_dir()?, args)
}

/// 发布前校验（014a）：name/version/description 必填
fn validate_for_publish(manifest: &PackageManifest) -> PackageResult<()> {
    if manifest
        .package
        .description
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return Err(PackageError::InvalidManifest(format!(
            "发布要求 [package] 提供 description（RFC-014a 发布前校验）：{}",
            MANIFEST_FILE
        )));
    }
    Ok(())
}

/// 发布形态的 manifest 内容：workspace 引用替换为版本/继承依赖（RFC-014c 6d）
///
/// 无 workspace 引用时原样返回磁盘内容；有引用则定位工作空间根，取成员
/// `[package].version` 加 `^` 前缀（版本的权威来源是成员 toml，不是 Registry）。
fn publishing_manifest_toml(
    project_dir: &Path,
    manifest: &PackageManifest,
) -> PackageResult<String> {
    let has_ws_ref = manifest
        .dependencies
        .values()
        .chain(manifest.dev_dependencies.values())
        .any(|v| v.get("workspace").is_some());
    if !has_ws_ref {
        return Ok(std::fs::read_to_string(project_dir.join(MANIFEST_FILE))?);
    }

    let Some(ws_root) = find_workspace_root(project_dir) else {
        return Err(PackageError::InvalidManifest(
            "依赖含 workspace 引用但未找到工作空间根".to_string(),
        ));
    };
    let ws = load_workspace(&ws_root)?;

    let member_version = |key: &str| -> PackageResult<String> {
        ws.members
            .iter()
            .find(|m| m.name == key)
            .map(|m| format!("^{}", m.manifest.package.version))
            .ok_or_else(|| PackageError::MemberInvalid {
                key: key.to_string(),
                reason: "workspace 引用目标不存在".to_string(),
            })
    };

    let mut out = manifest.clone();
    for deps in [&mut out.dependencies, &mut out.dev_dependencies] {
        for (dep_name, value) in deps.iter_mut() {
            let Some(ws_val) = value.get("workspace") else {
                continue;
            };
            match ws_val {
                // 成员引用：替换为被依赖成员的 ^version
                toml::Value::String(key) => {
                    *value = toml::Value::String(member_version(key)?);
                }
                // 继承：物化根 [workspace.dependencies] 的声明（014c 修订）
                toml::Value::Boolean(true) => {
                    let inherited = ws
                        .manifest
                        .workspace
                        .dependencies
                        .get(dep_name)
                        .cloned()
                        .ok_or_else(|| PackageError::MemberInvalid {
                            key: dep_name.clone(),
                            reason: "继承引用在 [workspace.dependencies] 无声明".to_string(),
                        })?;
                    *value = inherited;
                }
                _ => {
                    return Err(PackageError::InvalidManifest(format!(
                        "依赖 '{dep_name}' 的 workspace 字段类型非法（应为成员 key 字符串或 true）"
                    )));
                }
            }
        }
    }
    toml::to_string_pretty(&out).map_err(|e| PackageError::Toml(e.to_string()))
}

/// 解析发布目标仓库：`[package].repository` 优先，回退 git remote origin
fn resolve_repo(
    project_dir: &Path,
    manifest: &PackageManifest,
) -> PackageResult<(String, String)> {
    if let Some(repo) = &manifest.package.repository {
        if let Some(owner_repo) = crate::package::source::github::parse_owner_repo(repo) {
            return Ok(owner_repo);
        }
    }
    if let Ok(output) = std::process::Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .args(["remote", "get-url", "origin"])
        .output()
    {
        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Some(owner_repo) = crate::package::source::github::parse_owner_repo(&url) {
                return Ok(owner_repo);
            }
        }
    }
    Err(PackageError::PublishTarget(t_simple(
        MSG::PackagePublishRepoUnresolved,
        current_lang(),
    )))
}

/// GitHub 发布流：版本不重复 → tag 已存在 → 创建 Release → 上传资产
///
/// client 携带认证（构造时读 `$YX_GITHUB_TOKEN`）；参数化以便测试注入 mock。
pub(crate) async fn publish_github_flow(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    name: &str,
    version: &str,
    description: &str,
    artifact: &Path,
) -> PackageResult<String> {
    let tag = format!("v{version}");

    if client.release_exists(owner, repo, &tag).await? {
        return Err(PackageError::VersionAlreadyExists(t(
            MSG::PackagePublishVersionExists,
            current_lang(),
            Some(&[&tag]),
        )));
    }
    if !client.tag_exists(owner, repo, &tag).await? {
        return Err(PackageError::PublishTarget(t(
            MSG::PackagePublishTagMissing,
            current_lang(),
            Some(&[&tag]),
        )));
    }
    let release = client
        .create_release(
            owner,
            repo,
            &tag,
            &format!("{name} v{version}"),
            description,
        )
        .await?;
    let file_name = artifact
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .ok_or_else(|| {
            PackageError::InvalidManifest(format!("非法产物路径: {}", artifact.display()))
        })?;
    let bytes = std::fs::read(artifact)?;
    client
        .upload_release_asset(owner, repo, release.id, &file_name, bytes)
        .await?;
    Ok(release.html_url)
}
