//! Drop 语义检查测试

use crate::middle::core::ir::{Instruction, Operand};
use crate::middle::passes::lifetime::error::{OwnershipCheck, OwnershipError};
use crate::middle::passes::lifetime::drop_semantics::DropChecker;
use super::create_test_function_with_locals;

// ============ 基础测试 ============

#[test]
fn test_drop_checker_new() {
    let checker = DropChecker::new();
    assert!(checker.errors().is_empty());
}

// ============ DropMovedValue 测试 ============

#[test]
fn test_drop_moved_value() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Drop(Operand::Local(0)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::DropMovedValue { .. }));
}

#[test]
fn test_drop_value_moved_to_temp() {
    // Temp(0) = Local(0) -> Temp(0) 拥有 Local(0) 的所有权
    // Drop(Temp(0)) -> 合法，Temp(0) 是 owned 状态
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Drop(Operand::Temp(0)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "Dropping owned temp is valid");
}

// ============ DoubleDrop 测试 ============

#[test]
fn test_double_drop() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Drop(Operand::Local(0)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::DoubleDrop { .. }));
}

#[test]
fn test_triple_drop() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Drop(Operand::Local(0)),
        Instruction::Drop(Operand::Local(0)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 2);
}

#[test]
fn test_double_drop_different_values() {
    let mut func = create_test_function_with_locals(2);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Drop(Operand::Local(1)),
        Instruction::Drop(Operand::Local(0)),
        Instruction::Drop(Operand::Local(1)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 2);
}

// ============ UseAfterDrop 测试 ============

#[test]
fn test_use_after_drop() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Add {
            dst: Operand::Temp(0),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::UseAfterDrop { .. }));
}

#[test]
fn test_use_after_drop_multiple_times() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Add {
            dst: Operand::Temp(0),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
        Instruction::Sub {
            dst: Operand::Temp(1),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 2);
}

#[test]
fn test_use_after_drop_in_sub() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Sub {
            dst: Operand::Temp(0),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
}

#[test]
fn test_use_after_drop_in_mul() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Mul {
            dst: Operand::Temp(0),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(2)),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
}

#[test]
fn test_use_after_drop_in_div() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Div {
            dst: Operand::Temp(0),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(2)),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
}

#[test]
fn test_use_after_drop_in_mod() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Mod {
            dst: Operand::Temp(0),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(2)),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
}

// ============ Load 测试 ============

#[test]
fn test_use_after_drop_in_load_index() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::LoadIndex {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
            index: Operand::Const(crate::middle::core::ir::ConstValue::Int(0)),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::UseAfterDrop { .. }));
}

#[test]
fn test_use_after_drop_in_load_field() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::LoadField {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
            field: 0,
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::UseAfterDrop { .. }));
}

// ============ Cast 测试 ============

#[test]
fn test_use_after_drop_in_cast() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Cast {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
            target_type: crate::frontend::core::parser::ast::Type::Float(64),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::UseAfterDrop { .. }));
}

// ============ Ret 测试 ============

#[test]
fn test_use_after_drop_in_ret() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Ret(Some(Operand::Local(0))),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::UseAfterDrop { .. }));
}

// ============ Move 后 Drop 测试 ============

#[test]
fn test_move_then_drop_dst() {
    let mut func = create_test_function_with_locals(2);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Drop(Operand::Temp(0)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "Dropping dst after move should be OK");
}

#[test]
fn test_move_then_drop_src() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Drop(Operand::Local(0)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::DropMovedValue { .. }));
}

// ============ 组合场景测试 ============

#[test]
fn test_drop_then_call() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Call {
            dst: Some(Operand::Temp(0)),
            func: Operand::Global(0),
            args: vec![],
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}

#[test]
fn test_multiple_drops_no_error() {
    let mut func = create_test_function_with_locals(2);
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        Instruction::Drop(Operand::Local(1)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}

#[test]
fn test_complex_scenario_with_errors() {
    // 场景：
    // 1. Move { Temp(0), Local(0) } -> Local(0) = Moved, Temp(0) = Owned
    // 2. Drop(Local(0)) -> 错误: DropMovedValue (Local(0) 是 Moved 状态)
    // 3. Add { Temp(1), Local(0) } -> Local(0) 仍是 Moved，UseAfterDrop 检查 Dropped 状态，所以不报错
    // 4. Drop(Local(0)) -> 错误: DropMovedValue (Local(0) 仍是 Moved 状态)
    let mut func = create_test_function_with_locals(3);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Drop(Operand::Local(0)), // 错误: DropMovedValue
        Instruction::Add {
            dst: Operand::Temp(1),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
        Instruction::Drop(Operand::Local(0)), // 错误: DropMovedValue
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 2);
    assert!(matches!(errors[0], OwnershipError::DropMovedValue { .. }));
    assert!(matches!(errors[1], OwnershipError::DropMovedValue { .. }));
}
