//! Git 仓库依赖来源
//!
//! 从 Git 仓库（GitHub 等）下载依赖。

use std::path::Path;
use std::process::Command;

use crate::package::dependency::DependencySpec;
use crate::package::error::{PackageError, PackageResult};
use crate::package::source::{ResolvedPackage, Source, SourceKind};

/// Git 仓库引用类型
#[derive(Debug, Clone, PartialEq)]
pub enum GitRef {
    /// 指定标签
    Tag(String),
    /// 指定分支
    Branch(String),
    /// 指定 commit hash
    Rev(String),
    /// 默认分支
    DefaultBranch,
}

/// Git 来源
///
/// 从 Git 仓库克隆并下载依赖。
/// 支持 `?tag=`, `?branch=`, `?rev=` 参数。
#[derive(Debug)]
pub struct GitSource;

impl GitSource {
    /// 创建新的 Git 来源
    pub fn new() -> Self {
        GitSource
    }

    /// 从 Git URL 解析引用信息
    ///
    /// 支持的格式:
    /// - `https://github.com/user/repo` → DefaultBranch
    /// - `https://github.com/user/repo?tag=v1.0.0` → Tag("v1.0.0")
    /// - `https://github.com/user/repo?branch=dev` → Branch("dev")
    /// - `https://github.com/user/repo?rev=abc123` → Rev("abc123")
    pub fn parse_git_url(url: &str) -> (String, GitRef) {
        if let Some(idx) = url.find('?') {
            let base_url = url[..idx].to_string();
            let query = &url[idx + 1..];

            for param in query.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    match key {
                        "tag" => return (base_url, GitRef::Tag(value.to_string())),
                        "branch" => return (base_url, GitRef::Branch(value.to_string())),
                        "rev" => return (base_url, GitRef::Rev(value.to_string())),
                        _ => {}
                    }
                }
            }

            (base_url, GitRef::DefaultBranch)
        } else {
            (url.to_string(), GitRef::DefaultBranch)
        }
    }

    /// 克隆 Git 仓库到目标目录
    fn clone_repo(
        &self,
        url: &str,
        git_ref: &GitRef,
        dest: &Path,
    ) -> PackageResult<()> {
        // 确保目标目录的父目录存在
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 如果已存在则删除
        if dest.exists() {
            std::fs::remove_dir_all(dest)?;
        }

        // 克隆仓库
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth").arg("1");

        match git_ref {
            GitRef::Tag(tag) => {
                cmd.arg("--branch").arg(tag);
            }
            GitRef::Branch(branch) => {
                cmd.arg("--branch").arg(branch);
            }
            GitRef::Rev(_) => {
                // rev 需要完整克隆后 checkout
                cmd.args(["--no-single-branch"]);
            }
            GitRef::DefaultBranch => {}
        }

        cmd.arg(url).arg(dest);

        let output = cmd
            .output()
            .map_err(|e| PackageError::InvalidManifest(format!("无法执行 git 命令: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PackageError::InvalidManifest(format!(
                "Git 克隆失败: {}",
                stderr.trim()
            )));
        }

        // 如果是 rev，需要 checkout 到指定 commit
        if let GitRef::Rev(rev) = git_ref {
            let checkout_output = Command::new("git")
                .arg("-C")
                .arg(dest)
                .arg("checkout")
                .arg(rev)
                .output()
                .map_err(|e| {
                    PackageError::InvalidManifest(format!("无法执行 git checkout: {}", e))
                })?;

            if !checkout_output.status.success() {
                let stderr = String::from_utf8_lossy(&checkout_output.stderr);
                return Err(PackageError::InvalidManifest(format!(
                    "Git checkout 失败: {}",
                    stderr.trim()
                )));
            }
        }

        Ok(())
    }

    /// 获取 Git 仓库中的标签列表
    fn list_tags(
        &self,
        url: &str,
    ) -> PackageResult<Vec<String>> {
        let output = Command::new("git")
            .arg("ls-remote")
            .arg("--tags")
            .arg("--refs")
            .arg(url)
            .output()
            .map_err(|e| PackageError::InvalidManifest(format!("无法执行 git ls-remote: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let tags: Vec<String> = stdout
            .lines()
            .filter_map(|line| line.split("refs/tags/").nth(1).map(|tag| tag.to_string()))
            .collect();

        Ok(tags)
    }

    /// 根据语义化版本要求选择最佳标签
    fn select_best_tag(
        &self,
        tags: &[String],
        version_req: &str,
    ) -> PackageResult<Option<String>> {
        use crate::package::source::resolver::VersionReq;

        let req = VersionReq::parse(version_req)?;

        let mut matching_versions: Vec<(String, crate::package::source::resolver::SemVer)> =
            Vec::new();

        for tag in tags {
            // 去掉 "v" 前缀
            let version_str = tag.strip_prefix('v').unwrap_or(tag);
            if let Ok(version) = crate::package::source::resolver::SemVer::parse(version_str) {
                if req.matches(&version) {
                    matching_versions.push((tag.clone(), version));
                }
            }
        }

        // 选择最高匹配版本
        matching_versions.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(matching_versions.into_iter().next().map(|(tag, _)| tag))
    }

    /// 获取克隆目录中的版本信息
    fn detect_version(
        &self,
        dest: &Path,
    ) -> String {
        // 尝试从 yaoxiang.toml 读取版本
        let manifest_path = dest.join("yaoxiang.toml");
        if manifest_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = toml::from_str::<toml::Value>(&content) {
                    if let Some(version) = manifest
                        .get("package")
                        .and_then(|p| p.get("version"))
                        .and_then(|v| v.as_str())
                    {
                        return version.to_string();
                    }
                }
            }
        }

        // 尝试获取 git 最新 tag
        let output = Command::new("git")
            .arg("-C")
            .arg(dest)
            .arg("describe")
            .arg("--tags")
            .arg("--abbrev=0")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return tag.strip_prefix('v').unwrap_or(&tag).to_string();
            }
        }

        "0.0.0".to_string()
    }
}

