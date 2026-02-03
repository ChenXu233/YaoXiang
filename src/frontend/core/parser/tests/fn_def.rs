//! Function definition parser tests - 函数定义测试

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ast::{StmtKind};
use crate::frontend::core::parser::parse;

#[cfg(test)]
mod fn_def_tests {
    use super::*;

    #[test]
    fn test_parse_fn_def_no_params() {
        let source = "main: () -> () = () => {}";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Fn { name, params, .. } => {
                assert_eq!(name, "main");
                assert!(params.is_empty());
            }
            _ => panic!("Expected Fn statement"),
        }
    }

    #[test]
    fn test_parse_fn_def_with_params() {
        let source = "add: (Int, Int) -> Int = (a, b) => a + b";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        // With standard syntax, function definitions are parsed as StmtKind::Fn
        match &module.items[0].kind {
            StmtKind::Fn { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "a");
                assert_eq!(params[1].name, "b");
            }
            _ => panic!("Expected Fn statement"),
        }
    }

    #[test]
    fn test_parse_complex_fn_def() {
        let source = "add: (Int, Int) -> Int = (a, b) => { return a + b }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn test_parse_generic_param_with_constraint_bracket() {
        let source = "test: [T: Clone](x: T) -> T = x";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Fn {
                name,
                generic_params,
                ..
            } => {
                assert_eq!(name, "test");
                assert_eq!(generic_params.len(), 1);
                assert_eq!(generic_params[0].name, "T");
                assert_eq!(generic_params[0].constraints.len(), 1);
                // Check that constraint is a Type::Name with "Clone"
                match &generic_params[0].constraints[0] {
                    crate::frontend::core::parser::ast::Type::Name(n) => {
                        assert_eq!(n, "Clone");
                    }
                    _ => panic!("Expected Type::Name for constraint"),
                }
            }
            _ => panic!("Expected Fn statement, got {:?}", module.items[0].kind),
        }
    }

    #[test]
    fn test_parse_multiple_generic_params_with_constraints() {
        let source = "pair: [T: Clone, U: Serializable](a: T, b: U) -> (T, U) = (a, b) => (a, b)";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Fn {
                name,
                generic_params,
                ..
            } => {
                assert_eq!(name, "pair");
                assert_eq!(generic_params.len(), 2);
                assert_eq!(generic_params[0].name, "T");
                assert_eq!(generic_params[0].constraints.len(), 1);
                assert_eq!(generic_params[1].name, "U");
                assert_eq!(generic_params[1].constraints.len(), 1);
            }
            _ => panic!("Expected Fn statement"),
        }
    }

    #[test]
    fn test_parse_generic_param_without_constraint() {
        let source = "identity: [T](value: T) -> T = value";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Fn {
                name,
                generic_params,
                ..
            } => {
                assert_eq!(name, "identity");
                assert_eq!(generic_params.len(), 1);
                assert_eq!(generic_params[0].name, "T");
                assert!(generic_params[0].constraints.is_empty());
            }
            _ => panic!("Expected Fn statement"),
        }
    }

    #[test]
    fn test_parse_method_bind_basic() {
        // Basic method binding syntax: Point.draw: (Point, Surface) -> Void = (self, surface) => { ... }
        let source = "Point.draw: (Point, Surface) -> Void = (self, surface) => { }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                method_type,
                params,
                ..
            } => {
                assert_eq!(type_name, "Point");
                assert_eq!(method_name, "draw");
                // Check method type is a function type with 2 params
                if let crate::frontend::core::parser::ast::Type::Fn {
                    params: type_params,
                    ..
                } = method_type
                {
                    assert_eq!(type_params.len(), 2);
                } else {
                    panic!("Expected Fn type for method_type, got {:?}", method_type);
                }
                // Check params
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "self");
                assert_eq!(params[1].name, "surface");
            }
            _ => panic!(
                "Expected MethodBind statement, got {:?}",
                module.items[0].kind
            ),
        }
    }

    #[test]
    fn test_parse_method_bind_with_expression_body() {
        let source =
            "Point.serialize: (Point) -> String = (self) => \"Point(${self.x}, ${self.y})\"";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                params,
                body: (stmts, expr),
                ..
            } => {
                assert_eq!(type_name, "Point");
                assert_eq!(method_name, "serialize");
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "self");
                assert!(stmts.is_empty());
                assert!(expr.is_some());
            }
            _ => panic!("Expected MethodBind statement"),
        }
    }

    #[test]
    fn test_parse_method_bind_with_block_body() {
        let source = r#"
            Point.distance: (Point, Point) -> Float = (self, other) => {
                dx = self.x - other.x
                dy = self.y - other.y
                return (dx * dx + dy * dy).sqrt()
            }
        "#;
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                params,
                body: (stmts, _expr),
                ..
            } => {
                assert_eq!(type_name, "Point");
                assert_eq!(method_name, "distance");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "self");
                assert_eq!(params[1].name, "other");
                // Should have statements (dx = ..., dy = ...) and no expression
                assert!(stmts.len() >= 2);
            }
            _ => panic!("Expected MethodBind statement"),
        }
    }

    #[test]
    fn test_parse_method_bind_no_params() {
        // Method binding with no parameters
        let source = "Point.reset: () -> Void = () => { x = 0 }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                method_type,
                params,
                ..
            } => {
                assert_eq!(type_name, "Point");
                assert_eq!(method_name, "reset");
                // Check method type has 0 params
                if let crate::frontend::core::parser::ast::Type::Fn {
                    params: type_params,
                    ..
                } = method_type
                {
                    assert_eq!(type_params.len(), 0);
                } else {
                    panic!("Expected Fn type for method_type");
                }
                assert_eq!(params.len(), 0);
            }
            _ => panic!("Expected MethodBind statement"),
        }
    }

    #[test]
    fn test_parse_method_bind_complex_types() {
        // Method binding with complex generic types
        let source = "List.map: (List[T], (T) -> U) -> List[U] = (self, mapper) => { }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                params,
                ..
            } => {
                assert_eq!(type_name, "List");
                assert_eq!(method_name, "map");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "self");
                assert_eq!(params[1].name, "mapper");
            }
            _ => panic!("Expected MethodBind statement"),
        }
    }

    #[test]
    fn test_parse_method_bind_with_method_call_in_body() {
        // Method binding with method call in body
        let source = r#"
            Point.add: (Point, Point) -> Point = (self, other) => {
                result = Point(self.x + other.x, self.y + other.y)
                result
            }
        "#;
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                params,
                body: (stmts, expr),
                ..
            } => {
                assert_eq!(type_name, "Point");
                assert_eq!(method_name, "add");
                assert_eq!(params.len(), 2);
                assert!(stmts.len() >= 1);
                assert!(expr.is_some());
            }
            _ => panic!("Expected MethodBind statement"),
        }
    }

    #[test]
    fn test_parse_method_bind_tuple_return() {
        // Method binding with tuple return type
        let source = "Point.decompose: (Point) -> (Float, Float) = (self) => (self.x, self.y)";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                method_type,
                ..
            } => {
                assert_eq!(type_name, "Point");
                assert_eq!(method_name, "decompose");
                // Verify it parsed as Fn type
                if let crate::frontend::core::parser::ast::Type::Fn { return_type, .. } =
                    method_type
                {
                    match **return_type {
                        crate::frontend::core::parser::ast::Type::Tuple(_) => {}
                        _ => panic!("Expected Tuple return type"),
                    }
                } else {
                    panic!("Expected Fn type for method_type");
                }
            }
            _ => panic!("Expected MethodBind statement"),
        }
    }

    #[test]
    fn test_parse_method_bind_and_function_together() {
        // Parse method binding and function definition together
        let source = r#"
            Point.x: (Point) -> Float = (self) => self.x

            get_value: (Int) -> Int = (x) => x + 1
        "#;
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 2);

        // First item should be MethodBind
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                ..
            } => {
                assert_eq!(type_name, "Point");
                assert_eq!(method_name, "x");
            }
            _ => panic!("Expected MethodBind statement as first item"),
        }

        // Second item should be Fn
        match &module.items[1].kind {
            StmtKind::Fn { name, .. } => {
                assert_eq!(name, "get_value");
            }
            _ => panic!("Expected Fn statement as second item"),
        }
    }

    #[test]
    fn test_parse_method_bind_with_ternary_in_body() {
        // Method binding with if expression in body (using block-style if syntax)
        let source = "Number.sign: (Number) -> String = (self) => if self.value > 0 { \"positive\" } else { \"non-positive\" }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                body: (stmts, expr),
                ..
            } => {
                assert_eq!(type_name, "Number");
                assert_eq!(method_name, "sign");
                assert!(stmts.is_empty());
                assert!(expr.is_some());
            }
            _ => panic!("Expected MethodBind statement"),
        }
    }

    #[test]
    fn test_parse_method_bind_option_type() {
        // Method binding with Option type
        let source = "Optional.get_or_default: (Optional[T], T) -> T = (self, default) => { return self.value or default }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::MethodBind {
                type_name,
                method_name,
                params,
                ..
            } => {
                assert_eq!(type_name, "Optional");
                assert_eq!(method_name, "get_or_default");
                assert_eq!(params.len(), 2);
            }
            _ => panic!("Expected MethodBind statement"),
        }
    }
}
