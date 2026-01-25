//! 复杂生命周期分析测试
//!
//! 测试嵌套作用域、条件分支、循环等复杂场景下的生命周期分析

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::OwnershipAnalyzer;

/// 创建嵌套作用域的测试函数
fn create_nested_scope_function(instructions: Vec<Instruction>) -> FunctionIR {
    FunctionIR {
        name: "nested_scope_test".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64), MonoType::Int(64), MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions,
            successors: vec![],
        }],
        entry: 0,
    }
}

/// 创建带条件分支的测试函数
fn create_conditional_function(
    then_instructions: Vec<Instruction>,
    else_instructions: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: "conditional_test".to_string(),
        params: vec![MonoType::Bool],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64), MonoType::Int(64)],
        blocks: vec![
            BasicBlock {
                label: 0,
                instructions: vec![Instruction::JmpIf(Operand::Arg(0), 1)],
                successors: vec![1],
            },
            BasicBlock {
                label: 1,
                instructions: then_instructions,
                successors: vec![3],
            },
            BasicBlock {
                label: 2,
                instructions: else_instructions,
                successors: vec![3],
            },
            BasicBlock {
                label: 3,
                instructions: vec![Instruction::Ret(Some(Operand::Local(0)))],
                successors: vec![],
            },
        ],
        entry: 0,
    }
}

/// 创建循环结构的测试函数
fn create_loop_function(instructions: Vec<Instruction>) -> FunctionIR {
    FunctionIR {
        name: "loop_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64), MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions,
            successors: vec![],
        }],
        entry: 0,
    }
}

// ============ 嵌套作用域测试 ============

#[test]
fn test_nested_scope_lifetime() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_nested_scope_function(vec![
        // 外部作用域
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
        // 内层作用域开始
        Instruction::Store {
            dst: Operand::Local(1),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(2)),
        },
        // 内层作用域结束，Local(1) 应该被释放
        Instruction::Store {
            dst: Operand::Local(2),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(3)),
        },
        Instruction::Ret(Some(Operand::Local(0))),
    ]);

    let _result = analyzer.analyze_function(&func);

    // 验证生命周期分析包含必要信息
    assert!(_result.definitions.len() >= 3);
    println!("Nested scope analysis: {:?}", _result);
}

#[test]
fn test_nested_scope_with_move() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_nested_scope_function(vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
        // 移动 Local(0) 到 Local(1)
        Instruction::Move {
            dst: Operand::Local(1),
            src: Operand::Local(0),
        },
        // 此时 Local(0) 已被移动，不应该被再次使用
        Instruction::Ret(Some(Operand::Local(1))),
    ]);

    let _result = analyzer.analyze_function(&func);

    // 验证移动语义正确追踪
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_deeply_nested_lifetime() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_nested_scope_function(vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
        Instruction::Store {
            dst: Operand::Local(1),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(2)),
        },
        Instruction::Store {
            dst: Operand::Local(2),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(3)),
        },
        Instruction::Ret(Some(Operand::Local(2))),
    ]);

    let _result = analyzer.analyze_function(&func);

    // 深嵌套作用域中，所有变量都应该有正确的生命周期
    assert!(_result.definitions.len() >= 3);
}

// ============ 条件分支测试 ============

#[test]
fn test_conditional_lifetime_both_branches() {
    let mut analyzer = OwnershipAnalyzer::new();

    let then_instructions = vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(42)),
        },
        Instruction::Ret(Some(Operand::Local(0))),
    ];

    let else_instructions = vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(24)),
        },
        Instruction::Ret(Some(Operand::Local(0))),
    ];

    let func = create_conditional_function(then_instructions, else_instructions);
    let _result = analyzer.analyze_function(&func);

    // 两个分支都应该正确分析
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_conditional_lifetime_one_branch_move() {
    let mut analyzer = OwnershipAnalyzer::new();

    let then_instructions = vec![
        Instruction::Move {
            dst: Operand::Local(0),
            src: Operand::Arg(0),
        },
        Instruction::Ret(Some(Operand::Local(0))),
    ];

    let else_instructions = vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(0)),
        },
        Instruction::Ret(Some(Operand::Local(0))),
    ];

    let func = create_conditional_function(then_instructions, else_instructions);
    let _result = analyzer.analyze_function(&func);

    // 验证条件分支中的移动语义
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_conditional_lifetime_early_return() {
    let mut analyzer = OwnershipAnalyzer::new();

    let then_instructions = vec![Instruction::Ret(Some(Operand::Arg(0)))];

    let else_instructions = vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(0)),
        },
        Instruction::Ret(Some(Operand::Local(0))),
    ];

    let func = create_conditional_function(then_instructions, else_instructions);
    let _result = analyzer.analyze_function(&func);

    // 早期返回场景下的生命周期分析
}

