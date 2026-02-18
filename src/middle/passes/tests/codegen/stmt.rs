//! 语句代码生成测试

use crate::middle::passes::codegen::CodegenContext;
use crate::middle::core::ir::{FunctionIR, ModuleIR};
use crate::backends::common::Opcode;

/// 测试函数定义生成
#[test]
fn test_function_definition() {
    let module = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![FunctionIR {
            name: "test_fn".to_string(),
            params: vec![
                crate::frontend::typecheck::MonoType::Int(64),
                crate::frontend::typecheck::MonoType::Float(64),
            ],
            return_type: crate::frontend::typecheck::MonoType::Int(64),
            is_async: false,
            locals: vec![crate::frontend::typecheck::MonoType::Int(64)],
            blocks: Vec::new(),
            entry: 0,
        }],
        mut_locals: Default::default(),
        loop_binding_locals: Default::default(),
        local_names: Default::default(),
        native_bindings: Vec::new(),
    };

    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let result = ctx.generate();

    assert!(result.is_ok());
}

/// 测试局部变量分配
#[test]
fn test_local_allocation() {
    let module = ModuleIR::default();
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);

    let local1 = ctx.test_next_local();
    let local2 = ctx.test_next_local();

    assert_ne!(local1, local2);
    assert!(local1 < local2);
}

/// 测试存储指令
#[test]
fn test_store_opcodes() {
    // Test store opcodes
    assert_eq!(Opcode::StoreLocal.name(), "StoreLocal");
    assert_eq!(Opcode::StoreElement.name(), "StoreElement");
    assert!(Opcode::StoreLocal.is_store_op());
    assert!(Opcode::StoreElement.is_store_op());
    assert!(!Opcode::Nop.is_store_op());
}

/// 测试加载指令
#[test]
fn test_load_opcodes() {
    // Test load opcodes
    assert_eq!(Opcode::LoadLocal.name(), "LoadLocal");
    assert_eq!(Opcode::LoadArg.name(), "LoadArg");
    assert_eq!(Opcode::LoadConst.name(), "LoadConst");
    assert!(Opcode::LoadLocal.is_load_op());
    assert!(Opcode::LoadArg.is_load_op());
    assert!(Opcode::LoadConst.is_load_op());
}

/// 测试内存分配指令
#[test]
fn test_memory_allocation_opcodes() {
    // Test memory allocation opcodes
    assert_eq!(Opcode::StackAlloc.name(), "StackAlloc");
    assert_eq!(Opcode::HeapAlloc.name(), "HeapAlloc");
    assert_eq!(Opcode::StackAlloc.operand_count(), 1);
    assert_eq!(Opcode::HeapAlloc.operand_count(), 2);
}

/// 测试返回指令
#[test]
fn test_return_opcodes() {
    // Test return opcodes
    assert!(Opcode::Return.is_return_op());
    assert!(Opcode::ReturnValue.is_return_op());
    assert!(Opcode::TailCall.is_return_op());
    assert!(!Opcode::Nop.is_return_op());

    assert_eq!(Opcode::Return.operand_count(), 0);
    assert_eq!(Opcode::ReturnValue.operand_count(), 1);
}

/// 测试参数处理
#[test]
fn test_parameter_handling() {
    let mut module = ModuleIR::default();

    // 添加带参数的函数
    module.functions.push(FunctionIR {
        name: "add".to_string(),
        params: vec![
            crate::frontend::typecheck::MonoType::Int(64),
            crate::frontend::typecheck::MonoType::Int(64),
        ],
        return_type: crate::frontend::typecheck::MonoType::Int(64),
        is_async: false,
        locals: Vec::new(),
        blocks: Vec::new(),
        entry: 0,
    });

    let mut ctx = CodegenContext::new(module);
    let result = ctx.generate();

    assert!(result.is_ok());
    let bytecode = result.unwrap();
    assert!(bytecode.code_section.functions.len() >= 1);

    let func = &bytecode.code_section.functions[0];
    assert_eq!(func.params.len(), 2);
}

/// 测试作用域级别
#[test]
fn test_scope_level() {
    let module = ModuleIR::default();
    let mut ctx = CodegenContext::new(module);

    assert_eq!(ctx.test_symbols().scope_level(), 0);

    ctx.test_symbols().push_scope();
    assert_eq!(ctx.test_symbols().scope_level(), 1);

    ctx.test_symbols().push_scope();
    assert_eq!(ctx.test_symbols().scope_level(), 2);
}

