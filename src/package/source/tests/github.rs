//! GitHub 适配层测试（RFC-014a 决议 6：指数退避 + ETag 条件请求缓存）
//!
//! 用 std TcpListener 起本地 mock API（响应按序消费、逐连接关闭），
//! 覆盖：版本解析（releases → tags 回退）、304 条件请求、5xx 退避重试、
//! 403 速率限制不重试、Release `.yxpkg` 资产下载安装、URL 路由。

use std::time::Duration;

use crate::package::cache::GlobalCache;
use crate::package::dependency::DependencySpec;
use crate::package::source::github::GitHubSource;
use crate::package::source::{select_source, Source, SourceKind};
use crate::package::yxpkg;
use crate::package::runtime;

use crate::package::tests::mock_http::{MockApi, MockResp};

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

fn gh_source(api: &MockApi) -> GitHubSource {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let cache = GlobalCache::with_root(std::env::temp_dir().join(format!(
        "yx-gh-test-{}-{}",
        std::process::id(),
        seq
    )));
    GitHubSource::for_tests(cache, api.url.clone(), Duration::from_millis(1))
}

fn spec(version: &str) -> DependencySpec {
    DependencySpec {
        name: "demo".to_string(),
        version: version.to_string(),
        git: Some("https://github.com/owner/demo".to_string()),
        path: None,
    }
}

#[test]
fn resolve_selects_best_tag_from_releases() {
    let api = MockApi::spawn(vec![json_resp(
        200,
        &releases_json(&["v1.2.0", "v1.3.0", "v2.0.0"]),
    )]);
    let source = gh_source(&api);
    let resolved = runtime::drive(source.resolve(&spec("^1.2.0"))).unwrap();
    assert_eq!(resolved, "1.3.0");
    assert_eq!(api.recorded().len(), 1);
    api.handle.join().unwrap();
}

#[test]
fn resolve_falls_back_to_tags_endpoint_when_no_releases() {
    let api = MockApi::spawn(vec![
        json_resp(200, "[]"),
        json_resp(
            200,
            r#"[{"name":"v0.9.0","commit":{"sha":"abc","url":"u"}}]"#,
        ),
    ]);
    let source = gh_source(&api);
    let resolved = runtime::drive(source.resolve(&spec("^0.9.0"))).unwrap();
    assert_eq!(resolved, "0.9.0");
    assert_eq!(api.recorded().len(), 2, "releases 为空后应回退 tags 端点");
    api.handle.join().unwrap();
}

