//! Ref 语义所有权检查测试

use crate::middle::OwnershipCheck;
use crate::middle::passes::lifetime::{OwnershipChecker, RefChecker};
use crate::middle::core::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::frontend::typecheck::MonoType;

/// 测试 RefChecker 基本功能
#[test]
fn test_ref_checker_creation() {
    let checker = RefChecker::new();
    assert!(checker.errors().is_empty());
}

/// 测试有效的 ref 所有者
#[test]
fn test_valid_ref_owner() {
    // 创建一个函数：p 被定义，然后 ref p
    let func = FunctionIR {
        name: "test_valid_ref".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // HeapAlloc 创建对象
                Instruction::HeapAlloc {
                    dst: Operand::Temp(0),
                    type_id: 0,
                },
                // ref 操作（这里用 ArcNew 模拟）
                Instruction::ArcNew {
                    dst: Operand::Temp(1),
                    src: Operand::Temp(0),
                },
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let mut checker = RefChecker::new();
    let errors = checker.check_function(&func);

    // 有效所有者，不应该有错误
    assert!(
        errors.is_empty(),
        "Expected no errors for valid ref, got: {:?}",
        errors
    );
}

/// 测试 OwnershipChecker 包含 RefChecker
#[test]
fn test_ownership_checker_includes_ref() {
    let mut checker = OwnershipChecker::new();

    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(None)],
            successors: vec![],
        }],
        entry: 0,
    };

    let errors = checker.check_function(&func);
    // 应该能成功运行，不应该有 ref 相关错误
    assert!(
        errors.is_empty()
            || errors.iter().all(|e| {
                !matches!(
                    e,
                    crate::middle::passes::lifetime::OwnershipError::RefNonOwner { .. }
                )
            })
    );
}

/// 测试 ArcNew 不影响原值状态
#[test]
fn test_arc_new_preserves_owner() {
    // ref 操作（ArcNew）不应该影响原值的状态
    let func = FunctionIR {
        name: "test_arc_preserves".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 创建对象
                Instruction::HeapAlloc {
                    dst: Operand::Temp(0),
                    type_id: 0,
                },
                // ref 创建 Arc（不移动原值）
                Instruction::ArcNew {
                    dst: Operand::Temp(1),
                    src: Operand::Temp(0),
                },
                // 原值仍然可用
                Instruction::Drop(Operand::Temp(0)),
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let mut checker = RefChecker::new();
    let errors = checker.check_function(&func);

    // ArcNew 不应该阻止后续使用原值
    assert!(
        errors.is_empty(),
        "ArcNew should not affect owner state: {:?}",
        errors
    );
}

/// 测试 ArcClone 不影响原值状态
#[test]
fn test_arc_clone_preserves_owner() {
    let func = FunctionIR {
        name: "test_arc_clone".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 创建第一个 Arc
                Instruction::ArcNew {
                    dst: Operand::Temp(0),
                    src: Operand::Local(0),
                },
                // 克隆 Arc
                Instruction::ArcClone {
                    dst: Operand::Temp(1),
                    src: Operand::Temp(0),
                },
                // 再次克隆
                Instruction::ArcClone {
                    dst: Operand::Temp(2),
                    src: Operand::Temp(0),
                },
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let mut checker = RefChecker::new();
    let errors = checker.check_function(&func);

    assert!(
        errors.is_empty(),
        "ArcClone should not cause errors: {:?}",
        errors
    );
}

/// 测试 ArcDrop 不影响底层值状态
#[test]
fn test_arc_drop_preserves_owner() {
    let func = FunctionIR {
        name: "test_arc_drop".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 创建 Arc
                Instruction::ArcNew {
                    dst: Operand::Temp(0),
                    src: Operand::Local(0),
                },
                // 释放 Arc 副本
                Instruction::ArcDrop(Operand::Temp(1)),
                // 再次释放
                Instruction::ArcDrop(Operand::Temp(2)),
                // 释放最后一个 Arc
                Instruction::ArcDrop(Operand::Temp(0)),
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let mut checker = RefChecker::new();
    let errors = checker.check_function(&func);

    // ArcDrop 不应该影响底层值状态
    assert!(
        errors.is_empty(),
        "ArcDrop should not cause ref errors: {:?}",
        errors
    );
}
