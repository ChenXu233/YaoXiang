//! GitHub Release 适配层（RFC-014a 决议 1/6/7）
//!
//! github.com 的 git 依赖经此层解析与下载：优先 Release 的 `.yxpkg` 资产
//! （解包校验后入全局缓存，决议 3 的源码包格式），无资产回退 git 克隆
//! （SourceKind 如实报 `Git`）。API 访问带指数退避与 ETag 条件请求缓存
//! （决议 6）；认证取 `$YX_GITHUB_TOKEN`（014a 认证表；login/logout 命令
//! 随官方 Registry 后置，环境变量优先于未来的 credentials.toml）。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::package::cache::{sanitize_key, GlobalCache};
use crate::package::dependency::DependencySpec;
use crate::package::error::{PackageError, PackageResult};
use crate::package::source::git::{GitRef, GitSource};
use crate::package::source::resolver;
use crate::package::source::{ResolvedPackage, Source, SourceKind};
use crate::package::yxpkg;

/// GitHub REST API 版本头（014a 决议 5 的平台侧对应物）
const API_VERSION: &str = "2022-11-28";

/// 默认退避基数：1s, 2s, 4s
const DEFAULT_BACKOFF: Duration = Duration::from_secs(1);

/// 资产下载的宽限超时（20 MiB 慢链路；API 请求用 client 级 60s）
const ASSET_TIMEOUT: Duration = Duration::from_secs(300);

/// GitHub REST API 客户端（指数退避 + ETag 条件请求缓存）
#[derive(Debug, Clone)]
pub(crate) struct GitHubClient {
    http: reqwest::Client,
    api_base: String,
    upload_base: String,
    token: Option<String>,
    cache: GlobalCache,
    backoff_base: Duration,
}

/// Release（解析版本与查找资产共用一次响应）
#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

/// Release 资产（`.yxpkg` 分发件）
#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// 仓库标签（releases 为空时的版本解析回退源）
#[derive(Debug, Deserialize)]
struct GhTag {
    name: String,
}

/// 创建 Release 的响应（publish 所需字段）
#[derive(Debug, Deserialize)]
pub(crate) struct GhCreatedRelease {
    pub(crate) id: u64,
    pub(crate) html_url: String,
}

/// 创建 Release 的请求体
#[derive(Debug, Serialize)]
struct GhCreateRelease<'a> {
    tag_name: &'a str,
    name: &'a str,
    body: &'a str,
}

impl GitHubClient {
    /// 面向 github.com 的客户端（读 `$YX_GITHUB_TOKEN`）
    pub(crate) fn new(cache: GlobalCache) -> Self {
        let api_base = "https://api.github.com".to_string();
        let upload_base = derive_upload_base(&api_base);
        GitHubClient {
            http: crate::package::http::client(false),
            api_base,
            upload_base,
            token: std::env::var("YX_GITHUB_TOKEN")
                .ok()
                .filter(|s| !s.is_empty()),
            cache,
            backoff_base: DEFAULT_BACKOFF,
        }
    }

    /// 测试与 GHES 场景：注入 API 基址与退避基数；请求绕过系统代理
    #[cfg(test)]
    pub(crate) fn for_tests(
        cache: GlobalCache,
        api_base: String,
        backoff_base: Duration,
    ) -> Self {
        let upload_base = derive_upload_base(&api_base);
        GitHubClient {
            http: crate::package::http::client(true),
            api_base,
            upload_base,
            token: None,
            cache,
            backoff_base,
        }
    }

