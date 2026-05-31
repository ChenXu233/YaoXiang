//! CheckSession 测试 — 基于 check-improvement 设计规范
//!
//! §6.1: CheckSession 增量检查

use crate::util::diagnostic::session::CheckSession;
use std::fs;

#[test]
fn test_session_create_empty_session() {
    let session = CheckSession::new();
    assert!(
        session.all_files().is_empty(),
        "new session should have no files"
    );
}

#[test]
fn test_session_check_all_single_file() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let file = dir.path().join("test.yx");
    fs::write(
        &file,
        r#"use std.io

main: () -> Void = {
    print("hello")
}
"#,
    )
    .expect("write file");

    let mut session = CheckSession::new();
    let result = session.check_all(&[file]).expect("check all");
    assert_eq!(result.error_count, 0, "valid file should have no errors");
}

#[test]
fn test_session_check_incremental_empty_changes() {
    let mut session = CheckSession::new();
    let result = session.check_incremental(&[]).expect("check incremental");
    assert_eq!(
        result.error_count, 0,
        "empty changes should return no errors"
    );
}

#[test]
fn test_session_check_incremental_after_change() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let file = dir.path().join("test.yx");
    fs::write(
        &file,
        r#"use std.io

main: () -> Void = {
    print("hello")
}
"#,
    )
    .expect("write file");

    let mut session = CheckSession::new();
    session.check_all(&[file.clone()]).expect("initial check");

    // 修改文件后增量检查
    fs::write(
        &file,
        r#"use std.io

main: () -> Void = {
    print("world")
}
"#,
    )
    .expect("rewrite file");

    let result = session
        .check_incremental(&[file])
        .expect("incremental check");
    assert_eq!(
        result.error_count, 0,
        "valid modified file should have no errors"
    );
}
