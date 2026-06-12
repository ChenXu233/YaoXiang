//! 测试 Git 来源的 URL 解析和来源信息
//!
//! 覆盖:
//! - 基本 Git URL 解析
//! - 带 tag 参数的 URL 解析
//! - 带 branch 参数的 URL 解析
//! - 带 rev 参数的 URL 解析
//! - GitSource 的 name 和 kind

use crate::package::source::git::{GitRef, GitSource};
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
