//! 跨模块单态化测试
//!
//! 测试跨模块泛型实例化的核心功能

use crate::frontend::typecheck::{MonoType, TypeVar};
use crate::middle::core::ir::{BasicBlock, FunctionIR};
use crate::middle::passes::module::{ModuleGraph, ModuleId};
use crate::middle::cross_module::{CrossModuleMonomorphizer, GlobalInstanceKey};
use crate::middle::passes::mono::module_state::ModuleMonoState;
use crate::middle::ModuleIR;
use std::collections::HashMap;

fn create_dummy_function(
    name: &str,
    params: Vec<MonoType>,
    return_type: MonoType,
) -> FunctionIR {
    FunctionIR {
        name: name.to_string(),
        params,
        return_type,
        locals: Vec::new(),
        blocks: vec![BasicBlock {
            label: 0,
            instructions: Vec::new(),
            successors: Vec::new(),
        }],
        is_async: false,
        entry: 0,
    }
}

#[test]
fn test_module_registration() {
    let mut mono = CrossModuleMonomorphizer::new();

    // 注册模块
    let id_a = mono.register_module(ModuleId::new(0), std::path::PathBuf::from("module_a.yx"));
    let id_b = mono.register_module(ModuleId::new(1), std::path::PathBuf::from("module_b.yx"));

    assert_eq!(mono.module_count(), 2);
    assert!(mono.get_module_state(id_a).is_some());
    assert!(mono.get_module_state(id_b).is_some());
}

#[test]
fn test_collect_generic_functions() {
    let mut mono = CrossModuleMonomorphizer::new();

    let id = mono.register_module(ModuleId::new(0), std::path::PathBuf::from("test.yx"));

    // 创建泛型函数
    let generic_func = create_dummy_function(
        "identity",
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
    );

    // 创建非泛型函数
    let non_generic = create_dummy_function(
        "add",
        vec![MonoType::Int(64), MonoType::Int(64)],
        MonoType::Int(64),
    );

    let ir = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![generic_func, non_generic],
        mut_locals: Default::default(),
    };

    mono.collect_generics(id, &ir);

    let state = mono.get_module_state(id).unwrap();
    assert_eq!(state.generic_function_count(), 1); // 只有 identity 是泛型
}

#[test]
fn test_global_function_instantiation() {
    let mut mono = CrossModuleMonomorphizer::new();

    let id = mono.register_module(ModuleId::new(0), std::path::PathBuf::from("test.yx"));

    // 创建泛型函数
    let generic_func = create_dummy_function(
        "identity",
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
    );

    let ir = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![generic_func],
        mut_locals: Default::default(),
    };

    mono.collect_generics(id, &ir);

    // 实例化 Int 版本
    let func_int = mono.instantiate_function_global(id, "identity", vec![MonoType::Int(64)]);

    assert!(func_int.is_some());
    let inst = func_int.unwrap();

    // 验证参数类型已替换
    assert_eq!(inst.params.len(), 1);
    assert_eq!(inst.params[0], MonoType::Int(64));
    assert_eq!(inst.return_type, MonoType::Int(64));

    // 验证名称已特化
    assert!(inst.name.contains("identity"));
}

#[test]
fn test_instance_sharing() {
    let mut mono = CrossModuleMonomorphizer::new();

    let id = mono.register_module(ModuleId::new(0), std::path::PathBuf::from("test.yx"));

    let generic_func = create_dummy_function(
        "identity",
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
    );

    let ir = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![generic_func],
        mut_locals: Default::default(),
    };

    mono.collect_generics(id, &ir);

    // 首次实例化
    let func1 = mono.instantiate_function_global(id, "identity", vec![MonoType::Int(64)]);
    assert!(func1.is_some());
    assert_eq!(mono.instance_count(), 1);

    // 相同类型再次实例化（应该命中缓存）
    let func2 = mono.instantiate_function_global(id, "identity", vec![MonoType::Int(64)]);
    assert!(func2.is_some());
    assert_eq!(mono.instance_count(), 1); // 不变
}

