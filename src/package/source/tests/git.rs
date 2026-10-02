//! 测试 Git 来源的 URL 解析和来源信息
//!
//! 覆盖:
//! - 基本 Git URL 解析
//! - 带 tag 参数的 URL 解析
//! - 带 branch 参数的 URL 解析
//! - 带 rev 参数的 URL 解析
//! - GitSource 的 name 和 kind
//! - ls-remote 失败的显式报错语义（RFC-014 Phase 3：不得静默降级）
//! - git 子进程剥离仓库定位环境变量

use crate::package::source::git::{GitRef, GitSource, git_command};
use crate::package::source::{Source, SourceKind};

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

#[test]
fn test_list_tags_unreachable_url_errors_loudly() {
    // Arrange：不存在的本地仓库路径——git ls-remote 必然失败
    let source = GitSource::new();

    // Act
    let result = source.list_tags("/nonexistent/yx-probe/repo");

    // Assert：失败必须显式报错。静默返回空列表会让 semver 择优落空、
    // 回退到默认分支，把钉住的版本请求静默解析成 HEAD 的内容
    //（CI 上曾因此把 "1.0.0" 解析成 v1.1.0 的 manifest）
    assert!(
        result.is_err(),
        "ls-remote 失败应显式报错，实际: {result:?}"
    );
}

#[test]
fn test_git_command_scrubs_repo_location_env() {
    // Arrange：模拟 `git commit` 钩子环境注入的仓库定位变量——
    // 子 git 继承后会把 fixture 的 init/clone 重定向到宿主仓库
    let repo_location_vars = [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_NAMESPACE",
        "GIT_CONFIG_PARAMETERS",
    ];

    // Act
    let cmd = git_command();

    // Assert：定位类变量全部被显式移除（get_envs 以 (key, None) 呈现）
    for key in repo_location_vars {
        let target = std::ffi::OsStr::new(key);
        let scrubbed = cmd.get_envs().any(|(k, v)| k == target && v.is_none());
        assert!(scrubbed, "{key} 应被 git_command 剥离");
    }
}
