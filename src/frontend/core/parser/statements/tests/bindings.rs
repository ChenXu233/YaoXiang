//! Method binding tests — based on RFC-004

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{parse, ParseError};
use crate::frontend::core::parser::ast::{StmtKind, BindingKind};

fn parse_binding(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    assert_eq!(module.items.len(), 1);
    module.items.into_iter().next().unwrap().kind
}

#[test]
fn test_external_binding_single_pos() {
    let kind = parse_binding("Point.distance = distance[0]");
    if let StmtKind::ExternalBindingStmt {
        type_name,
        method_name,
        binding,
    } = &kind
    {
        assert_eq!(type_name, "Point");
        assert_eq!(method_name, "distance");
        if let BindingKind::External { positions, .. } = binding {
            assert_eq!(positions, &vec![0]);
        } else {
            panic!("Expected External binding");
        }
    } else {
        panic!("Expected ExternalBindingStmt");
    }
}

#[test]
fn test_external_binding_multi_pos() {
    let kind = parse_binding("Point.calc = calculate[1, 2]");
    if let StmtKind::ExternalBindingStmt {
        type_name,
        method_name,
        binding,
    } = &kind
    {
        assert_eq!(type_name, "Point");
        if let BindingKind::External { positions, .. } = binding {
            assert_eq!(positions, &vec![1, 2]);
        } else {
            panic!("Expected External binding with positions");
        }
    }
}

#[test]
fn test_external_binding_default() {
    let kind = parse_binding("Point.method = some_function");
    if let StmtKind::ExternalBindingStmt { binding, .. } = &kind {
        assert!(matches!(binding, BindingKind::DefaultExternal { .. }));
    } else {
        panic!("Expected ExternalBindingStmt");
    }
}
