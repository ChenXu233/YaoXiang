//! Extended code generation integration tests
//!
//! Tests for the code generation pipeline from high-level IR to bytecode.

use yaoxiang::middle::codegen::CodegenContext;
use yaoxiang::middle::ModuleIR;
use yaoxiang::middle::bytecode::{BytecodeInstr, Reg, Label};
use yaoxiang::backends::common::opcode;

#[test]
fn test_codegen_context_creation() {
    let module = ModuleIR::default();
    let ctx = CodegenContext::new(module);

    // Verify context can be created
    let _ = ctx;
}

#[test]
fn test_bytecode_instruction_sizes() {
    // Test that instruction size calculation is correct
    let nop = BytecodeInstr::Nop;
    assert_eq!(nop.size(), 1);

    let mov = BytecodeInstr::Mov {
        dst: Reg(0),
        src: Reg(1),
    };
    assert_eq!(mov.size(), 5);

    let return_val = BytecodeInstr::ReturnValue { value: Reg(0) };
    assert_eq!(return_val.size(), 3);

    let jmp = BytecodeInstr::Jmp { target: Label(0) };
    assert_eq!(jmp.size(), 5);
}

#[test]
fn test_bytecode_instruction_opcodes() {
    // Test that instructions have correct opcodes
    let nop = BytecodeInstr::Nop;
    assert_eq!(nop.opcode(), opcode::NOP);

    let ret = BytecodeInstr::Return;
    assert_eq!(ret.opcode(), opcode::RETURN);

    let mov = BytecodeInstr::Mov {
        dst: Reg(0),
        src: Reg(1),
    };
    assert_eq!(mov.opcode(), opcode::MOV);
}

#[test]
fn test_constant_pool_types() {
    use yaoxiang::middle::ConstValue;

    // Test various constant types
    let int_val = ConstValue::Int(42);
    let float_val = ConstValue::Float(std::f64::consts::PI);
    let string_val = ConstValue::String("test".to_string());
    let bool_val = ConstValue::Bool(true);

    // These should all be constructible
    assert_eq!(int_val, ConstValue::Int(42));
    assert_eq!(float_val, ConstValue::Float(std::f64::consts::PI));
    assert_eq!(string_val, ConstValue::String("test".to_string()));
    assert_eq!(bool_val, ConstValue::Bool(true));
}

#[test]
fn test_ir_types() {
    use yaoxiang::frontend::core::MonoType;

    // Arrange: 字节码/IR 的类型载体已是 MonoType（6.2 载体统一）
    let int_ty = MonoType::Int(64);
    let float_ty = MonoType::Float(64);
    let bool_ty = MonoType::Bool;
    let void_ty = MonoType::Void;
    let string_ty = MonoType::make_string();

    // Assert: 可构造且按值判等（MonoType 已 derive PartialEq），互不相等
    assert_eq!(int_ty, MonoType::Int(64), "Int(64) 应与同构字面量相等");
    assert_eq!(
        float_ty,
        MonoType::Float(64),
        "Float(64) 应与同构字面量相等"
    );
    assert_eq!(bool_ty, MonoType::Bool, "Bool 应与自身相等");
    assert_eq!(void_ty, MonoType::Void, "Void 应与自身相等");
    assert_eq!(
        string_ty,
        MonoType::make_string(),
        "make_string() 应产出相等的 Generic(String)"
    );
    assert_ne!(int_ty, void_ty, "Int(64) 不应等于 Void");
    assert_ne!(string_ty, int_ty, "String 不应等于 Int(64)");
}

#[test]
fn test_register_display() {
    let reg0 = Reg(0);
    let reg42 = Reg(42);

    assert_eq!(format!("{}", reg0), "r0");
    assert_eq!(format!("{}", reg42), "r42");
}

#[test]
fn test_label_display() {
    let label0 = Label(0);
    let label42 = Label(42);

    assert_eq!(format!("{}", label0), "L0");
    assert_eq!(format!("{}", label42), "L42");
}

#[test]
fn test_binary_operations() {
    use yaoxiang::middle::bytecode::BinaryOp;

    let add = BinaryOp::Add;
    let sub = BinaryOp::Sub;
    let mul = BinaryOp::Mul;
    let div = BinaryOp::Div;

    assert_eq!(add, BinaryOp::Add);
    assert_eq!(sub, BinaryOp::Sub);
    assert_eq!(mul, BinaryOp::Mul);
    assert_eq!(div, BinaryOp::Div);
}

#[test]
fn test_unary_operations() {
    use yaoxiang::middle::bytecode::UnaryOp;

    let neg = UnaryOp::Neg;
    let not = UnaryOp::Not;

    assert_eq!(neg, UnaryOp::Neg);
    assert_eq!(not, UnaryOp::Not);
}

#[test]
fn test_compare_operations() {
    use yaoxiang::middle::bytecode::CompareOp;

    let eq = CompareOp::Eq;
    let ne = CompareOp::Ne;
    let lt = CompareOp::Lt;
    let le = CompareOp::Le;
    let gt = CompareOp::Gt;
    let ge = CompareOp::Ge;

    assert_eq!(eq, CompareOp::Eq);
    assert_eq!(ne, CompareOp::Ne);
    assert_eq!(lt, CompareOp::Lt);
    assert_eq!(le, CompareOp::Le);
    assert_eq!(gt, CompareOp::Gt);
    assert_eq!(ge, CompareOp::Ge);
}
