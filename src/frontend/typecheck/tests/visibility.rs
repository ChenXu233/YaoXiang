//! Cross-module visibility tests

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::typecheck::TypeChecker;

/// Test that private items are not exported
#[test]
fn test_private_items_not_exported() {
    let code = r#"
        foo: (x: Int) -> Int = (x) => x + 1
        bar: (x: Int) -> Int = (x) => x * 2
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Private functions should not be exported
    assert!(
        !checker.env().is_exported("foo"),
        "foo should not be exported"
    );
    assert!(
        !checker.env().is_exported("bar"),
        "bar should not be exported"
    );
}

/// Test that type definitions are automatically exported
#[test]
fn test_type_def_auto_exported() {
    let code = r#"
        Point: Type = Point(x: Float, y: Float)
        Line: Type = Line(start: Point, end: Point)
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Type definitions should be exported by default
    assert!(
        checker.env().is_exported("Point"),
        "Point should be exported"
    );
    assert!(checker.env().is_exported("Line"), "Line should be exported");
}

/// Test explicit method bind visibility
#[test]
fn test_method_bind_visibility() {
    let code = r#"
        Point: Type = Point(x: Float, y: Float)

        Point.distance: (self: Point, other: Point) -> Float = (self, other) => 0.0
        Point.add: (self: Point, other: Point) -> Point = (self, other) => self
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Method binds should be exported
    assert!(
        checker.env().is_exported("Point.distance"),
        "Point.distance should be exported"
    );
    assert!(
        checker.env().is_exported("Point.add"),
        "Point.add should be exported"
    );
}

/// Test pub function with type annotation auto-binds
#[test]
fn test_pub_fn_auto_binds() {
    let code = r#"
        Point: Type = Point(x: Float, y: Float)

        pub distance: (self: Point, other: Point) -> Float = (self, other) => 0.0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Pub function should be exported
    assert!(
        checker.env().is_exported("distance"),
        "distance should be exported"
    );

    // Should also be available as Point.distance
    assert!(
        checker.env().is_exported("Point.distance"),
        "Point.distance should be exported via auto-bind"
    );
}

/// Test local visibility check
#[test]
fn test_local_visibility() {
    let code = r#"
        pub foo: (x: Int) -> Int = (x) => x
        bar: (x: Int) -> Int = (x) => x + 1
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Both should be visible locally (in the same module)
    assert!(
        checker.env().is_visible("foo"),
        "foo should be visible locally"
    );
    assert!(
        checker.env().is_visible("bar"),
        "bar should be visible locally"
    );
}

/// Test type visibility
#[test]
fn test_type_visibility() {
    let code = r#"
        MyInt: Type = Int

        pub create: (x: Int) -> MyInt = (x) => x
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Types should be visible locally
    assert!(
        checker.env().is_visible("MyInt"),
        "MyInt type should be visible locally"
    );
}

/// Test pub function without matching type does not auto-bind
#[test]
fn test_pub_fn_no_auto_bind_without_type() {
    let code = r#"
        pub foo: (x: Int) -> Int = (x) => x
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Function should be exported, but no type to bind to
    assert!(checker.env().is_exported("foo"), "foo should be exported");

    // No auto-binding should happen since there's no type named "Int"
    // (Int is built-in, not user-defined)
    assert!(
        !checker.env().is_exported("Int.foo"),
        "Int.foo should not be created"
    );
}

/// Test multiple pub functions binding to same type
#[test]
fn test_multiple_pub_fn_same_type() {
    let code = r#"
        Point: Type = Point(x: Float, y: Float)

        pub length: (p: Point) -> Float = (p) => 0.0
        pub normalize: (p: Point) -> Point = (p) => p
        pub dot: (a: Point, b: Point) -> Float = (a, b) => 0.0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // All functions should be exported
    assert!(
        checker.env().is_exported("length"),
        "length should be exported"
    );
    assert!(
        checker.env().is_exported("normalize"),
        "normalize should be exported"
    );
    assert!(checker.env().is_exported("dot"), "dot should be exported");

    // All should auto-bind to Point
    assert!(
        checker.env().is_exported("Point.length"),
        "Point.length should be exported"
    );
    assert!(
        checker.env().is_exported("Point.normalize"),
        "Point.normalize should be exported"
    );
    assert!(
        checker.env().is_exported("Point.dot"),
        "Point.dot should be exported"
    );
}

/// Test pub function auto-binds to first param type (Counter)
#[test]
fn test_pub_fn_auto_binds_to_counter() {
    let code = r#"
        Point: Type = Point(x: Float, y: Float)

        pub length: (p: Point) -> Float = (p) => 0.0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(
        result.is_ok(),
        "Type check should succeed, errors: {:?}",
        result.err()
    );

    // Function should be exported
    assert!(
        checker.env().is_exported("length"),
        "length should be exported"
    );

    // Should auto-bind to Point (first param type)
    assert!(
        checker.env().is_exported("Point.length"),
        "Point.length should be created"
    );
}

/// Test method binding gets the correct function type
#[test]
fn test_method_bind_type_preserved() {
    let code = r#"
        Container: Type = Container(items: Vec<Int>)

        Container.size: (c: Container) -> Int = (c) => 0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Method should be exported
    assert!(
        checker.env().is_exported("Container.size"),
        "Container.size should be exported"
    );
}

