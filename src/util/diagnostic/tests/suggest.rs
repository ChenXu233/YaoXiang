//! SuggestionEngine 测试 — 基于 check-improvement 设计规范
//!
//! §4.5: SuggestionEngine 集成
//! X1: get_suggestions() 接入

use crate::util::diagnostic::suggest::{SuggestionEngine, Suggestion};

#[test]
fn test_similarity_identical_strings() {
    let engine = SuggestionEngine::new();
    assert_eq!(engine.similarity("hello", "hello"), 1.0);
}

#[test]
fn test_similarity_similar_strings() {
    let engine = SuggestionEngine::new();
    let sim = engine.similarity("hello", "hallo");
    assert!(sim > 0.7, "Expected similarity > 0.7, got {}", sim);
}

#[test]
fn test_similarity_unrelated_strings() {
    let engine = SuggestionEngine::new();
    let sim = engine.similarity("abc", "xyz");
    assert!(sim < 0.5, "Expected low similarity, got {}", sim);
}

#[test]
fn test_find_similar_returns_matching_names() {
    let mut engine = SuggestionEngine::new();
    engine.add_defined_name("variable");
    engine.add_defined_name("variant");
    engine.add_defined_name("value");
    engine.add_defined_name("valley");

    let similar = engine.find_similar("varible");
    assert!(!similar.is_empty(), "Should find similar names");

    let names: Vec<String> = similar.iter().map(|(n, _)| n.clone()).collect();
    assert!(
        names.contains(&"variable".to_string()) || names.contains(&"variant".to_string()),
        "should find 'variable' or 'variant', got {:?}",
        names
    );
}

#[test]
fn test_levenshtein_distance_known_cases() {
    let engine = SuggestionEngine::new();

    assert_eq!(engine.levenshtein_distance("kitten", "sitting"), 3);
    assert_eq!(engine.levenshtein_distance("", ""), 0);
    assert_eq!(engine.levenshtein_distance("abc", ""), 3);
    assert_eq!(engine.levenshtein_distance("", "abc"), 3);
    assert_eq!(engine.levenshtein_distance("abc", "abc"), 0);
}

#[test]
fn test_from_scope_populates_names() {
    let engine = SuggestionEngine::from_scope(&["foo", "bar", "baz"]);
    assert_eq!(engine.len(), 3);
    assert!(!engine.is_empty());
}

#[test]
fn test_add_name_type_stores_mapping() {
    let mut engine = SuggestionEngine::new();
    engine.add_name_type("foo", "Int -> Int");
    assert_eq!(
        engine.name_to_types.get("foo"),
        Some(&"Int -> Int".to_string()),
        "name_to_types should store the mapping"
    );
}

#[test]
fn test_suggest_for_unknown_variable_returns_suggestion() {
    let mut engine = SuggestionEngine::new();
    engine.add_defined_name("variable");
    engine.add_defined_name("variant");

    let suggestion = engine.suggest_for_unknown_variable("varible");
    assert!(suggestion.is_some(), "should find suggestion for typo");
    match suggestion.unwrap() {
        Suggestion::Variable { typo, suggestions } => {
            assert_eq!(typo, "varible");
            assert!(
                !suggestions.is_empty(),
                "should have at least one suggestion"
            );
        }
        other => panic!("Expected Variable suggestion, got {:?}", other),
    }
}

#[test]
fn test_clear_cache_empties_cache() {
    let mut engine = SuggestionEngine::new();
    engine.add_defined_name("foo");
    engine.find_similar("fuo"); // writes to cache
    engine.clear_cache();
    assert!(
        engine.similarity_cache.read().is_empty(),
        "cache should be empty after clear"
    );
}
