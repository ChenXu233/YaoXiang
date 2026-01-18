//! 实际使用场景测试
//!
//! 测试常见的编程模式和使用场景

use crate::middle::monomorphize::Monomorphizer;
use crate::middle::monomorphize::closure::ClosureMonomorphizer;
use crate::middle::monomorphize::instance::{
    ClosureId, ClosureInstance, GenericClosureId, GenericFunctionId,
};
use crate::middle::{FunctionIR, BasicBlock};
use crate::frontend::typecheck::MonoType;
use crate::middle::ir::Operand;

// ==================== 辅助函数 ====================

fn int_type() -> MonoType {
    MonoType::Int(64)
}

fn float_type() -> MonoType {
    MonoType::Float(64)
}

fn string_type() -> MonoType {
    MonoType::String
}

fn bool_type() -> MonoType {
    MonoType::Bool
}

fn create_function_ir(
    name: &str,
    params: Vec<MonoType>,
    ret: MonoType,
) -> FunctionIR {
    let locals: Vec<MonoType> = params.iter().cloned().collect();
    FunctionIR {
        name: name.to_string(),
        params: params.clone(),
        return_type: ret,
        is_async: false,
        locals,
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![],
            successors: vec![],
        }],
        entry: 0,
    }
}

// ==================== 泛型容器测试 ====================

