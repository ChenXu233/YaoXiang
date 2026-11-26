//! 绑定力单一源的声明面测试（pratt/precedence.rs 的算子表）。
//!
//! 条款来源：SPEC syntax §2.2 的优先级表（级号与「结合性」列）。
//! 本文件只断言**语言级层级关系**与**表的内部一致性**，不断言具体数值：
//! 按 coding-rules 7.1，冻数值的实现选择会随一次正确的重排变红。
//!
//! 绑定力是编译期常量，比较常量的断言对 clippy 是常量断言（本文件正是要断言
//! 常量之间的层级关系），故文件级豁免。
#![allow(clippy::assertions_on_constants)]

use crate::ast::BinOp;
use crate::frontend::core::lexer::tokens::TokenKind;
use crate::frontend::core::parser::pratt::precedence::{
    infix_ast_op, infix_op, prefix_op, InfixStrategy, BP_ADD, BP_AND, BP_ASSIGN, BP_BITWISE,
    BP_CALL, BP_CAST, BP_EQUALITY, BP_HIGHEST, BP_LOWEST, BP_MUL, BP_OR, BP_PATTERN_PROBE,
    BP_SHIFT, BP_UNARY,
};

#[test]
fn test_bp_ordering_follows_spec_levels() {
    // SPEC §2.2：级号越大绑定越松；等价地左值越大绑定越紧。
    assert!(
        BP_LOWEST < BP_ASSIGN,
        "入口必须低于赋值：{BP_LOWEST} vs {BP_ASSIGN}"
    );
    assert!(
        BP_ASSIGN < BP_OR,
        "赋值（级 13）必须低于 or（级 11）：{BP_ASSIGN} vs {BP_OR}"
    );
    assert!(
        BP_OR < BP_AND,
        "or（级 11）低于 and（级 10）：{BP_OR} vs {BP_AND}"
    );
    assert!(
        BP_AND < BP_EQUALITY,
        "and（级 10）必须低于相等/比较（级 9）：{BP_AND} vs {BP_EQUALITY}"
    );
    assert!(
        BP_EQUALITY < BP_BITWISE,
        "级 9 必须低于位运算（级 8）：{BP_EQUALITY} vs {BP_BITWISE}"
    );
    assert!(
        BP_BITWISE < BP_SHIFT,
        "级 8 必须低于移位（级 7）：{BP_BITWISE} vs {BP_SHIFT}"
    );
    assert!(
        BP_SHIFT <= BP_ADD,
        "级 7 必须不高于加减（级 5）：{BP_SHIFT} vs {BP_ADD}"
    );
    assert!(
        BP_ADD < BP_MUL,
        "级 5 必须低于乘除（级 4）：{BP_ADD} vs {BP_MUL}"
    );
    assert!(
        BP_MUL < BP_CAST,
        "级 4 必须低于 as（级 2）：{BP_MUL} vs {BP_CAST}"
    );
    assert!(
        BP_CAST < BP_CALL,
        "级 2 必须低于调用/字段/索引（级 1）：{BP_CAST} vs {BP_CALL}"
    );
    assert!(
        BP_CALL < BP_HIGHEST,
        "调用层必须低于原子层：{BP_CALL} vs {BP_HIGHEST}"
    );
    assert!(
        BP_HIGHEST < BP_PATTERN_PROBE,
        "模式阈值必须高于任何中缀：{BP_HIGHEST} vs {BP_PATTERN_PROBE}"
    );
}

#[test]
fn test_infix_table_entries_are_self_consistent() {
    for entry in crate::frontend::core::parser::pratt::precedence::INFIX_OPS {
        let expected_right = match entry.strategy {
            InfixStrategy::Binary | InfixStrategy::In | InfixStrategy::Shift => entry.left_bp + 1,
            InfixStrategy::Assign => entry.left_bp,
            InfixStrategy::Lambda => 1,
            InfixStrategy::Call
            | InfixStrategy::Field
            | InfixStrategy::Index
            | InfixStrategy::Try
            | InfixStrategy::Cast => entry.left_bp + 1,
        };
        assert_eq!(
            entry.right_bp, expected_right,
            "{:?} 的右绑定力必须由结合性派生（左结合 +1 / 右结合同值）：{entry:?}",
            entry.token
        );
        assert!(
            entry.left_bp >= BP_ASSIGN,
            "{:?} 的左绑定力不得低于赋值层：{entry:?}",
            entry.token
        );
        assert!(
            entry.left_bp < BP_PATTERN_PROBE,
            "{:?} 的左绑定力必须低于模式阈值：{entry:?}",
            entry.token
        );
    }
}

#[test]
fn test_infix_lookup_returns_the_table_entry() {
    for entry in crate::frontend::core::parser::pratt::precedence::INFIX_OPS {
        if entry.double_token {
            // `<<` / `>>` 与比较级共享 Lt/Gt token：先命中的是单 token 比较行。
            continue;
        }
        let found = infix_op(&entry.token).unwrap_or_else(|| panic!("表行必须可查：{entry:?}"));
        assert_eq!(
            (found.left_bp, found.right_bp, found.strategy),
            (entry.left_bp, entry.right_bp, entry.strategy),
            "{:?} 的查表结果必须与表行一致",
            entry.token
        );
    }
    assert!(
        infix_op(&TokenKind::FatArrow).is_some(),
        "lambda 中缀必须在表里"
    );
    assert!(
        infix_op(&TokenKind::Eof).is_none(),
        "非算子 token 不得查出行"
    );
}

#[test]
fn test_infix_ast_op_matches_table() {
    let expected = [
        (TokenKind::Plus, Some(BinOp::Add)),
        (TokenKind::Minus, Some(BinOp::Sub)),
        (TokenKind::Star, Some(BinOp::Mul)),
        (TokenKind::Slash, Some(BinOp::Div)),
        (TokenKind::Percent, Some(BinOp::Mod)),
        (TokenKind::EqEq, Some(BinOp::Eq)),
        (TokenKind::Neq, Some(BinOp::Neq)),
        (TokenKind::Le, Some(BinOp::Le)),
        (TokenKind::Ge, Some(BinOp::Ge)),
        (TokenKind::Ampersand, Some(BinOp::BitAnd)),
        (TokenKind::Pipe, Some(BinOp::BitOr)),
        (TokenKind::Caret, Some(BinOp::BitXor)),
        (TokenKind::DotDot, Some(BinOp::Range)),
        (TokenKind::Eq, Some(BinOp::Assign)),
        (TokenKind::KwIn, None),
        (TokenKind::KwAs, None),
    ];
    for (token, op) in expected {
        assert_eq!(infix_ast_op(&token), op, "{token:?} 的 AST 表示必须来自表");
    }
}

#[test]
fn test_prefix_table_entries_share_the_unary_level() {
    for entry in crate::frontend::core::parser::pratt::precedence::PREFIX_OPS {
        assert_eq!(
            entry.bp, BP_UNARY,
            "{:?} 必须在前缀层：{entry:?}",
            entry.token
        );
        let found =
            prefix_op(&entry.token).unwrap_or_else(|| panic!("前缀表行必须可查：{entry:?}"));
        assert_eq!(
            found.strategy, entry.strategy,
            "{:?} 的前缀策略必须一致",
            entry.token
        );
    }
    assert!(
        prefix_op(&TokenKind::Identifier("x".to_string())).is_none(),
        "标识符不是前缀算子"
    );
}
