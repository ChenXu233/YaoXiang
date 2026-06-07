//! Token system integration tests
//!
//! 验证 PLDI SRC demo 文件能正确编译运行（或产生预期错误）

use yaoxiang::run;

/// Helper: read .yx file and run it
fn run_yx_file(path: &str) -> Result<(), String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;
    run(&source).map_err(|e| format!("{:?}", e))
}

#[test]
fn test_move_basic() {
    let result = run_yx_file("tests/yaoxiang/08-ownership/move_basic.yx");
    assert!(result.is_ok(), "Move basic demo should compile and run: {:?}", result.err());
}

#[test]
fn test_borrow_immutable() {
    let result = run_yx_file("tests/yaoxiang/08-ownership/borrow_immutable.yx");
    assert!(result.is_ok(), "Immutable borrow demo should compile and run: {:?}", result.err());
}

#[test]
fn test_borrow_mutable() {
    let result = run_yx_file("tests/yaoxiang/08-ownership/borrow_mutable.yx");
    assert!(result.is_ok(), "Mutable borrow demo should compile and run: {:?}", result.err());
}

#[test]
fn test_token_in_struct() {
    let result = run_yx_file("tests/yaoxiang/08-ownership/token_in_struct.yx");
    assert!(result.is_ok(), "Token in struct demo should compile and run: {:?}", result.err());
}

#[test]
fn test_spawn_basic() {
    let result = run_yx_file("tests/yaoxiang/09-concurrency/spawn_basic.yx");
    assert!(result.is_ok(), "Spawn basic demo should compile and run: {:?}", result.err());
}

#[test]
fn test_spawn_ref() {
    let result = run_yx_file("tests/yaoxiang/09-concurrency/spawn_ref.yx");
    assert!(result.is_ok(), "Spawn ref demo should compile and run: {:?}", result.err());
}

#[test]
fn test_destructure() {
    let result = run_yx_file("tests/yaoxiang/09-concurrency/test_destructure.yx");
    assert!(result.is_ok(), "Destructure demo should compile and run: {:?}", result.err());
}

#[test]
fn test_shadow_err() {
    let result = run_yx_file("tests/yaoxiang/10-errors/shadow_err.yx");
    assert!(result.is_err(), "Shadow error demo should produce compile error");
}
