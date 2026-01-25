//! 闭包单态化测试
//!
//! 测试闭包的单态化功能，包括：
//! - 简单闭包单态化
//! - 带捕获变量的闭包
//! - 缓存命中
//! - 不同类型生成不同实例

use crate::middle::passes::mono::Monomorphizer;
use crate::middle::passes::mono::closure::ClosureMonomorphizer;
use crate::middle::passes::mono::instance::{
    ClosureId, ClosureInstance, ClosureSpecializationKey, GenericClosureId, CaptureVariable,
};
use crate::middle::{FunctionIR, BasicBlock};
use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::Operand;

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

/// 创建简单的闭包实例辅助函数
fn create_simple_closure_instance(
    name: &str,
    capture_types: Vec<MonoType>,
) -> ClosureInstance {
    let capture_vars: Vec<CaptureVariable> = capture_types
        .iter()
        .enumerate()
        .map(|(i, ty)| {
            CaptureVariable::new(format!("capture_{}", i), ty.clone(), Operand::Local(i))
        })
        .collect();

    let body_ir = create_simple_function_ir(&format!("{}_body", name), vec![], int_type());

    ClosureInstance::new(
        ClosureId::new(name.to_string(), vec![], capture_types.clone()),
        GenericClosureId::new(
            name.to_string(),
            vec![],
            capture_vars.iter().map(|c| c.name.clone()).collect(),
        ),
        vec![],
        capture_vars,
        body_ir,
    )
}

