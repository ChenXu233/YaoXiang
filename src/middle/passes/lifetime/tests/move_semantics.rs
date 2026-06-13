//! Move 语义检查单元测试
//!
//! 测试 MoveChecker 的 UseAfterMove 检测、状态追踪和清除功能。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::error::Checker;
use crate::middle::passes::lifetime::move_semantics::MoveChecker;

fn make_test_function() -> FunctionIR {
    FunctionIR {
        name: "returns_param".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
            successors: vec![],
        }],
        entry: 0,
    }
}

#[test]
fn test_move_checker_no_error_on_valid_move() {
    let mut checker = MoveChecker::new();

    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Move {
                    dst: Operand::Temp(0),
                    src: Operand::Arg(0),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}

#[test]
fn test_use_after_move_detection() {
    let mut checker = MoveChecker::new();

    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Move {
                    dst: Operand::Temp(0),
                    src: Operand::Arg(0),
                },
                // 再次使用 Arg(0) 应该报错
                Instruction::Add {
                    dst: Operand::Temp(1),
                    lhs: Operand::Arg(0),
                    rhs: Operand::Const(ConstValue::Int(1)),
                },
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let errors = checker.check_function(&func);
    let has_use_after_move = errors.iter().any(|e| e.code == "E2014");
    assert!(has_use_after_move);
}

#[test]
fn test_checker_clear() {
    let mut checker = MoveChecker::new();

    let func = make_test_function();

    // 第一次检查
    checker.check_function(&func);

    // 清除状态
    checker.clear();

    // 再次检查应该正常
    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}

#[test]
fn test_returns_function_no_error() {
    // 测试返回参数的函数不应有错误
    let mut checker = MoveChecker::new();
    let func = make_test_function();

    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}

#[test]
fn test_consumes_function_no_error() {
    // 测试消费参数的函数不应有错误
    let mut checker = MoveChecker::new();

    let func = FunctionIR {
        name: "consumes_param".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(None)],
            successors: vec![],
        }],
        entry: 0,
    };

    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}

#[test]
fn test_multi_param_function() {
    // 测试多参数函数
    let mut checker = MoveChecker::new();

    let func = FunctionIR {
        name: "multi_param".to_string(),
        params: vec![MonoType::Int(0), MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
            successors: vec![],
        }],
        entry: 0,
    };

    let errors = checker.check_function(&func);
    assert!(errors.is_empty());
}
