//! ref 表达式类型推断测试

use crate::frontend::typecheck::MonoType;

/// 测试 ref 表达式推断为 Arc<T>
#[test]
fn test_ref_type_is_arc() {
    // 验证 Arc 类型正确构造
    let inner_type = MonoType::Int(64);
    let arc_type = MonoType::Arc(Box::new(inner_type));

    match arc_type {
        MonoType::Arc(inner) => {
            assert_eq!(*inner, MonoType::Int(64));
        }
        _ => panic!("Expected Arc type"),
    }
}

/// 测试嵌套 Arc 类型
#[test]
fn test_nested_arc_type() {
    let inner_type = MonoType::String;
    let arc_type = MonoType::Arc(Box::new(inner_type));

    match arc_type {
        MonoType::Arc(inner) => {
            // inner 是 Box<MonoType>，需要解引用
            match inner.as_ref() {
                MonoType::String => {}
                _ => panic!("Expected String inside Arc"),
            }
        }
        _ => panic!("Expected Arc type"),
    }
}

/// 测试 Arc 类型的 type_name
#[test]
fn test_arc_type_name() {
    let int_type = MonoType::Int(64);
    let arc_type = MonoType::Arc(Box::new(int_type));

    let type_name = arc_type.type_name();
    assert_eq!(type_name, "Arc<int64>");
}

/// 测试 Arc 类型与其他类型不同
#[test]
fn test_arc_type_difference() {
    let int_type = MonoType::Int(64);
    let arc_type = MonoType::Arc(Box::new(MonoType::Int(64)));

    // Arc<Int> 不等于 Int
    assert_ne!(arc_type, int_type);
    // Arc<Int> 等于 Arc<Int>
    let arc_type2 = MonoType::Arc(Box::new(MonoType::Int(64)));
    assert_eq!(arc_type, arc_type2);
}

/// 测试 Arc 类型格式化输出
#[test]
fn test_arc_type_display() {
    // 创建一个简单的列表类型
    let list_type = MonoType::List(Box::new(MonoType::Int(64)));
    let arc_type = MonoType::Arc(Box::new(list_type));

    let display = format!("{}", arc_type);
    assert!(display.contains("Arc"));
}

// ============================================================================
// Weak 类型测试
// ============================================================================

/// 测试 Weak 类型构造
#[test]
fn test_weak_type_construction() {
    let inner_type = MonoType::Int(64);
    let weak_type = MonoType::Weak(Box::new(inner_type));

    match weak_type {
        MonoType::Weak(inner) => {
            assert_eq!(*inner, MonoType::Int(64));
        }
        _ => panic!("Expected Weak type"),
    }
}

/// 测试 Weak 类型的 type_name
#[test]
fn test_weak_type_name() {
    let int_type = MonoType::Int(32);
    let weak_type = MonoType::Weak(Box::new(int_type));

    let type_name = weak_type.type_name();
    assert_eq!(type_name, "Weak<int32>");
}

/// 测试嵌套 Weak 类型
#[test]
fn test_nested_weak_type() {
    let struct_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Node".to_string(),
        fields: vec![],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });
    let weak_type = MonoType::Weak(Box::new(struct_type));

    match weak_type {
        MonoType::Weak(inner) => match inner.as_ref() {
            MonoType::Struct(s) => {
                assert_eq!(s.name, "Node");
            }
            _ => panic!("Expected Struct inside Weak"),
        },
        _ => panic!("Expected Weak type"),
    }
}

/// 测试 Weak 类型与其他类型不同
#[test]
fn test_weak_type_difference() {
    let int_type = MonoType::Int(64);
    let weak_type = MonoType::Weak(Box::new(MonoType::Int(64)));

    // Weak<Int> 不等于 Int
    assert_ne!(weak_type, int_type);
    // Weak<Int> 等于 Weak<Int>
    let weak_type2 = MonoType::Weak(Box::new(MonoType::Int(64)));
    assert_eq!(weak_type, weak_type2);
    // Weak<Int> 不等于 Arc<Int>
    let arc_type = MonoType::Arc(Box::new(MonoType::Int(64)));
    assert_ne!(weak_type, arc_type);
}

/// 测试 Weak 类型的 Display 实现
#[test]
fn test_weak_type_display() {
    let list_type = MonoType::List(Box::new(MonoType::String));
    let weak_type = MonoType::Weak(Box::new(list_type));

    let display = format!("{}", weak_type);
    assert!(display.contains("Weak"));
    assert!(display.contains("List"));
}

/// 测试 Weak 包含类型变量
#[test]
fn test_weak_with_type_var() {
    use crate::frontend::core::type_system::TypeVar;

    let tv = TypeVar::new(0);
    let type_var = MonoType::TypeVar(tv);
    let weak_type = MonoType::Weak(Box::new(type_var));

    let type_name = weak_type.type_name();
    assert!(type_name.contains("Weak"));
    assert!(type_name.contains("t0")); // TypeVar displays as "t{index}"
}
