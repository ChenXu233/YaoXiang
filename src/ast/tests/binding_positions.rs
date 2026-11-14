//! RFC-004 绑定位置提取测试 — `Expr::extract_binding_positions`（ast/mod.rs）
//!
//! 规范来源:
//! - `docs/src/rfc/accepted/004-curry-multi-position-binding.md` 附录 A
//!   （负数索引支持，从末尾计数）与运行时行为表（`func[-1]`）
//! - E1064 绑定位置索引无效（RFC-004）：极端值由 checker 的
//!   `normalize_binding_positions` 既有校验拒绝，提取器不在此判域
//!
//! 覆盖: 负位提取语义（issue #560：取负臂 checked 化，i64::MIN 形态不再 panic）
//! 与正位/元组提取对照。

use crate::ast::{Expr, Literal, UnOp};
use crate::util::span::Span;

fn int_lit(n: i128) -> Expr {
    Expr::Lit(Literal::Int(n), Span::dummy())
}

fn neg_int_lit(n: i128) -> Expr {
    Expr::UnOp {
        op: UnOp::Neg,
        expr: Box::new(int_lit(n)),
        span: Span::dummy(),
    }
}

#[test]
fn test_extract_binding_positions_neg_i64_min_form_returns_min_not_panic() {
    // Arrange — 用户源码 `f[-9223372036854775808]` 的 AST 形态：Neg(Lit(2^63))。
    // 修前裸 `-(i64::MIN)` 在溢出检查构建下 panic（探针实测 exit 101，
    // panic 逐字 "attempt to negate with overflow"，ast/mod.rs 取负臂）；
    // 修后 i128 域精确取负得 -2^63 = i64::MIN，交 checker E1064 既有判域（#560）。
    let index = neg_int_lit(9223372036854775808);

    // Act
    let positions = Expr::extract_binding_positions(&index);

    // Assert
    assert_eq!(
        positions,
        vec![i64::MIN],
        "i64::MIN 形态应精确取负为 i64::MIN（负位，交下游 E1064 判域）而非 panic"
    );
}

#[test]
fn test_extract_binding_positions_negative_positions_preserved() {
    // Arrange — RFC-004 附录 A：负数索引支持（从末尾计数），提取器原样保留。
    let cases = [(1_i128, -1_i64), (5, -5)];

    for (n, expected) in cases {
        let index = neg_int_lit(n);

        // Act
        let positions = Expr::extract_binding_positions(&index);

        // Assert
        assert_eq!(
            positions,
            vec![expected],
            "[-{n}] 应提取为位置 {expected}（RFC-004 负位原样保留）"
        );
    }
}

#[test]
fn test_extract_binding_positions_positive_positions_preserved() {
    // Arrange — 正位字面量直取（Lit 臂，无取负）。
    let index = int_lit(1);

    // Act
    let positions = Expr::extract_binding_positions(&index);

    // Assert
    assert_eq!(positions, vec![1], "[1] 应提取为位置 1");
}

#[test]
fn test_extract_binding_positions_tuple_mixed_positions_preserved() {
    // Arrange — RFC-004 多位置联合绑定 `f[0, -1]`。
    let index = Expr::Tuple(vec![int_lit(0), neg_int_lit(1)], Span::dummy());

    // Act
    let positions = Expr::extract_binding_positions(&index);

    // Assert
    assert_eq!(
        positions,
        vec![0, -1],
        "元组 [0, -1] 应逐位提取（正位直取、负位取负），顺序保持"
    );
}