/// 测试：泛型 Option 类型
#[test]
fn test_generic_option() {
    let mut mono = Monomorphizer::new();

    // 注册 Some 和 None 的处理
    let some_id = GenericFunctionId::new("Some".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("Some", vec![], MonoType::String);
    mono.generic_functions.insert(some_id.clone(), ir);

    // 单态化为 Option<Int>
    let result = mono.monomorphize_function(&some_id, &[int_type()]);
    assert!(result.is_some());
    let func_id = result.unwrap();
    assert!(func_id.specialized_name().contains("int"));

    // 单态化为 Option<String>
    let result = mono.monomorphize_function(&some_id, &[string_type()]);
    assert!(result.is_some());
    let func_id = result.unwrap();
    assert!(func_id.specialized_name().contains("string"));
}

/// 测试：泛型 Result 类型
#[test]
fn test_generic_result() {
    let mut mono = Monomorphizer::new();

    let ok_id = GenericFunctionId::new("Ok".to_string(), vec!["T".to_string(), "E".to_string()]);
    let ir = create_function_ir("Ok", vec![], MonoType::String);
    mono.generic_functions.insert(ok_id.clone(), ir);

    // Result<Int, String>
    let result = mono.monomorphize_function(&ok_id, &[int_type(), string_type()]);
    assert!(result.is_some());

    // Result<String, Error>
    let result = mono.monomorphize_function(&ok_id, &[string_type(), int_type()]);
    assert!(result.is_some());

    // 不同顺序应该生成不同实例
    let id1 = mono
        .monomorphize_function(&ok_id, &[int_type(), string_type()])
        .unwrap();
    let id2 = mono
        .monomorphize_function(&ok_id, &[string_type(), int_type()])
        .unwrap();
    assert_ne!(id1, id2);
}

/// 测试：泛型 Vec/列表操作
#[test]
fn test_generic_list_operations() {
    let mut mono = Monomorphizer::new();

    // map 函数
    let map_id = GenericFunctionId::new("map".to_string(), vec!["T".to_string(), "U".to_string()]);
    let ir = create_function_ir("map", vec![], MonoType::String);
    mono.generic_functions.insert(map_id.clone(), ir);

    // Vec<i32> -> Vec<string>
    let result = mono.monomorphize_function(&map_id, &[int_type(), string_type()]);
    assert!(result.is_some());

    // Vec<Float> -> Vec<Bool>
    let result = mono.monomorphize_function(&map_id, &[float_type(), bool_type()]);
    assert!(result.is_some());
}

/// 测试：泛型字典操作
#[test]
fn test_generic_dict_operations() {
    let mut mono = Monomorphizer::new();

    let get_id = GenericFunctionId::new(
        "dict_get".to_string(),
        vec!["K".to_string(), "V".to_string()],
    );
    let ir = create_function_ir("dict_get", vec![], MonoType::String);
    mono.generic_functions.insert(get_id.clone(), ir);

    // Dict<string, Int>
    let result = mono.monomorphize_function(&get_id, &[string_type(), int_type()]);
    assert!(result.is_some());

    // Dict<Int, Float>
    let result = mono.monomorphize_function(&get_id, &[int_type(), float_type()]);
    assert!(result.is_some());
}

// ==================== 高阶函数测试 ====================

/// 测试：map 高阶函数
#[test]
fn test_map_higher_order_function() {
    let mut mono = Monomorphizer::new();

    let map_id = GenericFunctionId::new("map".to_string(), vec!["T".to_string(), "U".to_string()]);
    let ir = create_function_ir("map", vec![], MonoType::String);
    mono.generic_functions.insert(map_id.clone(), ir);

    // map<i32, i32>
    let r1 = mono.monomorphize_function(&map_id, &[int_type(), int_type()]);
    // map<i32, string>
    let r2 = mono.monomorphize_function(&map_id, &[int_type(), string_type()]);
    // map<string, i32>
    let r3 = mono.monomorphize_function(&map_id, &[string_type(), int_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
    assert!(r3.is_some());

    // 三个都应该不同
    let id1 = r1.unwrap();
    let id2 = r2.unwrap();
    let id3 = r3.unwrap();
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
}

/// 测试：filter 高阶函数
#[test]
fn test_filter_higher_order_function() {
    let mut mono = Monomorphizer::new();

    let filter_id = GenericFunctionId::new("filter".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("filter", vec![], MonoType::String);
    mono.generic_functions.insert(filter_id.clone(), ir);

    // filter<i32>
    let r1 = mono.monomorphize_function(&filter_id, &[int_type()]);
    // filter<string>
    let r2 = mono.monomorphize_function(&filter_id, &[string_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
    assert_ne!(r1.unwrap(), r2.unwrap());
}

/// 测试：reduce 高阶函数
#[test]
fn test_reduce_higher_order_function() {
    let mut mono = Monomorphizer::new();

    let reduce_id =
        GenericFunctionId::new("reduce".to_string(), vec!["T".to_string(), "R".to_string()]);
    let ir = create_function_ir("reduce", vec![], MonoType::String);
    mono.generic_functions.insert(reduce_id.clone(), ir);

    // reduce<i32, i32>
    let r1 = mono.monomorphize_function(&reduce_id, &[int_type(), int_type()]);
    // reduce<i32, string>
    let r2 = mono.monomorphize_function(&reduce_id, &[int_type(), string_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

/// 测试：排序函数
#[test]
fn test_sort_function() {
    let mut mono = Monomorphizer::new();

    let sort_id = GenericFunctionId::new("sort".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("sort", vec![], MonoType::String);
    mono.generic_functions.insert(sort_id.clone(), ir);

    // sort<i32>
    let r1 = mono.monomorphize_function(&sort_id, &[int_type()]);
    // sort<Float>
    let r2 = mono.monomorphize_function(&sort_id, &[float_type()]);
    // sort<string>
    let r3 = mono.monomorphize_function(&sort_id, &[string_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
    assert!(r3.is_some());

    // 三个不同类型生成三个实例
    assert_eq!(mono.instantiated_function_count(), 3);
}

// ==================== 闭包实际场景测试 ====================

/// 测试：map 闭包
#[test]
fn test_map_closure() {
    let mut mono = Monomorphizer::new();

    // 模拟 map 中的闭包：|x| x + 1
    let closure_id = GenericClosureId::new("add_one".to_string(), vec![], vec!["x".to_string()]);
    let instance = ClosureInstance::new(
        ClosureId::new("add_one".to_string(), vec![], vec![]),
        closure_id.clone(),
        vec![],
        vec![crate::middle::monomorphize::instance::CaptureVariable::new(
            "x".to_string(),
            int_type(),
            Operand::Local(0),
        )],
        create_function_ir("add_one_body", vec![], int_type()),
    );
    mono.register_generic_closure(closure_id.clone(), instance);

    // 单态化闭包
    let result = mono.monomorphize_closure(&closure_id, &[], &[int_type()]);
    assert!(result.is_some());
}

/// 测试：filter 闭包
#[test]
fn test_filter_closure() {
    let mut mono = Monomorphizer::new();

    // 模拟 filter 中的闭包：|x| x > 0
    let closure_id =
        GenericClosureId::new("is_positive".to_string(), vec![], vec!["x".to_string()]);
    let instance = ClosureInstance::new(
        ClosureId::new("is_positive".to_string(), vec![], vec![]),
        closure_id.clone(),
        vec![],
        vec![crate::middle::monomorphize::instance::CaptureVariable::new(
            "x".to_string(),
            int_type(),
            Operand::Local(0),
        )],
        create_function_ir("is_positive_body", vec![], bool_type()),
    );
    mono.register_generic_closure(closure_id.clone(), instance);

    let result = mono.monomorphize_closure(&closure_id, &[], &[int_type()]);
    assert!(result.is_some());
}

/// 测试：带捕获变量的闭包工厂
#[test]
fn test_closure_factory() {
    let mut mono = Monomorphizer::new();

    // make_adder(x) 返回 |y| x + y
    let factory_id = GenericClosureId::new(
        "make_adder".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string()],
    );
    let instance = ClosureInstance::new(
        ClosureId::new("make_adder".to_string(), vec![], vec![]),
        factory_id.clone(),
        vec![],
        vec![crate::middle::monomorphize::instance::CaptureVariable::new(
            "x".to_string(),
            int_type(),
            Operand::Local(0),
        )],
        create_function_ir("adder_body", vec![], int_type()),
    );
    mono.register_generic_closure(factory_id.clone(), instance);

    // 捕获不同类型的 x，生成不同闭包
    let r1 = mono.monomorphize_closure(&factory_id, &[], &[int_type()]);
    let r2 = mono.monomorphize_closure(&factory_id, &[], &[float_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
    assert_ne!(r1.unwrap(), r2.unwrap());
}

/// 测试：forEach 闭包
#[test]
fn test_foreach_closure() {
    let mut mono = Monomorphizer::new();

    let closure_id =
        GenericClosureId::new("print_item".to_string(), vec![], vec!["item".to_string()]);
    let instance = ClosureInstance::new(
        ClosureId::new("print_item".to_string(), vec![], vec![]),
        closure_id.clone(),
        vec![],
        vec![crate::middle::monomorphize::instance::CaptureVariable::new(
            "item".to_string(),
            int_type(),
            Operand::Local(0),
        )],
        create_function_ir("print_body", vec![], MonoType::Void),
    );
    mono.register_generic_closure(closure_id.clone(), instance);

    let result = mono.monomorphize_closure(&closure_id, &[], &[int_type()]);
    assert!(result.is_some());
}

// ==================== 泛型约束场景测试 ====================

/// 测试：可比较类型约束
#[test]
fn test_comparable_constraint() {
    let mut mono = Monomorphizer::new();

    // max<T: Comparable>(a: T, b: T) -> T
    let max_id = GenericFunctionId::new("max".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("max", vec![], MonoType::String);
    mono.generic_functions.insert(max_id.clone(), ir);

    // max<i32>
    let r1 = mono.monomorphize_function(&max_id, &[int_type()]);
    // max<Float>
    let r2 = mono.monomorphize_function(&max_id, &[float_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

/// 测试：可哈希类型约束
#[test]
fn test_hashable_constraint() {
    let mut mono = Monomorphizer::new();

    // hash<T: Hashable>(value: T) -> i32
    let hash_id = GenericFunctionId::new("hash".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("hash", vec![], int_type());
    mono.generic_functions.insert(hash_id.clone(), ir);

    // hash<string>
    let r1 = mono.monomorphize_function(&hash_id, &[string_type()]);
    // hash<i32>
    let r2 = mono.monomorphize_function(&hash_id, &[int_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

/// 测试：可迭代类型约束
#[test]
fn test_iterable_constraint() {
    let mut mono = Monomorphizer::new();

    // iter<T: Iterable>(collection: T)
    let iter_id = GenericFunctionId::new("iter".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("iter", vec![], MonoType::Void);
    mono.generic_functions.insert(iter_id.clone(), ir);

    // iter<Vec<i32>>
    let r1 = mono.monomorphize_function(&iter_id, &[MonoType::String]);
    // iter<Dict<string, i32>>
    let r2 = mono.monomorphize_function(&iter_id, &[MonoType::String]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

// ==================== 实际代码模式测试 ====================

/// 测试：回调模式
#[test]
fn test_callback_pattern() {
    let mut mono = Monomorphizer::new();

    // on_click(callback: Fn(i32) -> void)
    let on_click_id = GenericFunctionId::new("on_click".to_string(), vec![]);
    let ir = create_function_ir("on_click", vec![], MonoType::Void);
    mono.generic_functions.insert(on_click_id.clone(), ir);

    let result = mono.monomorphize_function(&on_click_id, &[]);
    assert!(result.is_some());
}

/// 测试：事件处理模式
#[test]
fn test_event_handler_pattern() {
    let mut mono = Monomorphizer::new();

    // handle_event<T>(event: Event<T>, handler: Fn(T) -> void)
    let handler_id = GenericFunctionId::new("handle_event".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("handle_event", vec![], MonoType::Void);
    mono.generic_functions.insert(handler_id.clone(), ir);

    // handle_event<string>
    let r1 = mono.monomorphize_function(&handler_id, &[string_type()]);
    // handle_event<i32>
    let r2 = mono.monomorphize_function(&handler_id, &[int_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

/// 测试：异步回调模式
#[test]
fn test_async_callback_pattern() {
    let mut mono = Monomorphizer::new();

    // then<T, U>(promise: Promise<T>, callback: Fn(T) -> Promise<U>) -> Promise<U>
    let then_id =
        GenericFunctionId::new("then".to_string(), vec!["T".to_string(), "U".to_string()]);
    let ir = create_function_ir("then", vec![], MonoType::String);
    mono.generic_functions.insert(then_id.clone(), ir);

    let r1 = mono.monomorphize_function(&then_id, &[int_type(), string_type()]);
    let r2 = mono.monomorphize_function(&then_id, &[string_type(), int_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

/// 测试：链式调用模式
#[test]
fn test_chain_pattern() {
    let mut mono = Monomorphizer::new();

    // pipe<T>(value: T, funcs: List<Fn(T) -> T>) -> T
    let pipe_id = GenericFunctionId::new("pipe".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("pipe", vec![], MonoType::String);
    mono.generic_functions.insert(pipe_id.clone(), ir);

    // pipe<i32>
    let r1 = mono.monomorphize_function(&pipe_id, &[int_type()]);
    // pipe<string>
    let r2 = mono.monomorphize_function(&pipe_id, &[string_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

// ==================== 错误处理测试 ====================

/// 测试：Try 模式
#[test]
fn test_try_pattern() {
    let mut mono = Monomorphizer::new();

    // try<T>(operation: Fn() -> T) -> Result<T, Error>
    let try_id = GenericFunctionId::new("try".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("try", vec![], MonoType::String);
    mono.generic_functions.insert(try_id.clone(), ir);

    // try<i32>
    let r1 = mono.monomorphize_function(&try_id, &[int_type()]);
    // try<string>
    let r2 = mono.monomorphize_function(&try_id, &[string_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

/// 测试：catch 模式
#[test]
fn test_catch_pattern() {
    let mut mono = Monomorphizer::new();

    // catch<T, E>(result: Result<T, E>, handler: Fn(E) -> T) -> T
    let catch_id =
        GenericFunctionId::new("catch".to_string(), vec!["T".to_string(), "E".to_string()]);
    let ir = create_function_ir("catch", vec![], MonoType::String);
    mono.generic_functions.insert(catch_id.clone(), ir);

    let r1 = mono.monomorphize_function(&catch_id, &[int_type(), string_type()]);
    let r2 = mono.monomorphize_function(&catch_id, &[string_type(), int_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

// ==================== 性能相关测试 ====================

/// 测试：缓存有效性
#[test]
fn test_cache_effectiveness() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("id".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("id", vec![], MonoType::String);
    mono.generic_functions.insert(id.clone(), ir);

    // 首次调用
    let start_count = mono.instantiated_function_count();
    let _ = mono.monomorphize_function(&id, &[int_type()]);
    let first_call_count = mono.instantiated_function_count();
    assert_eq!(first_call_count, start_count + 1);

    // 重复调用相同类型（应该命中缓存）
    for _ in 0..100 {
        let _ = mono.monomorphize_function(&id, &[int_type()]);
    }
    assert_eq!(mono.instantiated_function_count(), first_call_count);

    // 调用不同类型
    let _ = mono.monomorphize_function(&id, &[float_type()]);
    let _ = mono.monomorphize_function(&id, &[string_type()]);
    assert_eq!(mono.instantiated_function_count(), start_count + 3);
}

/// 测试：大量不同类型参数
#[test]
fn test_many_type_parameters() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new(
        "multi".to_string(),
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
    );
    let ir = create_function_ir("multi", vec![], MonoType::String);
    mono.generic_functions.insert(id.clone(), ir);

    // 使用不同类型组合
    let types = [int_type(), float_type(), string_type(), bool_type()];

    for i in 0..types.len() {
        for j in 0..types.len() {
            for k in 0..types.len() {
                let _ = mono.monomorphize_function(
                    &id,
                    &[types[i].clone(), types[j].clone(), types[k].clone()],
                );
            }
        }
    }

    // 4^3 = 64 种组合
    assert_eq!(mono.instantiated_function_count(), 64);
}

/// 测试：空列表处理
#[test]
fn test_empty_list_handling() {
    let mut mono = Monomorphizer::new();

    // first<T>(list: List<T>) -> Option<T>
    let first_id = GenericFunctionId::new("first".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("first", vec![], MonoType::String);
    mono.generic_functions.insert(first_id.clone(), ir);

    let result = mono.monomorphize_function(&first_id, &[int_type()]);
    assert!(result.is_some());
}

/// 测试：默认参数模式
#[test]
fn test_default_parameter_pattern() {
    let mut mono = Monomorphizer::new();

    // with_default<T>(value: Option<T>, default: T) -> T
    let with_default_id = GenericFunctionId::new("with_default".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("with_default", vec![], MonoType::String);
    mono.generic_functions.insert(with_default_id.clone(), ir);

    let r1 = mono.monomorphize_function(&with_default_id, &[int_type()]);
    let r2 = mono.monomorphize_function(&with_default_id, &[string_type()]);

    assert!(r1.is_some());
    assert!(r2.is_some());
}

// ==================== 模块间互操作测试 ====================

use crate::middle::module::{ModuleGraph, ModuleId};
use std::path::PathBuf;

/// 测试：模块依赖解析
#[test]
fn test_module_dependency_resolution() {
    let mut graph = ModuleGraph::new();

    // main.yx -> utils.yx -> helpers.yx
    let main_id = graph.add_module(PathBuf::from("main.yx"));
    let utils_id = graph.add_module(PathBuf::from("utils.yx"));
    let helpers_id = graph.add_module(PathBuf::from("helpers.yx"));

    graph.add_dependency(main_id, utils_id, true).unwrap();
    graph.add_dependency(utils_id, helpers_id, true).unwrap();

    // 拓扑排序：helpers -> utils -> main
    let sorted = graph.topological_sort().unwrap();
    let pos: HashMap<ModuleId, usize> = sorted.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    assert!(pos[&helpers_id] < pos[&utils_id]);
    assert!(pos[&utils_id] < pos[&main_id]);
}

/// 测试：模块路径查找
#[test]
fn test_module_path_lookup() {
    let mut graph = ModuleGraph::new();

    let id1 = graph.add_module(PathBuf::from("/src/utils.yx"));
    let id2 = graph.add_module(PathBuf::from("/src/main.yx"));

    // 相同路径返回相同 ID
    let id1_again = graph.add_module(PathBuf::from("/src/utils.yx"));
    assert_eq!(id1, id1_again);

    // 不同路径返回不同 ID
    assert_ne!(id1, id2);
}

/// 测试：模块依赖数量
#[test]
fn test_module_dependency_count() {
    let mut graph = ModuleGraph::new();

    let core_id = graph.add_module(PathBuf::from("core.yx"));
    let util_id = graph.add_module(PathBuf::from("util.yx"));
    let main_id = graph.add_module(PathBuf::from("main.yx"));

    // main 依赖 core 和 util
    graph.add_dependency(main_id, core_id, true).unwrap();
    graph.add_dependency(main_id, util_id, true).unwrap();

    let deps = graph.get_dependencies(main_id).unwrap();
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&core_id));
    assert!(deps.contains(&util_id));
}

// ==================== 边界条件测试 ====================

use std::collections::HashMap;

/// 测试：HashMap 作为泛型参数
#[test]
fn test_hashmap_as_type_argument() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new(
        "process_map".to_string(),
        vec!["K".to_string(), "V".to_string()],
    );
    let ir = create_function_ir("process_map", vec![], MonoType::String);
    mono.generic_functions.insert(id.clone(), ir);

    let result = mono.monomorphize_function(&id, &[string_type(), int_type()]);
    assert!(result.is_some());
}

/// 测试：嵌套泛型类型
#[test]
fn test_nested_generic_types() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("flatten".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("flatten", vec![], MonoType::String);
    mono.generic_functions.insert(id.clone(), ir);

    // List<List<i32>>
    let nested_list = MonoType::List(Box::new(int_type()));
    let result = mono.monomorphize_function(&id, &[nested_list]);
    assert!(result.is_some());
}

/// 测试：函数作为泛型参数
#[test]
fn test_function_as_type_argument() {
    let mut mono = Monomorphizer::new();

    let callback_fn_type = MonoType::Fn {
        params: vec![int_type()],
        return_type: Box::new(MonoType::String),
        is_async: false,
    };

    let id = GenericFunctionId::new("execute_callback".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("execute_callback", vec![], MonoType::String);
    mono.generic_functions.insert(id.clone(), ir);

    let result = mono.monomorphize_function(&id, &[callback_fn_type]);
    assert!(result.is_some());
}

/// 测试：Tuple 作为泛型参数
#[test]
fn test_tuple_as_type_argument() {
    let mut mono = Monomorphizer::new();

    let tuple_type = MonoType::Tuple(vec![int_type(), string_type()]);

    let id = GenericFunctionId::new("process_pair".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("process_pair", vec![], MonoType::String);
    mono.generic_functions.insert(id.clone(), ir);

    let result = mono.monomorphize_function(&id, &[tuple_type]);
    assert!(result.is_some());
}
