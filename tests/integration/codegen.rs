#![allow(unused_imports)]
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
use yaoxiang::middle::codegen::CodegenContext;
use yaoxiang::middle::ir::ModuleIR;

#[test]
fn test_bytecode_serialization() {
    let module = ModuleIR::default();
    let bytecode_file = BytecodeFile::from_ir(&module);

    let mut buffer = Vec::new();
    bytecode_file
        .write_to(&mut buffer)
        .expect("Serialization failed");

    assert!(buffer.len() > 0);
    // Magic number check - Big Endian: 0x59584243 = 'Y' 'X' 'B' 'C'
    assert_eq!(buffer[0], 0x59); // Y
    assert_eq!(buffer[1], 0x58); // X
    assert_eq!(buffer[2], 0x42); // B
    assert_eq!(buffer[3], 0x43); // C
}

#[test]
fn test_switch_generation() {
    // TODO: Construct AST for switch and test generation
}

#[test]
fn test_loop_optimization() {
    // TODO: Construct AST for range loop and test optimization
}
