//! 消费分析器单元测试
//!
//! 测试 ConsumeAnalyzer 的函数消费模式分析、缓存机制和内置函数处理功能。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::consume_analysis::ConsumeAnalyzer;
use crate::middle::passes::lifetime::ownership_flow::ConsumeMode;

fn make_test_function(returns_param: bool) -> FunctionIR {
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
fn test_analyze_returns_mode() {
    let mut analyzer = ConsumeAnalyzer::new();
    let func = make_test_function(true);

    let modes = analyzer.analyze_and_cache(&func);

    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Returns);
}

#[test]
fn test_analyze_consumes_mode() {
    let mut analyzer = ConsumeAnalyzer::new();
    let func = make_test_function(false);

    let modes = analyzer.analyze_and_cache(&func);

    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Consumes);
}

#[test]
fn test_builtin_consume_mode() {
    let analyzer = ConsumeAnalyzer::new();

    assert_eq!(
        analyzer.get_builtin_consume_mode("consume", 0),
        ConsumeMode::Consumes
    );
    assert_eq!(
        analyzer.get_builtin_consume_mode("clone", 0),
        ConsumeMode::Returns
    );
}

#[test]
fn test_cache_after_analysis() {
    let mut analyzer = ConsumeAnalyzer::new();
    let func = make_test_function(true);

    // 第一次分析
    let modes1 = analyzer.analyze_and_cache(&func);
    // 第二次查询（应该命中缓存）
    let modes2 = analyzer.get_function_consume_mode(&func);

    assert_eq!(modes1, modes2);
}

#[test]
fn test_multiple_params_partial_return() {
    // 测试多参数函数，部分参数返回
    let mut analyzer = ConsumeAnalyzer::new();

    let func = FunctionIR {
        name: "partial_return".to_string(),
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

    let modes = analyzer.analyze_and_cache(&func);
    assert_eq!(modes.len(), 2);
    assert_eq!(modes[0], ConsumeMode::Returns); // arg_0 返回
    assert_eq!(modes[1], ConsumeMode::Consumes); // arg_1 不返回
}

#[test]
fn test_multiple_params_all_consumed() {
    // 测试多参数函数，所有参数都被消费
    let mut analyzer = ConsumeAnalyzer::new();

    let func = FunctionIR {
        name: "all_consumed".to_string(),
        params: vec![MonoType::Int(0), MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Const(ConstValue::Int(0))))],
            successors: vec![],
        }],
        entry: 0,
    };

    let modes = analyzer.analyze_and_cache(&func);
    assert_eq!(modes.len(), 2);
    assert_eq!(modes[0], ConsumeMode::Consumes);
    assert_eq!(modes[1], ConsumeMode::Consumes);
}

#[test]
fn test_no_return_returns_void() {
    // 测试无返回值的函数
    let mut analyzer = ConsumeAnalyzer::new();

    let func = FunctionIR {
        name: "void_func".to_string(),
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

    let modes = analyzer.analyze_and_cache(&func);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Consumes);
}

#[test]
fn test_returns_via_temp_variable() {
    // 测试通过临时变量返回参数
    let mut analyzer = ConsumeAnalyzer::new();

    let func = FunctionIR {
        name: "via_temp".to_string(),
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

    let modes = analyzer.analyze_and_cache(&func);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ConsumeMode::Returns);
}

#[test]
fn test_get_call_consume_mode() {
    let mut analyzer = ConsumeAnalyzer::new();
    let func = make_test_function(true);
    analyzer.analyze_and_cache(&func);

    let mode = analyzer.get_call_consume_mode("test_func", 0);
    assert_eq!(mode, Some(ConsumeMode::Returns));
}

#[test]
fn test_unknown_function_uses_builtin() {
    let analyzer = ConsumeAnalyzer::new();
    // 未知函数使用内置处理
    let mode = analyzer.get_builtin_consume_mode("unknown_func", 0);
    assert_eq!(mode, ConsumeMode::Undetermined);
}

#[test]
fn test_clear_cache() {
    let mut analyzer = ConsumeAnalyzer::new();
    let func = make_test_function(true);
    analyzer.analyze_and_cache(&func);

    // 确认缓存有数据
    assert!(analyzer
        .get_function_consume_mode_by_name("test_func")
        .is_some());

    // 清除缓存
    analyzer.clear_cache();

    // 缓存应该为空
    assert!(analyzer
        .get_function_consume_mode_by_name("test_func")
        .is_none());
}
