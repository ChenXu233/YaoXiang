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
        let source = "add: (Int, Int) -> Int = (a, b) => { a + b }";
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
            StmtKind::Fn { name, generic_params, .. } => {
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
            StmtKind::Fn { name, generic_params, .. } => {
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
            StmtKind::Fn { name, generic_params, .. } => {
                assert_eq!(name, "identity");
                assert_eq!(generic_params.len(), 1);
                assert_eq!(generic_params[0].name, "T");
                assert!(generic_params[0].constraints.is_empty());
            }
            _ => panic!("Expected Fn statement"),
        }
    }
}
