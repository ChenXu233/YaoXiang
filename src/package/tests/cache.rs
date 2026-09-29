//! 测试全局包缓存（RFC-014 Phase 3）
//!
//! 覆盖:
//! - URL / 键的目录名安全化
//! - 缓存条目复制（跳过 `.git`、替换既有目标）
//! - 分支指针的写入与读取（离线回退）
//! - 缓存清理与大小统计
//! - 本地 git 仓库端到端：下载入缓存 → vendor 副本无 `.git` → 缓存复用

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use crate::package::cache::{GlobalCache, sanitize_key};
use crate::package::dependency::DependencySpec;
use crate::package::source::git::{GitRef, GitSource};
use crate::package::source::Source;

// === sanitize_key 测试 ===

#[test]
fn test_sanitize_key_strips_scheme() {
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
    assert_eq!(sanitize_key("v1.2.3"), "v1.2.3");
    assert_eq!(sanitize_key("abc123def"), "abc123def");
    assert_eq!(sanitize_key("my_pkg.name-x"), "my_pkg.name-x");
}

#[test]
fn test_sanitize_key_folds_specials() {
    assert_eq!(sanitize_key("a b/c:d"), "a-b-c-d");
    // 连续特殊字符折叠为单个 '-'
    assert_eq!(sanitize_key("a///b"), "a-b");
    assert_eq!(sanitize_key("  x  "), "x");
}

// === 复制与指针测试 ===

#[test]
fn test_copy_into_skips_git_and_replaces_dest() {
    let tmp = TempDir::new().unwrap();
    let cache = GlobalCache::with_root(tmp.path().join("cache"));

    let entry = tmp.path().join("entry");
    std::fs::create_dir_all(entry.join(".git").join("objects")).unwrap();
    std::fs::write(entry.join(".git").join("HEAD"), "ref: refs/heads/main").unwrap();
    std::fs::create_dir_all(entry.join("src")).unwrap();
    std::fs::write(entry.join("src").join("lib.yx"), "pub fn f() { 1 }").unwrap();
    std::fs::write(entry.join("yaoxiang.toml"), "[package]").unwrap();

    // 预置脏目标，copy_into 应先清空
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("stale.junk"), "old").unwrap();

    cache.copy_into(&entry, &dest).unwrap();

    assert!(dest.join("yaoxiang.toml").exists());
    assert!(dest.join("src").join("lib.yx").exists());
    // .git 被剔除
    assert!(!dest.join(".git").exists());
    // 旧内容被清空
    assert!(!dest.join("stale.junk").exists());
}

#[test]
fn test_pointer_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let cache = GlobalCache::with_root(tmp.path().join("cache"));

    assert!(cache
        .read_pointer("https://github.com/u/r", "main")
        .unwrap()
        .is_none());

    cache
        .write_pointer("https://github.com/u/r", "main", "abc123")
        .unwrap();
    assert_eq!(
        cache
            .read_pointer("https://github.com/u/r", "main")
            .unwrap(),
        Some("abc123".to_string())
    );

    // 指针文件位于 git 条目目录
    assert!(cache
        .pointer_file("https://github.com/u/r", "main")
        .exists());
}

#[test]
fn test_clean_and_size() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("cache");
    let cache = GlobalCache::with_root(root.clone());

    // 缓存不存在 → None
    assert!(cache.clean().unwrap().is_none());

    std::fs::create_dir_all(root.join("git").join("entry")).unwrap();
    std::fs::write(root.join("git").join("entry").join("f.txt"), "hello").unwrap();

    assert!(GlobalCache::dir_size(&root) >= 5);
    assert!(cache.clean().unwrap().is_some());
    assert!(!root.exists());
}

#[test]
fn test_format_size() {
    assert_eq!(GlobalCache::format_size(512), "512 B");
    assert_eq!(GlobalCache::format_size(2048), "2.00 KB");
    assert_eq!(GlobalCache::format_size(3 * 1024 * 1024), "3.00 MB");
}

#[test]
fn test_from_config_default_has_root() {
    // HOME / APPDATA 均存在的常规环境下，from_config 可定位默认根
    if std::env::var("HOME").is_ok() || std::env::var("APPDATA").is_ok() {
        let cache = GlobalCache::from_config().unwrap();
        assert!(!cache.root().as_os_str().is_empty());
    }
}

// === 本地 git 仓库端到端测试 ===

