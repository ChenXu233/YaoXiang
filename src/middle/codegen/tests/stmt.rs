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

    assert_eq!(ctx.scope_level, 0);

    ctx.scope_level += 1;
    assert_eq!(ctx.scope_level, 1);

    ctx.scope_level += 1;
    assert_eq!(ctx.scope_level, 2);
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
    let main_idx = ctx.function_indices.get("main");
    let helper_idx = ctx.function_indices.get("helper");

    assert!(main_idx.is_some());
    assert!(helper_idx.is_some());
    assert_ne!(main_idx, helper_idx);
}
