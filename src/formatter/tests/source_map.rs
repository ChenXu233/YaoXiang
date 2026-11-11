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
