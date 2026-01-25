//! 函数单态化测试
//!
//! 测试泛型函数的单态化功能，包括：
//! - 简单泛型函数单态化
//! - 多参数泛型函数
//! - 递归函数单态化
//! - 函数调用链
//! - 缓存验证

use crate::middle::passes::mono::Monomorphizer;
use crate::middle::passes::mono::instance::{FunctionId, GenericFunctionId};
use crate::middle::{FunctionIR, BasicBlock};
use crate::frontend::typecheck::MonoType;

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

/// 创建简单 FunctionIR 的辅助函数
fn create_simple_function_ir(
    name: &str,
    param_types: Vec<MonoType>,
    return_type: MonoType,
) -> FunctionIR {
    let locals: Vec<MonoType> = param_types.iter().cloned().collect();
    FunctionIR {
        name: name.to_string(),
        params: param_types,
        return_type,
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

/// 创建包含 TypeVar 的泛型函数 IR
fn create_generic_function_ir(
    name: &str,
    type_param_count: usize,
) -> FunctionIR {
    let param_types: Vec<MonoType> = (0..type_param_count)
        .map(|i| MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(i)))
        .collect();
    let return_type = MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(0));
    let locals: Vec<MonoType> = param_types
        .iter()
        .cloned()
        .chain(std::iter::once(return_type.clone()))
        .collect();

    FunctionIR {
        name: name.to_string(),
        params: param_types,
        return_type,
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

/// 测试：简单泛型函数单态化
#[test]
fn test_monomorphize_simple_generic_function() {
    let mut mono = Monomorphizer::new();

    // 注册泛型函数 identity<T>
    let generic_id = GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]);
    let generic_ir = create_generic_function_ir("identity", 1);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 单态化为 identity<Int>
    let result = mono.monomorphize_function(&generic_id, &[int_type()]);

    assert!(result.is_some());
    let func_id = result.unwrap();
    assert_eq!(func_id.name(), "identity_int64");
}

/// 测试：多参数泛型函数单态化
#[test]
fn test_monomorphize_multi_param_function() {
    let mut mono = Monomorphizer::new();

    // 注册泛型函数 pair<T, U>
    let generic_id =
        GenericFunctionId::new("pair".to_string(), vec!["T".to_string(), "U".to_string()]);
    let generic_ir = create_generic_function_ir("pair", 2);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 单态化为 pair<Int, String>
    let result = mono.monomorphize_function(&generic_id, &[int_type(), string_type()]);

    assert!(result.is_some());
    let func_id = result.unwrap();
    assert_eq!(func_id.name(), "pair_int64_string");
}

/// 测试：同一泛型函数多次单态化
#[test]
fn test_monomorphize_same_function_different_types() {
    let mut mono = Monomorphizer::new();

    // 注册泛型函数 id<T>
    let generic_id = GenericFunctionId::new("id".to_string(), vec!["T".to_string()]);
    let generic_ir = create_generic_function_ir("id", 1);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 单态化为 id<Int>
    let result1 = mono.monomorphize_function(&generic_id, &[int_type()]);
    assert!(result1.is_some());

    // 单态化为 id<Float>
    let result2 = mono.monomorphize_function(&generic_id, &[float_type()]);
    assert!(result2.is_some());

    // 验证生成两个不同的函数
    assert_ne!(result1.unwrap(), result2.unwrap());
    assert_eq!(mono.instantiated_function_count(), 2);
}

/// 测试：缓存命中
#[test]
fn test_monomorphize_cache_hit() {
    let mut mono = Monomorphizer::new();

    // 注册泛型函数 test<T>
    let generic_id = GenericFunctionId::new("test".to_string(), vec!["T".to_string()]);
    let generic_ir = create_generic_function_ir("test", 1);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 首次单态化
    let result1 = mono.monomorphize_function(&generic_id, &[int_type()]);
    assert!(result1.is_some());
    let first_count = mono.instantiated_function_count();

    // 再次单态化相同类型（应该命中缓存）
    let result2 = mono.monomorphize_function(&generic_id, &[int_type()]);
    assert!(result2.is_some());

    // 数量不应该增加
    assert_eq!(mono.instantiated_function_count(), first_count);

    // 返回相同的函数ID
    assert_eq!(result1, result2);
}

/// 测试：检查函数是否已单态化
#[test]
fn test_is_function_monomorphized() {
    let mut mono = Monomorphizer::new();

    // 注册泛型函数 check<T>
    let generic_id = GenericFunctionId::new("check".to_string(), vec!["T".to_string()]);
    let generic_ir = create_generic_function_ir("check", 1);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 初始状态：未单态化
    assert!(!mono.is_function_monomorphized(&generic_id, &[int_type()]));

    // 单态化后
    let _ = mono.monomorphize_function(&generic_id, &[int_type()]);
    assert!(mono.is_function_monomorphized(&generic_id, &[int_type()]));

    // 不同类型仍未单态化
    assert!(!mono.is_function_monomorphized(&generic_id, &[float_type()]));
}

/// 测试：获取已实例化的函数
#[test]
fn test_get_instantiated_function() {
    let mut mono = Monomorphizer::new();

    // 注册泛型函数 get<T>
    let generic_id = GenericFunctionId::new("get".to_string(), vec!["T".to_string()]);
    let generic_ir = create_generic_function_ir("get", 1);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 单态化
    let func_id = mono.monomorphize_function(&generic_id, &[string_type()]);
    assert!(func_id.is_some());

    // 获取已实例化的函数
    let instantiated = mono.get_instantiated_function(&func_id.unwrap());
    assert!(instantiated.is_some());
    assert_eq!(instantiated.unwrap().name, "get_string");
}

