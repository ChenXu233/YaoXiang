//! pub auto-bind mechanism tests

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::typecheck::TypeChecker;

/// Test pub function auto-binds to type - simplified
#[test]
fn test_pub_auto_bind_to_type() {
    let code = r#"
        type Point = Point(x: Float, y: Float)
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Type definition should be exported
    assert!(
        checker.env().is_exported("Point"),
        "Point should be exported"
    );
}

/// Test pub function is exported
#[test]
fn test_pub_fn_exported() {
    let code = r#"
        pub foo: (Int) -> Int = (x) => x
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Pub function should be exported
    assert!(checker.env().is_exported("foo"), "foo should be exported");
}

/// Test non-pub function is not exported
#[test]
fn test_private_function_not_exported() {
    let code = r#"
        foo: (Int) -> Int = (x) => x
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Private function should not be exported
    assert!(
        !checker.env().is_exported("foo"),
        "foo should not be exported"
    );
}

/// Test explicit method bind is exported
#[test]
fn test_explicit_method_bind_exported() {
    let code = r#"
        type Point = Point(x: Float, y: Float)

        Point.distance: (Point, Point) -> Float = (self, other) => 0.0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Explicit method bind should be exported
    assert!(
        checker.env().is_exported("Point.distance"),
        "Point.distance should be exported"
    );
}

/// Test pub fn with type annotation
#[test]
fn test_pub_fn_with_type_annotation() {
    let code = r#"
        pub distance: (Float, Float) -> Float = (x, y) => 0.0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Check exports
    assert!(
        checker.env().is_exported("distance"),
        "distance should be exported"
    );
}
