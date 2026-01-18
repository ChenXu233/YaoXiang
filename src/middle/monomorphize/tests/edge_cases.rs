//! 边缘情况与高级场景测试
//!
//! 测试各种边界情况、错误处理和复杂场景

use crate::middle::monomorphize::Monomorphizer;
use crate::middle::monomorphize::closure::ClosureMonomorphizer;
use crate::middle::monomorphize::type_mono::TypeMonomorphizer;
use crate::middle::monomorphize::instance::{
    ClosureId, ClosureInstance, GenericClosureId, GenericFunctionId, FunctionId, FunctionInstance,
    TypeId, GenericTypeId,
};
use crate::middle::{FunctionIR, BasicBlock};
use crate::frontend::typecheck::MonoType;
use crate::middle::ir::Operand;
use crate::middle::module::{ModuleGraph, ModuleId};
use std::path::PathBuf;

/// 创建整数的 MonoType
fn int_type() -> MonoType {
    MonoType::Int(64)
}

/// 创建浮点数的 MonoType
fn float_type() -> MonoType {
    MonoType::Float(64)
}

/// 创建字符串类型的 MonoType
fn string_type() -> MonoType {
    MonoType::String
}

/// 创建布尔类型的 MonoType
fn bool_type() -> MonoType {
    MonoType::Bool
}

/// 创建 Void 类型的 MonoType
fn void_type() -> MonoType {
    MonoType::Void
}

/// 创建 FunctionIR 的辅助函数
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

// ==================== 实例缓存测试 ====================

/// 测试：同名不同参数的泛型函数应该生成不同实例
#[test]
fn test_same_name_different_params() {
    let mut mono = Monomorphizer::new();

    // 创建一个单参数泛型函数
    let id1 = GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]);
    let ir1 = create_function_ir("identity", vec![], int_type());
    mono.generic_functions.insert(id1.clone(), ir1);

    // 单态化为 Int
    let result1 = mono.monomorphize_function(&id1, &[int_type()]);
    assert!(result1.is_some());
    let id1_inst = result1.unwrap();

    // 验证名称包含类型信息
    assert!(id1_inst.specialized_name().contains("int"));
}

/// 测试：大量单态化请求的缓存命中
#[test]
fn test_mass_instantiation_cache_hit() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("id".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("id", vec![], int_type());
    mono.generic_functions.insert(id.clone(), ir);

    // 大量单态化请求（应该全部命中缓存）
    for _ in 0..100 {
        let result = mono.monomorphize_function(&id, &[int_type()]);
        assert!(result.is_some());
    }

    // 应该只有一个实例（缓存命中）
    assert_eq!(mono.instantiated_function_count(), 1);
}

/// 测试：泛型函数递归调用单态化
#[test]
fn test_recursive_monomorphization() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("factorial".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("factorial", vec![], int_type());
    mono.generic_functions.insert(id.clone(), ir);

    // 多次调用（递归模拟）
    for _ in 0..5 {
        let _ = mono.monomorphize_function(&id, &[int_type()]);
    }

    // 应该只有一个实例
    assert_eq!(mono.instantiated_function_count(), 1);
}

// ==================== 闭包高级测试 ====================

/// 测试：嵌套闭包单态化
#[test]
fn test_nested_closure() {
    let mut mono = Monomorphizer::new();

    // 外层闭包
    let outer_id = GenericClosureId::new("outer".to_string(), vec![], vec!["x".to_string()]);
    let outer_instance = ClosureInstance::new(
        ClosureId::new("outer".to_string(), vec![], vec![]),
        outer_id.clone(),
        vec![],
        vec![],
        create_function_ir("outer_body", vec![], int_type()),
    );
    mono.register_generic_closure(outer_id.clone(), outer_instance);

    // 内层闭包
    let inner_id = GenericClosureId::new("inner".to_string(), vec![], vec!["y".to_string()]);
    let inner_instance = ClosureInstance::new(
        ClosureId::new("inner".to_string(), vec![], vec![]),
        inner_id.clone(),
        vec![],
        vec![],
        create_function_ir("inner_body", vec![], int_type()),
    );
    mono.register_generic_closure(inner_id.clone(), inner_instance);

    // 单态化两个闭包
    let outer = mono.monomorphize_closure(&outer_id, &[], &[int_type()]);
    let inner = mono.monomorphize_closure(&inner_id, &[], &[float_type()]);

    assert!(outer.is_some());
    assert!(inner.is_some());
    assert_ne!(outer.unwrap(), inner.unwrap());
}

