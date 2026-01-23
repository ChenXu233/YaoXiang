use crate::frontend::typecheck::MonoType;
use crate::middle::codegen::ir_builder::BytecodeGenerator;
use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::backends::common::Opcode;

#[test]
fn test_generate_complex_flow() {
    // Simulate:
    // complex():(Int) -> Int = (a) => {
    //     x = 0;
    //     if a > 10 {
    //         x = a + 1;
    //     } else {
    //         x = a * 2;
    //     }
    //     return x;
    // }

    let mut func = FunctionIR {
        name: "complex".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)], // x
        blocks: vec![],
        entry: 0,
    };

    // Block 0: Entry
    // x = 0
    // if a > 10 goto 1 else goto 2
    let block0 = BasicBlock {
        label: 0,
        instructions: vec![
            Instruction::Move {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::ir::ConstValue::Int(0)),
            },
            Instruction::Gt {
                dst: Operand::Temp(0),
                lhs: Operand::Arg(0),
                rhs: Operand::Const(crate::middle::ir::ConstValue::Int(10)),
            },
            Instruction::JmpIf(Operand::Temp(0), 1),
            Instruction::Jmp(2),
        ],
        successors: vec![1, 2],
    };

    // Block 1: Then
    // x = a + 1
    // goto 3
    let block1 = BasicBlock {
        label: 1,
        instructions: vec![
            Instruction::Add {
                dst: Operand::Local(0),
                lhs: Operand::Arg(0),
                rhs: Operand::Const(crate::middle::ir::ConstValue::Int(1)),
            },
            Instruction::Jmp(3),
        ],
        successors: vec![3],
    };

    // Block 2: Else
    // x = a * 2
    // goto 3
    let block2 = BasicBlock {
        label: 2,
        instructions: vec![
            Instruction::Mul {
                dst: Operand::Local(0),
                lhs: Operand::Arg(0),
                rhs: Operand::Const(crate::middle::ir::ConstValue::Int(2)),
            },
            Instruction::Jmp(3),
        ],
        successors: vec![3],
    };

    // Block 3: Exit
    // return x
    let block3 = BasicBlock {
        label: 3,
        instructions: vec![Instruction::Ret(Some(Operand::Local(0)))],
        successors: vec![],
    };

    func.blocks.push(block0);
    func.blocks.push(block1);
    func.blocks.push(block2);
    func.blocks.push(block3);

    let mut constants = Vec::new();
    let generator = BytecodeGenerator::new(&func, &mut constants);
    let code = generator.generate();

    // Verify we have instructions
    assert!(!code.instructions.is_empty());

    // Check for specific opcodes we expect
    let opcodes: Vec<u8> = code.instructions.iter().map(|i| i.opcode).collect();

    // Should contain LoadConst, Gt, JumpIfFalse (or similar), Add, Jump, Mul, Jump, Ret
    assert!(opcodes.contains(&(Opcode::I64Gt as u8)));
    assert!(opcodes.contains(&(Opcode::JmpIf as u8)));
    assert!(opcodes.contains(&(Opcode::I64Add as u8)));
    assert!(opcodes.contains(&(Opcode::I64Mul as u8)));
    assert!(opcodes.contains(&(Opcode::Jmp as u8)));
    assert!(opcodes.contains(&(Opcode::ReturnValue as u8)));
}
