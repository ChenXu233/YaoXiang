//! Bytecode IR unit tests (RFC-009 v9 Borrow/Release token opcodes)
//!
//! Covers:
//! - Instruction size calculations
//! - Register and label Display formatting
//! - Borrow/Release opcode and size mapping
//! - Borrow/Release round-trip encode-decode via `build_and_decode`
//! - MonoType::Ref -> IrType::Void conversion

use crate::middle::core::bytecode::{
    BinaryOp, BytecodeInstr, BytecodeModule, CompareOp, FunctionRef, Label, Reg, UnaryOp,
};
use crate::middle::core::ir::{ConstValue, Type as IrType};
use crate::backends::common::Opcode;
use crate::frontend::core::typecheck::MonoType;
use crate::middle::passes::codegen::bytecode::BytecodeInstruction;

// ========================
// Instruction Size
// ========================

#[test]
fn test_nop_size_is_one_byte() {
    // Arrange
    let nop = BytecodeInstr::Nop;
    // Act
    let size = nop.size();
    // Assert
    assert_eq!(
        size, 1,
        "Nop instruction should occupy exactly 1 byte (opcode only)"
    );
}

#[test]
fn test_mov_size_is_five_bytes() {
    // Arrange
    let mov = BytecodeInstr::Mov {
        dst: Reg(0),
        src: Reg(1),
    };
    // Act
    let size = mov.size();
    // Assert
    assert_eq!(
        size, 5,
        "Mov instruction should occupy 5 bytes: opcode(1) + dst(2) + src(2)"
    );
}

// ========================
// Display Formatting
// ========================

#[test]
fn test_reg_display_format() {
    // Arrange / Act / Assert
    assert_eq!(
        format!("{}", Reg(0)),
        "r0",
        "Reg(0) should display as \"r0\""
    );
    assert_eq!(
        format!("{}", Reg(15)),
        "r15",
        "Reg(15) should display as \"r15\""
    );
}

#[test]
fn test_label_display_format() {
    // Arrange / Act / Assert
    assert_eq!(
        format!("{}", Label(0)),
        "L0",
        "Label(0) should display as \"L0\""
    );
    assert_eq!(
        format!("{}", Label(10)),
        "L10",
        "Label(10) should display as \"L10\""
    );
}

// ========================
// Borrow/Release Opcode Mapping
// ========================

#[test]
fn test_borrow_immutable_opcode_is_borrow() {
    // Arrange
    let instr = BytecodeInstr::Borrow {
        dst: Reg(1),
        src: Reg(2),
        mutable: false,
    };
    // Act
    let opcode = instr.opcode();
    // Assert
    assert_eq!(
        opcode,
        Opcode::Borrow,
        "Immutable Borrow should map to Opcode::Borrow"
    );
}

#[test]
fn test_borrow_mutable_opcode_is_borrow() {
    // Arrange
    let instr = BytecodeInstr::Borrow {
        dst: Reg(1),
        src: Reg(2),
        mutable: true,
    };
    // Act
    let opcode = instr.opcode();
    // Assert
    assert_eq!(
        opcode,
        Opcode::Borrow,
        "Mutable Borrow should map to Opcode::Borrow"
    );
}

#[test]
fn test_borrow_size_is_six_bytes() {
    // Arrange
    let instr = BytecodeInstr::Borrow {
        dst: Reg(1),
        src: Reg(2),
        mutable: true,
    };
    // Act
    let size = instr.size();
    // Assert: opcode(1) + dst(2) + src(2) + mutable(1) = 6
    assert_eq!(
        size, 6,
        "Borrow instruction should occupy 6 bytes: opcode(1)+dst(2)+src(2)+mutable(1)"
    );
}

#[test]
fn test_release_opcode_is_release() {
    // Arrange
    let instr = BytecodeInstr::Release { src: Reg(3) };
    // Act
    let opcode = instr.opcode();
    // Assert
    assert_eq!(
        opcode,
        Opcode::Release,
        "Release should map to Opcode::Release"
    );
}

#[test]
fn test_release_size_is_three_bytes() {
    // Arrange
    let instr = BytecodeInstr::Release { src: Reg(3) };
    // Act
    let size = instr.size();
    // Assert: opcode(1) + src(2) = 3
    assert_eq!(
        size, 3,
        "Release instruction should occupy 3 bytes: opcode(1)+src(2)"
    );
}

// ========================
// Borrow/Release Round-trip Tests
// ========================

/// Helper: build a minimal BytecodeFile with one function containing
/// the given raw BytecodeInstructions, then decode via `From<BytecodeFile>`.
fn build_and_decode(instrs: Vec<BytecodeInstruction>) -> BytecodeModule {
    use crate::middle::passes::codegen::bytecode as bcfile;
    let func = bcfile::FunctionCode {
        name: "test_fn".to_string(),
        params: vec![],
        return_type: crate::frontend::core::typecheck::MonoType::Void,
        instructions: instrs,
        local_count: 0,
        debug_map: std::collections::HashMap::new(),
    };
    let file = bcfile::BytecodeFile {
        header: bcfile::FileHeader::default(),
        type_table: vec![],
        const_pool: vec![],
        code_section: bcfile::CodeSection {
            functions: vec![func],
        },
        debug_section: None,
    };
    BytecodeModule::from(file)
}

