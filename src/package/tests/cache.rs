//! 测试全局包缓存 — 基于 RFC-014 Phase 3（全局缓存设计）
//!
//! 覆盖:
//! - URL / 键的目录名安全化
//! - 缓存条目复制（跳过 `.git`、替换既有目标）
//! - 分支指针的写入与读取（离线回退）
//! - 缓存清理与大小统计
//! - 本地 git 仓库端到端：下载入缓存 → vendor 副本无 `.git` → 缓存复用

use std::path::Path;

use tempfile::TempDir;

use crate::package::cache::{GlobalCache, sanitize_key};
use crate::package::dependency::DependencySpec;
use crate::package::source::git::{GitRef, GitSource, git_command};
use crate::package::source::Source;

// === sanitize_key 测试 ===

/// Helper: test_cache_entry_key_resolves_branch_to_commit 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: 本地 git 仓库 + 缓存根（返回 TempDir 保活）。
fn git_branch_fixture() -> (TempDir, std::path::PathBuf, GlobalCache) {
    // Arrange：本地仓库 + 分支引用
    let tmp = TempDir::new().expect("create temp dir 应成功");
    let repo = make_git_repo(&tmp.path().join("repo"));
    let cache = GlobalCache::with_root(tmp.path().join("cache"));

    (tmp, repo, cache)
}

/// Fixture: 不可达仓库路径（`name` 目录不存在）。
fn unreachable_repo_path(
    tmp: &std::path::Path,
    name: &str,
) -> String {
    tmp.join(name)
        .to_str()
        .expect("路径应可转字符串")
        .to_string()
}

/// Act: 解析分支引用 → 缓存条目键（失败时打印路径与错误）。
fn cache_key_for(
    source: &GitSource,
    cache: &GlobalCache,
    repo: &str,
    branch: &str,
) -> String {
    source
        .cache_entry_key(cache, repo, &GitRef::Branch(branch.to_string()))
        .unwrap_or_else(|e| panic!("解析 {repo}#{branch} 的缓存键应成功: {e}"))
}

/// Assert: commit 键形态（40 位十六进制）与分支指针写入。
fn assert_commit_key_and_pointer(
    key: &String,
    cache: &GlobalCache,
    repo: &str,
) {
    assert_eq!(key.len(), 40, "commit 键应为 40 位 sha");
    assert!(
        key.chars().all(|c| c.is_ascii_hexdigit()),
        "commit 键应为十六进制: {key}"
    );
    assert_eq!(
        cache.read_pointer(repo, "main").expect("路径应可转字符串"),
        Some(key.clone()),
        "解析时应写分支指针"
    );
}

/// Assert: 不可达且无指针时应报错。
fn assert_unreachable_without_pointer_errors(
    source: &GitSource,
    cache: &GlobalCache,
    bogus: &str,
) {
    assert!(
        source
            .cache_entry_key(cache, bogus, &GitRef::Branch("main".to_string()))
            .is_err(),
        "不可达且无指针应报错"
    );
}
/// Fixture: 本地 git 仓库（v1.0.0 / v1.1.0 两个 tag）+ 缓存根 + 两个空 vendor。
fn git_cache_fixture() -> (
    TempDir,
    std::path::PathBuf,
    GlobalCache,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    // Arrange：本地 git 仓库（v1.0.0 / v1.1.0 两个 tag）+ 两个空 vendor
    let tmp = TempDir::new().expect("create temp dir 应成功");
    let repo = make_git_repo(&tmp.path().join("repo"));
    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let vendor = tmp.path().join("vendor");
    std::fs::create_dir_all(&vendor).expect("std::fs::create_dir_all(&vendor) 应成功");
    let vendor2 = tmp.path().join("vendor2");
    std::fs::create_dir_all(&vendor2).expect("std::fs::create_dir_all(&vendor2) 应成功");

    (tmp, repo, cache, vendor, vendor2)
}

/// Act: 下载 `spec` 到 `vendor`（失败时打印错误）。
fn download_ok(
    source: &GitSource,
    spec: &DependencySpec,
    vendor: &std::path::Path,
) -> crate::package::source::ResolvedPackage {
    crate::package::runtime::drive(source.download(spec, vendor))
        .unwrap_or_else(|e| panic!("下载 {spec:?} 应成功: {e}"))
}