/// 测试：简单闭包单态化（无捕获变量）
#[test]
fn test_simple_closure_monomorphize() {
    let mut mono = Monomorphizer::new();

    // 注册泛型闭包（无捕获变量）
    let generic_id = GenericClosureId::new("simple_closure".to_string(), vec![], vec![]);
    let closure_instance = create_simple_closure_instance("simple_closure", vec![]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 单态化
    let result = mono.monomorphize_closure(&generic_id, &[], &[]);

    assert!(result.is_some());
    let closure_id = result.unwrap();
    assert!(closure_id.name().starts_with("closure_"));
}

/// 测试：带捕获变量的闭包单态化
#[test]
fn test_closure_with_captures() {
    let mut mono = Monomorphizer::new();

    // 注册带捕获变量的闭包
    let generic_id = GenericClosureId::new("adder".to_string(), vec![], vec!["x".to_string()]);
    let closure_instance = create_simple_closure_instance("adder", vec![int_type()]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 单态化，捕获类型为 Int
    let result = mono.monomorphize_closure(&generic_id, &[], &[int_type()]);

    assert!(result.is_some());
    let closure_id = result.unwrap();

    // 验证捕获类型
    assert_eq!(closure_id.capture_types().len(), 1);
    assert_eq!(closure_id.capture_types()[0], int_type());
}

/// 测试：闭包缓存命中
#[test]
fn test_closure_cache_hit() {
    let mut mono = Monomorphizer::new();

    // 注册泛型闭包
    let generic_id =
        GenericClosureId::new("cached_closure".to_string(), vec![], vec!["x".to_string()]);
    let closure_instance = create_simple_closure_instance("cached_closure", vec![int_type()]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 首次单态化
    let result1 = mono.monomorphize_closure(&generic_id, &[], &[int_type()]);
    assert!(result1.is_some());
    let first_count = mono.instantiated_closure_count();

    // 再次单态化相同类型（应该命中缓存）
    let result2 = mono.monomorphize_closure(&generic_id, &[], &[int_type()]);
    assert!(result2.is_some());

    // 数量不应该增加
    assert_eq!(mono.instantiated_closure_count(), first_count);

    // 返回相同的闭包ID
    assert_eq!(result1, result2);
}

/// 测试：不同捕获类型生成不同实例
#[test]
fn test_different_capture_types() {
    let mut mono = Monomorphizer::new();

    // 注册泛型闭包
    let generic_id =
        GenericClosureId::new("multi_closure".to_string(), vec![], vec!["x".to_string()]);
    let closure_instance = create_simple_closure_instance("multi_closure", vec![]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 单态化为捕获 Int
    let result1 = mono.monomorphize_closure(&generic_id, &[], &[int_type()]);
    assert!(result1.is_some());

    // 单态化为捕获 Float
    let result2 = mono.monomorphize_closure(&generic_id, &[], &[float_type()]);
    assert!(result2.is_some());

    // 应该生成不同的实例
    assert_ne!(result1.unwrap(), result2.unwrap());
    assert_eq!(mono.instantiated_closure_count(), 2);
}

/// 测试：检查闭包是否已单态化
#[test]
fn test_is_closure_monomorphized() {
    let mut mono = Monomorphizer::new();

    // 注册泛型闭包
    let generic_id =
        GenericClosureId::new("check_closure".to_string(), vec![], vec!["x".to_string()]);
    let closure_instance = create_simple_closure_instance("check_closure", vec![]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 初始状态：未单态化
    assert!(!mono.is_closure_monomorphized(&generic_id, &[], &[int_type()]));

    // 单态化后
    let _ = mono.monomorphize_closure(&generic_id, &[], &[int_type()]);
    assert!(mono.is_closure_monomorphized(&generic_id, &[], &[int_type()]));

    // 不同类型仍未单态化
    assert!(!mono.is_closure_monomorphized(&generic_id, &[], &[float_type()]));
}

/// 测试：获取已实例化的闭包
#[test]
fn test_get_instantiated_closure() {
    let mut mono = Monomorphizer::new();

    // 注册泛型闭包
    let generic_id =
        GenericClosureId::new("get_closure".to_string(), vec![], vec!["x".to_string()]);
    let closure_instance = create_simple_closure_instance("get_closure", vec![string_type()]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 单态化
    let closure_id = mono.monomorphize_closure(&generic_id, &[], &[string_type()]);
    assert!(closure_id.is_some());

    // 获取已实例化的闭包
    let instantiated = mono.get_instantiated_closure(&closure_id.unwrap());
    assert!(instantiated.is_some());
    assert_eq!(instantiated.unwrap().generic_id.name(), "get_closure");
}

/// 测试：已单态化闭包数量
#[test]
fn test_instantiated_closure_count() {
    let mono = Monomorphizer::new();
    assert_eq!(mono.instantiated_closure_count(), 0);

    let mut mono = Monomorphizer::new();
    let generic_id =
        GenericClosureId::new("count_closure".to_string(), vec![], vec!["x".to_string()]);
    let closure_instance = create_simple_closure_instance("count_closure", vec![]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    let _ = mono.monomorphize_closure(&generic_id, &[], &[int_type()]);
    assert_eq!(mono.instantiated_closure_count(), 1);

    let _ = mono.monomorphize_closure(&generic_id, &[], &[float_type()]);
    assert_eq!(mono.instantiated_closure_count(), 2);
}

/// 测试：泛型闭包ID签名
#[test]
fn test_generic_closure_id_signature() {
    let id1 = GenericClosureId::new("simple".to_string(), vec![], vec![]);
    assert_eq!(id1.signature(), "simple");

    let id2 = GenericClosureId::new("with_capture".to_string(), vec![], vec!["x".to_string()]);
    assert_eq!(id2.signature(), "with_capture|[x]|");

    let id3 = GenericClosureId::new(
        "generic".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string()],
    );
    assert_eq!(id3.signature(), "generic<T>|[x]|");
}

/// 测试：闭包ID特化名称
#[test]
fn test_closure_id_specialized_name() {
    let id1 = ClosureId::new("test".to_string(), vec![], vec![]);
    assert_eq!(id1.specialized_name(), "test");

    let id2 = ClosureId::new("test".to_string(), vec![int_type()], vec![]);
    assert_eq!(id2.specialized_name(), "test_int64");

    let id3 = ClosureId::new("test".to_string(), vec![], vec![int_type()]);
    assert_eq!(id3.specialized_name(), "test_cap_int64");

    let id4 = ClosureId::new("test".to_string(), vec![int_type()], vec![string_type()]);
    assert_eq!(id4.specialized_name(), "test_int64_cap_string");
}

/// 测试：闭包特化缓存键
#[test]
fn test_closure_specialization_key() {
    let key1 = ClosureSpecializationKey::new("closure1".to_string(), vec![], vec![]);
    assert_eq!(key1.as_string(), "closure1");

    let key2 = ClosureSpecializationKey::new("closure2".to_string(), vec![int_type()], vec![]);
    assert_eq!(key2.as_string(), "closure2<int64>");

    let key3 = ClosureSpecializationKey::new("closure3".to_string(), vec![], vec![int_type()]);
    assert_eq!(key3.as_string(), "closure3|[int64]|");

    let key4 = ClosureSpecializationKey::new(
        "closure4".to_string(),
        vec![int_type()],
        vec![string_type()],
    );
    assert_eq!(key4.as_string(), "closure4<int64>|[string]|");
}

/// 测试：不存在的闭包单态化
#[test]
fn test_monomorphize_nonexistent_closure() {
    let mut mono = Monomorphizer::new();

    let generic_id = GenericClosureId::new("nonexistent".to_string(), vec![], vec![]);

    // 尝试单态化不存在的闭包
    let result = mono.monomorphize_closure(&generic_id, &[], &[]);

    assert!(result.is_none());
    assert_eq!(mono.instantiated_closure_count(), 0);
}

/// 测试：多捕获变量
#[test]
fn test_multiple_captures() {
    let mut mono = Monomorphizer::new();

    // 注册带多个捕获变量的闭包
    let generic_id = GenericClosureId::new(
        "multi_capture".to_string(),
        vec![],
        vec!["x".to_string(), "y".to_string()],
    );
    let closure_instance =
        create_simple_closure_instance("multi_capture", vec![int_type(), string_type()]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 单态化
    let result = mono.monomorphize_closure(&generic_id, &[], &[int_type(), string_type()]);

    assert!(result.is_some());
    let closure_id = result.unwrap();

    // 验证捕获类型数量
    assert_eq!(closure_id.capture_types().len(), 2);
}

/// 测试：空捕获列表
#[test]
fn test_empty_captures() {
    let mut mono = Monomorphizer::new();

    // 注册无捕获的闭包
    let generic_id = GenericClosureId::new("no_capture".to_string(), vec![], vec![]);
    let closure_instance = create_simple_closure_instance("no_capture", vec![]);
    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 单态化
    let result = mono.monomorphize_closure(&generic_id, &[], &[]);

    assert!(result.is_some());
    let closure_id = result.unwrap();

    // 验证无捕获
    assert!(closure_id.capture_types().is_empty());
}

/// 测试：泛型闭包带类型参数
#[test]
fn test_generic_closure_with_type_params() {
    let mut mono = Monomorphizer::new();

    // 注册泛型闭包（带类型参数）
    let generic_id = GenericClosureId::new(
        "generic_closure".to_string(),
        vec!["T".to_string()],
        vec!["x".to_string()],
    );

    // 创建带 TypeVar 的闭包体
    let type_var = MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(0));
    let capture_vars = vec![CaptureVariable::new(
        "x".to_string(),
        type_var.clone(),
        Operand::Local(0),
    )];
    let body_ir = FunctionIR {
        name: "generic_closure_body".to_string(),
        params: vec![],
        return_type: type_var.clone(),
        is_async: false,
        locals: vec![type_var.clone()],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![],
            successors: vec![],
        }],
        entry: 0,
    };

    let closure_instance = ClosureInstance::new(
        ClosureId::new("generic_closure".to_string(), vec![], vec![]),
        generic_id.clone(),
        vec![],
        capture_vars,
        body_ir,
    );

    mono.register_generic_closure(generic_id.clone(), closure_instance);

    // 单态化为 Int
    let result = mono.monomorphize_closure(&generic_id, &[int_type()], &[int_type()]);

    assert!(result.is_some());
    let closure_id = result.unwrap();

    // 验证类型参数
    assert_eq!(closure_id.type_args().len(), 1);
    assert_eq!(closure_id.type_args()[0], int_type());
}

/// 测试：闭包实例的捕获变量
#[test]
fn test_closure_instance_captures() {
    let mut mono = Monomorphizer::new();

    let generic_id = GenericClosureId::new(
        "capture_test".to_string(),
        vec![],
        vec!["x".to_string(), "y".to_string()],
    );

    let capture_vars = vec![
        CaptureVariable::new("x".to_string(), int_type(), Operand::Local(0)),
        CaptureVariable::new("y".to_string(), string_type(), Operand::Local(1)),
    ];

    let closure_instance = ClosureInstance::new(
        ClosureId::new("capture_test".to_string(), vec![], vec![]),
        generic_id.clone(),
        vec![],
        capture_vars.clone(),
        create_simple_function_ir("body", vec![], int_type()),
    );

    mono.register_generic_closure(generic_id.clone(), closure_instance);

    let closure_id = mono.monomorphize_closure(&generic_id, &[], &[int_type(), string_type()]);
    assert!(closure_id.is_some());

    let instantiated = mono.get_instantiated_closure(&closure_id.unwrap());
    assert!(instantiated.is_some());

    let captures = &instantiated.unwrap().capture_vars;
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].name, "x");
    assert_eq!(captures[1].name, "y");
}
