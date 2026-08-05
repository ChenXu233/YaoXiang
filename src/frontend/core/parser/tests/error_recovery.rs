//! 错误恢复与类型语法错误测试
//!
//! 规范来源：
//! - 类型系统规范 §1.1 TypeExpr：合法类型表达式文法，`?`（try 运算符）不在其中
//! - 标准库规范 §1.4 错误传播：`ErrorPropagate ::= Expr '?'`，`?` 仅用于表达式
//! - RFC-010：统一赋值语法 `name: (param: Type, ...) -> Ret = body`
//! - RFC-007：类型标注可选（HM 推断），省略标注即可，无需 `?` 占位

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

#[test]
fn test_parse_valid_input() {
    let tokens = tokenize("x = 42").unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors);
    assert_eq!(result.module.items.len(), 1);
}

#[test]
fn test_parse_empty_input() {
    let tokens = tokenize("").unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors);
    assert!(result.module.items.is_empty());
}

#[test]
fn test_recovery_continues_after_error() {
    // @ 不是有效的语句起始，但解析器应该继续解析后续有效语句
    let source = "@\nx = 42";
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    // 应该包含错误
    assert!(result.has_errors);
    // 也应该包含后续有效语句
    assert!(!result.module.items.is_empty());
}

#[test]
fn test_parse_returns_error() {
    // parse() 应该返回 has_errors = true
    let tokens = tokenize("@").unwrap();
    let result = parse(&tokens);
    assert!(result.has_errors);
}

#[test]
fn test_recovery_multiple_errors() {
    let source = "@\n@\nx = 42";
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(result.has_errors);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_parse_errors_collected() {
    let tokens = tokenize("@").unwrap();
    let result = parse(&tokens);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_question_mark_as_param_type_reports_error() {
    // Arrange: #250 复现 — `?` 不是合法类型（类型系统规范 §1.1），必须是 try 运算符（标准库规范 §1.4）
    let source =
        "f: (a: ?) -> Void = (a) => { println(\"defined\") }\nmain = { println(\"main only\") }";
    let tokens = tokenize(source).unwrap();

    // Act
    let result = parse(&tokens);

    // Assert: 解析器必须在定义处（`?` 所在行）报 E0012 无效语法
    assert!(result.has_errors, "`?` 作为参数类型必须产生解析错误");
    assert!(
        result
            .errors
            .iter()
            .any(|e| { e.code == "E0012" && e.span.is_some_and(|s| s.start.line == 1) }),
        "错误必须指向定义处第 1 行的 `?`，而非调用点"
    );
}

#[test]
fn test_fn_signature_with_tuple_return_type() {
    // Arrange: #253 — 返回类型为元组时（`(Int, Int) -> (Int, Int)`），
    // 解析器不得把返回元组误判为 curry 参数组（否则报 "Expected Arrow, found Eq"）。
    // 规范来源：类型系统规范 §3.4 TupleType `::= '(' TypeList? ')'` 可作为任意类型位置。
    let tokens = tokenize("mk: (Int, Int) -> (Int, Int) = (x, y) => x + y").unwrap();

    // Act
    let result = parse(&tokens);

    // Assert: 签名解析成功，无错误
    assert!(
        !result.has_errors,
        "tuple-returning fn signature should parse cleanly, got: {:?}",
        result.errors
    );
}
