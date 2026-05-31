//! SourceMap 测试
//!
//! 对应 formatter 规范 §C (comments)

use crate::formatter::source_map::{CommentStyle, SourceMap};

#[test]
fn test_source_map_single_line_comment() {
    let source = "// hello\nlet x = 1";
    let sm = SourceMap::build(source);
    assert_eq!(sm.comments.len(), 1);
    assert_eq!(sm.comments[0].content, "// hello");
    assert_eq!(sm.comments[0].style, CommentStyle::SingleLine);
}

#[test]
fn test_source_map_multiline_comment() {
    let source = "/* multi\nline */\nlet x = 1";
    let sm = SourceMap::build(source);
    assert_eq!(sm.comments.len(), 1);
    assert_eq!(sm.comments[0].content, "/* multi\nline */");
    assert_eq!(sm.comments[0].style, CommentStyle::MultiLine);
}

#[test]
fn test_source_map_doc_comment() {
    let source = "/// doc comment\nfn foo() {}";
    let sm = SourceMap::build(source);
    assert_eq!(sm.comments.len(), 1);
    assert_eq!(sm.comments[0].style, CommentStyle::Doc);
}

#[test]
fn test_source_map_nested_comment() {
    let source = "/* outer /* inner */ still outer */\ncode";
    let sm = SourceMap::build(source);
    assert_eq!(sm.comments.len(), 1);
    assert_eq!(
        sm.comments[0].content,
        "/* outer /* inner */ still outer */"
    );
}

#[test]
fn test_source_map_blank_lines() {
    let source = "line1\n\nline3\n\n\nline6";
    let sm = SourceMap::build(source);
    assert!(sm.blank_lines.contains(&2));
    assert!(sm.blank_lines.contains(&4));
    assert!(sm.blank_lines.contains(&5));
}

#[test]
fn test_source_map_offset_to_line() {
    let source = "abc\ndef\nghi";
    let sm = SourceMap::build(source);
    assert_eq!(sm.offset_to_line(0), 1);
    assert_eq!(sm.offset_to_line(4), 2);
    assert_eq!(sm.offset_to_line(8), 3);
}

#[test]
fn test_source_map_comment_in_string_ignored() {
    let source = "let x = \"// not a comment\"\n// real comment";
    let sm = SourceMap::build(source);
    assert_eq!(sm.comments.len(), 1);
    assert_eq!(sm.comments[0].content, "// real comment");
}

#[test]
fn test_source_map_comment_in_block_body() {
    let source = "for x in [1, 2, 3] {\n    // loop comment\n    let y = x\n}\n";
    let sm = SourceMap::build(source);
    assert_eq!(sm.comments.len(), 1);
    assert_eq!(sm.comments[0].content, "// loop comment");
    assert_eq!(sm.comments[0].span.start.line, 2);
}