/// Helper: assert that the single decoded instruction is `Borrow`
/// with the expected `dst`, `src`, and `mutable` fields.
fn assert_borrow_instr(
    instr: &BytecodeInstr,
    expected_dst: Reg,
    expected_src: Reg,
    expected_mutable: bool,
) {
    match instr {
        BytecodeInstr::Borrow { dst, src, mutable } => {
            assert_eq!(
                *dst, expected_dst,
                "Borrow dst should be {:?}",
                expected_dst
            );
            assert_eq!(
                *src, expected_src,
                "Borrow src should be {:?}",
                expected_src
            );
            assert_eq!(
                *mutable, expected_mutable,
                "Borrow mutable should be {}",
                expected_mutable
            );
        }
        other => panic!("Expected Borrow instruction, got {:?}", other),
    }
}

/// Helper: assert that the single decoded instruction is `Release`
/// with the expected `src` field.
fn assert_release_instr(
    instr: &BytecodeInstr,
    expected_src: Reg,
) {
    match instr {
        BytecodeInstr::Release { src } => {
            assert_eq!(
                *src, expected_src,
                "Release src should be {:?}",
                expected_src
            );
        }
        other => panic!("Expected Release instruction, got {:?}", other),
    }
}

#[test]
fn test_borrow_roundtrip_immutable() {
    // Arrange: encode Borrow dst=1, src=2, mutable=false
    let encoded = BytecodeInstruction::new(
        Opcode::Borrow,
        vec![1, 0, 2, 0, 0], // dst=1 LE, src=2 LE, mutable=false
    );
    // Act
    let module = build_and_decode(vec![encoded]);
    let instrs = &module.functions[0].instructions;
    // Assert
    assert_eq!(
        module.functions.len(),
        1,
        "Module should contain exactly 1 function"
    );
    assert_eq!(
        instrs.len(),
        1,
        "Function should contain exactly 1 instruction"
    );
    assert_borrow_instr(&instrs[0], Reg(1), Reg(2), false);
}

#[test]
fn test_borrow_roundtrip_mutable() {
    // Arrange: encode Borrow dst=1, src=2, mutable=true
    let encoded = BytecodeInstruction::new(
        Opcode::Borrow,
        vec![1, 0, 2, 0, 1], // dst=1 LE, src=2 LE, mutable=true
    );
    // Act
    let module = build_and_decode(vec![encoded]);
    let instrs = &module.functions[0].instructions;
    // Assert
    assert_eq!(
        instrs.len(),
        1,
        "Function should contain exactly 1 instruction"
    );
    assert_borrow_instr(&instrs[0], Reg(1), Reg(2), true);
}

#[test]
fn test_release_roundtrip() {
    // Arrange: encode Release src=3
    let encoded = BytecodeInstruction::new(
        Opcode::Release,
        vec![3, 0], // src=3 LE
    );
    // Act
    let module = build_and_decode(vec![encoded]);
    let instrs = &module.functions[0].instructions;
    // Assert
    assert_eq!(
        instrs.len(),
        1,
        "Function should contain exactly 1 instruction"
    );
    assert_release_instr(&instrs[0], Reg(3));
}

#[test]
fn test_borrow_release_combined_roundtrip() {
    // Arrange: Borrow(dst=5, src=10, mutable=true) followed by Release(src=5)
    let borrow_instr = BytecodeInstruction::new(
        Opcode::Borrow,
        vec![5, 0, 10, 0, 1], // dst=5, src=10, mutable=true
    );
    let release_instr = BytecodeInstruction::new(
        Opcode::Release,
        vec![5, 0], // src=5
    );
    // Act
    let module = build_and_decode(vec![borrow_instr, release_instr]);
    let instrs = &module.functions[0].instructions;
    // Assert
    assert_eq!(
        instrs.len(),
        2,
        "Function should contain exactly 2 instructions"
    );
    assert_borrow_instr(&instrs[0], Reg(5), Reg(10), true);
    assert_release_instr(&instrs[1], Reg(5));
}

// ========================
// MonoType::Ref -> IrType Conversion
// ========================

#[test]
fn test_ref_type_maps_to_void_ir_type() {
    // Arrange
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };
    // Act
    let ir_type: IrType = ref_ty.into();
    // Assert: Ref is ZST, should map to Void
    assert!(
        matches!(ir_type, IrType::Void),
        "Immutable Ref<i64> should map to IrType::Void (ZST has no runtime repr)"
    );
}

#[test]
fn test_ref_type_mutable_maps_to_void_ir_type() {
    // Arrange
    let ref_ty = MonoType::Ref {
        mutable: true,
        inner: Box::new(MonoType::String),
    };
    // Act
    let ir_type: IrType = ref_ty.into();
    // Assert: Ref is ZST regardless of mutability
    assert!(
        matches!(ir_type, IrType::Void),
        "Mutable Ref<String> should map to IrType::Void (ZST has no runtime repr)"
    );
}
