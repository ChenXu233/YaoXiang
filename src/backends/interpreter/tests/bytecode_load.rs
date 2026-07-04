//! 字节码文件 (.42) 加载和执行集成测试
//!
//! 验证：
//! - 编译 .yx → .42 → 加载并执行 .42 文件（端到端循环）
use std::io::Write;
use std::path::PathBuf;

/// 编译 .yx 源文件为 .42 字节码文件，然后加载并执行它。
#[test]
fn test_run_bytecode_file() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let source_path = dir.path().join("test.yx");
    let bytecode_path = dir.path().join("test.42");

    // 写入最小的 YaoXiang 源文件
    std::fs::write(
        &source_path,
        "main = () => { print(\"hello from bytecode\") }",
    )
    .expect("write source file");

    // 编译为 .42 字节码
    crate::build_bytecode_with_options(&source_path, &bytecode_path, false)
        .expect("build bytecode");

    // 加载 .42 文件
    let bytecode_file = crate::middle::passes::codegen::BytecodeFile::load(&bytecode_path)
        .expect("load bytecode file");

    // 转换为 BytecodeModule
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

    // 创建解释器并执行
    let interp = crate::backends::interpreter::Interpreter::new();
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);
    executor
        .execute_module(&bytecode_module)
        .expect("execute bytecode module");
}

/// 无效魔数的 .42 文件应产生清晰的错误信息。
#[test]
fn test_run_bytecode_file_invalid_magic() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let path = dir.path().join("bad.42");

    // 写入全零字节（错误魔数）
    let mut f = std::fs::File::create(&path).expect("create file");
    f.write_all(&[0u8; 32]).expect("write data");
    drop(f);

    let err = crate::middle::passes::codegen::BytecodeFile::load(&path)
        .expect_err("expected error for invalid magic");

    let msg = format!("{}", err);
    assert!(
        msg.contains("invalid magic"),
        "error should mention 'invalid magic', got: {msg}"
    );
}

/// 正确魔数但版本不匹配的 .42 文件应产生清晰的错误信息。
#[test]
fn test_run_bytecode_file_version_mismatch() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let path = dir.path().join("old.42");

    let mut f = std::fs::File::create(&path).expect("create file");
    // 写入有效魔数（YXBC = 0x59584243 大端序），但版本号为 99
    f.write_all(&[0x59, 0x58, 0x42, 0x43]).expect("write magic");
    f.write_all(&[99u8, 0, 0, 0]).expect("write version 99");
    drop(f);

    let err = crate::middle::passes::codegen::BytecodeFile::load(&path)
        .expect_err("expected error for version mismatch");

    let msg = format!("{}", err);
    assert!(
        msg.contains("unsupported"),
        "error should mention 'unsupported', got: {msg}"
    );
}

/// 验证 .yx 源文件路径不受 .42 检测分支影响。
#[test]
fn test_run_yx_file_not_affected() {
    // 此测试验证 .yx 扩展名不会触发 .42 分支。
    // 我们只需确认一个不存在的 .yx 文件会产生 "Failed to read" 错误，
    // 而不是 "failed to load bytecode file" 错误。
    let path = PathBuf::from("/nonexistent/path/file.yx");
    let err = crate::util::diagnostic::run_file_with_diagnostics(&path, false, "embedded", 0)
        .expect_err("expected error for nonexistent .yx file");

    let msg = format!("{}", err);
    assert!(
        msg.contains("Failed to read file"),
        "error should mention 'Failed to read file', got: {msg}"
    );

    // 同时验证 .42 扩展名会尝试字节码路径
    let path_42 = PathBuf::from("/nonexistent/path/file.42");
    let err = crate::util::diagnostic::run_file_with_diagnostics(&path_42, false, "embedded", 0)
        .expect_err("expected error for nonexistent .42 file");

    let msg = format!("{}", err);
    assert!(
        msg.contains("Failed to load bytecode file"),
        "error should mention 'Failed to load bytecode file', got: {msg}"
    );
}
