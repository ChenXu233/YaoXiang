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

/// Helper: test_type_error_message_no_solver_typevar_leak 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: `x: Int = "hello"` 的用例文件（返回 TempDir 以保活）。
fn leak_case_file() -> (tempfile::TempDir, std::path::PathBuf) {
    // TypeVar 的 Display 形态为 t<N>（如 't62'），不得进入用户可见消息。
    let dir = tempdir().expect("create temp dir");
    let file = dir.path().join("leak.yx");
    fs::write(
        &file,
        r#"main = {
  x: Int = "hello"
}
"#,
    )
    .expect("write yx file");

    (dir, file)
}

/// 诊断文本中是否出现求解器内部 TypeVar 名（`'t<数字>`）。
///
/// 排除合法以 `'t` 开头的单词（如 `'true'`）——只标记 t 后紧跟数字者。
fn has_solver_typevar_leak(joined: &str) -> bool {
    if !(joined.contains("'t") && {
        let bytes = joined.as_bytes();
        let mut found = false;
        let mut i = 0;
        while i + 2 < bytes.len() {
            if bytes[i] == b'\'' && bytes[i + 1] == b't' && bytes[i + 2].is_ascii_digit() {
                found = true;
                break;
            }
            i += 1;
        }
        found
    }) {
        return false;
    }
    true
}

/// Helper: test_render_runtime_function_not_found_with_span 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: `error.yx = main() { foo() }` 的源表 + 单函数模块（ip=0 → 2:3-2:6）。
fn error_module_with_debug_span() -> (SourceMap, BytecodeModule) {
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
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![BytecodeInstr::Nop],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::from([(0usize, debug_span)]),
    });

    (sources, module)
}

/// Fixture: 找不到函数 `foo`、调用点位于 main:0 的运行时错误。
fn function_not_found_err() -> ExecutorError {
    ExecutorError::function_not_found(
        "foo".to_string(),
        vec![StackFrame {
            function_name: "main".to_string(),
            ip: 0,
        }],
    )
}

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
    // 消息文本是 locales 产物（en 由 bot 维护），不断言精确措辞；
    // 锚定"取到了真实标题"而非 i18n 缺失兜底串
    assert!(
        !clean_output.contains("missing i18n template"),
        "{}",
        clean_output
    );
    assert!(clean_output.contains("error.yx:5:7"), "{}", clean_output);
    assert!(clean_output.contains("print(a)"), "{}", clean_output);
    assert!(clean_output.contains("^"), "{}", clean_output);
}

#[test]
fn test_render_error_without_source_file() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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
    // Arrange — error.yx 的 main 调 foo()，调试表把 ip=0 映射到 2:3-2:6
    let (sources, module) = error_module_with_debug_span();
    let err = function_not_found_err();

    // Act — 渲染运行时错误并去掉 ANSI，便于断言纯文本
    let output = render_runtime_error(&err, &module, Some(&sources));
    let clean_output = strip_ansi(&output);

    // Assert — 错误码 / 消息 / 源码定位 / 调用点 / 栈帧；#327：不暴露字节码 ip
    assert!(clean_output.contains("error [E6006]"), "{}", clean_output);
    // 大小写不敏感：en.json 的 E6006 模板大小写会被 i18n 自动管线改写（见 9a1ccdbd），
    // 本断言的语义是「报了这个错」，不是「文案逐字不变」
    assert!(
        clean_output.to_lowercase().contains("function not found"),
        "{}",
        clean_output
    );
    assert!(clean_output.contains("error.yx:2:3"), "{}", clean_output);
    assert!(clean_output.contains("foo()"), "{}", clean_output);
    assert!(clean_output.contains("stack trace:"), "{}", clean_output);
    assert!(
        clean_output.contains("at main (error.yx:2:3)"),
        "{}",
        clean_output
    );
    // #327：用户面向输出不暴露字节码 ip
    assert!(!clean_output.contains("(ip:"), "{}", clean_output);
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
fn test_type_error_message_no_solver_typevar_leak() {
    // #322 M3：类型错误消息不得泄漏求解器内部 TypeVar 名（#287 锚定）。
    // TypeVar 的 Display 形态为 t<N>（如 't62'），不得进入用户可见消息。
    // Arrange — 写入 x: Int = "hello" 的用例文件
    let (_dir, file) = leak_case_file();

    // Act — 类型检查并拼接全部诊断消息
    let result = check_files_with_diagnostics(&[file]).expect("run check");
    assert!(result.error_count > 0, "应有类型错误");
    let joined = result
        .diagnostics
        .iter()
        .map(|d| d.diagnostic.message.clone())
        .collect::<Vec<_>>()
        .join("\n");
    // 检测 't<N>' 形态（引号包裹的 t+纯数字标识符）
    let leak = has_solver_typevar_leak(&joined);

    // Assert — 有错误且无 TypeVar 名泄漏
    assert!(!leak, "求解器 TypeVar 名泄漏进用户消息: {joined}");
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
