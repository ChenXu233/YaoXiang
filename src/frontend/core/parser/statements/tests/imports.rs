//! Import statement tests — based on spec §7.2

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::ast::StmtKind;

fn parse_use(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors);
    assert_eq!(result.module.items.len(), 1);
    result.module.items.into_iter().next().unwrap().kind
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
