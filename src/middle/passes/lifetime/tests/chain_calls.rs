//! 链式调用分析单元测试
//!
//! 测试 ChainCallAnalyzer 的链式调用提取、消费模式推断和所有权闭合检测功能。

use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::chain_calls::{ChainAnalysisResult, ChainCallAnalyzer, MethodInfo};
use crate::middle::passes::lifetime::ownership_flow::ConsumeMode;
use crate::frontend::core::typecheck::MonoType;
use crate::util::span::Span;

fn create_test_func_with_calls(calls: Vec<Instruction>) -> FunctionIR {
    FunctionIR {
        name: "test_chain".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: calls,
            successors: vec![],
        }],
        entry: 0,
    }
}

#[test]
fn test_extract_chain_calls() {
    let analyzer = ChainCallAnalyzer::new();

    let calls = vec![
        Instruction::CallVirt {
            dst: Some(Operand::Temp(1)),
            obj: Operand::Temp(0),
            method_name: "rotate".to_string(),
            args: vec![Operand::Const(ConstValue::Int(90))],
            span: Span::dummy(),
        },
        Instruction::CallVirt {
            dst: Some(Operand::Temp(2)),
            obj: Operand::Temp(1),
            method_name: "scale".to_string(),
            args: vec![Operand::Const(ConstValue::Int(2))],
            span: Span::dummy(),
        },
    ];

    let func = create_test_func_with_calls(calls);
    let extracted = analyzer.extract_chain_calls(&func, 0);

    assert_eq!(extracted.len(), 2);
}

#[test]
fn test_infer_consume_mode() {
    let analyzer = ChainCallAnalyzer::new();

    let call = Instruction::CallVirt {
        dst: Some(Operand::Temp(1)),
        obj: Operand::Temp(0),
        method_name: "rotate".to_string(),
        args: vec![],
        span: Span::dummy(),
    };

    // 返回值被使用
    assert_eq!(
        analyzer.infer_consume_mode(&call, true),
        ConsumeMode::Returns
    );

    // 返回值未被使用
    assert_eq!(
        analyzer.infer_consume_mode(&call, false),
        ConsumeMode::Consumes
    );
}

#[test]
fn test_check_ownership_closure() {
    let analyzer = ChainCallAnalyzer::new();

    // Consumes 模式，所有权闭合
    let chain_consumes = ChainAnalysisResult {
        methods: vec![MethodInfo {
            name: "consume".to_string(),
            receiver: Operand::Temp(0),
            args: vec![],
            consume_mode: ConsumeMode::Consumes,
            result: None,
        }],
        initial_receiver: Operand::Temp(0),
        final_result: None,
        ownership_closed: false,
    };
    assert!(analyzer.check_ownership_closure(&chain_consumes));

    // Returns 模式，返回值被使用
    let chain_returns = ChainAnalysisResult {
        methods: vec![MethodInfo {
            name: "transform".to_string(),
            receiver: Operand::Temp(0),
            args: vec![],
            consume_mode: ConsumeMode::Returns,
            result: Some(Operand::Temp(1)),
        }],
        initial_receiver: Operand::Temp(0),
        final_result: Some(Operand::Temp(1)),
        ownership_closed: false,
    };
    assert!(analyzer.check_ownership_closure(&chain_returns));
}

#[test]
fn test_long_chain_calls() {
    // 测试长链式调用: p.rotate(90).scale(2.0).translate(1.0)
    let mut analyzer = ChainCallAnalyzer::new();

    let calls = vec![
        Instruction::CallVirt {
            dst: Some(Operand::Temp(1)),
            obj: Operand::Temp(0),
            method_name: "rotate".to_string(),
            args: vec![Operand::Const(ConstValue::Int(90))],
            span: Span::dummy(),
        },
        Instruction::CallVirt {
            dst: Some(Operand::Temp(2)),
            obj: Operand::Temp(1),
            method_name: "scale".to_string(),
            args: vec![Operand::Const(ConstValue::Int(2))],
            span: Span::dummy(),
        },
        Instruction::CallVirt {
            dst: Some(Operand::Temp(3)),
            obj: Operand::Temp(2),
            method_name: "translate".to_string(),
            args: vec![Operand::Const(ConstValue::Int(1))],
            span: Span::dummy(),
        },
    ];

    let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

    assert_eq!(result.methods.len(), 3);
    assert_eq!(result.methods[0].name, "rotate");
    assert_eq!(result.methods[1].name, "scale");
    assert_eq!(result.methods[2].name, "translate");
}

#[test]
fn test_chain_with_different_temp_indices() {
    // 测试链中临时变量索引的正确追踪
    let mut analyzer = ChainCallAnalyzer::new();

    let calls = vec![
        Instruction::CallVirt {
            dst: Some(Operand::Temp(100)),
            obj: Operand::Temp(0),
            method_name: "method1".to_string(),
            args: vec![],
            span: Span::dummy(),
        },
        Instruction::CallVirt {
            dst: Some(Operand::Temp(200)),
            obj: Operand::Temp(100),
            method_name: "method2".to_string(),
            args: vec![],
            span: Span::dummy(),
        },
    ];

    let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

    assert_eq!(result.methods.len(), 2);
    assert_eq!(result.methods[0].name, "method1");
    assert_eq!(result.methods[1].name, "method2");
}

