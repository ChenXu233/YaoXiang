//! `Instruction` 结构判据 — 04-ssa.md §目标设计「`Phi` 作为新 `Instruction` 变体」
//!
//! 覆盖 7b.1a 新增 `Instruction::Phi` 的两个穷尽 match 臂：
//! `Instruction::span()` 与 `Instruction::for_each_operand_mut()`
//! （09a §P7 7b.1a 契约 §二 第 1/2 行）。

use crate::middle::ir::ir::{ConstValue, Instruction, Operand, ValueId};
use crate::util::span::{Position, Span};

/// 固定形状的 Phi：dst 一个 SSA 值，incoming 两个来源（值 + 常量）。
fn phi_with_span(span: Span) -> Instruction {
    Instruction::Phi {
        dst: Operand::Value(ValueId(0)),
        incoming: vec![
            (1, Operand::Value(ValueId(1))),
            (2, Operand::Const(ConstValue::Int(7))),
        ],
        span,
    }
}

/// `span()`：Phi 返回自身的 span 字段（不是 dummy、也不是某个输入的 span）。
#[test]
fn test_phi_span_returns_own_span() {
    // Arrange：起止行列都可区分的 span
    let span = Span::new(
        Position::with_offset(7, 1, 0),
        Position::with_offset(8, 1, 0),
    );

    // Act
    let actual = phi_with_span(span).span();

    // Assert
    assert_eq!(actual, span, "Phi::span() 必须返回自身的 span 字段");
}

/// `for_each_operand_mut()`：按 dst → 各 incoming 的次序访问全部操作数位。
#[test]
fn test_phi_for_each_operand_mut_visits_dst_and_incoming_in_order() {
    // Arrange
    let mut instr = phi_with_span(Span::dummy());
    let mut visited: Vec<Operand> = Vec::new();

    // Act
    instr.for_each_operand_mut(&mut |op| visited.push(op.clone()));

    // Assert：3 个操作数位 = dst + 2 个 incoming，顺序稳定
    assert_eq!(
        visited,
        vec![
            Operand::Value(ValueId(0)),
            Operand::Value(ValueId(1)),
            Operand::Const(ConstValue::Int(7)),
        ],
        "Phi 的操作数位应为 dst 后按 incoming 顺序展开"
    );
}

/// `for_each_operand_mut()`：回调写入必须落到原指令（不是副本）。
#[test]
fn test_phi_for_each_operand_mut_writes_through() {
    // Arrange
    let mut instr = phi_with_span(Span::dummy());

    // Act：把每个操作数位替换为同一个常量，再从同一指令读回
    instr.for_each_operand_mut(&mut |op| *op = Operand::Const(ConstValue::Int(9)));
    let mut after: Vec<Operand> = Vec::new();
    instr.for_each_operand_mut(&mut |op| after.push(op.clone()));

    // Assert
    assert_eq!(
        after,
        vec![
            Operand::Const(ConstValue::Int(9)),
            Operand::Const(ConstValue::Int(9)),
            Operand::Const(ConstValue::Int(9)),
        ],
        "Phi 的每个操作数位都必须被回调写穿"
    );
}
