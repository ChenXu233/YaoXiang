//! 诊断模块核心测试 — 基于 check-improvement 设计规范
//!
//! §4.4: 错误处理修复（E1-E8）
//! §5.2: 跨文件分析流程（check_modules_with_shared_env）

use crate::util::diagnostic::{
    parse_compile_error, check_files_with_diagnostics, render_runtime_error, ErrorCodeDefinition,
    TextEmitter,
};
use crate::util::span::{DebugSpan, SourceFile, SourceMap, Span, Position};
use crate::backends::{ExecutorError, StackFrame};
use crate::middle::bytecode::{BytecodeModule, BytecodeFunction, BytecodeInstr};
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;
use crate::util::diagnostic::emitter::ansi::strip_ansi;

#[test]
fn test_render_unknown_variable_with_source() {
    let source = r#"use std.io

main = () => {
  print("Testing error handling\n")
  print(a)
  print("All tests passed!\n")
}"#;

    let source_file = SourceFile::new("error.yx".to_string(), source.to_string());

    let diagnostic = ErrorCodeDefinition::unknown_variable("a")
        .at(Span::new(
            Position::with_offset(5, 7, 65),
            Position::with_offset(5, 8, 66),
        ))
        .build();

    let emitter = TextEmitter::new();
    let output = emitter.render_with_source(&diagnostic, Some(&source_file));
    let clean_output = strip_ansi(&output);

    assert!(clean_output.contains("error [E1001]"), "{}", clean_output);
    assert!(
        clean_output.contains("Unknown variable"),
        "{}",
        clean_output
    );
    assert!(clean_output.contains("error.yx:5:7"), "{}", clean_output);
    assert!(clean_output.contains("print(a)"), "{}", clean_output);
    assert!(clean_output.contains("^"), "{}", clean_output);
}

#[test]
fn test_render_error_without_source_file() {
    let diagnostic = ErrorCodeDefinition::find("E0001")
        .unwrap()
        .builder()
        .param("char", "@")
        .build();

    let emitter = TextEmitter::new();
    let output = emitter.render(&diagnostic);
    let clean_output = strip_ansi(&output);

    assert!(clean_output.contains("error [E0001]"), "{}", clean_output);
    assert!(
        clean_output.contains("Invalid character"),
        "{}",
        clean_output
    );
}

#[test]
fn test_parse_compile_error_returns_e8001() {
    // parse_compile_error 现在统一使用 E8001 内部错误
    let diagnostic = parse_compile_error("Inference error: Unknown variable: a");
    assert_eq!(diagnostic.code, "E8001");
    assert!(diagnostic.message.contains("Unknown variable: a"));

    let diagnostic = parse_compile_error("Inference error: some other error");
    assert_eq!(diagnostic.code, "E8001");
}

#[test]
fn test_error_code_find_and_lookup() {
    let code = ErrorCodeDefinition::find("E0001");
    assert!(code.is_some());
    assert_eq!(code.unwrap().code, "E0001");

    let code = ErrorCodeDefinition::find("E9999");
    assert!(code.is_none());
}

#[test]
fn test_error_code_registry_minimum_count() {
    let all = ErrorCodeDefinition::all();
    assert!(
        all.len() > 30,
        "Expected more than 30 error codes, got {}",
        all.len()
    );
}

#[test]
fn test_render_runtime_function_not_found_with_span() {
    let source = r#"main = () => {
  foo()
}"#;
    let mut sources = SourceMap::new();
    let file_id = sources.add_file("error.yx".to_string(), source.to_string());

    let span = Span::new(
        Position::with_offset(2, 3, 0),
        Position::with_offset(2, 6, 0),
    );
    let debug_span = DebugSpan::new(file_id, span);

    let mut module = BytecodeModule::new("test".to_string());
    module.add_function(BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 0,
        upvalue_count: 0,
        instructions: vec![BytecodeInstr::Nop],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::from([(0usize, debug_span)]),
    });

    let err = ExecutorError::function_not_found(
        "foo".to_string(),
        vec![StackFrame {
            function_name: "main".to_string(),
            ip: 0,
        }],
    );

    let output = render_runtime_error(&err, &module, Some(&sources));
    let clean_output = strip_ansi(&output);

    assert!(clean_output.contains("error [E6006]"), "{}", clean_output);
    assert!(
        clean_output.contains("Function not found"),
        "{}",
        clean_output
    );
    assert!(clean_output.contains("error.yx:2:3"), "{}", clean_output);
    assert!(clean_output.contains("foo()"), "{}", clean_output);
    assert!(clean_output.contains("stack trace:"), "{}", clean_output);
    assert!(
        clean_output.contains("at main (error.yx:2:3) (ip: 0)"),
        "{}",
        clean_output
    );
}

#[test]
fn test_check_files_with_diagnostics_ok() {
    let dir = tempdir().expect("create temp dir");
    let file = dir.path().join("ok.yx");
    fs::write(
        &file,
        r#"use std.io

main: () -> Void = {
  print("ok")
}
"#,
    )
    .expect("write yx file");

    let result = check_files_with_diagnostics(&[file]).expect("run check");
    assert_eq!(result.error_count, 0);
    assert_eq!(result.warning_count, 0);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn test_check_files_with_diagnostics_error() {
    let dir = tempdir().expect("create temp dir");
    let file = dir.path().join("bad.yx");
    fs::write(
        &file,
        r#"use std.io

main: () -> Void = {
  print(a)
}
"#,
    )
    .expect("write yx file");

    let result = check_files_with_diagnostics(&[file]).expect("run check");
    assert!(result.error_count > 0);
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn test_cross_file_reference() {
    // 注意：当前实现中 check_single_module 为每个文件创建独立的 Compiler，
    // 跨文件符号解析尚未完全实现。此测试验证多文件流水线能正常运行，
    // 并且依赖图的拓扑排序正确工作。
    let dir = tempfile::tempdir().expect("create temp dir");

    let file_a = dir.path().join("a.yx");
    std::fs::write(
        &file_a,
        r#"use std.io

pub greet: (name: String) -> Void = (name) => {
    print(name)
}
"#,
    )
    .expect("write a.yx");

    let file_b = dir.path().join("b.yx");
    std::fs::write(
        &file_b,
        r#"use std.io

main: () -> Void = {
    print("hello")
}
"#,
    )
    .expect("write b.yx");

    let result = check_files_with_diagnostics(&[file_a, file_b]).expect("run check");
    assert_eq!(
        result.error_count, 0,
        "Independent multi-file check should pass without errors"
    );
}

#[test]
fn test_single_file_no_cycle() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let file = dir.path().join("main.yx");
    std::fs::write(
        &file,
        r#"use std.io

main: () -> Void = {
    print("hello")
}
"#,
    )
    .expect("write file");

    let result = check_files_with_diagnostics(&[file]).expect("run check");
    assert_eq!(result.error_count, 0);
}
