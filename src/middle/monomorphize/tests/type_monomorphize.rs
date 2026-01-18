//! 类型单态化测试
//!
//! 测试泛型类型的单态化功能，包括：
//! - 结构体类型单态化
//! - 枚举类型单态化
//! - Arc 类型处理（所有权模型）
//! - 类型缓存

use crate::middle::monomorphize::Monomorphizer;
use crate::middle::monomorphize::type_mono::TypeMonomorphizer;
use crate::middle::monomorphize::instance::GenericTypeId;
use crate::frontend::typecheck::{MonoType, StructType, EnumType, TypeVar};

/// 创建类型变量的辅助函数
fn make_type_var(index: usize) -> TypeVar {
    TypeVar::new(index)
}

/// 创建类型变量的 MonoType
fn type_var(index: usize) -> MonoType {
    MonoType::TypeVar(make_type_var(index))
}

/// 创建整数的辅助函数
fn int_type() -> MonoType {
    MonoType::Int(64)
}

/// 创建浮点数的辅助函数
fn float_type() -> MonoType {
    MonoType::Float(64)
}

/// 创建字符串类型的辅助函数
fn string_type() -> MonoType {
    MonoType::String
}

/// 创建 Arc 类型的辅助函数
fn arc_type(inner: MonoType) -> MonoType {
    MonoType::Arc(Box::new(inner))
}

/// 创建列表类型的辅助函数
fn list_type(elem: MonoType) -> MonoType {
    MonoType::List(Box::new(elem))
}

/// 创建字典类型的辅助函数
fn dict_type(
    key: MonoType,
    value: MonoType,
) -> MonoType {
    MonoType::Dict(Box::new(key), Box::new(value))
}

/// 测试：简单结构体单态化
#[test]
fn test_monomorphize_simple_struct() {
    let mut mono = Monomorphizer::new();

    // 定义泛型结构体 Point<T>
    let generic_type = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), type_var(0)),
            ("y".to_string(), type_var(0)),
        ],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("Point".to_string(), vec!["T".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 单态化为 Point<Int>
    let result = mono.monomorphize_type(&generic_id, &[int_type()]);

    assert!(result.is_some());
    let mono_type = result.unwrap();

    match mono_type {
        MonoType::Struct(s) => {
            assert_eq!(s.name, "Point_int64");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].0, "x");
            assert_eq!(s.fields[0].1, int_type());
            assert_eq!(s.fields[1].0, "y");
            assert_eq!(s.fields[1].1, int_type());
        }
        _ => panic!("Expected Struct type"),
    }
}