impl Default for GitSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for GitSource {
    fn name(&self) -> &str {
        "git"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Git
    }

    fn resolve(
        &self,
        spec: &DependencySpec,
    ) -> PackageResult<String> {
        let git_url = spec.git.as_ref().ok_or_else(|| {
            PackageError::InvalidManifest(format!("Git 依赖 '{}' 缺少 git 字段", spec.name))
        })?;

        let (base_url, git_ref) = Self::parse_git_url(git_url);

        match &git_ref {
            GitRef::Tag(tag) => {
                // 标签就是版本
                Ok(tag.strip_prefix('v').unwrap_or(tag).to_string())
            }
            GitRef::DefaultBranch => {
                // 尝试通过 semver 选择最佳标签
                if spec.version != "*" {
                    let tags = self.list_tags(&base_url)?;
                    if let Some(tag) = self.select_best_tag(&tags, &spec.version)? {
                        Ok(tag.strip_prefix('v').unwrap_or(&tag).to_string())
                    } else {
                        Ok(spec.version.clone())
                    }
                } else {
                    Ok(spec.version.clone())
                }
            }
            _ => Ok(spec.version.clone()),
        }
    }

    fn download(
        &self,
        spec: &DependencySpec,
        dest: &Path,
    ) -> PackageResult<ResolvedPackage> {
        let git_url = spec.git.as_ref().ok_or_else(|| {
            PackageError::InvalidManifest(format!("Git 依赖 '{}' 缺少 git 字段", spec.name))
        })?;

        let (base_url, git_ref) = Self::parse_git_url(git_url);

        // 如果是 semver 匹配且没有指定 ref，尝试选择最佳标签
        let effective_ref = if matches!(git_ref, GitRef::DefaultBranch) && spec.version != "*" {
            let tags = self.list_tags(&base_url)?;
            if let Some(tag) = self.select_best_tag(&tags, &spec.version)? {
                GitRef::Tag(tag)
            } else {
                git_ref
            }
        } else {
            git_ref
        };

        // 计算目标目录
        let version = self.resolve(spec)?;
        let target_dir = dest.join(format!("{}-{}", spec.name, version));

        // 克隆仓库
        self.clone_repo(&base_url, &effective_ref, &target_dir)?;

        // 检测实际版本
        let resolved_version = self.detect_version(&target_dir);

        Ok(ResolvedPackage {
            name: spec.name.clone(),
            version: resolved_version,
            source_kind: SourceKind::Git,
            source_url: base_url,
            local_path: target_dir,
            checksum: None, // 将在 Step 4 中计算
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_url_basic() {
        let (url, git_ref) = GitSource::parse_git_url("https://github.com/user/repo");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(git_ref, GitRef::DefaultBranch);
    }

    #[test]
    fn test_parse_git_url_tag() {
        let (url, git_ref) = GitSource::parse_git_url("https://github.com/user/repo?tag=v1.0.0");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(git_ref, GitRef::Tag("v1.0.0".to_string()));
    }

    #[test]
    fn test_parse_git_url_branch() {
        let (url, git_ref) = GitSource::parse_git_url("https://github.com/user/repo?branch=dev");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(git_ref, GitRef::Branch("dev".to_string()));
    }

    #[test]
    fn test_parse_git_url_rev() {
        let (url, git_ref) = GitSource::parse_git_url("https://github.com/user/repo?rev=abc123");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(git_ref, GitRef::Rev("abc123".to_string()));
    }

    #[test]
    fn test_git_source_name() {
        let source = GitSource::new();
        assert_eq!(source.name(), "git");
        assert_eq!(source.kind(), SourceKind::Git);
    }
}
