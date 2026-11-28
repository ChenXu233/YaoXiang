//! 字节码文件 (.42) 加载和执行测试
//!
//! 覆盖: `src/middle/passes/codegen/bytecode.rs` (BytecodeFile::load / read_from)
//! 和 `src/util/diagnostic/mod.rs` (run_file_with_diagnostics .42 分支)
//! 设计: `docs/superpowers/specs/2026-07-03-bytecode-run-support-design.md`
//! 修正: `docs/superpowers/specs/2026-07-26-issue231-bytecode-run-magic-probe-design.md` (魔数探针替代扩展名)

use std::io::Write;
use std::path::PathBuf;

/// Fixture + Act：写入带方法（`Point.get_x`）的项目，编译为字节码并反序列化加载。
fn compile_point_method_project(
    dir: &std::path::Path
) -> crate::middle::bytecode::file::BytecodeFile {
    let source_path = dir.join("test.yx");
    let bytecode_path = dir.join("test.42");
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

    crate::build_bytecode_with_options(&source_path, &bytecode_path, false)
        .expect("build bytecode");
    crate::middle::bytecode::file::BytecodeFile::load(&bytecode_path).expect("load bytecode file")
}

/// Assert：vtables 段经序列化往返后仍携带 get_x（裸方法名 + 函数表索引）。
fn assert_vtables_carry_get_x(bytecode_file: &crate::middle::bytecode::file::BytecodeFile) {
    assert!(
        bytecode_file
            .vtables
            .iter()
            .any(|(ty, methods)| ty == "Point" && methods.iter().any(|(bare, _)| bare == "get_x")),
        "vtables section should carry (get_x, func_idx) after roundtrip, got {:?}",
        bytecode_file.vtables
    );
}

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
    let bytecode_file = crate::middle::bytecode::file::BytecodeFile::load(&bytecode_path)
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
    // Arrange：定义带方法的类型，main 中调用方法；编译 → 落盘 → 反序列化加载
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let bytecode_file = compile_point_method_project(dir.path());

    // Assert：vtables 段经序列化往返后仍携带 get_x
    assert_vtables_carry_get_x(&bytecode_file);

    // Act：方法分发经 vtable 执行（转换为执行器模块）
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);
    let interp = crate::backends::interpreter::Interpreter::new();
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);

    // Assert：分发成功（失败即 panic，消息说明往返语义）
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
    let err = crate::middle::bytecode::file::BytecodeFile::load(&path)
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
    let err = crate::middle::bytecode::file::BytecodeFile::load(&path)
        .expect_err("expected error for version mismatch");

    // Assert
    let msg = format!("{}", err);
    assert!(
        msg.contains("unsupported"),
        "error should mention 'unsupported', got: {msg}"
    );
}

/// `.42` 直跑与源码直跑的函数上值槽数必须一致（RFC-039 D17）。
///
/// 条款门：RFC-039 D17「新增字段时把 .42 版本号从 4 升到 5」——`upvalue_count`
/// 必须随产物落盘。病史门：载入侧曾把它硬编 0（7e.1 收编的数据丢失），
/// 闭包的上值槽数落盘即丢，产物与运行结果都看不出异常。
///
/// 两端都直驱：生产端是 `Compiler::compile_with_source` → `CodegenContext::generate`
/// （源码直跑同一路径），消费端是 `build_bytecode_with_options` → `BytecodeFile::load`
/// （.42 直跑同一路径）；不手工构造登记数据。
#[test]
fn test_dot42_roundtrip_preserves_source_upvalue_count() {
    // Arrange：spawn-for 的闭包体用 LoadUpvalue 读外层捕获 n
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let source_path = dir.path().join("test.yx");
    let bytecode_path = dir.path().join("test.42");
    let source = r#"main: () -> Void = {
    n = 100
    items = [1, 2]
    spawn for item in items {
        print(item * 1000 + n)
    }
}
main()
"#;
    std::fs::write(&source_path, source).expect("write source file");

    // Act 1：源码直跑路径（编译 → 直接翻译，不落盘）
    let module = crate::frontend::Compiler::new()
        .compile_with_source("test.yx", source)
        .expect("compile source");
    let mut direct_ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let direct = direct_ctx.generate().expect("codegen from source");
    let direct_counts: Vec<usize> = direct
        .code_section
        .functions
        .iter()
        .map(|function| function.upvalue_count)
        .collect();

    // Act 2：.42 直跑路径（落盘 → 读回）
    crate::build_bytecode_with_options(&source_path, &bytecode_path, false)
        .expect("build bytecode");
    let loaded = crate::middle::bytecode::file::BytecodeFile::load(&bytecode_path)
        .expect("load bytecode file");
    let loaded_counts: Vec<usize> = loaded
        .code_section
        .functions
        .iter()
        .map(|function| function.upvalue_count)
        .collect();

    // Assert：逐函数一致；且确有函数读上值（否则该用例对旧行为不可分辨）
    assert_eq!(
        loaded_counts, direct_counts,
        ".42 往返后每函数上值槽数必须与源码直跑一致（旧行为恒为 0）；源码直跑={direct_counts:?}"
    );
    assert!(
        direct_counts.iter().any(|count| *count > 0),
        "夹具必须真的产生上值读取，否则本用例分辨不出「落盘」与「硬编 0」：{direct_counts:?}"
    );
}

