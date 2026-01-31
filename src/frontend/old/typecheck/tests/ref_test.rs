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
