//! Lifetime 模块单元测试
//!
//! 测试生命周期分析和引用计数插入功能

use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::middle::lifetime::{OwnershipAnalysisResult, OwnershipAnalyzer};

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
    // 在这个简单的函数中，临时变量和局部变量可能会在块结束时被 drop
    // 具体行为取决于 OwnershipAnalyzer 的实现细节
    println!("Drop points: {:?}", result.drop_points);

    // 至少应该有一些分析结果
    // result.ownership_graph.edges is private, so we can't check it directly if it's not pub
    // But OwnershipAnalysisResult fields are pub in mod.rs
    // pub struct OwnershipAnalysisResult {
    //    pub ownership_graph: OwnershipGraph,
    //    pub definitions: HashMap<Operand, Definition>,
    //    pub drop_points: HashMap<usize, Vec<Operand>>,
    // }
    // OwnershipGraph fields are private though.

    assert!(result.definitions.len() > 0 || result.drop_points.len() > 0);
}

#[test]
fn test_lifetime_analyzer_new() {
    let analyzer = OwnershipAnalyzer::new();
    // 验证分析器可以创建
    let _ = analyzer;
}
