//! ref 表达式 Codegen 测试
//!
//! 测试 ref 表达式的代码生成功能

use crate::middle::passes::codegen::CodegenContext;
use crate::middle::core::ir::{FunctionIR, Instruction, Operand, ModuleIR};
use crate::frontend::typecheck::MonoType;

/// 测试 ArcNew 指令生成
#[test]
fn test_codegen_arc_new() {
    let func = FunctionIR {
        name: "test_arc".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![crate::middle::core::ir::BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::ArcNew {
                    dst: Operand::Temp(0),
                    src: Operand::Local(0),
                },
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let module = ModuleIR {
        types: vec![],
        globals: vec![],
        functions: vec![func],
    };

    let mut codegen = CodegenContext::new(module);
    let _ = codegen.generate();
}

/// 测试 ArcClone 指令生成
#[test]
fn test_codegen_arc_clone() {
    let func = FunctionIR {
        name: "test_arc_clone".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![crate::middle::core::ir::BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::ArcClone {
                    dst: Operand::Temp(1),
                    src: Operand::Temp(0),
                },
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let module = ModuleIR {
        types: vec![],
        globals: vec![],
        functions: vec![func],
    };

    let mut codegen = CodegenContext::new(module);
    let _ = codegen.generate();
}

/// 测试 ArcDrop 指令生成
#[test]
fn test_codegen_arc_drop() {
    let func = FunctionIR {
        name: "test_arc_drop".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![crate::middle::core::ir::BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::ArcDrop(Operand::Temp(0)),
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let module = ModuleIR {
        types: vec![],
        globals: vec![],
        functions: vec![func],
    };

    let mut codegen = CodegenContext::new(module);
    let _ = codegen.generate();
}

/// 测试完整 Arc 操作序列
#[test]
fn test_arc_operation_sequence() {
    let func = FunctionIR {
        name: "test_arc_sequence".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![crate::middle::core::ir::BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::ArcNew {
                    dst: Operand::Temp(0),
                    src: Operand::Local(0),
                },
                Instruction::ArcClone {
                    dst: Operand::Temp(1),
                    src: Operand::Temp(0),
                },
                Instruction::ArcClone {
                    dst: Operand::Temp(2),
                    src: Operand::Temp(0),
                },
                Instruction::ArcDrop(Operand::Temp(2)),
                Instruction::ArcDrop(Operand::Temp(1)),
                Instruction::ArcDrop(Operand::Temp(0)),
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let module = ModuleIR {
        types: vec![],
        globals: vec![],
        functions: vec![func],
    };

    let mut codegen = CodegenContext::new(module);
    let result = codegen.generate();
    assert!(
        result.is_ok(),
        "Failed to generate bytecode: {:?}",
        result.err()
    );
}

/// 测试 ref 表达式的字节码生成
#[test]
fn test_ref_bytecode_generation() {
    let func = FunctionIR {
        name: "test_ref_bytecode".to_string(),
        params: vec![],
        return_type: MonoType::Arc(Box::new(MonoType::Int(64))),
        is_async: false,
        locals: vec![],
        blocks: vec![crate::middle::core::ir::BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::ArcNew {
                    dst: Operand::Temp(0),
                    src: Operand::Local(0),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let module = ModuleIR {
        types: vec![],
        globals: vec![],
        functions: vec![func],
    };

    let mut codegen = CodegenContext::new(module);
    let result = codegen.generate();
    assert!(
        result.is_ok(),
        "Failed to generate bytecode: {:?}",
        result.err()
    );
}

/// 测试嵌套 ref 表达式的字节码生成
#[test]
fn test_nested_ref_bytecode() {
    let func = FunctionIR {
        name: "test_nested_ref".to_string(),
        params: vec![],
        return_type: MonoType::Arc(Box::new(MonoType::Arc(Box::new(MonoType::Int(64))))),
        is_async: false,
        locals: vec![],
        blocks: vec![crate::middle::core::ir::BasicBlock {
            label: 0,
            instructions: vec![
                // inner = ref local_0
                Instruction::ArcNew {
                    dst: Operand::Temp(0),
                    src: Operand::Local(0),
                },
                // outer = ref inner (Temp(0))
                Instruction::ArcNew {
                    dst: Operand::Temp(1),
                    src: Operand::Temp(0),
                },
                Instruction::Ret(Some(Operand::Temp(1))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let module = ModuleIR {
        types: vec![],
        globals: vec![],
        functions: vec![func],
    };

    let mut codegen = CodegenContext::new(module);
    let result = codegen.generate();
    assert!(
        result.is_ok(),
        "Failed to generate nested ref bytecode: {:?}",
        result.err()
    );
}