/// Assert: 首次下载的版本抉择、vendor 副本与缓存条目。
fn assert_first_download(
    resolved: &crate::package::source::ResolvedPackage,
    repo: &std::path::Path,
    cache: &GlobalCache,
    vendor: &std::path::Path,
) {
    assert_eq!(resolved.version, "1.0.0");
    let dep_dir = vendor.join("fixture-1.0.0");
    assert!(dep_dir.exists(), "vendor 目录应以探测版本命名");
    assert!(dep_dir.join("yaoxiang.toml").exists());
    assert!(dep_dir.join("lib.yx").exists());
    assert_eq!(
        std::fs::read_to_string(dep_dir.join("lib.yx")).expect("读取 fixture 文件应成功"),
        "pub fn one() { 1 }"
    );
    assert!(!dep_dir.join(".git").exists(), "vendor 副本无 .git");
    let entry = cache.git_entry(repo.to_str().expect("路径应可转字符串"), "v1.0.0");
    assert!(entry.exists(), "缓存条目应建立（req 1.0.0 → tag 键）");
    assert!(entry.join(".git").exists(), "缓存条目保留 .git");
}

/// Assert: tilde 范围（~1.1）择优到 v1.1.0 且副本就位。
fn assert_tilde_prefers_v1_1(
    source: &GitSource,
    repo: &std::path::Path,
    vendor: &std::path::Path,
) {
    let spec_1_1 = git_spec("fixture", repo, "~1.1");
    let resolved_1_1 = download_ok(source, &spec_1_1, vendor);
    assert_eq!(resolved_1_1.version, "1.1.0");
    assert!(
        vendor.join("fixture-1.1.0").join("lib.yx").exists(),
        "v1.1.0 副本应就位"
    );
    assert_eq!(
        std::fs::read_to_string(vendor.join("fixture-1.1.0").join("lib.yx"))
            .expect("读取 fixture 文件应成功"),
        "pub fn two() { 2 }"
    );
}

#[test]
fn test_sanitize_key_strips_scheme() {
    // Act / Assert：scheme 与 `git@` 前缀、结尾 `.git` 被剥除
    assert_eq!(
        sanitize_key("https://github.com/user/repo"),
        "github.com-user-repo"
    );
    assert_eq!(
        sanitize_key("git@github.com:user/repo.git"),
        "github.com-user-repo"
    );
    assert_eq!(sanitize_key("ssh://gitlab.com/u/r.git"), "gitlab.com-u-r");
}

#[test]
fn test_sanitize_key_keeps_safe_chars() {
    // Act / Assert：安全字符（字母数字 . _ -）原样保留
    assert_eq!(sanitize_key("v1.2.3"), "v1.2.3");
    assert_eq!(sanitize_key("abc123def"), "abc123def");
    assert_eq!(sanitize_key("my_pkg.name-x"), "my_pkg.name-x");
}

#[test]
fn test_sanitize_key_folds_specials() {
    // Act / Assert：特殊字符折叠为 '-'；连续折叠为单个；首尾 trim
    assert_eq!(sanitize_key("a b/c:d"), "a-b-c-d");
    assert_eq!(sanitize_key("a///b"), "a-b", "连续特殊字符折叠为单个 '-'");
    assert_eq!(sanitize_key("  x  "), "x");
}

// === 复制与指针测试 ===

#[test]
fn test_copy_into_skips_git_and_replaces_dest() {
    // Arrange：缓存条目（含 .git 与源码）+ 预置脏目标
    let tmp = TempDir::new().unwrap();
    let cache = GlobalCache::with_root(tmp.path().join("cache"));

    let entry = tmp.path().join("entry");
    std::fs::create_dir_all(entry.join(".git").join("objects")).unwrap();
    std::fs::write(entry.join(".git").join("HEAD"), "ref: refs/heads/main").unwrap();
    std::fs::create_dir_all(entry.join("src")).unwrap();
    std::fs::write(entry.join("src").join("lib.yx"), "pub fn f() { 1 }").unwrap();
    std::fs::write(entry.join("yaoxiang.toml"), "[package]").unwrap();

    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("stale.junk"), "old").unwrap();

    // Act
    cache.copy_into(&entry, &dest).unwrap();

    // Assert：内容复制、.git 剔除、旧内容被清空
    assert!(dest.join("yaoxiang.toml").exists(), "manifest 应被复制");
    assert!(dest.join("src").join("lib.yx").exists(), "源码应被复制");
    assert!(!dest.join(".git").exists(), ".git 应被剔除");
    assert!(!dest.join("stale.junk").exists(), "旧内容应被清空");
}

