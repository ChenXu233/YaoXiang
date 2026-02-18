//! 表达式代码生成测试

use crate::middle::passes::codegen::{CodegenContext, bytecode::BytecodeFile};
use crate::middle::core::ir::{ConstValue, FunctionIR, ModuleIR, Operand};
use crate::backends::common::Opcode;

/// 测试字面量生成
#[test]
fn test_literal_generation() {
    // 创建模块 IR
    let module = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![FunctionIR {
            name: "test_literal".to_string(),
            params: vec![],
            return_type: crate::frontend::typecheck::MonoType::Int(64),
            is_async: false,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: 0,
        }],
        mut_locals: Default::default(),
        loop_binding_locals: Default::default(),
        native_bindings: Vec::new(),
    };

    let mut ctx = CodegenContext::new(module);

    // 验证可以生成模块
    let result = ctx.generate();
    assert!(result.is_ok(), "Should generate bytecode successfully");
}

/// 测试变量加载
#[test]
fn test_variable_loading() {
    let module = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![FunctionIR {
            name: "test_var".to_string(),
            params: vec![],
            return_type: crate::frontend::typecheck::MonoType::Int(64),
            is_async: false,
            locals: vec![crate::frontend::typecheck::MonoType::Int(64)],
            blocks: Vec::new(),
            entry: 0,
        }],
        mut_locals: Default::default(),
        loop_binding_locals: Default::default(),
        native_bindings: Vec::new(),
    };

    let mut ctx = CodegenContext::new(module);
    let result = ctx.generate();
    assert!(result.is_ok());
}

/// 测试二元运算类型选择
#[test]
fn test_binop_type_selection() {
    // Test I64 binary operations
    assert_eq!(Opcode::I64Add.name(), "I64Add");
    assert_eq!(Opcode::F64Add.name(), "F64Add");
    assert_eq!(Opcode::I64Sub.name(), "I64Sub");
    assert_eq!(Opcode::F64Sub.name(), "F64Sub");
    assert_eq!(Opcode::I64Mul.name(), "I64Mul");
    assert_eq!(Opcode::F64Mul.name(), "F64Mul");

    // Test they are numeric operations
    assert!(Opcode::I64Add.is_numeric_op());
    assert!(Opcode::F64Add.is_numeric_op());
}

/// 测试比较指令
#[test]
fn test_comparison_opcodes() {
    // Test I64 comparison operations
    assert_eq!(Opcode::I64Eq.name(), "I64Eq");
    assert_eq!(Opcode::I64Ne.name(), "I64Ne");
    assert_eq!(Opcode::I64Lt.name(), "I64Lt");
    assert_eq!(Opcode::I64Le.name(), "I64Le");
    assert_eq!(Opcode::I64Gt.name(), "I64Gt");
    assert_eq!(Opcode::I64Ge.name(), "I64Ge");

    // Test F64 comparison operations
    assert_eq!(Opcode::F64Eq.name(), "F64Eq");
    assert_eq!(Opcode::F64Ne.name(), "F64Ne");
}

/// 测试操作数数量
#[test]
fn test_operand_counts() {
    // Test 0-operand instructions
    assert_eq!(Opcode::Nop.operand_count(), 0);
    assert_eq!(Opcode::Return.operand_count(), 0);

    // Test 1-operand instructions
    assert_eq!(Opcode::ReturnValue.operand_count(), 1);

    // Test 2-operand instructions
    assert_eq!(Opcode::Mov.operand_count(), 2);

    // Test 3-operand instructions
    assert_eq!(Opcode::I64Add.operand_count(), 3);
    assert_eq!(Opcode::F64Mul.operand_count(), 3);

    // Test 4-operand instructions
    assert_eq!(Opcode::LoadElement.operand_count(), 4);

    // Test 5-operand instructions (function calls)
    assert_eq!(Opcode::CallStatic.operand_count(), 5);
    assert_eq!(Opcode::CallVirt.operand_count(), 5);
    assert_eq!(Opcode::CallDyn.operand_count(), 5);
}

/// 测试字节码文件生成
#[test]
fn test_bytecode_file_generation() {
    let module = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![FunctionIR {
            name: "main".to_string(),
            params: vec![],
            return_type: crate::frontend::typecheck::MonoType::Int(64),
            is_async: false,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: 0,
        }],
        mut_locals: Default::default(),
        loop_binding_locals: Default::default(),
        native_bindings: Vec::new(),
    };

    let mut ctx = CodegenContext::new(module);
    let result = ctx.generate();

    assert!(result.is_ok());
    let bytecode = result.unwrap();

    // 验证文件头魔术数
    assert_eq!(bytecode.header.magic, 0x59584243);
    // 验证版本
    assert_eq!(bytecode.header.version, 2);
}

/// 测试寄存器分配
#[test]
fn test_register_allocation() {
    let module = ModuleIR::default();
    let mut ctx = CodegenContext::new(module);

    // 测试临时寄存器分配
    let temp1 = ctx.test_next_temp();
    let temp2 = ctx.test_next_temp();

    assert_ne!(temp1, temp2, "Should allocate different temp registers");
    assert!(temp1 < temp2, "Should allocate in increasing order");
}

/// 测试标签生成
#[test]
fn test_label_generation() {
    let module = ModuleIR::default();
    let mut ctx = CodegenContext::new(module);

    let label1 = ctx.next_label();
    let label2 = ctx.next_label();

    assert_ne!(label1, label2, "Should generate different labels");
    assert!(
        label1 < label2,
        "Should generate labels in increasing order"
    );
}

/// 测试常量池
#[test]
fn test_constant_pool() {
    let mut ctx = CodegenContext::new(ModuleIR::default());

    // 添加常量
    let idx1 = ctx.test_add_constant(ConstValue::Int(42));
    let idx2 = ctx.test_add_constant(ConstValue::Float(3.14));
    let idx3 = ctx.test_add_constant(ConstValue::String("hello".to_string()));

    assert_eq!(idx1, 0, "First constant should have index 0");
    assert_eq!(idx2, 1, "Second constant should have index 1");
    assert_eq!(idx3, 2, "Third constant should have index 2");
}

/// 测试操作数到寄存器的转换
#[test]
fn test_operand_to_reg() {
    use crate::middle::core::ir::Operand;

    let module = ModuleIR::default();
    let ctx = CodegenContext::new(module);

    // Local 操作数
    assert_eq!(ctx.test_operand_to_reg(&Operand::Local(5)).unwrap(), 5);
    // Temp 操作数
    assert_eq!(ctx.test_operand_to_reg(&Operand::Temp(10)).unwrap(), 10);
    // Arg 操作数
    assert_eq!(ctx.test_operand_to_reg(&Operand::Arg(3)).unwrap(), 3);
}