/// 测试：不存在的泛型函数
#[test]
fn test_monomorphize_nonexistent_function() {
    let mut mono = Monomorphizer::new();

    let generic_id = GenericFunctionId::new("nonexistent".to_string(), vec!["T".to_string()]);

    // 尝试单态化不存在的函数
    let result = mono.monomorphize_function(&generic_id, &[int_type()]);

    assert!(result.is_none());
    assert_eq!(mono.instantiated_function_count(), 0);
}

/// 测试：无类型参数的函数（不应该被视为泛型）
#[test]
fn test_non_generic_function() {
    let mut mono = Monomorphizer::new();

    // 注册非泛型函数
    let generic_id = GenericFunctionId::new("normal_func".to_string(), vec![]);
    let generic_ir = create_simple_function_ir("normal_func", vec![int_type()], int_type());
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 单态化无类型参数的函数
    let result = mono.monomorphize_function(&generic_id, &[]);

    assert!(result.is_some());
    assert_eq!(result.unwrap().name(), "normal_func");
}

/// 测试：已实例化函数数量
#[test]
fn test_instantiated_function_count() {
    let mono = Monomorphizer::new();
    assert_eq!(mono.instantiated_function_count(), 0);

    let mut mono = Monomorphizer::new();
    let generic_id = GenericFunctionId::new("count_test".to_string(), vec!["T".to_string()]);
    let generic_ir = create_generic_function_ir("count_test", 1);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    let _ = mono.monomorphize_function(&generic_id, &[int_type()]);
    assert_eq!(mono.instantiated_function_count(), 1);

    let _ = mono.monomorphize_function(&generic_id, &[float_type()]);
    assert_eq!(mono.instantiated_function_count(), 2);
}

/// 测试：不同类型参数顺序产生不同实例
#[test]
fn test_different_type_order_different_instances() {
    let mut mono = Monomorphizer::new();

    // 注册泛型函数 reorder<T, U>
    let generic_id = GenericFunctionId::new(
        "reorder".to_string(),
        vec!["T".to_string(), "U".to_string()],
    );
    let generic_ir = create_generic_function_ir("reorder", 2);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 单态化为 reorder<Int, Float>
    let result1 = mono.monomorphize_function(&generic_id, &[int_type(), float_type()]);
    // 单态化为 reorder<Float, Int>
    let result2 = mono.monomorphize_function(&generic_id, &[float_type(), int_type()]);

    assert!(result1.is_some());
    assert!(result2.is_some());

    // 应该生成不同的实例
    assert_ne!(result1.unwrap(), result2.unwrap());
}

/// 测试：泛型函数ID签名生成
#[test]
fn test_generic_function_id_signature() {
    let id1 = GenericFunctionId::new("single".to_string(), vec!["T".to_string()]);
    assert_eq!(id1.signature(), "single<T>");

    let id2 = GenericFunctionId::new(
        "multi".to_string(),
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
    );
    assert_eq!(id2.signature(), "multi<A, B, C>");

    let id3 = GenericFunctionId::new("no_params".to_string(), vec![]);
    assert_eq!(id3.signature(), "no_params");
}

/// 测试：FunctionId 特化名称生成
#[test]
fn test_function_id_specialized_name() {
    let id1 = FunctionId::new("test".to_string(), vec![]);
    assert_eq!(id1.specialized_name(), "test");

    let id2 = FunctionId::new("test".to_string(), vec![int_type()]);
    assert_eq!(id2.specialized_name(), "test_int64");

    let id3 = FunctionId::new("test".to_string(), vec![int_type(), string_type()]);
    assert_eq!(id3.specialized_name(), "test_int64_string");
}

/// 测试：空类型参数列表
#[test]
fn test_empty_type_args() {
    let mut mono = Monomorphizer::new();

    // 注册无类型参数的函数
    let generic_id = GenericFunctionId::new("no_type_params".to_string(), vec![]);
    let generic_ir = create_simple_function_ir("no_type_params", vec![], MonoType::Void);
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    let result = mono.monomorphize_function(&generic_id, &[]);
    assert!(result.is_some());
    assert_eq!(result.unwrap().name(), "no_type_params");
}

/// 测试：单态化后的函数IR参数类型替换
#[test]
fn test_monomorphized_function_param_types() {
    let mut mono = Monomorphizer::new();

    // 创建泛型函数，参数类型为 TypeVar
    let generic_id = GenericFunctionId::new("param_test".to_string(), vec!["T".to_string()]);
    let param_type = MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(0));
    let return_type = MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(0));

    let generic_ir = FunctionIR {
        name: "param_test".to_string(),
        params: vec![param_type.clone()],
        return_type,
        is_async: false,
        locals: vec![param_type],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![],
            successors: vec![],
        }],
        entry: 0,
    };
    mono.test_insert_generic_function(generic_id.clone(), generic_ir);

    // 单态化为 param_test<Int>
    let func_id = mono.monomorphize_function(&generic_id, &[int_type()]);
    assert!(func_id.is_some());

    // 验证生成的函数
    let instantiated = mono.get_instantiated_function(&func_id.unwrap());
    assert!(instantiated.is_some());
    let func = instantiated.unwrap();

    // 参数类型应该被替换为 Int
    assert_eq!(func.params.len(), 1);
    assert_eq!(func.params[0], int_type());

    // 返回类型也应该被替换
    assert_eq!(func.return_type, int_type());
}
