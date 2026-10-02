//! RFC-012: F-string integration tests
//!
//! Tests for f-string template literal compilation pipeline.

use yaoxiang::run;

/// Helper: run source and check it doesn't fail at parse/compile stage
fn compile_ok(source: &str) {
    let adapted = crate::fixture::with_main_invoked(source);
    let result = run(&adapted);
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                !error_msg.contains("parse") && !error_msg.contains("syntax"),
                "Compilation error for f-string: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_fstring_basic_compilation() {
    compile_ok(
        r#"
        x = f"hello world"
    "#,
    );
}

#[test]
fn test_fstring_with_variable_compilation() {
    compile_ok(
        r#"
        name = "Alice"
        greeting = f"Hello {name}"
    "#,
    );
}

#[test]
fn test_fstring_with_expression_compilation() {
    compile_ok(
        r#"
        x = 10
        y = 20
        result = f"Sum: {x + y}"
    "#,
    );
}

#[test]
fn test_fstring_multiple_interpolations_compilation() {
    compile_ok(
        r#"
        x = 10
        y = 20
        s = f"{x} + {y} = {x + y}"
    "#,
    );
}

#[test]
fn test_fstring_const_eval() {
    // This should be optimized to a constant string at compile time
    compile_ok(
        r#"
        x = f"hello"
    "#,
    );
}

#[test]
fn test_fstring_with_format_spec_compilation() {
    compile_ok(
        r#"
        pi = 3.14159
        s = f"Pi: {pi:.2f}"
    "#,
    );
}

#[test]
fn test_fstring_in_print_compilation() {
    compile_ok(
        r#"
        name = "World"
        print(f"Hello {name}")
    "#,
    );
}

// #402: 花括号转义（RFC-012 与 Python 一致，双大括号表示字面花括号）

#[test]
fn test_fstring_brace_escape_execution() {
    // Arrange
    let source = r#"
        s = f"{{literal}}"
    "#;

    // Act
    let adapted = crate::fixture::with_main_invoked(source);
    let result = run(&adapted);

    // Assert: 未定义的 `literal` 不应再被当作插值变量（#402 修前报 E1001）
    assert!(
        result.is_ok(),
        "brace escape must not become interpolation: {result:?}"
    );
}

#[test]
fn test_fstring_multiline_execution() {
    // Arrange: f""" 多行模板（RFC-012, #402）
    let source = "s = f\"\"\"line1\nline2\"\"\"";

    // Act
    let adapted = crate::fixture::with_main_invoked(source);
    let result = run(&adapted);

    // Assert
    assert!(
        result.is_ok(),
        "multiline f-string must compile and run: {result:?}"
    );
}

#[test]
fn test_fstring_unknown_format_code_fails() {
    // Arrange: RFC-012 未定义的展示类型 'q' → 运行期拒绝而非静默忽略
    let source = r#"
        pi = 3.14
        s = f"{pi:q}"
    "#;

    // Act
    let adapted = crate::fixture::with_main_invoked(source);
    let result = run(&adapted);

    // Assert
    match result {
        Ok(_) => panic!("unknown format code 'q' must fail, not be silently ignored"),
        Err(e) => {
            let msg = format!("{e:?}");
            assert!(
                msg.contains('q'),
                "error should mention the invalid format code: {msg}"
            );
        }
    }
}
