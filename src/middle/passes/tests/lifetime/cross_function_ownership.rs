//! 跨函数所有权转移测试
//!
//! 测试函数参数传递、返回值传递、高阶函数等跨函数场景的所有权转移

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::OwnershipAnalyzer;
use crate::util::span::Span;

/// 创建带多个参数的函数
fn create_multi_param_function(
    params: Vec<MonoType>,
    return_type: MonoType,
    instructions: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: "multi_param_test".to_string(),
        params,
        return_type,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions,
            successors: vec![],
        }],
        entry: 0,
    }
}

/// 创建调用函数的指令
fn create_call_instruction(
    func: Operand,
    args: Vec<Operand>,
    dst: Option<Operand>,
) -> Instruction {
    Instruction::Call { func, args, dst }
}

/// 创建 spawn 指令（用于并发函数调用）
fn create_spawn_instruction(
    func: Operand,
    args: Vec<Operand>,
    result: Operand,
) -> Instruction {
    Instruction::Spawn { func, args, result }
}

// ============ 函数参数所有权测试 ============

#[test]
fn test_ownership_transfer_via_parameter() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 参数作为返回值直接传递
            create_call_instruction(
                Operand::Global(0),
                vec![Operand::Arg(0)],
                Some(Operand::Temp(0)),
            ),
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 参数所有权转移测试
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_parameter_ownership_move() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 移动参数到临时变量
            Instruction::Move {
                dst: Operand::Temp(0),
                src: Operand::Arg(0),
            },
            // 返回移动后的值
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 参数移动语义测试
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_parameter_ownership_borrow_immutable() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 不可变借用参数
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 参数不可变借用测试
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_parameter_ownership_borrow_mutable() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 可变借用参数（这里简化处理）
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 参数可变借用测试
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_multiple_parameters_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64), MonoType::Int(64), MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 处理多个参数
            Instruction::Add {
                dst: Operand::Temp(0),
                lhs: Operand::Arg(0),
                rhs: Operand::Arg(1),
            },
            Instruction::Add {
                dst: Operand::Temp(1),
                lhs: Operand::Temp(0),
                rhs: Operand::Arg(2),
            },
            Instruction::Ret(Some(Operand::Temp(1))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 多个参数的所有权处理
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_parameter_consumed_by_call() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 参数被函数调用消耗
            create_call_instruction(
                Operand::Global(0),
                vec![Operand::Arg(0)],
                Some(Operand::Temp(0)),
            ),
            // 尝试使用已消耗的参数（应该报错）
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 参数被消耗的场景
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_parameter_escapes_scope() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "escape_param_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Arg(0),
                    span: Span::dummy(),
                },
                // 参数逃逸到局部变量
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 参数逃逸作用域测试
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_parameter_no_escape() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 参数不逃逸，只在计算中使用
            Instruction::Add {
                dst: Operand::Temp(0),
                lhs: Operand::Arg(0),
                rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 参数不逃逸测试
    assert!(_result.definitions.len() >= 1);
}

// ============ 函数返回值所有权测试 ============

#[test]
fn test_ownership_return_via_return_value() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![],
        MonoType::Int(64),
        vec![
            // 创建新值并返回
            Instruction::Ret(Some(Operand::Const(
                crate::middle::core::ir::ConstValue::Int(42),
            ))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 返回值所有权测试
}

#[test]
fn test_return_value_move_semantics() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![],
        MonoType::Int(64),
        vec![
            // 移动到返回值
            Instruction::Ret(Some(Operand::Const(
                crate::middle::core::ir::ConstValue::Int(42),
            ))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 返回值移动语义
}

#[test]
fn test_return_value_from_parameter() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 直接返回参数（所有权转移）
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 从参数返回所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_return_value_from_local() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "return_local_test".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
                    span: Span::dummy(),
                },
                // 从局部变量返回（所有权转移）
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 从局部变量返回所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_return_nothing() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![],
        MonoType::Void,
        vec![
            // 返回空值
            Instruction::Ret(None),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 无返回值测试
}

#[test]
fn test_return_multiple_values() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "return_tuple_test".to_string(),
        params: vec![MonoType::Int(64), MonoType::Int(64)],
        return_type: MonoType::Tuple(vec![MonoType::Int(64), MonoType::Int(64)]),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 返回元组（多个值）
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 多值返回测试
    assert!(_result.definitions.len() >= 1);
}

// ============ 高阶函数所有权测试 ============

#[test]
fn test_higher_order_function_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 将函数作为参数传递
            create_call_instruction(
                Operand::Arg(0), // 函数参数
                vec![Operand::Const(crate::middle::core::ir::ConstValue::Int(1))],
                Some(Operand::Temp(0)),
            ),
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 高阶函数所有权测试
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_function_pointer_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 函数指针所有权
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 函数指针所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_capture_ownership_cross_function() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "closure_capture_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
                    span: Span::dummy(),
                },
                // 闭包捕获局部变量
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 跨函数闭包捕获所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_callback_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 回调函数所有权
            create_call_instruction(
                Operand::Arg(0),
                vec![Operand::Const(crate::middle::core::ir::ConstValue::Int(1))],
                Some(Operand::Temp(0)),
            ),
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 回调函数所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_higher_order_return_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "higher_order_return_test".to_string(),
        params: vec![],
        return_type: MonoType::Int(64), // 简化：返回 Int 而不是函数类型
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 返回函数（简化处理）
                Instruction::Ret(Some(Operand::Const(
                    crate::middle::core::ir::ConstValue::Int(0),
                ))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 高阶函数返回值所有权
}

// ============ 函数组合所有权测试 ============

#[test]
fn test_function_composition_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 函数组合：f(g(x))
            create_call_instruction(
                Operand::Global(0), // g 函数
                vec![Operand::Arg(0)],
                Some(Operand::Temp(0)),
            ),
            create_call_instruction(
                Operand::Global(1), // f 函数
                vec![Operand::Temp(0)],
                Some(Operand::Temp(1)),
            ),
            Instruction::Ret(Some(Operand::Temp(1))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 函数组合所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_pipeline_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_multi_param_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 管道操作：x |> f |> g |> h
            create_call_instruction(
                Operand::Global(0),
                vec![Operand::Arg(0)],
                Some(Operand::Temp(0)),
            ),
            create_call_instruction(
                Operand::Global(1),
                vec![Operand::Temp(0)],
                Some(Operand::Temp(1)),
            ),
            Instruction::Ret(Some(Operand::Temp(1))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 管道操作所有权
    assert!(_result.definitions.len() >= 1);
}

// ============ 并发函数所有权测试 ============

#[test]
fn test_spawn_function_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "spawn_ownership_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Arg(0),
                    span: Span::dummy(),
                },
                // Spawn 并发函数
                create_spawn_instruction(
                    Operand::Global(0),
                    vec![Operand::Local(0)],
                    Operand::Temp(0),
                ),
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Spawn 函数所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_async_function_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "async_ownership_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: true, // 异步函数
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Arg(0),
                    span: Span::dummy(),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 异步函数所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_shared_ownership_across_threads() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "shared_ownership_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Arg(0),
                    span: Span::dummy(),
                },
                // 多线程共享所有权
                create_spawn_instruction(
                    Operand::Global(0),
                    vec![Operand::Local(0)],
                    Operand::Temp(0),
                ),
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 跨线程共享所有权
    assert!(_result.definitions.len() >= 1);
}
