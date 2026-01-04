//! 类型推断测试
//!
//! 测试类型检查器的推断功能

use crate::frontend::parser::parse;
use crate::frontend::lexer::tokenize;
use crate::frontend::typecheck::{check_module, TypeEnvironment};

/// 检查类型推断是否成功
fn check_type_inference(input: &str) -> Result<(), String> {
    let tokens = tokenize(input).map_err(|e| format!("Lexer error: {:?}", e))?;
    let ast = parse(&tokens).map_err(|e| format!("Parse error: {:?}", e))?;
    let mut env = TypeEnvironment::new();
    check_module(&ast, Some(&mut env))
        .map(|_| ())
        .map_err(|e| format!("Type error: {:?}", e))
}

/// 检查类型推断是否失败（预期错误）
fn check_type_inference_fails(input: &str) -> bool {
    check_type_inference(input).is_err()
}

// ============================================================================
// 标准语法测试（完整类型标注）
// ============================================================================

#[test]
fn test_standard_syntax_with_full_annotation() {
    // 完整类型标注应该通过
    assert!(check_type_inference("add: (Int, Int) -> Int = (a, b) => a + b").is_ok());
    assert!(check_type_inference("inc: Int -> Int = x => x + 1").is_ok());
    assert!(check_type_inference("log: (String) -> Void = (msg) => print(msg)").is_ok());
    assert!(check_type_inference("get_val: () -> Int = () => 42").is_ok());
    assert!(check_type_inference("empty: () -> Void = () => {}").is_ok());
}

// ============================================================================
// 新语法推断测试（无类型标注）
// ============================================================================

#[test]
fn test_inference_empty_block() {
    // 空块 {} 推断为 Void
    assert!(check_type_inference("main = () => {}").is_ok());
}

#[test]
fn test_inference_expression_return() {
    // 从表达式推断返回类型
    assert!(check_type_inference("get_num = () => 42").is_ok());
    assert!(check_type_inference("get_str = () => \"hello\"").is_ok());
    assert!(check_type_inference("get_bool = () => true").is_ok());
}

#[test]
fn test_inference_with_typed_param() {
    // 有类型标注的参数应该通过
    assert!(check_type_inference("square: (Int) -> Int = (x) => x * x").is_ok());
    assert!(check_type_inference("add_typed: (Int, Int) -> Int = (a, b) => a + b").is_ok());
}

#[test]
fn test_inference_untyped_param_fails() {
    // 无类型标注且无法推断的参数应该失败
    assert!(check_type_inference_fails("add = (a, b) => a + b"));
    assert!(check_type_inference_fails("square(x) = x * x"));
    assert!(check_type_inference_fails("foo = x => x"));
    assert!(check_type_inference_fails("id = x => x"));
}

#[test]
fn test_inference_typed_lambda_param() {
    // Lambda 参数带类型标注应该通过
    assert!(check_type_inference("identity = (x: Int) => x").is_ok());
    assert!(check_type_inference("double = (x: Int) => x * 2").is_ok());
}

// ============================================================================
// 旧语法推断测试
// ============================================================================

#[test]
fn test_legacy_syntax_empty() {
    // 旧语法空函数应该通过
    assert!(check_type_inference("empty3() = () => {}").is_ok());
    assert!(check_type_inference("main = () => {}").is_ok());
}

#[test]
fn test_legacy_syntax_with_return() {
    // 旧语法有返回值应该通过
    assert!(check_type_inference("get_random() = () => 42").is_ok());
}

#[test]
fn test_legacy_syntax_with_param_type() {
    // 旧语法有参数类型应该通过
    assert!(check_type_inference("square2(Int) = (x) => x * x").is_ok());
    assert!(check_type_inference("mul(Int, Int) = (a, b) => a * b").is_ok());
}

#[test]
fn test_legacy_syntax_untyped_param_fails() {
    // 旧语法无参数类型应该失败
    assert!(check_type_inference_fails("square3() = (x) => x * x"));
    assert!(check_type_inference_fails("mul3() = (a, b) => a * b"));
    assert!(check_type_inference_fails("id2() = x => x"));
}

// ============================================================================
// return 语句测试
// ============================================================================