    /// 按 tag 的 Release 是否已存在（publish 版本查重；404 → false）
    pub(crate) async fn release_exists(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> PackageResult<bool> {
        let any: Option<serde::de::IgnoredAny> = self
            .get_json_opt(&format!("/repos/{owner}/{repo}/releases/tags/{tag}"))
            .await?;
        Ok(any.is_some())
    }

    /// tag 是否存在（`/git/ref/tags/{tag}` 端点；publish 要求 tag 先行）
    pub(crate) async fn tag_exists(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> PackageResult<bool> {
        let any: Option<serde::de::IgnoredAny> = self
            .get_json_opt(&format!("/repos/{owner}/{repo}/git/ref/tags/{tag}"))
            .await?;
        Ok(any.is_some())
    }

    /// 创建 Release（POST 不自动重试：非幂等，5xx 后重发可能重复创建）
    pub(crate) async fn create_release(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
        title: &str,
        body: &str,
    ) -> PackageResult<GhCreatedRelease> {
        let payload = GhCreateRelease {
            tag_name: tag,
            name: title,
            body,
        };
        self.post_json(&format!("/repos/{owner}/{repo}/releases"), &payload)
            .await
    }

    /// 上传资产到 Release（octet-stream POST，同样不自动重试）
    pub(crate) async fn upload_release_asset(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
        file_name: &str,
        bytes: Vec<u8>,
    ) -> PackageResult<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/releases/{release_id}/assets?name={file_name}",
            self.upload_base
        );
        let mut req = self
            .http
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream");
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req
            .timeout(ASSET_TIMEOUT)
            .body(bytes)
            .send()
            .await
            .map_err(|e| PackageError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(PackageError::Network(format!(
                "asset upload failed: {status}: {}",
                truncate(&text, 200)
            )));
        }
        Ok(())
    }

    /// POST JSON：带认证与 API 版本头；无自动重试（非幂等）
    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> PackageResult<T> {
        let url = format!("{}{}", self.api_base, path);
        let payload = serde_json::to_vec(body)
            .map_err(|e| PackageError::InvalidManifest(format!("请求序列化失败: {}", e)))?;
        let mut req = self
            .http
            .post(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req
            .body(payload)
            .send()
            .await
            .map_err(|e| PackageError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(PackageError::Network(format!(
                "GitHub API POST {path}: {status}: {}",
                truncate(&text, 200)
            )));
        }
        let text = resp
            .text()
            .await
            .map_err(|e| PackageError::Network(e.to_string()))?;
        parse_json::<T>(&text)?
            .ok_or_else(|| PackageError::Network("empty response body".to_string()))
    }

    /// GET JSON：带 ETag 条件请求缓存与退避重试；404 → None
    async fn get_json_opt<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> PackageResult<Option<T>> {
        let url = format!("{}{}", self.api_base, path);
        let key = sanitize_key(path);
        let etag_file = self.cache.github_dir().join(format!("{}.etag", key));
        let body_file = self.cache.github_dir().join(format!("{}.body", key));
        let cached_etag = std::fs::read_to_string(&etag_file)
            .ok()
            .filter(|s| !s.trim().is_empty());
        let cached_body = std::fs::read_to_string(&body_file).ok();

        let mut attempt = 0u32;
        loop {
            let mut req = self
                .http
                .get(&url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", API_VERSION);
            if let Some(token) = &self.token {
                req = req.bearer_auth(token);
            }
            if let Some(etag) = &cached_etag {
                req = req.header(reqwest::header::IF_NONE_MATCH, etag);
            }
            let resp = req
                .send()
                .await
                .map_err(|e| PackageError::Network(e.to_string()))?;
            let status = resp.status();

            // 304：条件请求命中，用缓存体（GitHub 对 304 不计速率配额——决议 6 的收益点）
            if status == reqwest::StatusCode::NOT_MODIFIED {
                let body = cached_body.ok_or_else(|| {
                    PackageError::Cache("304 未命中本地缓存体（缓存被外部清除）".to_string())
                })?;
                return parse_json(&body);
            }

            if status == reqwest::StatusCode::NOT_FOUND {
                return Ok(None);
            }

            // GitHub 对主速率限制返回 403 + remaining: 0——不重试，如实上报
            if status.as_u16() == 403
                && resp
                    .headers()
                    .get("x-ratelimit-remaining")
                    .and_then(|v| v.to_str().ok())
                    == Some("0")
            {
                let reset = resp
                    .headers()
                    .get("x-ratelimit-reset")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("-");
                return Err(PackageError::RateLimited(format!(
                    "GitHub API rate limit exhausted (resets at unix time {reset}); \
                     set $YX_GITHUB_TOKEN to raise the limit"
                )));
            }

            // 瞬时故障退避：Retry-After 优先，否则 1s/2s/4s 指数
            if attempt < crate::package::http::MAX_RETRIES
                && (status.as_u16() == 429 || status.is_server_error())
            {
                let delay = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(Duration::from_secs)
                    .unwrap_or_else(|| self.backoff_base * 2u32.pow(attempt));
                tokio::time::sleep(delay).await;
                attempt += 1;
                continue;
            }

            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(PackageError::Network(format!(
                    "GitHub API {status}: {}",
                    truncate(&text, 200)
                )));
            }

            // ETag 必须在 text() 消费响应前取出
            let etag = resp
                .headers()
                .get(reqwest::header::ETAG)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp
                .text()
                .await
                .map_err(|e| PackageError::Network(e.to_string()))?;
            // 缓存写入是尽力而为：失败只损失下次条件请求的省额度收益
            if let Some(etag) = &etag {
                let _ = std::fs::write(&etag_file, etag);
            }
            let _ = std::fs::write(&body_file, &body);
            return parse_json(&body);
        }
    }

    /// 下载文件（Release 资产）；瞬时故障同样退避
    async fn download_file(
        &self,
        url: &str,
        dest: &Path,
    ) -> PackageResult<()> {
        crate::package::http::download_file(
            &self.http,
            url,
            self.token.as_deref(),
            dest,
            ASSET_TIMEOUT,
            self.backoff_base,
        )
        .await
    }
}

/// GitHub 来源（github.com git 依赖的快路径）
///
/// 解析：releases → tags 端点；下载：Release `.yxpkg` 资产 → git 克隆回退。
#[derive(Debug, Clone)]
pub struct GitHubSource {
    client: Arc<GitHubClient>,
    fallback: GitSource,
}

impl GitHubSource {
    /// 创建面向 github.com 的来源（读 `$YX_GITHUB_TOKEN`，缓存按用户配置）
    ///
    /// 缓存定位失败（无 HOME/APPDATA）时退回临时目录缓存——下载仍可用，
    /// 只损失跨会话复用。
    pub fn new() -> Self {
        let cache = GlobalCache::from_config().unwrap_or_else(|_| {
            GlobalCache::with_root(std::env::temp_dir().join("yaoxiang-cache-fallback"))
        });
        Self::with_cache(cache)
    }