/// 测试：闭包作为参数传递（缓存测试）
#[test]
fn test_closure_as_fn_param() {
    let mut mono = Monomorphizer::new();

    let closure_id = GenericClosureId::new("callback".to_string(), vec![], vec!["cb".to_string()]);
    let closure_instance = ClosureInstance::new(
        ClosureId::new("callback".to_string(), vec![], vec![]),
        closure_id.clone(),
        vec![],
        vec![],
        create_function_ir("callback_body", vec![], int_type()),
    );
    mono.register_generic_closure(closure_id.clone(), closure_instance);

    // 单态化多次
    for _ in 0..5 {
        let _ = mono.monomorphize_closure(&closure_id, &[], &[int_type()]);
    }

    assert_eq!(mono.instantiated_closure_count(), 1);
}

/// 测试：闭包捕获多个不同类型
#[test]
fn test_closure_multiple_different_types() {
    let mut mono = Monomorphizer::new();

    let id = GenericClosureId::new(
        "multi_capture".to_string(),
        vec![],
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
    );

    let capture_vars = vec![
        crate::middle::monomorphize::instance::CaptureVariable::new(
            "a".to_string(),
            int_type(),
            Operand::Local(0),
        ),
        crate::middle::monomorphize::instance::CaptureVariable::new(
            "b".to_string(),
            float_type(),
            Operand::Local(1),
        ),
        crate::middle::monomorphize::instance::CaptureVariable::new(
            "c".to_string(),
            string_type(),
            Operand::Local(2),
        ),
    ];

    let instance = ClosureInstance::new(
        ClosureId::new("multi_capture".to_string(), vec![], vec![]),
        id.clone(),
        vec![],
        capture_vars,
        create_function_ir("body", vec![], int_type()),
    );

    mono.register_generic_closure(id.clone(), instance);

    let result = mono.monomorphize_closure(&id, &[], &[int_type(), float_type(), string_type()]);
    assert!(result.is_some());

    let instantiated = mono.get_instantiated_closure(&result.unwrap()).unwrap();
    assert_eq!(instantiated.capture_vars.len(), 3);
}

// ==================== 跨模块高级测试 ====================

/// 测试：模块依赖循环检测
#[test]
fn test_cycle_detection() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(PathBuf::from("a.yx"));
    let id_b = graph.add_module(PathBuf::from("b.yx"));
    let id_c = graph.add_module(PathBuf::from("c.yx"));

    // a -> b -> c -> a
    graph.add_dependency(id_a, id_b, true).unwrap();
    graph.add_dependency(id_b, id_c, true).unwrap();
    graph.add_dependency(id_c, id_a, true).unwrap();

    let result = graph.topological_sort();
    assert!(result.is_err());
}

/// 测试：自依赖检测
#[test]
fn test_self_dependency() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(PathBuf::from("a.yx"));
    let result = graph.add_dependency(id_a, id_a, true);

    assert!(result.is_err());
}

/// 测试：模块图获取公开依赖
#[test]
fn test_public_dependency_closure() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(PathBuf::from("a.yx"));
    let id_b = graph.add_module(PathBuf::from("b.yx"));
    let id_c = graph.add_module(PathBuf::from("c.yx"));
    let id_d = graph.add_module(PathBuf::from("d.yx"));

    // a -> b (公开), b -> c (公开), c -> d (公开)
    graph.add_dependency(id_a, id_b, true).unwrap();
    graph.add_dependency(id_b, id_c, true).unwrap();
    graph.add_dependency(id_c, id_d, true).unwrap();

    // 获取 a 的公开依赖传递闭包
    let closure = graph.get_public_dependency_closure(id_a).unwrap();

    assert!(closure.contains(&id_b));
    assert!(closure.contains(&id_c));
    assert!(closure.contains(&id_d));
    assert!(!closure.contains(&id_a));
}

// ==================== 函数特化名称测试 ====================

/// 测试：FunctionId 特化名称格式
#[test]
fn test_function_id_specialized_name_format() {
    // 无参数
    let id = FunctionId::new("foo".to_string(), vec![]);
    assert_eq!(id.specialized_name(), "foo");

    // 单参数
    let id = FunctionId::new("foo".to_string(), vec![int_type()]);
    assert_eq!(id.specialized_name(), "foo_int64");

    // 多参数
    let id = FunctionId::new("foo".to_string(), vec![int_type(), float_type()]);
    assert_eq!(id.specialized_name(), "foo_int64_float64");
}