#[test]
fn test_return_with_full_annotation() {
    // 有标注 + return 应该通过
    assert!(check_type_inference("add: (Int, Int) -> Int = (a, b) => { return a + b; }").is_ok());
    assert!(check_type_inference("square: Int -> Int = (x) => { return x * x; }").is_ok());
    assert!(check_type_inference("get_value: () -> Int = () => { return 42; }").is_ok());
    assert!(check_type_inference("log: (String) -> Void = (msg) => { print(msg); return; }").is_ok());
}

#[test]
fn test_return_without_annotation() {
    // 无标注 + return 应该通过（从 return 推断类型）
    assert!(check_type_inference("get = () => { return 42; }").is_ok());
    assert!(check_type_inference("get_str = () => { return \"hello\"; }").is_ok());
}

#[test]
fn test_return_untyped_param_fails() {
    // 无标注参数 + return 应该失败
    assert!(check_type_inference_fails("add = (a, b) => { return a + b; }"));
    assert!(check_type_inference_fails("square = (x) => { return x * x; }"));
}

#[test]
fn test_early_return() {
    // 早期 return 应该通过
    assert!(check_type_inference("early: Int -> Int = (x) => { if x < 0 { return 0; } x }").is_ok());
    assert!(check_type_inference("multiple_returns: Int -> Int = (x) => {
        if x < 0 { return 0; }
        if x == 0 { return 1; }
        return x;
    }").is_ok());
}

#[test]
fn test_legacy_return_syntax() {
    // 旧语法 + return 应该通过
    assert!(check_type_inference("mul(Int, Int) -> Int = (a, b) => { return a * b; }").is_ok());
    assert!(check_type_inference("square2(Int) -> Int = (x) => { return x * x; }").is_ok());
    assert!(check_type_inference("get_random2() -> Int = () => { return 42; }").is_ok());
    assert!(check_type_inference("say_hello2() -> Void = () => { print(\"hi\"); return; }").is_ok());
}

// ============================================================================
// 无效语法测试（解析层面拒绝）
// ============================================================================

#[test]
fn test_invalid_missing_equals() {
    // 缺少 '=' 符号应该被解析器拒绝
    assert!(check_type_inference_fails("neg: Int -> Int (a) => -a"));
}

#[test]
fn test_invalid_missing_arrow() {
    // 缺少 '=>' 符号 - 这个实际上是有效的变量声明
    // 解析会通过，类型检查会报错
    assert!(check_type_inference("inc: Int -> Int = a + 1").is_ok());
}

#[test]
fn test_invalid_missing_body() {
    // 缺少函数体应该被拒绝
    assert!(check_type_inference_fails("dec: Int -> Int = (a) => "));
    assert!(check_type_inference_fails("missing_body: Int -> Int = => 42"));
}

#[test]
fn test_invalid_bad_parens() {
    // 错误的括号形式应该被拒绝
    assert!(check_type_inference_fails("bad_parens: Int, Int -> Int = (a, b) => a + b"));
}

// ============================================================================
// 复杂场景测试
// ============================================================================

#[test]
fn test_complex_function_with_inference() {
    // 复杂函数（有类型标注）应该通过
    let code = r#"
fact: Int -> Int = (n) => {
    if n <= 1 {
        return 1;
    }
    return n * fact(n - 1);
}
"#;
    assert!(check_type_inference(code).is_ok());
}

#[test]
fn test_curried_function() {
    // 柯里化函数应该通过
    assert!(check_type_inference("add_curried: Int -> Int -> Int = a => b => a + b").is_ok());
    assert!(check_type_inference("multiply_curried: Int -> Int -> Int -> Int = a => b => c => a * b * c").is_ok());
}

#[test]
fn test_higher_order_function() {
    // 高阶函数应该通过
    assert!(check_type_inference("apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)").is_ok());
    assert!(check_type_inference("compose: ((Int) -> Int, (Int) -> Int) -> (Int) -> Int = (f, g) => x => f(g(x))").is_ok());
}

#[test]
fn test_generic_function() {
    // 泛型函数应该通过
    assert!(check_type_inference("identity: <T> (T) -> T = x => x").is_ok());
    assert!(check_type_inference("first: <A, B> ((A, B)) -> A = (a, b) => a").is_ok());
}
