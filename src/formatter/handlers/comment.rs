//! 注释格式化处理器

use super::super::source_map::{CommentStyle, SourceMap};

/// 格式化两个行号之间的注释
pub fn format_comments_between(
    source_map: &SourceMap,
    start_line: usize,
    end_line: usize,
) -> String {
    let comments = source_map.comments_between_lines(start_line, end_line);
    let mut result = String::new();

    for comment in comments {
        match &comment.style {
            CommentStyle::SingleLine | CommentStyle::Doc => {
                result.push_str(&comment.content);
                if !comment.content.ends_with('\n') {
                    result.push('\n');
                }
            }
            CommentStyle::MultiLine => {
                result.push_str(&comment.content);
                if !comment.content.ends_with('\n') {
                    result.push('\n');
                }
            }
        }
    }

    result
}

/// 格式化单行注释（确保 `// ` 后有空格）
pub fn normalize_single_line_comment(content: &str) -> String {
    if content.starts_with("// ") || content.starts_with("///") {
        content.to_string()
    } else if let Some(stripped) = content.strip_prefix("//") {
        format!("// {}", stripped)
    } else {
        content.to_string()
    }
}