    /// 以指定缓存创建（程序化使用）
    pub fn with_cache(cache: GlobalCache) -> Self {
        GitHubSource {
            client: Arc::new(GitHubClient::new(cache)),
            fallback: GitSource::new(),
        }
    }

    /// 测试构造：注入缓存、API 基址与毫秒级退避；请求绕过系统代理
    #[cfg(test)]
    pub(crate) fn for_tests(
        cache: GlobalCache,
        api_base: String,
        backoff: Duration,
    ) -> Self {
        GitHubSource {
            client: Arc::new(GitHubClient::for_tests(cache.clone(), api_base, backoff)),
            fallback: GitSource::with_cache(cache),
        }
    }

    /// 最佳匹配标签：releases 为空（纯源码仓库常态）时回退 tags 端点
    async fn best_tag(
        &self,
        owner: &str,
        repo: &str,
        version_req: &str,
    ) -> PackageResult<Option<String>> {
        let releases: Vec<GhRelease> = self
            .client
            .get_json_opt(&format!("/repos/{owner}/{repo}/releases?per_page=100"))
            .await?
            .unwrap_or_default();
        let tags: Vec<String> = if releases.is_empty() {
            let endpoint_tags: Vec<GhTag> = self
                .client
                .get_json_opt(&format!("/repos/{owner}/{repo}/tags?per_page=100"))
                .await?
                .unwrap_or_default();
            endpoint_tags.into_iter().map(|t| t.name).collect()
        } else {
            releases
                .into_iter()
                .filter(|r| !r.draft)
                .map(|r| r.tag_name)
                .collect()
        };
        resolver::select_best_tag(&tags, version_req)
    }

