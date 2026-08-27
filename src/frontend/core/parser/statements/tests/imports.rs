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

// use 语句各形式 (Spec §7.2)

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

// use 条目别名（#245 / RFC-029 Phase 4）

#[test]
fn test_use_item_inline_alias() {
    // Arrange / Act - use path.{item as alias};
    let kind = parse_use("use lib.{helper as h}");

    // Assert - items 记录原名，item_aliases 对齐记录别名
    if let StmtKind::Use {
        items,
        item_aliases,
        ..
    } = &kind
    {
        assert_eq!(
            items.as_ref().unwrap(),
            &vec!["helper".to_string()],
            "items 应保留原名"
        );
        assert_eq!(
            item_aliases.as_ref().unwrap(),
            &vec![Some("h".to_string())],
            "item_aliases 应记录内联别名"
        );
    } else {
        panic!("Expected StmtKind::Use");
    }
}

#[test]
fn test_use_item_mixed_alias_and_plain() {
    // Arrange / Act - 混合：带别名项 + 无别名项
    let kind = parse_use("use lib.{helper as h, Point}");

    // Assert - 无别名项对齐为 None
    if let StmtKind::Use {
        items,
        item_aliases,
        ..
    } = &kind
    {
        assert_eq!(
            items.as_ref().unwrap(),
            &vec!["helper".to_string(), "Point".to_string()],
            "items 应按序保留原名"
        );
        assert_eq!(
            item_aliases.as_ref().unwrap(),
            &vec![Some("h".to_string()), None],
            "无别名项应对齐为 None"
        );
    } else {
        panic!("Expected StmtKind::Use");
    }
}

#[test]
fn test_use_items_no_alias_gives_none_item_aliases() {
    // Arrange / Act - 普通花括号导入不应产生 item_aliases
    let kind = parse_use("use lib.{a, b}");

    // Assert
    if let StmtKind::Use { item_aliases, .. } = &kind {
        assert!(
            item_aliases.is_none(),
            "无内联别名时 item_aliases 应为 None，实际: {:?}",
            item_aliases
        );
    } else {
        panic!("Expected StmtKind::Use");
    }
}

#[test]
fn test_use_positional_item_alias_rejected() {
    // Arrange - 位置式条目别名不在 RFC-029 语法表内
    let tokens = tokenize("use lib.{helper} as h").unwrap();

    // Act
    let result = parse(&tokens);

    // Assert - 应报错并指向内联形式
    assert!(result.has_errors, "位置式条目别名应被拒绝");
}
