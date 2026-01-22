//! 语句代码生成测试

use crate::middle::codegen::CodegenContext;
use crate::middle::ir::{FunctionIR, ModuleIR};
use crate::vm::opcode::TypedOpcode;

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
    };

    let mut ctx = crate::middle::codegen::CodegenContext::new(module);
    let result = ctx.generate();

    assert!(result.is_ok());
}

/// 测试局部变量分配
#[test]
fn test_local_allocation() {
    let module = ModuleIR::default();
    let mut ctx = crate::middle::codegen::CodegenContext::new(module);

    let local1 = ctx.next_local();
    let local2 = ctx.next_local();

    assert_ne!(local1, local2);
    assert!(local1 < local2);
}

/// 测试存储指令
#[test]
fn test_store_opcodes() {
    assert_eq!(TypedOpcode::StoreLocal.name(), "StoreLocal");
    assert_eq!(TypedOpcode::StoreElement.name(), "StoreElement");
    assert!(TypedOpcode::StoreLocal.is_store_op());
    assert!(TypedOpcode::StoreElement.is_store_op());
    assert!(!TypedOpcode::Nop.is_store_op());
}

/// 测试加载指令
#[test]
fn test_load_opcodes() {
    assert_eq!(TypedOpcode::LoadLocal.name(), "LoadLocal");
    assert_eq!(TypedOpcode::LoadArg.name(), "LoadArg");
    assert_eq!(TypedOpcode::LoadConst.name(), "LoadConst");
    assert!(TypedOpcode::LoadLocal.is_load_op());
    assert!(TypedOpcode::LoadArg.is_load_op());
    assert!(TypedOpcode::LoadConst.is_load_op());
}

/// 测试内存分配指令
#[test]
fn test_memory_allocation_opcodes() {
    assert_eq!(TypedOpcode::StackAlloc.name(), "StackAlloc");
    assert_eq!(TypedOpcode::HeapAlloc.name(), "HeapAlloc");
    assert_eq!(TypedOpcode::StackAlloc.operand_count(), 1);
    assert_eq!(TypedOpcode::HeapAlloc.operand_count(), 2);
}

/// 测试返回指令
#[test]
fn test_return_opcodes() {
    assert!(TypedOpcode::Return.is_return_op());
    assert!(TypedOpcode::ReturnValue.is_return_op());
    assert!(TypedOpcode::TailCall.is_return_op());
    assert!(!TypedOpcode::Nop.is_return_op());

    assert_eq!(TypedOpcode::Return.operand_count(), 0);
    assert_eq!(TypedOpcode::ReturnValue.operand_count(), 1);
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

    assert_eq!(ctx.symbols.scope_level(), 0);

    ctx.symbols.push_scope();
    assert_eq!(ctx.symbols.scope_level(), 1);

    ctx.symbols.push_scope();
    assert_eq!(ctx.symbols.scope_level(), 2);
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

    let ctx = CodegenContext::new(module);

    // 检查函数索引
    let main_idx = ctx.flow.function_indices().get("main");
    let helper_idx = ctx.flow.function_indices().get("helper");

    assert!(main_idx.is_some());
    assert!(helper_idx.is_some());
    assert_ne!(main_idx, helper_idx);
}

/// 测试位运算操作码
#[test]
fn test_bitwise_opcodes() {
    use crate::vm::opcode::TypedOpcode;

    // I64 位运算指令
    assert_eq!(TypedOpcode::I64And.name(), "I64And");
    assert_eq!(TypedOpcode::I64Or.name(), "I64Or");
    assert_eq!(TypedOpcode::I64Xor.name(), "I64Xor");
    assert_eq!(TypedOpcode::I64Shl.name(), "I64Shl");
    assert_eq!(TypedOpcode::I64Shr.name(), "I64Shr");
    assert_eq!(TypedOpcode::I64Sar.name(), "I64Sar");

    // I32 位运算指令
    assert_eq!(TypedOpcode::I32And.name(), "I32And");
    assert_eq!(TypedOpcode::I32Or.name(), "I32Or");
    assert_eq!(TypedOpcode::I32Xor.name(), "I32Xor");
    assert_eq!(TypedOpcode::I32Shl.name(), "I32Shl");
    assert_eq!(TypedOpcode::I32Shr.name(), "I32Shr");
    assert_eq!(TypedOpcode::I32Sar.name(), "I32Sar");

    // 操作数数量验证
    assert!(TypedOpcode::I64And.is_integer_op());
    assert!(TypedOpcode::I64Or.is_integer_op());
    assert!(TypedOpcode::I64Xor.is_integer_op());
    assert!(TypedOpcode::I64Shl.is_integer_op());
    assert!(TypedOpcode::I64Shr.is_integer_op());
    assert!(TypedOpcode::I64Sar.is_integer_op());
}

/// 测试字符串操作码
#[test]
fn test_string_opcodes() {
    use crate::vm::opcode::TypedOpcode;

    // 字符串指令
    assert_eq!(TypedOpcode::StringLength.name(), "StringLength");
    assert_eq!(TypedOpcode::StringConcat.name(), "StringConcat");
    assert_eq!(TypedOpcode::StringGetChar.name(), "StringGetChar");
    assert_eq!(TypedOpcode::StringFromInt.name(), "StringFromInt");
    assert_eq!(TypedOpcode::StringFromFloat.name(), "StringFromFloat");

    // 操作数数量验证
    assert_eq!(TypedOpcode::StringLength.operand_count(), 2);
    assert_eq!(TypedOpcode::StringConcat.operand_count(), 4);
    assert_eq!(TypedOpcode::StringGetChar.operand_count(), 4);
    assert_eq!(TypedOpcode::StringFromInt.operand_count(), 2);
    assert_eq!(TypedOpcode::StringFromFloat.operand_count(), 2);
}

/// 测试闭包 Upvalue 操作码
#[test]
fn test_upvalue_opcodes() {
    use crate::vm::opcode::TypedOpcode;

    // Upvalue 指令
    assert_eq!(TypedOpcode::MakeClosure.name(), "MakeClosure");
    assert_eq!(TypedOpcode::LoadUpvalue.name(), "LoadUpvalue");
    assert_eq!(TypedOpcode::StoreUpvalue.name(), "StoreUpvalue");
    assert_eq!(TypedOpcode::CloseUpvalue.name(), "CloseUpvalue");

    // 操作数数量验证
    assert_eq!(TypedOpcode::MakeClosure.operand_count(), 4);
    assert_eq!(TypedOpcode::LoadUpvalue.operand_count(), 2);
    assert_eq!(TypedOpcode::StoreUpvalue.operand_count(), 2);
    assert_eq!(TypedOpcode::CloseUpvalue.operand_count(), 1);
}