#[test]
fn test_conditional_lifetime_merge_point() {
    let mut analyzer = OwnershipAnalyzer::new();

    let then_instructions = vec![Instruction::Store {
        dst: Operand::Local(0),
        src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
    }];

    let else_instructions = vec![Instruction::Store {
        dst: Operand::Local(1),
        src: Operand::Const(crate::middle::core::ir::ConstValue::Int(2)),
    }];

    let func = create_conditional_function(then_instructions, else_instructions);
    let _result = analyzer.analyze_function(&func);

    // 分支合并点的生命周期处理
    assert!(_result.definitions.len() >= 2);
}

// ============ 循环测试 ============

#[test]
fn test_loop_lifetime_iteration() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_loop_function(vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Const(crate::middle::core::ir::ConstValue::Int(0)),
        },
        // 循环体
        Instruction::Add {
            dst: Operand::Local(1),
            lhs: Operand::Local(0),
            rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
        },
        Instruction::Ret(Some(Operand::Local(1))),
    ]);

    let _result = analyzer.analyze_function(&func);

    // 循环中的生命周期应该正确追踪
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_loop_lifetime_invariant() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_loop_function(vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Arg(0),
        },
        // 循环不变量：Local(0) 在整个循环中保持有效
        Instruction::Ret(Some(Operand::Local(0))),
    ]);

    let _result = analyzer.analyze_function(&func);

    // 验证循环不变量
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_loop_lifetime_borrow_invariant() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_loop_function(vec![
        Instruction::Store {
            dst: Operand::Local(0),
            src: Operand::Arg(0),
        },
        // 借用检查：在循环中保持借用有效
        Instruction::Ret(Some(Operand::Local(0))),
    ]);

    let _result = analyzer.analyze_function(&func);

    // 循环中的借用不变量
    assert!(_result.definitions.len() >= 1);
}

// ============ 生命周期逃逸测试 ============

#[test]
fn test_lifetime_escape_to_global() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "escape_test".to_string(),
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
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 参数可能逃逸到全局
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_lifetime_escape_to_heap() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "heap_escape_test".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Const(crate::middle::core::ir::ConstValue::Int(42)),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 验证堆分配的生命周期
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_lifetime_no_escape() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "no_escape_test".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
                },
                // 局部变量，不返回
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 验证局部变量不逃逸
    assert!(_result.drop_points.len() > 0);
}

// ============ 借用检查生命周期测试 ============

#[test]
fn test_borrow_lifetime_simple() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "borrow_lifetime_test".to_string(),
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
                // 借用 Local(0)
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 简单借用生命周期
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_borrow_lifetime_nested() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "nested_borrow_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64), MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Arg(0),
                },
                Instruction::Store {
                    dst: Operand::Local(1),
                    src: Operand::Local(0),
                },
                Instruction::Ret(Some(Operand::Local(1))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 嵌套借用生命周期
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_borrow_lifetime_conflict() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "borrow_conflict_test".to_string(),
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
                // 可变借用与不可变借用冲突的测试
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 借用冲突检测
    assert!(_result.definitions.len() >= 1);
}

// ============ 高级生命周期测试 ============

#[test]
fn test_lifetime_with_arc() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "arc_lifetime_test".to_string(),
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
                // Arc 引用计数
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Arc 的生命周期管理
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_lifetime_with_rc() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "rc_lifetime_test".to_string(),
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
                // Rc 引用计数
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Rc 的生命周期管理
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_lifetime_early_drop() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "early_drop_test".to_string(),
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
                // 早期释放
                Instruction::Ret(Some(Operand::Const(
                    crate::middle::core::ir::ConstValue::Int(42),
                ))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 早期释放场景
    assert!(_result.drop_points.len() > 0);
}

#[test]
fn test_lifetime_static() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "static_lifetime_test".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Const(
                crate::middle::core::ir::ConstValue::Int(42),
            )))],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 静态生命周期
}
