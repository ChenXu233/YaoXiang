//! GitHub 适配层测试 — 基于 RFC-014a 决议 6（指数退避 + ETag 条件请求缓存）
//! + 决议 1/3（Release `.yxpkg` 资产优先分发）
//!
//! 用 std TcpListener 起本地 mock API（响应按序消费、逐连接关闭），
//! 覆盖：版本解析（releases → tags 回退）、304 条件请求、5xx 退避重试、
//! 403 速率限制不重试、Release `.yxpkg` 资产下载安装、URL 路由、
//! publish --github 流程（查重/tag 校验/创建/上传）。

use std::time::Duration;

use crate::package::cache::GlobalCache;
use crate::package::commands::publish::publish_github_flow;
use crate::package::dependency::DependencySpec;
use crate::package::runtime;
use crate::package::source::github::{parse_owner_repo, GitHubClient, GitHubSource};
use crate::package::source::{select_source, Source, SourceKind};
use crate::package::tests::mock_http::{MockApi, MockResp};
use crate::package::yxpkg;

fn json_resp(
    status: u16,
    body: &str,
) -> MockResp {
    MockResp::json(status, body)
}

fn releases_json(tags: &[&str]) -> String {
    let items: Vec<String> = tags
        .iter()
        .map(|t| format!(r#"{{"tag_name":"{t}","draft":false,"prerelease":false,"assets":[]}}"#))
        .collect();
    format!("[{}]", items.join(","))
}

fn gh_source(api: &MockApi) -> (GitHubSource, TestCacheRoot) {
    let root = TestCacheRoot::new();
    let source = GitHubSource::for_tests(
        root.cache().clone(),
        api.url.clone(),
        Duration::from_millis(1),
    );
    (source, root)
}

/// 测试用缓存根守卫：Drop 时删除本次自建的根目录（不碰任何非本测试创建的目录）。
struct TestCacheRoot {
    cache: GlobalCache,
}

impl TestCacheRoot {
    /// 建一个跨次运行唯一的缓存根（见 unique_test_cache）。
    fn new() -> Self {
        Self {
            cache: unique_test_cache(),
        }
    }

    fn cache(&self) -> &GlobalCache {
        &self.cache
    }
}

impl Drop for TestCacheRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.cache.root());
    }
}

/// Helper: 每次调用都唯一的测试缓存根。
///
/// 旧实现用 yx-gh-test-<pid>-<seq>，pid 与序号跨进程可复用：%TEMP% 下遗留的同名
/// 目录（尤其其中的 .etag）会让首次请求带上 If-None-Match，使 304 用例随机失败。
/// 此处加入纳秒时间戳与进程随机熵，保证缓存根跨次运行唯一。
fn unique_test_cache() -> GlobalCache {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let entropy = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    GlobalCache::with_root(std::env::temp_dir().join(format!(
        "yx-gh-test-{}-{}-{}-{}",
        std::process::id(),
        seq,
        nanos,
        entropy
    )))
}

fn spec(version: &str) -> DependencySpec {
    DependencySpec {
        name: "demo".to_string(),
        version: version.to_string(),
        git: Some("https://github.com/owner/demo".to_string()),
        path: None,
    }
}

/// Fixture: 304 Not Modified 空响应。
fn not_modified() -> MockResp {
    MockResp {
        status: 304,
        headers: Vec::new(),
        body: Vec::new(),
    }
}

/// Act: 解析 version_req 并返回版本号（失败时打印请求要求与底层错误）。
fn resolve_ok(
    source: &GitHubSource,
    version_req: &str,
) -> String {
    runtime::drive(source.resolve(&spec(version_req)))
        .unwrap_or_else(|e| panic!("解析 {version_req} 应成功: {e}"))
}

