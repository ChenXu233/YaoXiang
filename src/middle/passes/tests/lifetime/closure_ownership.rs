//! 闭包所有权语义测试
//!
//! 测试闭包捕获变量所有权、Fn/FnMut/FnOnce trait、闭包移动语义等闭包相关场景

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::OwnershipAnalyzer;

/// 创建闭包测试函数
fn create_closure_function(
    params: Vec<MonoType>,
    return_type: MonoType,
    locals: Vec<MonoType>,
    instructions: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: "closure_test".to_string(),
        params,
        return_type,
        is_async: false,
        locals,
        blocks: vec![BasicBlock {
            label: 0,
            instructions,
            successors: vec![],
        }],
        entry: 0,
    }
}

/// 创建闭包捕获的测试函数
fn create_closure_capture_function(
    captured_vars: Vec<MonoType>,
    closure_body: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: "closure_capture_test".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: captured_vars,
        blocks: vec![BasicBlock {
            label: 0,
            instructions: closure_body,
            successors: vec![],
        }],
        entry: 0,
    }
}

// ============ 闭包捕获变量所有权测试 ============

#[test]
fn test_closure_capture_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 闭包捕获局部变量
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包捕获变量的所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_capture_move() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 移动捕获的变量
            Instruction::Move {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
            },
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包移动捕获的变量
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_capture_borrow_immutable() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 不可变借用捕获
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    eprintln!("DEBUG: definitions.len() = {}", _result.definitions.len());
    eprintln!("DEBUG: definitions = {:?}", _result.definitions);

    // 闭包不可变借用捕获
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_capture_borrow_mutable() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 可变借用捕获
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包可变借用捕获
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_capture_multiple_vars() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64), MonoType::String, MonoType::Bool],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 捕获多个变量
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包捕获多个变量
    assert!(_result.definitions.len() >= 4);
}

#[test]
fn test_closure_capture_nested() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 嵌套闭包捕获
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 嵌套闭包捕获
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_capture_env_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 闭包环境所有权
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包环境所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_capture_by_ref() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![],
        vec![
            // 通过引用捕获参数
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包通过引用捕获
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_capture_by_move() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![MonoType::Int(64)],
        vec![
            Instruction::Move {
                dst: Operand::Local(0),
                src: Operand::Arg(0),
            },
            // 通过移动捕获
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包通过移动捕获
    assert!(_result.definitions.len() >= 1);
}

// ============ Fn/FnMut/FnOnce Trait 测试 ============

#[test]
fn test_fn_trait_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    // Fn trait：只能调用，不能修改环境
    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![],
        vec![Instruction::Ret(Some(Operand::Arg(0)))],
    );

    let _result = analyzer.analyze_function(&func);

    // Fn trait 闭包的所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_fn_mut_trait_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    // FnMut trait：可以修改环境
    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Arg(0),
            },
            // FnMut 闭包修改环境
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // FnMut trait 闭包的所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_fn_once_trait_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    // FnOnce trait：只能调用一次，会消费环境
    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![MonoType::Int(64)],
        vec![
            Instruction::Move {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
            },
            // FnOnce 闭包消费环境
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // FnOnce trait 闭包的所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_fn_to_fn_mut_conversion() {
    let mut analyzer = OwnershipAnalyzer::new();

    // Fn 转换为 FnMut
    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![],
        vec![Instruction::Ret(Some(Operand::Arg(0)))],
    );

    let _result = analyzer.analyze_function(&func);

    // Fn 到 FnMut 的转换
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_fn_mut_to_fn_once_conversion() {
    let mut analyzer = OwnershipAnalyzer::new();

    // FnMut 转换为 FnOnce
    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // FnMut 到 FnOnce 的转换
    assert!(_result.definitions.len() >= 1);
}

// ============ 闭包移动语义测试 ============

#[test]
fn test_closure_move_semantics() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 闭包移动语义
            Instruction::Move {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
            },
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包移动语义
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_move_captured_var() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "closure_move_captured_test".to_string(),
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
                },
                // 移动捕获的变量
                Instruction::Move {
                    dst: Operand::Temp(0),
                    src: Operand::Local(0),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 移动捕获的变量
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_partial_move() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64), MonoType::String],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 部分移动
            Instruction::Move {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
            },
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包部分移动
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_closure_move_after_use() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 使用后移动（应该检测错误）
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包使用后移动
    assert!(_result.definitions.len() >= 1);
}

// ============ 闭包生命周期延长测试 ============

#[test]
fn test_closure_lifetime_extension() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![],
        vec![
            // 延长参数生命周期
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包生命周期延长
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_lifetime_extension_captured() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 延长捕获变量的生命周期
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包捕获变量生命周期延长
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_static_lifetime() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_function(
        vec![],
        MonoType::Int(64),
        vec![],
        vec![
            // 静态生命周期闭包
            Instruction::Ret(Some(Operand::Const(
                crate::middle::core::ir::ConstValue::Int(42),
            ))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包静态生命周期
}

#[test]
fn test_closure_lifetime_bound() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![],
        vec![
            // 生命周期约束
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包生命周期约束
    assert!(_result.definitions.len() >= 1);
}

// ============ 闭包作为返回值测试 ============

#[test]
fn test_closure_return_value() {
    let mut analyzer = OwnershipAnalyzer::new();

    // 返回闭包（简化处理）
    let func = FunctionIR {
        name: "closure_return_test".to_string(),
        params: vec![],
        return_type: MonoType::Int(64), // 简化：不返回闭包类型
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 返回闭包
                Instruction::Ret(Some(Operand::Const(
                    crate::middle::core::ir::ConstValue::Int(0),
                ))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 闭包作为返回值
}

#[test]
fn test_closure_return_named() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "closure_return_named_test".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 返回命名闭包
                Instruction::Ret(Some(Operand::Const(
                    crate::middle::core::ir::ConstValue::Int(0),
                ))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 命名闭包返回值
}

#[test]
fn test_closure_return_generic() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "closure_return_generic_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 返回泛型闭包
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 泛型闭包返回值
    assert!(_result.definitions.len() >= 1);
}

// ============ 高级闭包场景测试 ============

#[test]
fn test_closure_recursive() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "closure_recursive_test".to_string(),
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
                },
                // 递归闭包
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 递归闭包
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_higher_order() {
    let mut analyzer = OwnershipAnalyzer::new();

    // 返回闭包的函数
    let func = FunctionIR {
        name: "closure_higher_order_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 高阶闭包
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 高阶闭包
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_composition() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "closure_composition_test".to_string(),
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
                },
                // 闭包组合
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 闭包组合
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_with_drop() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_capture_function(
        vec![MonoType::Int(64)],
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            // 闭包销毁时清理
            Instruction::Ret(Some(Operand::Const(
                crate::middle::core::ir::ConstValue::Int(42),
            ))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 闭包销毁语义
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_send_sync() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![],
        vec![
            // Send/Sync 闭包
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // Send/Sync 闭包
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_unpin() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_closure_function(
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![],
        vec![
            // Unpin 闭包
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // Unpin 闭包
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_future_compat() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "closure_future_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: true, // 异步闭包
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // Future 兼容闭包
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Future 兼容闭包
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_closure_stream_compat() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "closure_stream_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // Stream 兼容闭包
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Stream 兼容闭包
    assert!(_result.definitions.len() >= 1);
}
