//! Drop 语义检查测试

use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::frontend::typecheck::MonoType;
use crate::middle::lifetime::error::OwnershipError;
use crate::middle::lifetime::drop_semantics::DropChecker;

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
fn test_drop_checker_new() {
    let checker = DropChecker::new();
    assert!(checker.errors().is_empty());
}

#[test]
fn test_drop_moved_value() {
    let mut func = create_test_function();
    func.blocks[0].instructions = vec![
        // Move: x -> y
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        // Drop 已移动的值
        Instruction::Drop(Operand::Local(0)),
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::DropMovedValue { .. }));
}

#[test]
fn test_double_drop() {
    let mut func = create_test_function();
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
fn test_use_after_drop() {
    let mut func = create_test_function();
    func.blocks[0].instructions = vec![
        Instruction::Drop(Operand::Local(0)),
        // Use after drop
        Instruction::Add {
            dst: Operand::Temp(0),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::ir::ConstValue::Int(1)),
        },
    ];

    let mut checker = DropChecker::new();
    let errors = checker.check_function(&func);
    assert!(!errors.is_empty());
    assert!(matches!(errors[0], OwnershipError::UseAfterDrop { .. }));
}