/// Assert: 304 条件请求的端点、首跳无 If-None-Match、次跳带 ETag。
fn assert_etag_revalidation(api: &MockApi) {
    let reqs = api.recorded();
    assert_eq!(reqs.len(), 2);
    assert_eq!(
        reqs[0].path, "/repos/owner/demo/releases?per_page=100",
        "版本解析应命中 releases 端点"
    );
    assert_eq!(reqs[0].if_none_match, None);
    assert_eq!(
        reqs[1].if_none_match.as_deref(),
        Some(r#"W/"abc123""#),
        "第二次请求必须带 If-None-Match（决议 6）"
    );
}

/// Fixture: 在 tmp 下建样例 demo 项目（1.2.3）并打包，返回 .yxpkg 资产字节。
fn packed_demo_project(tmp: &tempfile::TempDir) -> Vec<u8> {
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(project.join("src")).expect("create fixture src dir");
    std::fs::write(
        project.join("yaoxiang.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.2.3\"\n",
    )
    .expect("write fixture manifest");
    std::fs::write(project.join("src/lib.yx"), "pub x = 1\n").expect("write fixture source");
    let artifact = tmp.path().join("demo-1.2.3.yxpkg");
    yxpkg::pack(&project, &artifact).expect("pack fixture artifact");
    std::fs::read(&artifact).expect("read packed fixture")
}

/// Fixture: 固定资产字节的 mock 资产端点。
fn asset_mock_api(bytes: Vec<u8>) -> MockApi {
    MockApi::spawn(vec![MockResp {
        status: 200,
        headers: Vec::new(),
        body: bytes,
    }])
}

/// Fixture: /releases/tags/{tag} 的单个 release 响应体（资产 URL 指向 asset_api）。
fn release_body_for(asset_api: &MockApi) -> String {
    r#"{"tag_name":"v1.2.3","draft":false,"prerelease":false,"assets":[{"name":"demo-1.2.3.yxpkg","browser_download_url":"ASSET_URL","digest":null}]}"#
        .replace(
            "ASSET_URL",
            &format!("{}/assets/demo-1.2.3.yxpkg", asset_api.url),
        )
}

/// Fixture: 带 tag 的 github 依赖 spec。
fn tagged_spec(tag: &str) -> DependencySpec {
    let mut tagged = spec("*");
    tagged.git = Some(format!("https://github.com/owner/demo?tag={tag}"));
    tagged
}

/// Act: 下载 tagged 到 vendor（失败时打印依赖与错误）。
fn download_ok(
    source: &GitHubSource,
    tagged: &DependencySpec,
    vendor: &std::path::Path,
) -> crate::package::source::ResolvedPackage {
    runtime::drive(source.download(tagged, vendor))
        .unwrap_or_else(|e| panic!("下载 {tagged:?} 应成功: {e}"))
}

/// Assert: Release 资产路径的下载结果（来源种类 / 版本 / vendor / 全局缓存条目）。
fn assert_release_asset_download(
    resolved: &crate::package::source::ResolvedPackage,
    vendor: &std::path::Path,
    cache: &GlobalCache,
) {
    assert_eq!(
        resolved.source_kind,
        SourceKind::GitHub,
        "资产路径应报 GitHub"
    );
    assert_eq!(resolved.version, "1.2.3", "版本取自包内 yaoxiang.toml");
    assert!(
        vendor.join("demo-1.2.3").join("src/lib.yx").exists(),
        "解包结果应复制进 vendor"
    );
    assert!(
        cache
            .root()
            .join("github")
            .join("owner-demo-v1.2.3")
            .exists(),
        "解包结果应入全局缓存"
    );
}

/// Fixture: publish --github 全流程的 4 个按序响应（查重 404 / tag 200 / 创建 201 / 上传 201）。
fn publish_flow_responses() -> Vec<MockResp> {
    vec![
        json_resp(404, r#"{"message":"Not Found"}"#),
        json_resp(
            200,
            r#"{"ref":"refs/tags/v1.0.0","object":{"sha":"abc","type":"commit"}}"#,
        ),
        json_resp(
            201,
            r#"{"id":42,"html_url":"https://github.com/owner/demo/releases/tag/v1.0.0"}"#,
        ),
        json_resp(201, "{}"),
    ]
}

/// Act: 跑 publish 全流程并返回 Release URL（失败时打印版本与错误）。
fn publish_ok(
    client: &GitHubClient,
    artifact: &std::path::Path,
) -> String {
    runtime::drive(publish_github_flow(
        client, "owner", "demo", "demo", "1.0.0", "demo pkg", artifact,
    ))
    .unwrap_or_else(|e| panic!("publish 1.0.0 应成功: {e}"))
}

/// Assert: 4 次请求及其方法序列（查重/tag 校验 GET，创建/上传 POST）。
fn assert_publish_request_methods(api: &MockApi) {
    let reqs = api.recorded();
    assert_eq!(reqs.len(), 4, "release 查重/tag 校验/创建/上传共 4 次请求");
    assert_eq!(reqs[0].method, "GET");
    assert_eq!(reqs[1].method, "GET");
    assert_eq!(reqs[2].method, "POST");
    assert_eq!(reqs[3].method, "POST");
}

#[test]
fn test_resolve_selects_best_tag_from_releases() {
    // Arrange：releases 含 v1.2.0 / v1.3.0 / v2.0.0
    let api = MockApi::spawn(vec![json_resp(
        200,
        &releases_json(&["v1.2.0", "v1.3.0", "v2.0.0"]),
    )]);
    let (source, _cache_root) = gh_source(&api);

    // Act
    let resolved = runtime::drive(source.resolve(&spec("^1.2.0"))).unwrap();

    // Assert：满足 ^1.2.0 的最高版本，且只打一次 releases 端点
    assert_eq!(resolved, "1.3.0");
    assert_eq!(api.recorded().len(), 1);
    api.handle.join().unwrap();
}

#[test]
fn test_resolve_falls_back_to_tags_endpoint_when_no_releases() {
    // Arrange：releases 为空（纯源码仓库常态）→ tags 端点回退
    let api = MockApi::spawn(vec![
        json_resp(200, "[]"),
        json_resp(
            200,
            r#"[{"name":"v0.9.0","commit":{"sha":"abc","url":"u"}}]"#,
        ),
    ]);
    let (source, _cache_root) = gh_source(&api);

    // Act
    let resolved = runtime::drive(source.resolve(&spec("^0.9.0"))).unwrap();

    // Assert：解析结果正确，且确实回退了端点
    assert_eq!(resolved, "0.9.0");
    assert_eq!(api.recorded().len(), 2, "releases 为空后应回退 tags 端点");
    api.handle.join().unwrap();
}

#[test]
fn test_etag_conditional_request_hits_cache_on_304() {
    // Arrange：首次响应带 ETag；第二次同一端点返回 304
    let mut ok = json_resp(200, &releases_json(&["v1.0.0"]));
    ok.headers
        .push(("ETag".to_string(), r#"W/"abc123""#.to_string()));
    let api = MockApi::spawn(vec![ok, not_modified()]);
    let (source, _cache_root) = gh_source(&api);

    // Act：同一要求解析两次
    let first = resolve_ok(&source, "^1.0.0");
    let second = resolve_ok(&source, "^1.0.0");

    // Assert：结果一致；第二次请求必须带 If-None-Match（决议 6，
    // GitHub 对 304 不计速率配额——缓存的意义所在）
    assert_eq!(first, "1.0.0");
    assert_eq!(second, "1.0.0");
    assert_etag_revalidation(&api);
    api.handle.join().unwrap();
}

#[test]
fn test_server_errors_back_off_then_succeed() {
    // Arrange：两次 5xx 后一次成功（退避基数测试注入 1ms）
    let api = MockApi::spawn(vec![
        json_resp(500, "boom"),
        json_resp(500, "boom"),
        json_resp(200, &releases_json(&["v1.0.0"])),
    ]);
    let (source, _cache_root) = gh_source(&api);

    // Act
    let resolved = runtime::drive(source.resolve(&spec("^1.0.0"))).unwrap();

    // Assert：重试后成功，共 3 次请求
    assert_eq!(resolved, "1.0.0");
    assert_eq!(api.recorded().len(), 3, "两次 5xx 后第三次应成功");
    api.handle.join().unwrap();
}

#[test]
fn test_rate_limit_reported_without_retry() {
    // Arrange：403 + x-ratelimit-remaining: 0（GitHub 主速率限制形态）
    let api = MockApi::spawn(vec![MockResp {
        status: 403,
        headers: vec![
            ("x-ratelimit-remaining".to_string(), "0".to_string()),
            ("x-ratelimit-reset".to_string(), "1700000000".to_string()),
        ],
        body: b"rate limited".to_vec(),
    }]);
    let (source, _cache_root) = gh_source(&api);

    // Act
    let err = runtime::drive(source.resolve(&spec("^1.0.0"))).unwrap_err();

    // Assert：如实报 RateLimited，不重试（重发无意义）
    assert!(
        matches!(err, crate::package::PackageError::RateLimited(_)),
        "got: {err}"
    );
    assert_eq!(api.recorded().len(), 1, "速率限制不应重试");
    api.handle.join().unwrap();
}

#[test]
fn test_download_prefers_release_yxpkg_asset() {
    // Arrange：样例项目 → 真实 .yxpkg 字节；/releases/tags/{tag} 返回单个对象
    let tmp = tempfile::tempdir().unwrap();
    let asset_bytes = packed_demo_project(&tmp);
    let asset_api = asset_mock_api(asset_bytes);
    let meta_api = MockApi::spawn(vec![json_resp(200, &release_body_for(&asset_api))]);
    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let source = GitHubSource::for_tests(
        cache.clone(),
        meta_api.url.clone(),
        Duration::from_millis(1),
    );
    let tagged = tagged_spec("v1.2.3");

    // Act
    let vendor = tmp.path().join("vendor");
    let resolved = download_ok(&source, &tagged, &vendor);

    // Assert：走 GitHub 资产路径、版本取自包内 manifest、解包结果入全局缓存
    assert_release_asset_download(&resolved, &vendor, &cache);
    meta_api.handle.join().unwrap();
    asset_api.handle.join().unwrap();
}

#[test]
fn test_select_source_routes_github_urls_to_github_adapter() {
    // Arrange：github.com（https / scp 形态）、非 github git 主机、裸版本
    let mk = |git: Option<&str>| DependencySpec {
        name: "x".to_string(),
        version: "*".to_string(),
        git: git.map(|s| s.to_string()),
        path: None,
    };

    // Act / Assert
    assert_eq!(
        select_source(&mk(Some("https://github.com/owner/repo"))).kind(),
        SourceKind::GitHub
    );
    assert_eq!(
        select_source(&mk(Some("git@github.com:owner/repo.git"))).kind(),
        SourceKind::GitHub
    );
    assert_eq!(
        select_source(&mk(Some("https://gitlab.com/owner/repo"))).kind(),
        SourceKind::Git
    );
    assert_eq!(
        select_source(&mk(None)).kind(),
        SourceKind::Registry,
        "裸版本依赖归 Registry 占位"
    );
}

#[test]
fn test_owner_repo_parsing_covers_url_forms() {
    // Act / Assert：github.com 四种形态 + 拒绝非 github 主机与残缺路径
    assert_eq!(
        parse_owner_repo("https://github.com/owner/repo"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_owner_repo("https://github.com/owner/repo.git?tag=v1.0.0"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_owner_repo("git@github.com:owner/repo"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_owner_repo("https://gitlab.com/owner/repo"),
        None,
        "非 github.com 主机不路由到适配层"
    );
    assert_eq!(parse_owner_repo("https://github.com/owner"), None);
}

/// publish --github 全流程：版本查重（404）→ tag 校验（200）→ 创建 Release
/// （201）→ 上传资产（201）
#[test]
fn test_publish_github_flow_creates_release_and_uploads() {
    // Arrange
    let tmp = tempfile::tempdir().unwrap();
    let api = MockApi::spawn(publish_flow_responses());
    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let client = GitHubClient::for_tests(cache, api.url.clone(), Duration::from_millis(1));
    let artifact = tmp.path().join("demo-1.0.0.yxpkg");
    std::fs::write(&artifact, b"PKGBYTES").expect("write fixture artifact");

    // Act
    let url = publish_ok(&client, &artifact);

    // Assert：4 次请求（查重/tag 校验是 GET，创建/上传是非幂等 POST）
    assert_eq!(url, "https://github.com/owner/demo/releases/tag/v1.0.0");
    assert_publish_request_methods(&api);
    api.handle.join().unwrap();
}

/// 版本已存在时 publish 必须拒绝（014a 发布前校验 2：版本号不可复用）
#[test]
fn test_publish_github_flow_rejects_existing_release() {
    // Arrange：查重端点直接命中已有 Release
    let tmp = tempfile::tempdir().unwrap();
    let api = MockApi::spawn(vec![json_resp(
        200,
        r#"{"tag_name":"v1.0.0","draft":false,"prerelease":false,"assets":[]}"#,
    )]);
    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let client = GitHubClient::for_tests(cache, api.url.clone(), Duration::from_millis(1));
    let artifact = tmp.path().join("demo-1.0.0.yxpkg");
    std::fs::write(&artifact, b"PKGBYTES").expect("write fixture artifact");

    // Act
    let err = runtime::drive(publish_github_flow(
        &client, "owner", "demo", "demo", "1.0.0", "d", &artifact,
    ))
    .unwrap_err();

    // Assert：报 VersionAlreadyExists 且查重命中后不再继续
    assert!(
        matches!(err, crate::package::PackageError::VersionAlreadyExists(_)),
        "got: {err}"
    );
    assert_eq!(api.recorded().len(), 1, "查重命中后不应继续");
    api.handle.join().unwrap();
}