/// 测试函数索引
#[test]
fn test_function_indices() {
    let mut module = ModuleIR::default();

    module.functions.push(FunctionIR {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::frontend::typecheck::MonoType::Int(64),
        is_async: false,
        locals: Vec::new(),
        blocks: Vec::new(),
        entry: 0,
    });

    module.functions.push(FunctionIR {
        name: "helper".to_string(),
        params: vec![],
        return_type: crate::frontend::typecheck::MonoType::Void,
        is_async: false,
        locals: Vec::new(),
        blocks: Vec::new(),
        entry: 0,
    });

    let mut ctx = CodegenContext::new(module);

    // 检查函数索引
    let flow = ctx.test_flow();
    let main_idx = flow.function_indices().get("main");
    let helper_idx = flow.function_indices().get("helper");

    assert!(main_idx.is_some());
    assert!(helper_idx.is_some());
    assert_ne!(main_idx, helper_idx);
}

/// 测试位运算操作码
#[test]
fn test_bitwise_opcodes() {
    // Test I64 bitwise operations
    assert_eq!(Opcode::I64And.name(), "I64And");
    assert_eq!(Opcode::I64Or.name(), "I64Or");
    assert_eq!(Opcode::I64Xor.name(), "I64Xor");
    assert_eq!(Opcode::I64Shl.name(), "I64Shl");
    assert_eq!(Opcode::I64Shr.name(), "I64Shr");
    assert_eq!(Opcode::I64Sar.name(), "I64Sar");

    // Test I32 bitwise operations
    assert_eq!(Opcode::I32And.name(), "I32And");
    assert_eq!(Opcode::I32Or.name(), "I32Or");
    assert_eq!(Opcode::I32Xor.name(), "I32Xor");
    assert_eq!(Opcode::I32Shl.name(), "I32Shl");
    assert_eq!(Opcode::I32Shr.name(), "I32Shr");
    assert_eq!(Opcode::I32Sar.name(), "I32Sar");

    // Test they are integer operations
    assert!(Opcode::I64And.is_numeric_op());
    assert!(Opcode::I64Or.is_numeric_op());
    assert!(Opcode::I64Xor.is_numeric_op());
    assert!(Opcode::I64Shl.is_numeric_op());
    assert!(Opcode::I64Shr.is_numeric_op());
    assert!(Opcode::I64Sar.is_numeric_op());
}

/// 测试字符串操作码
#[test]
fn test_string_opcodes() {
    // Test string operations
    assert_eq!(Opcode::StringLength.name(), "StringLength");
    assert_eq!(Opcode::StringConcat.name(), "StringConcat");
    assert_eq!(Opcode::StringGetChar.name(), "StringGetChar");
    assert_eq!(Opcode::StringFromInt.name(), "StringFromInt");
    assert_eq!(Opcode::StringFromFloat.name(), "StringFromFloat");

    // Test operand counts
    assert_eq!(Opcode::StringLength.operand_count(), 2);
    assert_eq!(Opcode::StringConcat.operand_count(), 4);
    assert_eq!(Opcode::StringGetChar.operand_count(), 4);
    assert_eq!(Opcode::StringFromInt.operand_count(), 2);
    assert_eq!(Opcode::StringFromFloat.operand_count(), 2);
}

/// 测试闭包 Upvalue 操作码
#[test]
fn test_upvalue_opcodes() {
    // Test upvalue operations
    assert_eq!(Opcode::MakeClosure.name(), "MakeClosure");
    assert_eq!(Opcode::LoadUpvalue.name(), "LoadUpvalue");
    assert_eq!(Opcode::StoreUpvalue.name(), "StoreUpvalue");
    assert_eq!(Opcode::CloseUpvalue.name(), "CloseUpvalue");

    // Test operand counts
    assert_eq!(Opcode::MakeClosure.operand_count(), 4);
    assert_eq!(Opcode::LoadUpvalue.operand_count(), 2);
    assert_eq!(Opcode::StoreUpvalue.operand_count(), 2);
    assert_eq!(Opcode::CloseUpvalue.operand_count(), 1);
}
