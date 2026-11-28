//! `Instruction` / `FunctionBody` 结构判据 — 04-ssa.md §目标设计
//!
//! 7b.1a：`Instruction::Phi` 的 `span()` 与 `for_each_operand_mut()`
//! 两个穷尽 match 臂（提交信息）。
//! 7b.1b：`FunctionBody::Code.values` 值表与 `ValueInfo`（提交信息）。

use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand, ValueId, ValueInfo};
use crate::util::span::{Position, Span};

use super::ir_build::{func, ret_void, temp_slot};

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

/// `FunctionBody::Code.values` 的初值为空表（7b.1b 的生产形态：全仓构造点一律空表）。
#[test]
fn test_code_body_values_start_empty() {
    // Arrange
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        vec![ret_void()],
    );

    // Act
    let values = f.values();

    // Assert
    assert!(
        values.is_empty(),
        "新建 Code 体的 values 必须是空表，实际 {} 项",
        values.len()
    );
}

/// `values_mut()` 的写入穿到 `values()`（同一张表，不是副本）。
#[test]
fn test_code_body_values_mut_writes_through() {
    // Arrange
    let mut f = func("f", vec![], MonoType::Void, vec![], vec![ret_void()]);
    let info = ValueInfo {
        ty: MonoType::Int(64),
        debug_name: Some("x".to_string()),
        defining_block: 3,
    };

    // Act
    f.values_mut().push(info);

    // Assert：`values()` 读回同一项，逐字段保真
    let values = f.values();
    assert_eq!(values.len(), 1, "values_mut 写入后 values() 必须恰好 1 项");
    assert_eq!(
        values[0].ty,
        MonoType::Int(64),
        "写入的 ValueInfo.ty 必须原样读回"
    );
    assert_eq!(
        values[0].debug_name.as_deref(),
        Some("x"),
        "写入的 ValueInfo.debug_name 必须原样读回"
    );
    assert_eq!(
        values[0].defining_block, 3,
        "写入的 ValueInfo.defining_block 必须原样读回"
    );
}

/// `ValueInfo` 构造逐字段保真（04 §292-303：ty / debug_name / defining_block）。
#[test]
fn test_value_info_construction_preserves_fields() {
    // Arrange + Act
    let info = ValueInfo {
        ty: MonoType::Bool,
        debug_name: None,
        defining_block: 2,
    };

    // Assert
    assert!(
        matches!(info.ty, MonoType::Bool),
        "ValueInfo.ty 必须保留构造时的类型，实际 {:?}",
        info.ty
    );
    assert_eq!(info.debug_name, None, "debug_name=None 必须表示纯临时值");
    assert_eq!(
        info.defining_block, 2,
        "defining_block 必须保留定义块 label"
    );
}
