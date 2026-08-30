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

#[test]
fn test_invalid_lambda_params_report_error() {
    // Arrange: `=>` 左侧不是合法参数列表时（如调用表达式），
    // 此前整个表达式静默消失，残留 token 变误导诊断（RFC-010 lambda 语法）
    let source = "main = { f(x) => x }";
    let tokens = tokenize(source).unwrap();

    // Act
    let result = parse(&tokens);

    // Assert
    assert!(result.has_errors, "非法 lambda 参数列表必须报错");
    assert!(
        result.errors.iter().any(|e| {
            e.message.contains("lambda parameter") && e.span.is_some_and(|s| s.start.line == 1)
        }),
        "必须报 lambda 参数错误且指向第 1 行"
    );
}

#[test]
fn test_invalid_struct_pattern_field_reports_error() {
    // Arrange: 结构模式字段必须是标识符/构造子模式；字面量等非法字段
    // 此前被静默剔除，模式变得比源码更宽（语法规范 §5 模式匹配）
    let source = "main = { match q { { x + y } => 1, _ => 2 } }";
    let tokens = tokenize(source).unwrap();

    // Act
    let result = parse(&tokens);

    // Assert
    assert!(result.has_errors, "非法结构模式字段必须报错");
    for e in &result.errors {
        eprintln!("ERR code={} msg={} span={:?}", e.code, e.message, e.span);
    }
    assert!(
        result.errors.iter().any(|e| {
            e.message.contains("struct pattern field") && e.span.is_some_and(|s| s.start.line == 1)
        }),
        "必须报结构模式字段错误且指向第 1 行"
    );
}
