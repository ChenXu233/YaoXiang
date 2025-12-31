//! VM instructions

/// VM instruction
#[derive(Debug, Clone, Copy)]
pub struct Instruction {
    /// Opcode
    pub opcode: u8,
    /// Operands
    pub operands: [u32; 4],
}