/// Test that type itself is exported separately from methods
#[test]
fn test_type_and_methods_separate_exports() {
    let code = r#"
        Vector: Type = Vector(x: Float, y: Float)

        pub magnitude: (v:Vector) -> Float = (v) => 0.0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Type should be exported
    assert!(
        checker.env().is_exported("Vector"),
        "Vector should be exported"
    );

    // Method should be exported separately
    assert!(
        checker.env().is_exported("Vector.magnitude"),
        "Vector.magnitude should be exported"
    );
}

/// Test nested type definitions
#[test]
fn test_nested_type_exports() {
    let code = r#"
        Outer: Type = Outer(inner: Inner)
        Inner: Type = Inner(value: Int)

        pub get_value: (o:Outer) -> Int = (o) => 0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Both types should be exported
    assert!(
        checker.env().is_exported("Outer"),
        "Outer should be exported"
    );
    assert!(
        checker.env().is_exported("Inner"),
        "Inner should be exported"
    );

    // Function should bind to Outer
    assert!(
        checker.env().is_exported("Outer.get_value"),
        "Outer.get_value should be exported"
    );
}

/// Test that private function is not exported while pub function is
#[test]
fn test_private_function_not_exported() {
    let code = r#"
        private_val: (x: Int) -> Int = (x) => x + 1
        pub public_val: (x: Int) -> Int = (x) => x * 2
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Private should not be exported
    assert!(
        !checker.env().is_exported("private_val"),
        "private_val should not be exported"
    );

    // Public should be exported
    assert!(
        checker.env().is_exported("public_val"),
        "public_val should be exported"
    );
}

/// Test aliased type definition
#[test]
fn test_aliased_type_exported() {
    let code = r#"
        MyString: Type = String

        pub get_my_string: () -> MyString = () => ""
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Type alias should be exported
    assert!(
        checker.env().is_exported("MyString"),
        "MyString should be exported"
    );

    // Function should be exported
    assert!(
        checker.env().is_exported("get_my_string"),
        "get_my_string should be exported"
    );
}

/// Test enum type export
#[test]
fn test_enum_type_exported() {
    let code = r#"
        Option: Type[T] = Some(T) | None

        pub is_some: (o: Option[Int]) -> Bool = (o) => true
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Enum type should be exported
    assert!(
        checker.env().is_exported("Option"),
        "Option should be exported"
    );
}

/// Test mixed visibility in same module
#[test]
fn test_mixed_visibility() {
    let code = r#"
        PublicType: Type = PublicType(value: Int)
        PrivateType: Type = PrivateType(value: Int)

        pub public_fn: (p: PublicType) -> Int = (p) => 0
        private_fn: (p: PrivateType) -> Int = (p) => 0

        pub use_public: () -> Int = () => 0
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // All types should be exported (default visibility)
    assert!(
        checker.env().is_exported("PublicType"),
        "PublicType should be exported"
    );
    assert!(
        checker.env().is_exported("PrivateType"),
        "PrivateType should be exported"
    );

    // Pub fn should be exported and auto-bound
    assert!(
        checker.env().is_exported("public_fn"),
        "public_fn should be exported"
    );
    assert!(
        checker.env().is_exported("PublicType.public_fn"),
        "PublicType.public_fn should be exported"
    );

    // Private fn should not be exported
    assert!(
        !checker.env().is_exported("private_fn"),
        "private_fn should not be exported"
    );

    // Another pub fn should be exported
    assert!(
        checker.env().is_exported("use_public"),
        "use_public should be exported"
    );
}

/// Test method binding with complex parameter types
#[test]
fn test_method_complex_params() {
    let code = r#"
        Tree: Type = Tree(left: Tree, right: Tree, value: Int)

        pub height: (t: Tree) -> Int = (t) => 0
        pub sum: (t: Tree) -> Int = (t) => 0
        pub contains: (t: Tree, v: Int) -> Bool = (t, v) => true
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // All functions should bind to Tree
    assert!(
        checker.env().is_exported("Tree.height"),
        "Tree.height should be exported"
    );
    assert!(
        checker.env().is_exported("Tree.sum"),
        "Tree.sum should be exported"
    );
    assert!(
        checker.env().is_exported("Tree.contains"),
        "Tree.contains should be exported"
    );
}

/// Test that visibility works with type inference
#[test]
fn test_visibility_with_inference() {
    let code = r#"
        pub calculate = (x: Int, y: Int) => x + y
        private_helper = (x: Int) => x * 2
    "#;

    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();

    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);

    assert!(result.is_ok(), "Type check should succeed");

    // Pub function should be exported
    assert!(
        checker.env().is_exported("calculate"),
        "calculate should be exported"
    );

    // Private should not be exported
    assert!(
        !checker.env().is_exported("private_helper"),
        "private_helper should not be exported"
    );

    // Both should be visible locally
    assert!(
        checker.env().is_visible("calculate"),
        "calculate should be visible locally"
    );
    assert!(
        checker.env().is_visible("private_helper"),
        "private_helper should be visible locally"
    );
}
