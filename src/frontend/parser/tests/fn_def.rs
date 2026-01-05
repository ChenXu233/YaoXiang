use crate::frontend::lexer::tokenize;
use crate::frontend::parser::ast::{Expr, StmtKind};
use crate::frontend::parser::parse;

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
        },
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
        },
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
