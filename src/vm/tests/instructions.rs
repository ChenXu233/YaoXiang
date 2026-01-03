//! Instruction 单元测试
//!
//! 测试虚拟机指令的结构和行为

use crate::vm::instructions::Instruction;

#[cfg(test)]
mod instruction_tests {
    use super::*;

    #[test]
    fn test_instruction_new() {
        let instruction = Instruction::new(0x01, [1, 2, 3, 4]);
        assert_eq!(instruction.opcode, 0x01);
        assert_eq!(instruction.operands, [1, 2, 3, 4]);
    }

    #[test]
    fn test_instruction_get_opcode() {
        let instruction = Instruction::new(0xAB, [0, 0, 0, 0]);
        assert_eq!(instruction.get_opcode(), 0xAB);
    }

    #[test]
    fn test_instruction_get_operand_valid() {
        let instruction = Instruction::new(0x01, [10, 20, 30, 40]);
        assert_eq!(instruction.get_operand(0), Some(10));
        assert_eq!(instruction.get_operand(1), Some(20));
        assert_eq!(instruction.get_operand(2), Some(30));
        assert_eq!(instruction.get_operand(3), Some(40));
    }

    #[test]
    fn test_instruction_get_operand_invalid() {
        let instruction = Instruction::new(0x01, [10, 20, 30, 40]);
        assert_eq!(instruction.get_operand(4), None);
        assert_eq!(instruction.get_operand(100), None);
    }

    #[test]
    fn test_instruction_debug() {
        let instruction = Instruction::new(0x01, [1, 2, 3, 4]);
        let debug = format!("{:?}", instruction);
        assert!(debug.contains("Instruction"));
    }

    #[test]
    fn test_instruction_clone() {
        let instruction = Instruction::new(0xFF, [100, 200, 300, 400]);
        let cloned = instruction.clone();
        assert_eq!(cloned.opcode, instruction.opcode);
        assert_eq!(cloned.operands, instruction.operands);
    }

    #[test]
    fn test_instruction_copy() {
        let instruction = Instruction::new(0x55, [5, 10, 15, 20]);
        let copied = instruction;
        assert_eq!(copied.opcode, 0x55);
    }
}

#[cfg(test)]
mod instruction_variants_tests {
    use super::*;

    /// 测试各种 opcode 和 operand 组合
    #[test]
    fn test_instruction_nop() {
        let instruction = Instruction::new(0x00, [0, 0, 0, 0]);
        assert_eq!(instruction.opcode, 0x00);
    }

    #[test]
    fn test_instruction_with_large_operands() {
        let instruction = Instruction::new(0xFF, [u32::MAX, u32::MAX, u32::MAX, u32::MAX]);
        assert_eq!(instruction.operands, [u32::MAX, u32::MAX, u32::MAX, u32::MAX]);
    }

    #[test]
    fn test_instruction_with_zero_operands() {
        let instruction = Instruction::new(0x10, [0, 0, 0, 0]);
        assert_eq!(instruction.get_operand(0), Some(0));
        assert_eq!(instruction.get_operand(1), Some(0));
    }

    #[test]
    fn test_instruction_partial_operands() {
        let instruction = Instruction::new(0x05, [42, 0, 0, 0]);
        assert_eq!(instruction.get_operand(0), Some(42));
        assert_eq!(instruction.get_operand(1), Some(0));
        assert_eq!(instruction.get_operand(2), Some(0));
        assert_eq!(instruction.get_operand(3), Some(0));
    }
}
