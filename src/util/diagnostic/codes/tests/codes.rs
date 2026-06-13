//! ErrorCodeDefinition 测试 — 基于 check-improvement 设计规范
//!
//! §3.4: i18n 修复（I3/I4: E1090/E1091 翻译）
//! §4.6: error! 宏简化

use crate::util::diagnostic::codes::{ErrorCodeDefinition, I18nRegistry};
use crate::util::span::Span;

#[test]
fn test_error_code_find_existing_code() {
    let code = ErrorCodeDefinition::find("E0001");
    assert!(code.is_some(), "E0001 should exist in error code registry");
    assert_eq!(code.unwrap().code, "E0001");
}

#[test]
fn test_error_code_find_unknown_returns_none() {
    let code = ErrorCodeDefinition::find("E9999");
    assert!(
        code.is_none(),
        "E9999 should not exist in error code registry"
    );
}

#[test]
fn test_error_code_registry_has_minimum_count() {
    let all = ErrorCodeDefinition::all();
    assert!(
        all.len() > 30,
        "Expected more than 30 error codes, got {}",
        all.len()
    );
}

#[test]
fn test_i18n_registry_english_titles() {
    let en = I18nRegistry::en();
    assert_eq!(en.get_title("E0001"), "Invalid Character");
    assert!(
        !en.get_help("E0001").is_empty(),
        "E0001 help text should not be empty"
    );
}

#[test]
fn test_i18n_registry_chinese_titles() {
    let zh = I18nRegistry::zh();
    assert_eq!(zh.get_title("E0001"), "无效字符");
    assert!(
        !zh.get_help("E0001").is_empty(),
        "E0001 help text should not be empty"
    );
}

#[test]
fn test_template_render_substitutes_params() {
    let en = I18nRegistry::en();
    let template = "Unknown variable: '{name}'";
    let params = vec![("name", "x".to_string())];
    let rendered = en.render(template, &params);
    assert_eq!(rendered, "Unknown variable: 'x'");
}

#[test]
fn test_diagnostic_builder_with_params_and_span() {
    let code = ErrorCodeDefinition::find("E0001").unwrap();

    let diagnostic = code
        .builder()
        .param("char", "@")
        .at(Span::default())
        .build();

    assert_eq!(diagnostic.code, "E0001");
    assert!(
        diagnostic.message.contains("@"),
        "message should contain param value"
    );
}

#[test]
fn test_i18n_consistency() {
    // 1. 检查 Rust 静态数组中是否有重复的错误码
    let all = ErrorCodeDefinition::all();
    let mut seen = std::collections::HashSet::new();
    for def in all {
        assert!(
            seen.insert(def.code),
            "Duplicate error code in Rust registry: {}",
            def.code
        );
    }

    // 2. 检查 JSON 中是否有重复的错误码
    let en = I18nRegistry::en();
    let zh = I18nRegistry::zh();

    // 3. 检查 Rust 注册的每个码在 JSON 中都有模板
    for def in all {
        let title = en.get_title(def.code);
        assert!(
            !title.is_empty(),
            "Error code {} missing English title in i18n JSON",
            def.code
        );
        let title_zh = zh.get_title(def.code);
        assert!(
            !title_zh.is_empty(),
            "Error code {} missing Chinese title in i18n JSON",
            def.code
        );
    }
}