    /// Release `.yxpkg` 资产路径；无 release / 无资产返回 None（走 git 回退）
    async fn try_release_asset(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
        spec: &DependencySpec,
        base_url: &str,
        dest: &Path,
    ) -> PackageResult<Option<ResolvedPackage>> {
        let Some(release) = self
            .client
            .get_json_opt::<GhRelease>(&format!("/repos/{owner}/{repo}/releases/tags/{tag}"))
            .await?
        else {
            return Ok(None);
        };
        let Some(asset) = release.assets.iter().find(|a| a.name.ends_with(".yxpkg")) else {
            return Ok(None);
        };

        // 临时目录承载下载与解包校验（校验通过才进缓存/vendor）
        let scratch = std::env::temp_dir().join(format!(
            "yx-dl-{}-{}",
            std::process::id(),
            sanitize_key(&format!("{owner}/{repo}/{tag}"))
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch)?;

        let extract_dir = match self.stage_asset(asset, &scratch).await {
            Ok(dir) => dir,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&scratch);
                return Err(e);
            }
        };

        let version = detect_manifest_version(&extract_dir)
            .unwrap_or_else(|| tag.strip_prefix('v').unwrap_or(tag).to_string());

        // 全局缓存条目（不可变键：repo+tag），vendor 从缓存复制
        let cache = self.client.cache.clone();
        let entry = cache.github_dir().join(format!(
            "{}-{}",
            sanitize_key(&format!("{owner}/{repo}")),
            sanitize_key(tag)
        ));
        let target_dir = dest.join(format!("{}-{}", spec.name, version));
        let copied = cache
            .copy_into(&extract_dir, &entry)
            .and_then(|_| cache.copy_into(&entry, &target_dir));
        let _ = std::fs::remove_dir_all(&scratch);
        copied?;

        Ok(Some(ResolvedPackage {
            name: spec.name.clone(),
            version,
            source_kind: SourceKind::GitHub,
            source_url: base_url.to_string(),
            local_path: target_dir,
            checksum: None, // vendor 层统一计算目录校验和
        }))
    }

    /// 下载并解包资产到 scratch，返回解包目录（完整性由 yxpkg::unpack 强制）
    async fn stage_asset(
        &self,
        asset: &GhAsset,
        scratch: &Path,
    ) -> PackageResult<PathBuf> {
        let archive = scratch.join(&asset.name);
        self.client
            .download_file(&asset.browser_download_url, &archive)
            .await?;
        let extract_dir = scratch.join("extracted");
        yxpkg::unpack(&archive, &extract_dir)?;
        Ok(extract_dir)
    }
}

impl Default for GitHubSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for GitHubSource {
    fn name(&self) -> &str {
        "github"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::GitHub
    }

