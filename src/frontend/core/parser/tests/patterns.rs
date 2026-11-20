//! 模式解析测试 — 基于 SPEC syntax §2.8（模式匹配）与 issue #405（RangePattern）
//!
//! §2.8: `Pattern ::= ... | RangePattern`，`RangePattern ::= Literal '..' Literal`
//! issue #405: `80..89 => "B"` 合法化；端点为字面量（负号前缀归一化为负 Int），
//! 非常量端点在模式位是绑定名，语法层点名拒绝（E0010）。

use crate::ast::{Expr, Pattern, StmtKind};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::{parse_with_probe, NullProbe};

/// 解析完整程序并返回最后一个表达语句的 match 臂列表。
///
/// 夹具形态：顶层绑定 `r = match scrutinee { arms }`（script 模式顶层语句）。
fn parse_match_arms(source: &str) -> Vec<Pattern> {
    let tokens = tokenize(source).expect("tokenize failed");
    let parsed = parse_with_probe(&tokens, &NullProbe);
    assert!(!parsed.has_errors, "parse failed: {:?}", parsed.errors);
    let mut arms = Vec::new();
    for stmt in &parsed.module.items {
        if let StmtKind::Assign { value, .. } = &stmt.kind {
            if let Some(Expr::Match {
                arms: match_arms, ..
            }) = value.as_deref()
            {
                arms.extend(match_arms.iter().map(|a| a.pattern.clone()));
            }
        }
    }
    arms
}

/// 解析完整程序并返回全部诊断码（拒绝形态用例）。
fn parse_error_codes(source: &str) -> Vec<String> {
    let tokens = tokenize(source).expect("tokenize failed");
    let parsed = parse_with_probe(&tokens, &NullProbe);
    parsed.errors.iter().map(|e| e.code.clone()).collect()
}

#[test]
fn test_match_range_pattern_parses_to_range_arm() {
    // Arrange：issue #405 原始夹具——`80..89 => "B"` 此前报 E0010
    let source = r#"
        r = match 85 {
            80..89 => "B",
            _ => "F",
        }
    "#;

    // Act
    let arms = parse_match_arms(source);

    // Assert：首臂是 Range{Int(80), Int(89)}，末臂保持通配
    assert_eq!(arms.len(), 2, "fixture should have two arms");
    assert!(
        matches!(
            &arms[0],
            Pattern::Range {
                lo: Literal::Int(80),
                hi: Literal::Int(89)
            }
        ),
        "first arm should be Range{{80, 89}}, got {:?}",
        arms[0]
    );
    assert!(
        matches!(&arms[1], Pattern::Wildcard),
        "fallback arm unchanged"
    );
}

#[test]
fn test_match_range_pattern_negative_bounds_normalized() {
    // Arrange：负端点——词法器字面量恒非负，负值由 parser 归一化
    let source = r#"
        r = match x {
            -5..-1 => "neg",
            _ => "other",
        }
    "#;

    // Act
    let arms = parse_match_arms(source);

    // Assert：两端点为负 Int 字面量
    assert!(
        matches!(
            &arms[0],
            Pattern::Range {
                lo: Literal::Int(-5),
                hi: Literal::Int(-1)
            }
        ),
        "negative bounds should be normalized to Int(-5)..Int(-1), got {:?}",
        arms[0]
    );
}

#[test]
fn test_match_range_pattern_in_or_alternatives() {
    // Arrange：范围臂作或模式备选（RFC-010b 或模式收链不因范围臂变形）
    let source = r#"
        r = match x {
            80..89 | 90..99 => "both",
            _ => "other",
        }
    "#;

    // Act
    let arms = parse_match_arms(source);

    // Assert：或模式内两个范围备选
    let ok = matches!(
        &arms[0],
        Pattern::Or(ps) if ps.len() == 2
            && matches!(&ps[0], Pattern::Range { lo: Literal::Int(80), hi: Literal::Int(89) })
            && matches!(&ps[1], Pattern::Range { lo: Literal::Int(90), hi: Literal::Int(99) })
    );
    assert!(
        ok,
        "or chain should hold two range alternatives, got {:?}",
        arms[0]
    );
}

#[test]
fn test_match_range_pattern_with_guard() {
    // Arrange：范围臂带守卫（`pat if cond => ...` 的守卫解包不因范围臂变形）
    let source = r#"
        r = match x {
            80..89 if x > 85 => "high B",
            _ => "other",
        }
    "#;

    // Act
    let arms = parse_match_arms(source);

    // Assert：Guard 包裹 Range
    let ok = matches!(
        &arms[0],
        Pattern::Guard { pattern, .. }
            if matches!(pattern.as_ref(), Pattern::Range { lo: Literal::Int(80), hi: Literal::Int(89) })
    );
    assert!(ok, "guard should wrap the range pattern, got {:?}", arms[0]);
}

#[test]
fn test_match_range_pattern_non_literal_bound_rejected() {
    // Arrange：标识符端点——模式位标识符是绑定，无范围判定语义
    let source = r#"
        r = match x {
            n..20 => "bad",
            _ => "other",
        }
    "#;

    // Act
    let codes = parse_error_codes(source);

    // Assert：E0010 点名范围端点要求
    assert!(
        codes.iter().any(|c| c == "E0010"),
        "non-literal bound should report E0010, got {:?}",
        codes
    );
}

#[test]
fn test_match_range_pattern_non_literal_upper_bound_rejected() {
    // Arrange：上界标识符端点——下界形态 `n..20` 已有锚，此处补对称的上界
    // 形态 `80..n`。判据门（条款）：SPEC syntax §2.8 `RangePattern` 端点为
    // 字面量，issue #405 语义登记「标识符端点在模式位是绑定名，无判定语义，
    // 语法层拒绝（E0010）」——上界端点由 parse_range_bound 直读，返回 None
    // 的调用点发射 E0010（见 nud.rs「range pattern upper bound」发射点）。
    let source = r#"
        r = match x {
            80..n => "bad",
            _ => "other",
        }
    "#;

    // Act
    let codes = parse_error_codes(source);

    // Assert：E0010 点名范围上界端点要求
    assert!(
        codes.iter().any(|c| c == "E0010"),
        "non-literal upper bound should report E0010, got {:?}",
        codes
    );
}