/// 测试：TypeId 特化名称格式
#[test]
fn test_type_id_specialized_name_format() {
    // 无参数
    let id = TypeId::new("MyType".to_string(), vec![]);
    assert_eq!(id.specialized_name(), "MyType");

    // 带参数
    let id = TypeId::new("Box".to_string(), vec![int_type()]);
    assert_eq!(id.specialized_name(), "Box_int64");
}

/// 测试：FunctionInstance 设置和获取 IR
#[test]
fn test_function_instance_ir() {
    let generic_id = GenericFunctionId::new("test".to_string(), vec!["T".to_string()]);
    let mut instance = FunctionInstance::new(
        FunctionId::new("test_int64".to_string(), vec![int_type()]),
        generic_id,
        vec![int_type()],
    );

    assert!(instance.get_ir().is_none());

    let ir = create_function_ir("test_int64", vec![], int_type());
    instance.set_ir(ir.clone());

    assert!(instance.get_ir().is_some());
    assert_eq!(instance.get_ir().unwrap().name, "test_int64");
}

// ==================== 泛型函数 ID 签名测试 ====================

/// 测试：GenericFunctionId 签名格式
#[test]
fn test_generic_function_id_signature() {
    // 无类型参数
    let id = GenericFunctionId::new("simple".to_string(), vec![]);
    assert_eq!(id.signature(), "simple");

    // 单类型参数
    let id = GenericFunctionId::new("id".to_string(), vec!["T".to_string()]);
    assert_eq!(id.signature(), "id<T>");

    // 多类型参数
    let id = GenericFunctionId::new("pair".to_string(), vec!["T".to_string(), "U".to_string()]);
    assert_eq!(id.signature(), "pair<T, U>");
}

/// 测试：GenericTypeId 访问器
#[test]
fn test_generic_type_id_accessors() {
    let id = GenericTypeId::new("Option".to_string(), vec!["T".to_string()]);
    assert_eq!(id.name(), "Option");
    assert_eq!(id.type_params(), &vec!["T"]);
}

// ==================== 闭包特化键测试 ====================

/// 测试：ClosureId 特化名称格式
#[test]
fn test_closure_id_specialized_name_format() {
    // 无类型参数，无捕获
    let id = ClosureId::new("foo".to_string(), vec![], vec![]);
    assert_eq!(id.specialized_name(), "foo");

    // 有类型参数
    let id = ClosureId::new("foo".to_string(), vec![int_type()], vec![]);
    assert_eq!(id.specialized_name(), "foo_int64");

    // 有捕获变量
    let id = ClosureId::new("foo".to_string(), vec![], vec![float_type()]);
    assert_eq!(id.specialized_name(), "foo_cap_float64");

    // 都有
    let id = ClosureId::new("foo".to_string(), vec![int_type()], vec![float_type()]);
    assert_eq!(id.specialized_name(), "foo_int64_cap_float64");
}

// ==================== 错误场景测试 ====================

/// 测试：不存在的函数单态化
#[test]
fn test_nonexistent_function_monomorphize() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("does_not_exist".to_string(), vec!["T".to_string()]);
    let result = mono.monomorphize_function(&id, &[int_type()]);

    assert!(result.is_none());
}

/// 测试：不存在的闭包单态化
#[test]
fn test_nonexistent_closure_monomorphize() {
    let mut mono = Monomorphizer::new();

    let id = GenericClosureId::new("ghost".to_string(), vec![], vec![]);
    let result = mono.monomorphize_closure(&id, &[], &[]);

    assert!(result.is_none());
}

/// 测试：不存在的类型单态化
#[test]
fn test_nonexistent_type_monomorphize() {
    let mut mono = Monomorphizer::new();

    let result = mono.monomorphize_type(
        &GenericTypeId::new("Phantom".to_string(), vec!["T".to_string()]),
        &[int_type()],
    );

    assert!(result.is_none());
}

/// 测试：空类型参数的泛型函数
#[test]
fn test_empty_type_args() {
    let mut mono = Monomorphizer::new();

    // 非泛型函数
    let id = GenericFunctionId::new("regular".to_string(), vec![]);
    let ir = create_function_ir("regular", vec![], int_type());
    mono.generic_functions.insert(id.clone(), ir);

    let result = mono.monomorphize_function(&id, &[]);
    assert!(result.is_some());
}