/// 在目录中创建带两个 tag 的本地 git 仓库，返回仓库路径
fn make_git_repo(dir: &Path) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let git = |args: &[&str]| {
        let mut cmd = Command::new("git");
        cmd.arg("-c")
            .arg("user.name=test")
            .arg("-c")
            .arg("user.email=t@t.local");
        cmd.args(args);
        let out = cmd.output().expect("git 可用");
        assert!(
            out.status.success(),
            "git {:?} 失败: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    };

    git(&["-C", dir.to_str().unwrap(), "init", "-b", "main"]);
    git(&[
        "-C",
        dir.to_str().unwrap(),
        "config",
        "commit.gpgsign",
        "false",
    ]);

    std::fs::write(dir.join("lib.yx"), "pub fn one() { 1 }").unwrap();
    std::fs::write(
        dir.join("yaoxiang.toml"),
        "[package]\nname = \"fixture\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    git(&["-C", dir.to_str().unwrap(), "add", "-A"]);
    git(&["-C", dir.to_str().unwrap(), "commit", "-m", "one"]);
    git(&["-C", dir.to_str().unwrap(), "tag", "v1.0.0"]);

    std::fs::write(dir.join("lib.yx"), "pub fn two() { 2 }").unwrap();
    std::fs::write(
        dir.join("yaoxiang.toml"),
        "[package]\nname = \"fixture\"\nversion = \"1.1.0\"\n",
    )
    .unwrap();
    git(&["-C", dir.to_str().unwrap(), "add", "-A"]);
    git(&["-C", dir.to_str().unwrap(), "commit", "-m", "two"]);
    git(&["-C", dir.to_str().unwrap(), "tag", "v1.1.0"]);

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
    let tmp = TempDir::new().unwrap();
    let repo = make_git_repo(&tmp.path().join("repo"));
    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let vendor = tmp.path().join("vendor");
    std::fs::create_dir_all(&vendor).unwrap();

    let source = GitSource::with_cache(cache.clone());
    let spec = git_spec("fixture", &repo, "1.0.0");

    // 首次下载：条目缺失 → 克隆入缓存 → 复制到 vendor
    let resolved = futures::executor::block_on(source.download(&spec, &vendor)).unwrap();

    // semver 择优（req 1.0.0 → tag v1.0.0）；版本探测读取该 tag 处 manifest
    assert_eq!(resolved.version, "1.0.0");
    let dep_dir = vendor.join("fixture-1.0.0");
    assert!(dep_dir.exists());
    assert!(dep_dir.join("yaoxiang.toml").exists());
    assert!(dep_dir.join("lib.yx").exists());
    assert_eq!(
        std::fs::read_to_string(dep_dir.join("lib.yx")).unwrap(),
        "pub fn one() { 1 }"
    );
    // vendor 副本无 .git
    assert!(!dep_dir.join(".git").exists());

    // 缓存条目已建立（req 1.0.0 → tag 键）
    let entry = cache.git_entry(repo.to_str().unwrap(), "v1.0.0");
    assert!(entry.exists());
    assert!(entry.join(".git").exists(), "缓存条目保留 .git");

    // 第二次下载到全新 vendor：命中缓存复用
    let vendor2 = tmp.path().join("vendor2");
    std::fs::create_dir_all(&vendor2).unwrap();
    let resolved2 = futures::executor::block_on(source.download(&spec, &vendor2)).unwrap();
    assert_eq!(resolved2.version, "1.0.0");
    assert!(vendor2.join("fixture-1.0.0").join("lib.yx").exists());

    // tilde 择优（~1.1 → tag v1.1.0）→ 版本探测走 v1.1.0 处 manifest
    let spec_1_1 = git_spec("fixture", &repo, "~1.1");
    let resolved_1_1 = futures::executor::block_on(source.download(&spec_1_1, &vendor)).unwrap();
    assert_eq!(resolved_1_1.version, "1.1.0");
    assert!(vendor.join("fixture-1.1.0").join("lib.yx").exists());
    assert_eq!(
        std::fs::read_to_string(vendor.join("fixture-1.1.0").join("lib.yx")).unwrap(),
        "pub fn two() { 2 }"
    );
}

#[test]
fn test_git_download_tag_pinned() {
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

    let resolved = futures::executor::block_on(source.download(&spec, &vendor)).unwrap();
    // tag 钉住的 ref 不做 semver 择优；版本探测读取该 tag 处 manifest
    assert_eq!(resolved.version, "1.0.0");
    assert!(vendor.join("fixture-1.0.0").exists());
}

#[test]
fn test_cache_entry_key_resolves_branch_to_commit() {
    let tmp = TempDir::new().unwrap();
    let repo = make_git_repo(&tmp.path().join("repo"));
    let cache = GlobalCache::with_root(tmp.path().join("cache"));
    let source = GitSource::new();

    let key = source
        .cache_entry_key(
            &cache,
            repo.to_str().unwrap(),
            &GitRef::Branch("main".to_string()),
        )
        .unwrap();
    // 键是完整 commit sha（40 位十六进制）
    assert_eq!(key.len(), 40);
    assert!(key.chars().all(|c| c.is_ascii_hexdigit()));

    // 指针已写入，供离线回退
    assert_eq!(
        cache.read_pointer(repo.to_str().unwrap(), "main").unwrap(),
        Some(key.clone())
    );

    // 离线回退：不可达 URL + 已有指针 → 返回指针值
    let bogus = tmp
        .path()
        .join("nonexistent-repo")
        .to_str()
        .unwrap()
        .to_string();
    cache.write_pointer(&bogus, "main", "deadbeef").unwrap();
    let key = source
        .cache_entry_key(&cache, &bogus, &GitRef::Branch("main".to_string()))
        .unwrap();
    assert_eq!(key, "deadbeef");

    // 无指针且不可达 → 报错
    let bogus2 = tmp
        .path()
        .join("no-pointer-repo")
        .to_str()
        .unwrap()
        .to_string();
    assert!(source
        .cache_entry_key(&cache, &bogus2, &GitRef::Branch("main".to_string()))
        .is_err());
}
