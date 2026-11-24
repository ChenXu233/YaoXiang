//! RFC-004 绑定位置提取测试 — `Expr::extract_binding_positions`（ast/mod.rs）
//!
//! 规范来源:
//! - `docs/src/rfc/accepted/004-curry-multi-position-binding.md` 附录 A
//!   （负数索引支持，从末尾计数）与运行时行为表（`func[-1]`）、
//!   「2. 验证所有位置索引有效」条款（`pos >= func.params.len()` 即
//!   `TypeError::InvalidBindingPosition`）
//! - E1064 绑定位置索引无效（RFC-004）：参数范围极端值由 checker 的
//!   `normalize_binding_positions` 既有校验拒绝，提取器不在此判域
//!
//! 覆盖: 负位提取语义（issue #560：取负臂 checked 化，i64::MIN 形态不再 panic）、
//! checked_neg 溢出哨兵分支（仅手工构造 AST 可达，见 i128::MIN 形态锚）、
//! 正位/元组提取对照，以及超 i64 表示域拒绝（issue #563 甲案：位置是结构性
//! 参数，超域字面量返回 Err 交编译期 E1064，不静默截断——截断是值字面量的
//! 另一条既有语义，不在本判域）。

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
        positions.unwrap(),
        vec![i64::MIN],
        "i64::MIN 形态应精确取负为 i64::MIN（负位极端，交下游 E1064 判域）而非 panic"
    );
}

/// 病史锚（coding-rules 第七部分·病史门）：#560 修复笔曾把取负溢出哨兵写成
/// i128::MIN，但其低 64 位全零、`as i64` 截断得 0——哨兵与「直接回退 0」行为
/// 完全等价，溢出形态会静默绑定位 0、绕过 checker E1064 校验，且该等价性使
/// 任何只观测返回值的锚都无法区分两种哨兵（变异不可杀死）。3.20.26 补锚轮
/// 实测暴露此缺陷：既有锚无一覆盖 checked_neg 的 None 路径（各自内层字面量
/// 经 checked_neg 均成功），探针锚在当时的修复态即为红。哨兵遂更正为 i64::MIN
/// 的 i128 表示。本锚构造该 None 路径唯一可达形态 Neg(Lit(i128::MIN))（用户
/// 源码整数字面量经词法 parse::<i128> 恒非负），钉死溢出必须落 E1064 判域
/// 而非静默绑定位 0。
#[test]
fn test_extract_binding_positions_neg_i128_min_overflow_sentinel_returns_i64_min() {
    // Arrange — 手工构造 Neg(Lit(i128::MIN))：checked_neg 唯一溢出返回 None 的形态。
    let index = neg_int_lit(i128::MIN);

    // Act
    let positions = Expr::extract_binding_positions(&index);

    // Assert
    assert_eq!(
        positions.unwrap(),
        vec![i64::MIN],
        "取负溢出哨兵必须取 i64::MIN（负位极端，交 checker normalize 判 E1064）；\
         回退 0 会静默绑定位 0、绕过校验"
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
            positions.unwrap(),
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
    assert_eq!(positions.unwrap(), vec![1], "[1] 应提取为位置 1");
}

#[test]
fn test_extract_binding_positions_tuple_mixed_positions_preserved() {
    // Arrange — RFC-004 多位置联合绑定 `f[0, -1]`。
    let index = Expr::Tuple(vec![int_lit(0), neg_int_lit(1)], Span::dummy());

    // Act
    let positions = Expr::extract_binding_positions(&index);

    // Assert
    assert_eq!(
        positions.unwrap(),
        vec![0, -1],
        "元组 [0, -1] 应逐位提取（正位直取、负位取负），顺序保持"
    );
}

/// #563 甲案：正位字面量超 i64 表示域（2^64+1）必须拒绝而非静默截断。
/// 修前 Lit 臂 `*n as i64` 截断得 1、绑定静默落位 1（CLI check 全绿，
/// issue #563 披露①实测）；修后 Err 载荷按书写形态渲染原字面量。
#[test]
fn test_extract_binding_positions_positive_overflow_rejected_with_exact_literal() {
    // Arrange — 用户源码 `f[18446744073709551617]` 的 AST 形态：Lit(2^64+1)。
    let index = int_lit(18446744073709551617);

    // Act
    let positions = Expr::extract_binding_positions(&index);

    // Assert
    assert_eq!(
        positions.unwrap_err(),
        "[18446744073709551617]",
        "2^64+1 位置字面量应拒绝并渲染原字面量（#563 甲案），不得静默截断为 1"
    );
}

/// #563 甲案：负位取负后低于 i64::MIN（[-(2^64-1)]）同样拒绝——修前
/// `as i64` 截断低 64 位得 1、绑定静默落位 1（CLI 实测 check 全绿）。
#[test]
fn test_extract_binding_positions_negative_overflow_rejected_with_exact_literal() {
    // Arrange — `f[-18446744073709551615]` 的 AST 形态：Neg(Lit(2^64-1))。
    let index = neg_int_lit(18446744073709551615);

    // Act
    let positions = Expr::extract_binding_positions(&index);

    // Assert
    assert_eq!(
        positions.unwrap_err(),
        "[-18446744073709551615]",
        "-(2^64-1) 位置字面量应拒绝并渲染原字面量（#563 甲案），不得静默截断为 1"
    );
}

/// #563 甲案：元组中任一项超域即整条拒绝，渲染串保留域内项的原值与
/// 超域项的原字面量（E1064 的 {positions} 参数与 normalize 的
/// `format!("{:?}", positions)` 渲染形态同构）。
#[test]
fn test_extract_binding_positions_tuple_with_overflow_item_rejects_whole_list() {
    // Arrange — `f[1, 18446744073709551617]`：一项域内、一项超域。
    let index = Expr::Tuple(
        vec![int_lit(1), int_lit(18446744073709551617)],
        Span::dummy(),
    );

    // Act
    let positions = Expr::extract_binding_positions(&index);

    // Assert
    assert_eq!(
        positions.unwrap_err(),
        "[1, 18446744073709551617]",
        "元组任一项超域应整条拒绝，渲染串保留域内项与超域字面量"
    );
}

/// 边界锚：i64 表示域两端（i64::MAX / i64::MIN）必须仍在域内——
/// 超界拒绝的判据是 i64 表示域本身，不是任意大数。
#[test]
fn test_extract_binding_positions_i64_domain_boundaries_stay_in_domain() {
    // Arrange — 域边界两端：i64::MAX 与 -i64::MAX（i64::MIN 形态另有专锚）。
    let cases = [
        (9223372036854775807_i128, i64::MAX),
        (-9223372036854775807_i128, -9223372036854775807_i64),
    ];

    for (n, expected) in cases {
        let index = if n >= 0 { int_lit(n) } else { neg_int_lit(-n) };

        // Act
        let positions = Expr::extract_binding_positions(&index);

        // Assert
        assert_eq!(
            positions.unwrap(),
            vec![expected],
            "i64 域边界值 {n} 必须在域内（提取为 {expected}）"
        );
    }
}
