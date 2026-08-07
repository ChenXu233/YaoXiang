//! 字节码文件 (.42) 加载和执行测试
//!
//! 覆盖: `src/middle/passes/codegen/bytecode.rs` (BytecodeFile::load / read_from)
//! 和 `src/util/diagnostic/mod.rs` (run_file_with_diagnostics .42 分支)
//! 设计: `docs/superpowers/specs/2026-07-03-bytecode-run-support-design.md`
//! 修正: `docs/superpowers/specs/2026-07-26-issue231-bytecode-run-magic-probe-design.md` (魔数探针替代扩展名)

use std::io::Write;
use std::path::PathBuf;

/// 编译 .yx 源文件为 .42 字节码文件，然后加载并执行它。
#[test]
fn test_run_bytecode_file_roundtrip() {
    // Arrange
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let source_path = dir.path().join("test.yx");
    let bytecode_path = dir.path().join("test.42");
    std::fs::write(
        &source_path,
        "main = () => { print(\"hello from bytecode\") }",
    )
    .expect("write source file");

    // Act
    crate::build_bytecode_with_options(&source_path, &bytecode_path, false)
        .expect("build bytecode");
    let bytecode_file = crate::middle::passes::codegen::BytecodeFile::load(&bytecode_path)
        .expect("load bytecode file");
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);
    let interp = crate::backends::interpreter::Interpreter::new();
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);

    // Assert
    executor
        .execute_module(&bytecode_module)
        .expect("execute bytecode module — roundtrip should succeed");
}

/// 带方法的类型经字节码序列化往返后，编译期 vtables 段应保留并驱动方法分发。
///
/// 回归保护：vtable 已不再由解释器运行时按 `{type}.` 前缀扫描构建，而由 codegen
/// 写入字节码 vtables 段、加载期重建。若该段序列化丢失，方法调用会失败。
#[test]
fn test_run_bytecode_file_roundtrip_with_method_vtable() {
    // Arrange：定义带方法的类型，main 中调用方法并断言结果
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let source_path = dir.path().join("test.yx");
    let bytecode_path = dir.path().join("test.42");
    std::fs::write(
        &source_path,
        r#"
Point: Type = { x: Float, y: Float }

Point.get_x: (self: &Point) -> Float = {
    return self.x
}

main = {
    p = Point(3.0, 4.0)
    print(p.get_x())
}
"#,
    )
    .expect("write source file");

    // Act：编译为字节码 → 序列化落盘 → 反序列化加载
    crate::build_bytecode_with_options(&source_path, &bytecode_path, false)
        .expect("build bytecode");
    let bytecode_file = crate::middle::passes::codegen::BytecodeFile::load(&bytecode_path)
        .expect("load bytecode file");

    // Assert：vtables 段经序列化往返后仍携带 get_x（裸方法名 + 函数表索引）
    assert!(
        bytecode_file
            .vtables
            .iter()
            .any(|(ty, methods)| ty == "Point" && methods.iter().any(|(bare, _)| bare == "get_x")),
        "vtables section should carry (get_x, func_idx) after roundtrip, got {:?}",
        bytecode_file.vtables
    );

    // Assert：方法分发经 vtable 成功执行
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);
    let interp = crate::backends::interpreter::Interpreter::new();
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);
    executor
        .execute_module(&bytecode_module)
        .expect("execute bytecode module — vtable method dispatch should survive roundtrip");
}

/// 无效魔数的 .42 文件应产生清晰的错误信息。
#[test]
fn test_run_bytecode_file_invalid_magic() {
    // Arrange
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let path = dir.path().join("bad.42");
    let mut f = std::fs::File::create(&path).expect("create file");
    f.write_all(&[0u8; 32]).expect("write data");
    drop(f);

    // Act
    let err = crate::middle::passes::codegen::BytecodeFile::load(&path)
        .expect_err("expected error for invalid magic");

    // Assert
    let msg = format!("{}", err);
    assert!(
        msg.contains("invalid magic"),
        "error should mention 'invalid magic', got: {msg}"
    );
}

/// 正确魔数但版本不匹配的 .42 文件应产生清晰的错误信息。
#[test]
fn test_run_bytecode_file_version_mismatch() {
    // Arrange
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let path = dir.path().join("old.42");
    let mut f = std::fs::File::create(&path).expect("create file");
    // 写入有效魔数（YXBC = 0x59584243 大端序），但版本号为 99
    f.write_all(&[0x59, 0x58, 0x42, 0x43]).expect("write magic");
    f.write_all(&[99u8, 0, 0, 0]).expect("write version 99");
    drop(f);

    // Act
    let err = crate::middle::passes::codegen::BytecodeFile::load(&path)
        .expect_err("expected error for version mismatch");

    // Assert
    let msg = format!("{}", err);
    assert!(
        msg.contains("unsupported"),
        "error should mention 'unsupported', got: {msg}"
    );
}

/// 验证 .yx 源文件路径走源码编译分支，不进入字节码加载分支。
#[test]
fn test_run_yx_file_uses_source_compile_path() {
    // Arrange
    let path = PathBuf::from("/nonexistent/path/file.yx");

    // Act
    let err = crate::util::diagnostic::run_file_with_diagnostics(&path, false, "embedded", 0)
        .expect_err("expected error for nonexistent .yx file");

    // Assert
    let msg = format!("{}", err);
    assert!(
        msg.contains("Failed to read file"),
        "error should mention 'Failed to read file', got: {msg}"
    );
}

/// 验证包含 YXBC 魔数的文件路径进入字节码加载分支。
/// 通过临时文件写入有效魔数但损坏数据的方式测试，确保 probe() 返回 true 后进入 load 路径。
#[test]
fn test_run_file_with_yxbc_magic_uses_bytecode_load_path() {
    // Arrange: 写入一个有效 YXBC 魔数但内容损坏的文件
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let path = dir.path().join("test.bin");
    // 魔数 YXBC (0x59584243) 大端序 + 垃圾数据
    let mut data = Vec::new();
    data.extend_from_slice(&0x59584243u32.to_be_bytes());
    data.extend_from_slice(b"corrupted data");
    std::fs::write(&path, &data).expect("write file");

    // Act
    let err = crate::util::diagnostic::run_file_with_diagnostics(&path, false, "embedded", 0)
        .expect_err("expected error for file with YXBC magic");

    // Assert
    let msg = format!("{}", err);
    assert!(
        msg.contains("Failed to load bytecode file"),
        "error should mention 'Failed to load bytecode file', got: {msg}"
    );
}