// ==================== 泛型类型实例化测试 ====================

/// 测试：泛型类型实例化注册
#[test]
fn test_type_instantiation_registration() {
    let mut mono = Monomorphizer::new();

    // 注册类型实例
    let type_id = mono.register_monomorphized_type(MonoType::String);
    assert_eq!(type_id.specialized_name(), "string");
}

// ==================== 类型参数顺序测试 ====================

/// 测试：不同类型参数顺序生成不同实例
#[test]
fn test_type_param_order_matters() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("pair".to_string(), vec!["T".to_string(), "U".to_string()]);
    let ir = create_function_ir("pair", vec![], int_type());
    mono.generic_functions.insert(id.clone(), ir);

    // (Int, Float) vs (Float, Int)
    let result1 = mono.monomorphize_function(&id, &[int_type(), float_type()]);
    let result2 = mono.monomorphize_function(&id, &[float_type(), int_type()]);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let name1 = result1.unwrap().specialized_name();
    let name2 = result2.unwrap().specialized_name();

    assert_ne!(name1, name2);
}

// ==================== 模块 ID 测试 ====================

/// 测试：模块 ID 索引访问
#[test]
fn test_module_id_index() {
    let id0 = ModuleId::new(0);
    let id5 = ModuleId::new(5);

    assert_eq!(id0.index(), 0);
    assert_eq!(id5.index(), 5);
}

/// 测试：模块图多次添加相同路径
#[test]
fn test_module_graph_duplicate_path() {
    let mut graph = ModuleGraph::new();

    let path = PathBuf::from("test.yx");
    let id1 = graph.add_module(path.clone());
    let id2 = graph.add_module(path.clone());

    // 应该返回相同的 ID
    assert_eq!(id1, id2);
    assert_eq!(graph.len(), 1);
}

/// 测试：模块图空图拓扑排序
#[test]
fn test_empty_graph_topological_sort() {
    let graph = ModuleGraph::new();

    let result = graph.topological_sort();
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

// ==================== 实例相等性测试 ====================

/// 测试：相同泛型 ID 和类型参数的实例相等
#[test]
fn test_instance_equality() {
    let id1 = FunctionId::new("test".to_string(), vec![int_type()]);
    let id2 = FunctionId::new("test".to_string(), vec![int_type()]);

    assert_eq!(id1, id2);
    assert_eq!(id1.name(), id2.name());
}

/// 测试：不同类型参数的实例不等
#[test]
fn test_instance_inequality() {
    let id1 = FunctionId::new("test".to_string(), vec![int_type()]);
    let id2 = FunctionId::new("test".to_string(), vec![float_type()]);

    assert_ne!(id1, id2);
}

/// 测试：泛型闭包 ID 相等性
#[test]
fn test_generic_closure_id_equality() {
    let id1 = GenericClosureId::new(
        "foo".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string()],
    );
    let id2 = GenericClosureId::new(
        "foo".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string()],
    );
    let id3 = GenericClosureId::new(
        "bar".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string()],
    );

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

