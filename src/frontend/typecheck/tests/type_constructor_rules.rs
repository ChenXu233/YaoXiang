//! 端到端验证：类型构造器与代码块函数的判定规则

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::typecheck::TypeChecker;
use crate::util::diagnostic::Diagnostic;

fn parse_module(code: &str) -> Result<crate::frontend::core::parser::ast::Module, String> {
    let tokens = tokenize(code).map_err(|e| format!("Lexer error: {e:?}"))?;
    parse(&tokens).map_err(|e| format!("Parse error: {e:?}"))
}

fn typecheck_ok(code: &str) -> bool {
    let module = match parse_module(code) {
        Ok(m) => m,
        Err(_) => return false,
    };

    let mut checker = TypeChecker::new("test");
    checker.check_module(&module).is_ok()
}

fn typecheck_err_codes(code: &str) -> Vec<String> {
    let module = parse_module(code).expect("parse should succeed");
    let mut checker = TypeChecker::new("test");
    checker
        .check_module(&module)
        .expect_err("typecheck should fail")
        .into_iter()
        .map(|d| d.code)
        .collect()
}

fn typecheck_errs(code: &str) -> Vec<Diagnostic> {
    let module = parse_module(code).expect("parse should succeed");
    let mut checker = TypeChecker::new("test");
    checker
        .check_module(&module)
        .expect_err("typecheck should fail")
}

#[test]
fn test_type_constructor_point_passes() {
    assert!(typecheck_ok("Point: Type = { x: Float, y: Float }"));
}

#[test]
fn test_generic_type_constructor_list_passes() {
    assert!(typecheck_ok("List: (T: Type) -> Type = { data: Array[T] }"));
}

#[test]
fn test_type_constructor_with_default_passes() {
    assert!(typecheck_ok("Id: (T: Type) -> Type = { x: T }"));
}

#[test]
fn test_block_function_not_type_constructor() {
    // 带有函数体的不应该是类型构造器
    assert!(typecheck_ok("inc: (x: Int) -> Int = { x + 1 }"));
}

#[test]
fn test_lambda_function_not_type_constructor() {
    // Lambda 函数不应该是类型构造器
    assert!(typecheck_ok("add: (a: Int, b: Int) -> Int = (a, b) => a + b"));
}

#[test]
fn test_empty_block_variable_with_annotation() {
    // x: Int = {} 应该报错，因为 {} 是 Void 而不是 Int
    let errs = typecheck_err_codes("main: () -> Void = { x: Int = {} }");
    assert!(!errs.is_empty(), "x: Int = {{}} must produce a type-check error");
}

#[test]
fn test_empty_block_variable_without_annotation() {
    // x = {} 应该推断为 Void
    assert!(typecheck_ok("main: () -> Void = { x = {}, return x }"));
}