/// 测试：多参数结构体单态化
#[test]
fn test_monomorphize_multi_param_struct() {
    let mut mono = Monomorphizer::new();

    // 定义泛型结构体 Pair<K, V>
    let generic_type = MonoType::Struct(StructType {
        name: "Pair".to_string(),
        fields: vec![
            ("key".to_string(), type_var(0)),
            ("value".to_string(), type_var(1)),
        ],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("Pair".to_string(), vec!["K".to_string(), "V".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 单态化为 Pair<String, Int>
    let result = mono.monomorphize_type(&generic_id, &[string_type(), int_type()]);

    assert!(result.is_some());
    let mono_type = result.unwrap();

    match mono_type {
        MonoType::Struct(s) => {
            assert_eq!(s.name, "Pair_string_int64");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].0, "key");
            assert_eq!(s.fields[0].1, string_type());
            assert_eq!(s.fields[1].0, "value");
            assert_eq!(s.fields[1].1, int_type());
        }
        _ => panic!("Expected Struct type"),
    }
}

/// 测试：枚举类型单态化
#[test]
fn test_monomorphize_enum() {
    let mut mono = Monomorphizer::new();

    // 定义泛型枚举 Option<T>
    let generic_type = MonoType::Enum(EnumType {
        name: "Option".to_string(),
        variants: vec!["Some".to_string(), "None".to_string()],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("Option".to_string(), vec!["T".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 单态化为 Option<Int>
    let result = mono.monomorphize_type(&generic_id, &[int_type()]);

    assert!(result.is_some());
    let mono_type = result.unwrap();

    match mono_type {
        MonoType::Enum(e) => {
            assert_eq!(e.name, "Option_int64");
            assert_eq!(e.variants, vec!["Some", "None"]);
        }
        _ => panic!("Expected Enum type"),
    }
}

/// 测试：Arc 类型处理（所有权模型）
#[test]
fn test_monomorphize_with_arc() {
    let mut mono = Monomorphizer::new();

    // 定义包含 Arc 的泛型结构体
    let generic_type = MonoType::Struct(StructType {
        name: "Shared".to_string(),
        fields: vec![("data".to_string(), arc_type(type_var(0)))],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("Shared".to_string(), vec!["T".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 单态化为 Shared<Int>
    let result = mono.monomorphize_type(&generic_id, &[int_type()]);

    assert!(result.is_some());
    let mono_type = result.unwrap();

    match mono_type {
        MonoType::Struct(s) => {
            assert_eq!(s.name, "Shared_int64");
            assert_eq!(s.fields.len(), 1);

            // 验证 Arc 类型保持，内部类型被替换
            match &s.fields[0].1 {
                MonoType::Arc(inner) => {
                    assert_eq!(**inner, int_type());
                }
                _ => panic!("Expected Arc type for field"),
            }
        }
        _ => panic!("Expected Struct type"),
    }
}

/// 测试：嵌套 Arc 类型
#[test]
fn test_monomorphize_nested_arc() {
    let mut mono = Monomorphizer::new();

    // 定义嵌套 Arc 结构体
    let generic_type = MonoType::Struct(StructType {
        name: "NestedArc".to_string(),
        fields: vec![("outer".to_string(), arc_type(arc_type(type_var(0))))],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("NestedArc".to_string(), vec!["T".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 单态化
    let result = mono.monomorphize_type(&generic_id, &[int_type()]);

    assert!(result.is_some());
    let mono_type = result.unwrap();

    match mono_type {
        MonoType::Struct(s) => {
            // 验证双层 Arc 结构保持
            match &s.fields[0].1 {
                MonoType::Arc(inner1) => match inner1.as_ref() {
                    MonoType::Arc(inner2) => {
                        assert_eq!(inner2.as_ref(), &int_type());
                    }
                    _ => panic!("Expected inner Arc"),
                },
                _ => panic!("Expected outer Arc"),
            }
        }
        _ => panic!("Expected Struct type"),
    }
}

/// 测试：列表类型单态化
#[test]
fn test_monomorphize_list_type() {
    let mut mono = Monomorphizer::new();

    // 定义泛型列表类型（使用 Struct 包装）
    let generic_type = MonoType::Struct(StructType {
        name: "Container".to_string(),
        fields: vec![("items".to_string(), list_type(type_var(0)))],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("Container".to_string(), vec!["T".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 单态化为 Container<String>
    let result = mono.monomorphize_type(&generic_id, &[string_type()]);

    assert!(result.is_some());
    let mono_type = result.unwrap();

    match mono_type {
        MonoType::Struct(s) => match &s.fields[0].1 {
            MonoType::List(elem) => {
                assert_eq!(**elem, string_type());
            }
            _ => panic!("Expected List type for field"),
        },
        _ => panic!("Expected Struct type"),
    }
}

/// 测试：字典类型单态化
#[test]
fn test_monomorphize_dict_type() {
    let mut mono = Monomorphizer::new();

    // 定义泛型字典类型
    let generic_type = MonoType::Struct(StructType {
        name: "Map".to_string(),
        fields: vec![("entries".to_string(), dict_type(type_var(0), type_var(1)))],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("Map".to_string(), vec!["K".to_string(), "V".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 单态化为 Map<String, Int>
    let result = mono.monomorphize_type(&generic_id, &[string_type(), int_type()]);

    assert!(result.is_some());
    let mono_type = result.unwrap();

    match mono_type {
        MonoType::Struct(s) => match &s.fields[0].1 {
            MonoType::Dict(key, value) => {
                assert_eq!(**key, string_type());
                assert_eq!(**value, int_type());
            }
            _ => panic!("Expected Dict type for field"),
        },
        _ => panic!("Expected Struct type"),
    }
}

/// 测试：类型缓存
#[test]
fn test_type_cache() {
    let mut mono = Monomorphizer::new();

    // 定义泛型结构体
    let generic_type = MonoType::Struct(StructType {
        name: "Cached".to_string(),
        fields: vec![("value".to_string(), type_var(0))],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("Cached".to_string(), vec!["T".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 首次单态化
    let result1 = mono.monomorphize_type(&generic_id, &[int_type()]);
    assert!(result1.is_some());
    assert_eq!(mono.type_instance_count(), 1);

    // 再次单态化相同类型（应该命中缓存）
    let result2 = mono.monomorphize_type(&generic_id, &[int_type()]);
    assert!(result2.is_some());
    assert_eq!(mono.type_instance_count(), 1); // 数量不变

    // 单态化不同类型
    let result3 = mono.monomorphize_type(&generic_id, &[float_type()]);
    assert!(result3.is_some());
    assert_eq!(mono.type_instance_count(), 2); // 数量增加
}

/// 测试：类型实例ID生成
#[test]
fn test_type_id_generation() {
    let mono = Monomorphizer::new();

    let generic_id = GenericTypeId::new("Test".to_string(), vec!["T".to_string()]);

    // 无类型参数
    let id1 = mono.generate_type_id(&generic_id, &[]);
    assert_eq!(id1.name(), "Test");

    // 有类型参数
    let id2 = mono.generate_type_id(&generic_id, &[int_type()]);
    assert_eq!(id2.name(), "Test_int64");

    let id3 = mono.generate_type_id(&generic_id, &[string_type(), int_type()]);
    assert_eq!(id3.name(), "Test_string_int64");
}

/// 测试：单态化不存在的泛型类型
#[test]
fn test_monomorphize_nonexistent_type() {
    let mut mono = Monomorphizer::new();

    let generic_id = GenericTypeId::new("NonExistent".to_string(), vec!["T".to_string()]);

    // 尝试单态化不存在的类型
    let result = mono.monomorphize_type(&generic_id, &[int_type()]);

    assert!(result.is_none());
    assert_eq!(mono.type_instance_count(), 0);
}

/// 测试：注册和获取单态化类型
#[test]
fn test_register_monomorphized_type() {
    let mut mono = Monomorphizer::new();

    // 注册一个已单态化的类型
    let mono_type = MonoType::Struct(StructType {
        name: "Registered".to_string(),
        fields: vec![("value".to_string(), int_type())],
    });

    let type_id = mono.register_monomorphized_type(mono_type.clone());

    assert_eq!(type_id.name(), "Registered");
    assert_eq!(mono.type_instance_count(), 1);

    // 验证可以获取到
    let retrieved = mono.type_instances.get(&type_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().get_mono_type(), Some(&mono_type));
}

/// 测试：嵌套结构体单态化
#[test]
fn test_monomorphize_nested_struct() {
    let mut mono = Monomorphizer::new();

    // 定义嵌套泛型结构体
    let generic_type = MonoType::Struct(StructType {
        name: "Nested".to_string(),
        fields: vec![(
            "inner".to_string(),
            MonoType::Struct(StructType {
                name: "Inner".to_string(),
                fields: vec![("value".to_string(), type_var(0))],
            }),
        )],
    });

    // 注册泛型类型
    let generic_id = GenericTypeId::new("Nested".to_string(), vec!["T".to_string()]);
    mono.generic_types.insert(generic_id.clone(), generic_type);

    // 单态化
    let result = mono.monomorphize_type(&generic_id, &[int_type()]);

    assert!(result.is_some());
    let mono_type = result.unwrap();

    match mono_type {
        MonoType::Struct(s) => {
            assert_eq!(s.name, "Nested_int64");
            assert_eq!(s.fields.len(), 1);

            // 验证内部结构体类型也被替换
            match &s.fields[0].1 {
                MonoType::Struct(inner) => {
                    assert_eq!(inner.name, "Inner");
                    assert_eq!(inner.fields[0].1, int_type());
                }
                _ => panic!("Expected Struct type for inner field"),
            }
        }
        _ => panic!("Expected Struct type"),
    }
}

/// 测试：泛型类型参数计数
#[test]
fn test_generic_type_param_count() {
    let generic_id_0 = GenericTypeId::new("Zero".to_string(), vec![]);
    let generic_id_1 = GenericTypeId::new("One".to_string(), vec!["T".to_string()]);
    let generic_id_2 =
        GenericTypeId::new("Two".to_string(), vec!["T".to_string(), "U".to_string()]);

    assert_eq!(generic_id_0.type_params().len(), 0);
    assert_eq!(generic_id_1.type_params().len(), 1);
    assert_eq!(generic_id_2.type_params().len(), 2);
}
