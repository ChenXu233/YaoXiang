//! Loader 模块测试
//!
//! 测试模块加载器功能，包括：
//! - 循环依赖检测
//! - 导出提取
//! - 类型格式化

use std::collections::HashMap;

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ast::AstType;
use crate::frontend::core::parser::parse;
use crate::frontend::module::loader::{ExportKind, ModuleLoader, ModuleSource};
use crate::util::span::Span;

#[test]
fn test_detect_no_cycles() {
    let mut deps = HashMap::new();
    deps.insert("a".to_string(), vec!["b".to_string()]);
    deps.insert("b".to_string(), vec!["c".to_string()]);
    deps.insert("c".to_string(), vec![]);

    let result = ModuleLoader::detect_cycles(&deps);
    assert!(result.is_ok());
}

#[test]
fn test_detect_direct_cycle() {
    let mut deps = HashMap::new();
    deps.insert("a".to_string(), vec!["b".to_string()]);
    deps.insert("b".to_string(), vec!["a".to_string()]);

    let result = ModuleLoader::detect_cycles(&deps);
    assert!(result.is_err());
}

#[test]
fn test_detect_indirect_cycle() {
    let mut deps = HashMap::new();
    deps.insert("a".to_string(), vec!["b".to_string()]);
    deps.insert("b".to_string(), vec!["c".to_string()]);
    deps.insert("c".to_string(), vec!["a".to_string()]);

    let result = ModuleLoader::detect_cycles(&deps);
    assert!(result.is_err());
}

#[test]
fn test_detect_self_reference() {
    let mut deps = HashMap::new();
    deps.insert("a".to_string(), vec!["a".to_string()]);

    let result = ModuleLoader::detect_cycles(&deps);
    assert!(result.is_err());
}

#[test]
fn test_extract_exports_pub_fn() {
    let source = r#"
pub add: (a: Int, b: Int) -> Int = (a, b) => {
    a + b
}

helper: (x: Int) -> Int = (x) => {
    x * 2
}
"#;
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors, "parse failed: {:?}", result.errors);
    let ast = result.module;
    let module = ModuleLoader::extract_exports("my_module", &ast, &ModuleSource::User);

    // pub 函数应该被导出
    assert!(module.has_export("add"));
    assert_eq!(module.get_export("add").unwrap().kind, ExportKind::Function);
    assert_eq!(module.get_export("add").unwrap().full_path, "my_module.add");

    // 非 pub 函数不导出
    assert!(!module.has_export("helper"));
}

#[test]
fn test_extract_exports_typedef() {
    let source = r#"
Point: Type = {
    x: Int
    y: Int
}
"#;
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors, "parse failed: {:?}", result.errors);
    let ast = result.module;
    let module = ModuleLoader::extract_exports("shapes", &ast, &ModuleSource::User);

    // 类型定义始终导出
    assert!(module.has_export("Point"));
    assert_eq!(module.get_export("Point").unwrap().kind, ExportKind::Type);
}

#[test]
fn test_extract_exports_constant() {
    let source = r#"
MAX_SIZE = 100
mut counter = 0
"#;
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors, "parse failed: {:?}", result.errors);
    let ast = result.module;
    let module = ModuleLoader::extract_exports("config", &ast, &ModuleSource::User);

    // 不可变绑定导出为常量
    assert!(module.has_export("MAX_SIZE"));
    assert_eq!(
        module.get_export("MAX_SIZE").unwrap().kind,
        ExportKind::Constant
    );

    // 可变绑定不导出
    assert!(!module.has_export("counter"));
}

#[test]
fn test_format_type_fn() {
    let ty = AstType::Fn {
        params: vec![
            AstType::Name {
                name: "Int".to_string(),
                span: Span::dummy(),
            },
            AstType::Name {
                name: "Int".to_string(),
                span: Span::dummy(),
            },
        ],
        return_type: Box::new(AstType::Name {
            name: "Bool".to_string(),
            span: Span::dummy(),
        }),
    };
    assert_eq!(format_type(&ty), "(Int, Int) -> Bool");
}

#[test]
fn test_format_type_generic() {
    let ty = AstType::Generic {
        name: "List".to_string(),
        name_span: Span::dummy(),
        args: vec![AstType::Name {
            name: "Int".to_string(),
            span: Span::dummy(),
        }],
    };
    assert_eq!(format_type(&ty), "List(Int)");
}
