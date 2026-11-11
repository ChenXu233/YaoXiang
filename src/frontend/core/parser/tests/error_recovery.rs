//! 错误恢复与类型语法错误测试
//!
//! 规范来源：
//! - 类型系统规范 §1.1 TypeExpr：合法类型表达式文法，`?`（try 运算符）不在其中
//! - 标准库规范 §1.4 错误传播：`ErrorPropagate ::= Expr '?'`，`?` 仅用于表达式
//! - 语法规范 §3.4 break 语句：`BreakStmt ::= 'break'`，无标签（#314）
//! - RFC-010：统一赋值语法 `name: (param: Type, ...) -> Ret = body`
//! - RFC-007：类型标注可选（HM 推断），省略标注即可，无需 `?` 占位

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

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
fn test_invalid_token_as_param_type_reports_error() {
    // Arrange: #250 审计残余——`?` 之外的非法类型 token（如字符串字面量）
    // 曾被 `_ => None` 静默吞掉，参数变无标注、绑定照常编译（规范：类型系统 §1.1）
    let source = "f: (a: \"hello\") -> Void = (a) => { a }\nmain = { f(1) }";
    let tokens = tokenize(source).unwrap();

    // Act
    let result = parse(&tokens);

    // Assert: 必须在非法 token 处报错，而非静默丢弃标注
    assert!(result.has_errors, "非法类型 token 必须产生解析错误");
    assert!(
        result
            .errors
            .iter()
            .any(|e| { e.code == "E0010" && e.span.is_some_and(|s| s.start.line == 1) }),
        "必须报 E0010 且指向第 1 行的非法类型 token"
    );
}

#[test]
fn test_break_with_label_syntax_reports_error() {
    // Arrange: #314 定案 Python 风格——break/continue 无标签，`break ::x`
    // 必须明确报错而非静默忽略标签按裸 break 执行
    let source = "main = { while true { break ::123 } }";
    let tokens = tokenize(source).unwrap();

    // Act
    let result = parse(&tokens);

    // Assert
    assert!(result.has_errors, "break :: 必须报错（标签语法已移除）");
    assert!(
        result
            .errors
            .iter()
            .any(|e| { e.code == "E0011" && e.span.is_some_and(|s| s.start.line == 1) }),
        "必须报 E0011（unexpected token '::'）且指向第 1 行"
    );
}
