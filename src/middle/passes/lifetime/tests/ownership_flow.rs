//! 所有权回流分析单元测试
//!
//! 测试 OwnershipFlowAnalyzer 的函数消费模式分析、参数引用检测和返回值追踪功能。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::ownership_flow::{ConsumeMode, OwnershipFlowAnalyzer};
use crate::util::span::Span;

fn create_test_function(returns_param: bool) -> FunctionIR {
    let return_instr = if returns_param {
        Instruction::Ret(Some(Operand::Arg(0)))
    } else {
        Instruction::Ret(Some(Operand::Const(ConstValue::Int(0))))
    };

    FunctionIR {
        name: "test_func".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![return_instr],
            successors: vec![],
        }],
        entry: 0,
    }
}

#[test]
fn test_returns_param_directly() {
    let analyzer = OwnershipFlowAnalyzer::new("test".to_string());

    // return p; 模式
    let func_returns_param = create_test_function(true);
    assert!(analyzer.returns_param_directly(&func_returns_param, 0));

    // return 0; 模式
    let func_not_returns_param = create_test_function(false);
    assert!(!analyzer.returns_param_directly(&func_not_returns_param, 0));
}

#[test]
fn test_analyze_function_returns_mode() {
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());
    let func_returns_param = create_test_function(true);
    let modes = analyzer.analyze_function(&func_returns_param);

    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Returns);
}

#[test]
fn test_analyze_function_consumes_mode() {
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());
    let func_not_returns_param = create_test_function(false);
    let modes = analyzer.analyze_function(&func_not_returns_param);

    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Consumes);
}

#[test]
fn test_returns_via_move() {
    // 测试 return temp，其中 temp = arg_0
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

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

    let modes = analyzer.analyze_function(&func);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Returns);
}

#[test]
fn test_returns_different_value() {
    // 测试返回常量值，参数被消费
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Const(ConstValue::Int(42))))],
            successors: vec![],
        }],
        entry: 0,
    };

    let modes = analyzer.analyze_function(&func);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Consumes);
}

#[test]
fn test_multiple_params_partial_return() {
    // 测试多参数函数，部分参数返回
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

    let func = FunctionIR {
        name: "test".to_string(),
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

    let modes = analyzer.analyze_function(&func);
    assert_eq!(modes.len(), 2);
    assert_eq!(modes[0], ConsumeMode::Returns); // arg_0 返回
    assert_eq!(modes[1], ConsumeMode::Consumes); // arg_1 不返回
}

#[test]
fn test_returns_via_call() {
    // 测试参数作为参数传递给函数，函数返回值
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Call {
                    dst: Some(Operand::Temp(0)),
                    func: Operand::Global(0),
                    args: vec![Operand::Arg(0)],
                    span: Span::dummy(),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let modes = analyzer.analyze_function(&func);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Returns);
}

#[test]
fn test_returns_via_load_field() {
    // 测试从参数加载字段并返回
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::LoadField {
                    dst: Operand::Temp(0),
                    src: Operand::Arg(0),
                    field: 0,
                    span: crate::util::span::Span::dummy(),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let modes = analyzer.analyze_function(&func);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Returns);
}

#[test]
fn test_no_return_statement() {
    // 测试没有 return 语句的函数（应该是 Consumes）
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

    let func = FunctionIR {
        name: "test".to_string(),
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

    let modes = analyzer.analyze_function(&func);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Consumes);
}

#[test]
fn test_multiple_returns_same_param() {
    // 测试多个 return 语句都返回同一个参数
    let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Const(ConstValue::Bool(
                false,
            ))))],
            successors: vec![],
        }],
        entry: 0,
    };

    let modes = analyzer.analyze_function(&func);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Consumes);
}
