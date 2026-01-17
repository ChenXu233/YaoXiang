//! Lifetime 模块单元测试
//!
//! 测试生命周期分析和所有权检查功能

mod move_semantics;
mod drop_semantics;

use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::middle::lifetime::OwnershipAnalyzer;

/// 创建测试用的 FunctionIR
pub fn create_test_function_with_locals(locals: usize) -> FunctionIR {
    let locals_vec: Vec<MonoType> = (0..locals).map(|_| MonoType::Int(64)).collect();
    FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: locals_vec,
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![],
            successors: vec![],
        }],
        entry: 0,
    }
}

fn create_test_function() -> FunctionIR {
    FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64), MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
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
                Instruction::Ret(Some(Operand::Temp(2))),
            ],
            successors: vec![],
        }],
        entry: 0,
    }
}

#[test]
fn test_lifetime_analysis() {
    let func = create_test_function();
    let mut analyzer = OwnershipAnalyzer::new();
    let result = analyzer.analyze_function(&func);

    // 验证分析结果包含 drop points
    println!("Drop points: {:?}", result.drop_points);

    // 至少应该有一些分析结果
    assert!(result.definitions.len() > 0 || result.drop_points.len() > 0);
}

#[test]
fn test_lifetime_analyzer_new() {
    let analyzer = OwnershipAnalyzer::new();
    let _ = analyzer;
}