#[test]
fn test_chain_with_args() {
    // 测试带参数的方法链
    let mut analyzer = ChainCallAnalyzer::new();

    let calls = vec![Instruction::CallVirt {
        dst: Some(Operand::Temp(1)),
        obj: Operand::Temp(0),
        method_name: "rotate".to_string(),
        args: vec![
            Operand::Const(ConstValue::Int(90)),
            Operand::Const(ConstValue::Float(std::f64::consts::PI)),
        ],
        span: Span::dummy(),
    }];

    let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

    assert_eq!(result.methods.len(), 1);
    assert_eq!(result.methods[0].name, "rotate");
    assert_eq!(result.methods[0].args.len(), 2);
}

#[test]
fn test_empty_chain() {
    // 测试空链
    let mut analyzer = ChainCallAnalyzer::new();

    let result = analyzer.analyze_chain(Operand::Temp(0), &[]);

    assert!(result.methods.is_empty());
    assert_eq!(result.initial_receiver, Operand::Temp(0));
}

#[test]
fn test_non_chain_call() {
    // 测试非链式调用（接收者不匹配）
    let mut analyzer = ChainCallAnalyzer::new();

    let calls = vec![Instruction::CallVirt {
        dst: Some(Operand::Temp(1)),
        obj: Operand::Temp(5), // 不同的索引，不匹配
        method_name: "method".to_string(),
        args: vec![],
        span: Span::dummy(),
    }];

    let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

    assert!(result.methods.is_empty());
}

#[test]
fn test_call_instruction_chain() {
    // 测试普通函数调用链（第一个参数是接收者）
    let mut analyzer = ChainCallAnalyzer::new();

    let calls = vec![
        Instruction::Call {
            dst: Some(Operand::Temp(1)),
            func: Operand::Global(0),
            args: vec![Operand::Temp(0), Operand::Const(ConstValue::Int(1))],
            span: Span::dummy(),
        },
        Instruction::Call {
            dst: Some(Operand::Temp(2)),
            func: Operand::Global(0),
            args: vec![Operand::Temp(1), Operand::Const(ConstValue::Int(2))],
            span: Span::dummy(),
        },
    ];

    let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

    assert_eq!(result.methods.len(), 2);
    assert_eq!(result.methods[0].name, "call");
    assert_eq!(result.methods[1].name, "call");
}

#[test]
fn test_mixed_call_chain() {
    // 测试混合调用链（Call 和 CallVirt）
    let mut analyzer = ChainCallAnalyzer::new();

    let calls = vec![
        Instruction::CallVirt {
            dst: Some(Operand::Temp(1)),
            obj: Operand::Temp(0),
            method_name: "method1".to_string(),
            args: vec![],
            span: Span::dummy(),
        },
        Instruction::Call {
            dst: Some(Operand::Temp(2)),
            func: Operand::Global(0),
            args: vec![Operand::Temp(1)],
            span: Span::dummy(),
        },
        Instruction::CallVirt {
            dst: Some(Operand::Temp(3)),
            obj: Operand::Temp(2),
            method_name: "method2".to_string(),
            args: vec![],
            span: Span::dummy(),
        },
    ];

    let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

    assert_eq!(result.methods.len(), 3);
}

#[test]
fn test_extract_stops_at_non_call() {
    // 测试提取链时遇到非 Call 指令停止
    let analyzer = ChainCallAnalyzer::new();

    let calls = vec![
        Instruction::CallVirt {
            dst: Some(Operand::Temp(1)),
            obj: Operand::Temp(0),
            method_name: "method1".to_string(),
            args: vec![],
            span: Span::dummy(),
        },
        Instruction::Add {
            dst: Operand::Temp(2),
            lhs: Operand::Temp(0),
            rhs: Operand::Const(ConstValue::Int(1)),
        }, // 非调用指令
        Instruction::CallVirt {
            dst: Some(Operand::Temp(3)),
            obj: Operand::Temp(1),
            method_name: "method2".to_string(),
            args: vec![],
            span: Span::dummy(),
        },
    ];

    let func = create_test_func_with_calls(calls);
    let extracted = analyzer.extract_chain_calls(&func, 0);

    // 应该只提取第一个 CallVirt
    assert_eq!(extracted.len(), 1);
}

#[test]
fn test_ownership_closure_with_undetermined() {
    // 测试 Undetermined 模式的保守闭合检查
    let analyzer = ChainCallAnalyzer::new();

    let chain = ChainAnalysisResult {
        methods: vec![MethodInfo {
            name: "unknown".to_string(),
            receiver: Operand::Temp(0),
            args: vec![],
            consume_mode: ConsumeMode::Undetermined,
            result: None,
        }],
        initial_receiver: Operand::Temp(0),
        final_result: None,
        ownership_closed: false,
    };

    // Undetermined 应该保守返回 true
    assert!(analyzer.check_ownership_closure(&chain));
}

#[test]
fn test_chain_final_result_tracking() {
    // 测试链中最终返回值的追踪
    let mut analyzer = ChainCallAnalyzer::new();

    let calls = vec![
        Instruction::CallVirt {
            dst: Some(Operand::Temp(1)),
            obj: Operand::Temp(0),
            method_name: "method1".to_string(),
            args: vec![],
            span: Span::dummy(),
        },
        Instruction::CallVirt {
            dst: Some(Operand::Temp(2)),
            obj: Operand::Temp(1),
            method_name: "method2".to_string(),
            args: vec![],
            span: Span::dummy(),
        },
    ];

    let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

    // 最终返回值应该是最后一个调用的返回值
    assert_eq!(result.final_result, Some(Operand::Temp(2)));
}
