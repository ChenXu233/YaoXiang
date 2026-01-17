//! Move 语义检查测试

use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::frontend::typecheck::MonoType;
use crate::middle::lifetime::error::OwnershipError;
use crate::middle::lifetime::move_semantics::MoveChecker;

fn create_test_function() -> FunctionIR {
    FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![],
            successors: vec![],
        }],
        entry: 0,
    }
}

#[test]
fn test_move_checker_new() {
    let checker = MoveChecker::new();
    assert!(checker.errors().is_empty());
}

#[test]
fn test_no_error_on_simple_move() {
    let mut func = create_test_function();
    func.blocks[0].instructions = vec![
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
    ];

    let mut checker = MoveChecker::new();
    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "Simple move should not produce errors");
}

#[test]
fn test_use_after_move() {
    let mut func = create_test_function();
    func.blocks[0].instructions = vec![
        // Move: x -> y
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        // Use after move
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
