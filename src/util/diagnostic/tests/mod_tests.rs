//! 诊断模块核心测试 — 基于 check-improvement 设计规范
//!
//! §4.4: 错误处理修复（E1-E8）
//! §5.2: 跨文件分析流程（check_modules_with_shared_env）

use crate::backends::{ExecutorError, StackFrame};
use crate::frontend::config::CompileConfig;
use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, BytecodeModule};
use crate::util::diagnostic::emitter::ansi::strip_ansi;
use crate::util::diagnostic::{
    check_files_with_diagnostics, parse_compile_error, render_runtime_error, CheckDiagnostic,
    CheckResult, ErrorCodeDefinition, Severity, TextEmitter,
};
use crate::util::span::{DebugSpan, Position, SourceFile, SourceMap, Span};
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;

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
        return_type: crate::frontend::core::sema::MonoType::Void,
        local_count: 0,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![BytecodeInstr::Nop],
        labels: HashMap::new(),
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
fn test_render_runtime_stack_overflow_carries_real_limit() {
    // Arrange — WBS 3.9.1：E6004 的 limit 必须来自配置真实上限（修复前恒传 0）
    let module = BytecodeModule::new("stack_overflow_limit_test".to_string());
    let err = ExecutorError::stack_overflow(8191, vec![]);

    // Act — 渲染运行时错误并去掉 ANSI，便于断言纯文本
    let output = render_runtime_error(&err, &module, None);
    let clean_output = strip_ansi(&output);

    // Assert — 码正确 + 报文携带真实上限（占位 0 会被后两条断言挡住）
    assert!(clean_output.contains("error [E6004]"), "{}", clean_output);
    assert!(
        clean_output.contains("8191"),
        "E6004 报文应携带配置真实上限 8191：{}",
        clean_output
    );
    assert!(
        !clean_output.contains("限制 0") && !clean_output.contains("limit 0"),
        "E6004 报文不得再出现占位上限 0：{}",
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

    let result =
        check_files_with_diagnostics(&[file], CompileConfig::default()).expect("run check");
    assert_eq!(result.error_count, 0);
    assert_eq!(result.warning_count, 0);
    assert!(result.diagnostics.is_empty());
}

// ==================== CheckResult::suppress_codes（issue #499 --allow） ====================
//
// 契约（run_check_command_once 唯一过滤闸的对称记账方法）：
// - 被允许的 Warning 级诊断移除且 warning_count 对称递减——抑制 = 对用户
//   完全不可见（issue #499，用户 2026-10-24 裁决 A / 2026-11-20 甲案）；
// - Error 级诊断不可抑制——错误不可被 CLI 旗标静音（W 族码可被发射点显式
//   升为 Error，唯一组合 W1090，见 codes/w1xxx.rs 的解耦注释）；
// - 空码表 = 无操作（不带 --allow 的调用零行为变化）。

#[test]
fn test_suppress_codes_removes_matching_warning_and_decrements_count() {
    // Arrange: W1001 与 W1003 各一条，warning_count = 2（push_diagnostic 口径）
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut result = CheckResult::default();
    result.diagnostics.push(CheckDiagnostic {
        file: "a.yx".to_string(),
        diagnostic: ErrorCodeDefinition::unused_function("foo").build(),
    });
    result.diagnostics.push(CheckDiagnostic {
        file: "a.yx".to_string(),
        diagnostic: ErrorCodeDefinition::unused_import("io").build(),
    });
    result.warning_count = 2;

    // Act
    result.suppress_codes(&["W1001".to_string()]);

    // Assert
    assert_eq!(result.warning_count, 1, "被抑制码的计数应对称递减");
    assert_eq!(result.diagnostics.len(), 1, "被抑制码的诊断应被移除");
    assert_eq!(
        result.diagnostics[0].diagnostic.code, "W1003",
        "未被允许的码应保留"
    );
}

#[test]
fn test_suppress_codes_keeps_error_severity_diagnostics() {
    // Arrange: W1090 以显式 Error 级发射（码族与发射级别解耦）
    let mut result = CheckResult::default();
    result.diagnostics.push(CheckDiagnostic {
        file: "a.yx".to_string(),
        diagnostic: ErrorCodeDefinition::obligation_not_drained("field", "a.yx", "detail")
            .severity(Severity::Error)
            .build(),
    });
    result.error_count = 1;

    // Act
    result.suppress_codes(&["W1090".to_string()]);

    // Assert: Error 级不可被 --allow 静音——被静音的错误会让失败的 check 伪装成通过
    assert_eq!(result.error_count, 1, "Error 级诊断不可抑制");
    assert_eq!(result.diagnostics.len(), 1, "Error 级诊断应保留");
}

#[test]
fn test_suppress_codes_empty_list_is_noop() {
    // Arrange
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut result = CheckResult::default();
    result.diagnostics.push(CheckDiagnostic {
        file: "a.yx".to_string(),
        diagnostic: ErrorCodeDefinition::unused_function("foo").build(),
    });
    result.warning_count = 1;

    // Act: 空码表（不带 --allow 的调用形态）
    result.suppress_codes(&[]);

    // Assert
    assert_eq!(result.warning_count, 1, "空码表不得改变计数");
    assert_eq!(result.diagnostics.len(), 1, "空码表不得移除诊断");
}