#[test]
fn test_pointer_roundtrip() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let cache = GlobalCache::with_root(tmp.path().join("cache"));

    // Act / Assert：未写时 None → 写入后读回一致 → 文件在 git 条目目录
    assert!(
        cache
            .read_pointer("https://github.com/u/r", "main")
            .unwrap()
            .is_none(),
        "未写入指针时读取应返回 None"
    );

    cache
        .write_pointer("https://github.com/u/r", "main", "abc123")
        .unwrap();
    assert_eq!(
        cache
            .read_pointer("https://github.com/u/r", "main")
            .unwrap(),
        Some("abc123".to_string()),
        "写入后应能读回指针"
    );

    assert!(
        cache
            .pointer_file("https://github.com/u/r", "main")
            .exists(),
        "指针文件应位于 git 条目目录"
    );
}

#[test]
fn test_clean_and_size() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("cache");
    let cache = GlobalCache::with_root(root.clone());

    // Act / Assert：不存在的缓存 → None；有内容后清理返回释放字节数并删根
    assert!(
        cache.clean().unwrap().is_none(),
        "缓存不存在时 clean 应返回 None"
    );

    std::fs::create_dir_all(root.join("git").join("entry")).unwrap();
    std::fs::write(root.join("git").join("entry").join("f.txt"), "hello").unwrap();

    assert!(GlobalCache::dir_size(&root) >= 5, "dir_size 应统计内容字节");
    assert!(
        cache.clean().unwrap().is_some(),
        "有内容时 clean 应返回释放量"
    );
    assert!(!root.exists(), "清理后缓存根应被删除");
}

#[test]
fn test_format_size() {
    // Act / Assert：人类可读格式（B / KB / MB）
    assert_eq!(GlobalCache::format_size(512), "512 B");
    assert_eq!(GlobalCache::format_size(2048), "2.00 KB");
    assert_eq!(GlobalCache::format_size(3 * 1024 * 1024), "3.00 MB");
}

#[test]
fn test_from_config_default_has_root() {
    // Arrange / Act：HOME / APPDATA 均存在的常规环境下，from_config 可定位默认根
    if std::env::var("HOME").is_ok() || std::env::var("APPDATA").is_ok() {
        let cache = GlobalCache::from_config().unwrap();

        // Assert
        assert!(!cache.root().as_os_str().is_empty(), "默认缓存根不应为空");
    }
}

// === 本地 git 仓库端到端测试 ===