    async fn resolve(
        &self,
        spec: &DependencySpec,
    ) -> PackageResult<String> {
        let git_url = spec.git.as_ref().ok_or_else(|| {
            PackageError::InvalidManifest(format!("GitHub 依赖 '{}' 缺少 git 字段", spec.name))
        })?;
        let (base_url, git_ref) = GitSource::parse_git_url(git_url);
        let Some((owner, repo)) = parse_owner_repo(&base_url) else {
            return self.fallback.resolve(spec).await;
        };

        match &git_ref {
            GitRef::Tag(tag) => Ok(tag.strip_prefix('v').unwrap_or(tag).to_string()),
            GitRef::DefaultBranch if spec.version != "*" => {
                let tag = self.best_tag(&owner, &repo, &spec.version).await?;
                Ok(tag
                    .map(|t| t.strip_prefix('v').unwrap_or(&t).to_string())
                    .unwrap_or_else(|| spec.version.clone()))
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
            PackageError::InvalidManifest(format!("GitHub 依赖 '{}' 缺少 git 字段", spec.name))
        })?;
        let (base_url, git_ref) = GitSource::parse_git_url(git_url);
        let Some((owner, repo)) = parse_owner_repo(&base_url) else {
            return self.fallback.download(spec, dest).await;
        };

        // semver 选择：与 GitSource 同语义，只是候选来自 API 而非 ls-remote
        let effective_ref = if matches!(git_ref, GitRef::DefaultBranch) && spec.version != "*" {
            self.best_tag(&owner, &repo, &spec.version)
                .await?
                .map(GitRef::Tag)
                .unwrap_or(git_ref)
        } else {
            git_ref
        };

        if let GitRef::Tag(tag) = &effective_ref {
            if let Some(resolved) = self
                .try_release_asset(&owner, &repo, tag, spec, &base_url, dest)
                .await?
            {
                return Ok(resolved);
            }
        }

        // git 克隆回退：已选定的 tag 固定进 URL，避免回退层重复选版
        let mut fallback_spec = spec.clone();
        if let GitRef::Tag(tag) = &effective_ref {
            if !base_url.contains('?') {
                fallback_spec.git = Some(format!("{}?tag={}", base_url, tag));
            }
        }
        // SourceKind 由回退层如实报 Git（传输渠道是克隆）
        self.fallback.download(&fallback_spec, dest).await
    }
}

/// git URL 是否指向 github.com（`select_source` 的路由判据）
pub(crate) fn is_github_url(url: &str) -> bool {
    parse_owner_repo(url).is_some()
}

/// 从 git URL 解析 `(owner, repo)`；仅识别 github.com 四种形态
///
/// 支持 `https://`、`http://`、`ssh://git@`、`git@…:` 前缀与 `.git`、`/`
/// 结尾；`?query` 已由调用方（或本函数内部）剥离。
pub(crate) fn parse_owner_repo(url: &str) -> Option<(String, String)> {
    let base = url.split('?').next().unwrap_or(url);
    let base = base.trim_end_matches('/');
    let base = base.strip_suffix(".git").unwrap_or(base);

    let after_scheme = base
        .strip_prefix("https://")
        .or_else(|| base.strip_prefix("http://"))
        .or_else(|| base.strip_prefix("ssh://"))
        .unwrap_or(base);
    // 剥离 user@ 认证段（git@github.com:owner/repo）
    let after_auth = after_scheme
        .split_once('@')
        .map(|(_, r)| r)
        .unwrap_or(after_scheme);
    // 取第一个分隔符（https 形态是 /，scp 形态是 :，且 : 可能先于 / 出现）
    let sep = after_auth
        .find('/')
        .into_iter()
        .chain(after_auth.find(':'))
        .min()?;
    let host = &after_auth[..sep];
    let path = &after_auth[sep + 1..];
    if !host.eq_ignore_ascii_case("github.com") {
        return None;
    }
    let mut parts = path.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// 资产上传基址：官方 api.github.com → uploads.github.com；自建/测试同源
fn derive_upload_base(api_base: &str) -> String {
    if let Some(rest) = api_base.strip_prefix("https://api.") {
        format!("https://uploads.{rest}")
    } else if let Some(rest) = api_base.strip_prefix("http://api.") {
        format!("http://uploads.{rest}")
    } else {
        api_base.to_string()
    }
}

/// JSON 响应解析
fn parse_json<T: serde::de::DeserializeOwned>(body: &str) -> PackageResult<Option<T>> {
    serde_json::from_str(body)
        .map(Some)
        .map_err(|e| PackageError::InvalidManifest(format!("GitHub 响应解析失败: {}", e)))
}

/// 截断错误响应体（报错可读）
fn truncate(
    s: &str,
    max: usize,
) -> &str {
    if s.len() <= max {
        s
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}

/// 从解包目录读 `[package].version`
fn detect_manifest_version(dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("yaoxiang.toml")).ok()?;
    let value: toml::Value = toml::from_str(&content).ok()?;
    value
        .get("package")?
        .get("version")?
        .as_str()
        .map(|s| s.to_string())
}
