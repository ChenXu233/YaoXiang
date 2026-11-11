//! Function definition parsing tests — based on spec §6.1–§6.4

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::ast::{Expr, StmtKind};

fn parse_fn(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors, "解析不应有错误");
    assert_eq!(result.module.items.len(), 1);
    result.module.items.into_iter().next().unwrap().kind
}

/// 旧函数定义语法的块体形态 `f: () -> Int = { 5 }`：带 Fn 注解的块被收编为
/// `Lambda`（空参数表 + 块体），不构造已删除的 FnDef 变体（RFC-039 6.5.1）。
#[test]
fn test_fn_block_body_with_fn_annotation_parses_to_lambda() {
    // Arrange
    let kind = parse_fn("f: () -> Int = { 5 }");

    // Act
    let StmtKind::Assign { target, value, .. } = &kind else {
        panic!("Expected Assign");
    };

    // Assert
    assert!(
        matches!(target.as_ref(), Expr::Var(name, _) if name == "f"),
        "左值应为变量 f，实际: {target:?}"
    );
    assert!(
        matches!(
            value.as_deref(),
            Some(Expr::Lambda { params, .. }) if params.is_empty()
        ),
        "块体函数定义的 value 应为 Lambda（空参数表），实际: {value:?}"
    );
}