#[test]
fn test_different_types_different_instances() {
    let mut mono = CrossModuleMonomorphizer::new();

    let id = mono.register_module(ModuleId::new(0), std::path::PathBuf::from("test.yx"));

    let generic_func = create_dummy_function(
        "identity",
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
    );

    let ir = ModuleIR {
        types: Vec::new(),
        globals: Vec::new(),
        functions: vec![generic_func],
        mut_locals: Default::default(),
    };

    mono.collect_generics(id, &ir);

    // 实例化 Int 版本
    let func_int = mono.instantiate_function_global(id, "identity", vec![MonoType::Int(64)]);
    assert!(func_int.is_some());
    assert_eq!(mono.instance_count(), 1);

    // 实例化 String 版本（不同类型）
    let func_str = mono.instantiate_function_global(id, "identity", vec![MonoType::String]);
    assert!(func_str.is_some());
    assert_eq!(mono.instance_count(), 2); // 新增实例
}

#[test]
fn test_multiple_type_parameters() {
    let mut mono = CrossModuleMonomorphizer::new();

    let id = mono.register_module(ModuleId::new(0), std::path::PathBuf::from("test.yx"));

    // 创建双类型参数的泛型函数
    let generic_func = FunctionIR {
        name: "pair".to_string(),
        params: vec![
            MonoType::TypeVar(TypeVar::new(0)),
            MonoType::TypeVar(TypeVar::new(1)),
        ],
        return_type: MonoType::Tuple(vec![
            MonoType::TypeVar(TypeVar::new(0)),
            MonoType::TypeVar(TypeVar::new(1)),
        ]),
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
    };

    mono.collect_generics(id, &ir);

    // 实例化 (Int, String) 版本
    let func =
        mono.instantiate_function_global(id, "pair", vec![MonoType::Int(64), MonoType::String]);

    assert!(func.is_some());
    let inst = func.unwrap();

    // 验证两个类型参数都已替换
    assert_eq!(inst.params.len(), 2);
    assert_eq!(inst.params[0], MonoType::Int(64));
    assert_eq!(inst.params[1], MonoType::String);

    if let MonoType::Tuple(types) = &inst.return_type {
        assert_eq!(types.len(), 2);
        assert_eq!(types[0], MonoType::Int(64));
        assert_eq!(types[1], MonoType::String);
    } else {
        panic!("Expected tuple return type");
    }
}

#[test]
fn test_module_graph() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(std::path::PathBuf::from("a.yx"));
    let id_b = graph.add_module(std::path::PathBuf::from("b.yx"));
    let id_c = graph.add_module(std::path::PathBuf::from("c.yx"));

    // a 依赖 b，b 依赖 c
    graph.add_dependency(id_a, id_b, true).unwrap();
    graph.add_dependency(id_b, id_c, true).unwrap();

    // 拓扑排序：c -> b -> a
    // 验证排序后的依赖关系：每个模块应该在其依赖之后
    let sorted = graph.topological_sort().unwrap();
    assert_eq!(sorted.len(), 3);

    // 创建索引映射
    let pos: HashMap<ModuleId, usize> = sorted.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    // a 依赖 b，所以 a 应该在 b 后面（索引更大）
    assert!(pos[&id_a] > pos[&id_b], "a 应该在 b 后面");
    // b 依赖 c，所以 b 应该在 c 后面（索引更大）
    assert!(pos[&id_b] > pos[&id_c], "b 应该在 c 后面");
}

#[test]
fn test_module_state_creation() {
    let state = ModuleMonoState::new(ModuleId::new(0), "test_module".to_string());

    assert_eq!(state.module_id, ModuleId::new(0));
    assert_eq!(state.module_name, "test_module");
    assert_eq!(state.generic_function_count(), 0);
    assert_eq!(state.generic_type_count(), 0);
}

#[test]
fn test_global_instance_key() {
    let key1 = GlobalInstanceKey::new("foo".to_string(), vec![MonoType::Int(64)]);
    let key2 = GlobalInstanceKey::new("foo".to_string(), vec![MonoType::Int(64)]);
    let key3 = GlobalInstanceKey::new("foo".to_string(), vec![MonoType::String]);
    let key4 = GlobalInstanceKey::new("bar".to_string(), vec![MonoType::Int(64)]);

    assert_eq!(key1, key2); // 相同
    assert_ne!(key1, key3); // 不同类型参数
    assert_ne!(key1, key4); // 不同名称
}
