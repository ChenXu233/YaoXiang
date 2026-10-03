//! Git 仓库依赖来源
//!
//! 从 Git 仓库（GitHub 等）下载依赖。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::package::cache::GlobalCache;
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

/// git 子进程的稳定工作目录。
///
/// 进程级 cwd 可能被并行的 chdir 测试（如 init 的 CwdGuard）移进随后
/// 被删除的临时目录：子进程 spawn 时 getcwd 失败，报 "Unable to read
/// current working directory"。显式钉到系统临时根（生命周期覆盖进程）
/// 免疫该竞态；git 命令的路径参数均为绝对/显式路径，cwd 无业务含义。
fn stable_cwd() -> PathBuf {
    std::env::temp_dir()
}

/// 构造剥离仓库定位环境变量的 git 子进程 Command。
///
/// 实测（git 2.55）：`git commit` **只在 linked worktree 中**向钩子环境导出
/// 仓库定位变量（`GIT_DIR` + `GIT_INDEX_FILE`；主检出里 `GIT_DIR` 为空，
/// `GIT_WORK_TREE` 不出现）。钩子链里 spawn 的 git（本仓 pre-commit 会跑
/// `cargo test`，夹具的建仓首当其冲）会继承它们，于是 `git init <dir>` 实际
/// reinit 的是被指向的真实仓库——又因 cwd 不是该仓库的工作区，git 判定它
/// bare，把 `core.bare = true` 写进**所有 worktree 共享的** `.git/config`，
/// 整个检出随即 `git status` 报 "must be run in a work tree"。这正是
/// 「一开 worktree 就中招、主检出单独提交却没事」的原因。
///
/// 仓库定位一律走显式参数（`-C` / URL / 目标路径），定位类环境变量在此统一
/// 剥离；配合 [`stable_cwd`] 的中性 cwd，即使漏清某个变量，git 也无法按 cwd
/// 发现调用者的仓库。
pub(crate) fn git_command() -> Command {
    let mut cmd = Command::new("git");

    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_NAMESPACE",
        "GIT_CONFIG_PARAMETERS",
    ] {
        cmd.env_remove(key);
    }

    cmd
}

/// 夹具建仓前置守卫：`dir` 已是 git 仓库时报错，`force` 才放行。
///
/// git 对已有仓库是**静默 reinit**（只打印 "Reinitialized existing Git
/// repository"）：仓库定位一旦被环境变量重定向，破坏就发生在别处且没有任何
/// 提示。夹具一律先用本函数把这种静默 reinit 变成显式失败——`force=false`
/// 是默认，`true` 只留给确需原地重建的场景（对应命令行的 `-f` 出口）。
///
/// # Errors
///
/// `dir` 下已存在 `.git` 且 `force == false` 时返回
/// [`std::io::ErrorKind::AlreadyExists`]。
#[cfg(test)]
pub(crate) fn guard_repo_absent(
    dir: &Path,
    force: bool,
) -> std::io::Result<()> {
    if force {
        return Ok(());
    }
    let git_dir = dir.join(".git");
    if git_dir.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "{} 已是 git 仓库，拒绝 reinit（确需重建请传 force=true）",
                git_dir.display()
            ),
        ));
    }
    Ok(())
}

/// 夹具建仓后置守卫：`dir` 必须真的出现 `.git` 目录。
///
/// 仓库定位被重定向时，建仓命令不报错却什么都没建在 `dir` 里。本断言把这种
/// 「建到别处」立刻变成失败，而不是等到后续 add/commit 莫名报错才暴露。
///
/// # Errors
///
/// `dir/.git` 不是目录时返回 [`std::io::ErrorKind::NotFound`]。
#[cfg(test)]
pub(crate) fn guard_repo_created(dir: &Path) -> std::io::Result<()> {
    let git_dir = dir.join(".git");
    if !git_dir.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "git init 未在 {} 建出仓库（仓库定位可能被环境变量重定向）",
                dir.display()
            ),
        ));
    }
    Ok(())
}

/// Git 来源
///
/// 从 Git 仓库克隆并下载依赖。
/// 支持 `?tag=`, `?branch=`, `?rev=` 参数。
/// 下载经由全局缓存（RFC-014 Phase 3），vendor 目录从缓存复制。
#[derive(Debug, Clone)]
pub struct GitSource {
    /// 注入的缓存（测试用）；None 时按用户配置惰性定位
    cache: Option<GlobalCache>,
}

impl GitSource {
    /// 创建新的 Git 来源
    pub fn new() -> Self {
        GitSource { cache: None }
    }

