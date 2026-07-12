//! 类型单态化测试 — 基于 RFC-011: 泛型系统设计
//!
//! RFC-011 §1.3: 单态化 — 编译期零成本抽象，泛型类型特化为具体类型
//! RFC-011 §1.1: 泛型类型参数 — (T: Type) -> Type 的实例化
//!
//! 测试覆盖：
//! - collect_generic_types: 从 ModuleIR 中识别泛型类型定义
//! - monomorphize_type: 泛型类型实例化
//! - 缓存去重

use crate::frontend::core::parser::ast::{StructField, Type as AstType};
use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::ModuleIR;
use crate::middle::passes::mono::instance::GenericTypeId;
use crate::middle::passes::mono::Monomorphizer;
use crate::util::span::Span;

// ── 辅助函数 ──────────────────────────────────────────────────

/// 创建包含泛型类型 List(T) 定义的 ModuleIR
///
/// List = { data: Array(T), length: Int }
/// RFC-011 §1.1: 泛型类型参数 (T: Type)
fn make_module_with_generic_list() -> ModuleIR {
    ModuleIR {
        types: vec![AstType::NamedStruct {
            name: "List".to_string(),
            name_span: Span::default(),
            fields: vec![
                StructField::new(
                    "data".to_string(),
                    false,
                    AstType::Generic {
                        name: "Array".to_string(),
                        name_span: Span::default(),
                        args: vec![AstType::Name {
                            name: "T".to_string(),
                            span: Span::default(),
                        }],
                    },
                ),
                StructField::new("length".to_string(), false, AstType::Int(64)),
            ],
        }],
        ..ModuleIR::default()
    }
}

/// 创建不包含泛型类型（仅具体类型）的 ModuleIR
fn make_module_with_concrete_types() -> ModuleIR {
    ModuleIR {
        types: vec![AstType::NamedStruct {
            name: "Person".to_string(),
            name_span: Span::default(),
            fields: vec![
                StructField::new("name".to_string(), false, AstType::String),
                StructField::new("age".to_string(), false, AstType::Int(64)),
            ],
        }],
        ..ModuleIR::default()
    }
}

// ── collect_generic_types 测试 ─────────────────────────────────

#[test]
fn test_collect_generic_types_detects_generic_struct() {
    // Arrange
    let module = make_module_with_generic_list();
    let mut mono = Monomorphizer::new();

    // Act
    mono.collect_generic_types(&module);

    // Assert
    assert!(
        mono.generic_types.contains_key("List"),
        "含类型变量 T 的 NamedStruct 应被识别为泛型类型"
    );
}

#[test]
fn test_collect_generic_types_skips_concrete_types() {
    // Arrange
    let module = make_module_with_concrete_types();
    let mut mono = Monomorphizer::new();

    // Act
    mono.collect_generic_types(&module);

    // Assert
    assert!(
        mono.generic_types.is_empty(),
        "应跳过不含类型变量的具体类型"
    );
}

#[test]
fn test_collect_generic_types_empty_module() {
    // Arrange
    let module = ModuleIR::default();
    let mut mono = Monomorphizer::new();

    // Act
    mono.collect_generic_types(&module);

    // Assert
    assert!(
        mono.generic_types.is_empty(),
        "空 ModuleIR 不应收集到任何泛型类型"
    );
}

// ── monomorphize_type 测试 ─────────────────────────────────────

#[test]
fn test_monomorphize_type_returns_mono_type_for_known_generic() {
    // Arrange
    let module = make_module_with_generic_list();
    let mut mono = Monomorphizer::new();
    mono.collect_generic_types(&module);
    let generic_id = GenericTypeId::new("List".to_string(), vec!["T".to_string()]);
    let type_args = vec![MonoType::Int(64)];

    // Act
    let result = mono.monomorphize_type(&generic_id, &type_args);

    // Assert
    assert!(result.is_some(), "已知泛型类型 List(Int64) 应成功单态化");
    let mono_type = result.unwrap();
    assert!(
        matches!(&mono_type, MonoType::Struct(s) if s.name == "List_int64"),
        "单态化后类型名应为 List_int64，实际得到 {:?}",
        mono_type.type_name()
    );
}

#[test]
fn test_monomorphize_type_unknown_generic_returns_none() {
    // Arrange
    let mut mono = Monomorphizer::new();
    let generic_id = GenericTypeId::new("Unknown".to_string(), vec!["T".to_string()]);
    let type_args = vec![MonoType::Int(64)];

    // Act
    let result = mono.monomorphize_type(&generic_id, &type_args);

    // Assert
    assert!(result.is_none(), "未知泛型类型 Unknown 应返回 None");
}

#[test]
fn test_monomorphize_type_with_string_arg() {
    // Arrange
    let module = make_module_with_generic_list();
    let mut mono = Monomorphizer::new();
    mono.collect_generic_types(&module);
    let generic_id = GenericTypeId::new("List".to_string(), vec!["T".to_string()]);
    let type_args = vec![MonoType::String];

    // Act
    let result = mono.monomorphize_type(&generic_id, &type_args);

    // Assert
    assert!(result.is_some(), "List(String) 应成功单态化");
    let mono_type = result.unwrap();
    assert!(
        matches!(&mono_type, MonoType::Struct(s) if s.name == "List_string"),
        "单态化后类型名应为 List_string，实际得到 {:?}",
        mono_type.type_name()
    );
}

// ── 缓存去重测试 ───────────────────────────────────────────────

#[test]
fn test_monomorphize_type_deduplicates_identical_requests() {
    // Arrange
    let module = make_module_with_generic_list();
    let mut mono = Monomorphizer::new();
    mono.collect_generic_types(&module);
    let generic_id = GenericTypeId::new("List".to_string(), vec!["T".to_string()]);
    let type_args = vec![MonoType::Int(64)];

    // Act
    let result1 = mono.monomorphize_type(&generic_id, &type_args);
    let result2 = mono.monomorphize_type(&generic_id, &type_args);

    // Assert
    assert!(result1.is_some(), "第一次调 List(Int64) 单态化应成功");
    assert_eq!(
        result1, result2,
        "相同类型参数的两次调用应返回相同实例（缓存去重）"
    );
}

#[test]
fn test_monomorphize_type_different_args_produce_different_types() {
    // Arrange
    let module = make_module_with_generic_list();
    let mut mono = Monomorphizer::new();
    mono.collect_generic_types(&module);
    let generic_id = GenericTypeId::new("List".to_string(), vec!["T".to_string()]);

    // Act
    let int_result = mono.monomorphize_type(&generic_id, &[MonoType::Int(64)]);
    let string_result = mono.monomorphize_type(&generic_id, &[MonoType::String]);

    // Assert
    assert!(int_result.is_some(), "List(Int64) 单态化应成功");
    assert!(string_result.is_some(), "List(String) 单态化应成功");
    assert_ne!(
        int_result, string_result,
        "不同类型参数 Int64 和 String 应产生不同的单态化结果"
    );
}