/// 测试：GenericFunctionId 相等性
#[test]
fn test_generic_function_id_equality() {
    let id1 = GenericFunctionId::new("foo".to_string(), vec!["T".to_string()]);
    let id2 = GenericFunctionId::new("foo".to_string(), vec!["T".to_string()]);
    let id3 = GenericFunctionId::new("bar".to_string(), vec!["T".to_string()]);

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

/// 测试：ClosureId 相等性
#[test]
fn test_closure_id_equality() {
    let id1 = ClosureId::new("test".to_string(), vec![int_type()], vec![]);
    let id2 = ClosureId::new("test".to_string(), vec![int_type()], vec![]);
    let id3 = ClosureId::new("test".to_string(), vec![float_type()], vec![]);

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

/// 测试：GenericTypeId 相等性
#[test]
fn test_generic_type_id_equality() {
    let id1 = GenericTypeId::new("Option".to_string(), vec!["T".to_string()]);
    let id2 = GenericTypeId::new("Option".to_string(), vec!["T".to_string()]);
    let id3 = GenericTypeId::new("Result".to_string(), vec!["T".to_string()]);

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

/// 测试：TypeId 相等性
#[test]
fn test_type_id_equality() {
    let id1 = TypeId::new("Box".to_string(), vec![int_type()]);
    let id2 = TypeId::new("Box".to_string(), vec![int_type()]);
    let id3 = TypeId::new("Box".to_string(), vec![float_type()]);

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

/// 测试：泛型函数访问器
#[test]
fn test_generic_function_id_accessors() {
    let id = GenericFunctionId::new("foo".to_string(), vec!["T".to_string(), "U".to_string()]);

    assert_eq!(id.name(), "foo");
    assert_eq!(id.type_params(), &vec!["T", "U"]);
}

/// 测试：GenericClosureId 访问器
#[test]
fn test_generic_closure_id_accessors() {
    let id = GenericClosureId::new(
        "callback".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string(), "y".to_string()],
    );

    assert_eq!(id.name(), "callback");
    assert_eq!(id.type_params(), &vec!["T"]);
    assert_eq!(id.capture_names(), &vec!["x", "y"]);
}

/// 测试：ClosureId 访问器
#[test]
fn test_closure_id_accessors() {
    let id = ClosureId::new("test".to_string(), vec![int_type()], vec![float_type()]);

    assert_eq!(id.name(), "test");
    assert_eq!(id.type_args().len(), 1);
    assert_eq!(id.capture_types().len(), 1);
}

/// 测试：FunctionId 访问器
#[test]
fn test_function_id_accessors() {
    let id = FunctionId::new("foo".to_string(), vec![int_type(), float_type()]);

    assert_eq!(id.name(), "foo");
    assert_eq!(id.specialized_name(), "foo_int64_float64");
}

/// 测试：TypeId 访问器
#[test]
fn test_type_id_accessors() {
    let id = TypeId::new("Box".to_string(), vec![int_type()]);

    assert_eq!(id.name(), "Box");
    assert_eq!(id.specialized_name(), "Box_int64");
}

/// 测试：Void 类型单态化
#[test]
fn test_void_type_monomorphize() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("make_void".to_string(), vec![]);
    let ir = create_function_ir("make_void", vec![], void_type());
    mono.generic_functions.insert(id.clone(), ir);

    let result = mono.monomorphize_function(&id, &[]);
    assert!(result.is_some());
}

/// 测试：Bool 类型单态化
#[test]
fn test_bool_type_monomorphize() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("flip".to_string(), vec![]);
    let ir = create_function_ir("flip", vec![], bool_type());
    mono.generic_functions.insert(id.clone(), ir);

    let result = mono.monomorphize_function(&id, &[]);
    assert!(result.is_some());
}

/// 测试：检查函数是否已单态化
#[test]
fn test_is_function_monomorphized() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("test".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("test", vec![], int_type());
    mono.generic_functions.insert(id.clone(), ir);

    // 初始状态：未单态化
    assert!(!mono.is_function_monomorphized(&id, &[int_type()]));

    // 单态化后
    let _ = mono.monomorphize_function(&id, &[int_type()]);
    assert!(mono.is_function_monomorphized(&id, &[int_type()]));

    // 不同类型仍未单态化
    assert!(!mono.is_function_monomorphized(&id, &[float_type()]));
}

/// 测试：检查闭包是否已单态化
#[test]
fn test_is_closure_monomorphized() {
    let mut mono = Monomorphizer::new();

    let id = GenericClosureId::new("test".to_string(), vec![], vec!["x".to_string()]);
    let instance = ClosureInstance::new(
        ClosureId::new("test".to_string(), vec![], vec![]),
        id.clone(),
        vec![],
        vec![],
        create_function_ir("body", vec![], int_type()),
    );
    mono.register_generic_closure(id.clone(), instance);

    // 初始状态
    assert!(!mono.is_closure_monomorphized(&id, &[], &[int_type()]));

    // 单态化后
    let _ = mono.monomorphize_closure(&id, &[], &[int_type()]);
    assert!(mono.is_closure_monomorphized(&id, &[], &[int_type()]));
}

/// 测试：获取已实例化的函数
#[test]
fn test_get_instantiated_function() {
    let mut mono = Monomorphizer::new();

    let id = GenericFunctionId::new("test".to_string(), vec!["T".to_string()]);
    let ir = create_function_ir("test", vec![], int_type());
    mono.generic_functions.insert(id.clone(), ir);

    let func_id = mono.monomorphize_function(&id, &[int_type()]);
    assert!(func_id.is_some());

    let instantiated = mono.get_instantiated_function(&func_id.unwrap());
    assert!(instantiated.is_some());
    // 单态化后的函数名包含类型信息
    assert!(instantiated.unwrap().name.contains("test"));
}

/// 测试：获取已单态化的类型
#[test]
fn test_type_registration() {
    let mut mono = Monomorphizer::new();

    // 注册类型实例
    let type_id = mono.register_monomorphized_type(MonoType::String);
    assert_eq!(type_id.specialized_name(), "string");

    // 再注册一个
    let type_id2 = mono.register_monomorphized_type(MonoType::Int(64));
    assert_eq!(type_id2.specialized_name(), "int64");

    // 验证数量
    assert_eq!(mono.type_instance_count(), 2);
}

/// 测试：类型实例数量
#[test]
fn test_type_instance_count() {
    let mono = Monomorphizer::new();
    assert_eq!(mono.type_instance_count(), 0);
}

/// 测试：泛型类型数量
#[test]
fn test_generic_type_count() {
    let mono = Monomorphizer::new();
    assert_eq!(mono.generic_type_count(), 0);
}

/// 测试：GenericClosureId 签名格式
#[test]
fn test_generic_closure_id_signature() {
    // 无捕获
    let id = GenericClosureId::new("simple".to_string(), vec![], vec![]);
    assert_eq!(id.signature(), "simple");

    // 有捕获
    let id = GenericClosureId::new("lambda".to_string(), vec![], vec!["x".to_string()]);
    assert_eq!(id.signature(), "lambda|[x]|");

    // 有类型参数和捕获
    let id = GenericClosureId::new(
        "generic".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string()],
    );
    assert_eq!(id.signature(), "generic<T>|[x]|");
}