#[test]
fn etag_conditional_request_hits_cache_on_304() {
    let mut ok = json_resp(200, &releases_json(&["v1.0.0"]));
    ok.headers
        .push(("ETag".to_string(), r#"W/"abc123""#.to_string()));
    let api = MockApi::spawn(vec![
        ok,
        MockResp {
            status: 304,
            headers: Vec::new(),
            body: Vec::new(),
        },
    ]);
    let source = gh_source(&api);

    let first = runtime::drive(source.resolve(&spec("^1.0.0"))).unwrap();
    assert_eq!(first, "1.0.0");
    // 第二次：带 If-None-Match 的条件请求，304 后用缓存体
    let second = runtime::drive(source.resolve(&spec("^1.0.0"))).unwrap();
    assert_eq!(second, "1.0.0");

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
    api.handle.join().unwrap();
}

#[test]
fn server_errors_back_off_then_succeed() {
    let api = MockApi::spawn(vec![
        json_resp(500, "boom"),
        json_resp(500, "boom"),
        json_resp(200, &releases_json(&["v1.0.0"])),
    ]);
    let source = gh_source(&api);
    let resolved = runtime::drive(source.resolve(&spec("^1.0.0"))).unwrap();
    assert_eq!(resolved, "1.0.0");
    assert_eq!(api.recorded().len(), 3, "两次 5xx 后第三次应成功");
    api.handle.join().unwrap();
}

#[test]
fn rate_limit_reported_without_retry() {
    let api = MockApi::spawn(vec![MockResp {
        status: 403,
        headers: vec![
            ("x-ratelimit-remaining".to_string(), "0".to_string()),
            ("x-ratelimit-reset".to_string(), "1700000000".to_string()),
        ],
        body: b"rate limited".to_vec(),
    }]);
    let source = gh_source(&api);
    let err = runtime::drive(source.resolve(&spec("^1.0.0"))).unwrap_err();
    assert!(
        matches!(err, crate::package::PackageError::RateLimited(_)),
        "got: {err}"
    );
    assert_eq!(api.recorded().len(), 1, "速率限制不应重试");
    api.handle.join().unwrap();
}

#[test]
fn download_prefers_release_yxpkg_asset() {
    // 样例项目 → 真实 .yxpkg 字节
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("yaoxiang.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.2.3\"\n",
    )
    .unwrap();
    std::fs::write(project.join("src/lib.yx"), "pub x = 1\n").unwrap();
    let artifact = tmp.path().join("demo-1.2.3.yxpkg");
    yxpkg::pack(&project, &artifact).unwrap();
    let asset_bytes = std::fs::read(&artifact).unwrap();

    // 资产与元数据各起一个 mock：资产 URL 需在 releases 响应生成前确定。
    // /releases/tags/{tag} 端点返回单个对象（非数组）。
    let asset_api = MockApi::spawn(vec![MockResp {
        status: 200,
        headers: Vec::new(),
        body: asset_bytes,
    }]);
    let release_body = r#"{"tag_name":"v1.2.3","draft":false,"prerelease":false,"assets":[{"name":"demo-1.2.3.yxpkg","browser_download_url":"ASSET_URL","digest":null}]}"#
        .replace(
            "ASSET_URL",
            &format!("{}/assets/demo-1.2.3.yxpkg", asset_api.url),
        );
    let meta_api = MockApi::spawn(vec![json_resp(200, &release_body)]);

    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let source = GitHubSource::for_tests(
        cache.clone(),
        meta_api.url.clone(),
        Duration::from_millis(1),
    );

    let mut tagged = spec("*");
    tagged.git = Some("https://github.com/owner/demo?tag=v1.2.3".to_string());
    let vendor = tmp.path().join("vendor");
    let resolved = runtime::drive(source.download(&tagged, &vendor)).unwrap();

    assert_eq!(
        resolved.source_kind,
        SourceKind::GitHub,
        "资产路径应报 GitHub"
    );
    assert_eq!(resolved.version, "1.2.3", "版本取自包内 yaoxiang.toml");
    assert!(vendor.join("demo-1.2.3").join("src/lib.yx").exists());
    assert!(
        cache
            .root()
            .join("github")
            .join("owner-demo-v1.2.3")
            .exists(),
        "解包结果应入全局缓存"
    );
    meta_api.handle.join().unwrap();
    asset_api.handle.join().unwrap();
}

#[test]
fn routing_github_urls_go_to_github_adapter() {
    let mk = |git: Option<&str>| DependencySpec {
        name: "x".to_string(),
        version: "*".to_string(),
        git: git.map(|s| s.to_string()),
        path: None,
    };
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
    assert_eq!(select_source(&mk(None)).kind(), SourceKind::Registry);
}

#[test]
fn owner_repo_parsing() {
    use crate::package::source::github::parse_owner_repo;
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
    assert_eq!(parse_owner_repo("https://gitlab.com/owner/repo"), None);
    assert_eq!(parse_owner_repo("https://github.com/owner"), None);
}

/// publish --github 全流程：版本查重（404）→ tag 校验（200）→ 创建 Release
/// （201）→ 上传资产（201）
#[test]
fn publish_github_flow_creates_release_and_uploads() {
    use crate::package::commands::publish::publish_github_flow;
    use crate::package::source::github::GitHubClient;

    let tmp = tempfile::tempdir().unwrap();
    let api = MockApi::spawn(vec![
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
    ]);
    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let client = GitHubClient::for_tests(cache, api.url.clone(), Duration::from_millis(1));

    let artifact = tmp.path().join("demo-1.0.0.yxpkg");
    std::fs::write(&artifact, b"PKGBYTES").unwrap();

    let url = runtime::drive(publish_github_flow(
        &client, "owner", "demo", "demo", "1.0.0", "demo pkg", &artifact,
    ))
    .unwrap();
    assert_eq!(url, "https://github.com/owner/demo/releases/tag/v1.0.0");
    let reqs = api.recorded();
    assert_eq!(reqs.len(), 4, "release 查重/tag 校验/创建/上传共 4 次请求");
    // 查重与 tag 校验是 GET，创建与上传是非幂等 POST（重试策略差异的依据）
    assert_eq!(reqs[0].method, "GET");
    assert_eq!(reqs[1].method, "GET");
    assert_eq!(reqs[2].method, "POST");
    assert_eq!(reqs[3].method, "POST");
    api.handle.join().unwrap();
}

/// 版本已存在时 publish 必须拒绝（014a 发布前校验 2：版本号不可复用）
#[test]
fn publish_github_flow_rejects_existing_release() {
    use crate::package::commands::publish::publish_github_flow;
    use crate::package::source::github::GitHubClient;

    let tmp = tempfile::tempdir().unwrap();
    let api = MockApi::spawn(vec![json_resp(
        200,
        r#"{"tag_name":"v1.0.0","draft":false,"prerelease":false,"assets":[]}"#,
    )]);
    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let client = GitHubClient::for_tests(cache, api.url.clone(), Duration::from_millis(1));

    let artifact = tmp.path().join("demo-1.0.0.yxpkg");
    std::fs::write(&artifact, b"PKGBYTES").unwrap();

    let err = runtime::drive(publish_github_flow(
        &client, "owner", "demo", "demo", "1.0.0", "d", &artifact,
    ))
    .unwrap_err();
    assert!(
        matches!(err, crate::package::PackageError::VersionAlreadyExists(_)),
        "got: {err}"
    );
    assert_eq!(api.recorded().len(), 1, "查重命中后不应继续");
    api.handle.join().unwrap();
}