/// 在目录中创建带两个 tag 的本地 git 仓库，返回仓库路径
fn make_git_repo(dir: &Path) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).expect("std::fs::create_dir_all(dir) 应成功");
    let git = |args: &[&str]| {
        let mut cmd = git_command();
        cmd.arg("-c")
            .arg("user.name=test")
            .arg("-c")
            .arg("user.email=t@t.local");
        cmd.args(args);
        cmd.env("GIT_CONFIG_GLOBAL", dir.join("test-global-gitconfig"));
        cmd.env("GIT_CONFIG_SYSTEM", dir.join("test-system-gitconfig"));
        cmd.env("GIT_CONFIG_NOSYSTEM", "1");
        let out = cmd.output().expect("git 可用");
        assert!(
            out.status.success(),
            "git {:?} 失败: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    };

    // 建仓前置守卫：拒绝在已有仓库上静默 reinit（默认 force=false）
    crate::package::source::git::guard_repo_absent(dir, false).expect("夹具目录不应已是 git 仓库");
    git(&[
        "-C",
        dir.to_str().expect("utf8 repo path"),
        "init",
        "-b",
        "main",
    ]);
    // 建仓后置守卫：仓库必须建在夹具目录里，而不是被定位变量引到别处
    crate::package::source::git::guard_repo_created(dir).expect("git init 应在本目录建出仓库");
    git(&[
        "-C",
        dir.to_str().expect("utf8 repo path"),
        "config",
        "commit.gpgsign",
        "false",
    ]);

    std::fs::write(dir.join("lib.yx"), "pub fn one() { 1 }").expect("写 fixture 文件应成功");
    std::fs::write(
        dir.join("yaoxiang.toml"),
        "[package]\nname = \"fixture\"\nversion = \"1.0.0\"\n",
    )
    .expect("git 命令应成功执行");
    git(&["-C", dir.to_str().expect("utf8 repo path"), "add", "-A"]);
    git(&[
        "-C",
        dir.to_str().expect("utf8 repo path"),
        "commit",
        "-m",
        "one",
    ]);
    git(&["-C", dir.to_str().expect("utf8 repo path"), "tag", "v1.0.0"]);

    std::fs::write(dir.join("lib.yx"), "pub fn two() { 2 }").expect("写 fixture 文件应成功");
    std::fs::write(
        dir.join("yaoxiang.toml"),
        "[package]\nname = \"fixture\"\nversion = \"1.1.0\"\n",
    )
    .expect("git 命令应成功执行");
    git(&["-C", dir.to_str().expect("utf8 repo path"), "add", "-A"]);
    git(&[
        "-C",
        dir.to_str().expect("utf8 repo path"),
        "commit",
        "-m",
        "two",
    ]);
    git(&["-C", dir.to_str().expect("utf8 repo path"), "tag", "v1.1.0"]);

    dir.to_path_buf()
}

fn git_spec(
    name: &str,
    url: &Path,
    version: &str,
) -> DependencySpec {
    DependencySpec {
        name: name.to_string(),
        version: version.to_string(),
        git: Some(url.to_string_lossy().to_string()),
        path: None,
    }
}

#[test]
fn test_git_download_uses_global_cache() {
    // Arrange：本地 git 仓库（v1.0.0 / v1.1.0 两个 tag）+ 空 vendor
    let (_tmp, repo, cache, vendor, vendor2) = git_cache_fixture();
    let source = GitSource::with_cache(cache.clone());
    let spec = git_spec("fixture", &repo, "1.0.0");

    // Act：首次下载（条目缺失 → 克隆入缓存 → 复制到 vendor）
    let resolved = download_ok(&source, &spec, &vendor);

    // Assert：semver 择优命中 v1.0.0；vendor 副本无 .git；缓存条目建立
    assert_first_download(&resolved, &repo, &cache, &vendor);

    // Act & Assert：第二次下载到全新 vendor 命中缓存复用
    let resolved2 = download_ok(&source, &spec, &vendor2);
    assert_eq!(resolved2.version, "1.0.0");
    assert!(
        vendor2.join("fixture-1.0.0").join("lib.yx").exists(),
        "第二次下载应命中缓存复用"
    );

    // Act & Assert：tilde 择优（~1.1 → tag v1.1.0）
    assert_tilde_prefers_v1_1(&source, &repo, &vendor);
}

#[test]
fn test_git_download_tag_pinned() {
    // Arrange：`?tag=v1.0.0` 钉住 ref
    let tmp = TempDir::new().unwrap();
    let repo = make_git_repo(&tmp.path().join("repo"));
    let vendor = tmp.path().join("vendor");
    std::fs::create_dir_all(&vendor).unwrap();

    let source = GitSource::with_cache(GlobalCache::with_root(tmp.path().join("cache")));
    let url = format!("{}?tag=v1.0.0", repo.to_str().unwrap());
    let spec = DependencySpec {
        name: "fixture".to_string(),
        version: "1.0.0".to_string(),
        git: Some(url),
        path: None,
    };

    // Act
    let resolved = crate::package::runtime::drive(source.download(&spec, &vendor)).unwrap();

    // Assert：钉住 ref 不做 semver 择优；版本探测读取该 tag 处 manifest
    assert_eq!(resolved.version, "1.0.0");
    assert!(vendor.join("fixture-1.0.0").exists());
}

#[test]
fn test_cache_entry_key_resolves_branch_to_commit() {
    // Arrange：本地仓库 + 分支引用
    let (tmp, repo, cache) = git_branch_fixture();
    let source = GitSource::new();
    let repo_str = repo.to_str().expect("路径应可转字符串").to_string();

    // Act：分支 → commit 键解析
    let key = cache_key_for(&source, &cache, &repo_str, "main");

    // Assert：键是完整 commit sha（40 位十六进制），指针已写入供离线回退
    assert_commit_key_and_pointer(&key, &cache, &repo_str);

    // 离线回退：不可达 URL + 已有指针 → 返回指针值
    let bogus = unreachable_repo_path(tmp.path(), "nonexistent-repo");
    cache
        .write_pointer(&bogus, "main", "deadbeef")
        .expect("cache.write_pointer(&bogus, \"main\", \"deadbeef\") 应成功");
    let key = cache_key_for(&source, &cache, &bogus, "main");
    assert_eq!(key, "deadbeef", "离线时应回退到指针值");

    // 无指针且不可达 → 报错
    let bogus2 = unreachable_repo_path(tmp.path(), "no-pointer-repo");
    assert_unreachable_without_pointer_errors(&source, &cache, &bogus2);
}
