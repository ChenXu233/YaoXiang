//! GlobalMonomorphizer 测试

use crate::middle::core::ir::{BasicBlock, FunctionIR, ModuleIR};
use crate::middle::cross_module::CrossModuleMonomorphizer;
use crate::middle::ModuleId;
use crate::frontend::typecheck::{MonoType, TypeVar};

#[test]
fn test_register_module() {
    let mut mono = CrossModuleMonomorphizer::new();

    let id = mono.register_module(ModuleId::new(0), std::path::PathBuf::from("test.yx"));

    assert_eq!(mono.module_count(), 1);
    assert!(mono.get_module_state(id).is_some());
}

#[test]
fn test_instance_sharing() {
    let mut mono = CrossModuleMonomorphizer::new();

    // 注册模块
    let id = mono.register_module(ModuleId::new(0), std::path::PathBuf::from("test.yx"));

    // 创建泛型函数
    let generic_func = FunctionIR {
        name: "identity".to_string(),
        params: vec![MonoType::TypeVar(TypeVar::new(0))],
        return_type: MonoType::TypeVar(TypeVar::new(0)),
        locals: Vec::new(),
        blocks: vec![BasicBlock {
            label: 0,
            instructions: Vec::new(),
            successors: Vec::new(),
        }],
        is_async: false,
        entry: 0,
    };

    let ir = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![generic_func],
        mut_locals: Default::default(),
        loop_binding_locals: Default::default(),
        native_bindings: Vec::new(),
    };
    mono.collect_generics(id, &ir);

    // 实例化 Int 版本
    let func_int = mono.instantiate_function_global(id, "identity", vec![MonoType::Int(64)]);
    assert!(func_int.is_some());
    assert_eq!(mono.instance_count(), 1);

    // 实例化 String 版本
    let func_str = mono.instantiate_function_global(id, "identity", vec![MonoType::String]);
    assert!(func_str.is_some());
    assert_eq!(mono.instance_count(), 2); // 两个不同的实例
}
