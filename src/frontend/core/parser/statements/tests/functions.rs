//! Function definition parsing tests — based on spec §6.1–§6.4

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::parser::ast::StmtKind;

fn parse_fn(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    assert_eq!(module.items.len(), 1);
    module.items.into_iter().next().unwrap().kind
}

#[test]
fn test_fn_no_params_block_body() {
    let kind = parse_fn("f = () => { return 1 }");
    if let StmtKind::Binding { name, params, .. } = &kind {
        assert_eq!(name, "f");
        assert!(params.is_empty());
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_fn_one_param_expr_body() {
    let kind = parse_fn("inc = (x) => x + 1");
    if let StmtKind::Binding { name, params, .. } = &kind {
        assert_eq!(name, "inc");
        assert_eq!(params.len(), 1);
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_fn_multi_param() {
    let kind = parse_fn("add = (a, b) => a + b");
    if let StmtKind::Binding { params, .. } = &kind {
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[1].name, "b");
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_fn_typed_params() {
    let kind = parse_fn("add: (a: Int, b: Int) -> Int = (a, b) => a + b");
    if let StmtKind::Binding { params, .. } = &kind {
        assert_eq!(params.len(), 2);
        assert!(params[0].ty.is_some());
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_fn_mut_param() {
    let kind = parse_fn("inc: (mut x: Int) -> Int = (mut x) => x + 1");
    if let StmtKind::Binding { params, .. } = &kind {
        assert!(params[0].is_mut);
    } else {
        panic!("Expected Binding");
    }
}
