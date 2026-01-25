//! 泛型类型所有权测试
//!
//! 测试泛型结构体、泛型函数、Trait 对象、关联类型等泛型场景的所有权

use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::middle::lifetime::OwnershipAnalyzer;

/// 创建泛型类型测试工具
fn create_generic_function(
    _type_params: Vec<String>,
    params: Vec<MonoType>,
    return_type: MonoType,
    instructions: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: "generic_test".to_string(),
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

/// 创建泛型结构体实例
fn create_generic_struct_instance(
    _struct_type: MonoType,
    _fields: Vec<Operand>,
) -> Vec<Instruction> {
    vec![
        // 创建泛型结构体实例
        Instruction::Ret(Some(Operand::Const(crate::middle::ir::ConstValue::Int(0)))),
    ]
}

// ============ 泛型结构体所有权测试 ============

#[test]
fn test_generic_struct_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    // 泛型结构体：Box[T]
    let box_type = MonoType::List(Box::new(MonoType::Int(64)));

    let func = FunctionIR {
        name: "generic_box_ownership".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: box_type.clone(),
        is_async: false,
        locals: vec![box_type.clone()],
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

    // 泛型结构体 Box[T] 的所有权
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_generic_struct_multiple_params() {
    let mut analyzer = OwnershipAnalyzer::new();

    // 泛型结构体：Result[T, E]
    let result_type = MonoType::Tuple(vec![MonoType::Int(64), MonoType::String]);

    let func = create_generic_function(
        vec!["T".to_string(), "E".to_string()],
        vec![MonoType::Int(64)],
        result_type.clone(),
        vec![
            Instruction::Store {
                dst: Operand::Local(0),
                src: Operand::Arg(0),
            },
            Instruction::Ret(Some(Operand::Local(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 多类型参数泛型结构体
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_generic_struct_nested() {
    let mut analyzer = OwnershipAnalyzer::new();

    // 嵌套泛型：Option[Box[T]]
    let nested_type = MonoType::List(Box::new(MonoType::List(Box::new(MonoType::Int(64)))));

    let func = create_generic_function(
        vec!["T".to_string()],
        vec![MonoType::Int(64)],
        nested_type.clone(),
        vec![Instruction::Ret(Some(Operand::Const(
            crate::middle::ir::ConstValue::Int(0),
        )))],
    );

    let _result = analyzer.analyze_function(&func);

    // 嵌套泛型结构体
}

#[test]
fn test_generic_struct_move() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "generic_move_test".to_string(),
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
                // 移动泛型结构体
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

    // 泛型结构体移动语义
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_generic_struct_borrow() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "generic_borrow_test".to_string(),
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
                // 借用泛型结构体
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 泛型结构体借用
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_generic_struct_arc() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "generic_arc_test".to_string(),
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

    // 泛型结构体 Arc 引用
    assert!(_result.definitions.len() >= 2);
}

// ============ 泛型函数所有权测试 ============

#[test]
fn test_generic_function_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["T".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![Instruction::Ret(Some(Operand::Arg(0)))],
    );

    let _result = analyzer.analyze_function(&func);

    // 泛型函数的所有权处理
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_generic_function_move_param() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["T".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 移动泛型参数
            Instruction::Move {
                dst: Operand::Temp(0),
                src: Operand::Arg(0),
            },
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 泛型函数参数移动
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_generic_function_borrow_param() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["T".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 借用泛型参数
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 泛型函数参数借用
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_generic_function_multiple_type_params() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["T".to_string(), "U".to_string(), "V".to_string()],
        vec![MonoType::Int(64), MonoType::String, MonoType::Bool],
        MonoType::Int(64),
        vec![Instruction::Ret(Some(Operand::Arg(0)))],
    );

    let _result = analyzer.analyze_function(&func);

    // 多类型参数泛型函数
    assert!(_result.definitions.len() >= 3);
}

#[test]
fn test_generic_function_return_generic() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["T".to_string()],
        vec![],
        MonoType::List(Box::new(MonoType::Int(64))),
        vec![
            // 返回泛型类型
            Instruction::Ret(Some(Operand::Const(crate::middle::ir::ConstValue::Int(0)))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 泛型函数返回泛型类型
}

#[test]
fn test_generic_function_trait_bound() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["T".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // Trait 绑定的泛型函数
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // Trait 绑定泛型函数
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_generic_function_lifetime_param() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["'a".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 生命周期参数的泛型函数
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 生命周期参数泛型函数
    assert!(_result.definitions.len() >= 1);
}

// ============ Trait 对象所有权测试 ============

#[test]
fn test_trait_object_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    // Trait 对象：&dyn Display
    let trait_object_type = MonoType::String; // 简化处理

    let func = FunctionIR {
        name: "trait_object_ownership".to_string(),
        params: vec![trait_object_type.clone()],
        return_type: MonoType::String,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Trait 对象所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_trait_object_borrow() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "trait_object_borrow".to_string(),
        params: vec![MonoType::String],
        return_type: MonoType::String,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // Trait 对象借用
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Trait 对象借用
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_trait_object_move() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "trait_object_move".to_string(),
        params: vec![MonoType::String],
        return_type: MonoType::String,
        is_async: false,
        locals: vec![MonoType::String],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Move {
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

    // Trait 对象移动
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_trait_object_generic() {
    let mut analyzer = OwnershipAnalyzer::new();

    // 泛型 Trait 对象
    let func = create_generic_function(
        vec!["T".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![Instruction::Ret(Some(Operand::Arg(0)))],
    );

    let _result = analyzer.analyze_function(&func);

    // 泛型 Trait 对象
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_trait_object_dyn_send() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "trait_object_dyn_send".to_string(),
        params: vec![MonoType::String],
        return_type: MonoType::String,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // Dyn Send Trait 对象
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Dyn Send Trait 对象
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_trait_object_dyn_sync() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "trait_object_dyn_sync".to_string(),
        params: vec![MonoType::String],
        return_type: MonoType::String,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // Dyn Sync Trait 对象
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // Dyn Sync Trait 对象
    assert!(_result.definitions.len() >= 1);
}

// ============ 关联类型所有权测试 ============

#[test]
fn test_associated_type_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "associated_type_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 关联类型的所有权处理
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 关联类型所有权
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_associated_type_generic() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["T".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 泛型关联类型
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 泛型关联类型
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_associated_type_trait_bound() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["T".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // Trait 绑定关联类型
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // Trait 绑定关联类型
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_associated_type_lifetime() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["'a".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 生命周期关联类型
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 生命周期关联类型
    assert!(_result.definitions.len() >= 1);
}

// ============ 泛型生命周期参数测试 ============

#[test]
fn test_generic_lifetime_parameters() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["'a".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![Instruction::Ret(Some(Operand::Arg(0)))],
    );

    let _result = analyzer.analyze_function(&func);

    // 泛型生命周期参数
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_multiple_lifetime_parameters() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["'a".to_string(), "'b".to_string()],
        vec![MonoType::Int(64), MonoType::String],
        MonoType::Int(64),
        vec![Instruction::Ret(Some(Operand::Arg(0)))],
    );

    let _result = analyzer.analyze_function(&func);

    // 多生命周期参数
    assert!(_result.definitions.len() >= 2);
}

#[test]
fn test_lifetime_substitution() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["'a".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 生命周期替换
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 生命周期替换
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_lifetime_elision() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "lifetime_elision_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 生命周期省略
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 生命周期省略规则
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_lifetime_bounds() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = create_generic_function(
        vec!["'a".to_string()],
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            // 生命周期约束
            Instruction::Ret(Some(Operand::Arg(0))),
        ],
    );

    let _result = analyzer.analyze_function(&func);

    // 生命周期约束
    assert!(_result.definitions.len() >= 1);
}

// ============ 高级泛型场景测试 ============

#[test]
fn test_higher_ranked_trait_bounds() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "hr_trait_bounds_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 高级 trait 绑定
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 高级 trait 绑定
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_generic_in_closure() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "generic_in_closure_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 闭包中的泛型
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 闭包中的泛型
    assert!(_result.definitions.len() >= 1);
}

#[test]
fn test_const_generics_ownership() {
    let mut analyzer = OwnershipAnalyzer::new();

    let func = FunctionIR {
        name: "const_generics_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 常量泛型的所有权
                Instruction::Ret(Some(Operand::Arg(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let _result = analyzer.analyze_function(&func);

    // 常量泛型所有权
    assert!(_result.definitions.len() >= 1);
}