/// `.42` 直跑与源码直跑的**全局表**与**全局槽值**必须一致（RFC-039 D52 的 `2341` 项）。
///
/// 条款门：RFC-039 D52「`.42` 三处硬编码丢弃全部收进 P7」。
/// 病史门：`From<BytecodeFile>` 曾写 `globals: Vec::new()`——顶层绑定只剩调试段里的名字，
/// 「有哪些全局、各是什么类型」整张表消失，而运行结果看不出异常（全局槽按需扩容）。
///
/// 两端都直驱：生产端 `Compiler::compile_with_source` → `CodegenContext::generate`（源码直跑同一路径），
/// 消费端 `build_bytecode_with_options` → `BytecodeFile::load`（.42 直跑同一路径）。
#[test]
fn test_dot42_run_matches_source_run_for_globals() {
    // Arrange：顶层可变全局 + 初始化表达式 + 函数内读写 + 模块初始化交互
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let source_path = dir.path().join("globals.yx");
    let bytecode_path = dir.path().join("globals.42");
    let source = r#"use std.assert
mut counter: Int = 41
label: String = "yx"

bump: () -> Void = () => {
    counter = counter + 1
}

read_counter: () -> Int = () => counter

main: () -> Void = () => {
    assert.assert(read_counter() == 41, "初始化表达式必须已求值")
    bump()
    assert.assert(read_counter() == 42, "函数内写全局必须生效")
    assert.assert(label == "yx", "String 全局必须可读")
}
main()
"#;
    std::fs::write(&source_path, source).expect("write source file");

    // Act 1：源码直跑（编译 → 直接翻译 → 解码进 BytecodeModule）
    let module = crate::frontend::Compiler::new()
        .compile_with_source("globals.yx", source)
        .expect("compile source");
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let direct_file = ctx.generate().expect("codegen from source");
    let direct = crate::middle::bytecode::BytecodeModule::from(direct_file);

    // Act 2：.42 直跑（落盘 → 读回 → 解码）
    crate::build_bytecode_with_options(&source_path, &bytecode_path, true).expect("build bytecode");
    let loaded = crate::middle::bytecode::file::BytecodeFile::load(&bytecode_path)
        .expect("load bytecode file");
    let from_dot42 = crate::middle::bytecode::BytecodeModule::from(loaded);

    // Assert 1：全局表逐项一致（表序 + 名字 + 类型号）
    let table_of =
        |m: &crate::middle::bytecode::BytecodeModule| -> Vec<(String, crate::frontend::core::sema::MonoType)> {
            m.globals
                .iter()
                .map(|g| (g.name.clone(), g.ty.clone()))
                .collect()
        };
    let direct_globals = table_of(&direct);
    let dot42_globals = table_of(&from_dot42);
    assert_eq!(
        dot42_globals, direct_globals,
        ".42 往返后全局表必须与源码直跑一致；空表即载入侧硬编（旧行为）"
    );
    assert!(
        dot42_globals.iter().any(|(name, _)| name == "counter"),
        "夹具必须真的产生顶层全局，否则本用例分辨不出「落盘」与「硬编空表」：{dot42_globals:?}"
    );

    // Act 3：两侧各自执行，读回全部全局槽值
    let run = |m: &crate::middle::bytecode::BytecodeModule| -> Vec<String> {
        use crate::backends::Executor;
        let mut interp = crate::backends::interpreter::Interpreter::new();
        interp.execute_module(m).expect("execute module");
        (0..m.globals.len())
            .map(|i| format!("{:?}", interp.global_slot(i)))
            .collect()
    };
    let direct_slots = run(&direct);
    let dot42_slots = run(&from_dot42);

    // Assert 2：行为等价（模块初始化序列 + 函数内读写全局）
    assert_eq!(
        dot42_slots, direct_slots,
        ".42 直跑与源码直跑的全局槽值必须逐槽一致"
    );
    assert!(
        dot42_slots.iter().any(|v| v.contains("42")),
        "bump 之后的 counter 必须是 42（初始化 41 + 写入 1）：{dot42_slots:?}"
    );
}

/// 验证 .yx 源文件路径走源码编译分支，不进入字节码加载分支。
#[test]
fn test_run_yx_file_uses_source_compile_path() {
    // Arrange
    let path = PathBuf::from("/nonexistent/path/file.yx");

    // Act
    let err = crate::util::diagnostic::run_file_with_diagnostics(
        &path,
        "embedded",
        0,
        crate::frontend::config::CompileConfig::default(),
    )
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
    let err = crate::util::diagnostic::run_file_with_diagnostics(
        &path,
        "embedded",
        0,
        crate::frontend::config::CompileConfig::default(),
    )
    .expect_err("expected error for file with YXBC magic");

    // Assert
    let msg = format!("{}", err);
    assert!(
        msg.contains("Failed to load bytecode file"),
        "error should mention 'Failed to load bytecode file', got: {msg}"
    );
}
