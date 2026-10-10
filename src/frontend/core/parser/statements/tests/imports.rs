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
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"print"));
        assert!(names.contains(&"read"));
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
        let items = items.as_ref().unwrap();
        assert_eq!(
            items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            vec!["helper"],
            "items 应保留原名"
        );
        let aliases = item_aliases.as_ref().unwrap();
        let alias_names: Vec<&str> = aliases
            .iter()
            .map(|a| a.as_ref().map(|s| s.name.as_str()).unwrap_or(""))
            .collect();
        assert_eq!(alias_names, vec!["h"], "item_aliases 应记录内联别名");
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
        let items = items.as_ref().unwrap();
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["helper", "Point"], "items 应按序保留原名");
        let aliases = item_aliases.as_ref().unwrap();
        assert!(
            matches!(&aliases[0], Some(a) if a.name == "h"),
            "带别名项应记录 h，实际: {:?}",
            aliases[0]
        );
        assert!(aliases[1].is_none(), "无别名项应对齐为 None");
    } else {
        panic!("Expected StmtKind::Use");
    }
}

#[test]
fn test_use_items_carry_source_spans() {
    // Arrange / Act - 花括号项与内联别名各带名源位置（#433 余项：导入项跳转）
    let kind = parse_use("use lib.{helper as h, plain}");

    // Assert - span 精确覆盖各标识符 token（同一行，按列界定）
    if let StmtKind::Use {
        items,
        item_aliases,
        ..
    } = &kind
    {
        let items = items.as_ref().unwrap();
        assert_eq!(items.len(), 2, "两项导入");
        let helper = &items[0];
        assert_eq!(helper.name, "helper", "首项为 helper");
        assert_eq!(
            (helper.span.start.line, helper.span.end.line),
            (1, 1),
            "helper span 在第一行"
        );
        assert_eq!(
            (helper.span.start.column, helper.span.end.column),
            (10, 16),
            "helper span 覆盖第 10..16 列（use lib.{{ 后）"
        );
        let plain = &items[1];
        assert_eq!(plain.name, "plain", "次项为 plain");
        assert_eq!(
            (plain.span.start.column, plain.span.end.column),
            (23, 28),
            "plain span 覆盖第 23..28 列"
        );
        let aliases = item_aliases.as_ref().unwrap();
        let h = aliases[0].as_ref().expect("helper 的别名 h 应存在");
        assert_eq!(h.name, "h", "别名为 h");
        assert_eq!(
            (h.span.start.column, h.span.end.column),
            (20, 21),
            "别名 h span 覆盖第 20..21 列"
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
