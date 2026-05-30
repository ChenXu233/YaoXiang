//! TextEmitter 测试 — 基于 check-improvement 设计规范
//!
//! §4.1: Emitter 合并（RichEmitter → TextEmitter）

use crate::util::diagnostic::emitter::{TextEmitter, EmitterConfig};
use crate::util::diagnostic::emitter::ansi::strip_ansi;
use crate::util::diagnostic::codes::ErrorCodeDefinition;
use crate::util::span::Span;

#[test]
fn test_text_emitter_render_basic_error() {
    let diagnostic = ErrorCodeDefinition::invalid_character("@").build();

    let emitter = TextEmitter::new();
    let output = emitter.render(&diagnostic);
    let clean_output = strip_ansi(&output);

    assert!(clean_output.contains("error [E0001]"), "{}", clean_output);
    assert!(
        clean_output.contains("Invalid character"),
        "{}",
        clean_output
    );
}

#[test]
fn test_text_emitter_render_error_with_span() {
    let diagnostic = ErrorCodeDefinition::type_mismatch("Int", "String")
        .at(Span::dummy())
        .build();

    let emitter = TextEmitter::new();
    let output = emitter.render(&diagnostic);
    let clean_output = strip_ansi(&output);

    assert!(clean_output.contains("error [E1002]"), "{}", clean_output);
}

#[test]
fn test_text_emitter_config_disables_colors() {
    let config = EmitterConfig {
        use_colors: false,
        show_help: true,
        ..Default::default()
    };

    let diagnostic = ErrorCodeDefinition::invalid_character("@").build();

    let emitter = TextEmitter::with_config(config);
    let output = emitter.render(&diagnostic);

    assert!(!output.contains("\x1b[31m"), "colors should be disabled");
}
