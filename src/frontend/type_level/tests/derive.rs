//! Derive 宏测试

use crate::frontend::type_level::derive::{DeriveGenerator, DeriveParser};
use crate::frontend::type_level::trait_bounds::TraitTable;

#[test]
fn test_parse_derive_simple() {
    let tokens = vec![
        crate::frontend::core::lexer::tokens::Token {
            kind: crate::frontend::core::lexer::tokens::TokenKind::Identifier("Clone".to_string()),
            span: crate::util::span::Span::dummy(),
        },
    ];

    let result = DeriveParser::parse_derive(&tokens);
    assert_eq!(result, Some(vec!["Clone".to_string()]));
}

#[test]
fn test_can_derive() {
    let table = TraitTable::default();
    let gens = DeriveGenerator::new(&table);

    assert!(gens.can_derive("Clone"));
    assert!(gens.can_derive("Copy"));
    assert!(!gens.can_derive("Unknown"));
}
