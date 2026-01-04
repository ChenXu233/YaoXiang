//! 类型推断测试
//!
//! 测试类型检查器的推断功能

use crate::frontend::parser::parse;
use crate::frontend::lexer::tokenize;
use crate::frontend::typecheck::{check_module, TypeEnvironment};

/// 检查类型推断是否成功
fn check_type_inference(input: &str) -> Result<(), String> {
    eprintln!("[TEST] Input: {}", input);
    let tokens = tokenize(input).map_err(|e| format!("Lexer error: {:?}", e))?;
    let ast = parse(&tokens).map_err(|e| format!("Parse error: {:?}", e))?;
    eprintln!("[TEST] AST items: {}", ast.items.len());
    for (i, item) in ast.items.iter().enumerate() {
        eprintln!("[TEST]   Item {}: {:?}", i, std::mem::discriminant(&item.kind));
    }
    let mut env = TypeEnvironment::new();
    match check_module(&ast, Some(&mut env)) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("[TEST] Type errors: {:?}", e);
            Err(format!("Type error: {:?}", e))
        }
    }
}

/// 检查类型推断是否失败（预期错误）
fn check_type_inference_fails(input: &str) -> bool {
    eprintln!("[TEST] Checking if fails: {}", input);
    let tokens = tokenize(input).ok().map(|t| t.clone());
    if let Some(ref tokens) = tokens {
        eprintln!("[TEST]   Tokens: {:?}", tokens.len());
    }
    let result = check_type_inference(input);
    eprintln!("[TEST]   Result: {:?}", result.is_err());
    result.is_err()
}

// ============================================================================
// 标准语法测试（完整类型标注）
// ============================================================================

#[test]
fn test_standard_syntax_with_full_annotation() {
    // 完整类型标注应该通过
    assert!(check_type_inference("add: (Int, Int) -> Int = (a, b) => a + b").is_ok(), "multi-param function");
    assert!(check_type_inference("log: (String) -> Void = (msg) => print(msg)").is_ok(), "void return");
    assert!(check_type_inference("get_val: () -> Int = () => 42").is_ok(), "no params");
    assert!(check_type_inference("empty: () -> Void = () => {}").is_ok(), "empty body");
}

#[test]
fn test_single_param_function() {
    // 单参数函数测试
    assert!(check_type_inference("square: (Int) -> Int = (x) => x * x").is_ok(), "single param with parens");
}

// ============================================================================
// 新语法推断测试（无类型标注）
// ============================================================================

#[test]
fn test_inference_empty_block() {
    // 空块 {} 推断为 Void
    assert!(check_type_inference("main = () => {}").is_ok(), "empty block");
}

#[test]
fn test_inference_expression_return() {
    // 从表达式推断返回类型
    assert!(check_type_inference("get_num = () => 42").is_ok(), "int literal");
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
    // 注意：这些是"旧语法"形式，参数 x 没有类型标注
    assert!(check_type_inference_fails("square(x) = x * x"), "one untyped param (legacy syntax)");
    assert!(check_type_inference_fails("add(a, b) = a + b"), "two untyped params (legacy syntax)");
    // 新语法形式，参数无标注
    assert!(check_type_inference_fails("add = (a, b) => a + b"), "two untyped params (new syntax)");
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
}

// ============================================================================
// return 语句测试
// ============================================================================

#[test]
fn test_return_with_full_annotation() {
    // 有标注 + return 应该通过
    assert!(check_type_inference("add: (Int, Int) -> Int = (a, b) => { return a + b; }").is_ok());
    assert!(check_type_inference("get_value: () -> Int = () => { return 42; }").is_ok());
}

#[test]
fn test_return_without_annotation() {
    // 无标注 + return 应该通过（从 return 推断类型）
    assert!(check_type_inference("get = () => { return 42; }").is_ok());
}

#[test]
fn test_return_untyped_param_fails() {
    // 无标注参数 + return 应该失败
    assert!(check_type_inference_fails("add = (a, b) => { return a + b; }"));
}

#[test]
fn test_early_return() {
    // 早期 return 应该通过
    assert!(check_type_inference("early: Int -> Int = (x) => { if x < 0 { return 0; } x }").is_ok());
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
fn test_invalid_missing_body() {
    // 缺少函数体应该被拒绝
    assert!(check_type_inference_fails("dec: Int -> Int = (a) => "));
}

#[test]
fn test_invalid_bad_parens() {
    // 错误的括号形式应该被拒绝
    assert!(check_type_inference_fails("bad_parens: Int, Int -> Int = (a, b) => a + b"));
}
