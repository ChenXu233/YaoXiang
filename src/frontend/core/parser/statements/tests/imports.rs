//! Import statement tests — based on spec §7.2

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::parser::ast::StmtKind;

fn parse_use(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors);
    assert_eq!(result.module.items.len(), 1);
    result.module.items.into_iter().next().unwrap().kind
}

// ============================================================================
// use 语句各形式 (Spec §7.2)
// ============================================================================

#[test]
fn test_use_simple_path() {
    // use path;
    let kind = parse_use("use std.io");
    if let StmtKind::Use {
        path, items, alias, ..
    } = &kind
    {
        assert_eq!(path, "std.io");
        assert!(items.is_none());
        assert!(alias.is_none());
    } else {
        panic!("Expected StmtKind::Use");
    }
}

#[test]
fn test_use_with_items() {
    // use path.{a, b};
    let kind = parse_use("use std.io.{print, read}");
    if let StmtKind::Use { items, .. } = &kind {
        let items = items.as_ref().unwrap();
        assert!(items.contains(&"print".to_string()));
        assert!(items.contains(&"read".to_string()));
    } else {
        panic!("Expected StmtKind::Use");
    }
}

#[test]
fn test_use_with_alias() {
    // use path as alias;
    let kind = parse_use("use std.io as io");
    if let StmtKind::Use { alias, items, .. } = &kind {
        assert!(items.is_none());
        assert_eq!(alias.as_ref().unwrap(), &vec!["io".to_string()]);
    } else {
        panic!("Expected StmtKind::Use");
    }
}

#[test]
fn test_use_deep_path() {
    let kind = parse_use("use a.b.c.d");
    if let StmtKind::Use { path, .. } = &kind {
        assert_eq!(path, "a.b.c.d");
    } else {
        panic!("Expected StmtKind::Use");
    }
}
