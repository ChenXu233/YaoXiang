//! Bytecode Generator 单元测试
//!
//! 测试中间 IR 到字节码的翻译功能

use crate::frontend::typecheck::MonoType;
use crate::middle::codegen::bytecode::FunctionCode;
use crate::middle::codegen::generator::BytecodeGenerator;
use crate::middle::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, Operand};
use crate::vm::opcode::TypedOpcode;

#[test]
fn test_generate_add() {
    let mut func = FunctionIR {
        name: "add".to_string(),
        params: vec![MonoType::Int(64), MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![],
        entry: 0,
    };

    let block = BasicBlock {
        label: 0,
        instructions: vec![
            Instruction::Add {
                dst: Operand::Temp(0),
                lhs: Operand::Arg(0),
                rhs: Operand::Arg(1),
            },
            Instruction::Ret(Some(Operand::Temp(0))),
        ],
        successors: vec![],
    };
    func.blocks.push(block);

    let generator = BytecodeGenerator::new(&func);
    let code = generator.generate();

    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0].opcode, TypedOpcode::I64Add as u8);
    assert_eq!(code.instructions[1].opcode, TypedOpcode::ReturnValue as u8);
}

#[test]
fn test_bytecode_generator_new() {
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![],
            successors: vec![],
        }],
        entry: 0,
    };

    let generator = BytecodeGenerator::new(&func);
    // 验证生成器可以通过函数名创建（通过生成结果验证）
    let code = generator.generate();
    assert_eq!(code.name, "test");
}

#[test]
fn test_generate_sub() {
    let func = FunctionIR {
        name: "sub".to_string(),
        params: vec![MonoType::Int(64), MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Sub {
                    dst: Operand::Temp(0),
                    lhs: Operand::Arg(0),
                    rhs: Operand::Arg(1),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let generator = BytecodeGenerator::new(&func);
    let code = generator.generate();

    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0].opcode, TypedOpcode::I64Sub as u8);
}

#[test]
fn test_generate_mul() {
    let func = FunctionIR {
        name: "mul".to_string(),
        params: vec![MonoType::Float(64), MonoType::Float(64)],
        return_type: MonoType::Float(64),
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Mul {
                    dst: Operand::Temp(0),
                    lhs: Operand::Arg(0),
                    rhs: Operand::Arg(1),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let generator = BytecodeGenerator::new(&func);
    let code = generator.generate();

    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0].opcode, TypedOpcode::F64Mul as u8);
}

#[test]
fn test_generate_move() {
    let func = FunctionIR {
        name: "move_test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
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

    let generator = BytecodeGenerator::new(&func);
    let code = generator.generate();

    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0].opcode, TypedOpcode::Mov as u8);
}

#[test]
fn test_generate_jump() {
    let func = FunctionIR {
        name: "jump_test".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![
            BasicBlock {
                label: 0,
                instructions: vec![Instruction::Jmp(1)],
                successors: vec![1],
            },
            BasicBlock {
                label: 1,
                instructions: vec![Instruction::Ret(None)],
                successors: vec![],
            },
        ],
        entry: 0,
    };

    let generator = BytecodeGenerator::new(&func);
    let code = generator.generate();

    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0].opcode, TypedOpcode::Jmp as u8);
}