    /// 以指定缓存创建 Git 来源（测试与程序化使用）
    pub fn with_cache(cache: GlobalCache) -> Self {
        GitSource { cache: Some(cache) }
    }

    /// 获取本次下载使用的缓存实例
    fn effective_cache(&self) -> PackageResult<GlobalCache> {
        match &self.cache {
            Some(c) => Ok(c.clone()),
            None => GlobalCache::from_config(),
        }
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

        // 克隆仓库。--no-local 强制本地路径也走传输路径：本地硬链接克隆
        // 会忽略 --depth，且 git 2.55 下 checkout 间歇性失败
        //（"fatal: this operation must be run in a work tree"）
        let mut cmd = git_command();
        cmd.current_dir(stable_cwd());
        cmd.arg("clone").arg("--no-local").arg("--depth").arg("1");

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
            let checkout_output = git_command()
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
    pub fn list_tags(
        &self,
        url: &str,
    ) -> PackageResult<Vec<String>> {
        let output = git_command()
            .current_dir(stable_cwd())
            .arg("ls-remote")
            .arg("--tags")
            .arg("--refs")
            .arg(url)
            .output()
            .map_err(|e| PackageError::InvalidManifest(format!("无法执行 git ls-remote: {}", e)))?;

        if !output.status.success() {
            // 不得静默降级为空列表：空列表会让 semver 择优落空、回退到
            // 默认分支，把"请求 1.0.0"静默解析成 HEAD 的内容
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PackageError::InvalidManifest(format!(
                "git ls-remote 失败: {}",
                stderr.trim()
            )));
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
        super::resolver::select_best_tag(tags, version_req)
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
        let output = git_command()
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

    async fn resolve(
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

    async fn download(
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

        // 全局缓存优先（RFC-014 Phase 3）：条目键不可变（tag/rev/commit）
        let cache = self.effective_cache()?;
        let entry_key = self.cache_entry_key(&cache, &base_url, &effective_ref)?;
        let entry = cache.git_entry(&base_url, &entry_key);
        if !entry.exists() {
            self.clone_repo(&base_url, &effective_ref, &entry)?;
        }

        // 版本探测在缓存条目上进行（条目保留 .git，git describe 兜底可用）
        let resolved_version = self.detect_version(&entry);

        // vendor 目录以探测到的真实版本命名（lock/清理/完整性校验同源）
        let target_dir = dest.join(format!("{}-{}", spec.name, resolved_version));

        // vendor 副本不含 .git（缓存条目才是 git 实体）
        cache.copy_into(&entry, &target_dir)?;

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

impl GitSource {
    /// 计算全局缓存条目键（RFC-014 Phase 3）
    ///
    /// tag/rev 直接作键（不可变）；分支与默认分支经 `git ls-remote` 解析到
    /// commit 后以 commit 作键（分支引用可移动，commit 不可变），并写指针
    /// 文件；`ls-remote` 失败（离线）时回退到上次成功解析的指针。
    pub(crate) fn cache_entry_key(
        &self,
        cache: &GlobalCache,
        base_url: &str,
        git_ref: &GitRef,
    ) -> PackageResult<String> {
        match git_ref {
            GitRef::Tag(tag) => Ok(tag.clone()),
            GitRef::Rev(rev) => Ok(rev.clone()),
            GitRef::Branch(branch) => {
                let pattern = format!("refs/heads/{}", branch);
                match Self::remote_head(base_url, &pattern) {
                    Ok(commit) => {
                        cache.write_pointer(base_url, branch, &commit)?;
                        Ok(commit)
                    }
                    Err(e) => cache.read_pointer(base_url, branch)?.ok_or(e),
                }
            }
            GitRef::DefaultBranch => match Self::remote_head(base_url, "HEAD") {
                Ok(commit) => {
                    cache.write_pointer(base_url, "HEAD", &commit)?;
                    Ok(commit)
                }
                Err(e) => cache.read_pointer(base_url, "HEAD")?.ok_or(e),
            },
        }
    }

    /// `git ls-remote` 查询远程引用指向的 commit
    fn remote_head(
        url: &str,
        pattern: &str,
    ) -> PackageResult<String> {
        let output = git_command()
            .current_dir(stable_cwd())
            .arg("ls-remote")
            .arg(url)
            .arg(pattern)
            .output()
            .map_err(|e| PackageError::InvalidManifest(format!("无法执行 git ls-remote: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PackageError::InvalidManifest(format!(
                "git ls-remote 失败: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().next())
            .map(|commit| commit.to_string())
            .ok_or_else(|| {
                PackageError::InvalidManifest(format!(
                    "git ls-remote 未返回引用: {} {}",
                    url, pattern
                ))
            })
    }
}
