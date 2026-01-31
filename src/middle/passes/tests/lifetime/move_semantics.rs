//! Move 语义检查测试

use crate::middle::core::ir::{Instruction, Operand};
use crate::middle::passes::lifetime::error::{OwnershipCheck, OwnershipError};
use crate::middle::passes::lifetime::move_semantics::MoveChecker;
use super::create_test_function_with_locals;

// ============ 基础测试 ============

#[test]
fn test_move_checker_new() {
    let checker = MoveChecker::new();
    assert!(checker.errors().is_empty());
}

#[test]
fn test_no_error_on_simple_move() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![Instruction::Move {
        dst: Operand::Temp(0),
        src: Operand::Local(0),
    }];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "Simple move should not produce errors");
}

// ============ UseAfterMove 测试 ============

#[test]
fn test_use_after_move() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Add {
            dst: Operand::Temp(1),
            lhs: Operand::Temp(0),
            rhs: Operand::Local(0), // Local(0) 已被移动
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], OwnershipError::UseAfterMove { .. }));
}

#[test]
fn test_multiple_use_after_move() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Add {
            dst: Operand::Temp(1),
            lhs: Operand::Local(0),
            rhs: Operand::Local(0), // Local(0) 被使用两次
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 2);
}

#[test]
fn test_use_after_move_in_sub() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Sub {
            dst: Operand::Temp(1),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
}

#[test]
fn test_use_after_move_in_mul() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Mul {
            dst: Operand::Temp(1),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(2)),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
}

// ============ 比较运算测试 ============

#[test]
fn test_use_after_move_in_eq() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Eq {
            dst: Operand::Temp(1),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(0)),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
}

#[test]
fn test_use_after_move_in_lt() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Lt {
            dst: Operand::Temp(1),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(0)),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
}

// ============ Load/Field 测试 ============

#[test]
fn test_use_after_move_in_load_index() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::LoadIndex {
            dst: Operand::Temp(1),
            src: Operand::Local(0),
            index: Operand::Const(crate::middle::core::ir::ConstValue::Int(0)),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
}

// ============ 连续 Move 测试 ============

#[test]
fn test_chain_moves() {
    // Temp(0) = Local(0) -> Temp(0) 拥有 Local(0) 的所有权
    // Temp(1) = Temp(0) -> Temp(0) 移动到 Temp(1)，Temp(0) 不再可用
    // 这是合法的链式移动，不会报错
    let mut func = create_test_function_with_locals(3);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Move {
            dst: Operand::Temp(1),
            src: Operand::Temp(0),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "Chain moves are valid");
}

#[test]
fn test_multiple_independent_moves() {
    let mut func = create_test_function_with_locals(2);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Move {
            dst: Operand::Temp(1),
            src: Operand::Local(1),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}

// ============ 函数调用测试 ============

#[test]
fn test_call_moves_arguments() {
    let mut func = create_test_function_with_locals(2);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Call {
            dst: Some(Operand::Temp(1)),
            func: Operand::Global(0),
            args: vec![Operand::Local(0), Operand::Local(1)],
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    // Local(0) 已被移动，不能再次移动
    assert_eq!(errors.len(), 1);
}

#[test]
fn test_call_with_temp_args() {
    // Temp(0) 和 Temp(1) 被移动到函数是合法的
    // 它们被消耗并传递给函数参数
    let mut func = create_test_function_with_locals(2);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Move {
            dst: Operand::Temp(1),
            src: Operand::Local(1),
        },
        Instruction::Call {
            dst: Some(Operand::Temp(2)),
            func: Operand::Global(0),
            args: vec![Operand::Temp(0), Operand::Temp(1)],
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "Moving temps to function call is valid");
}

// ============ Return 测试 ============

#[test]
fn test_ret_moves_value() {
    // Local(0) 被移动后返回，是合法的
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Ret(Some(Operand::Local(0))),
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "Returning moved value is valid");
}

#[test]
fn test_ret_already_moved() {
    // Temp(0) 被移动到它自己后，再返回它
    // 这是合法的，因为 Temp(0) 拥有 Local(0) 的所有权
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Ret(Some(Operand::Temp(0))),
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "Returning temp that owns value is valid");
}

// ============ 一元运算测试 ============

#[test]
fn test_use_after_move_in_neg() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Neg {
            dst: Operand::Temp(1),
            src: Operand::Local(0),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
}

// ============ Cast 测试 ============

#[test]
fn test_use_after_move_in_cast() {
    let mut func = create_test_function_with_locals(1);
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::Cast {
            dst: Operand::Temp(1),
            src: Operand::Local(0),
            target_type: crate::frontend::core::parser::ast::Type::Float(64),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
}

// ============ 复杂场景测试 ============

#[test]
fn test_complex_scenario() {
    let mut func = create_test_function_with_locals(3);
    func.blocks[0].instructions = vec![
        // a -> x
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        // b -> y
        Instruction::Move {
            dst: Operand::Temp(1),
            src: Operand::Local(1),
        },
        // x + a (错误)
        Instruction::Add {
            dst: Operand::Temp(2),
            lhs: Operand::Temp(0),
            rhs: Operand::Local(0),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert_eq!(errors.len(), 1);
}