/// 测试：泛型闭包 ID 哈希
#[test]
fn test_generic_closure_id_hash() {
    use std::collections::HashMap;

    let mut map: HashMap<GenericClosureId, i32> = HashMap::new();

    let id1 = GenericClosureId::new("test".to_string(), vec!["T".to_string()], vec![]);
    let id2 = GenericClosureId::new("test".to_string(), vec!["T".to_string()], vec![]);

    map.insert(id1.clone(), 1);
    assert_eq!(map.get(&id2), Some(&1));
}

/// 测试：闭包实例访问器
#[test]
fn test_closure_instance_accessors() {
    let capture_vars = vec![crate::middle::monomorphize::instance::CaptureVariable::new(
        "x".to_string(),
        int_type(),
        Operand::Local(0),
    )];

    let closure_id = ClosureId::new("test".to_string(), vec![int_type()], vec![]);
    let generic_id = GenericClosureId::new(
        "test".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string()],
    );

    let instance = ClosureInstance::new(
        closure_id.clone(),
        generic_id.clone(),
        vec![int_type()],
        capture_vars,
        create_function_ir("body", vec![], int_type()),
    );

    assert_eq!(instance.id.name(), "test");
    assert_eq!(instance.generic_id.name(), "test");
    assert_eq!(instance.type_args.len(), 1);
    assert_eq!(instance.capture_vars.len(), 1);
}

/// 测试：FunctionInstance 访问器
#[test]
fn test_function_instance_accessors() {
    let generic_id = GenericFunctionId::new("foo".to_string(), vec!["T".to_string()]);
    let func_id = FunctionId::new("foo_int64".to_string(), vec![int_type()]);
    let instance = FunctionInstance::new(func_id.clone(), generic_id.clone(), vec![int_type()]);

    assert_eq!(instance.id.name(), "foo_int64");
    assert_eq!(instance.generic_id.name(), "foo");
    assert_eq!(instance.type_args.len(), 1);
    assert!(instance.get_ir().is_none());
}

/// 测试：TypeInstance 访问器
#[test]
fn test_type_instance_accessors() {
    let generic_id = GenericTypeId::new("Box".to_string(), vec!["T".to_string()]);
    let type_id = TypeId::new("Box_int64".to_string(), vec![int_type()]);
    let mut instance = crate::middle::monomorphize::instance::TypeInstance::new(
        type_id.clone(),
        generic_id.clone(),
        vec![int_type()],
    );

    assert_eq!(instance.id.name(), "Box_int64");
    assert_eq!(instance.generic_id.name(), "Box");
    assert_eq!(instance.type_args.len(), 1);
    assert!(instance.get_mono_type().is_none());

    instance.set_mono_type(MonoType::String);
    assert!(instance.get_mono_type().is_some());
}
